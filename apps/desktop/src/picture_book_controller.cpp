#include "picture_book_controller.hpp"
#include "desktop_backend.hpp"
#include <QBuffer>
#include <QDebug>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPageLayout>
#include <QPageSize>
#include <QPdfWriter>
#include <QTemporaryFile>
#include <algorithm>
#include <filesystem>
#include <stdexcept>

namespace {
QString json(const QJsonObject& object) {
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}
QJsonObject object(const QString& value) {
    QJsonParseError error;
    const auto document = QJsonDocument::fromJson(value.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isObject())
        throw std::runtime_error("invalid book JSON");
    return document.object();
}
QJsonObject pin(const QVariantMap& item) {
    return {
        {"artifact_id", item.value(QStringLiteral("id")).toString()},
        {"revision_id", item.value(QStringLiteral("acceptedRevisionId")).toString()}
    };
}
} // namespace
PictureBookController::PictureBookController(DesktopBackend& backend, QObject* parent)
    : QObject(parent), backend_(backend), project_id_(backend.projectId()),
      project_path_(backend.bundlePath()) {
    connect(&backend_, &DesktopBackend::projectChanged, this, [this] {
        if (project_id_ != backend_.projectId() || project_path_ != backend_.bundlePath()) {
            project_id_ = backend_.projectId();
            project_path_ = backend_.bundlePath();
            invalidate();
            draft_ = {};
            source_ = {};
            composition_ = {};
            accepted_composition_ = {};
            drafts_.clear();
            draft_changes_ = true;
            emit documentChanged();
            emit draftChanged();
            emit draftsChanged();
        }
    });
}
bool PictureBookController::hasTitlePage() const {
    const auto parsed = preview_["parsed_scripts"].toObject();
    return !parsed.isEmpty() && parsed.begin().value().toObject()["has_title"].toBool();
}
void PictureBookController::fail(const QString& message, const std::exception& error) {
    qWarning().noquote() << "picture book:" << error.what();
    error_ = message;
    emit previewChanged();
}
void PictureBookController::invalidate() {
    if (!candidate_id_.isEmpty())
        backend_.discardCandidate(candidate_id_);
    candidate_id_.clear();
    candidate_artifact_id_.clear();
    error_.clear();
    status_.clear();
    preview_source_.clear();
    preview_ = {};
    native_export_ = QUrl();
    emit reviewChanged();
    emit previewChanged();
}
void PictureBookController::beginNew() {
    const QJsonObject composition = {
        {"name", tr("New picture book")},
        {"default_language", "en"},
        {"languages", QJsonObject{}},
        {"pages",
         QJsonArray{
             QJsonObject{{"kind", "blank"}, {"print_only", false}},
             QJsonObject{{"kind", "blank"}, {"print_only", false}}
         }},
        {"canvas_width", 1024},
        {"canvas_height", 768},
        {"font_size", 26}
    };
    try {
        const QJsonObject state{
            {"composition", composition},
            {"active_language", "en"},
            {"page_index", 0},
            {"script", "[Cover]\n\n[Author]\n\n[1]\n"},
            {"script_dirty", false},
            {"artwork_edit", QJsonValue::Null}
        };
        const auto result = draftAction({{"action", "draft_new"}, {"state", state}});
        invalidate();
        accepted_composition_ = {};
        setDraft(result["draft"].toObject());
        refresh();
        emit documentChanged();
    } catch (const std::exception& e) {
        fail(tr("Could not create a book draft. Your current work is still here."), e);
    }
}
QJsonObject PictureBookController::draftAction(const QJsonObject& request) {
    return object(backend_.bookDraftAction(json(request)));
}
void PictureBookController::setDraft(const QJsonObject& value) {
    if (value["artifact_id"].toString().isEmpty() || value["draft_digest"].toString().isEmpty()
        || !value["state"].isObject())
        throw std::runtime_error("incomplete book draft");
    draft_ = value;
    composition_ = value["state"].toObject()["composition"].toObject();
    source_ = value["source"].toObject();
    draft_changes_ = source_.isEmpty() || composition_ != accepted_composition_
                     || value["state"].toObject()["script_dirty"].toBool();
    error_.clear();
    emit draftChanged();
}
bool PictureBookController::refresh() {
    try {
        drafts_ = draftAction({{"action", "draft_list"}})["drafts"].toArray().toVariantList();
        emit draftsChanged();
        return true;
    } catch (const std::exception& e) {
        fail(tr("Could not load saved book drafts. Your current work is still here."), e);
        return false;
    }
}
bool PictureBookController::saveState(const QVariantMap& value) {
    if (draft_.isEmpty())
        return false;
    const auto state = QJsonObject::fromVariantMap(value);
    if (state == draft_["state"].toObject())
        return true;
    try {
        const auto previous = draft_["state"].toObject();
        const auto result = draftAction(
            {{"action", "draft_save"},
             {"artifact_id", draft_["artifact_id"]},
             {"draft_digest", draft_["draft_digest"]},
             {"state", state}}
        );
        if (state["composition"] != previous["composition"] || state["script_dirty"].toBool())
            invalidate();
        setDraft(result["draft"].toObject());
        // Saving a checkpoint never replaces the live QML script or creates an accepted version.
        refresh();
        return true;
    } catch (const std::exception& e) {
        fail(
            tr("Could not save this book draft. Keep this window open and retry before closing the "
               "project."),
            e
        );
        return false;
    }
}
bool PictureBookController::discardDraft() {
    try {
        const auto id = draft_["artifact_id"].toString();
        const bool accepted = !source_.isEmpty();
        (void)draftAction(
            {{"action", "draft_discard"},
             {"artifact_id", id},
             {"draft_digest", draft_["draft_digest"]}}
        );
        if (accepted)
            return load(id);
        beginNew();
        return !draft_.isEmpty();
    } catch (const std::exception& e) {
        fail(
            tr("Could not discard a changed draft. Reopen its saved version before trying again."),
            e
        );
        return false;
    }
}
bool PictureBookController::load(const QString& id) {
    try {
        const auto value =
            draftAction({{"action", "draft_load"}, {"artifact_id", id}})["draft"].toObject();
        const auto source = value["source"].toObject();
        const auto accepted =
            source.isEmpty() ? QJsonObject() : object(backend_.bookRead(json(source)));
        invalidate();
        accepted_composition_ = accepted;
        setDraft(value);
        refresh();
        emit documentChanged();
        const auto state = value["state"].toObject();
        if (!state["script_dirty"].toBool() && !composition_["languages"].toObject().isEmpty()) {
            try {
                preview_ = preview(composition_);
                showPage(state["active_language"].toString(), state["page_index"].toInt());
            } catch (const std::exception&) { /* An incomplete draft must remain recoverable. */
            }
        }
        return true;
    } catch (const std::exception& e) {
        fail(tr("Could not open this book. Its saved version has not changed."), e);
        return false;
    }
}
QJsonObject PictureBookController::preview(const QJsonObject& composition) {
    if (composition["canvas_width"].toInt() < 256 || composition["canvas_height"].toInt() < 256)
        throw std::runtime_error(
            "reading layout requires a canvas of at least 256 pixels per axis"
        );
    return object(backend_.bookPreview(json(composition)));
}
bool PictureBookController::review(const QVariantMap& value) {
    invalidate();
    try {
        const auto composition = QJsonObject::fromVariantMap(value);
        if (draft_.isEmpty())
            beginNew();
        auto state = draft_["state"].toObject();
        if (state["script_dirty"].toBool())
            throw std::runtime_error("adopt or discard the script draft first");
        state["composition"] = composition;
        auto language = state["active_language"].toString();
        if (!composition["languages"].toObject().contains(language))
            language = composition["default_language"].toString();
        state["active_language"] = language;
        state["page_index"] = std::clamp(
            state["page_index"].toInt(),
            0,
            std::max(0, static_cast<int>(composition["pages"].toArray().size()) - 1)
        );
        state["script"] =
            scriptText(composition["languages"].toObject()[language].toObject().toVariantMap());
        if (!saveState(state.toVariantMap()))
            throw std::runtime_error("could not checkpoint book before review");
        auto prepared = preview(composition);
        // Check each language/page before offering adoption. No clipped text enters the review.
        for (const auto& language : composition["languages"].toObject().keys()) {
            const auto pages = composition["pages"].toArray();
            for (int index = 0; index < pages.size(); ++index) {
                const auto book_page = page(prepared, language, index);
                QImage measure(1, 1, QImage::Format_ARGB32);
                QPainter painter(&measure);
                const auto layout = picture_book::paint(painter, book_page);
                if (!layout.fits || layout.missing_glyphs)
                    throw std::runtime_error("text does not fit or required glyphs are missing");
            }
        }
        const auto candidate = backend_.bookDraftPropose(
            draft_["artifact_id"].toString(),
            draft_["draft_digest"].toString()
        );
        candidate_id_ = candidate;
        for (const auto& c : backend_.candidates())
            if (c.toMap().value(QStringLiteral("id")).toString() == candidate)
                candidate_artifact_id_ = c.toMap().value(QStringLiteral("artifactId")).toString();
        composition_ = composition;
        preview_ = prepared;
        emit reviewChanged();
        return showPage(language, state["page_index"].toInt());
    } catch (const std::exception& e) {
        fail(
            tr("Could not prepare the book. Check language page tags, current materials, text "
               "length and font size."),
            e
        );
        return false;
    }
}
bool PictureBookController::accept() {
    if (candidate_id_.isEmpty())
        return false;
    const auto artifact = candidate_artifact_id_;
    if (!backend_.acceptCandidate(candidate_id_)) {
        error_ = tr(
            "The book or a material changed. Refresh materials and review again before adopting."
        );
        emit previewChanged();
        return false;
    }
    candidate_id_.clear();
    candidate_artifact_id_.clear();
    emit reviewChanged();
    return load(artifact);
}
picture_book::Page
PictureBookController::page(const QJsonObject& prepared, const QString& language, int index) {
    const auto composition = prepared["composition"].toObject();
    const auto scripts = prepared["parsed_scripts"].toObject();
    if (!scripts.contains(language))
        throw std::runtime_error("language is not in this book");
    const auto pages = composition["pages"].toArray();
    if (index < 0 || index >= pages.size())
        throw std::runtime_error("invalid book page");
    const auto script = scripts[language].toObject();
    const auto item = pages[index].toObject();
    picture_book::Page result;
    result.canvas =
        QSizeF(composition["canvas_width"].toInt(), composition["canvas_height"].toInt());
    result.font_size = composition["font_size"].toInt(26);
    result.index = index;
    result.role = index == 0                                     ? QStringLiteral("cover")
                  : (index == 1 && script["has_title"].toBool()) ? QStringLiteral("title")
                                                                 : QStringLiteral("body");
    result.text = script["page_text"].toObject()[QString::number(index)].toString();
    result.author = index == 0 ? script["author"].toString() : QString();
    if (item["kind"].toString() == QStringLiteral("image")) {
        const auto bytes =
            backend_.bookMaterial(json(item["source"].toObject()), QStringLiteral("image"));
        result.artwork = QImage::fromData(bytes, "PNG");
        if (result.artwork.isNull())
            throw std::runtime_error("book image cannot be decoded");
    }
    return result;
}
QImage PictureBookController::pageImage(
    const QString& language,
    int index,
    picture_book::LayoutResult* result
) {
    return picture_book::preview(page(preview_, language, index), result);
}
bool PictureBookController::showPage(const QString& language, int index) {
    try {
        picture_book::LayoutResult layout;
        const auto image = pageImage(language, index, &layout);
        if (!layout.fits || layout.missing_glyphs)
            throw std::runtime_error("page does not fit or has missing glyphs");
        QByteArray png;
        QBuffer buffer(&png);
        buffer.open(QIODevice::WriteOnly);
        image.save(&buffer, "PNG");
        preview_source_ =
            QStringLiteral("data:image/png;base64,") + QString::fromLatin1(png.toBase64());
        const auto item = composition_["pages"].toArray().at(index).toObject();
        status_ = tr("Page %1 · %2 · %3").arg(index + 1).arg(language, layout.font_family);
        if (item["print_only"].toBool())
            status_ += tr(" · Print only");
        if (!preview_["stale_inputs"].toArray().isEmpty())
            status_ += tr(" · Saved materials have newer versions");
        if (!preview_["missing_text"].toArray().isEmpty())
            status_ += tr(" · Some language pages have no text; review each language");
        error_.clear();
        emit previewChanged();
        return true;
    } catch (const std::exception& e) {
        preview_source_.clear();
        fail(tr("This page cannot be displayed. Check its materials and text layout."), e);
        return false;
    }
}
QString PictureBookController::scriptText(const QVariantMap& source) {
    try {
        return QString::fromUtf8(
            backend_.bookMaterial(json(QJsonObject::fromVariantMap(source)), QStringLiteral("text"))
        );
    } catch (const std::exception& e) {
        fail(tr("Could not read this script material."), e);
        return {};
    }
}
QVariantMap PictureBookController::adoptScript(
    const QString& name,
    const QVariantMap& source,
    const QString& text
) {
    try {
        auto value = object(backend_.bookScript(
            name,
            source.isEmpty() ? QString() : json(QJsonObject::fromVariantMap(source)),
            text
        ));
        invalidate();
        return value.toVariantMap();
    } catch (const std::exception& e) {
        fail(tr("Could not adopt this script. Check its page tags or reopen a changed source."), e);
        return {};
    }
}
QVariantMap PictureBookController::refreshMaterials(const QVariantMap& value) const {
    const auto previous = QJsonObject::fromVariantMap(value);
    auto composition = previous;
    const auto refresh = [this](const QJsonObject& source) {
        for (const auto& value : backend_.artifacts()) {
            const auto item = value.toMap();
            if (item.value(QStringLiteral("id")).toString() == source["artifact_id"].toString()
                && item.value(QStringLiteral("hasAcceptedRevision")).toBool())
                return pin(item);
        }
        return source;
    };
    auto languages = composition["languages"].toObject();
    for (const auto& key : languages.keys())
        languages[key] = refresh(languages[key].toObject());
    auto pages = composition["pages"].toArray();
    for (int i = 0; i < pages.size(); ++i) {
        auto item = pages[i].toObject();
        if (item.contains("source"))
            item["source"] = refresh(item["source"].toObject());
        pages[i] = item;
    }
    composition["languages"] = languages;
    composition["pages"] = pages;
    return {{"composition", composition.toVariantMap()}, {"changed", composition != previous}};
}
QVariantMap PictureBookController::beginArtworkEdit(const QVariantMap& source) {
    try {
        const auto requested = QJsonObject::fromVariantMap(source);
        bool current = false;
        for (const auto& value : backend_.artifacts()) {
            const auto item = value.toMap();
            if (pin(item) == requested && item.value(QStringLiteral("kindKey")) == "image_raster"
                && item.value(QStringLiteral("hasAcceptedRevision")).toBool())
                current = true;
        }
        if (!current)
            throw std::runtime_error("page artwork is not the current accepted source");
        const auto id = backend_.beginOperatorDraft(
            requested["artifact_id"].toString(),
            QStringLiteral("image.external_edit")
        );
        for (const auto& value : backend_.operatorDrafts()) {
            const auto draft = value.toMap();
            if (draft.value(QStringLiteral("id")) == id
                && draft.value(QStringLiteral("inputArtifactId")) == source.value("artifact_id")
                && draft.value(QStringLiteral("inputRevisionId")) == source.value("revision_id")) {
                error_.clear();
                return draft;
            }
        }
        throw std::runtime_error("artwork stage changed during creation");
    } catch (const std::exception& e) {
        fail(
            tr("Could not edit this page. Choose its current artwork version explicitly and try "
               "again."),
            e
        );
        return {};
    }
}
QVariantMap PictureBookController::adoptArtwork(const QString& output, const QString& candidate) {
    bool matches = false;
    for (const auto& value : backend_.candidates()) {
        const auto item = value.toMap();
        if (item.value(QStringLiteral("id")) == candidate
            && item.value(QStringLiteral("artifactId")) == output
            && item.value(QStringLiteral("kindKey")) == "image_raster")
            matches = true;
    }
    if (!matches || !backend_.acceptCandidate(candidate))
        return {};
    for (const auto& value : backend_.artifacts()) {
        const auto item = value.toMap();
        if (item.value(QStringLiteral("id")) == output
            && item.value(QStringLiteral("hasAcceptedRevision")).toBool())
            return pin(item).toVariantMap();
    }
    return {};
}
QString
PictureBookController::artworkCandidatePreview(const QString& output, const QString& candidate) {
    return backend_.prepareImagePreviews(output, candidate) ? backend_.candidateImageSource()
                                                            : QString();
}
QString PictureBookController::artworkPreview(const QVariantMap& source) {
    if (source.isEmpty())
        return {};
    try {
        auto image = QImage::fromData(
            backend_
                .bookMaterial(json(QJsonObject::fromVariantMap(source)), QStringLiteral("image")),
            "PNG"
        );
        if (image.isNull())
            return {};
        if (image.width() > 2048 || image.height() > 2048)
            image = image.scaled(2048, 2048, Qt::KeepAspectRatio, Qt::SmoothTransformation);
        QByteArray bytes;
        QBuffer buffer(&bytes);
        buffer.open(QIODevice::WriteOnly);
        if (!image.save(&buffer, "PNG"))
            return {};
        return QStringLiteral("data:image/png;base64,") + QString::fromLatin1(bytes.toBase64());
    } catch (const std::exception&) {
        return {};
    }
}
bool PictureBookController::revealNativeExport() {
    return native_export_.isLocalFile() && QFileInfo::exists(native_export_.toLocalFile())
           && QDesktopServices::openUrl(
               QUrl::fromLocalFile(QFileInfo(native_export_.toLocalFile()).absolutePath())
           );
}
bool PictureBookController::exportNative(const QUrl& destination) {
    try {
        if (!destination.isLocalFile() || source_.isEmpty())
            throw std::runtime_error("adopt a book and choose a local destination");
        (void)backend_.bookExport(json(source_), destination.toLocalFile());
        native_export_ = destination;
        status_ = tr("Exported the adopted book as an editable Storybook project.");
        error_.clear();
        emit previewChanged();
        return true;
    } catch (const std::exception& e) {
        fail(tr("Could not export. Choose a new .scproj file outside the Shape project."), e);
        return false;
    }
}
bool PictureBookController::exportPdf(
    const QUrl& destination,
    const QString& language,
    bool includePrintOnly
) {
    try {
        if (!destination.isLocalFile() || source_.isEmpty())
            throw std::runtime_error("adopt a book first");
        const QFileInfo info(destination.toLocalFile());
        const QString parent = info.dir().canonicalPath();
        const QString path = parent + QDir::separator() + info.fileName();
        const QString project = QDir(backend_.bundlePath()).canonicalPath();
        if (parent.isEmpty() || info.suffix().toLower() != QStringLiteral("pdf")
            || QFileInfo::exists(path) || path.startsWith(project + QDir::separator()))
            throw std::runtime_error("new PDF outside project required");
        const auto composition = object(backend_.bookRead(json(source_)));
        const auto prepared = preview(composition);
        QTemporaryFile temporary(parent + QStringLiteral("/.shape-book-XXXXXX.pdf"));
        if (!temporary.open())
            throw std::runtime_error("cannot create PDF temporary file");
        const auto temporary_path = temporary.fileName();
        temporary.close();
        int written = 0;
        {
            QPdfWriter writer(temporary_path);
            writer.setResolution(96);
            const QSizeF size(
                composition["canvas_width"].toInt() * 25.4 / 96.,
                composition["canvas_height"].toInt() * 25.4 / 96.
            );
            writer.setPageSize(QPageSize(size, QPageSize::Millimeter));
            writer.setPageMargins(QMarginsF(0, 0, 0, 0));
            writer.setTitle(composition["name"].toString());
            writer.setCreator(QStringLiteral("Shape"));
            QPainter painter(&writer);
            if (!painter.isActive())
                throw std::runtime_error("PDF writer unavailable");
            const auto pages = composition["pages"].toArray();
            for (int i = 0; i < pages.size(); ++i) {
                if (!includePrintOnly && pages[i].toObject()["print_only"].toBool())
                    continue;
                if (written && !writer.newPage())
                    throw std::runtime_error("PDF page failed");
                const auto book_page = page(prepared, language, i);
                painter.save();
                painter.scale(
                    writer.width() / book_page.canvas.width(),
                    writer.height() / book_page.canvas.height()
                );
                const auto layout = picture_book::paint(painter, book_page);
                painter.restore();
                if (!layout.fits || layout.missing_glyphs)
                    throw std::runtime_error("text overflow or missing glyphs");
                ++written;
            }
            painter.end();
        }
        if (!written || QFileInfo(temporary_path).size() == 0)
            throw std::runtime_error("empty PDF");
        std::error_code error;
        std::filesystem::create_hard_link(temporary_path.toStdString(), path.toStdString(), error);
        if (error)
            throw std::runtime_error(error.message());
        status_ = tr("Exported %1 pages from the adopted book.").arg(written);
        error_.clear();
        emit previewChanged();
        return true;
    } catch (const std::exception& e) {
        fail(
            tr("Could not export PDF. Use a new file outside the project and check every page for "
               "text overflow or missing fonts."),
            e
        );
        return false;
    }
}

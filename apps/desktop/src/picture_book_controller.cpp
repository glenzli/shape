#include "picture_book_controller.hpp"
#include "desktop_backend.hpp"
#include <QBuffer>
#include <QDebug>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPageLayout>
#include <QPageSize>
#include <QPdfWriter>
#include <QTemporaryFile>
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
    : QObject(parent), backend_(backend) {
    connect(&backend_, &DesktopBackend::projectChanged, this, [this] {
        if (project_id_ != backend_.projectId()) {
            project_id_ = backend_.projectId();
            beginNew();
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
    emit reviewChanged();
    emit previewChanged();
}
void PictureBookController::beginNew() {
    invalidate();
    source_ = {};
    composition_ = {
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
    emit documentChanged();
}
bool PictureBookController::load(const QString& id) {
    try {
        QJsonObject source;
        for (const auto& value : backend_.artifacts()) {
            const auto item = value.toMap();
            if (item.value(QStringLiteral("id")).toString() == id
                && item.value(QStringLiteral("kindKey")).toString()
                       == QStringLiteral("picture_book"))
                source = pin(item);
        }
        if (source.isEmpty())
            throw std::runtime_error("book not found");
        const auto composition = object(backend_.bookRead(json(source)));
        const auto prepared = preview(composition);
        invalidate();
        source_ = source;
        composition_ = composition;
        preview_ = prepared;
        emit documentChanged();
        return showPage(composition_["default_language"].toString(), 0);
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
        const auto candidate =
            backend_.bookPropose(source_.isEmpty() ? QString() : json(source_), json(composition));
        candidate_id_ = candidate;
        for (const auto& c : backend_.candidates())
            if (c.toMap().value(QStringLiteral("id")).toString() == candidate)
                candidate_artifact_id_ = c.toMap().value(QStringLiteral("artifactId")).toString();
        composition_ = composition;
        preview_ = prepared;
        emit reviewChanged();
        return showPage(composition_["default_language"].toString(), 0);
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
QVariantMap PictureBookController::refreshMaterials(const QVariantMap& value) {
    QJsonObject composition = QJsonObject::fromVariantMap(value);
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
    invalidate();
    return composition.toVariantMap();
}
bool PictureBookController::exportNative(const QUrl& destination) {
    try {
        if (!destination.isLocalFile() || source_.isEmpty())
            throw std::runtime_error("adopt a book and choose a local destination");
        (void)backend_.bookExport(json(source_), destination.toLocalFile());
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

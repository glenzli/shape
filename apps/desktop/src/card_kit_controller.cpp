#include "card_kit_controller.hpp"
#include "desktop_backend.hpp"
#include <QClipboard>
#include <QCryptographicHash>
#include <QDebug>
#include <QFileInfo>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QSettings>
#include <stdexcept>

CardKitController::CardKitController(DesktopBackend& backend, QObject* parent)
    : CardKitController(backend, QString{}, parent) {}
CardKitController::CardKitController(
    DesktopBackend& backend,
    const QString& settingsPath,
    QObject* parent
)
    : QObject(parent), backend_(backend),
      settings_(
          settingsPath.isEmpty() ? std::make_unique<QSettings>()
                                 : std::make_unique<QSettings>(settingsPath, QSettings::IniFormat)
      ),
      project_id_(backend.projectId()), bundle_path_(backend.bundlePath()) {
    connect(&backend_, &DesktopBackend::projectChanged, this, [this] {
        if (project_id_ == backend_.projectId() && bundle_path_ == backend_.bundlePath())
            return;
        project_id_ = backend_.projectId();
        bundle_path_ = backend_.bundlePath();
        clear();
    });
}
CardKitController::~CardKitController() = default;
QString CardKitController::artworkEditKey(const QString& cardId) const {
    const auto path = QFileInfo(bundle_path_).canonicalFilePath();
    const auto digest = QCryptographicHash::hash(path.toUtf8(), QCryptographicHash::Sha256).toHex();
    return QStringLiteral("card-kit/artwork-edits-by-bundle/%1/%2/%3/%4")
        .arg(
            QString::fromLatin1(digest),
            project_id_,
            draft_["artifact_id"].toString(),
            QString::fromLatin1(cardId.toUtf8().toHex())
        );
}
QString CardKitController::legacyArtworkEditKey(const QString& cardId) const {
    return QStringLiteral("card-kit/artwork-edits/%1/%2/%3")
        .arg(
            project_id_,
            draft_["artifact_id"].toString(),
            QString::fromLatin1(cardId.toUtf8().toHex())
        );
}
bool CardKitController::rememberArtworkEdit(
    const QString& cardId,
    const QVariantMap& source,
    const QVariantMap& stage
) {
    if (cardId.isEmpty() || draft_["artifact_id"].toString().isEmpty()
        || source.value("artifact_id").toString().isEmpty()
        || source.value("revision_id").toString().isEmpty()
        || stage.value("id").toString().isEmpty()
        || stage.value("contextArtifactId").toString().isEmpty())
        return false;
    const QJsonObject record{
        {"schema", "shape.card-artwork-local-session/1"},
        {"project_path", bundle_path_},
        {"source", QJsonObject::fromVariantMap(source)},
        {"operator_draft_id", stage.value("id").toString()},
        {"output_artifact_id", stage.value("contextArtifactId").toString()}
    };
    settings_->setValue(
        artworkEditKey(cardId),
        QString::fromUtf8(QJsonDocument(record).toJson(QJsonDocument::Compact))
    );
    settings_->sync();
    return settings_->status() == QSettings::NoError;
}
QVariantMap CardKitController::recoverArtworkEdit(const QString& cardId) {
    settings_->sync();
    auto serialized = settings_->value(artworkEditKey(cardId)).toString();
    bool legacy = serialized.isEmpty();
    if (legacy)
        serialized = settings_->value(legacyArtworkEditKey(cardId)).toString();
    const auto record = QJsonDocument::fromJson(serialized.toUtf8()).object();
    if (record["schema"] != "shape.card-artwork-local-session/1"
        || record["project_path"].toString() != bundle_path_)
        return {};
    const auto source = record["source"].toObject();
    bool matches_card = false, current_source = false;
    for (const auto& item : draft_["composition"].toObject()["cards"].toArray()) {
        const auto card = item.toObject();
        if (card["id"].toString() == cardId && card["artwork"].toObject() == source)
            matches_card = true;
    }
    for (const auto& item : backend_.artifacts()) {
        const auto artifact = item.toMap();
        if (artifact.value("id") == source["artifact_id"].toString()
            && artifact.value("acceptedRevisionId") == source["revision_id"].toString())
            current_source = true;
    }
    if (!matches_card || !current_source)
        return {};
    for (const auto& item : backend_.operatorDrafts()) {
        const auto stage = item.toMap();
        if (stage.value("id") == record["operator_draft_id"].toString()
            && stage.value("contextArtifactId") == record["output_artifact_id"].toString()
            && stage.value("operatorTypeKey") == "image.external_edit"
            && stage.value("inputArtifactId") == source["artifact_id"].toString()
            && stage.value("inputRevisionId") == source["revision_id"].toString()) {
            if (legacy) {
                settings_->setValue(artworkEditKey(cardId), serialized);
                settings_->remove(legacyArtworkEditKey(cardId));
                settings_->sync();
            }
            return {{"source", source.toVariantMap()}, {"stage", stage}};
        }
    }
    return {};
}
void CardKitController::forgetArtworkEdit(const QString& cardId) {
    settings_->remove(artworkEditKey(cardId));
    const auto legacy = legacyArtworkEditKey(cardId);
    const auto record =
        QJsonDocument::fromJson(settings_->value(legacy).toString().toUtf8()).object();
    if (record["project_path"].toString() == bundle_path_)
        settings_->remove(legacy);
    settings_->sync();
}
void CardKitController::clear() {
    draft_ = {};
    source_ = {};
    drafts_.clear();
    accepted_current_ = false;
    error_.clear();
    status_.clear();
    invalidateReview();
    emit documentChanged();
    emit draftsChanged();
    emit statusChanged();
}
QJsonObject CardKitController::action(QJsonObject request) {
    QJsonParseError parse_error;
    const auto text = backend_.cardKitAction(
        QString::fromUtf8(QJsonDocument(request).toJson(QJsonDocument::Compact))
    );
    const auto result = QJsonDocument::fromJson(text.toUtf8(), &parse_error);
    if (parse_error.error != QJsonParseError::NoError || !result.isObject())
        throw std::runtime_error("invalid card kit response");
    return result.object();
}
bool CardKitController::fail(const char* message, const std::exception& error) {
    qWarning().noquote() << "card kit:" << error.what();
    error_ = QString::fromUtf8(message);
    status_.clear();
    emit statusChanged();
    return false;
}
void CardKitController::invalidateReview() {
    review_digest_.clear();
    preview_source_.clear();
    preview_ = {};
    emit previewChanged();
}
void CardKitController::setDraft(const QJsonObject& value) {
    if (value["artifact_id"].toString().isEmpty() || value["draft_digest"].toString().isEmpty()
        || !value["composition"].isObject())
        throw std::runtime_error("incomplete card kit draft");
    draft_ = value;
    error_.clear();
    status_.clear();
    updateSource();
    for (auto& saved : drafts_) {
        if (saved.toMap().value("artifact_id").toString() == draft_["artifact_id"].toString()) {
            saved = draft_.toVariantMap();
            emit draftsChanged();
            break;
        }
    }
    emit documentChanged();
    emit statusChanged();
}
void CardKitController::updateSource() {
    source_ = {};
    for (const auto& value : backend_.artifacts()) {
        const auto item = value.toMap();
        if (item.value("id").toString() == draft_["artifact_id"].toString()
            && item.value("hasAcceptedRevision").toBool()) {
            source_ = {
                {"artifact_id", item.value("id").toString()},
                {"revision_id", item.value("acceptedRevisionId").toString()}
            };
            break;
        }
    }
}
bool CardKitController::refresh() {
    try {
        template_keys_.clear();
        const auto templates = QJsonDocument::fromJson(backend_.workflowTemplates().toUtf8())
                                   .object()["templates"]
                                   .toArray();
        for (const auto& item : templates)
            template_keys_.push_back(item.toObject()["id"].toString());
        drafts_ = action({{"action", "draft_list"}})["drafts"].toArray().toVariantList();
        emit draftsChanged();
        return true;
    } catch (const std::exception& e) {
        return fail(
            QT_TR_NOOP("Could not load saved card drafts. Your current work is still here."),
            e
        );
    }
}
bool CardKitController::beginNew() {
    try {
        const QJsonObject card{
            {"id", "card-1"},
            {"title", ""},
            {"card_type", ""},
            {"cost", ""},
            {"rules", ""},
            {"stats", ""},
            {"flavor", ""},
            {"art_prompt", ""},
            {"quantity", 1},
            {"artwork", QJsonValue::Null},
            {"text_material", QJsonValue::Null}
        };
        const QJsonObject composition{
            {"name", tr("New card materials")},
            {"brief", ""},
            {"style", ""},
            {"canvas_width", 750},
            {"canvas_height", 1050},
            {"cards", QJsonArray{card}}
        };
        auto result = action({{"action", "draft_new"}, {"composition", composition}});
        invalidateReview();
        accepted_current_ = false;
        setDraft(result["draft"].toObject());
        refresh();
        return true;
    } catch (const std::exception& e) {
        return fail(
            QT_TR_NOOP("Could not create a card draft. Check that this project is writable."),
            e
        );
    }
}
bool CardKitController::load(const QString& artifactId) {
    try {
        auto result = action({{"action", "draft_load"}, {"artifact_id", artifactId}});
        invalidateReview();
        accepted_current_ = false;
        setDraft(result["draft"].toObject());
        if (!source_.isEmpty()) {
            const auto accepted = action({{"action", "read"}, {"source", source_}});
            accepted_current_ =
                accepted["composition"].toObject() == draft_["composition"].toObject();
            if (accepted_current_) {
                preview_ = accepted["preview"].toObject();
                preview_source_ =
                    "data:image/svg+xml;base64,"
                    + QString::fromLatin1(preview_["svg"].toString().toUtf8().toBase64());
                emit previewChanged();
            }
        }
        return true;
    } catch (const std::exception& e) {
        return fail(
            QT_TR_NOOP("Could not open these cards. The saved version has not changed."),
            e
        );
    }
}
bool CardKitController::save(const QVariantMap& composition) {
    if (draft_.isEmpty())
        return false;
    try {
        const auto value = QJsonObject::fromVariantMap(composition);
        if (value == draft_["composition"].toObject())
            return true;
        auto result = action(
            {{"action", "draft_save"},
             {"artifact_id", draft_["artifact_id"]},
             {"draft_digest", draft_["draft_digest"]},
             {"composition", value}}
        );
        invalidateReview();
        accepted_current_ = false;
        setDraft(result["draft"].toObject());
        status_ = QString::fromUtf8(QT_TR_NOOP("Draft saved in this project."));
        emit statusChanged();
        return true;
    } catch (const std::exception& e) {
        return fail(
            QT_TR_NOOP(
                "Could not save this draft. Keep this window open and copy your text before "
                "reopening the project."
            ),
            e
        );
    }
}
bool CardKitController::review(const QString& cardId) {
    if (draft_.isEmpty())
        return false;
    try {
        auto result = action(
            {{"action", "preview"},
             {"artifact_id", draft_["artifact_id"]},
             {"draft_digest", draft_["draft_digest"]},
             {"card_id", cardId}}
        );
        preview_ = result["preview"].toObject();
        review_digest_ = result["review_digest"].toString();
        preview_source_ = "data:image/svg+xml;base64,"
                          + QString::fromLatin1(preview_["svg"].toString().toUtf8().toBase64());
        error_.clear();
        status_ = QString::fromUtf8(QT_TR_NOOP("Check every card before using this version."));
        emit previewChanged();
        emit statusChanged();
        return !review_digest_.isEmpty();
    } catch (const std::exception& e) {
        invalidateReview();
        return fail(
            QT_TR_NOOP(
                "Could not check these cards. Check the text, artwork and current materials, then "
                "try again."
            ),
            e
        );
    }
}
bool CardKitController::accept() {
    if (!hasReview())
        return false;
    try {
        const auto result = action(
            {{"action", "accept"},
             {"artifact_id", draft_["artifact_id"]},
             {"draft_digest", draft_["draft_digest"]},
             {"review_digest", review_digest_}}
        );
        setDraft(result["draft"].toObject());
        source_ = result["source"].toObject();
        accepted_current_ = true;
        review_digest_.clear();
        status_ = QString::fromUtf8(QT_TR_NOOP("This card version is now accepted."));
        emit documentChanged();
        emit previewChanged();
        emit statusChanged();
        refresh();
        return true;
    } catch (const std::exception& e) {
        invalidateReview();
        accepted_current_ = false;
        return fail(
            QT_TR_NOOP(
                "The cards or a material changed. Reopen the draft and check again before using "
                "this version."
            ),
            e
        );
    }
}
bool CardKitController::showCard(const QString& cardId) {
    if (!accepted_current_ || source_.isEmpty())
        return hasReview() ? review(cardId) : false;
    try {
        preview_ = action({{"action", "read"}, {"source", source_}, {"card_id", cardId}})["preview"]
                       .toObject();
        preview_source_ = "data:image/svg+xml;base64,"
                          + QString::fromLatin1(preview_["svg"].toString().toUtf8().toBase64());
        emit previewChanged();
        return preview_["selected_card_id"].toString() == cardId;
    } catch (const std::exception& e) {
        invalidateReview();
        return fail(
            QT_TR_NOOP("Could not open these cards. The saved version has not changed."),
            e
        );
    }
}
bool CardKitController::canExport() const {
    return accepted_current_ && !source_.isEmpty() && preview_["ready_to_export"].toBool();
}
bool CardKitController::exportFiles(const QUrl& destination) {
    if (!canExport() || !destination.isLocalFile())
        return false;
    try {
        action(
            {{"action", "export"}, {"source", source_}, {"destination", destination.toLocalFile()}}
        );
        error_.clear();
        status_ = QString::fromUtf8(QT_TR_NOOP("Exported accepted card files for inspection."));
        emit statusChanged();
        return true;
    } catch (const std::exception& e) {
        return fail(QT_TR_NOOP("Could not export the cards. Choose a new writable ZIP file."), e);
    }
}

void CardKitController::copyDraft(const QVariantMap& composition) {
    QGuiApplication::clipboard()->setText(
        QString::fromUtf8(
            QJsonDocument(QJsonObject::fromVariantMap(composition)).toJson(QJsonDocument::Indented)
        )
    );
    status_ = QString::fromUtf8(QT_TR_NOOP("Copied the complete card draft."));
    emit statusChanged();
}

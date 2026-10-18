#include "legacy_work_controller.hpp"
#include "desktop_backend.hpp"
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QProcess>
#include <QSettings>
#include <QStandardPaths>
#include <QUuid>
namespace {
QJsonObject object(const QString& value) {
    return QJsonDocument::fromJson(value.toUtf8()).object();
}
QString json(const QJsonObject& value) {
    return QString::fromUtf8(QJsonDocument(value).toJson(QJsonDocument::Compact));
}
QString resolve(const QString& selected) {
    QFileInfo info(selected);
#ifdef Q_OS_MACOS
    if (info.isDir() && info.suffix().toLower() == "app") {
        QProcess plist;
        plist.start(
            "/usr/bin/plutil",
            {"-extract",
             "CFBundleExecutable",
             "raw",
             "-o",
             "-",
             info.filePath() + "/Contents/Info.plist"}
        );
        if (!plist.waitForFinished(1500) || plist.exitCode() != 0)
            return {};
        const auto name = QString::fromUtf8(plist.readAllStandardOutput()).trimmed();
        if (name.isEmpty() || name != QFileInfo(name).fileName())
            return {};
        info.setFile(info.filePath() + "/Contents/MacOS/" + name);
    }
#endif
    return info.isAbsolute() && info.isFile() && info.isExecutable() ? info.canonicalFilePath()
                                                                     : QString();
}
QString executable(
    QSettings& settings,
    const QString& app,
    const QString& environment,
    const QString& fallback
) {
    const QString configured = settings.value("specialists/" + app).toString();
    if (!configured.isEmpty())
        return resolve(configured);
    const QString overridden = qEnvironmentVariable(environment.toUtf8().constData());
    if (!overridden.isEmpty())
        return resolve(overridden);
    const auto path = QStandardPaths::findExecutable(app);
    return !path.isEmpty() ? resolve(path) : resolve(fallback);
}
QJsonObject artifact(DesktopBackend& backend, const QString& id) {
    for (const auto& item : backend.artifacts()) {
        const auto value = QJsonObject::fromVariantMap(item.toMap());
        if (value["id"].toString() == id)
            return value;
    }
    return {};
}
bool launch(const QString& executable, const QStringList& arguments) {
    QProcess application;
    application.setProgram(executable);
    application.setArguments(arguments);
    application.setStandardInputFile(QProcess::nullDevice());
    application.setStandardOutputFile(QProcess::nullDevice());
    application.setStandardErrorFile(QProcess::nullDevice());
    return application.startDetached();
}
} // namespace
LegacyWorkController::LegacyWorkController(
    DesktopBackend& backend,
    const QString& settingsPath,
    QObject* parent
)
    : QObject(parent), backend_(backend),
      settings_(std::make_unique<QSettings>(settingsPath, QSettings::IniFormat)) {
    connect(&backend_, &DesktopBackend::projectChanged, this, [this] {
        if (artifact_id_.isEmpty())
            return;
        if (!projectCurrent()) {
            artifact_id_.clear();
            work_ = {};
            selected_ = {};
            snapshots_.clear();
            materials_.clear();
            exported_token_.clear();
            last_export_.clear();
            vox_result_ = {};
            emit changed();
        } else {
            const auto id = artifact_id_, node = draft_id_, choice = snapshot_key_;
            if (openWork(id, node) && choice != snapshot_key_)
                selectSnapshot(choice);
        }
    });
}
LegacyWorkController::~LegacyWorkController() = default;
bool LegacyWorkController::projectCurrent() const {
    return backend_.projectOpen() && backend_.projectId() == project_id_
           && QFileInfo(backend_.bundlePath()).canonicalFilePath() == bundle_;
}
bool LegacyWorkController::fail(const QString& message) {
    error_ = message;
    status_.clear();
    emit changed();
    return false;
}
QString LegacyWorkController::selectionToken() const {
    return projectCurrent() ? json(
                                  {{"bundle", bundle_},
                                   {"project", project_id_},
                                   {"artifact", artifact_id_},
                                   {"draft", draft_id_},
                                   {"selection", selected_}}
                              )
                            : QString();
}
QString LegacyWorkController::storybookExecutable() const {
    return executable(
        *settings_,
        "storybook-co-editor",
        "SHAPE_STORYBOOK_BINARY",
        "/Applications/storybook-co-editor.app"
    );
}
QString LegacyWorkController::voxExecutable() const {
    return executable(*settings_, "vox-desktop", "SHAPE_VOX_BINARY", "/Applications/Vox.app");
}
void LegacyWorkController::addMaterial(const QJsonObject& pin, const QString& role) {
    if (pin["artifact_id"].toString().isEmpty() || pin["revision_id"].toString().isEmpty())
        return;
    for (const auto& item : materials_)
        if (item.toMap()["source"].toMap() == pin.toVariantMap())
            return;
    const auto value = artifact(backend_, pin["artifact_id"].toString());
    materials_.append(
        QVariantMap{
            {"source", pin.toVariantMap()},
            {"role", role},
            {"name", value["name"].toString()},
            {"kind", value["kindKey"].toString()}
        }
    );
}
bool LegacyWorkController::openWork(const QString& id, const QString& nodeId) {
    if (!backend_.projectOpen())
        return false;
    try {
        catalog_ = object(backend_.legacySnapshot());
        bundle_ = QFileInfo(backend_.bundlePath()).canonicalFilePath();
        project_id_ = backend_.projectId();
        artifact_id_ = id;
        draft_id_ = nodeId;
        error_.clear();
        status_.clear();
        work_ = {};
        selected_ = {};
        draft_ = {};
        composition_ = {};
        snapshots_.clear();
        materials_.clear();
        for (const auto& entry : catalog_["legacy_artifacts"].toArray()) {
            const auto value = entry.toObject();
            if (value["artifact_id"].toString() != id)
                continue;
            work_ = value;
            const auto kind = value["kind"].toString();
            const auto accepted = value["accepted_revision"].toString();
            if (!accepted.isEmpty())
                snapshots_.append(
                    QVariantMap{{"key", "accepted"}, {"label", tr("Accepted version")}}
                );
            const auto drafts =
                object(
                    kind == "card_kit" ? backend_.cardKitAction(json({{"action", "draft_list"}}))
                                       : backend_.bookDraftAction(json({{"action", "draft_list"}}))
                )["drafts"]
                    .toArray();
            for (const auto& item : drafts)
                if (item.toObject()["artifact_id"].toString() == id)
                    draft_ = item.toObject();
            if (!draft_.isEmpty())
                snapshots_.append(
                    QVariantMap{{"key", "draft"}, {"label", tr("Saved unfinished draft")}}
                );
            return selectSnapshot(!draft_.isEmpty() ? "draft" : "accepted");
        }
        QJsonObject speech;
        for (const auto& entry : catalog_["speech_drafts"].toArray()) {
            const auto value = entry.toObject();
            if ((!nodeId.isEmpty() && value["draft_id"].toString() == nodeId)
                || (nodeId.isEmpty() && value["target_artifact_id"].toString() == id)) {
                speech = value;
                break;
            }
        }
        const auto target = artifact(backend_, id);
        QString textId =
            !speech.isEmpty() ? speech["input"].toObject()["artifact_id"].toString() : id;
        if (target["kindKey"] == "audio_clip" && speech.isEmpty()) {
            const auto inputs = target["transformationInputArtifactIds"].toArray();
            if (!inputs.isEmpty())
                textId = inputs.first().toString();
        }
        QJsonObject text;
        for (const auto& entry : catalog_["texts"].toArray())
            if (entry.toObject()["artifact_id"].toString() == textId)
                text = entry.toObject();
        if (text.isEmpty())
            return fail(
                tr("This saved work is unavailable. Select another item or reopen the project.")
            );
        selected_ = {
            {"text_artifact_id", textId},
            {"expected_text_head", text["accepted_revision"]}
        };
        if (text["authoring_graph_digest"].isString())
            selected_["authoring_graph_digest"] = text["authoring_graph_digest"];
        if (!speech.isEmpty()) {
            selected_["speech_target_artifact_id"] = speech["target_artifact_id"];
            selected_["speech_graph_digest"] = speech["graph_digest"];
            draft_id_ = speech["draft_id"].toString();
            addMaterial(speech["input"].toObject(), tr("Original script"));
        }
        if (target["kindKey"] == "audio_clip"
            && !target["acceptedRevisionId"].toString().isEmpty()) {
            const QJsonObject audio{
                {"artifact_id", id},
                {"revision_id", target["acceptedRevisionId"]}
            };
            selected_["accepted_audio"] = audio;
            addMaterial(audio, tr("Accepted recording"));
        }
        if (text["accepted_revision"].isString())
            addMaterial(
                {{"artifact_id", textId}, {"revision_id", text["accepted_revision"]}},
                tr("Text")
            );
        work_ = {
            {"artifact_id", id},
            {"name", target["name"]},
            {"kind", "speech"},
            {"has_authoring_draft", text["has_authoring_draft"]},
            {"has_speech_draft", !speech.isEmpty()}
        };
        snapshot_key_ = "speech";
        emit changed();
        return true;
    } catch (const std::exception&) {
        return fail(tr("Could not read this saved work. The Shape project is unchanged."));
    }
}
bool LegacyWorkController::selectSnapshot(const QString& key) {
    if (!projectCurrent() || (key != "draft" && key != "accepted"))
        return false;
    try {
        const auto kind = work_["kind"].toString();
        materials_.clear();
        composition_ = {};
        selected_ = {};
        if (key == "draft" && !draft_.isEmpty()) {
            selected_ = {{"artifact_id", artifact_id_}, {"draft_digest", draft_["draft_digest"]}};
            composition_ = kind == "card_kit"
                               ? draft_["composition"].toObject()
                               : draft_["state"].toObject()["composition"].toObject();
            work_["unfinished_script"] =
                kind == "picture_book" ? draft_["state"].toObject()["script"] : QJsonValue();
        } else if (key == "accepted" && work_["accepted_revision"].isString()) {
            selected_ = {
                {"artifact_id", artifact_id_},
                {"revision_id", work_["accepted_revision"]}
            };
            composition_ =
                kind == "card_kit"
                    ? object(
                          backend_.cardKitAction(json({{"action", "read"}, {"source", selected_}}))
                      )["composition"]
                          .toObject()
                    : object(backend_.bookRead(json(selected_)));
            work_["unfinished_script"] = QJsonValue();
        } else
            return false;
        snapshot_key_ = key;
        work_["deck_selection"] = key == "draft"
                                      ? selected_["draft_digest"].toString()
                                      : "accepted:" + selected_["revision_id"].toString();
        if (kind == "card_kit") {
            const auto cards = composition_["cards"].toArray();
            work_["item_count"] = cards.size();
            QStringList lines;
            for (const auto& item : cards) {
                const auto card = item.toObject();
                lines.append(card["title"].toString());
                if (!card["rules"].toString().isEmpty())
                    lines.append(card["rules"].toString());
                lines.append(QString());
                addMaterial(card["artwork"].toObject(), tr("Card image"));
                addMaterial(card["text_material"].toObject(), tr("Card text"));
            }
            work_["summary"] = lines.join('\n');
        } else {
            const auto pages = composition_["pages"].toArray();
            work_["item_count"] = pages.size();
            for (const auto& page : pages)
                addMaterial(page.toObject()["source"].toObject(), tr("Page image"));
            for (const auto& language : composition_["languages"].toObject())
                addMaterial(language.toObject(), tr("Language script"));
        }
        work_["brief"] = composition_["brief"];
        work_["style"] = composition_["style"];
        emit changed();
        return true;
    } catch (const std::exception&) {
        return fail(tr("Could not read that snapshot. The Shape project is unchanged."));
    }
}
bool LegacyWorkController::exportBook(const QUrl& destination, const QString& token) {
    if (token.isEmpty() || token != selectionToken() || !destination.isLocalFile()
        || work_["kind"] != "picture_book")
        return fail(tr("The selected work changed. Choose the snapshot and export again."));
    try {
        QJsonObject receipt;
        if (snapshot_key_ == "draft")
            receipt = object(backend_.bookDraftExport(
                artifact_id_,
                selected_["draft_digest"].toString(),
                destination.toLocalFile()
            ));
        else
            receipt = object(backend_.bookExport(json(selected_), destination.toLocalFile()));
        last_export_ = destination.toLocalFile();
        exported_token_ = token;
        status_ = receipt["unaccepted_script_preserved"].toBool()
                      ? tr("Exported an editable Storybook project with the unfinished script "
                           "preserved for review. It has not been accepted.")
                      : tr("Exported an editable Storybook project. Open that file in Storybook Co "
                           "Editor.");
        if (!receipt["warnings"].toArray().isEmpty())
            status_ += " " + tr("Some content still needs review in Storybook.");
        error_.clear();
        emit changed();
        return true;
    } catch (const std::exception&) {
        return fail(tr("Could not export. Choose a new .scproj file outside the Shape project."));
    }
}
bool LegacyWorkController::exportVox(const QUrl& parentDirectory, const QString& token) {
    if (token.isEmpty() || token != selectionToken() || !parentDirectory.isLocalFile()
        || work_["kind"] != "speech")
        return fail(tr("The selected work changed. Choose the snapshot and export again."));
    try {
        QJsonObject request = selected_;
        request["request_id"] = QUuid::createUuid().toString(QUuid::WithoutBraces);
        request["destination"] = QDir(parentDirectory.toLocalFile())
                                     .filePath("Vox-transfer-" + request["request_id"].toString());
        vox_result_ = object(backend_.exportVoxTransfer(json(request)));
        last_export_ = vox_result_["manifest_path"].toString();
        exported_token_ = token;
        error_.clear();
        status_ =
            tr("Saved the script, voice settings and existing audio for Vox. The Shape project is "
               "unchanged.");
        emit changed();
        return true;
    } catch (const std::exception&) {
        return fail(
            tr("Could not export the Vox transfer. Reload the saved work and choose a folder "
               "outside the project.")
        );
    }
}
bool LegacyWorkController::openStorybook() {
    const auto program = storybookExecutable();
    if (program.isEmpty() || !launch(program, {}))
        return fail(
            tr("Choose an installed Storybook Co Editor application, then open the exported "
               ".scproj file there.")
        );
    return true;
}
bool LegacyWorkController::openVox(const QUrl& newProject, const QString& token) {
    if (token.isEmpty() || token != selectionToken() || !newProject.isLocalFile()
        || work_["kind"] != "speech")
        return fail(tr("The selected text changed. Choose the text and continue again."));
    const auto path = newProject.toLocalFile(), program = voxExecutable();
    if (program.isEmpty())
        return fail(
            tr("Choose an installed Vox application. You can also export the transfer files "
               "separately.")
        );
    QProcess capability;
    capability.setProcessChannelMode(QProcess::SeparateChannels);
    capability.start(QFileInfo(program).dir().filePath("vox-cli"), {"capabilities"});
    if (!capability.waitForFinished(2000)) {
        capability.kill();
        capability.waitForFinished(500);
    }
    const auto support = QJsonDocument::fromJson(capability.readAllStandardOutput()).object();
    if (capability.exitStatus() != QProcess::NormalExit || capability.exitCode() != 0
        || support["schema"] != "vox.capabilities/1"
        || !support["operations"].toArray().contains("import_shape_transfer"))
        return fail(
            tr("This Vox version cannot import Shape transfers. Update Vox or export the transfer "
               "files separately.")
        );
    const auto project = QFileInfo(backend_.bundlePath()).canonicalFilePath();
    const QFileInfo destination(path);
    const auto parent = destination.dir().canonicalPath();
    if (parent.isEmpty() || destination.exists() || destination.suffix().toLower() != "vox"
        || parent == project || parent.startsWith(project + '/'))
        return fail(tr("Choose a new .vox project outside the Shape project."));
    if (!hasVoxTransfer()) {
        const auto staging = QFileInfo(settings_->fileName()).dir().filePath("handoffs/vox");
        if (!QDir().mkpath(staging) || !exportVox(QUrl::fromLocalFile(staging), token))
            return false;
    }
    if (token != selectionToken())
        return fail(
            tr("The text changed after export. The saved transfer is available in its folder.")
        );
    if (!launch(
            program,
            {"--import-shape-transfer",
             last_export_,
             "--project",
             parent + '/' + destination.fileName(),
             "--expected-blake3",
             vox_result_["manifest_blake3"].toString()}
        ))
        return fail(tr("Vox could not be started. The transfer files remain available."));
    status_ = tr("Vox started. It will check the transfer before creating the new project.");
    emit changed();
    return true;
}
bool LegacyWorkController::chooseStorybook(const QUrl& path) {
    if (!path.isLocalFile() || resolve(path.toLocalFile()).isEmpty())
        return fail(tr("Choose a runnable application."));
    settings_->setValue(
        "specialists/storybook-co-editor",
        QDir::cleanPath(QFileInfo(path.toLocalFile()).absoluteFilePath())
    );
    settings_->sync();
    error_.clear();
    emit changed();
    return true;
}
bool LegacyWorkController::chooseVox(const QUrl& path) {
    if (!path.isLocalFile() || resolve(path.toLocalFile()).isEmpty())
        return fail(tr("Choose a runnable application."));
    settings_->setValue(
        "specialists/vox-desktop",
        QDir::cleanPath(QFileInfo(path.toLocalFile()).absoluteFilePath())
    );
    settings_->sync();
    error_.clear();
    emit changed();
    return true;
}
bool LegacyWorkController::revealExport() {
    return !last_export_.isEmpty()
           && QDesktopServices::openUrl(
               QUrl::fromLocalFile(QFileInfo(last_export_).absolutePath())
           );
}
void LegacyWorkController::retranslate() {
    if (artifact_id_.isEmpty()) {
        emit changed();
        return;
    }
    const auto choice = snapshot_key_;
    if (openWork(artifact_id_, draft_id_) && choice != snapshot_key_)
        selectSnapshot(choice);
}

bool LegacyWorkController::openText(const QString& id, const QString& expectedHead) {
    const auto text = artifact(backend_, id);
    if (text["kindKey"] != "text_document" || text["acceptedRevisionId"].toString() != expectedHead)
        return fail(
            tr("The selected text changed. Open its current version before continuing in Vox.")
        );
    return openWork(id);
}

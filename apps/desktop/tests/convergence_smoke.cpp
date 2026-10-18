//! Linked, read-only specialist compatibility and general-writing navigation.
#include "convergence_smoke.hpp"
#include "deck_handoff_smoke.hpp"
#include "desktop_backend.hpp"
#include "legacy_work_controller.hpp"
#include "text_authoring_smoke.hpp"
#include "ui_preferences.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJSValue>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMouseEvent>
#include <QProcess>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QThread>
#include <iostream>
#include <stdexcept>
namespace {
void require(bool condition, const char* message) {
    if (!condition)
        throw std::runtime_error(message);
}
void settle() {
    for (int i = 0; i < 8; ++i) {
        QCoreApplication::processEvents();
        QThread::msleep(15);
    }
}
QVariant value(QVariant v) {
    return v.metaType() == QMetaType::fromType<QJSValue>() ? v.value<QJSValue>().toVariant() : v;
}
QVariant evaluate(QObject& root, const QString& code) {
    QQmlExpression expression(QQmlEngine::contextForObject(&root), &root, code);
    auto result = expression.evaluate();
    require(!expression.hasError(), qPrintable(expression.error().toString()));
    return value(result);
}
QJsonObject object(const QString& text) {
    return QJsonDocument::fromJson(text.toUtf8()).object();
}
void save(const QString& path, const QJsonObject& data) {
    QFile file(path);
    require(file.open(QIODevice::WriteOnly | QIODevice::NewOnly), "new evidence file");
    require(file.write(QJsonDocument(data).toJson()) > 0, "write evidence");
}
void click(QObject& root, const QString& name) {
    auto* item = root.findChild<QQuickItem*>(name);
    require(
        item && item->isVisible() && item->isEnabled() && item->window(),
        qPrintable("visible action: " + name)
    );
    const auto point = item->mapToScene(QPointF(item->width() / 2, item->height() / 2));
    auto* window = item->window();
    require(
        point.x() >= 0 && point.y() >= 0 && point.x() < window->width()
            && point.y() < window->height(),
        "action inside window"
    );
    const auto global = window->mapToGlobal(point.toPoint());
    QMouseEvent down(
        QEvent::MouseButtonPress,
        point,
        point,
        global,
        Qt::LeftButton,
        Qt::LeftButton,
        Qt::NoModifier
    );
    QMouseEvent up(
        QEvent::MouseButtonRelease,
        point,
        point,
        global,
        Qt::LeftButton,
        Qt::NoButton,
        Qt::NoModifier
    );
    QCoreApplication::sendEvent(window, &down);
    QCoreApplication::sendEvent(window, &up);
    settle();
}
void openWork(QObject& root, const QString& id) {
    evaluate(root, "openCreativeWork('" + id + "', artifactIndex('" + id + "'))");
    settle();
    auto* surface = root.findChild<QObject*>("workspaceSurface");
    require(
        surface && surface->property("loadedWorkspaceObjectName") == "legacyWorkWorkspace",
        "main-area compatibility route"
    );
}
} // namespace
bool convergence_smoke::verify(
    DesktopBackend& backend,
    LegacyWorkController& legacy,
    DeckHandoffController& deck,
    DeckMaterialTaskController& task,
    UiPreferences& preferences,
    QObject& root,
    const QString& directory,
    const QString& settingsPath
) {
    try {
        require(QDir().mkpath(directory), "evidence directory");
        const auto originalBundle = backend.bundlePath();
        const auto before = object(backend.legacySnapshot());
        require(
            before["read_only"] == true && before["legacy_artifacts"].toArray().size() >= 3,
            "real frozen legacy fixture"
        );
        require(
            !root.findChild<QObject*>("cardKitWorkspace")
                && !root.findChild<QObject*>("pictureBookWorkspace")
                && !root.findChild<QObject*>("audioSpeechOperatorWorkspace"),
            "specialist production UI removed"
        );
        QString book, acceptedKit;
        for (const auto& entry : before["legacy_artifacts"].toArray()) {
            const auto work = entry.toObject();
            if (work["kind"] == "picture_book")
                book = work["artifact_id"].toString();
            if (work["kind"] == "card_kit" && work["accepted_revision"].isString())
                acceptedKit = work["artifact_id"].toString();
        }
        require(
            !book.isEmpty() && !acceptedKit.isEmpty(),
            "accepted card and unfinished book fixture"
        );
        openWork(root, acceptedKit);
        require(legacy.selectSnapshot("accepted"), "select exact accepted kit");
        require(
            legacy.work()["deck_selection"].toString().startsWith("accepted:"),
            "accepted selector remains exact pin"
        );
        click(root, "legacyContinueDeckButton");
        auto* dialog = root.findChild<QObject*>("deckHandoffDialog");
        require(dialog && dialog->property("visible").toBool(), "real Continue in Deck click");
        QMetaObject::invokeMethod(dialog, "close");
        settle();
        openWork(root, book);
        require(legacy.snapshotKey() == "draft", "unfinished book selected explicitly");
        const auto bookToken = legacy.selectionToken();
        const auto bookPath = directory + "/Unfinished book.scproj";
        require(
            legacy.exportBook(QUrl::fromLocalFile(bookPath), bookToken),
            "draft editable book export"
        );
        require(QFileInfo::exists(bookPath), "Storybook file exists");
        save(
            directory + "/book-export.json",
            {{"path", bookPath},
             {"status", legacy.status()},
             {"work", QJsonObject::fromVariantMap(legacy.work())}}
        );
        for (const auto& language : {QStringLiteral("en"), QStringLiteral("zh_CN")}) {
            preferences.setLanguageMode(language);
            settle();
            auto* window = qobject_cast<QQuickWindow*>(&root);
            require(window, "root window");
            window->setWidth(1120);
            window->setHeight(720);
            settle();
            require(
                window->grabWindow().save(directory + "/book-" + language + "-1120.png"),
                "book screenshot"
            );
        }
        preferences.setLanguageMode("en");
        settle();
        openWork(root, acceptedKit);
        const auto stalePath = directory + "/Stale must not exist.scproj";
        require(
            !legacy.exportBook(QUrl::fromLocalFile(stalePath), bookToken)
                && !QFileInfo::exists(stalePath),
            "late export token rejected before output"
        );
        const auto speech = before["speech_drafts"].toArray();
        require(!speech.isEmpty(), "speech compatibility fixture");
        require(
            legacy.openWork(
                speech.first().toObject()["target_artifact_id"].toString(),
                speech.first().toObject()["draft_id"].toString()
            ),
            "exact old speech context"
        );
        require(
            legacy.exportVox(QUrl::fromLocalFile(directory), legacy.selectionToken()),
            "Vox transfer preserves legacy draft"
        );
        QFile manifest(legacy.lastExport());
        require(manifest.open(QIODevice::ReadOnly), "Vox manifest");
        const auto transfer = QJsonDocument::fromJson(manifest.readAll()).object();
        require(
            transfer["schema"] == "vox.shape-transfer/1" && transfer["authoring_draft"].isObject()
                && transfer["speech_draft"].isObject() && transfer["accepted_audio"].isNull(),
            "unfinished voice settings do not claim another recording"
        );
        save(
            directory + "/vox-selection.json",
            {{"manifest_path", legacy.lastExport()}, {"selection", object(legacy.selectionToken())}}
        );
        QString audioId, audioHead;
        for (const auto& item : backend.artifacts()) {
            const auto artifact = item.toMap();
            if (artifact["kindKey"] == "audio_clip"
                && !artifact["acceptedRevisionId"].toString().isEmpty()) {
                audioId = artifact["id"].toString();
                audioHead = artifact["acceptedRevisionId"].toString();
                break;
            }
        }
        require(!audioId.isEmpty() && legacy.openWork(audioId), "selected historical recording");
        require(
            legacy.exportVox(QUrl::fromLocalFile(directory), legacy.selectionToken()),
            "export selected historical recording"
        );
        QFile historicalFile(legacy.lastExport());
        require(historicalFile.open(QIODevice::ReadOnly), "historical manifest");
        const auto historical = QJsonDocument::fromJson(historicalFile.readAll()).object();
        require(
            historical["accepted_audio"].isObject()
                && historical["accepted_audio"]
                           .toObject()["material"]
                           .toObject()["source"]
                           .toObject()["revision_id"]
                       == audioHead,
            "historical audio keeps its exact selected revision"
        );
        save(
            directory + "/vox-historical.json",
            {{"manifest_path", legacy.lastExport()}, {"source", historical["accepted_audio"]}}
        );
        require(
            object(backend.legacySnapshot()) == before,
            "read, export and navigation preserve source logical state"
        );
        require(
            backend.createProject(QUrl::fromLocalFile(directory), "Ordinary writing"),
            "separate plain-writing project"
        );
        require(
            text_authoring_smoke::verify(backend, root),
            "general text Candidate/Accept preserved"
        );
        auto* host = root.findChild<QObject*>("operatorWorkspaceHost");
        auto* writer = host ? qvariant_cast<QObject*>(host->property("loadedWorkspace")) : nullptr;
        require(writer, "writing workspace");
        auto voxProgram = qEnvironmentVariable("SHAPE_VOX_BINARY");
        if (voxProgram.isEmpty()) {
            const auto fake = directory + "/vox-fixture";
            require(QDir().mkpath(fake), "fake Vox directory");
            for (const auto& name : {QStringLiteral("Vox"), QStringLiteral("vox-cli")}) {
                QFile file(fake + '/' + name);
                require(
                    file.open(QIODevice::WriteOnly | QIODevice::NewOnly),
                    "new executable fixture"
                );
                file.write(
                    name == "vox-cli" ? "#!/bin/sh\necho "
                                        "'{\"schema\":\"vox.capabilities/"
                                        "1\",\"operations\":[\"import_shape_transfer\"]}'\n"
                                      : "#!/bin/sh\nexit 0\n"
                );
                file.close();
                file.setPermissions(QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner);
            }
            voxProgram = fake + "/Vox";
        }
        if (!voxProgram.isEmpty())
            require(legacy.chooseVox(QUrl::fromLocalFile(voxProgram)), "configured real Vox app");
        const auto textId = writer->property("artifactId").toString();
        if (auto* scroll = writer->findChild<QObject*>("writingScroll"))
            evaluate(*scroll, "contentItem.contentY = 0");
        settle();
        click(*writer, "writingContinueVoxButton");
        require(
            legacy.work()["kind"].toString() == "speech",
            "new ordinary text reaches Vox handoff"
        );
        auto* chooser = root.findChild<QObject*>("legacyVoxProjectDialog");
        require(
            chooser && chooser->property("visible").toBool(),
            "one project picker from primary Vox action"
        );
        require(
            !root.findChild<QObject*>("legacyVoxExportDialog")->property("visible").toBool(),
            "export-folder picker is optional"
        );
        QMetaObject::invokeMethod(chooser, "close");
        settle();
        const auto unsupported = directory + "/old-vox";
        require(QDir().mkpath(unsupported), "old Vox fixture directory");
        for (const auto& name : {QStringLiteral("Vox"), QStringLiteral("vox-cli")}) {
            QFile file(unsupported + '/' + name);
            require(file.open(QIODevice::WriteOnly | QIODevice::NewOnly), "old app fixture");
            file.write("#!/bin/sh\necho '{}'\n");
            file.close();
            file.setPermissions(QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner);
        }
        require(legacy.chooseVox(QUrl::fromLocalFile(unsupported + "/Vox")), "choose old Vox");
        const auto unsupportedDestination = directory + "/Must not create.vox";
        require(
            !legacy.openVox(QUrl::fromLocalFile(unsupportedDestination), legacy.selectionToken())
                && !QFileInfo::exists(unsupportedDestination) && legacy.lastExport().isEmpty(),
            "old Vox fails capability check before export or launch"
        );
        require(legacy.chooseVox(QUrl::fromLocalFile(voxProgram)), "restore supported Vox");
        if (!voxProgram.isEmpty()) {
            const auto voxProject = directory + "/Ordinary writing.vox";
            require(
                legacy.openVox(QUrl::fromLocalFile(voxProject), legacy.selectionToken()),
                "single-step actual Vox launch"
            );
            save(
                directory + "/vox-launch.json",
                {{"project", voxProject},
                 {"manifest_path", legacy.lastExport()},
                 {"status", legacy.status()}}
            );
        }
        QString head;
        for (const auto& item : backend.artifacts())
            if (item.toMap()["id"].toString() == textId)
                head = item.toMap()["acceptedRevisionId"].toString();
        require(!head.isEmpty(), "current accepted plain text pin");
        evaluate(root, "workspaceSurface.openPinnedMaterial('" + textId + "','" + head + "')");
        settle();
        auto* source = qvariant_cast<QObject*>(host->property("loadedWorkspace"));
        require(
            source && source->objectName() == "sourceMaterialWorkspace",
            "read-only text source"
        );
        click(*source, "sourceContinueVoxButton");
        require(
            chooser->property("visible").toBool(),
            "read-only Source continues to Vox in one action"
        );
        QMetaObject::invokeMethod(chooser, "close");
        settle();
        require(
            backend.openProject(QUrl::fromLocalFile(originalBundle)),
            "reopen exact legacy source"
        );
        require(
            object(backend.legacySnapshot()) == before,
            "reopen does not initialize specialist drafts"
        );
        save(directory + "/source-snapshot.json", before);
        require(
            deck_handoff_smoke::verify(
                backend,
                deck,
                task,
                root,
                directory + "/deck",
                settingsPath
            ),
            "existing Deck receipt and standalone image-task contracts"
        );
        save(
            directory + "/report.json",
            {{"status", "passed"},
             {"read_only_snapshots", true},
             {"book_export", bookPath},
             {"stale_dialog_no_output", true},
             {"plain_text_vox_action", true},
             {"single_vox_project_picker", true},
             {"deck_handoff", true}}
        );
        return true;
    } catch (const std::exception& error) {
        std::cerr << "convergence smoke: " << error.what() << std::endl;
        return false;
    }
}

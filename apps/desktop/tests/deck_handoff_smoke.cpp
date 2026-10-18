#include "deck_handoff_smoke.hpp"
#include "deck_handoff_controller.hpp"
#include "deck_material_task_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_image_controller.hpp"
#include <QColor>
#include <QCoreApplication>
#include <QCryptographicHash>
#include <QDir>
#include <QDirIterator>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMouseEvent>
#include <QProcess>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSettings>
#include <QTimer>
#include <QUuid>
#include <functional>
#include <iostream>

namespace {
void write(const QString& path, const QByteArray& bytes) {
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::NewOnly) || file.write(bytes) != bytes.size())
        throw std::runtime_error("fixture publication failed");
}

QJsonObject read(const QString& path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return {};
    return QJsonDocument::fromJson(file.readAll()).object();
}
bool wait(const std::function<bool()>& done, int milliseconds = 12000) {
    QElapsedTimer timer;
    timer.start();
    while (!done() && timer.elapsed() < milliseconds) {
        QEventLoop loop;
        QTimer::singleShot(20, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return done();
}
QJsonObject digests(const QString& directory) {
    QJsonObject result;
    QDirIterator files(directory, QDir::Files, QDirIterator::Subdirectories);
    while (files.hasNext()) {
        const auto path = files.next();
        if (path.endsWith("-shm"))
            continue;
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly))
            return {};
        result[QDir(directory).relativeFilePath(path)] = QString::fromLatin1(
            QCryptographicHash::hash(file.readAll(), QCryptographicHash::Sha256).toHex()
        );
    }
    return result;
}
void require(bool condition, const char* message) {
    if (!condition)
        throw std::runtime_error(message);
}
QJsonObject action(DesktopBackend& backend, const QJsonObject& request) {
    const auto result =
        QJsonDocument::fromJson(
            backend.cardKitAction(QString::fromUtf8(QJsonDocument(request).toJson())).toUtf8()
        )
            .object();
    require(!result.isEmpty(), "card fixture action failed");
    return result;
}
int visibleTextCount(QQuickItem* item, const QString& text, bool buttonsOnly = false) {
    if (!item)
        return 0;
    int count = item->isVisible() && item->isEnabled() && item->width() > 0 && item->height() > 0
                        && item->property("text").toString() == text
                        && (!buttonsOnly || item->metaObject()->indexOfSignal("clicked()") >= 0)
                    ? 1
                    : 0;
    for (auto* child : item->childItems())
        count += visibleTextCount(child, text, buttonsOnly);
    return count;
}
QJsonObject verifyReturnedReceipts(
    DesktopBackend& backend,
    DeckHandoffController& deck,
    QObject& root,
    const QString& directory,
    const QString& settingsPath,
    const QString& kit,
    const QString& draftDigest
) {
    QSettings settings(settingsPath, QSettings::IniFormat);
    settings.sync();
    QString recordKey;
    for (const auto& key : settings.allKeys())
        if (key.startsWith("deck/links/") && key.endsWith('/' + kit))
            recordKey = key;
    const auto record = QJsonDocument::fromJson(settings.value(recordKey).toByteArray()).object();
    require(!record.isEmpty(), "prepared handoff record missing");
    const auto request = read(record.value("request_path").toString());

    // A copied fixture isolates receipt verification from the source. It has a real accepted kit.
    const auto documentPath = directory + "/Returned cards.deck";
    QDirIterator copied(backend.bundlePath(), QDir::Files, QDirIterator::Subdirectories);
    require(QDir().mkpath(documentPath), "returned document directory");
    while (copied.hasNext()) {
        const auto file = copied.next();
        const auto destination =
            documentPath + '/' + QDir(backend.bundlePath()).relativeFilePath(file);
        require(
            QDir().mkpath(QFileInfo(destination).absolutePath()) && QFile::copy(file, destination),
            "copy accepted fixture document"
        );
    }
    DesktopBackend document(shape::desktop::open_desktop_session(documentPath.toStdString()));
    QJsonObject source;
    const auto catalog = QJsonDocument::fromJson(document.legacySnapshot().toUtf8()).object();
    for (const auto& item : catalog["legacy_artifacts"].toArray()) {
        const auto saved = item.toObject();
        if (saved["kind"] == "card_kit" && saved["accepted_revision"].isString()) {
            source = {
                {"artifact_id", saved["artifact_id"]},
                {"revision_id", saved["accepted_revision"]}
            };
            break;
        }
    }
    require(!source.isEmpty(), "fixture has no accepted card version");
    write(
        documentPath + "/deck-transfer.json",
        QJsonDocument(
            QJsonObject{
                {"schema", "deck.transfer-import/1"},
                {"project_id", document.projectId()},
                {"kit_id", source["artifact_id"]},
                {"request_id", request["request_id"]},
                {"manifest_blake3", request["input"].toObject()["manifest_blake3"]},
                {"origin", request["origin"]}
            }
        ).toJson()
    );
    const auto zipPath = directory + "/returned-cards.zip";
    // Deterministic valid ZIP tests receipt publication, not removed Shape card authoring.
    // The actual Deck producer/consumer export is checked by the cross-application gate.
    const auto zip = QByteArray::fromHex(
        "504b03041400000000000000525dec32ff0f25000000250000000b000000726563656970742e747874"
        "4e6174697665207265636569707420766572696669636174696f6e20666978747572652e0a"
        "504b010214031400000000000000525dec32ff0f25000000250000000b00000000000000000000008001000000"
        "00726563656970742e747874"
        "504b05060000000001000100390000004e0000000000"
    );
    write(zipPath, zip);
    const QJsonObject exported{
        {"byte_length", zip.size()},
        {"output_blake3", "ba77e2f83650861e6936dc8fa461b0d448e6d40e83988b20eca18e21b6f10330"}
    };
    // Deterministic valid one-page PDF; the native worker verifies these exact bytes and BLAKE3.
    const QByteArray pdf =
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n"
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 72 72] /Resources << >> >>\n"
        "endobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n"
        "0000000058 00000 n \n0000000115 00000 n \ntrailer\n<< /Size 4 /Root 1 0 R >>\n"
        "startxref\n201\n%%EOF\n";
    const auto pdfPath = directory + "/returned-cards.pdf";
    write(pdfPath, pdf);
    const QJsonObject completed{
        {"schema", "deck.external-edit-result/1"},
        {"request_id", request["request_id"]},
        {"request_blake3", record["request_blake3"]},
        {"outcome", "completed"},
        {"document",
         QJsonObject{
             {"project_path", documentPath},
             {"project_id", document.projectId()},
             {"kit_id", source["artifact_id"]},
             {"accepted_revision_id", source["revision_id"]}
         }},
        {"outputs",
         QJsonArray{
             QJsonObject{
                 {"kind", "print_pdf"},
                 {"path", pdfPath},
                 {"byte_length", pdf.size()},
                 {"blake3", "38a62899b9e2d4e047f1a82daa20415de19ae0ad4b19a88a165254ab71ac6dc4"},
                 {"media_type", "application/pdf"}
             },
             QJsonObject{
                 {"kind", "materials_zip"},
                 {"path", zipPath},
                 {"byte_length", exported["byte_length"]},
                 {"blake3", exported["output_blake3"]},
                 {"media_type", "application/zip"}
             }
         }},
        {"error", QJsonValue::Null}
    };
    const auto resultPath = record.value("result_path").toString();
    write(resultPath, QJsonDocument(completed).toJson());
    auto* dialog = root.findChild<QObject*>("deckHandoffDialog");
    require(
        dialog
            && QMetaObject::invokeMethod(
                dialog,
                "openForKit",
                Q_ARG(QVariant, QVariant(kit)),
                Q_ARG(QVariant, QVariant(draftDigest))
            ),
        "open actual returned-output dialog"
    );
    auto* readButton = root.findChild<QQuickItem*>("deckReadResult");
    require(
        readButton && readButton->isVisible() && readButton->isEnabled()
            && QMetaObject::invokeMethod(readButton, "clicked") && deck.busy(),
        "actual Read returned files action did not start Verify"
    );
    const auto successStatus = QCoreApplication::translate(
        "DeckHandoffController",
        "Returned files verified. Open them below."
    );
    require(
        wait([&] { return !deck.busy(); }) && !deck.waiting() && deck.error().isEmpty()
            && deck.status() == successStatus && deck.hasDocument() && deck.outputs().size() == 2,
        "verified completed receipt did not reach returned state"
    );
    auto* content = qvariant_cast<QQuickItem*>(dialog->property("contentItem"));
    const auto label = [](const char* text) {
        return QCoreApplication::translate("DeckHandoffDialog", text);
    };
    const bool linksVisible = wait([&] {
        return visibleTextCount(content, label("Verified print PDF")) == 1
               && visibleTextCount(content, label("Verified materials ZIP")) == 1
               && visibleTextCount(content, label("Open"), true) == 2
               && visibleTextCount(content, label("Show folder"), true) == 2;
    });
    QEventLoop rendered;
    QTimer::singleShot(200, &rendered, &QEventLoop::quit);
    rendered.exec();
    auto* window = qobject_cast<QQuickWindow*>(&root);
    require(
        window && window->grabWindow().save(directory + "/returned-output-links.png"),
        "capture actual returned-output links"
    );
    if (!linksVisible) {
        QJsonArray items;
        const std::function<void(QQuickItem*)> inspect = [&](QQuickItem* item) {
            if (!item)
                return;
            items.append(
                QJsonObject{
                    {"name", item->objectName()},
                    {"class", item->metaObject()->className()},
                    {"text", item->property("text").toString()},
                    {"visible", item->isVisible()},
                    {"enabled", item->isEnabled()},
                    {"width", item->width()},
                    {"height", item->height()}
                }
            );
            for (auto* child : item->childItems())
                inspect(child);
        };
        inspect(content);
        write(
            directory + "/returned-links-diagnostic.json",
            QJsonDocument(
                QJsonObject{
                    {"kit", dialog->property("kitId").toString()},
                    {"bound", dialog->property("boundToKit").toBool()},
                    {"items", items}
                }
            ).toJson()
        );
    }
    require(linksVisible, "verified PDF and ZIP links are not visible and enabled");
    require(QMetaObject::invokeMethod(dialog, "close"), "close returned-output dialog");
    DeckHandoffController reopened(backend, settingsPath);
    require(
        reopened.attach(kit) && reopened.busy() && wait([&] { return !reopened.busy(); })
            && reopened.error().isEmpty() && reopened.outputs().size() == 2
            && reopened.status() == successStatus,
        "reopened controller lost the verified completed receipt"
    );
    settings.sync();
    const auto saved = QJsonDocument::fromJson(settings.value(recordKey).toByteArray()).object();
    require(
        saved["local_state"] == "returned" && !saved["waiting"].toBool()
            && saved["outputs"].toArray().size() == 2,
        "completed returned state was not persisted"
    );
    for (const auto& outcome : {QString("cancelled"), QString("failed"), QString("tampered")}) {
        auto receipt = completed;
        const bool tampered = outcome == "tampered";
        if (tampered) {
            auto outputs = receipt["outputs"].toArray();
            auto output = outputs[0].toObject();
            output["blake3"] = QString(64, '0');
            outputs[0] = output;
            receipt["outputs"] = outputs;
        } else {
            receipt["outcome"] = outcome;
            receipt["document"] = QJsonValue::Null;
            receipt["outputs"] = QJsonArray{};
            receipt["error"] = outcome == "failed" ? QJsonValue("Fixture producer failure")
                                                   : QJsonValue(QJsonValue::Null);
        }
        const auto path = directory + '/' + outcome + "-result.json";
        write(path, QJsonDocument(receipt).toJson());
        auto isolatedRecord = saved;
        isolatedRecord["result_path"] = path;
        const auto profile = directory + '/' + outcome + ".ini";
        QSettings isolated(profile, QSettings::IniFormat);
        isolated.setValue(recordKey, QJsonDocument(isolatedRecord).toJson());
        isolated.sync();
        DeckHandoffController terminal(backend, profile);
        require(
            terminal.attach(kit) && terminal.busy() && wait([&] { return !terminal.busy(); })
                && !terminal.waiting() && terminal.outputs().isEmpty(),
            "terminal receipt worker did not finish safely"
        );
        const auto expectedStatus = QCoreApplication::translate(
            "DeckHandoffController",
            outcome == "cancelled" ? "Deck editing was cancelled. Saved Deck work can be continued."
                                   : "Deck reported an error. Saved work can be continued."
        );
        require(
            tampered ? !terminal.error().isEmpty() && terminal.status().isEmpty()
                     : terminal.error().isEmpty() && terminal.status() == expectedStatus,
            "producer terminal outcome confused with verifier failure"
        );
        isolated.sync();
        const auto terminalRecord =
            QJsonDocument::fromJson(isolated.value(recordKey).toByteArray()).object();
        require(
            terminalRecord["local_state"] == (tampered ? "incomplete" : "returned")
                && terminalRecord["outputs"].toArray().isEmpty() && read(path) == receipt,
            "terminal receipt state or immutable result changed"
        );
    }
    require(read(resultPath) == completed, "completed receipt overwritten");
    return {
        {"native_verify_completed_error_null", true},
        {"verified_pdf_zip_links_visible", true},
        {"returned_record_reopened", true},
        {"cancelled_error_null_returned", true},
        {"failed_producer_error_returned", true},
        {"tampered_output_rejected_and_links_cleared", true},
        {"completed_result_path", resultPath},
        {"returned_document", completed["document"]}
    };
}
} // namespace
bool deck_handoff_smoke::verify(
    DesktopBackend& backend,
    DeckHandoffController& deck,
    DeckMaterialTaskController& task,
    QObject& root,
    const QString& outputDirectory,
    const QString& settingsPath
) {
    try {
        const auto directory =
            outputDirectory + "/run-" + QUuid::createUuid().toString(QUuid::WithoutBraces);
        require(QDir().mkpath(directory), "cannot create fixture directory");
        require(backend.projectOpen(), "a frozen legacy fixture project is required");
        QJsonObject draft;
        const auto saved = action(backend, {{"action", "draft_list"}})["drafts"].toArray();
        for (const auto& item : saved)
            if (item.toObject()["expected_head"].isNull()) {
                draft = item.toObject();
                break;
            }
        require(!draft.isEmpty(), "fixture has no unfinished legacy card draft");
        const auto kit = draft["artifact_id"].toString(), digest = draft["draft_digest"].toString();
        const auto before = digests(backend.bundlePath());
        require(!before.isEmpty(), "source snapshot");
        const auto executable = directory + "/deck-process-fixture";
        write(
            executable,
            "#!/bin/sh\nif [ \"$1\" = \"--handoff-capabilities\" ]; then\n  echo "
            "'{\"schema\":\"deck.handoff-capabilities/1\",\"external_edit\":\"deck.external-edit/"
            "1\"}'\n  exit 0\nfi\nexit 0\n"
        );
        QFile::setPermissions(executable, QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner);
#ifdef Q_OS_MACOS
        const auto application = directory + "/Fixture Deck.app";
        require(QDir().mkpath(application + "/Contents/MacOS"), "application fixture directory");
        require(
            QFile::copy(executable, application + "/Contents/MacOS/deck-main"),
            "application main executable"
        );
        require(
            QFile::copy(executable, application + "/Contents/MacOS/deck-cli"),
            "application companion executable"
        );
        write(
            application + "/Contents/Info.plist",
            "<?xml version=\"1.0\"?><plist "
            "version=\"1.0\"><dict><key>CFBundleExecutable</key><string>deck-main</string></dict></"
            "plist>"
        );
        require(
            deck.setExecutable(QUrl::fromLocalFile(application))
                && deck.executable().endsWith("/deck-main"),
            "application bundle selected a companion CLI"
        );
#endif
        require(
            deck.setExecutable(QUrl::fromLocalFile(executable)),
            "set isolated lifecycle fixture"
        );
        deck.openKit(kit, digest, true);
        require(
            wait([&] { return !deck.busy() && !deck.waiting() && !deck.error().isEmpty(); }),
            "missing receipt did not become terminal"
        );
        const auto requestPath = deck.requestPath();
        require(!requestPath.isEmpty(), "real source transfer not published");
        const auto request = read(requestPath);
        const auto manifestPath =
            request.value("input").toObject().value("manifest_path").toString();
        const auto manifest = read(manifestPath);
        require(
            manifest.value("schema") == "deck.card-transfer/1"
                && manifest.value("composition").toObject() == draft["composition"].toObject(),
            "complete real transfer"
        );
        require(
            !manifest.value("composition").toObject().contains("layout"),
            "legacy layout upgraded"
        );
        require(digests(backend.bundlePath()) == before, "transfer changed mixed source bundle");
        DeckHandoffController restored(backend, settingsPath);
        require(
            restored.attach(kit) && !restored.waiting(),
            "known dead process resumed waiting after controller restart"
        );
#ifdef Q_OS_MACOS
        const auto nextApplication = directory + "/Next Fixture Deck.app";
        require(QDir().mkpath(nextApplication + "/Contents/MacOS"), "next application fixture");
        require(
            QFile::copy(
                application + "/Contents/Info.plist",
                nextApplication + "/Contents/Info.plist"
            ) && QFile::copy(executable, nextApplication + "/Contents/MacOS/deck-main"),
            "next application executable"
        );
        const auto firstRelease = directory + "/release-one",
                   secondRelease = directory + "/release-two", alias = directory + "/current-debug";
        require(
            QDir().mkpath(firstRelease) && QDir().mkpath(secondRelease)
                && QFile::link(application, firstRelease + "/Deck.app")
                && QFile::link(nextApplication, secondRelease + "/Deck.app")
                && QFile::link(firstRelease, alias),
            "debug alias fixture"
        );
        const auto choice = alias + "/Deck.app";
        require(
            deck.setExecutable(QUrl::fromLocalFile(choice)),
            "choose current-debug application"
        );
        QSettings choices(settingsPath, QSettings::IniFormat);
        choices.sync();
        require(
            choices.value("deck/executable").toString() == QDir::cleanPath(choice),
            "persisted choice lost the user alias"
        );
        const auto firstBinary = deck.executable();
        require(QFile::remove(alias) && QFile::link(secondRelease, alias), "advance debug alias");
        const auto nextBinary = deck.executable();
        require(
            nextBinary != firstBinary
                && nextBinary
                       == QFileInfo(nextApplication + "/Contents/MacOS/deck-main")
                              .canonicalFilePath(),
            "new executable lookup did not follow debug promotion"
        );
        DeckHandoffController reopenedChoice(backend, settingsPath);
        require(
            reopenedChoice.executable() == nextBinary,
            "reopened settings did not follow debug promotion"
        );
        require(
            QFile::remove(alias) && QFile::link(firstRelease, alias),
            "restore initial alias before launch"
        );
        bool advancedWhileLaunching = false, advanceSucceeded = false;
        const auto aliasConnection =
            QObject::connect(&deck, &DeckHandoffController::changed, &root, [&] {
                if (deck.waiting() && !deck.busy() && !advancedWhileLaunching) {
                    advancedWhileLaunching = true;
                    advanceSucceeded = QFile::remove(alias) && QFile::link(secondRelease, alias);
                }
            });
        deck.openKit(kit, digest, true);
        require(
            wait([&] { return !deck.busy() && !deck.waiting() && !deck.error().isEmpty(); }),
            "alias launch did not complete its lifecycle"
        );
        QObject::disconnect(aliasConnection);
        require(
            advancedWhileLaunching && advanceSucceeded && deck.executable() == nextBinary,
            "debug alias did not advance during the launch"
        );
        choices.sync();
        QJsonObject launchRecord;
        for (const auto& key : choices.allKeys())
            if (key.startsWith("deck/links/") && key.endsWith('/' + kit))
                launchRecord = QJsonDocument::fromJson(choices.value(key).toByteArray()).object();
        require(
            launchRecord.value("executable").toString() == firstBinary
                && launchRecord.value("pid").toDouble() > 0,
            "running process identity drifted with its debug alias"
        );
#endif
        QJsonObject report{
            {"schema", "shape.deck-handoff-smoke/1"},
            {"status", "passed"},
            {"source_bundle", backend.bundlePath()},
            {"source_digests", before},
            {"request_path", requestPath},
            {"manifest_path", manifestPath},
            {"legacy_incomplete_transfer", true},
            {"mixed_audio_source_unchanged", true},
            {"process_exit_without_receipt", true}
        };
#ifdef Q_OS_MACOS
        report["configured_debug_alias_preserved"] = true;
        report["reopened_getter_follows_promotion"] = true;
        report["launched_process_identity_pinned"] = true;
#endif
        report["returned_receipts"] =
            verifyReturnedReceipts(backend, deck, root, directory, settingsPath, kit, digest);
        QJsonObject acceptedSource;
        for (const auto& entry : QJsonDocument::fromJson(backend.legacySnapshot().toUtf8())
                                     .object()["legacy_artifacts"]
                                     .toArray()) {
            const auto work = entry.toObject();
            if (work["kind"] == "card_kit" && work["accepted_revision"].isString()) {
                acceptedSource = {
                    {"artifact_id", work["artifact_id"]},
                    {"revision_id", work["accepted_revision"]}
                };
                break;
            }
        }
        require(!acceptedSource.isEmpty(), "accepted snapshot fixture");
        deck.openKit(
            acceptedSource["artifact_id"].toString(),
            "accepted:" + acceptedSource["revision_id"].toString(),
            true
        );
        require(
            wait([&] { return !deck.busy() && !deck.waiting() && !deck.error().isEmpty(); }),
            "accepted snapshot child lifecycle"
        );
        const auto acceptedRequest = read(deck.requestPath());
        const auto acceptedManifest =
            read(acceptedRequest["input"].toObject()["manifest_path"].toString());
        require(
            acceptedManifest["composition"]
                == action(backend, {{"action", "read"}, {"source", acceptedSource}})["composition"],
            "accepted snapshot contains no saved draft edits"
        );
        report["accepted_snapshot_exact"] = true;
        require(digests(backend.bundlePath()) == before, "receipt reads changed the source bundle");
        const auto taskPath = directory + "/image-task.json",
                   resultPath = directory + "/image-result.json";
        write(
            taskPath,
            QJsonDocument(
                QJsonObject{
                    {"schema", "deck.material-task/1"},
                    {"request_id", "native-image-task"},
                    {"target",
                     QJsonObject{
                         {"project_id", "foreign-project"},
                         {"kit_id", "foreign-kit"},
                         {"card_id", "original-card"},
                         {"draft_digest", QString(64, 'a')},
                         {"artwork", QJsonValue::Null}
                     }},
                    {"prompt", "A clear friendly image of a tree"},
                    {"canvas", QJsonObject{{"width", 750}, {"height", 1050}}}
                }
            ).toJson()
        );
        require(
            task.initialize(taskPath, resultPath, directory + "/isolated-tasks"),
            "initialize real task entry"
        );
        QQmlExpression open(
            QQmlEngine::contextForObject(&root),
            &root,
            QStringLiteral("openDeckMaterialTask()")
        );
        open.evaluate();
        QCoreApplication::processEvents();
        require(
            !open.hasError() && task.active() && !task.canReturn(),
            "task incorrectly returnable before accepted image"
        );
        auto* bar = root.findChild<QObject*>("deckMaterialTaskBar");
        require(bar && bar->property("visible").toBool(), "native task return UI absent");
        auto* surface = root.findChild<QObject*>("workspaceSurface");
        require(
            surface && wait([&] {
                return surface->property("loadedWorkspaceObjectName").toString()
                       == "aiImageOperatorWorkspace";
            }),
            "task did not land on the actual image workspace"
        );
        QVariantMap target;
        for (const auto& value : backend.operatorDrafts())
            if (value.toMap().value("id").toString() == task.draftId())
                target = value.toMap();
        require(
            target.value("contextArtifactId").toString() == task.artifactId()
                && target.value("aiImageInstruction").toString()
                       == "A clear friendly image of a tree"
                && target.value("aiImageOutputWidth").toInt() == 750
                && target.value("aiImageOutputHeight").toInt() == 1050,
            "task prompt/canvas or image identity was not preserved"
        );
        require(task.cancel(), "explicit cancel");
        const auto cancelled = read(resultPath);
        require(
            cancelled.value("outcome") == "cancelled" && cancelled.value("output").isNull(),
            "cancelled terminal receipt"
        );
        const auto exact = QJsonDocument(cancelled).toJson();
        require(
            !task.initialize(taskPath, resultPath, directory + "/isolated-tasks"),
            "existing result reused"
        );
        require(QJsonDocument(read(resultPath)).toJson() == exact, "terminal receipt overwritten");
        report["isolated_material_task_cancel"] = true;
        report["unaccepted_image_cannot_return"] = true;
        report["existing_result_preserved"] = true;
        auto childTask = read(taskPath);
        childTask["request_id"] = "whole-process-task";
        const auto childRequest = directory + "/child-task.json",
                   childResult = directory + "/child-result.json";
        write(childRequest, QJsonDocument(childTask).toJson());
        QProcess child;
        child.start(
            QCoreApplication::applicationFilePath(),
            {"--deck-material-task",
             childRequest,
             "--result",
             childResult,
             "--smoke-deck-material-cancel"}
        );
        require(
            child.waitForStarted(3000)
                && wait([&] { return child.state() == QProcess::NotRunning; }, 30000),
            "actual isolated Shape task process did not close"
        );
        require(
            child.exitStatus() == QProcess::NormalExit && child.exitCode() == 0,
            "actual Shape task process failed"
        );
        const auto childReceipt = read(childResult);
        require(
            childReceipt.value("outcome") == "cancelled"
                && childReceipt.value("request_id") == "whole-process-task",
            "whole-process close did not publish the exact terminal receipt"
        );
        report["whole_process_material_task_cancel"] = true;
        write(directory + "/report.json", QJsonDocument(report).toJson());
        std::cout << "Deck handoff native smoke passed; real source transfer, no-result lifecycle, "
                     "isolated material task/cancel"
                  << std::endl;
        return true;
    } catch (const std::exception& error) {
        std::cerr << "Deck handoff smoke: " << error.what() << std::endl;
        return false;
    }
}

bool deck_handoff_smoke::runLiveTask(
    DesktopBackend& backend,
    InferImageController& image,
    DeckMaterialTaskController& task,
    QObject& root,
    const QString& model,
    const QString& effort
) {
    try {
        require(
            task.active() && !task.artifactId().isEmpty() && !task.draftId().isEmpty(),
            "live task not initialized"
        );
        image.generate(backend.bundlePath(), task.artifactId(), task.draftId(), model, effort, 1);
        require(wait([&] { return !image.running(); }, 660000), "live task generation timed out");
        require(image.errorCode().isEmpty(), "live task generation failed");
        QString candidate;
        for (const auto& value : backend.candidates()) {
            const auto item = value.toMap();
            if (item.value("artifactId").toString() == task.artifactId())
                candidate = item.value("id").toString();
        }
        require(
            !candidate.isEmpty() && !task.canReturn(),
            "unaccepted live candidate must not return"
        );
        require(
            backend.acceptCandidate(candidate) && task.canReturn(),
            "explicit live candidate acceptance"
        );
        QCoreApplication::processEvents();
        auto* button = root.findChild<QQuickItem*>("deckMaterialReturn");
        auto* window = qobject_cast<QQuickWindow*>(&root);
        require(
            button && window && button->isVisible() && button->isEnabled(),
            "live Return button not available"
        );
        const QPointF position =
            button->mapToScene(QPointF(button->width() / 2, button->height() / 2));
        const QPointF global = window->mapToGlobal(position.toPoint());
        QMouseEvent press(
            QEvent::MouseButtonPress,
            position,
            position,
            global,
            Qt::LeftButton,
            Qt::LeftButton,
            Qt::NoModifier
        );
        QCoreApplication::sendEvent(window, &press);
        QMouseEvent release(
            QEvent::MouseButtonRelease,
            position,
            position,
            global,
            Qt::LeftButton,
            Qt::NoButton,
            Qt::NoModifier
        );
        QCoreApplication::sendEvent(window, &release);
        require(
            wait([&] { return !task.active() || !task.error().isEmpty(); }),
            "live task return did not finish"
        );
        require(!task.active() && task.error().isEmpty(), "live task return failed");
        std::cout << "Deck material live task passed: exact image target, real generation, "
                     "explicit accept and Return pointer"
                  << std::endl;
        return true;
    } catch (const std::exception& error) {
        std::cerr << "Deck material live task: " << error.what() << std::endl;
        return false;
    }
}

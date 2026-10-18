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
        const auto projectName = QStringLiteral("Mixed legacy source");
        require(
            backend.createProject(QUrl::fromLocalFile(directory), projectName),
            "create mixed project"
        );
        QImage image(16, 12, QImage::Format_RGBA8888);
        image.fill(QColor(50, 120, 180));
        const auto png = directory + "/source.png";
        require(image.save(png), "PNG fixture");
        require(backend.importRaster(QUrl::fromLocalFile(png)), "import accepted raster");
        const auto art = backend.artifacts().last().toMap();
        require(
            backend.createTextScene("Source text", "Unfinished words\n未采用排版"),
            "accepted text fixture"
        );
        const auto text = backend.artifacts().last().toMap();
        QByteArray wav = QByteArray::fromHex(
            "524946462c00000057415645666d74201000000001000100401f0000803e00000200100064617461080000"
            "000000010002000300"
        );
        const auto audio = directory + "/source.wav";
        write(audio, wav);
        require(backend.importMaterial(QUrl::fromLocalFile(audio)), "accepted audio fixture");
        const auto pin = [](const QVariantMap& item) {
            return QJsonObject{
                {"artifact_id", item.value("id").toString()},
                {"revision_id", item.value("acceptedRevisionId").toString()}
            };
        };
        const QJsonObject composition{
            {"name", "Incomplete legacy cards"},
            {"cards",
             QJsonArray{
                 QJsonObject{
                     {"id", "image-card"},
                     {"art_prompt", "Unfinished prompt"},
                     {"artwork", pin(art)}
                 },
                 QJsonObject{{"id", "text-card"}, {"text_material", pin(text)}}
             }}
        };
        const auto draft =
            QJsonDocument::fromJson(
                backend
                    .cardKitAction(
                        QString::fromUtf8(
                            QJsonDocument(
                                QJsonObject{{"action", "draft_new"}, {"composition", composition}}
                            ).toJson()
                        )
                    )
                    .toUtf8()
            )
                .object()
                .value("draft")
                .toObject();
        const auto kit = draft.value("artifact_id").toString(),
                   digest = draft.value("draft_digest").toString();
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
                && manifest.value("materials").toArray().size() == 2,
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

#include "sound_live_smoke.hpp"
#include "audio_export_controller.hpp"
#include "audio_preview_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_sound_controller.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QStandardPaths>
#include <QTimer>
#include <QUrl>
#include <functional>
#include <iostream>
namespace {
bool waitUntil(const std::function<bool()>& ready, int timeout_ms) {
    QElapsedTimer elapsed;
    elapsed.start();
    while (!ready() && elapsed.elapsed() < timeout_ms) {
        QEventLoop loop;
        QTimer::singleShot(50, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return ready();
}
bool evaluate(QObject& object, const QString& expression) {
    QQmlExpression call(QQmlEngine::contextForObject(&object), &object, expression);
    call.evaluate();
    QCoreApplication::processEvents();
    if (call.hasError())
        std::cerr << call.error().toString().toStdString() << std::endl;
    return !call.hasError();
}
bool require(bool condition, const char* label) {
    if (!condition)
        std::cerr << "sound live smoke failed: " << label << std::endl;
    return condition;
}
bool exportAndCompare(
    AudioExportController& exporter,
    const QString& source,
    const QString& candidate,
    const QString& path,
    const QByteArray& expected
) {
    bool done = false;
    bool success = false;
    auto connection = QObject::connect(
        &exporter,
        &AudioExportController::finished,
        &exporter,
        [&](bool ok, const QString&, const QString&) {
            done = true;
            success = ok;
        }
    );
    exporter.exportAudio(source, candidate, QUrl::fromLocalFile(path));
    const bool completed = waitUntil([&] { return done; }, 10'000);
    QObject::disconnect(connection);
    QFile file(path);
    return require(
        completed && success && file.open(QIODevice::ReadOnly) && file.readAll() == expected,
        "exact WAV export"
    );
}
} // namespace

namespace sound_live_smoke {
bool run(
    DesktopBackend& backend,
    InferSoundController& sound,
    AudioPreviewController& preview,
    AudioExportController& exporter,
    QObject& root,
    const QString& output
) {
    if (!require(QDir(output).exists() && !backend.projectOpen(), "isolated launch")
        || !require(
            backend.createProject(QUrl::fromLocalFile(output), "Sound verification"),
            "project"
        ))
        return false;
    const QString bundle = backend.bundlePath();
    const QStringList prompts{
        QStringLiteral("轻柔的雨滴落在窗户上，远处雷声，没有人声和音乐。"),
        QStringLiteral("古琴与古筝轻柔对奏，80 BPM，soft piano 在远处，不要人声、鼓点或铃声。"),
        QStringLiteral("Dry wooden knock followed by a soft bell, no speech or music")
    };
    QString first_text_job;
    for (int index = 0; index < prompts.size(); ++index) {
        const QString name = QStringLiteral("Sound %1").arg(index + 1);
        if (!require(backend.createSoundScene(name), "sound source"))
            return false;
        QString artifact;
        int artifact_index = -1;
        for (int i = 0; i < backend.artifacts().size(); ++i) {
            const auto a = backend.artifacts()[i].toMap();
            if (a.value("name").toString() == name) {
                artifact = a.value("id").toString();
                artifact_index = i;
            }
        }
        QString draft;
        for (const auto& v : backend.operatorDrafts()) {
            const auto d = v.toMap();
            if (d.value("contextArtifactId").toString() == artifact)
                draft = d.value("id").toString();
        }
        if (!require(!artifact.isEmpty() && !draft.isEmpty(), "source draft identity"))
            return false;
        root.setProperty("selectedArtifactIndex", artifact_index);
        QCoreApplication::processEvents();
        auto* surface = root.findChild<QObject*>("workspaceSurface");
        if (!require(
                surface && evaluate(*surface, QStringLiteral("openOperatorDraft('%1')").arg(draft)),
                "open sound workspace"
            ))
            return false;
        auto* host = root.findChild<QObject*>("operatorWorkspaceHost");
        QObject* workspace = nullptr;
        if (!require(
                waitUntil(
                    [&] {
                        workspace = host
                                        ? qvariant_cast<QObject*>(host->property("loadedWorkspace"))
                                        : nullptr;
                        return workspace && workspace->objectName() == "soundGenerationWorkspace";
                    },
                    3000
                ),
                "packaged sound workspace"
            ))
            return false;
        auto* prompt = workspace->findChild<QObject*>("soundPromptField");
        auto* seconds = workspace->findChild<QObject*>("soundDurationField");
        auto* seed = workspace->findChild<QObject*>("soundSeedField");
        auto* model = workspace->findChild<QObject*>("soundModelPicker");
        auto* generate = workspace->findChild<QObject*>("soundGenerateButton");
        if (!require(prompt && seconds && seed && model && generate, "authoring controls"))
            return false;
        prompt->setProperty("text", prompts[index]);
        seconds->setProperty("text", "3");
        seed->setProperty("text", "314159");
        model->setProperty("currentIndex", index == 1 ? 1 : 0);
        if (index == 0) {
            sound.generate(bundle, artifact, "missing-draft");
            if (!require(
                    waitUntil([&] { return !sound.running(); }, 5000)
                        && !sound.errorCode().isEmpty() && !backend.hasCandidate(),
                    "failure leaves empty shelf"
                ))
                return false;
            sound.generate(bundle, artifact, draft);
            sound.stop();
            if (!require(
                    waitUntil([&] { return !sound.running(); }, 5000)
                        && sound.errorCode() == "generation_cancelled" && !backend.hasCandidate(),
                    "immediate stop without candidate"
                ))
                return false;
        }
        if (!require(
                QMetaObject::invokeMethod(generate, "clicked", Qt::DirectConnection),
                "click Generate"
            ))
            return false;
        if (!require(
                sound.running() && waitUntil([&] { return !sound.running(); }, 600000)
                    && sound.errorCode().isEmpty() && backend.hasCandidate(),
                "real sound response"
            )) {
            std::cerr << sound.errorCode().toStdString() << std::endl;
            sound.stop();
            return false;
        }
        QString candidate = backend.candidateId();
        auto details =
            QJsonDocument::fromJson(backend.soundDetails(artifact, candidate).toUtf8()).object();
        auto operation = details.value("operation").toObject();
        auto prepared = details.value("provenance").toObject().value("sound_prompt").toObject();
        if (!require(
                operation.value("prompt").toString() == prompts[index]
                    && operation.value("duration_seconds").toInt() == 3
                    && operation.value("seed").toInt() == 314159,
                "click-time numeric and prompt snapshot"
            ))
            return false;
        if (!require(
                prepared.value("original_prompt").toString() == prompts[index]
                    && !prepared.value("effective_prompt").toString().isEmpty(),
                "original and effective prompt binding"
            ))
            return false;
        if (index == 2
            && !require(
                prepared.value("effective_prompt").toString() == prompts[index]
                    && prepared.value("text_job").isNull(),
                "English bypass"
            ))
            return false;
        if (index == 0) {
            first_text_job = prepared.value("text_job").toObject().value("id").toString();
            seed->setProperty("text", "314160");
            if (!require(
                    QMetaObject::invokeMethod(generate, "clicked", Qt::DirectConnection)
                        && sound.running() && waitUntil([&] { return !sound.running(); }, 600000)
                        && sound.errorCode().isEmpty() && backend.candidates().size() == 2,
                    "second candidate"
                ))
                return false;
            const QString second = backend.candidateId();
            auto second_details =
                QJsonDocument::fromJson(backend.soundDetails(artifact, second).toUtf8()).object();
            if (!require(
                    second != candidate
                        && second_details.value("operation").toObject().value("seed").toInt()
                               == 314160
                        && second_details.value("provenance")
                                   .toObject()
                                   .value("sound_prompt")
                                   .toObject()
                                   .value("text_job")
                                   .toObject()
                                   .value("id")
                                   .toString()
                               == first_text_job,
                    "same description reuses one translation"
                ))
                return false;
            if (!require(
                    backend.selectCandidate(candidate) && preview.loadPreview(artifact, candidate)
                        && backend.discardCandidate(second) && backend.candidates().size() == 1,
                    "switch and delete exact candidate"
                ))
                return false;
        }
        const auto audio = backend.audioPreview(artifact, candidate);
        if (!require(
                audio.has_value() && preview.loadPreview(artifact, candidate)
                    && preview.durationMillis() == 3000 && preview.sampleRateHz() == 44100
                    && preview.channels() == 2,
                "verified stereo WAV"
            ))
            return false;
        preview.togglePlayback();
        if (!require(
                waitUntil([&] { return preview.positionMillis() > 200; }, 10000)
                    && preview.errorCode().isEmpty(),
                "playback advances"
            ))
            return false;
        preview.seekTo(1200);
        preview.togglePlayback();
        if (!exportAndCompare(
                exporter,
                artifact,
                candidate,
                QDir(output).filePath(QStringLiteral("sound-%1.wav").arg(index + 1)),
                audio->wav_bytes
            ))
            return false;
        if (!require(
                backend.acceptCandidate(candidate) && !backend.hasCandidate(),
                "explicit accept"
            ))
            return false;
        if (!require(
                backend.operatorDrafts().isEmpty(),
                "accepted source retires its authoring draft"
            ))
            return false;
        if (!require(backend.openProject(QUrl::fromLocalFile(bundle)), "reopen"))
            return false;
        const auto reopened = backend.audioPreview(artifact);
        const auto restored =
            QJsonDocument::fromJson(backend.soundDetails(artifact, QString()).toUtf8()).object();
        if (!require(
                reopened.has_value() && reopened->wav_bytes == audio->wav_bytes
                    && restored == details,
                "exact accepted bytes and provenance after reopen"
            ))
            return false;
        QFile evidence(QDir(output).filePath(QStringLiteral("sound-%1.json").arg(index + 1)));
        if (!evidence.open(QIODevice::WriteOnly))
            return false;
        evidence.write(QJsonDocument(restored).toJson());
        std::cout << "sound " << index + 1
                  << " passed: " << restored.value("job_id").toString().toStdString() << std::endl;
    }
    if (!require(backend.createSoundScene("Close during sound"), "close test source"))
        return false;
    const auto closing_draft = backend.operatorDrafts().last().toMap();
    const QString closing_id = closing_draft.value("id").toString();
    const QString closing_artifact = closing_draft.value("contextArtifactId").toString();
    if (!require(
            backend.updateSoundDraft(
                closing_id,
                "Soft rain, no speech or music",
                "sound_effect",
                30,
                "77"
            ),
            "close test request"
        ))
        return false;
    const QString credential =
        QDir(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation))
            .filePath("secrets/infer-runtime.token");
    auto closing = std::make_unique<InferSoundController>(backend, credential);
    closing->generate(bundle, closing_artifact, closing_id);
    if (!require(
            waitUntil([&] { return closing->stage() == 2 || !closing->running(); }, 10000)
                && closing->running(),
            "close during active audio request"
        ))
        return false;
    QElapsedTimer close_time;
    close_time.start();
    closing.reset();
    QCoreApplication::processEvents();
    if (!require(
            close_time.elapsed() < 2000 && !backend.hasCandidate(),
            "controller destruction cancels promptly without late adoption"
        ))
        return false;
    std::cout << "sound live smoke passed; close wait " << close_time.elapsed() << " ms"
              << std::endl;
    return true;
}
} // namespace sound_live_smoke

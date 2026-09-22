#include "speech_live_smoke.hpp"
#include "audio_export_controller.hpp"
#include "audio_preview_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_speech_controller.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QTimer>
#include <QUrl>
#include <QtEndian>
#include <cmath>
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
        std::cerr << "speech live smoke failed: " << label << std::endl;
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

namespace speech_live_smoke {
bool run(
    DesktopBackend& backend,
    InferSpeechController& speech,
    AudioPreviewController& preview,
    AudioExportController& exporter,
    QObject& root,
    const QString& output_directory
) {
    if (!require(
            QDir(output_directory).exists() && !backend.projectOpen(),
            "empty launch and output directory"
        )
        || !require(
            backend.createProject(
                QUrl::fromLocalFile(output_directory),
                QStringLiteral("Audio verification")
            ),
            "create project"
        ))
        return false;
    const QString bundle = backend.bundlePath();
    const QString paragraph = QStringLiteral(
        "今天我们用一段中英混合的文字，检查语音朗读的完整流程。Welcome to Shape. "
        "You can choose a voice, listen to the result, and export the audio. "
        "朗读应当按照原文的顺序，保留每一句话。长文本会分成几个片段，再合成为一个完整的音频文件。"
        "The language setting stays automatic, so there is no need to switch it for each sentence. "
    );
    const QList<QString> texts{
        paragraph.repeated(3)
            + QStringLiteral(
                "这是第二部分。Thank you for listening. 现在可以试听并保存完整的朗读了。"
            ),
        QStringLiteral("你好，这是另一种男声音色。Welcome to Shape. 中英混合朗读使用自动语言设置。")
    };
    for (int index = 0; index < texts.size(); ++index) {
        const int old_count = backend.artifactCount();
        const QString name =
            index == 0 ? QStringLiteral("Mixed narration") : QStringLiteral("Male voice");
        if (!require(backend.createTextScene(name, texts[index]), "create source"))
            return false;
        const auto artifacts = backend.artifacts();
        int source_index = -1;
        for (int i = 0; i < artifacts.size(); ++i)
            if (artifacts[i].toMap().value(QStringLiteral("name")).toString() == name)
                source_index = i;
        if (!require(source_index >= 0, "source projection"))
            return false;
        const auto source = artifacts[source_index].toMap();
        const QString source_id = source.value(QStringLiteral("id")).toString();
        const QString source_head = source.value(QStringLiteral("acceptedRevisionId")).toString();
        root.setProperty("selectedArtifactIndex", source_index);
        QCoreApplication::processEvents();
        QObject* surface = root.findChild<QObject*>(QStringLiteral("workspaceSurface"));
        if (surface)
            QMetaObject::invokeMethod(surface, "showGraph", Qt::DirectConnection);
        QCoreApplication::processEvents();
        QObject* palette = root.findChild<QObject*>(QStringLiteral("operatorPalette"));
        if (!require(
                palette
                    && evaluate(
                        *palette,
                        QStringLiteral("chooseOperator('audio.speech_synthesize')")
                    ),
                "open speech workspace"
            ))
            return false;
        QCoreApplication::processEvents();
        QObject* host = root.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
        QObject* workspace = nullptr;
        waitUntil(
            [&] {
                workspace =
                    host ? qvariant_cast<QObject*>(host->property("loadedWorkspace")) : nullptr;
                return workspace
                       && workspace->objectName() == QStringLiteral("audioSpeechOperatorWorkspace")
                       && workspace->property("sourceArtifactId").toString() == source_id
                       && !workspace->property("operatorDraftId").toString().isEmpty();
            },
            3'000
        );
        if (!require(workspace != nullptr, "packaged speech workspace"))
            return false;
        QObject* voice = workspace->findChild<QObject*>(QStringLiteral("speechVoiceSelector"));
        QObject* language =
            workspace->findChild<QObject*>(QStringLiteral("speechLanguageSelector"));
        if (!require(
                voice && language && voice->property("count").toInt() == 9
                    && language->property("currentIndex").toInt() == 0
                    && workspace->property("language").toString() == QStringLiteral("auto"),
                "nine voices and automatic language"
            ))
            return false;
        if (index == 1 && !evaluate(*workspace, QStringLiteral("chooseVoice(2)")))
            return false;
        if (index == 1)
            workspace->setProperty("pendingSpeedMilli", 1150);
        if (!require(
                workspace->property("language").toString() == QStringLiteral("auto"),
                "voice preserves language"
            ))
            return false;
        const auto status = QObject::connect(
            &speech,
            &InferSpeechController::statusChanged,
            &speech,
            [&, last = -1]() mutable {
                if (last != speech.completedSegments()) {
                    last = speech.completedSegments();
                    std::cout << "speech segments " << last << "/" << speech.totalSegments()
                              << std::endl;
                }
            }
        );
        if (!evaluate(*workspace, QStringLiteral("requestSynthesis()")))
            return false;
        const bool started = speech.running();
        const bool completed = waitUntil([&] { return !speech.running(); }, 1'200'000);
        QObject::disconnect(status);
        if (!require(
                started && completed && speech.errorCode().isEmpty() && backend.hasCandidate(),
                "real synthesis"
            )) {
            std::cerr << "speech error: " << speech.errorCode().toStdString() << std::endl;
            speech.cancel();
            return false;
        }
        if (!require(index != 0 || speech.totalSegments() > 1, "long text segmented"))
            return false;
        const QString candidate_id = backend.candidateId();
        const auto audio = backend.audioPreview(source_id, candidate_id);
        if (!require(
                audio.has_value() && backend.artifactCount() == old_count + 1
                    && backend.artifacts()[source_index]
                               .toMap()
                               .value(QStringLiteral("acceptedRevisionId"))
                               .toString()
                           == source_head,
                "candidate preserves accepted history"
            ))
            return false;
        if (!require(preview.loadPreview(source_id, candidate_id), "load WAV playback"))
            return false;
        const qint64 duration_millis = preview.durationMillis();
        preview.togglePlayback();
        if (!require(
                waitUntil([&] { return preview.positionMillis() > 200; }, 10'000)
                    && preview.errorCode().isEmpty(),
                "playback advances"
            ))
            return false;
        preview.seekTo(1000);
        if (!require(
                waitUntil([&] { return preview.positionMillis() >= 1000; }, 5'000),
                "playback seek"
            ))
            return false;
        preview.togglePlayback();
        const QString wav_path = QDir(output_directory)
                                     .filePath(
                                         index == 0 ? QStringLiteral("mixed-vivian.wav")
                                                    : QStringLiteral("mixed-uncle-fu.wav")
                                     );
        if (!exportAndCompare(exporter, source_id, candidate_id, wav_path, audio->wav_bytes))
            return false;
        // A bundle destination must be rejected before any output file is created.
        QString rejected_code;
        auto rejected = QObject::connect(
            &exporter,
            &AudioExportController::finished,
            &exporter,
            [&](bool, const QString&, const QString& code) { rejected_code = code; }
        );
        const QString forbidden = QDir(bundle).filePath(QStringLiteral("forbidden.wav"));
        exporter.exportAudio(source_id, candidate_id, QUrl::fromLocalFile(forbidden));
        QObject::disconnect(rejected);
        if (!require(
                rejected_code == QStringLiteral("export_inside_project")
                    && !QFile::exists(forbidden),
                "project bundle export rejected"
            ))
            return false;
        const QString target_id =
            backend.candidates().first().toMap().value(QStringLiteral("artifactId")).toString();
        if (!require(
                backend.acceptCandidate(candidate_id) && !backend.hasCandidate(),
                "explicit acceptance"
            ))
            return false;
        if (!require(backend.openProject(QUrl::fromLocalFile(bundle)), "reopen project"))
            return false;
        const auto accepted = backend.audioPreview(target_id);
        if (!require(
                accepted.has_value() && accepted->wav_bytes == audio->wav_bytes,
                "accepted WAV survives reopen"
            ))
            return false;
        if (!exportAndCompare(exporter, target_id, QString(), wav_path, audio->wav_bytes))
            return false;
        std::cout << "speech verified voice=" << (index == 0 ? "Vivian" : "Uncle_Fu")
                  << " seconds=" << duration_millis / 1000.0 << " bytes=" << audio->wav_bytes.size()
                  << " path=" << wav_path.toStdString() << std::endl;
    }
    std::cout << "speech live smoke passed; project=" << bundle.toStdString() << std::endl;
    return true;
}
} // namespace speech_live_smoke

namespace speech_live_smoke {
bool run_script(
    DesktopBackend& backend,
    InferSpeechController& speech,
    AudioPreviewController& preview,
    AudioExportController& exporter,
    QObject& root,
    const QString& output_directory
) {
    const QString script = QStringLiteral(
        "[production: listening]\n[role: 中文播音员; language: Chinese]\n[role: 英语朗读; "
        "language: English]\n"
        "[cue: 片头]\n[cue: 结束]\n[cue: 可选音乐]\n"
        "# 五年级英语听力测试\n[音效：片头]\n[角色：中文播音员]\n"
        "听力测试现在开始。请同学们抓紧时间审题。\n[停顿：10秒]\n"
        "一、听音，选择。每小题读两遍。\n[停顿：3秒]\n[scene: question-1]\n"
        "[角色：英语朗读]\nNumber one.\n[repeat: 2; gap: 2s]\n"
        "Look at the boy. He is going to do tai chi in the park this "
        "Sunday.\n[end-repeat]\n[停顿：5秒]\n"
        "[音效：结束]\n[音效：可选音乐]\n[备注：不朗读这句话]\n"
    );
    if (!require(
            !backend.projectOpen()
                && backend.createProject(
                    QUrl::fromLocalFile(output_directory),
                    QStringLiteral("Script verification")
                ),
            "create script project"
        )
        || !require(
            backend
                .createTextAuthoring(QStringLiteral("五年级听力脚本"), QStringLiteral("script")),
            "script source"
        ))
        return false;
    const auto writer = backend.operatorDrafts().first().toMap();
    const auto writerId = writer.value(QStringLiteral("id")).toString();
    auto settings = QJsonDocument::fromJson(
                        writer.value(QStringLiteral("textAuthoringJson")).toString().toUtf8()
    )
                        .object();
    settings.insert(QStringLiteral("entry"), QStringLiteral("manual"));
    settings.insert(QStringLiteral("text"), script);
    if (!require(
            backend.updateTextAuthoring(
                writerId,
                QString::fromUtf8(QJsonDocument(settings).toJson(QJsonDocument::Compact))
            ) && backend.proposeAuthoredText(writerId)
                && backend.acceptCandidate(backend.candidateId()),
            "adopt typed script"
        ))
        return false;
    const QString bundle = backend.bundlePath();
    const auto source = backend.artifacts().first().toMap();
    const QString source_id = source.value(QStringLiteral("id")).toString();
    const QString speechDraft = backend.beginAuthoringSpeech(source_id);
    if (!require(
            !speechDraft.isEmpty()
                && evaluate(root, QStringLiteral("openDraftTarget('%1')").arg(speechDraft)),
            "open script workspace"
        ))
        return false;
    QObject* host = root.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    QObject* workspace = nullptr;
    if (!require(
            waitUntil(
                [&] {
                    workspace =
                        host ? qvariant_cast<QObject*>(host->property("loadedWorkspace")) : nullptr;
                    return workspace
                           && !workspace->property("operatorDraftId").toString().isEmpty();
                },
                3000
            ),
            "script workspace ready"
        ))
        return false;
    const QString draft = workspace->property("operatorDraftId").toString();
    QObject* panel = workspace->findChild<QObject*>(QStringLiteral("speechScriptPanel"));
    QObject* generate = workspace->findChild<QObject*>(QStringLiteral("speechGenerateButton"));
    if (!require(
            panel && generate && !generate->property("enabled").toBool(),
            "unresolved cues prevent generation"
        ))
        return false;
    if (!evaluate(
            *panel,
            QStringLiteral(
                "setRole('中文播音员', 1); setRole('英语朗读', 6); setCue('片头', 1); "
                "setCue('可选音乐', 2)"
            )
        ))
        return false;
    // Short mono 24 kHz PCM16 cue exercises import without depending on a source asset.
    QByteArray cue("RIFF", 4);
    const auto append16 = [&](quint16 value) {
        const auto le = qToLittleEndian(value);
        cue.append(reinterpret_cast<const char*>(&le), 2);
    };
    const auto append32 = [&](quint32 value) {
        const auto le = qToLittleEndian(value);
        cue.append(reinterpret_cast<const char*>(&le), 4);
    };
    append32(36 + 12000);
    cue.append("WAVEfmt ", 8);
    append32(16);
    append16(1);
    append16(1);
    append32(24000);
    append32(48000);
    append16(2);
    append16(16);
    cue.append("data", 4);
    append32(12000);
    for (int i = 0; i < 6000; ++i) {
        const auto sample = static_cast<qint16>(
            3000.0 * std::sin(6.283185307179586 * 880.0 * i / 24000.0) * (1.0 - i / 6000.0)
        );
        append16(static_cast<quint16>(sample));
    }
    const QString cue_path = QDir(output_directory).filePath(QStringLiteral("end-cue.wav"));
    QFile cue_file(cue_path);
    if (!require(
            cue_file.open(QIODevice::WriteOnly) && cue_file.write(cue) == cue.size(),
            "write cue fixture"
        ))
        return false;
    cue_file.close();
    backend.importSpeechCue(draft, QStringLiteral("结束"), QUrl::fromLocalFile(cue_path));
    if (!require(
            waitUntil([&] { return !backend.speechCueImporting(); }, 10000)
                && backend.lastError().isEmpty(),
            "import cue"
        ))
        return false;
    const auto plan = QJsonDocument::fromJson(backend.speechScriptPreview(draft).toUtf8()).object();
    if (!require(
            plan.value(QStringLiteral("ready")).toBool()
                && plan.value(QStringLiteral("pause_ms")).toInt() == 20000,
            "full script ready with 20 seconds of pauses"
        ))
        return false;
    if (!require(
            waitUntil([&] { return generate->property("enabled").toBool(); }, 3000)
                && evaluate(*workspace, QStringLiteral("requestSynthesis()")),
            "generate script from UI"
        ))
        return false;
    if (!require(
            speech.running() && waitUntil([&] { return !speech.running(); }, 1200000)
                && speech.errorCode().isEmpty() && backend.hasCandidate(),
            "real Qwen script completes"
        )) {
        std::cerr << speech.errorCode().toStdString() << std::endl;
        speech.cancel();
        return false;
    }
    const QString candidate = backend.candidateId();
    const QString outputId = backend.candidateArtifactId();
    const auto audio = backend.audioPreview(outputId, candidate);
    if (!require(
            audio && audio->duration_millis > 21250 && speech.totalSegments() == 4
                && backend.artifactCount() == 2,
            "four speech requests and transient output"
        ))
        return false;
    if (!require(preview.loadPreview(outputId, candidate), "script playback loads"))
        return false;
    preview.togglePlayback();
    if (!require(
            waitUntil([&] { return preview.positionMillis() > 200; }, 10000),
            "script playback advances"
        ))
        return false;
    preview.togglePlayback();
    const QString wav_path =
        QDir(output_directory).filePath(QStringLiteral("listening-script.wav"));
    if (!exportAndCompare(exporter, outputId, candidate, wav_path, audio->wav_bytes))
        return false;
    const QString target =
        backend.candidates().first().toMap().value(QStringLiteral("artifactId")).toString();
    if (!require(backend.acceptCandidate(candidate), "accept source-validated script timeline"))
        return false;
    if (!require(backend.openProject(QUrl::fromLocalFile(bundle)), "reopen accepted script"))
        return false;
    const auto accepted = backend.audioPreview(target);
    if (!require(
            accepted && accepted->wav_bytes == audio->wav_bytes,
            "accepted script audio bytes survive reopen"
        ))
        return false;
    if (!exportAndCompare(exporter, target, QString(), wav_path, accepted->wav_bytes))
        return false;
    QFile script_file(QDir(output_directory).filePath(QStringLiteral("listening-script.txt")));
    if (script_file.open(QIODevice::WriteOnly))
        script_file.write(script.toUtf8());
    std::cout << "script live smoke passed: voices=Vivian,Ryan speech_segments=4 "
                 "explicit_pause_ms=20000 duration_ms="
              << audio->duration_millis << " wav=" << wav_path.toStdString() << std::endl;
    return true;
}
} // namespace speech_live_smoke

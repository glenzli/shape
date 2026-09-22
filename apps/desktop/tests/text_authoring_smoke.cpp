#include "text_authoring_smoke.hpp"
#include "audio_export_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_speech_controller.hpp"
#include "infer_text_controller.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTimer>
#include <QVariant>
#include <functional>
#include <iostream>

namespace {
bool check(bool condition, const char* label) {
    if (!condition)
        std::cerr << "text authoring smoke failed: " << label << std::endl;
    return condition;
}
bool waitUntil(const std::function<bool()>& ready, int timeout = 5000) {
    QElapsedTimer timer;
    timer.start();
    while (!ready() && timer.elapsed() < timeout) {
        QEventLoop loop;
        QTimer::singleShot(20, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return ready();
}

QVariant evaluate(QObject& object, const QString& source) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, source);
    const auto value = expression.evaluate();
    if (expression.hasError())
        std::cerr << expression.error().toString().toStdString() << std::endl;
    return expression.hasError() ? QVariant() : value;
}
QQuickItem* visualChild(QQuickItem* root, const QString& name) {
    if (!root)
        return nullptr;
    if (root->objectName() == name)
        return root;
    for (auto* child : root->childItems())
        if (auto* found = visualChild(child, name))
            return found;
    return nullptr;
}
bool click(QObject& owner, const char* name) {
    QObject* item = owner.findChild<QObject*>(QString::fromLatin1(name));
    if (!item)
        item = visualChild(qobject_cast<QQuickItem*>(&owner), QString::fromLatin1(name));
    return check(
        item && item->property("enabled").toBool()
            && QMetaObject::invokeMethod(item, "clicked", Qt::DirectConnection),
        name
    );
}
} // namespace

bool text_authoring_smoke::verify(DesktopBackend& backend, QObject& root) {
    auto* dialog = root.findChild<QObject*>(QStringLiteral("createSceneTypeDialog"));
    auto* host = root.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (!check(dialog && host, "entry and workspace host"))
        return false;
    QMetaObject::invokeMethod(dialog, "open", Qt::DirectConnection);
    QCoreApplication::processEvents();
    auto* freeCard = dialog->findChild<QObject*>(QStringLiteral("createFreeWritingCard"));
    auto* scriptCard = dialog->findChild<QObject*>(QStringLiteral("createScriptCard"));
    if (!check(
            freeCard && scriptCard && freeCard->property("width").toDouble() >= 240
                && scriptCard->property("width").toDouble() >= 240,
            "both creation formats remain readable"
        ))
        return false;
    if (!check(
            !dialog->findChild<QObject*>(QStringLiteral("createListeningScriptButton"))
                && !dialog->findChild<QObject*>(QStringLiteral("createDialogueScriptButton")),
            "script creation has no specialized presets"
        ) || !click(*dialog, "createProductionScriptButton"))
        return false;
    const auto current = [&] { return qvariant_cast<QObject*>(host->property("loadedWorkspace")); };
    if (!check(
            waitUntil([&] {
                return current()
                       && current()->objectName() == QStringLiteral("textAuthoringWorkspace");
            }),
            "entry opens writer directly"
        ))
        return false;
    auto* writer = current();
    auto* modelPicker = writer->findChild<QObject*>(QStringLiteral("writingModelPicker"));
    auto* modelChoice = modelPicker ? modelPicker->findChild<QObject*>(QStringLiteral("aiModelChoice")) : nullptr;
    auto* effortChoice = modelPicker ? modelPicker->findChild<QObject*>(QStringLiteral("aiEffortChoice")) : nullptr;
    if (!check(
            modelChoice && writer->property("selectedModelKey").toString()
                               == writer->property("defaultTextModel").toString()
                && QMetaObject::invokeMethod(
                    modelChoice, "activated", Qt::DirectConnection, Q_ARG(int, 3)
                )
                && writer->property("selectedModelKey").toString()
                       == QStringLiteral("gpt_6_sol")
                && effortChoice
                && QMetaObject::invokeMethod(
                    effortChoice, "activated", Qt::DirectConnection, Q_ARG(int, 3)
                )
                && writer->property("selectedEffortKey").toString() == QStringLiteral("high")
                && QMetaObject::invokeMethod(
                    modelChoice, "activated", Qt::DirectConnection, Q_ARG(int, 0)
                )
                && writer->property("selectedModelKey").toString()
                       == writer->property("defaultTextModel").toString()
                && writer->property("selectedEffortKey").toString().isEmpty(),
            "writing model and effort can override for one run and return to local default"
        ))
        return false;
    if (!check(
            writer->property("profile").toString() == QStringLiteral("script")
                && !writer->findChild<QObject*>(QStringLiteral("scriptExamplelistening"))
                && !QJsonDocument::fromJson(evaluate(*writer, QStringLiteral("stateJson()")).toString().toUtf8()).object().contains(QStringLiteral("example")),
            "new scripts have one format and no writing preset"
        ))
        return false;
    auto* prompt = writer->findChild<QObject*>(QStringLiteral("writingInstructionEditor"));
    auto* generate = writer->findChild<QObject*>(QStringLiteral("writingGenerateButton"));
    if (!check(
            prompt && generate && prompt->property("text").toString().isEmpty()
                && !generate->property("enabled").toBool()
                && !writer->property("hasAcceptedRevision").toBool(),
            "empty first, no accepted placeholder"
        ))
        return false;
    if (!click(*writer, "writingScriptRulesButton"))
        return false;
    auto* help = writer->findChild<QObject*>(QStringLiteral("speechScriptHelpDialog"));
    if (!check(help && help->property("visible").toBool(), "rules available before text"))
        return false;
    QMetaObject::invokeMethod(help, "close", Qt::DirectConnection);
    if (!click(*writer, "writingMode_manual"))
        return false;
    auto* editor = writer->findChild<QObject*>(QStringLiteral("writingManualEditor"));
    if (!check(editor, "manual editor"))
        return false;
    const auto oldSize = QSize(root.property("width").toInt(), root.property("height").toInt());
    root.setProperty("width", 1120);
    root.setProperty("height", 740);
    editor->setProperty(
        "text",
        QStringLiteral("A long bilingual paragraph. 中英文混合。\n").repeated(350)
    );
    if (!check(
            waitUntil([&] { return editor->property("canScroll").toBool(); }),
            "long text scrolls inside its editor"
        ))
        return false;
    evaluate(*editor, QStringLiteral("focusEditor()"));
    editor->setProperty("cursorPosition", editor->property("text").toString().size());
    if (!check(
            waitUntil([&] {
                return evaluate(
                           *editor,
                           QStringLiteral(
                               "contentItem.contentY > 0 && contentItem.contentY + availableHeight "
                               ">= contentHeight - 24"
                           )
                )
                    .toBool();
            }),
            "typing at the end keeps the caret in view"
        ))
        return false;
    auto* scroll = writer->findChild<QObject*>(QStringLiteral("writingScroll"));
    const auto originalScrollHeight = scroll ? scroll->property("height") : QVariant();
    if (scroll)
        scroll->setProperty("height", 600);
    QCoreApplication::processEvents();
    if (!check(
            scroll && editor->property("height").toDouble() <= 380
                && scroll->property("contentHeight").toDouble()
                       > scroll->property("availableHeight").toDouble(),
            "bounded editor and independently scrollable form"
        ))
        return false;
    auto* form = writer->findChild<QObject*>(QStringLiteral("writingForm"));
    if (!check(
            waitUntil([&] {
                return form
                       && form->property("width").toDouble()
                              <= scroll->property("availableWidth").toDouble();
            }),
            "form fits the viewport horizontally"
        ))
        return false;
    evaluate(
        *scroll,
        QStringLiteral("contentItem.contentY = Math.max(0, contentHeight - availableHeight)")
    );
    std::cout << "layout viewport=" << scroll->property("availableWidth").toDouble()
              << " form=" << form->property("width").toDouble()
              << " editor=" << editor->property("height").toDouble() << std::endl;
    QEventLoop settle;
    QTimer::singleShot(200, &settle, &QEventLoop::quit);
    settle.exec();
    evaluate(
        *scroll,
        QStringLiteral("contentItem.contentY = Math.max(0, contentHeight - availableHeight)")
    );
    auto* rulesButton = writer->findChild<QObject*>(QStringLiteral("writingScriptRulesButton"));
    if (!check(
            rulesButton
                && rulesButton->property("width").toDouble()
                       <= rulesButton->property("implicitWidth").toDouble() + 1,
            "action buttons keep natural width"
        ))
        return false;
    if (const QString screenshots = qEnvironmentVariable("SHAPE_LAYOUT_SMOKE_DIR");
        !screenshots.isEmpty()) {
        QDir().mkpath(screenshots);
        if (auto* window = qobject_cast<QQuickWindow*>(&root))
            window->grabWindow().save(screenshots + QStringLiteral("/long-editor.png"));
    }
    scroll->setProperty("height", originalScrollHeight);
    root.setProperty("width", oldSize.width());
    root.setProperty("height", oldSize.height());
    editor->setProperty("text", QStringLiteral("[pause: invalid]\nHello."));
    evaluate(*writer, QStringLiteral("updatePreview()"));
    auto* adopt = writer->findChild<QObject*>(QStringLiteral("writingAdoptButton"));
    if (!check(adopt && !adopt->property("enabled").toBool(), "invalid format blocks adoption"))
        return false;
    const QString script =
        QStringLiteral(
            "[role: Narrator]\n# Listening exercise\n[角色：Narrator]\nHello, 世界.\n[停顿：2秒]"
        )
        + QStringLiteral("\nThe boy is in the park. 中英混合正文。\n[pause: 1s]\n").repeated(60);
    editor->setProperty("text", script);
    evaluate(*writer, QStringLiteral("reviewing = true; editOutput()"));
    if (!check(editor->property("text").toString() == script, "manual review returns exact draft"))
        return false;
    if (!check(evaluate(*writer, QStringLiteral("checkpoint()")).toBool(), "save authored draft"))
        return false;
    evaluate(*writer, QStringLiteral("updatePreview()"));
    const QString artifactId = writer->property("artifactId").toString();
    if (!check(
            backend.textAuthoringContent(artifactId, QString()).isEmpty(),
            "save is not acceptance"
        ))
        return false;
    if (!click(*writer, "writingAdoptButton"))
        return false;
    if (!check(
            waitUntil([&] {
                return current()
                       && current()->objectName() == QStringLiteral("audioSpeechOperatorWorkspace");
            }),
            "adopt opens voices"
        ))
        return false;
    if (!check(
            current()->property("scriptMode").toBool()
                && backend.textAuthoringContent(artifactId, QString()) == script,
            "exact script accepted and mode selected"
        ))
        return false;
    root.setProperty("width", 1120);
    root.setProperty("height", 720);
    auto* voiceScroll = current()->findChild<QObject*>(QStringLiteral("speechColumn1"));
    if (!check(
            voiceScroll && waitUntil([&] {
                return voiceScroll->property("height").toDouble() > 100
                       && voiceScroll->property("availableWidth").toDouble() > 100;
            }),
            "voice settings remain bounded in the minimum window"
        ))
        return false;
    evaluate(
        *voiceScroll,
        QStringLiteral("contentItem.contentY = Math.max(0, contentHeight - availableHeight)")
    );
    if (const QString screenshots = qEnvironmentVariable("SHAPE_LAYOUT_SMOKE_DIR");
        !screenshots.isEmpty()) {
        QEventLoop settleAudio;
        QTimer::singleShot(200, &settleAudio, &QEventLoop::quit);
        settleAudio.exec();
        if (auto* window = qobject_cast<QQuickWindow*>(&root))
            window->grabWindow().save(screenshots + QStringLiteral("/audio-scroll.png"));
    }
    root.setProperty("width", oldSize.width());
    root.setProperty("height", oldSize.height());
    auto* rolePanel = current()->findChild<QObject*>(QStringLiteral("speechScriptPanel"));
    if (!check(
            rolePanel && waitUntil([&] {
                return rolePanel->property("contentHeight").toDouble()
                       > rolePanel->property("height").toDouble();
            }),
            "long playback plan scrolls inside the source panel"
        ))
        return false;
    if (!check(
            rolePanel && !evaluate(*rolePanel, QStringLiteral("preview.ready")).toBool(),
            "unbound named role blocks synthesis"
        ))
        return false;
    evaluate(*rolePanel, QStringLiteral("setRole('Narrator', 1)"));
    if (!check(
            waitUntil([&] {
                return evaluate(*rolePanel, QStringLiteral("preview.ready")).toBool();
            }),
            "explicit role binding enables synthesis"
        ))
        return false;
    if (!click(*current(), "speechReturnToWritingButton"))
        return false;
    if (!check(
            waitUntil([&] {
                return current()
                       && current()->objectName() == QStringLiteral("textAuthoringWorkspace");
            }),
            "return to writer"
        ))
        return false;
    if (!check(
            current()->property("profile").toString() == QStringLiteral("script")
                && current()->property("hasAcceptedRevision").toBool(),
            "script format survives accepted-source transition"
        ))
        return false;
    auto* palette = root.findChild<QObject*>(QStringLiteral("operatorPalette"));
    auto* search =
        palette ? palette->findChild<QObject*>(QStringLiteral("operatorSearchField")) : nullptr;
    if (!check(palette && search, "node palette available"))
        return false;
    QMetaObject::invokeMethod(palette, "open", Qt::DirectConnection);
    search->setProperty("text", QStringLiteral("摘要"));
    if (!check(
            waitUntil([&] {
                return palette->property("visibleOperatorCount").toInt() >= 1
                       && evaluate(
                              *palette,
                              QStringLiteral("filteredOperators[0].typeKey === 'text.summarize'")
                       )
                              .toBool();
            }),
            "Chinese search ranks the summary template first"
        ))
        return false;
    evaluate(*palette, QStringLiteral("chooseHighlighted()"));
    if (!check(
            waitUntil([&] {
                return current()
                       && current()->objectName() == QStringLiteral("textAuthoringWorkspace")
                       && current()->property("mode").toString() == QStringLiteral("summarize");
            }),
            "palette selection opens configured summary node"
        ))
        return false;
    return check(
        current()->property("artifactId").toString() != artifactId
            && backend.textAuthoringContent(artifactId, QString()) == script,
        "template output is separate and source remains unchanged"
    );
}

bool text_authoring_smoke::runLive(
    DesktopBackend& backend,
    InferTextController& text,
    InferSpeechController& speech,
    AudioExportController& exporter,
    QObject& root,
    const QString& outputDirectory
) {
    if (!check(
            !backend.projectOpen()
                && backend.createProject(
                    QUrl::fromLocalFile(outputDirectory),
                    QStringLiteral("AI listening workspace")
                ),
            "live project"
        ))
        return false;
    auto* dialog = root.findChild<QObject*>(QStringLiteral("createSceneTypeDialog"));
    auto* host = root.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (!check(dialog && host, "live entry"))
        return false;
    QMetaObject::invokeMethod(dialog, "open", Qt::DirectConnection);
    QCoreApplication::processEvents();
    const auto capture = [&](const char* name) {
        QCoreApplication::processEvents();
        if (auto* window = qobject_cast<QQuickWindow*>(&root))
            window->grabWindow().save(QDir(outputDirectory).filePath(QString::fromLatin1(name)));
    };
    capture("01-create.png");
    if (!click(*dialog, "createProductionScriptButton"))
        return false;
    const auto current = [&] { return qvariant_cast<QObject*>(host->property("loadedWorkspace")); };
    if (!check(
            waitUntil([&] {
                return current()
                       && current()->objectName() == QStringLiteral("textAuthoringWorkspace");
            }),
            "live writer"
        ))
        return false;
    auto* writer = current();
    auto* prompt = writer->findChild<QObject*>(QStringLiteral("writingInstructionEditor"));
    auto* generate = writer->findChild<QObject*>(QStringLiteral("writingGenerateButton"));
    if (!check(prompt && generate, "live writing controls"))
        return false;
    prompt->setProperty(
        "text",
        QStringLiteral(
            "写一个极短的五年级英语听力练习。中文开场只说：请听录音。英语题目只说：Number one. The "
            "boy will play football this Sunday. "
            "题末使用[audio: answer]提示音并停顿3秒；先声明beep提示音。题目使用[repeat: 2; gap: 2s]复读块。不要答案、选项、结束语或其他说明。"
        )
    );
    if (!check(waitUntil([&] { return generate->property("enabled").toBool(); }), "AI ready"))
        return false;
    capture("02-write.png");
    const auto beforeWidth = generate->property("width").toDouble();
    const auto beforeHeight = generate->property("height").toDouble();
    if (!click(*writer, "writingGenerateButton") || !check(text.running(), "AI generation started"))
        return false;
    capture("03-writing.png");
    if (!check(
            generate->property("width").toDouble() == beforeWidth
                && generate->property("height").toDouble() == beforeHeight,
            "busy button geometry"
        ))
        return false;
    std::cout << "authoring live: AI writing started" << std::endl;
    if (!check(
            waitUntil([&] { return !text.running(); }, 240000) && text.errorCode().isEmpty(),
            "real AI completed"
        )) {
        std::cerr << text.errorCode().toStdString() << std::endl;
        return false;
    }
    waitUntil([&] { return !writer->property("outputText").toString().isEmpty(); });
    evaluate(*writer, QStringLiteral("updatePreview()"));
    if (!evaluate(*writer, QStringLiteral("preview.valid")).toBool()) {
        QFile invalid(QDir(outputDirectory).filePath(QStringLiteral("initial-script.txt")));
        if (invalid.open(QIODevice::WriteOnly))
            invalid.write(writer->property("outputText").toString().toUtf8());
        capture("03-format-diagnostics.png");
        evaluate(*writer, QStringLiteral("repairScript(); generate()"));
        if (!check(
                waitUntil([&] { return !text.running(); }, 240000) && text.errorCode().isEmpty(),
                "AI format repair completed"
            ))
            return false;
        evaluate(*writer, QStringLiteral("updatePreview()"));
    }
    if (!check(
            waitUntil([&] { return evaluate(*writer, QStringLiteral("preview.valid")).toBool(); }),
            "AI production rules valid"
        ))
        return false;
    const QString sourceId = writer->property("artifactId").toString();
    const QString candidateId = backend.candidateId();
    const QString generated = backend.textAuthoringContent(sourceId, candidateId);
    if (!check(
            generated.count(QStringLiteral("The boy will play football this Sunday.")) == 1,
            "AI follows the selected repetition count"
        ))
        return false;
    if (!check(
            !generated.isEmpty() && backend.textAuthoringContent(sourceId, QString()).isEmpty(),
            "AI result remains candidate"
        ))
        return false;
    QFile script(QDir(outputDirectory).filePath(QStringLiteral("generated-script.txt")));
    if (!check(
            script.open(QIODevice::WriteOnly)
                && script.write(generated.toUtf8()) == generated.toUtf8().size(),
            "save generated evidence"
        ))
        return false;
    script.close();
    capture("04-review.png");
    if (!click(*writer, "writingAdoptButton"))
        return false;
    if (!check(
            waitUntil([&] {
                return current()
                       && current()->objectName() == QStringLiteral("audioSpeechOperatorWorkspace");
            }),
            "live voices"
        ))
        return false;
    auto* voices = current();
    if (!check(voices->property("scriptMode").toBool(), "automatic script handoff"))
        return false;
    const auto plan =
        QJsonDocument::fromJson(
            backend.speechScriptPreview(voices->property("operatorDraftId").toString()).toUtf8()
        )
            .object();
    if (!check(
            plan.value(QStringLiteral("pause_ms")).toInt() >= 3000,
            "AI includes requested real pause"
        ))
        return false;
    auto* rolePanel = voices->findChild<QObject*>(QStringLiteral("speechScriptPanel"));
    if (!check(rolePanel, "live role bindings"))
        return false;
    evaluate(*rolePanel, QStringLiteral("setRole('Narrator', 1); setRole('Reader', 6)"));
    auto* speechGenerate = voices->findChild<QObject*>(QStringLiteral("speechGenerateButton"));
    if (!check(
            speechGenerate
                && waitUntil([&] { return speechGenerate->property("enabled").toBool(); }),
            "script ready for speech"
        ))
        return false;
    capture("05-voices.png");
    if (!click(*voices, "speechGenerateButton") || !check(speech.running(), "speech started"))
        return false;
    std::cout << "authoring live: script adopted and speech started" << std::endl;
    if (!check(
            waitUntil([&] { return !speech.running(); }, 600000) && speech.errorCode().isEmpty(),
            "real speech completed"
        )) {
        std::cerr << speech.errorCode().toStdString() << std::endl;
        speech.cancel();
        return false;
    }
    const QString audioCandidate = backend.candidateId();
    const QString audioOutputId = backend.candidateArtifactId();
    const auto audio = backend.audioPreview(audioOutputId, audioCandidate);
    if (!check(audio && audio->duration_millis > 3000, "audio candidate"))
        return false;
    bool done = false;
    bool exported = false;
    auto connection = QObject::connect(
        &exporter,
        &AudioExportController::finished,
        &exporter,
        [&](bool success, const QString&, const QString&) {
            done = true;
            exported = success;
        }
    );
    const QString wavPath = QDir(outputDirectory).filePath(QStringLiteral("listening.wav"));
    exporter.exportAudio(audioOutputId, audioCandidate, QUrl::fromLocalFile(wavPath));
    const bool finished = waitUntil([&] { return done; });
    QObject::disconnect(connection);
    QFile wav(wavPath);
    if (!check(
            finished && exported && wav.open(QIODevice::ReadOnly)
                && wav.readAll() == audio->wav_bytes,
            "live exact WAV export"
        ))
        return false;
    capture("06-audio.png");
    if (!check(backend.acceptCandidate(audioCandidate), "accept recording"))
        return false;
    QJsonObject evidence{
        {"ai_script_valid", true},
        {"script_auto_enabled", true},
        {"button_geometry_stable", true},
        {"wav_bytes_verified", true},
        {"repeated_plays", 2},
        {"speech_requests", speech.totalSegments()},
        {"answer_beep", true},
        {"duration_ms", audio->duration_millis},
        {"project", backend.bundlePath()}
    };
    QFile report(QDir(outputDirectory).filePath(QStringLiteral("validation.json")));
    if (!check(
            report.open(QIODevice::WriteOnly) && report.write(QJsonDocument(evidence).toJson()) > 0,
            "live evidence"
        ))
        return false;
    std::cout << "authoring live: AI, speech, export and acceptance passed" << std::endl;
    return true;
}

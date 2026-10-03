//! Offline interaction checks for scoped narration feedback and request ownership.
#include "speech_feedback_smoke.hpp"
#include "desktop_backend.hpp"
#include "infer_speech_controller.hpp"
#include <QCoreApplication>
#include <QDataStream>
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
#include <QTemporaryDir>
#include <QTimer>
#include <functional>
#include <iostream>

namespace {
bool check(bool ok, const char* label) {
    std::cout << "speech feedback " << (ok ? "PASS " : "FAIL ") << label << std::endl;
    return ok;
}
bool waitUntil(const std::function<bool()>& ready) {
    QElapsedTimer timer;
    timer.start();
    while (!ready() && timer.elapsed() < 5000) {
        QEventLoop loop;
        QTimer::singleShot(10, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return ready();
}
QVariant evaluate(QObject& object, const QString& code) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, code);
    auto result = expression.evaluate();
    if (expression.hasError())
        std::cerr << expression.error().toString().toStdString() << std::endl;
    return expression.hasError() ? QVariant() : result;
}
QVariantMap draft(DesktopBackend& backend, const QString& id) {
    for (const auto& value : backend.operatorDrafts())
        if (value.toMap().value("id").toString() == id)
            return value.toMap();
    return {};
}
bool savePace(DesktopBackend& backend, const QString& id, int speed) {
    const auto d = draft(backend, id);
    return backend.updateAudioSpeechDraft(
        id,
        d.value("audioSpeechPresetAlias").toString(),
        d.value("audioSpeechPresetCatalogRevision").toString(),
        d.value("audioSpeechLanguage").toString(),
        speed,
        true
    );
}
struct Work {
    QString source;
    QString narration;
};
Work makeWork(DesktopBackend& backend, const QString& name) {
    if (!backend.createTextScene(name, QStringLiteral("[cue: Turn]\nHello.\n[audio: Turn]")))
        return {};
    const QString source = backend.artifacts().last().toMap().value("id").toString();
    const QString narration = backend.beginAuthoringSpeech(source);
    if (narration.isEmpty()
        || !backend.updateSpeechScript(
            narration,
            QStringLiteral(
                R"({"revision":"20260922.2","roles":{},"cues":{"Turn":{"action":"skip"}}})"
            )
        ))
        return {};
    return {source, narration};
}
struct View {
    QObject& root;
    QObject* host;
    explicit View(QObject& window)
        : root(window), host(window.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"))) {}
    QObject* current() const {
        return host ? qvariant_cast<QObject*>(host->property("loadedWorkspace")) : nullptr;
    }
    bool open(const QString& id) {
        evaluate(root, QStringLiteral("openDraftTarget('%1')").arg(id));
        return waitUntil([&] {
            return current() && current()->property("operatorDraftId").toString() == id
                   && evaluate(
                          *current(),
                          QStringLiteral(
                              "pendingSpeedMilli === speedMilli && backend.operatorDrafts.find(d "
                              "=> d.id === operatorDraftId)?.audioSpeechSpeedMilli === speedMilli"
                          )
                   )
                          .toBool();
        });
    }
    QObject* item(const char* name) const {
        return current() ? current()->findChild<QObject*>(QString::fromLatin1(name)) : nullptr;
    }
    bool click(const char* name) const {
        auto* button = item(name);
        return button && button->property("enabled").toBool()
               && QMetaObject::invokeMethod(button, "clicked", Qt::DirectConnection);
    }
    bool capture(const QString& name) const {
        const auto folder = qEnvironmentVariable("SHAPE_LAYOUT_SMOKE_DIR");
        if (folder.isEmpty())
            return true;
        QEventLoop settle;
        QTimer::singleShot(30, &settle, &QEventLoop::quit);
        settle.exec();
        auto* window = qobject_cast<QQuickWindow*>(&root);
        return window
               && window->grabWindow().save(QDir(folder).filePath(name + QStringLiteral(".png")));
    }
    bool hasOperationError() const {
        auto* label = item("speechOperationError");
        return label && label->property("visible").toBool()
               && !label->property("text").toString().isEmpty();
    }
    bool movePace(int speed) const {
        auto* slider = item("speechSpeedSlider");
        return slider && slider->setProperty("value", speed)
               && QMetaObject::invokeMethod(slider, "moved", Qt::DirectConnection);
    }
};
QByteArray smallWav() {
    QByteArray bytes;
    QDataStream stream(&bytes, QIODevice::WriteOnly);
    stream.setByteOrder(QDataStream::LittleEndian);
    stream.writeRawData("RIFF", 4);
    stream << quint32(44);
    stream.writeRawData("WAVEfmt ", 8);
    stream << quint32(16) << quint16(1) << quint16(1) << quint32(24000) << quint32(48000)
           << quint16(2) << quint16(16);
    stream.writeRawData("data", 4);
    stream << quint32(8);
    for (int i = 0; i < 4; ++i)
        stream << qint16(0);
    return bytes;
}
bool operationFeedback(DesktopBackend& backend, View& view, const Work& a, const Work& b) {
    QTemporaryDir files;
    if (!check(files.isValid() && view.open(a.narration), "open synthetic narration A"))
        return false;
    const auto original = backend.textAuthoringContent(a.source, {});
    QFile invalid(files.filePath(QStringLiteral("invalid.wav")));
    if (!invalid.open(QIODevice::WriteOnly))
        return false;
    invalid.write(QByteArray(64, '\0'));
    invalid.close();
    backend.importSpeechCue(
        a.narration,
        QStringLiteral("Turn"),
        QUrl::fromLocalFile(invalid.fileName())
    );
    if (!check(
            waitUntil([&] { return !backend.speechCueImporting(); })
                && !backend.lastError().isEmpty(),
            "invalid cue fails through the real asynchronous import"
        ))
        return false;
    if (!check(view.hasOperationError(), "cue import failure is visible beside narration controls"))
        return false;
    if (!check(view.capture(QStringLiteral("cue-error-en")), "capture the visible import failure"))
        return false;
    evaluate(view.root, QStringLiteral("uiPreferences.languageMode = 'zh_CN'"));
    QCoreApplication::processEvents();
    backend.importSpeechCue(
        a.narration,
        QStringLiteral("Turn"),
        QUrl::fromLocalFile(invalid.fileName())
    );
    if (!check(
            waitUntil([&] { return !backend.speechCueImporting(); }) && view.hasOperationError()
                && view.item("speechOperationError")
                       ->property("text")
                       .toString()
                       .contains(QStringLiteral("选择"))
                && view.capture(QStringLiteral("cue-error-zh")),
            "repeat failure is actionable in Chinese on the same workspace"
        ))
        return false;
    evaluate(view.root, QStringLiteral("uiPreferences.languageMode = 'en'"));
    QCoreApplication::processEvents();

    if (!check(
            savePace(backend, b.narration, 1050) && view.hasOperationError(),
            "unrelated narration success cannot erase A's error"
        ))
        return false;
    if (!check(
            view.open(b.narration) && !view.hasOperationError(),
            "A's operation error does not leak into B"
        ))
        return false;
    if (!view.open(a.narration))
        return false;
    QFile valid(files.filePath(QStringLiteral("valid.wav")));
    if (!valid.open(QIODevice::WriteOnly))
        return false;
    valid.write(smallWav());
    valid.close();
    backend.importSpeechCue(
        a.narration,
        QStringLiteral("Turn"),
        QUrl::fromLocalFile(valid.fileName())
    );
    if (!check(
            waitUntil([&] { return !backend.speechCueImporting(); }) && !view.hasOperationError()
                && draft(backend, a.narration)
                       .value("audioSpeechScriptJson")
                       .toString()
                       .contains(QStringLiteral("audio")),
            "valid cue retry stores the cue and clears its error"
        ))
        return false;
    const QString bundle = backend.bundlePath();
    DesktopBackend peer;
    if (!check(
            peer.openProject(QUrl::fromLocalFile(bundle)) && savePace(peer, a.narration, 1050)
                && view.movePace(1100),
            "concurrent editor changes A before the normal pace autosave"
        ))
        return false;
    if (!check(
            waitUntil([&] { return view.hasOperationError(); })
                && backend.lastError().contains(QStringLiteral("another session")),
            "conflicted pace save explains recovery without overwriting the peer"
        ))
        return false;
    if (!check(
            backend.openProject(QUrl::fromLocalFile(bundle)) && view.open(a.narration)
                && view.movePace(1150) && waitUntil([&] {
                       return draft(backend, a.narration).value("audioSpeechSpeedMilli").toInt()
                              == 1150;
                   })
                && !view.hasOperationError(),
            "reopen and successful pace save clear the failure"
        ))
        return false;
    if (!check(
            backend.proposeTextCandidate(a.source, original + QStringLiteral("\nMore words."))
                && backend.acceptCandidate(backend.candidateId()) && view.open(a.narration)
                && waitUntil([&] { return !view.current()->property("inputCurrent").toBool(); }),
            "new source leaves narration with an explicitly stale input"
        ))
        return false;
    const QString pinned = view.current()->property("inputRevisionId").toString();
    if (!check(
            peer.openProject(QUrl::fromLocalFile(bundle)) && savePace(peer, a.narration, 1200)
                && view.click("speechRefreshInputButton"),
            "refresh races a second editor through its real button"
        ))
        return false;
    if (!check(
            view.hasOperationError()
                && view.current()->property("inputRevisionId").toString() == pinned,
            "failed refresh is visible and leaves the pinned input unchanged"
        ))
        return false;
    if (!check(
            backend.openProject(QUrl::fromLocalFile(bundle)) && view.open(a.narration)
                && waitUntil([&] { return view.current()->property("speedMilli").toInt() == 1200; })
                && view.click("speechRefreshInputButton")
                && waitUntil([&] { return view.current()->property("inputCurrent").toBool(); })
                && !view.hasOperationError(),
            "reopened refresh succeeds and clears its own failure"
        ))
        return false;
    return check(
        backend.textAuthoringContent(a.source, {}) == original + QStringLiteral("\nMore words."),
        "feedback recovery preserves the exact accepted source"
    );
}
bool requestOwnership(
    DesktopBackend& backend,
    InferSpeechController& speech,
    View& view,
    const Work& a,
    const Work& b
) {
    if (!view.open(a.narration))
        return false;
    bool entered = false;
    bool passed = true;
    // Inspect normal admission synchronously before the worker starts. No fake model,
    // injected status or credentials: the offline worker ends at credential_missing.
    const auto connection =
        QObject::connect(&speech, &InferSpeechController::statusChanged, &view.root, [&] {
            if (!speech.running() || entered)
                return;
            entered = true;
            if (!check(view.open(b.narration), "open B while A owns speech admission")) {
                passed = false;
                return;
            }
            auto* name = view.item("speechArtifactNameField");
            const QString expectedName = view.current()->property("outputName").toString();
            passed &= check(
                name && expectedName.startsWith(QStringLiteral("Speech feedback B"))
                    && name->property("text").toString() == expectedName,
                "reused narration workspace displays B's own output name"
            );
            if (name)
                QMetaObject::invokeMethod(name, "editingFinished", Qt::DirectConnection);
            passed &= check(
                view.current()->property("outputName").toString() == expectedName,
                "finishing B's name field cannot copy A's stale name"
            );
            auto* generate = view.item("speechGenerateButton");
            auto* stop = view.item("speechStopButton");
            passed &= check(
                generate && !generate->property("busy").toBool() && stop
                    && !stop->property("visible").toBool(),
                "B shows neither A's spinner nor A's stop control"
            );
            auto* otherWork = view.item("speechOtherWorkRunning");
            passed &= check(
                otherWork && otherWork->property("visible").toBool()
                    && view.capture(QStringLiteral("other-work-running")),
                "B explains which work owns the busy executor"
            );
            const QString priorError = speech.errorCode();
            speech.generate(
                backend.bundlePath(),
                b.source,
                b.narration,
                QStringLiteral("Rejected B")
            );
            passed &= check(
                speech.requestDraftId() == a.narration && speech.errorCode() == priorError,
                "rejected B generation cannot steal A's status"
            );
            speech.cancelFor(
                backend.bundlePath() + QStringLiteral(".different-project"),
                a.narration
            );
            passed &= check(
                !speech.cancelling(),
                "a different project cannot cancel A with its draft ID"
            );

            if (stop)
                QMetaObject::invokeMethod(stop, "clicked", Qt::DirectConnection);
            passed &= check(!speech.cancelling(), "a stale B stop callback cannot cancel A");
            if (!check(view.open(a.narration), "return to owning narration A")) {
                passed = false;
                return;
            }
            stop = view.item("speechStopButton");
            passed &= check(
                stop && stop->property("visible").toBool() && view.click("speechStopButton")
                    && speech.cancelling(),
                "A retains its progress and stop authority"
            );
            passed &= view.open(b.narration);
        });
    speech.generate(
        backend.bundlePath(),
        a.source,
        a.narration,
        QStringLiteral("Synthetic narration A")
    );
    const bool finished = waitUntil([&] { return !speech.running(); });
    QObject::disconnect(connection);
    passed &= check(
        entered && finished && speech.errorCode() == QStringLiteral("credential_missing"),
        "offline generation stops before any service request"
    );
    auto* error = view.item("speechGenerationError");
    passed &=
        check(error && !error->property("visible").toBool(), "A's terminal error is absent in B");
    if (!view.open(a.narration))
        return false;
    error = view.item("speechGenerationError");
    passed &= check(
        error && error->property("visible").toBool()
            && error->property("text").toString().contains(QStringLiteral("Infer access")),
        "A retains its actionable terminal failure"
    );
    return passed;
}
} // namespace
bool speech_feedback_smoke::verify(
    DesktopBackend& backend,
    InferSpeechController& speech,
    QObject& root
) {
    const QString language =
        evaluate(root, QStringLiteral("uiPreferences.languageMode")).toString();
    evaluate(root, QStringLiteral("uiPreferences.languageMode = 'en'"));
    const Work a = makeWork(backend, QStringLiteral("Speech feedback A"));
    const Work b = makeWork(backend, QStringLiteral("Speech feedback B"));
    if (!check(
            !a.narration.isEmpty() && !b.narration.isEmpty(),
            "two independent synthetic narration works"
        ))
        return false;
    View view(root);
    const bool feedback = operationFeedback(backend, view, a, b);
    const bool ownership = requestOwnership(backend, speech, view, a, b);
    evaluate(root, QStringLiteral("uiPreferences.languageMode = '%1'").arg(language));
    return feedback && ownership;
}

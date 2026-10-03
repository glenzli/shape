//! Real offscreen desktop stage/candidate lifecycle, using synthetic pixels/WAV only.
#include "material_edit_smoke.hpp"
#include "audio_preview_controller.hpp"
#include "desktop_backend.hpp"
#include "material_edit_controller.hpp"
#include <QCoreApplication>
#include <QDataStream>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QImage>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTemporaryDir>
#include <QTimer>
#include <functional>
#include <iostream>
namespace {
bool check(bool value, const char* label) {
    std::cout << "material stage " << (value ? "PASS " : "FAIL ") << label << std::endl;
    return value;
}
bool wait(const std::function<bool()>& done) {
    QElapsedTimer timer;
    timer.start();
    while (!done() && timer.elapsed() < 10000) {
        QEventLoop loop;
        QTimer::singleShot(5, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return done();
}
QVariant eval(QObject& object, const QString& code) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, code);
    auto result = expression.evaluate();
    if (expression.hasError()) {
        std::cerr << expression.error().toString().toStdString() << std::endl;
        return {};
    }
    return result;
}
QString output(DesktopBackend& backend, const QString& draft) {
    for (const auto& item : backend.operatorDrafts())
        if (item.toMap().value("id").toString() == draft)
            return item.toMap().value("contextArtifactId").toString();
    return {};
}
QString candidate(DesktopBackend& backend, const QString& artifact) {
    for (const auto& item : backend.candidates())
        if (item.toMap().value("artifactId").toString() == artifact)
            return item.toMap().value("id").toString();
    return {};
}
QQuickItem* visualItem(QQuickItem* root, const QString& name) {
    if (!root)
        return nullptr;
    if (root->objectName() == name)
        return root;
    for (auto child : root->childItems())
        if (auto found = visualItem(child, name))
            return found;
    return nullptr;
}
QObject* findVisual(QObject& root, const QString& name) {
    const auto window = qobject_cast<QQuickWindow*>(&root);
    return visualItem(window ? window->contentItem() : qobject_cast<QQuickItem*>(&root), name);
}
QObject* current(QObject& root) {
    auto host = root.findChild<QObject*>("operatorWorkspaceHost");
    return host ? qvariant_cast<QObject*>(host->property("loadedWorkspace")) : nullptr;
}
bool open(QObject& root, const QString& draft) {
    eval(root, QStringLiteral("openDraftTarget('%1')").arg(draft));
    return wait([&] {
        auto view = current(root);
        return view && view->objectName() == "materialEditWorkspace"
               && view->property("operatorDraftId").toString() == draft;
    });
}
bool adoptVisible(QObject& root, DesktopBackend& backend, const QString& artifact) {
    const auto id = candidate(backend, artifact);
    auto button = findVisual(root, "filmstripAcceptButton");
    if (id.isEmpty() || !button || !button->property("visible").toBool()
        || !button->property("enabled").toBool() || !QMetaObject::invokeMethod(button, "clicked"))
        return false;
    return wait([&] { return candidate(backend, artifact).isEmpty(); });
}
bool capture(QObject& root, const QString& name) {
    const auto directory = qEnvironmentVariable("SHAPE_LAYOUT_SMOKE_DIR");
    if (directory.isEmpty())
        return true;
    auto window = qobject_cast<QQuickWindow*>(&root);
    if (!window)
        return false;
    window->requestUpdate();
    QEventLoop loop;
    QTimer::singleShot(150, &loop, &QEventLoop::quit);
    loop.exec();
    return window->grabWindow().save(directory + "/" + name + ".png");
}
bool writeWav(const QString& path, int bits) {
    QByteArray samples;
    for (int index = 0; index < 128; ++index) {
        const int value = bits == 16 ? index * 32 : index * 16385;
        for (int byte = 0; byte < bits / 8; ++byte)
            samples.append(static_cast<char>((value >> (8 * byte)) & 255));
    }
    QByteArray bytes;
    QDataStream stream(&bytes, QIODevice::WriteOnly);
    stream.setByteOrder(QDataStream::LittleEndian);
    stream.writeRawData("RIFF", 4);
    stream << quint32(36 + samples.size());
    stream.writeRawData("WAVEfmt ", 8);
    stream << quint32(16) << quint16(1) << quint16(1) << quint32(24000) << quint32(24000 * bits / 8)
           << quint16(bits / 8) << quint16(bits);
    stream.writeRawData("data", 4);
    stream << quint32(samples.size());
    stream.writeRawData(samples.constData(), samples.size());
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}
bool audioFlow(
    DesktopBackend& backend,
    MaterialEditController& controller,
    QObject& root,
    const QString& imageDraft,
    const QTemporaryDir& files
) {
    const auto path = files.filePath("material-tone.wav");
    const auto returned = files.filePath("echo-pcm24.wav");
    if (!check(
            writeWav(path, 16) && writeWav(returned, 24)
                && backend.importMaterial(QUrl::fromLocalFile(path)),
            "synthetic audio import"
        ))
        return false;
    const auto source = backend.artifacts().last().toMap();
    const auto sourceId = source.value("id").toString();
    const auto originalHead = source.value("acceptedRevisionId").toString();
    const auto draft = backend.beginOperatorDraft(sourceId, "audio.external_edit");
    const auto aid = output(backend, draft), project = backend.bundlePath();
    if (!check(
            open(root, draft) && !current(root)->property("imageMode").toBool(),
            "audio stage reaches its actual workspace"
        ))
        return false;
    auto conversion = current(root)->findChild<QObject*>("materialPcm24Conversion");
    auto review = current(root)->findChild<QObject*>("materialReviewReturn");
    if (!check(
            conversion && review && !conversion->property("checked").toBool(),
            "PCM24 conversion requires an explicit choice"
        ))
        return false;
    controller.exportCopy(project, aid, files.path());
    if (!check(
            wait([&] { return !controller.running(); }) && !controller.handoffPath().isEmpty(),
            "audio working copy exported"
        ))
        return false;
    current(root)->setProperty("handoffFile", controller.handoffPath());
    current(root)->setProperty("returnedFile", returned);
    if (!check(
            review->property("enabled").toBool() && QMetaObject::invokeMethod(review, "clicked"),
            "audio review button is reachable"
        ))
        return false;
    if (!check(
            wait([&] { return !controller.running(); }) && candidate(backend, aid).isEmpty()
                && controller.errorCode() == "material_return_invalid",
            "strict UI rejects PCM24 without silently reducing precision"
        ))
        return false;
    conversion->setProperty("checked", true);
    QMetaObject::invokeMethod(review, "clicked");
    if (!check(
            wait([&] { return !controller.running(); }) && !candidate(backend, aid).isEmpty()
                && current(root)->property("hasCandidate").toBool(),
            "explicit UI conversion returns a reviewable audio candidate"
        ))
        return false;
    AudioPreviewController preview(backend);
    if (!check(
            preview.loadPreview(sourceId, {}, originalHead) && preview.hasAudio()
                && preview.sampleRateHz() == 24000
                && preview.loadPreview(aid, candidate(backend, aid)) && preview.hasAudio(),
            "original and converted return both decode for audition"
        ))
        return false;
    if (!check(capture(root, "material-audio-review"), "capture audio review workspace"))
        return false;
    if (!check(
            adoptVisible(root, backend, aid),
            "adopt converted audio through the visible action"
        ))
        return false;
    bool originalKept = false;
    for (const auto& item : backend.artifacts()) {
        const auto artifact = item.toMap();
        if (artifact.value("id").toString() == sourceId)
            originalKept = artifact.value("acceptedRevisionId").toString() == originalHead;
    }
    if (!check(
            originalKept && open(root, imageDraft) && open(root, draft),
            "audio original survives adoption and stage navigation"
        ))
        return false;
    conversion = current(root)->findChild<QObject*>("materialPcm24Conversion");
    return check(
        conversion && !conversion->property("checked").toBool(),
        "conversion choice resets when returning to an editing stage"
    );
}
} // namespace
bool material_edit_smoke::verify(
    DesktopBackend& backend,
    MaterialEditController& controller,
    QObject& root
) {
    QTemporaryDir files;
    const auto sourcePath = files.filePath("material-original.png"),
               returnPath = files.filePath("material-return.png");
    QImage image(320, 200, QImage::Format_RGBA8888);
    image.fill(QColor(35, 70, 110));
    if (!check(
            files.isValid() && image.save(sourcePath)
                && backend.importRaster(QUrl::fromLocalFile(sourcePath)),
            "synthetic image import"
        ))
        return false;
    const auto source = backend.artifacts().last().toMap();
    const auto sourceId = source.value("id").toString();
    const auto originalHead = source.value("acceptedRevisionId").toString();
    const auto a = backend.beginOperatorDraft(sourceId, "image.external_edit"),
               b = backend.beginOperatorDraft(sourceId, "image.external_edit");
    const auto aid = output(backend, a), bid = output(backend, b), project = backend.bundlePath();
    if (!check(
            !aid.isEmpty() && aid != bid && aid != sourceId && open(root, a),
            "separate material stages and reachable workspace"
        ))
        return false;
    if (!check(
            wait([&] {
                const auto sourceView = findVisual(root, "workView-" + sourceId);
                return sourceView
                       && sourceView->property("viewLabel").toString()
                              == source.value("name").toString();
            }),
            "image material keeps its name in the work rail"
        )) {
        const auto sourceView = findVisual(root, "workView-" + sourceId);
        std::cerr << "rail expected=" << source.value("name").toString().toStdString() << " actual="
                  << (sourceView ? sourceView->property("viewLabel").toString().toStdString()
                                 : "missing delegate")
                  << std::endl;
        return false;
    }
    if (!check(current(root)->property("inputCurrent").toBool(), "stage input is current"))
        return false;
    if (!check(
            !current(root)->property("acceptedImageSource").toString().isEmpty(),
            "pinned original is visible before the first return"
        ))
        return false;
    controller.exportCopy(project, aid, files.path());
    if (!check(
            wait([&] { return !controller.running(); }) && !controller.handoffPath().isEmpty()
                && QFileInfo::exists(controller.handoffPath()),
            "working copy and manifest published"
        ))
        return false;
    const auto handoff = controller.handoffPath();
    image.fill(QColor(120, 160, 70));
    if (!image.save(returnPath))
        return false;
    controller.review(project, aid, handoff, returnPath);
    if (!check(
            wait([&] { return !controller.running(); }) && !candidate(backend, aid).isEmpty(),
            "real worker returns candidate"
        ))
        return false;
    const auto reviewed = candidate(backend, aid);
    if (!check(
            wait([&] { return current(root) && current(root)->property("hasCandidate").toBool(); }),
            "candidate review stays on originating stage"
        ))
        return false;
    if (!check(capture(root, "material-image-review"), "capture image return review"))
        return false;
    if (!qEnvironmentVariable("SHAPE_LAYOUT_SMOKE_DIR").isEmpty()) {
        const auto previousMode = eval(root, "uiPreferences.appearanceMode").toInt();
        eval(root, "uiPreferences.appearanceMode = 2");
        const auto captured = capture(root, "material-image-review-dark");
        eval(root, QStringLiteral("uiPreferences.appearanceMode = %1").arg(previousMode));
        if (!check(captured, "capture themed material actions"))
            return false;
    }
    {
        QFile file(returnPath);
        if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)
            || file.write("invalid PNG") != 11)
            return false;
    }
    controller.review(project, aid, handoff, returnPath);
    if (!check(
            wait([&] { return !controller.running(); }) && !controller.errorCode().isEmpty()
                && candidate(backend, aid) == reviewed,
            "invalid replacement retains reviewed candidate"
        ))
        return false;
    if (!check(
            open(root, b) && !current(root)->property("ownsRequest").toBool()
                && current(root)->property("scopedError").toString().isEmpty(),
            "other stage has no leaked failure"
        ))
        return false;
    image.fill(QColor(140, 100, 80));
    image.save(returnPath);
    controller.review(project, aid, handoff, returnPath);
    controller.review(project, bid, handoff, returnPath);
    if (!check(controller.requestArtifactId() == aid, "busy request retains owner"))
        return false;
    if (!check(
            wait([&] { return !controller.running(); })
                && current(root)->property("outputArtifactId").toString() == bid
                && candidate(backend, bid).isEmpty(),
            "late result does not navigate or populate other stage"
        ))
        return false;
    if (!check(open(root, a), "return to original stage"))
        return false;
    controller.review(project, aid, handoff, returnPath);
    if (!check(
            controller.restart(project, aid, false) && wait([&] { return !controller.running(); })
                && candidate(backend, aid).isEmpty(),
            "cancel invalidates running and reviewed returns"
        ))
        return false;
    controller.review(project, aid, handoff, returnPath);
    if (!check(
            wait([&] { return !controller.running(); })
                && controller.errorCode() == "material_session_changed"
                && candidate(backend, aid).isEmpty(),
            "cancelled handoff rejects late retry"
        ))
        return false;
    controller.exportCopy(project, aid, files.path());
    if (!check(
            wait([&] { return !controller.running(); }) && !controller.handoffPath().isEmpty(),
            "restart exports a fresh session"
        ))
        return false;
    controller.review(project, aid, controller.handoffPath(), returnPath);
    if (!check(
            wait([&] { return !controller.running(); }) && !candidate(backend, aid).isEmpty(),
            "fresh return recovers"
        ))
        return false;
    const auto finalCandidate = candidate(backend, aid);
    if (!check(
            adoptVisible(root, backend, aid) && !backend.acceptCandidate(finalCandidate),
            "visible adopt succeeds once"
        ))
        return false;
    bool originalKept = false;
    for (const auto& item : backend.artifacts()) {
        const auto map = item.toMap();
        if (map.value("id").toString() == sourceId)
            originalKept = map.value("acceptedRevisionId").toString() == originalHead;
    }
    if (!check(originalKept && output(backend, a) == aid, "original and stage identity retained"))
        return false;
    if (!check(
            backend.openProject(QUrl::fromLocalFile(project)) && open(root, a),
            "accepted editing stage reopens"
        ))
        return false;
    return check(capture(root, "material-stage"), "capture reopened editing stage")
           && audioFlow(backend, controller, root, a, files);
}

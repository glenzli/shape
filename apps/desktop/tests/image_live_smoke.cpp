#include "image_live_smoke.hpp"
#include "desktop_backend.hpp"
#include "image_preview_provider.hpp"
#include "infer_image_controller.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickWindow>
#include <QTimer>
#include <QUrl>
#include <iostream>

bool image_live_smoke::run(
    DesktopBackend& backend,
    InferImageController& image,
    QObject& root,
    const QString& directory
) {
    QDir().mkpath(directory);
    const QString project = directory + QStringLiteral("/Luna image.shape");
    if (!backend.createProject(QUrl::fromLocalFile(directory), QStringLiteral("Luna image"))
        || !backend.createAiImageScene(
            QStringLiteral("Weekend illustration"),
            QStringLiteral(
                "A clean friendly educational illustration of a child doing tai chi "
                "in a green park on a sunny morning. Flat editorial illustration, "
                "soft blue and green colors, clear silhouette, no text or lettering."
            ),
            1024,
            1024
        ))
        return false;
    const auto artifact = backend.artifacts().last().toMap();
    const auto draft = backend.operatorDrafts().last().toMap();
    const QString id = artifact.value(QStringLiteral("id")).toString();
    const QString draftId = draft.value(QStringLiteral("id")).toString();
    QQmlExpression show(
        QQmlEngine::contextForObject(&root),
        &root,
        QStringLiteral("openDraftTarget('%1')").arg(draftId)
    );
    show.evaluate();
    QCoreApplication::processEvents();
    image.generate(project, id, draftId);
    QElapsedTimer elapsed;
    elapsed.start();
    while (image.running() && elapsed.elapsed() < 660'000) {
        QEventLoop loop;
        QTimer::singleShot(100, &loop, &QEventLoop::quit);
        loop.exec();
    }
    if (image.running() || !image.errorCode().isEmpty() || backend.candidates().isEmpty()) {
        std::cerr << "image live smoke failed: " << image.errorCode().toStdString() << std::endl;
        return false;
    }
    const QString candidateId =
        backend.candidates().first().toMap().value(QStringLiteral("id")).toString();
    if (!backend.prepareImagePreviews(id, candidateId))
        return false;
    QString key = QUrl(backend.candidateImageSource()).path();
    if (key.startsWith('/'))
        key.remove(0, 1);
    const QImage preview = backend.imagePreviewStore()->image(key);
    if (preview.isNull() || preview.width() <= 0 || preview.height() <= 0
        || !preview.save(directory + QStringLiteral("/generated.png")))
        return false;
    if (auto* window = qobject_cast<QQuickWindow*>(&root))
        window->grabWindow().save(directory + QStringLiteral("/candidate.png"));
    if (!backend.acceptCandidate(candidateId) || !backend.openProject(QUrl::fromLocalFile(project))
        || !backend.prepareImagePreviews(id))
        return false;
    std::cout << "Luna image live smoke passed: native candidate, explicit accept, reopen; "
              << "width=" << preview.width() << " height=" << preview.height()
              << " elapsed_ms=" << elapsed.elapsed() << std::endl;
    return true;
}

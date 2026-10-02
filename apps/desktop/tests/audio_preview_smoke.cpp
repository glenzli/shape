//! Real WAV intake and preview identity across failed selection and recovery.
#include "audio_preview_smoke.hpp"
#include "audio_preview_controller.hpp"
#include "desktop_backend.hpp"
#include <QByteArray>
#include <QFile>
#include <QTemporaryDir>
#include <QUrl>
#include <iostream>

namespace {
bool check(bool value, const char* message) {
    if (!value)
        std::cerr << "audio preview smoke failed: " << message << std::endl;
    return value;
}
} // namespace

bool audio_preview_smoke::verify() {
    QTemporaryDir fixture;
    DesktopBackend backend;
    if (!fixture.isValid()
        || !backend.createProject(
            QUrl::fromLocalFile(fixture.path()),
            QStringLiteral("Preview identity")
        ))
        return false;
    QByteArray wav = QByteArray::fromHex(
        "52494646a4bb000057415645666d74201000000001000100c05d000080bb0000020010006461746180bb0000"
    );
    wav.append(QByteArray(48000, '\0'));
    const QString path = fixture.filePath(QStringLiteral("silence.wav"));
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly) || file.write(wav) != wav.size())
        return false;
    file.close();
    if (!backend.importMaterial(QUrl::fromLocalFile(path)))
        return false;
    const QString artifact =
        backend.artifacts().last().toMap().value(QStringLiteral("id")).toString();
    AudioPreviewController preview(backend);
    const auto load = [&] {
        return preview.loadPreview(artifact) && preview.hasAudio() && !preview.identity().isEmpty()
               && preview.durationMillis() == 1000 && preview.sampleRateHz() == 24000
               && preview.channels() == 1;
    };
    const auto empty = [&] {
        return !preview.hasAudio() && !preview.playing() && preview.identity().isEmpty()
               && preview.durationMillis() == 0 && preview.sampleRateHz() == 0
               && preview.channels() == 0
               && preview.errorCode() == QStringLiteral("preview_unavailable");
    };
    if (!check(load(), "load actual imported WAV"))
        return false;
    if (!check(
            !preview.loadPreview(QStringLiteral("missing-artifact")) && empty(),
            "failed selection clears old audio and disables playback"
        ))
        return false;
    if (!check(load() && preview.errorCode().isEmpty(), "reselect valid audio after failure"))
        return false;
    if (!check(
            !preview.loadPreview(artifact, QStringLiteral("missing-candidate")) && empty(),
            "missing candidate cannot leave accepted audio selected"
        ))
        return false;
    if (!check(load(), "recover before checking revision selection"))
        return false;
    if (!check(
            !preview.loadPreview(artifact, QString(), QStringLiteral("missing-revision"))
                && empty(),
            "missing revision cannot leave old audio selected"
        ))
        return false;
    return check(load() && preview.errorCode().isEmpty(), "recover after repeated failures");
}

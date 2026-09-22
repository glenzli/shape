#pragma once
class QObject;
class DesktopBackend;
class InferTextController;
class InferSpeechController;
class AudioExportController;
class QString;
namespace text_authoring_smoke {
bool verify(DesktopBackend& backend, QObject& root);
bool runLive(
    DesktopBackend& backend,
    InferTextController& text,
    InferSpeechController& speech,
    AudioExportController& exporter,
    QObject& root,
    const QString& outputDirectory
);
} // namespace text_authoring_smoke

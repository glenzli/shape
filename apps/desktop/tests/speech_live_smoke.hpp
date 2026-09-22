//! Opt-in real Runtime speech, playback, acceptance, and export verification.
#pragma once
#include <QString>
class QObject;
class DesktopBackend;
class InferSpeechController;
class AudioPreviewController;
class AudioExportController;
namespace speech_live_smoke {
bool run(
    DesktopBackend&,
    InferSpeechController&,
    AudioPreviewController&,
    AudioExportController&,
    QObject&,
    const QString& output_directory
);
bool run_script(
    DesktopBackend&,
    InferSpeechController&,
    AudioPreviewController&,
    AudioExportController&,
    QObject&,
    const QString&
);
} // namespace speech_live_smoke

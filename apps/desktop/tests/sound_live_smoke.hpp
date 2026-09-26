//! Opt-in real sound/music, UI snapshot, playback and immutable acceptance smoke.
#pragma once
#include <QString>
class QObject;
class DesktopBackend;
class InferSoundController;
class AudioPreviewController;
class AudioExportController;
namespace sound_live_smoke {
bool run(
    DesktopBackend&,
    InferSoundController&,
    AudioPreviewController&,
    AudioExportController&,
    QObject&,
    const QString&
);
}

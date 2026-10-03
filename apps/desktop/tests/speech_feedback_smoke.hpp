#pragma once
class DesktopBackend;
class InferSpeechController;
class QObject;
namespace speech_feedback_smoke {
bool verify(DesktopBackend& backend, InferSpeechController& speech, QObject& root);
}

#pragma once
#include <QString>
class DesktopBackend;
class DeckHandoffController;
class DeckMaterialTaskController;
class QObject;
class InferImageController;
namespace deck_handoff_smoke {
bool verify(
    DesktopBackend&,
    DeckHandoffController&,
    DeckMaterialTaskController&,
    QObject&,
    const QString& directory,
    const QString& settingsPath
);
bool runLiveTask(
    DesktopBackend&,
    InferImageController&,
    DeckMaterialTaskController&,
    QObject&,
    const QString& model,
    const QString& effort
);
} // namespace deck_handoff_smoke

#pragma once
#include <QString>
class DesktopBackend;
class LegacyWorkController;
class DeckHandoffController;
class DeckMaterialTaskController;
class UiPreferences;
class QObject;
namespace convergence_smoke {
bool verify(
    DesktopBackend&,
    LegacyWorkController&,
    DeckHandoffController&,
    DeckMaterialTaskController&,
    UiPreferences&,
    QObject&,
    const QString&,
    const QString& settingsPath
);
}

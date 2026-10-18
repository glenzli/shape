#pragma once
#include <QString>
class DesktopBackend;
class CardKitController;
class UiPreferences;
class QObject;
namespace workflow_smoke {
bool verify(
    DesktopBackend&,
    CardKitController&,
    UiPreferences&,
    QObject&,
    const QString&,
    const QString&
);
bool capturePreview(CardKitController&, UiPreferences&, QObject&, const QString&);
} // namespace workflow_smoke

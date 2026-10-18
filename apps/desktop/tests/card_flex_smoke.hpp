#pragma once
#include <QString>
class DesktopBackend;
class CardKitController;
class UiPreferences;
class QObject;
namespace card_flex_smoke {
bool verify(DesktopBackend&, CardKitController&, UiPreferences&, QObject&, const QString&);
}

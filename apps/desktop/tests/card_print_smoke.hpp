#pragma once
#include <QString>
class DesktopBackend;
class CardKitController;
class CardPrintController;
class UiPreferences;
class QObject;
namespace card_print_smoke {
bool verify(
    DesktopBackend&,
    CardKitController&,
    CardPrintController&,
    UiPreferences&,
    QObject&,
    const QString&
);
}

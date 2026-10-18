#pragma once
#include <QString>
class DesktopBackend;
class PictureBookController;
namespace picture_book_smoke {
// Explicit two-process opt-in probe; normal/offline book smoke never starts this branch.
bool processRestart(
    DesktopBackend&,
    PictureBookController&,
    const QString& directory,
    const QString& phase
);
void verifyBundleScopedShadowLocators(
    DesktopBackend&,
    const QString& originalBundle,
    const QString& copiedBundle,
    const QString& output,
    const QString& ticket,
    const QString& directory
);
} // namespace picture_book_smoke

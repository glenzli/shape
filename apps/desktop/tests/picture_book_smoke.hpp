#pragma once
#include <QString>
class DesktopBackend;
class PictureBookController;
class QObject;
namespace picture_book_smoke {
bool verify(DesktopBackend&, PictureBookController&, QObject&, const QString& directory);
}

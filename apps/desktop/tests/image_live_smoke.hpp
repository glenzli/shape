#pragma once
#include <QString>
class DesktopBackend;
class InferImageController;
class QObject;
namespace image_live_smoke {
bool run(DesktopBackend&, InferImageController&, QObject&, const QString&);
}

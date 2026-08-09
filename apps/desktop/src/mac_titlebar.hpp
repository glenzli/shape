//! macOS title-bar integration. The implementation keeps native window
//! controls while vertically aligning them with Shape's QML toolbar.

#pragma once

class QWindow;

void installMacTitleBarAlignment(QWindow* window, int title_bar_height);

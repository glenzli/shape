#pragma once
#include <QImage>
#include <QJsonObject>
#include <QPainter>
#include <QString>

namespace picture_book {
// One reading layout shared by the on-screen preview and vector PDF output.
struct Page {
    QSizeF canvas;
    QImage artwork;
    QString text;
    QString author;
    QString role;
    int font_size = 26;
    int index = 0;
};
struct LayoutResult {
    bool fits = true;
    bool missing_glyphs = false;
    QString font_family;
};
LayoutResult paint(QPainter& painter, const Page& page);
QImage preview(const Page& page, LayoutResult* result = nullptr);
} // namespace picture_book

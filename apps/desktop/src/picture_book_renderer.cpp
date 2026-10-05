#include "picture_book_renderer.hpp"
#include <QFontDatabase>
#include <QGlyphRun>
#include <QTextLayout>
#include <QTextOption>
#include <algorithm>
#include <memory>
#include <vector>

namespace picture_book {
namespace {
QString reading_font() {
    const auto families = QFontDatabase::families();
    for (const auto& name :
         {QStringLiteral("PingFang SC"),
          QStringLiteral("Microsoft YaHei"),
          QStringLiteral("Noto Sans CJK SC")}) {
        if (families.contains(name))
            return name;
    }
    return QFontDatabase::systemFont(QFontDatabase::GeneralFont).family();
}
struct Paragraphs {
    std::vector<std::unique_ptr<QTextLayout>> layouts;
    qreal height = 0;
    bool missing = false;
};
Paragraphs layout(const QString& text, const QFont& font, qreal width, Qt::Alignment alignment) {
    Paragraphs result;
    for (const auto& paragraph : text.split('\n')) {
        auto line_layout = std::make_unique<QTextLayout>(paragraph, font);
        QTextOption option;
        option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
        option.setAlignment(alignment);
        line_layout->setTextOption(option);
        line_layout->beginLayout();
        while (true) {
            auto line = line_layout->createLine();
            if (!line.isValid())
                break;
            line.setLineWidth(width);
            line.setPosition(QPointF(0, result.height));
            result.height += line.height() * 1.25;
        }
        line_layout->endLayout();
        for (const auto& run : line_layout->glyphRuns()) {
            if (run.glyphIndexes().contains(0))
                result.missing = true;
        }
        if (paragraph.isEmpty())
            result.height += font.pixelSize() * .8;
        result.layouts.push_back(std::move(line_layout));
    }
    return result;
}
} // namespace
LayoutResult paint(QPainter& painter, const Page& page) {
    const qreal width = page.canvas.width(), height = page.canvas.height();
    const qreal margin = width * .065;
    const QString family = reading_font();
    const bool heading = page.role != QStringLiteral("body");
    QFont font(family);
    font.setPixelSize(qRound(
        page.font_size
        * (page.role == QStringLiteral("cover")   ? 1.5
           : page.role == QStringLiteral("title") ? 1.25
                                                  : 1.0)
    ));
    font.setWeight(heading ? QFont::DemiBold : QFont::Normal);
    auto main =
        layout(page.text, font, width - margin * 2, heading ? Qt::AlignHCenter : Qt::AlignLeft);
    QFont credit_font(family);
    credit_font.setPixelSize(std::max(12, page.font_size * 2 / 3));
    auto credit = layout(page.author, credit_font, width - margin * 2, Qt::AlignHCenter);
    const qreal credit_height = page.author.isEmpty() ? 0 : credit.height + height * .015;
    const qreal text_height = page.text.isEmpty() ? 0 : main.height;
    const qreal panel_height =
        (text_height + credit_height == 0) ? 0 : text_height + credit_height + height * .06;
    const bool fits = panel_height <= height * .60 && font.pixelSize() <= width / 3;
    painter.save();
    painter.setRenderHints(
        QPainter::Antialiasing | QPainter::TextAntialiasing | QPainter::SmoothPixmapTransform
    );
    painter.fillRect(QRectF(0, 0, width, height), QColor(QStringLiteral("#faf7f0")));
    const QRectF art_box(
        margin,
        height * .045,
        width - margin * 2,
        std::max<qreal>(0, height * .90 - panel_height)
    );
    if (!page.artwork.isNull()) {
        QSizeF size = page.artwork.size();
        size.scale(art_box.size(), Qt::KeepAspectRatio);
        painter.drawImage(
            QRectF(art_box.center() - QPointF(size.width() / 2, size.height() / 2), size),
            page.artwork
        );
    }
    const qreal text_y = page.artwork.isNull() && heading
                             ? std::max(height * .16, (height - panel_height) / 2)
                             : height - panel_height - height * .025;
    painter.setPen(QColor(QStringLiteral("#223d49")));
    for (const auto& item : main.layouts)
        item->draw(&painter, QPointF(margin, text_y));
    painter.setPen(QColor(QStringLiteral("#637679")));
    for (const auto& item : credit.layouts)
        item->draw(&painter, QPointF(margin, text_y + text_height + height * .015));
    painter.restore();
    return {fits, main.missing || credit.missing, family};
}
QImage preview(const Page& page, LayoutResult* result) {
    QSize pixels = page.canvas.toSize();
    pixels.scale(QSize(1600, 1200), Qt::KeepAspectRatio);
    QImage image(pixels, QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::white);
    QPainter painter(&image);
    painter.scale(pixels.width() / page.canvas.width(), pixels.height() / page.canvas.height());
    const auto value = paint(painter, page);
    if (result)
        *result = value;
    return image;
}
} // namespace picture_book

#include "picture_book_smoke.hpp"
#include "desktop_backend.hpp"
#include "picture_book_controller.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QEventLoop>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPainter>
#include <QPainterPath>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QTimer>
#include <iostream>
#include <stdexcept>

namespace picture_book_smoke {
namespace {
void require(bool value, const char* why) {
    if (!value)
        throw std::runtime_error(why);
}
QJsonObject parsed(const QString& text) {
    return QJsonDocument::fromJson(text.toUtf8()).object();
}
QJsonObject pin(const QVariantMap& a) {
    return {
        {"artifact_id", a.value("id").toString()},
        {"revision_id", a.value("acceptedRevisionId").toString()}
    };
}
void evaluate(QObject& object, const QString& script) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, script);
    expression.evaluate();
    if (expression.hasError())
        throw std::runtime_error(expression.error().toString().toStdString());
    QCoreApplication::processEvents();
}
} // namespace
bool verify(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& root,
    const QString& directory
) {
    try {
        require(QDir().mkpath(directory), "fixture directory");
        require(
            backend.createProject(QUrl::fromLocalFile(directory), QStringLiteral("Lantern sample")),
            "create project"
        );
        QImage artwork(1024, 600, QImage::Format_RGB32);
        artwork.fill(QColor("#143147"));
        {
            QPainter p(&artwork);
            p.setRenderHint(QPainter::Antialiasing);
            QLinearGradient sky(0, 0, 0, 600);
            sky.setColorAt(0, QColor("#11283f"));
            sky.setColorAt(1, QColor("#457888"));
            p.fillRect(artwork.rect(), sky);
            p.setPen(Qt::NoPen);
            p.setBrush(QColor("#fae8ad"));
            p.drawEllipse(QPointF(772, 132), 60, 60);
            for (int i = 0; i < 24; ++i) {
                p.setBrush(QColor("#e8edd5"));
                p.drawEllipse(QPointF(55 + (i * 137) % 900, 35 + (i * 61) % 260), 2, 2);
            }
            QPainterPath hill;
            hill.moveTo(0, 435);
            hill.cubicTo(220, 280, 400, 400, 550, 360);
            hill.cubicTo(730, 310, 820, 375, 1024, 290);
            hill.lineTo(1024, 600);
            hill.lineTo(0, 600);
            p.setBrush(QColor("#244e54"));
            p.drawPath(hill);
            QPainterPath path;
            path.moveTo(260, 600);
            path.cubicTo(530, 495, 445, 475, 590, 377);
            path.cubicTo(540, 485, 635, 545, 540, 600);
            p.setBrush(QColor("#b2c6bd"));
            p.drawPath(path);
            p.setBrush(QColor("#0c2933"));
            p.drawRoundedRect(QRectF(466, 405, 34, 79), 15, 15);
            p.drawEllipse(QPointF(483, 390), 20, 20);
            p.setPen(QPen(QColor("#193641"), 7));
            p.drawLine(494, 424, 530, 416);
            p.setPen(Qt::NoPen);
            p.setBrush(QColor("#ffd787"));
            p.drawRoundedRect(QRectF(523, 408, 23, 32), 5, 5);
        }
        const auto art = directory + QStringLiteral("/moon.png");
        require(artwork.save(art), "write artwork");
        require(backend.importRaster(QUrl::fromLocalFile(art)), "import artwork");
        QJsonObject image;
        for (const auto& value : backend.artifacts())
            if (value.toMap().value("kindKey") == "image_raster")
                image = pin(value.toMap());
        const QString en = QStringLiteral(
            "[Cover]\nThe Lantern Path\n[Author]\nShape · Synthetic sample\n[Title]\nA small "
            "light, a long way home.\n[1]\nUnder the quiet moon, a little lantern follows the "
            "winding path.\n\nOne step, then another — the way home grows "
            "brighter.\n[2]\nPrint-only note\nThis page is retained in the editable book and "
            "omitted from its reading PDF."
        );
        const QString zh = QStringLiteral(
            "[封面]\n提灯的小路\n[作者]\nShape · "
            "合成示例\n[扉页]\n一点小小的光，照亮回家的路。\n[1]"
            "\n安静的月光下，一盏小灯沿着弯弯的小路前行。\n\n一步，再一步。回家的路，渐渐明亮起来。"
            "\n[2]\n仅供印刷的说明页\n这一页保留在可编辑的绘本中，默认不包含在阅读版 PDF 里。"
        );
        const auto en_pin = parsed(backend.bookScript(QStringLiteral("English story"), {}, en));
        const auto zh_pin = parsed(backend.bookScript(QStringLiteral("中文故事"), {}, zh));
        QJsonObject request{
            {"name", QStringLiteral("The Lantern Path · 提灯的小路")},
            {"default_language", "zh-CN"},
            {"languages", QJsonObject{{"en", en_pin}, {"zh-CN", zh_pin}}},
            {"pages",
             QJsonArray{
                 QJsonObject{{"kind", "image"}, {"source", image}},
                 QJsonObject{{"kind", "blank"}},
                 QJsonObject{{"kind", "image"}, {"source", image}},
                 QJsonObject{{"kind", "blank"}, {"print_only", true}}
             }},
            {"canvas_width", 1024},
            {"canvas_height", 768},
            {"font_size", 30}
        };
        auto* workspace = root.findChild<QObject*>(QStringLiteral("pictureBookWorkspace"));
        require(workspace, "book workspace packaged");
        evaluate(*workspace, QStringLiteral("openWorkspace()"));
        workspace->setProperty("draft", request.toVariantMap());
        workspace->setProperty("activeLanguage", QStringLiteral("zh-CN"));
        evaluate(*workspace, QStringLiteral("loadScript()"));
        workspace->setProperty("pageIndex", 3);
        evaluate(*workspace, QStringLiteral("movePage(-1)"));
        auto moved =
            QJsonObject::fromVariantMap(workspace->property("draft").toMap())["pages"].toArray();
        require(
            moved[2].toObject()["print_only"].toBool()
                && !moved[3].toObject()["print_only"].toBool(),
            "QML page move retains print-only marker"
        );
        evaluate(*workspace, QStringLiteral("movePage(1)"));
        workspace->setProperty("pageIndex", 0);
        const auto count = backend.artifactCount();
        require(book.review(request.toVariantMap()), "review new book");
        require(book.hasReview() && backend.artifactCount() == count, "review is transient");
        book.invalidate();
        require(!book.hasReview() && backend.artifactCount() == count, "cancel candidate");
        require(book.review(request.toVariantMap()) && book.accept(), "adopt new book");
        const auto id = book.source().value("artifact_id").toString();
        require(!id.isEmpty(), "saved book identity");
        const auto original_source = book.source();
        for (const int size : {32, 30}) {
            request["font_size"] = size;
            require(book.review(request.toVariantMap()) && book.accept(), "repeat edit and adopt");
        }
        require(book.source() != original_source, "book version changed");
        const auto latest = book.source();
        require(
            backend.openProject(QUrl::fromLocalFile(backend.bundlePath())) && book.load(id),
            "reopen saved book"
        );
        require(book.source() == latest, "exact saved head reopened");
        QJsonArray page_evidence;
        for (const auto& language : {QStringLiteral("en"), QStringLiteral("zh-CN")}) {
            for (int index = 0; index < 4; ++index) {
                picture_book::LayoutResult layout;
                auto image = book.pageImage(language, index, &layout);
                require(layout.fits && !layout.missing_glyphs, "page typography fits");
                const auto name = QStringLiteral("%1-page-%2.png").arg(language).arg(index + 1);
                require(image.save(directory + "/" + name), "save rendered page");
                page_evidence.append(
                    QJsonObject{
                        {"language", language},
                        {"page", index + 1},
                        {"font", layout.font_family},
                        {"fits", layout.fits},
                        {"missing_glyphs", layout.missing_glyphs}
                    }
                );
            }
            const auto pdf = QUrl::fromLocalFile(directory + "/" + language + "-reading.pdf");
            require(book.exportPdf(pdf, language, false), "reading PDF");
            require(!book.exportPdf(pdf, language, false), "PDF no clobber");
        }
        require(
            book.exportPdf(QUrl::fromLocalFile(directory + "/zh-CN-proof.pdf"), "zh-CN", true),
            "print-only proof"
        );
        const auto native = QUrl::fromLocalFile(directory + "/lantern.scproj");
        require(book.exportNative(native), "editable native export");
        require(!book.exportNative(native), "native no clobber");
        require(book.showPage("zh-CN", 2), "Chinese preview");
        workspace->setProperty("pageIndex", 2);
        evaluate(*workspace, QStringLiteral("loadScript()"));
        QCoreApplication::processEvents();
        auto* window = qobject_cast<QQuickWindow*>(&root);
        require(window, "window exists");
        window->requestUpdate();
        QEventLoop loop;
        QTimer::singleShot(450, &loop, &QEventLoop::quit);
        loop.exec();
        require(
            window->grabWindow().save(directory + "/shape-book-workspace.png"),
            "actual workspace capture"
        );
        QJsonObject report{
            {"schema", "shape.picture-book-ui-evidence/1"},
            {"source", QJsonObject::fromVariantMap(book.source())},
            {"project_path", backend.bundlePath()},
            {"pages", page_evidence},
            {"passed", true},
            {"renderer", QString::fromLocal8Bit(qgetenv("QT_QUICK_BACKEND"))},
            {"platform", QGuiApplication::platformName()},
            {"graphics_api", static_cast<int>(window->rendererInterface()->graphicsApi())},
            {"checks",
             QJsonArray{
                 "new book",
                 "QML page reorder retains print-only marker",
                 "cancel proposal",
                 "explicit adoption",
                 "repeated revisions",
                 "exact reopen",
                 "bilingual fit and glyphs",
                 "3-page reading PDFs",
                 "4-page proof",
                 "no-clobber PDF/scproj",
                 "packaged workspace screenshot"
             }}
        };
        QFile file(directory + "/report.json");
        require(file.open(QIODevice::WriteOnly), "report");
        file.write(QJsonDocument(report).toJson());
        std::cout << "picture-book desktop workflow passed" << std::endl;
        return true;
    } catch (const std::exception& error) {
        std::cerr << "picture-book smoke failed: " << error.what() << std::endl;
        return false;
    }
}
} // namespace picture_book_smoke

#include "card_print_smoke.hpp"
#include "card_kit_controller.hpp"
#include "card_print_controller.hpp"
#include "card_print_export.hpp"
#include "desktop_backend.hpp"
#include "ui_preferences.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPainter>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTimer>
#include <cmath>
#include <iostream>
#include <stdexcept>

namespace card_print_smoke {
namespace {
void require(bool value, const char* why) {
    if (!value)
        throw std::runtime_error(why);
}
void settle(int ms = 100) {
    QEventLoop loop;
    QTimer::singleShot(ms, &loop, &QEventLoop::quit);
    loop.exec();
}
QQuickItem* visual(QQuickItem* item, const QString& name) {
    if (!item)
        return nullptr;
    if (item->objectName() == name)
        return item;
    for (auto* child : item->childItems())
        if (auto* found = visual(child, name))
            return found;
    return nullptr;
}
QObject& child(QObject& root, const char* name) {
    if (auto* found = root.findChild<QObject*>(QString::fromLatin1(name)))
        return *found;
    auto* content = qvariant_cast<QQuickItem*>(root.property("contentItem"));
    if (auto* window = qobject_cast<QQuickWindow*>(&root))
        content = window->contentItem();
    if (auto* found = visual(content, QString::fromLatin1(name)))
        return *found;
    throw std::runtime_error(name);
}
void evaluate(QObject& root, const QString& script) {
    QQmlExpression expression(QQmlEngine::contextForObject(&root), &root, script);
    expression.evaluate();
    if (expression.hasError())
        throw std::runtime_error(expression.error().toString().toStdString());
    QCoreApplication::processEvents();
}
QString literal(const QString& text) {
    const auto encoded = QJsonDocument(QJsonArray{text}).toJson(QJsonDocument::Compact);
    return QString::fromUtf8(encoded.mid(1, encoded.size() - 2));
}
QJsonObject action(DesktopBackend& backend, const QJsonObject& request) {
    return QJsonDocument::fromJson(
               backend
                   .cardKitAction(
                       QString::fromUtf8(QJsonDocument(request).toJson(QJsonDocument::Compact))
                   )
                   .toUtf8()
    )
        .object();
}
QByteArray bytes(const QString& path) {
    QFile file(path);
    require(file.open(QIODevice::ReadOnly), "open evidence file");
    return file.readAll();
}
void exportReady(CardPrintController& printer, const QString& path) {
    bool success = false, finished = false;
    const auto connection =
        QObject::connect(&printer, &CardPrintController::finished, [&](bool ok) {
            success = ok;
            finished = true;
        });
    require(printer.exportPdf(QUrl::fromLocalFile(path), printer.version()), "admit PDF output");
    QElapsedTimer deadline;
    deadline.start();
    while (printer.running() && deadline.elapsed() < 20000)
        settle(25);
    QObject::disconnect(connection);
    require(
        finished && success && !printer.running() && bytes(path).startsWith("%PDF-"),
        "complete physical PDF publication"
    );
}
void capture(QQuickWindow& window, const QString& path) {
    window.requestUpdate();
    settle(180);
    require(window.grabWindow().save(path), "capture actual print UI");
}
QVariantMap
sourceFixture(DesktopBackend& backend, CardKitController& kit, const QString& directory) {
    if (backend.projectOpen()) {
        for (const auto& value : backend.artifacts()) {
            const auto item = value.toMap();
            if (item.value("kindKey") == "card_kit" && item.value("hasAcceptedRevision").toBool()) {
                require(kit.load(item.value("id").toString()), "load exact accepted sample");
                return kit.source();
            }
        }
        throw std::runtime_error("sample project has no accepted card kit");
    }
    require(
        backend.createProject(QUrl::fromLocalFile(directory), "Print layout cards"),
        "create isolated print project"
    );
    QImage art(600, 600, QImage::Format_RGB32);
    art.fill(QColor("#274d5f"));
    {
        QPainter painter(&art);
        painter.setBrush(QColor("#f4d887"));
        painter.setPen(Qt::NoPen);
        painter.drawEllipse(360, 70, 150, 150);
        painter.setBrush(QColor("#71928a"));
        painter.drawEllipse(-100, 380, 800, 350);
    }
    const auto image_path = directory + "/moon.png";
    require(
        art.save(image_path) && backend.importRaster(QUrl::fromLocalFile(image_path)),
        "import accepted print artwork"
    );
    QVariantMap pin;
    for (const auto& value : backend.artifacts()) {
        const auto item = value.toMap();
        if (item.value("kindKey") == "image_raster")
            pin = {
                {"artifact_id", item.value("id")},
                {"revision_id", item.value("acceptedRevisionId")}
            };
    }
    require(!pin.isEmpty() && kit.beginNew(), "begin recoverable print kit");
    auto composition = kit.draft().value("composition").toMap();
    QVariantList cards;
    const auto prototype = composition.value("cards").toList().first().toMap();
    for (int index = 0; index < 3; ++index) {
        auto card = prototype;
        card["id"] = QStringLiteral("print-card-%1").arg(index + 1);
        card["title"] =
            QStringList{"Moon garden", "Lantern friend", QString::fromUtf8("月光守卫")}[index];
        card["rules"] = index == 2 ? QString::fromUtf8("守护一名同伴，然后抽一张卡。")
                                   : QStringLiteral("Protect a friend. Draw one card.");
        card["quantity"] = QList<int>{4, 5, 3}[index];
        card["artwork"] = pin;
        cards.push_back(card);
    }
    composition["name"] = QString::fromUtf8("Moon garden · 月光卡牌");
    composition["cards"] = cards;
    composition.remove("layout"); // Retain the established 63 × 88 mm legacy regression fixture.
    require(
        kit.save(composition) && kit.review("print-card-1") && kit.accept(),
        "explicitly accept complete 12-copy kit"
    );
    return kit.source();
}
} // namespace
bool verify(
    DesktopBackend& backend,
    CardKitController& kit,
    CardPrintController& printer,
    UiPreferences& preferences,
    QObject& root,
    const QString& directory
) {
    try {
        require(QDir().mkpath(directory), "print evidence directory");
        const auto source = sourceFixture(backend, kit, directory);
        auto* window = qobject_cast<QQuickWindow*>(&root);
        require(window, "actual desktop print window");
        auto& workspace = child(root, "cardKitWorkspace");
        evaluate(
            workspace,
            "openWorkspace(" + literal(source.value("artifact_id").toString()) + ")"
        );
        preferences.setLanguageMode("en");
        auto& button = child(workspace, "cardKitPrint");
        require(
            button.property("enabled").toBool() && QMetaObject::invokeMethod(&button, "clicked"),
            "accepted kit print entry is reachable"
        );
        settle();
        auto& dialog = child(root, "cardPrintDialog");
        require(
            dialog.property("visible").toBool() && printer.ready(),
            "print entry prepares real accepted version"
        );
        const auto plan = action(
                              backend,
                              {{"action", "print_plan"},
                               {"source", QJsonObject::fromVariantMap(source)},
                               {"options", QJsonObject{}}}
        )["print_plan"]
                              .toObject();
        const auto document = card_print::prepare(plan);
        require(
            document->cards.size() == 3 && document->summary["slots_per_page"].toInt() == 9
                && std::abs(document->pages.first().placements.first().rect_mm.width() - 63) < 0.001
                && std::abs(document->pages.first().placements.first().rect_mm.height() - 88)
                       < 0.001,
            "native consumer uses exact 63 by 88 mm geometry and one SVG per card"
        );
        if (document->summary["total_copies"].toInt() == 12)
            require(
                document->pages.size() == 2 && document->pages[0].placements.size() == 9
                    && document->pages[1].placements.size() == 3,
                "12 copies produce a full and partial last sheet"
            );
        const auto a4 = directory + "/cards-a4.pdf";
        exportReady(printer, a4);
        require(
            card_print::preview(*document, 0).save(directory + "/sheet-a4-1.png"),
            "same-painter paper preview"
        );
        if (document->pages.size() > 1)
            require(
                card_print::preview(*document, document->pages.size() - 1)
                    .save(directory + "/sheet-a4-last.png"),
                "partial final sheet preview"
            );
        const auto a4_bytes = bytes(a4);
        bool refused = false;
        try {
            card_print::exportPdf(*document, a4, backend.bundlePath());
        } catch (const std::exception&) {
            refused = true;
        }
        require(refused && bytes(a4) == a4_bytes, "existing PDF is never overwritten");
        auto broken_document = *document;
        broken_document.cards[broken_document.pages.first().placements.first().card_id] =
            "not an SVG";
        const auto broken_pdf = directory + "/broken-render.pdf";
        refused = false;
        try {
            card_print::exportPdf(broken_document, broken_pdf, backend.bundlePath());
        } catch (const std::exception&) {
            refused = true;
        }
        require(
            refused && !QFileInfo::exists(broken_pdf)
                && QDir(directory)
                       .entryList({".shape-cards-*.pdf"}, QDir::Files | QDir::Hidden)
                       .isEmpty(),
            "late SVG rendering failure leaves no final PDF or temporary file"
        );
        const auto inside = backend.bundlePath() + "/forbidden.pdf";
        refused = false;
        try {
            card_print::exportPdf(*document, inside, backend.bundlePath());
        } catch (const std::exception&) {
            refused = true;
        }
        require(refused && !QFileInfo::exists(inside), "project destination produces no output");
        const auto symlink_path = directory + "/project-alias";
        require(
            QFile::link(backend.bundlePath(), symlink_path),
            "project directory symlink fixture"
        );
        refused = false;
        try {
            card_print::exportPdf(*document, symlink_path + "/forbidden.pdf", backend.bundlePath());
        } catch (const std::exception&) {
            refused = true;
        }
        require(
            refused && !QFileInfo::exists(inside),
            "symlink destination cannot bypass project guard"
        );
        const auto old_version = printer.version();
        require(printer.prepare(source, {{"paper", "letter"}}), "US Letter layout accepted");
        require(
            !printer.exportPdf(QUrl::fromLocalFile(directory + "/stale.pdf"), old_version)
                && !QFileInfo::exists(directory + "/stale.pdf"),
            "old file-dialog callback cannot export a changed plan"
        );
        require(printer.prepare(source, {{"paper", "letter"}}), "restore valid Letter page");
        exportReady(printer, directory + "/cards-letter.pdf");
        require(
            printer.plan().value("sheet_width_mm").toDouble() == 215.9
                && printer.plan().value("sheet_height_mm").toDouble() == 279.4,
            "Letter PDF retains the full physical paper size"
        );
        require(
            printer.prepare(
                source,
                {{"card_width_mm", 90}, {"card_height_mm", 120}, {"margin_mm", 6}, {"gap_mm", 4}}
            ),
            "custom physical card size and spacing accepted"
        );
        exportReady(printer, directory + "/cards-custom.pdf");
        require(
            !printer.prepare(source, {{"gap_mm", 1}}) && printer.error().contains("more room"),
            "crop clearance failure gives the correct spacing remedy"
        );
        const auto invalid = directory + "/invalid.pdf";
        require(
            !printer.prepare(source, {{"card_width_mm", 300}, {"card_height_mm", 300}})
                && !printer.ready() && printer.previewSource().isEmpty()
                && !printer.exportPdf(QUrl::fromLocalFile(invalid), printer.version())
                && !QFileInfo::exists(invalid),
            "layout error clears the old preview and produces no PDF"
        );
        require(printer.prepare(source, {}), "restore default accepted print layout");
        bool source_unchanged = false;
        for (const auto& value : backend.artifacts()) {
            const auto item = value.toMap();
            if (item.value("id") == source.value("artifact_id")
                && item.value("acceptedRevisionId") == source.value("revision_id"))
                source_unchanged = true;
        }
        require(source_unchanged, "printing preserves the exact accepted kit head");
        const auto original_size = window->size();
        window->resize(1120, 720);
        settle();
        capture(*window, directory + "/print-en-narrow.png");
        preferences.setLanguageMode("zh_CN");
        settle();
        capture(*window, directory + "/print-zh-narrow.png");
        require(
            printer.showPage(printer.plan().value("page_count").toInt() - 1),
            "navigate to final sheet"
        );
        capture(*window, directory + "/print-last-zh-narrow.png");
        window->resize(original_size);
        QMetaObject::invokeMethod(&dialog, "close");
        const auto other = directory + "/other";
        require(
            QDir().mkpath(other)
                && backend.createProject(QUrl::fromLocalFile(other), "Other print project"),
            "switch project during print session"
        );
        require(
            !printer.ready()
                && !printer.exportPdf(
                    QUrl::fromLocalFile(directory + "/foreign.pdf"),
                    printer.version()
                )
                && !QFileInfo::exists(directory + "/foreign.pdf"),
            "closed project cannot export stale paper content"
        );
        QFile report(directory + "/print-evidence.json");
        require(report.open(QIODevice::WriteOnly), "print report");
        report.write(QJsonDocument(
                         QJsonObject{
                             {"source", QJsonObject::fromVariantMap(source)},
                             {"default_plan", document->summary},
                             {"unique_svg_count", document->cards.size()},
                             {"a4_pdf", a4},
                             {"same_geometry_preview_pdf", true},
                             {"no_clobber", true},
                             {"project_guard", true},
                             {"stale_plan_guard", true}
                         }
        ).toJson());
        std::cout << "card print accepted source, physical geometry, quantities, partial sheets, "
                     "A4/Letter/custom, same-painter preview/PDF, no-clobber, error and project "
                     "guards passed"
                  << std::endl;
        return true;
    } catch (const std::exception& e) {
        std::cerr << "card print smoke: " << e.what() << std::endl;
        return false;
    }
}
} // namespace card_print_smoke

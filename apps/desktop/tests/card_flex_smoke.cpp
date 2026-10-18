#include "card_flex_smoke.hpp"
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
#include <QFontInfo>
#include <QFontMetricsF>
#include <QImage>
#include <QJSValue>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPainter>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTimer>
#include <QXmlStreamReader>
#include <algorithm>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <stdexcept>

namespace card_flex_smoke {
namespace {
void require(bool value, const char* why) {
    if (!value)
        throw std::runtime_error(why);
}
void settle(int ms = 80) {
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
void evaluate(QObject& object, const QString& code) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, code);
    expression.evaluate();
    if (expression.hasError())
        throw std::runtime_error(expression.error().toString().toStdString());
    QCoreApplication::processEvents();
}
QString literal(const QString& value) {
    const auto bytes = QJsonDocument(QJsonArray{value}).toJson(QJsonDocument::Compact);
    return QString::fromUtf8(bytes.mid(1, bytes.size() - 2));
}
void click(QObject& root, const char* name) {
    auto& item = child(root, name);
    require(item.property("enabled").toBool() && QMetaObject::invokeMethod(&item, "clicked"), name);
    settle();
}
void json(const QString& path, const QJsonObject& object) {
    QFile file(path);
    require(file.open(QIODevice::WriteOnly), "write flexible-card evidence");
    file.write(QJsonDocument(object).toJson());
}
QJsonObject action(DesktopBackend& backend, const QJsonObject& request) {
    return QJsonDocument::fromJson(
               backend.cardKitAction(QString::fromUtf8(QJsonDocument(request).toJson())).toUtf8()
    )
        .object();
}
void image(const QString& path, int width, int height, const QString& label) {
    QImage pixels(width, height, QImage::Format_RGB32);
    pixels.fill(QColor("#b060d0"));
    QPainter painter(&pixels);
    const int edge = std::max(8, std::min(width, height) / 10);
    painter.fillRect(0, 0, width, edge, QColor("#ff0000"));
    painter.fillRect(width - edge, 0, edge, height, QColor("#00aa00"));
    painter.fillRect(0, height - edge, width, edge, QColor("#0000ff"));
    painter.fillRect(0, 0, edge, height, QColor("#ffff00"));
    auto font = painter.font();
    font.setPixelSize(std::max(12, std::min(width, height) / 8));
    painter.setFont(font);
    painter.setPen(Qt::black);
    painter.drawText(QRect(0, 0, width, height), Qt::AlignCenter, label);
    painter.end();
    require(pixels.save(path), "four-edge PNG fixture");
}
QVariant plainProperty(QObject& object, const char* name) {
    auto value = object.property(name);
    if (value.metaType() == QMetaType::fromType<QJSValue>())
        value = value.value<QJSValue>().toVariant();
    return value;
}
QVariantMap documentState(QObject& workspace) {
    const auto result = plainProperty(workspace, "composition").toMap();
    require(
        result.contains("cards") && result.value("cards").canConvert<QVariantList>(),
        "composition snapshot contains the actual card list"
    );
    return result;
}
QVariantList cards(QObject& workspace) {
    return documentState(workspace).value("cards").toList();
}
void waitIntake(QObject& intake) {
    QElapsedTimer deadline;
    deadline.start();
    while (intake.property("running").toBool() && deadline.elapsed() < 15000)
        settle(30);
    require(!intake.property("running").toBool(), "bounded per-image intake completes");
}
void selectFiles(QObject& workspace, QObject& intake, const QStringList& paths) {
    evaluate(workspace, "beginCardImages()");
    auto& chooser = child(intake, "cardImagesFileDialog");
    require(chooser.property("fileMode").toInt() == 1, "native chooser allows multiple files");
    QJsonArray urls;
    for (const auto& path : paths)
        urls.push_back(QUrl::fromLocalFile(path).toString());
    QMetaObject::invokeMethod(&chooser, "close");
    // The native selectedFiles property is read-only. Exercise the same production
    // callback body after the real chooser has captured its target identity.
    evaluate(
        intake,
        "prepareFiles(" + QString::fromUtf8(QJsonDocument(urls).toJson(QJsonDocument::Compact))
            + ")"
    );
    settle();
}
void capture(QQuickWindow& window, const QString& path) {
    window.requestUpdate();
    settle(160);
    require(window.grabWindow().save(path), "actual flexible-card screenshot");
}
std::shared_ptr<const card_print::Document> exportLayout(
    DesktopBackend& backend,
    CardKitController& kit,
    CardPrintController& printer,
    QObject& root,
    QObject& workspace,
    const QString& prefix,
    const QJsonObject& options = {}
) {
    evaluate(workspace, "flush()");
    click(workspace, "cardKitPrint");
    auto& dialog = child(root, "cardPrintDialog");
    require(
        dialog.property("visible").toBool() && kit.canExport() && printer.ready(),
        "explicit accept-and-print action opens the real print consumer"
    );
    const auto source = QJsonObject::fromVariantMap(kit.source());
    if (!options.isEmpty())
        require(
            printer.prepare(source.toVariantMap(), options.toVariantMap()),
            "prepare explicit physical dimensions for the real PDF consumer"
        );
    const auto plan = action(
                          backend,
                          {{"action", "print_plan"}, {"source", source}, {"options", options}}
    )["print_plan"]
                          .toObject();
    const auto document = card_print::prepare(plan);
    json(prefix + "-plan.json", plan);
    json(
        prefix + "-document.json",
        {{"source", source},
         {"composition", QJsonObject::fromVariantMap(kit.draft().value("composition").toMap())}}
    );
    require(
        card_print::preview(*document, 0).save(prefix + "-sheet.png"),
        "actual same-painter flexible preview"
    );
    bool finished = false, success = false;
    const auto connection =
        QObject::connect(&printer, &CardPrintController::finished, [&](bool ok) {
            finished = true;
            success = ok;
        });
    require(
        printer.exportPdf(QUrl::fromLocalFile(prefix + ".pdf"), printer.version()),
        "admit flexible PDF"
    );
    QElapsedTimer deadline;
    deadline.start();
    while (printer.running() && deadline.elapsed() < 15000)
        settle(25);
    QObject::disconnect(connection);
    require(finished && success && !printer.running(), "complete flexible PDF");
    QMetaObject::invokeMethod(&dialog, "close");
    return document;
}
bool near(const QColor& actual, const QColor& expected) {
    return std::abs(actual.red() - expected.red()) < 35
           && std::abs(actual.green() - expected.green()) < 35
           && std::abs(actual.blue() - expected.blue()) < 35;
}
QColor sample(const QImage& pixels, const card_print::Document& document, double x, double y) {
    return pixels.pixelColor(
        int(std::lround(x * pixels.width() / document.sheet_mm.width())),
        int(std::lround(y * pixels.height() / document.sheet_mm.height()))
    );
}
void verifyImageFit(const card_print::Document& document, bool cover) {
    const auto pixels = card_print::preview(document, 0);
    const auto rect = document.pages.first().placements.first().rect_mm;
    const auto at = [&](double x, double y) {
        return sample(pixels, document, rect.x() + x * rect.width(), rect.y() + y * rect.height());
    };
    if (cover) {
        require(
            near(at(.5, .03), QColor("#ff0000")) && near(at(.5, .97), QColor("#0000ff")),
            "cover preserves top and bottom while cropping the wide image sides"
        );
        require(
            !near(at(.03, .5), QColor("#ffff00")) && !near(at(.97, .5), QColor("#00aa00")),
            "cover visibly crops the source side markers"
        );
        require(
            near(sample(pixels, document, rect.left() - .6, rect.center().y()), Qt::white)
                && near(sample(pixels, document, rect.right() + .6, rect.center().y()), Qt::white),
            "native SVG clipping keeps cover pixels outside the card white"
        );
    } else {
        require(
            near(at(.03, .5), QColor("#ffff00")) && near(at(.97, .5), QColor("#00aa00"))
                && near(at(.5, .03), Qt::white),
            "contain keeps both side markers and proportional white space"
        );
    }
}
void verifyInnerImageClip(const card_print::Document& document, bool image_text) {
    const auto& slot = document.pages.first().placements.first();
    QXmlStreamReader xml(document.cards[slot.card_id]);
    QRectF clip;
    double canvas_width = 0, canvas_height = 0;
    bool artwork = false;
    while (!xml.atEnd()) {
        xml.readNext();
        if (!xml.isStartElement())
            continue;
        const auto attributes = xml.attributes();
        if (xml.name() == "svg") {
            canvas_width = attributes.value("width").toDouble();
            canvas_height = attributes.value("height").toDouble();
        }
        if (xml.name() == "clipPath")
            artwork = attributes.value("id") == "art";
        if (xml.name() == "rect" && artwork) {
            clip = QRectF(
                attributes.value("x").toDouble(),
                attributes.value("y").toDouble(),
                attributes.value("width").toDouble(),
                attributes.value("height").toDouble()
            );
            break;
        }
        if (xml.name() == "use") {
            clip = QRectF(
                attributes.value("x").toDouble(),
                attributes.value("y").toDouble(),
                attributes.value("width").toDouble(),
                attributes.value("height").toDouble()
            );
            if (!clip.isEmpty())
                break;
        }
    }
    require(
        !clip.isEmpty() && clip.top() > 30 && canvas_width > 0 && canvas_height > 0,
        "padded image-text region exists"
    );
    const auto pixels = card_print::preview(document, 0);
    const auto color = [&](double x, double y) {
        return sample(
            pixels,
            document,
            slot.content_rect_mm.x() + x / canvas_width * slot.content_rect_mm.width(),
            slot.content_rect_mm.y() + y / canvas_height * slot.content_rect_mm.height()
        );
    };
    require(
        near(color(clip.center().x(), clip.top() - 25), QColor("#ffeecc"))
            && near(color(clip.center().x(), clip.bottom() + 12), QColor("#ffeecc"))
            && near(color(clip.left() - 25, clip.center().y()), QColor("#ffeecc"))
            && near(color(clip.right() + 25, clip.center().y()), QColor("#ffeecc")),
        "cover is clipped inside image-text region, preserving padding and the text gap"
    );
    if (image_text)
        require(
            near(color(clip.left() + 10, clip.center().y()), QColor("#ffff00")),
            "cover region still contains the source image"
        );
}
} // namespace

bool verify(
    DesktopBackend& backend,
    CardKitController& kit,
    UiPreferences& preferences,
    QObject& root,
    const QString& directory
) {
    try {
        require(
            QDir().mkpath(directory)
                && backend.createProject(QUrl::fromLocalFile(directory), "Flexible cards"),
            "isolated flexible card project"
        );
        preferences.setLanguageMode("en");
        QFont face_font("sans-serif");
        face_font.setPixelSize(105);
        const QFontMetricsF face_metrics(face_font);
        double maximum_ascii_advance = 0;
        for (int code = 32; code <= 126; ++code)
            maximum_ascii_advance = std::max(
                maximum_ascii_advance,
                face_metrics.horizontalAdvance(QChar(static_cast<char16_t>(code)))
            );
        require(
            maximum_ascii_advance <= 105 * 1.05,
            "the actual SVG sans-serif font fits the conservative flexible-card ASCII budget"
        );
        json(
            directory + "/native-font-metrics.json",
            {{"requested_family", "sans-serif"},
             {"resolved_family", QFontInfo(face_font).family()},
             {"pixel_size", 105},
             {"maximum_ascii_advance", maximum_ascii_advance},
             {"W_advance", face_metrics.horizontalAdvance(QChar(u'W'))},
             {"at_advance", face_metrics.horizontalAdvance(QChar(u'@'))},
             {"budget_per_character", 105 * 1.05}}
        );
        auto* window = qobject_cast<QQuickWindow*>(&root);
        auto* printer = qvariant_cast<CardPrintController*>(root.property("cardPrint"));
        require(window && printer, "real flexible card host");
        auto& workspace = child(root, "cardKitWorkspace");
        evaluate(workspace, "openWorkspace('')");
        auto& intake = child(workspace, "cardImageIntakeDialog");
        const auto a = directory + "/A-complete.png", b = directory + "/B-retry.png",
                   c = directory + "/C-square.png";
        image(a, 1200, 800, "A COMPLETE CARD");
        image(c, 800, 800, "C SQUARE");
        const auto original = documentState(workspace);
        const auto before = backend.artifacts().size();
        selectFiles(workspace, intake, {a, c});
        json(
            directory + "/pending-before-after.json",
            {{"before", QJsonObject::fromVariantMap(original)},
             {"after", QJsonObject::fromVariantMap(documentState(workspace))},
             {"artifacts_before", before},
             {"artifacts_after", backend.artifacts().size()}}
        );
        require(
            documentState(workspace) == original && backend.artifacts().size() == before,
            "pending file list does not import or mutate a card"
        );
        click(intake, "cardImagesCancel");
        require(
            documentState(workspace) == original,
            "cancel pending images preserves the starter"
        );
        selectFiles(workspace, intake, {a, b, c});
        const auto batch = plainProperty(intake, "entries").toList();
        require(batch.size() == 3, "pending multiselect contains all three ordered entries");
        click(intake, "cardImagesApply");
        waitIntake(intake);
        require(
            intake.property("addedCount").toInt() == 2
                && intake.property("failedCount").toInt() == 1 && cards(workspace).size() == 2,
            "a bad image reports partial success without an empty extra card"
        );
        const auto partial_count = backend.artifacts().size();
        image(b, 800, 1200, "B RETRY");
        click(intake, "cardImagesApply");
        waitIntake(intake);
        const auto imported = cards(workspace);
        require(
            imported.size() == 3 && backend.artifacts().size() == partial_count + 1,
            "retry imports only the failed file once"
        );
        for (int index = 0; index < 3; ++index)
            require(
                imported[index].toMap().value("id") == batch[index].toMap().value("id")
                    && imported[index].toMap().value("title").toString().isEmpty()
                    && imported[index].toMap().value("layout").toMap().value("mode") == "image",
                "partial retry restores A/B/C order, stable IDs, and explicit full-image cards"
            );
        json(
            directory + "/intake-proof.json",
            {{"entries", QJsonArray::fromVariantList(plainProperty(intake, "entries").toList())},
             {"composition", QJsonObject::fromVariantMap(documentState(workspace))}}
        );
        click(intake, "cardImagesCancel");
        const auto kit_id = kit.draft().value("artifact_id").toString();
        const auto first = imported[0].toMap().value("id").toString(),
                   second = imported[1].toMap().value("id").toString(),
                   third = imported[2].toMap().value("id").toString();
        evaluate(workspace, "selectedCardId = " + literal(first));
        const auto landscape = exportLayout(
            backend,
            kit,
            *printer,
            root,
            workspace,
            directory + "/image-landscape-contain"
        );
        require(
            landscape->summary["options"].toObject()["card_width_mm"].toDouble()
                > landscape->summary["options"].toObject()["card_height_mm"].toDouble(),
            "print defaults follow imported landscape proportions"
        );
        evaluate(workspace, "setCanvas(900, 900); flush()");
        const auto contained = exportLayout(
            backend,
            kit,
            *printer,
            root,
            workspace,
            directory + "/image-square-contain"
        );
        verifyImageFit(*contained, false);
        auto& card_options = child(workspace, "cardKitCardLayoutOptions");
        evaluate(
            card_options,
            "setValue('image_fit', 'cover'); setValue('background_color', '#ffeecc')"
        );
        evaluate(workspace, "flush()");
        const auto covered = exportLayout(
            backend,
            kit,
            *printer,
            root,
            workspace,
            directory + "/image-square-cover"
        );
        verifyImageFit(*covered, true);
        evaluate(
            workspace,
            "setCardField('layout', "
            "{mode:'image',image_fit:'cover',padding_percent:10,background_color:'#ffeecc'}); "
            "flush()"
        );
        const auto padded_image = exportLayout(
            backend,
            kit,
            *printer,
            root,
            workspace,
            directory + "/image-cover-padded"
        );
        verifyInnerImageClip(*padded_image, false);
        evaluate(
            workspace,
            "setCardField('rules', 'KEEP TEXT CLEAR'); setCardField('layout', "
            "{mode:'image_text',image_fit:'cover',padding_percent:10,background_color:'#ffeecc'}); "
            "flush()"
        );
        const auto padded = exportLayout(
            backend,
            kit,
            *printer,
            root,
            workspace,
            directory + "/image-text-cover-padded"
        );
        verifyInnerImageClip(*padded, true);
        click(workspace, "cardKitSetLayout");
        auto& default_options = child(workspace, "cardKitDefaultLayoutOptions");
        require(
            child(default_options, "cardImageFitDefault").property("currentIndex").toInt() == 0
                && child(default_options, "cardBackgroundDefault").property("text") == "#ffffff",
            "per-card cover and color never masquerade as set defaults"
        );
        evaluate(child(workspace, "cardKitSetLayoutDialog"), "customRatioExpanded = true");
        child(workspace, "cardKitRatioWidth").setProperty("text", "13");
        child(workspace, "cardKitRatioHeight").setProperty("text", "1");
        const auto ratio_before = documentState(workspace);
        click(workspace, "cardKitRatioApply");
        require(
            documentState(workspace) == ratio_before,
            "invalid custom proportions never clamp or mutate the set"
        );
        child(workspace, "cardKitRatioWidth").setProperty("text", "4");
        child(workspace, "cardKitRatioHeight").setProperty("text", "3");
        click(workspace, "cardKitRatioApply");
        require(
            documentState(workspace).value("canvas_width").toInt() * 3
                == documentState(workspace).value("canvas_height").toInt() * 4,
            "custom width-to-height input maps to the actual canvas"
        );
        click(workspace, "cardKitSetLayoutClose");
        evaluate(
            workspace,
            "setCanvas(900,900); selectedCardId = " + literal(second)
                + "; setCardField('layout', null); setCardField('rules', 'SPRING\\n春'); flush(); "
                  "controller.review(selectedCardId)"
        );
        const auto image_pin = cards(workspace)[1].toMap().value("artwork").toMap();
        require(
            plainProperty(workspace, "currentLayout").toMap().value("mode") == "image_text",
            "automatic layout sees image and body text without a title"
        );
        click(workspace, "cardKitRemoveImage");
        require(
            kit.review(second)
                && plainProperty(workspace, "currentLayout").toMap().value("mode") == "text",
            "removing an image binding naturally makes an automatic text card"
        );
        evaluate(
            workspace,
            "bindArtwork(" + literal(second) + ", "
                + QString::fromUtf8(
                    QJsonDocument::fromVariant(image_pin).toJson(QJsonDocument::Compact)
                )
                + "); setCardField('layout', {mode:'text',text_scale_percent:200}); flush(); "
                  "controller.review(selectedCardId)"
        );
        require(
            plainProperty(workspace, "hiddenContent").toMap().value("artwork").toBool()
                && child(workspace, "cardKitHiddenContent")
                       .property("text")
                       .toString()
                       .contains("kept"),
            "text-only layout explicitly retains and explains a hidden image"
        );
        const auto retained = cards(workspace)[1].toMap();
        evaluate(card_options, "chooseMode(choices.indexOf('image'))");
        evaluate(workspace, "flush(); controller.review(selectedCardId)");
        require(
            plainProperty(workspace, "hiddenContent")
                    .toMap()
                    .value("text_fields")
                    .toList()
                    .contains("rules")
                && cards(workspace)[1].toMap().value("rules") == retained.value("rules"),
            "image layout keeps existing body text and declares it hidden"
        );
        evaluate(card_options, "chooseMode(choices.indexOf('text'))");
        evaluate(
            workspace,
            "selectedCardId = " + literal(third)
                + "; setCardField('layout', {mode:'image_text'}); setCardField('rules', 'Which "
                  "colors mark the four edges?'); flush()"
        );
        exportLayout(backend, kit, *printer, root, workspace, directory + "/mixed-square");
        evaluate(workspace, "selectedCardId = " + literal(second));
        evaluate(card_options, "chooseColor('text_color')");
        auto& color_dialog = child(card_options, "cardLayoutColorDialog");
        color_dialog.setProperty("selectedColor", QColor("#123456"));
        QMetaObject::invokeMethod(&color_dialog, "accepted");
        require(
            cards(workspace)[1].toMap().value("layout").toMap().value("text_color") == "#123456",
            "native color-picker acceptance changes the intended card"
        );
        evaluate(card_options, "chooseColor('text_color')");
        evaluate(workspace, "selectedCardId = " + literal(third));
        const auto before_color = documentState(workspace);
        color_dialog.setProperty("selectedColor", QColor("#ff00ff"));
        QMetaObject::invokeMethod(&color_dialog, "accepted");
        require(
            documentState(workspace) == before_color,
            "late color dialog cannot edit a different selected card"
        );
        evaluate(workspace, "setCanvas(750,1050); flush()");
        exportLayout(backend, kit, *printer, root, workspace, directory + "/mixed-portrait");
        const auto wide_size = window->size();
        window->resize(1120, 720);
        settle();
        evaluate(workspace, "selectedCardId = " + literal(second));
        capture(*window, directory + "/cards-en-narrow.png");
        preferences.setLanguageMode("zh_CN");
        settle();
        capture(*window, directory + "/cards-zh-narrow.png");
        click(workspace, "cardKitPrint");
        capture(*window, directory + "/print-zh-narrow.png");
        preferences.setLanguageMode("en");
        settle();
        capture(*window, directory + "/print-en-narrow.png");
        QMetaObject::invokeMethod(&child(root, "cardPrintDialog"), "close");
        window->resize(wide_size);
        const auto accepted = kit.source();
        evaluate(workspace, "acceptAndPrint()");
        require(
            kit.source() == accepted,
            "already accepted current content opens print without a duplicate acceptance"
        );
        QMetaObject::invokeMethod(&child(root, "cardPrintDialog"), "close");
        evaluate(
            workspace,
            "newKit(); setCanvas(900,900); "
            "setCardField('layout',{mode:'text',text_scale_percent:50}); setCardField('rules',"
                + literal(QStringLiteral("cat ").repeated(120))
                + "); flush(); controller.review(selectedCardId)"
        );
        require(
            !kit.preview().value("ready_to_export").toBool() && !kit.currentAccepted(),
            "unaccepted default-size text overflow fixture"
        );
        click(workspace, "cardKitPrint");
        require(
            child(root, "cardPrintDialog").property("visible").toBool() && kit.currentAccepted()
                && !kit.canExport() && !printer->ready() && printer->error().contains("readable"),
            "explicit adoption succeeds while the print window honestly reports default physical "
            "text overflow"
        );
        const auto overflow_source = kit.source();
        const auto overflow_kit = kit.draft().value("artifact_id").toString();
        require(
            !kit.exportFiles(QUrl::fromLocalFile(directory + "/overflow-materials.zip"))
                && !QFile::exists(directory + "/overflow-materials.zip"),
            "material export is not relaxed for overflowing text"
        );
        QMetaObject::invokeMethod(&child(root, "cardPrintDialog"), "close");
        evaluate(workspace, "selectKit(" + literal(overflow_kit) + "); acceptAndPrint()");
        require(
            kit.currentAccepted() && kit.source() == overflow_source && !printer->ready(),
            "accepted overflowing version can reopen print without another acceptance"
        );
        const QJsonObject expanded_options{{"card_width_mm", 100}, {"card_height_mm", 100}};
        require(
            printer->prepare(overflow_source, expanded_options.toVariantMap()),
            "larger physical card makes the text readable without shrinking it"
        );
        const auto expanded_plan = action(
                                       backend,
                                       {{"action", "print_plan"},
                                        {"source", QJsonObject::fromVariantMap(overflow_source)},
                                        {"options", expanded_options}}
        )["print_plan"]
                                       .toObject();
        const auto expanded_document = card_print::prepare(expanded_plan);
        card_print::exportPdf(
            *expanded_document,
            directory + "/overflow-expanded.pdf",
            backend.bundlePath()
        );
        require(
            card_print::preview(*expanded_document, 0)
                .save(directory + "/overflow-expanded-sheet.png"),
            "expanded readable text preview"
        );
        json(directory + "/overflow-expanded-plan.json", expanded_plan);
        QMetaObject::invokeMethod(&child(root, "cardPrintDialog"), "close");
        evaluate(workspace, "selectKit(" + literal(kit_id) + ")");
        QStringList too_many;
        for (int index = 0; index < 62; ++index)
            too_many.push_back(directory + QStringLiteral("/pending-%1.png").arg(index));
        const auto before_capacity = backend.artifacts().size();
        selectFiles(workspace, intake, too_many);
        require(
            !intake.property("error").toString().isEmpty()
                && !child(intake, "cardImagesApply").property("enabled").toBool()
                && backend.artifacts().size() == before_capacity,
            "capacity is checked before the first file import"
        );
        click(intake, "cardImagesCancel");
        selectFiles(workspace, intake, {a});
        click(intake, "cardImagesCancel");
        evaluate(card_options, "chooseColor('text_color')");
        evaluate(workspace, "newKit()");
        const auto new_set = documentState(workspace);
        const auto before_stale = backend.artifacts().size();
        evaluate(intake, "prepareFiles([" + literal(QUrl::fromLocalFile(a).toString()) + "])");
        QMetaObject::invokeMethod(&color_dialog, "accepted");
        require(
            documentState(workspace) == new_set && backend.artifacts().size() == before_stale,
            "late image and color callbacks do not mutate or create material in a different kit"
        );
        const auto extreme = directory + "/extreme.png";
        image(extreme, 4000, 80, "EXTREME");
        evaluate(workspace, "newKit()");
        const auto extreme_before = documentState(workspace);
        selectFiles(workspace, intake, {extreme});
        click(intake, "cardImagesApply");
        waitIntake(intake);
        const auto extreme_entries = plainProperty(intake, "entries").toList();
        require(extreme_entries.size() == 1, "single extreme image intake entry");
        require(
            documentState(workspace).value("canvas_width") == extreme_before.value("canvas_width")
                && documentState(workspace).value("canvas_height")
                       == extreme_before.value("canvas_height")
                && extreme_entries.first().toMap().value("message").toString().contains(
                    "unusually"
                ),
            "extreme first image keeps the canvas and honestly explains complete-image fitting"
        );
        click(intake, "cardImagesCancel");
        if (qEnvironmentVariableIsSet("SHAPE_SMOKE_CARD_FLEX_EXTREME_COVER")) {
            evaluate(workspace, "setCardField('layout',{mode:'image',image_fit:'cover'}); flush()");
            const auto extreme_cover = exportLayout(
                backend,
                kit,
                *printer,
                root,
                workspace,
                directory + "/extreme-cover-190x266",
                {{"card_width_mm", 190}, {"card_height_mm", 266}}
            );
            verifyImageFit(*extreme_cover, true);
        }
        evaluate(workspace, "selectKit(" + literal(kit_id) + ")");
        require(kit.draft().value("artifact_id") == kit_id, "return to saved flexible kit");
        const auto original_bundle = backend.bundlePath();
        evaluate(workspace, "selectedCardId = " + literal(second));
        evaluate(card_options, "chooseColor('text_color')");
        const auto copied_bundle = directory + "/color-copy.shape";
        std::filesystem::copy(
            original_bundle.toStdString(),
            copied_bundle.toStdString(),
            std::filesystem::copy_options::recursive
        );
        require(
            backend.openProject(QUrl::fromLocalFile(copied_bundle)),
            "open copied bundle with identical project and card identities"
        );
        evaluate(workspace, "openWorkspace(" + literal(kit_id) + ")");
        const auto copied_state = documentState(workspace);
        QMetaObject::invokeMethod(&color_dialog, "accepted");
        require(
            documentState(workspace) == copied_state,
            "same-ID copied bundle rejects the old color callback"
        );
        require(
            backend.openProject(QUrl::fromLocalFile(original_bundle)),
            "restore original bundle after color callback probe"
        );
        evaluate(workspace, "openWorkspace(" + literal(kit_id) + "); newKit()");
        auto legacy = documentState(workspace);
        legacy.remove("layout");
        auto legacy_card = legacy.value("cards").toList().first().toMap();
        legacy_card["title"] = "Original game card";
        legacy_card["rules"] = "Keep the saved appearance.";
        legacy_card["artwork"] = imported[0].toMap().value("artwork");
        legacy["cards"] = QVariantList{legacy_card};
        require(
            kit.save(legacy) && kit.review(legacy_card["id"].toString()) && kit.accept(),
            "legacy accepted-card fixture"
        );
        const auto legacy_source = kit.source();
        const auto legacy_before = documentState(workspace);
        const auto legacy_artifacts = backend.artifacts().size();
        evaluate(workspace, "beginCardImages()");
        require(
            child(workspace, "cardKitSetLayoutDialog").property("visible").toBool()
                && !child(intake, "cardImagesFileDialog").property("visible").toBool()
                && documentState(workspace) == legacy_before
                && backend.artifacts().size() == legacy_artifacts,
            "legacy image intake explains the conversion before importing or changing the set"
        );
        evaluate(default_options, "expanded = true");
        evaluate(child(default_options, "cardBackgroundDefault"), "forceActiveFocus()");
        evaluate(child(default_options, "cardTextColorDefault"), "forceActiveFocus()");
        click(workspace, "cardKitSetLayoutClose");
        require(
            documentState(workspace) == legacy_before && kit.source() == legacy_source
                && !documentState(workspace).contains("layout"),
            "opening, focusing and leaving old layout controls never upgrades legacy content"
        );
        evaluate(
            workspace,
            "newKit(); setCardField('rules','A readable word'); flush(); "
            "controller.review(selectedCardId)"
        );
        require(kit.accept() && kit.currentAccepted(), "accepted-state notification fixture");
        const auto notification_source = kit.source();
        const auto notification_draft = QJsonObject::fromVariantMap(kit.draft());
        require(
            kit.review(workspace.property("selectedCardId").toString()),
            "review accepted content before a concurrent draft save"
        );
        auto concurrent = notification_draft["composition"].toObject();
        concurrent["name"] = "Concurrent draft change";
        require(
            !action(
                 backend,
                 {{"action", "draft_save"},
                  {"artifact_id", notification_draft["artifact_id"]},
                  {"draft_digest", notification_draft["draft_digest"]},
                  {"composition", concurrent}}
            )["draft"]
                 .toObject()
                 .isEmpty(),
            "concurrent change advances the persisted draft"
        );
        int accepted_notifications = 0, document_notifications = 0;
        QObject notification_observer;
        QObject::connect(
            &kit,
            &CardKitController::currentAcceptedChanged,
            &notification_observer,
            [&] { ++accepted_notifications; }
        );
        QObject::connect(&kit, &CardKitController::documentChanged, &notification_observer, [&] {
            ++document_notifications;
        });
        require(
            !kit.accept() && !kit.currentAccepted() && accepted_notifications == 1
                && document_notifications == 0 && kit.source() == notification_source,
            "failed acceptance notifies the current-state binding without resyncing the document "
            "or advancing the accepted pin"
        );
        evaluate(workspace, "selectKit(" + literal(kit_id) + ")");
        json(
            directory + "/report.json",
            {{"source", QJsonObject::fromVariantMap(accepted)},
             {"bundle", backend.bundlePath()},
             {"partial_retry_order", true},
             {"image_names_not_printed", true},
             {"cover_pixels_clipped", true},
             {"inner_cover_clip_preserves_text_gap", true},
             {"hidden_content_retained", true},
             {"stale_color_rejected", true},
             {"copied_bundle_color_rejected", true},
             {"legacy_no_upgrade", true},
             {"accept_failure_notifies_current_state", true},
             {"custom_ratio", true},
             {"extreme_ratio_honest", true},
             {"extreme_cover_opt_in",
              qEnvironmentVariableIsSet("SHAPE_SMOKE_CARD_FLEX_EXTREME_COVER")}}
        );
        std::cout << "flexible cards: full-image batch, partial retry order, text, hidden content, "
                     "custom ratio, cover pixels, physical PDFs and bilingual UI passed"
                  << std::endl;
        return true;
    } catch (const std::exception& error) {
        std::cerr << "flexible-card smoke: " << error.what() << std::endl;
        return false;
    }
}
} // namespace card_flex_smoke

#include "picture_book_smoke.hpp"
#include "desktop_backend.hpp"
#include "picture_book_controller.hpp"
#include "ui_preferences.hpp"
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
#include <utility>

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
QJsonObject artifactPin(DesktopBackend& backend, const QString& id) {
    for (const auto& value : backend.artifacts())
        if (value.toMap().value("id").toString() == id)
            return pin(value.toMap());
    throw std::runtime_error("material output missing");
}
QString artworkStage(DesktopBackend& backend, const QJsonObject& source) {
    const auto draft =
        backend.beginOperatorDraft(source["artifact_id"].toString(), "image.external_edit");
    for (const auto& value : backend.operatorDrafts())
        if (value.toMap().value("id").toString() == draft)
            return value.toMap().value("contextArtifactId").toString();
    throw std::runtime_error("artwork editing stage missing");
}
QJsonObject acceptArtworkReturn(
    DesktopBackend& backend,
    const QString& output,
    const QImage& image,
    const QString& directory
) {
    const auto project = backend.bundlePath().toStdString();
    const auto ticket = backend.materialEditTicket(output).toStdString();
    const auto handoff = directory + "/handoff.json";
    const auto returned = directory + "/returned.png";
    (void)shape::desktop::export_material_copy(project, ticket, directory.toStdString());
    require(image.save(returned), "write synthetic artwork return");
    auto prepared = shape::desktop::prepare_material_return(
        project,
        ticket,
        handoff.toStdString(),
        returned.toStdString(),
        false
    );
    const auto candidate = backend.adoptMaterialReturn(std::move(prepared));
    require(
        !candidate.isEmpty() && backend.acceptCandidate(candidate),
        "explicit artwork return adoption"
    );
    return artifactPin(backend, output);
}
void click(QObject& workspace, const QString& name) {
    auto* button = workspace.findChild<QObject*>(name);
    require(
        button && QMetaObject::invokeMethod(button, "clicked"),
        "packaged book action reachable"
    );
    QCoreApplication::processEvents();
}
void capture(QQuickWindow& window, const QString& path) {
    window.requestUpdate();
    QEventLoop loop;
    QTimer::singleShot(450, &loop, &QEventLoop::quit);
    loop.exec();
    require(window.grabWindow().save(path), "actual workspace capture");
}
void continuation(
    DesktopBackend& backend,
    PictureBookController& book,
    UiPreferences& preferences,
    QQuickWindow& window,
    QObject& workspace,
    const QString& output,
    const QImage& second,
    const QString& directory,
    QJsonObject& request
) {
    auto* editor = workspace.findChild<QObject*>("pictureBookScript");
    require(editor, "packaged script editor");
    evaluate(*editor, QStringLiteral("insert(length, '\\n继续写下去。')"));
    const auto typed = workspace.property("script").toString();
    require(
        workspace.property("scriptDirty").toBool() && typed.contains("继续写下去。"),
        "real script insertion marks unadopted text"
    );
    const auto authored = workspace.property("draft").toMap();
    click(workspace, "refreshPictureBookMaterials");
    require(
        workspace.property("script").toString() == typed
            && workspace.property("scriptDirty").toBool()
            && workspace.property("draft").toMap() == authored,
        "refresh cannot overwrite an unadopted script"
    );
    for (const auto& name :
         {"refreshPictureBookMaterials",
          "pictureBookLanguagePicker",
          "pictureBookTextMaterial",
          "addPictureBookLanguage"}) {
        auto* item = workspace.findChild<QObject*>(QString::fromUtf8(name));
        require(
            item && !item->property("enabled").toBool(),
            "script replacement action is disabled"
        );
    }
    auto* hint = workspace.findChild<QObject*>("pictureBookPendingScriptHint");
    require(hint && hint->property("visible").toBool(), "pending script guidance visible");
    require(
        hint->property("text").toString()
            == QStringLiteral(
                "Adopt this script or discard the draft before switching "
                "languages, choosing text, or refreshing materials."
            ),
        "packaged English script guidance"
    );
    capture(window, directory + "-pending-script-en.png");
    preferences.setLanguageMode(QStringLiteral("zh_CN"));
    QCoreApplication::processEvents();
    require(
        hint->property("text").toString()
                == QStringLiteral("切换语言、选择文本或刷新素材前，请先采用当前脚本或放弃草稿。")
            && workspace.property("script").toString() == typed
            && workspace.property("scriptDirty").toBool()
            && workspace.property("draft").toMap() == authored,
        "packaged Chinese guidance preserves typed script and composition"
    );
    capture(window, directory + "-pending-script-zh.png");
    preferences.setLanguageMode(QStringLiteral("en"));
    QCoreApplication::processEvents();
    evaluate(workspace, QStringLiteral("chooseLanguage('en'); selectText(1)"));
    auto* code = workspace.findChild<QObject*>("pictureBookLanguageCode");
    require(code, "language code field");
    code->setProperty("text", "fr");
    click(workspace, "addPictureBookLanguage");
    require(
        workspace.property("activeLanguage").toString() == "zh-CN"
            && workspace.property("script").toString() == typed
            && workspace.property("draft").toMap() == authored,
        "guarded handlers preserve script, language and material pin"
    );
    QEventLoop closed;
    require(
        bool(QObject::connect(&workspace, SIGNAL(closed()), &closed, SLOT(quit()))),
        "book close signal reachable"
    );
    QTimer::singleShot(2000, &closed, &QEventLoop::quit);
    evaluate(workspace, QStringLiteral("close()"));
    if (workspace.property("visible").toBool())
        closed.exec();
    require(
        !workspace.property("opened").toBool() && !workspace.property("visible").toBool(),
        "book workspace fully closed before return"
    );
    evaluate(workspace, QStringLiteral("openWorkspace()"));
    require(
        workspace.property("script").toString() == typed
            && workspace.property("scriptDirty").toBool(),
        "returning from material work preserves typed script"
    );
    evaluate(*editor, QStringLiteral("insert(length, '\\n[Unknown]\\nKeep this failed draft.')"));
    const auto failed_text = workspace.property("script").toString();
    click(workspace, "adoptPictureBookScript");
    require(
        workspace.property("script").toString() == failed_text
            && workspace.property("scriptDirty").toBool()
            && workspace.property("draft").toMap() == authored && !book.error().isEmpty(),
        "failed script adoption retains text and composition for retry"
    );
    evaluate(*editor, QStringLiteral("remove(%1, length)").arg(typed.size()));
    click(workspace, "adoptPictureBookScript");
    require(
        !workspace.property("scriptDirty").toBool()
            && workspace.property("script").toString() == typed,
        "explicit script adoption preserves exact authored text"
    );
    click(workspace, "reviewPictureBook");
    require(book.hasReview(), "review after script adoption");
    click(workspace, "acceptPictureBook");
    require(!workspace.property("dirty").toBool(), "book adoption restores clean draft");
    click(workspace, "reviewPictureBook");
    evaluate(workspace, QStringLiteral("choosePage(2)"));
    const auto candidate = book.candidateId();
    const auto preview = book.previewSource();
    const auto source = book.source();
    const auto reviewed = workspace.property("draft").toMap();
    require(!candidate.isEmpty(), "review before unchanged refresh");
    click(workspace, "refreshPictureBookMaterials");
    require(
        book.candidateId() == candidate && book.previewSource() == preview
            && !workspace.property("dirty").toBool() && book.source() == source
            && workspace.property("draft").toMap() == reviewed
            && workspace.property("activeLanguage").toString() == "zh-CN"
            && workspace.property("pageIndex").toInt() == 2,
        "unchanged material refresh preserves review, preview and clean state"
    );
    const auto previous = QJsonObject::fromVariantMap(workspace.property("draft").toMap());
    const auto image_pin = previous["pages"].toArray()[0].toObject()["source"].toObject();
    const auto newer = acceptArtworkReturn(backend, output, second, directory);
    require(newer != image_pin, "accepted editing-stage output advances");
    click(workspace, "refreshPictureBookMaterials");
    const auto refreshed = QJsonObject::fromVariantMap(workspace.property("draft").toMap());
    const auto pages = refreshed["pages"].toArray();
    require(
        pages[0].toObject()["source"].toObject() == newer
            && pages[2].toObject()["source"].toObject() == newer && !book.hasReview()
            && book.previewSource().isEmpty() && workspace.property("dirty").toBool(),
        "real material refresh updates repeated pages and requires review"
    );
    require(
        workspace.property("script").toString() == typed
            && !workspace.property("scriptDirty").toBool()
            && refreshed["languages"] == previous["languages"]
            && workspace.property("activeLanguage").toString() == "zh-CN"
            && workspace.property("pageIndex").toInt() == 2,
        "artwork refresh preserves adopted language scripts and selection"
    );
    const auto old = parsed(backend.bookRead(
        QString::fromUtf8(
            QJsonDocument(QJsonObject::fromVariantMap(source)).toJson(QJsonDocument::Compact)
        )
    ));
    require(
        old["pages"].toArray()[0].toObject()["source"].toObject() == image_pin,
        "old accepted book keeps its material pin"
    );
    click(workspace, "reviewPictureBook");
    require(book.hasReview(), "review continued material version");
    click(workspace, "acceptPictureBook");
    require(!workspace.property("dirty").toBool(), "continued book explicitly adopted");
    const auto saved = workspace.property("draft").toMap();
    const auto saved_source = book.source();
    evaluate(*editor, QStringLiteral("insert(length, '\\nTemporary draft to discard.')"));
    require(workspace.property("scriptDirty").toBool(), "explicit discard has typed changes");
    evaluate(workspace, QStringLiteral("setField('name', 'Temporary book title')"));
    require(workspace.property("draft").toMap() != saved, "explicit discard has book changes");
    click(workspace, "discardPictureBookDraft");
    require(
        !workspace.property("scriptDirty").toBool() && !workspace.property("dirty").toBool()
            && workspace.property("draft").toMap() == saved && book.source() == saved_source
            && workspace.property("script").toString() == typed,
        "explicit discard restores adopted book without changing its head"
    );
    request = QJsonObject::fromVariantMap(workspace.property("draft").toMap());
}
} // namespace
bool verify(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& root,
    const QString& directory
) {
    try {
        auto* preferences = root.property("uiPreferences").value<UiPreferences*>();
        require(preferences, "host UI preferences reachable");
        // The --smoke-picture-book host supplies a temporary, isolated settings file.
        preferences->setLanguageMode(QStringLiteral("en"));
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
        const auto original_image = image;
        const auto original_bytes = backend.bookMaterial(
            QString::fromUtf8(QJsonDocument(image).toJson(QJsonDocument::Compact)),
            "image"
        );
        const auto stage = artworkStage(backend, image);
        auto first_artwork = artwork.copy();
        {
            QPainter p(&first_artwork);
            p.fillRect(QRect(10, 570, 20, 20), QColor("#e8d789"));
        }
        image = acceptArtworkReturn(backend, stage, first_artwork, directory + "/artwork-first");
        auto second_artwork = first_artwork.copy();
        {
            QPainter p(&second_artwork);
            p.fillRect(QRect(40, 570, 20, 20), QColor("#ffe8ac"));
        }
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
        auto* window = qobject_cast<QQuickWindow*>(&root);
        require(window, "window exists");
        continuation(
            backend,
            book,
            *preferences,
            *window,
            *workspace,
            stage,
            second_artwork,
            directory + "/artwork-second",
            request
        );
        require(
            artifactPin(backend, original_image["artifact_id"].toString()) == original_image
                && backend.bookMaterial(
                       QString::fromUtf8(
                           QJsonDocument(original_image).toJson(QJsonDocument::Compact)
                       ),
                       "image"
                   ) == original_bytes,
            "editing-stage versions preserve original artwork bytes/head"
        );
        require(
            backend.bookMaterial(
                QString::fromUtf8(QJsonDocument(zh_pin).toJson(QJsonDocument::Compact)),
                "text"
            ) == zh.toUtf8(),
            "script branch preserves original Chinese material"
        );
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
        capture(*window, directory + "/shape-book-workspace.png");
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
                 "typed script protected from material/language replacement",
                 "typed script survives return to book",
                 "packaged English/Chinese guidance preserves typed script",
                 "failed script adoption retains input for retry",
                 "explicit script adoption preserves authored text",
                 "explicit discard restores adopted book head",
                 "no-op refresh preserves review and clean state",
                 "repeated artwork pages refresh exact accepted stage output",
                 "continued book re-reviewed and adopted",
                 "original artwork and language material preserved",
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

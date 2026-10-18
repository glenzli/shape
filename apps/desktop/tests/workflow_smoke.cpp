#include "workflow_smoke.hpp"
#include "card_flex_smoke.hpp"
#include "card_kit_controller.hpp"
#include "desktop_backend.hpp"
#include "ui_preferences.hpp"
#include <QClipboard>
#include <QCoreApplication>
#include <QDir>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QKeyEvent>
#include <QLinearGradient>
#include <QMouseEvent>
#include <QPainter>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSettings>
#include <QTimer>
#include <filesystem>
#include <iostream>
#include <stdexcept>

namespace workflow_smoke {
namespace {
void require(bool value, const char* why) {
    if (!value)
        throw std::runtime_error(why);
}
void settle(int milliseconds = 100) {
    QEventLoop loop;
    QTimer::singleShot(milliseconds, &loop, &QEventLoop::quit);
    loop.exec();
}
void evaluate(QObject& object, const QString& script) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, script);
    expression.evaluate();
    if (expression.hasError())
        throw std::runtime_error(expression.error().toString().toStdString());
    QCoreApplication::processEvents();
}
QQuickItem* visualChild(QQuickItem* root, const QString& name) {
    if (!root)
        return nullptr;
    if (root->objectName() == name)
        return root;
    for (auto* item : root->childItems())
        if (auto* found = visualChild(item, name))
            return found;
    return nullptr;
}
QObject& child(QObject& root, const char* name) {
    auto* found = root.findChild<QObject*>(QString::fromLatin1(name));
    if (!found) {
        auto* content = qobject_cast<QQuickItem*>(&root);
        if (!content)
            content = qvariant_cast<QQuickItem*>(root.property("contentItem"));
        if (auto* window = qobject_cast<QQuickWindow*>(&root))
            content = window->contentItem();
        found = visualChild(content, QString::fromLatin1(name));
    }
    require(found, name);
    return *found;
}
bool imageReady(QObject& root, const char* name) {
    for (int attempt = 0; attempt < 40; ++attempt) {
        const auto status = child(root, name).property("status").toInt();
        if (status == 1)
            return true;
        if (status == 3)
            return false;
        settle(25);
    }
    return false;
}
void click(QObject& root, const char* name) {
    auto& item = child(root, name);
    require(item.property("enabled").toBool() && QMetaObject::invokeMethod(&item, "clicked"), name);
    settle();
}
void pointerClick(QQuickWindow& window, QObject& object) {
    auto* item = qobject_cast<QQuickItem*>(&object);
    require(item && item->isVisible() && item->isEnabled(), "visible task entry");
    const auto position = item->mapToScene(QPointF(item->width() / 2, item->height() / 2));
    const QPointF global(window.mapToGlobal(position.toPoint()));
    QMouseEvent press(
        QEvent::MouseButtonPress,
        position,
        position,
        global,
        Qt::LeftButton,
        Qt::LeftButton,
        Qt::NoModifier
    );
    QMouseEvent release(
        QEvent::MouseButtonRelease,
        position,
        position,
        global,
        Qt::LeftButton,
        Qt::NoButton,
        Qt::NoModifier
    );
    QCoreApplication::sendEvent(&window, &press);
    QCoreApplication::sendEvent(&window, &release);
    settle(220);
}
void type(QQuickWindow& window, QObject& editor, const QString& text, bool multiline = false) {
    evaluate(editor, multiline ? "focusEditor()" : "forceActiveFocus()");
    QKeyEvent key(QEvent::KeyPress, Qt::Key_unknown, Qt::NoModifier, text);
    QCoreApplication::sendEvent(&window, &key);
    QKeyEvent release(QEvent::KeyRelease, Qt::Key_unknown, Qt::NoModifier, text);
    QCoreApplication::sendEvent(&window, &release);
    QCoreApplication::processEvents();
}
QString literal(const QString& value) {
    const auto json = QJsonDocument(QJsonArray{value}).toJson(QJsonDocument::Compact);
    return QString::fromUtf8(json.mid(1, json.size() - 2));
}
QVariantMap artifact(DesktopBackend& backend, const QString& id) {
    for (const auto& value : backend.artifacts())
        if (value.toMap().value("id").toString() == id)
            return value.toMap();
    throw std::runtime_error("artifact missing");
}
void capture(QQuickWindow& window, const QString& path) {
    window.requestUpdate();
    settle(220);
    require(window.grabWindow().save(path), "actual workflow screenshot");
}
void insideWindow(QQuickWindow& window, QObject& root, const char* name) {
    auto* item = qobject_cast<QQuickItem*>(&child(root, name));
    require(item && item->isVisible(), "visible narrow layout action");
    const auto bounds = item->mapRectToScene(item->boundingRect());
    require(
        bounds.left() >= 0 && bounds.top() >= 0 && bounds.right() <= window.width()
            && bounds.bottom() <= window.height(),
        "narrow layout action stays inside window"
    );
}
void insideDialog(QObject& dialog, const char* name) {
    auto* item = qobject_cast<QQuickItem*>(&child(dialog, name));
    require(item, "dialog action exists");
    const auto bounds = item->mapRectToScene(item->boundingRect());
    const QRectF panel(
        dialog.property("x").toReal(),
        dialog.property("y").toReal(),
        dialog.property("width").toReal(),
        dialog.property("height").toReal()
    );
    require(
        bounds.top() >= panel.top() + 8 && bounds.bottom() <= panel.bottom() - 8,
        "footer remains inside the dialog panel with vertical breathing room"
    );
}
void scrollToTools(QObject& entry, QQuickWindow& window, const QString& path) {
    auto& scroll = child(entry, "createTaskScroll");
    auto* flick = qvariant_cast<QQuickItem*>(scroll.property("contentItem"));
    require(
        flick && flick->property("contentHeight").toReal() > flick->height(),
        "narrow creation list is scrollable"
    );
    flick->setProperty("contentY", flick->property("contentHeight").toReal() - flick->height());
    settle();
    const auto viewport = flick->mapRectToScene(flick->boundingRect());
    for (const auto* name : {"importMaterialStartButton", "pasteMaterialStartButton"}) {
        auto* item = qobject_cast<QQuickItem*>(&child(entry, name));
        require(
            item && item->isVisible()
                && viewport.contains(item->mapRectToScene(item->boundingRect())),
            "import and paste actions are fully reachable by scrolling"
        );
    }
    capture(window, path);
    flick->setProperty("contentY", 0);
    settle();
}
QVariantMap card(const QVariantMap& composition, const QString& id) {
    for (const auto& value : composition.value("cards").toList())
        if (value.toMap().value("id").toString() == id)
            return value.toMap();
    throw std::runtime_error("card missing");
}
void pristineListIntake(QObject& workspace) {
    const auto original = workspace.property("composition").toMap();
    const auto starter = original.value("cards").toList().first().toMap();
    click(workspace, "cardKitAddFromList");
    auto& editor = child(workspace, "cardKitListText");
    QStringList rows;
    for (int index = 0; index < 64; ++index)
        rows.push_back(QStringLiteral("Card %1\tA useful effect\tA moonlit scene").arg(index));
    editor.setProperty("text", rows.join('\n'));
    QCoreApplication::processEvents();
    require(
        workspace.property("listPreview").toMap().value("replacesPlaceholder").toBool()
            && workspace.property("listPreview").toMap().value("rows").toList().size() == 64
            && child(workspace, "cardKitListApply").property("enabled").toBool(),
        "empty starter replacement shares the exact 64-card limit with its preview"
    );
    for (const auto* field :
         {"title",
          "card_type",
          "cost",
          "rules",
          "stats",
          "flavor",
          "art_prompt",
          "quantity",
          "artwork",
          "text_material",
          "extra_card"}) {
        auto changed = original, authored = starter;
        if (QString(field) == "quantity")
            authored[field] = 2;
        else if (QString(field) == "artwork" || QString(field) == "text_material")
            authored[field] = QVariantMap{{"artifact_id", "source"}, {"revision_id", "revision"}};
        else
            authored[field] = " ";
        changed["cards"] = QString(field) == "extra_card" ? QVariantList{starter, starter}
                                                          : QVariantList{authored};
        workspace.setProperty("composition", changed);
        QCoreApplication::processEvents();
        require(
            workspace.property("listPreview").toMap().value("error") == "too_many_cards",
            "any authored field, material, quantity or extra card is preserved"
        );
    }
    workspace.setProperty("composition", original);
    click(workspace, "cardKitListCancel");
    require(
        workspace.property("composition").toMap() == original,
        "cancel retains the original empty starter identity"
    );
    click(workspace, "cardKitAddFromList");
    editor.setProperty(
        "text",
        "First card\tProtect a friend\tSilver fox\nSecond card\tDraw a card\tGolden lantern"
    );
    QCoreApplication::processEvents();
    click(workspace, "cardKitListApply");
    const auto imported = workspace.property("composition").toMap().value("cards").toList();
    require(
        imported.size() == 2 && imported[0].toMap().value("title") == "First card"
            && imported[1].toMap().value("title") == "Second card"
            && imported[0].toMap().value("id") != starter.value("id"),
        "first list action replaces only the untouched starter without an empty export blocker"
    );
    evaluate(
        workspace,
        "changed("
            + QString::fromUtf8(QJsonDocument::fromVariant(original).toJson(QJsonDocument::Compact))
            + "); flush()"
    );
}
void cardListIntake(QObject& workspace) {
    const auto original = workspace.property("composition").toMap();
    const auto first = workspace.property("selectedCardId").toString();
    click(workspace, "cardKitAddFromList");
    auto& editor = child(workspace, "cardKitListText");
    editor.setProperty(
        "text",
        "Title\tRules\tArtwork description\nMoon Fox\tSecond effect\tSilver fox\n"
        "Lantern Friend\t\"Protect a friend.\nDraw a card.\"\tGolden lantern"
    );
    QCoreApplication::processEvents();
    auto preview = workspace.property("listPreview").toMap();
    require(
        preview.value("rows").toList().size() == 2 && preview.value("headerSkipped").toBool()
            && preview.value("duplicateCount").toInt() == 1,
        "table preview handles headings, repeated titles and quoted multiline cells"
    );
    click(workspace, "cardKitListCancel");
    require(
        workspace.property("composition").toMap() == original,
        "cancel leaves existing cards intact"
    );
    click(workspace, "cardKitAddFromList");
    editor.setProperty(
        "text",
        "Moon Fox\tSecond effect\tSilver fox\nLantern Friend\tProtect a friend.\tGolden lantern"
    );
    QCoreApplication::processEvents();
    click(workspace, "cardKitListApply");
    const auto added = workspace.property("composition").toMap().value("cards").toList();
    require(
        added.size() == 3 && added[0].toMap() == original.value("cards").toList()[0].toMap()
            && added[1].toMap().value("id") != added[2].toMap().value("id"),
        "explicit table intake appends unique card identities without replacing existing data"
    );
    evaluate(workspace, "appendCardList()");
    require(
        workspace.property("composition").toMap().value("cards").toList().size() == 3,
        "repeated apply cannot append the same preview twice"
    );
    for (int index = 1; index < added.size(); ++index) {
        evaluate(
            workspace,
            "selectedCardId = " + literal(added[index].toMap().value("id").toString())
                + "; removeCard()"
        );
    }
    evaluate(workspace, "flush()");
    require(
        workspace.property("composition").toMap() == original,
        "card removal preserves the original imported identity"
    );
    click(workspace, "cardKitAddFromList");
    editor.setProperty("text", "body\nhead\narm");
    QCoreApplication::processEvents();
    require(
        workspace.property("listPreview").toMap().value("rows").toList().size() == 3
            && !workspace.property("listPreview").toMap().value("headerSkipped").toBool(),
        "single-column learning words never become an implicit heading"
    );
    editor.setProperty("text", "text\nA body-only card\nAnother body-only card");
    workspace.setProperty("listFirstHeader", true);
    QCoreApplication::processEvents();
    const auto body_rows = workspace.property("listPreview").toMap().value("rows").toList();
    require(
        body_rows.size() == 2 && body_rows.first().toMap().value("title").toString().isEmpty()
            && body_rows.first().toMap().value("rules") == "A body-only card",
        "explicit body heading maps title-free text without inventing printed titles"
    );
    workspace.setProperty("listFirstHeader", false);
    for (const auto& pair : QList<QPair<QString, QString>>{
             {"One\tTwo\tThree\tFour", "columns"},
             {"\t\tOnly an image prompt", "content"},
             {"One\t\"Unclosed", "quote"},
             {QString(90, QChar(0x6708)) + "\tRules", "field_length"}
         }) {
        editor.setProperty("text", pair.first);
        QCoreApplication::processEvents();
        require(
            workspace.property("listPreview").toMap().value("error") == pair.second
                && !child(workspace, "cardKitListApply").property("enabled").toBool(),
            "invalid table rows cannot be added"
        );
    }
    QStringList boundary;
    for (int index = 0; index < 63; ++index)
        boundary.push_back(QStringLiteral("Card %1").arg(index));
    editor.setProperty("text", boundary.join('\n'));
    QCoreApplication::processEvents();
    require(
        workspace.property("listPreview").toMap().value("rows").toList().size() == 63
            && child(workspace, "cardKitListApply").property("enabled").toBool(),
        "64-card total boundary is admitted"
    );
    boundary.push_back("One too many");
    editor.setProperty("text", boundary.join('\n'));
    QCoreApplication::processEvents();
    require(
        workspace.property("listPreview").toMap().value("error") == "too_many_cards"
            && !child(workspace, "cardKitListApply").property("enabled").toBool(),
        "65-card total is rejected before mutation"
    );
    click(workspace, "cardKitListCancel");
    require(
        workspace.property("composition").toMap() == original
            && workspace.property("selectedCardId").toString() == first,
        "invalid and cancelled intake retains the selected original card"
    );
}
void wideGlyphLayout(
    QObject& workspace,
    CardKitController& kit,
    QQuickWindow& window,
    const QString& directory,
    const QString& id
) {
    const auto original = card(workspace.property("composition").toMap(), id);
    evaluate(
        workspace,
        "setCardField('title', " + literal(QString(25, QChar('W'))) + "); setCardField('rules', "
            + literal(
                QString(40, QChar('W'))
                + QString::fromUtf8("\n月光与 Lantern companions protect the garden.")
            )
            + "); flush(); controller.review(selectedCardId)"
    );
    require(
        kit.preview().value("ready_to_export").toBool()
            && imageReady(workspace, "cardKitPreviewImage"),
        "wide glyph and mixed-language SVG is decoded and fits the real card frame"
    );
    QFile svg(directory + "-wide-glyphs.svg");
    require(
        svg.open(QIODevice::WriteOnly)
            && svg.write(kit.preview().value("svg").toString().toUtf8()) > 0,
        "save exact production wide-glyph SVG for native full-size inspection"
    );
    capture(window, directory + "-wide-glyphs.png");
    evaluate(
        workspace,
        "setCardField('title', " + literal(QString(60, QChar('W')))
            + "); flush(); controller.review(selectedCardId)"
    );
    require(
        !kit.preview().value("ready_to_export").toBool()
            && kit.preview().value("overflow_card_ids").toList().contains(id)
            && !child(workspace, "cardKitExport").property("enabled").toBool(),
        "overflowing wide-glyph title is reported and cannot be exported"
    );
    evaluate(
        workspace,
        "setCardField('title', " + literal(original.value("title").toString())
            + "); setCardField('rules', " + literal(original.value("rules").toString())
            + "); flush()"
    );
}
} // namespace

bool verify(
    DesktopBackend& backend,
    CardKitController& kit,
    UiPreferences& preferences,
    QObject& root,
    const QString& directory,
    const QString& settingsPath
) {
    if (qEnvironmentVariableIsSet("SHAPE_SMOKE_CARD_FLEX_ONLY"))
        return card_flex_smoke::verify(backend, kit, preferences, root, directory + "-flex");
    try {
        require(QDir().mkpath(directory), "create evidence directory");
        require(
            backend.createProject(QUrl::fromLocalFile(directory), "Moon garden cards"),
            "create project"
        );
        preferences.setLanguageMode("en");
        auto* window = qobject_cast<QQuickWindow*>(&root);
        require(window, "real desktop window");
        auto& entry = child(root, "createSceneTypeDialog");
        evaluate(entry, "openForCreation()");
        settle(220);
        require(
            kit.templateKeys().contains("card_kit") && kit.templateKeys().contains("picture_book")
                && kit.templateKeys().contains("material_edit"),
            "production template catalog reaches entry"
        );
        capture(*window, directory + "-templates-en.png");
        const auto entry_wide_size = window->size();
        window->resize(1120, 720);
        settle();
        capture(*window, directory + "-templates-en-narrow.png");
        scrollToTools(entry, *window, directory + "-tools-en-narrow.png");
        preferences.setLanguageMode("zh_CN");
        settle();
        capture(*window, directory + "-templates-zh-narrow.png");
        scrollToTools(entry, *window, directory + "-tools-zh-narrow.png");
        preferences.setLanguageMode("en");
        window->resize(entry_wide_size);
        settle();
        pointerClick(*window, child(entry, "createCardKitButton"));
        auto& deck_entry = child(root, "deckHandoffDialog");
        require(
            deck_entry.property("visible").toBool() && kit.draft().isEmpty(),
            "new cards open Deck without creating a Shape card set"
        );
        QMetaObject::invokeMethod(&deck_entry, "close");
        auto& workspace = child(root, "cardKitWorkspace");
        QMetaObject::invokeMethod(
            &workspace,
            "openWorkspace",
            Q_ARG(QVariant, QVariant(QString()))
        );
        require(
            workspace.property("visible").toBool() && !kit.draft().isEmpty(),
            "explicit legacy open preserves the Shape card workspace"
        );
        const auto first = workspace.property("selectedCardId").toString();
        require(!first.isEmpty(), "stable first card identity");
        require(
            workspace.property("composition").toMap().value("layout").toMap().value("mode")
                == "auto",
            "new card sets explicitly use flexible automatic layout"
        );
        pristineListIntake(workspace);
        type(*window, child(workspace, "cardKitTitle"), "Moon Fox");
        type(
            *window,
            child(workspace, "cardKitRules"),
            "Draw a card.\nProtect the moon garden.",
            true
        );
        click(workspace, "cardKitSetLayout");
        evaluate(child(workspace, "cardKitSetLayoutDialog"), "promptsExpanded = true");
        type(
            *window,
            child(workspace, "cardKitBrief"),
            "A gentle woodland card game for families.",
            true
        );
        type(
            *window,
            child(workspace, "cardKitStyle"),
            "Midnight blue, soft gold, paper illustration.",
            true
        );
        require(
            card(workspace.property("composition").toMap(), first).value("title") == "Moon Fox"
                && workspace.property("dirty").toBool(),
            "real key input reaches authored card and draft"
        );
        click(workspace, "cardKitSetLayoutClose");
        require(kit.source().isEmpty(), "draft creation does not accept an output");
        cardListIntake(workspace);
        evaluate(
            workspace,
            "{ const legacy = copy(composition); delete legacy.layout; changed(legacy); flush() }"
        );

        const auto exact =
            backend.beginAiImageDraft("Identity fixture", "A quiet moonlit landscape", 1536, 1024);
        require(
            !exact.value("id").toString().isEmpty()
                && !exact.value("contextArtifactId").toString().isEmpty()
                && exact.value("operatorTypeKey") == "image.generate"
                && artifact(backend, exact.value("contextArtifactId").toString()).value("kindKey")
                       == "image_raster",
            "image creation returns an exact draft and target identity without snapshot ordering"
        );
        require(
            backend.discardOperatorDraft(exact.value("id").toString()),
            "discard unexecuted identity fixture"
        );

        QImage original(640, 420, QImage::Format_RGBA8888);
        {
            QPainter painter(&original);
            QLinearGradient gradient(0, 0, 640, 420);
            gradient.setColorAt(0, QColor("#152f4b"));
            gradient.setColorAt(1, QColor("#48737a"));
            painter.fillRect(original.rect(), gradient);
            painter.setBrush(QColor("#f2d58e"));
            painter.setPen(Qt::NoPen);
            painter.drawEllipse(435, 45, 110, 110);
            painter.setBrush(QColor("#173d3b"));
            painter.drawEllipse(-120, 240, 820, 300);
            painter.setBrush(QColor("#b8ba8b"));
            painter.drawEllipse(252, 215, 120, 105);
        }
        const auto original_file = directory + "/original.png";
        require(original.save(original_file), "write artwork fixture");
        const auto import_kit = kit.draft().value("artifact_id").toString();
        require(kit.beginNew(), "second kit reuses the default card-1 identity");
        const auto before_foreign_import = backend.artifactCount();
        evaluate(
            workspace,
            "importArtwork(" + literal(QUrl::fromLocalFile(original_file).toString()) + ","
                + literal(first) + "," + literal(backend.projectId()) + "," + literal(import_kit)
                + "," + literal(backend.bundlePath()) + ")"
        );
        require(
            backend.artifactCount() == before_foreign_import
                && card(workspace.property("composition").toMap(), first)
                       .value("artwork")
                       .toMap()
                       .isEmpty(),
            "late import from another kit with the same card id creates no orphan and changes no "
            "card"
        );
        require(kit.load(import_kit), "restore original import destination kit");
        evaluate(
            workspace,
            "importArtwork(" + literal(QUrl::fromLocalFile(original_file).toString()) + ","
                + literal(first) + "," + literal(backend.projectId()) + "," + literal(import_kit)
                + "," + literal(backend.bundlePath()) + ")"
        );
        const auto original_id = card(workspace.property("composition").toMap(), first)
                                     .value("artwork")
                                     .toMap()
                                     .value("artifact_id")
                                     .toString();
        const auto original_artifact = artifact(backend, original_id);
        const auto original_revision = original_artifact.value("acceptedRevisionId").toString();
        const auto before_stale_import = backend.artifactCount();
        evaluate(
            workspace,
            "importArtwork(" + literal(QUrl::fromLocalFile(original_file).toString()) + ","
                + literal(first) + ",'another-project'," + literal(import_kit) + ","
                + literal(backend.bundlePath()) + ")"
        );
        require(
            backend.artifactCount() == before_stale_import,
            "late import from another project cannot create or bind material"
        );
        click(workspace, "cardKitAddCard");
        const auto second = workspace.property("selectedCardId").toString();
        type(*window, child(workspace, "cardKitTitle"), QString::fromUtf8("灯笼守卫"));
        type(*window, child(workspace, "cardKitRules"), QString::fromUtf8("守护一名同伴。"), true);
        evaluate(workspace, "chooseArtwork(1)");
        evaluate(workspace, "selectedCardId = " + literal(first));
        require(kit.save(workspace.property("composition").toMap()), "checkpoint card authoring");
        click(workspace, "cardKitRefineArtwork");
        const auto stage = workspace.property("editingStage").toMap();
        const auto output = stage.value("contextArtifactId").toString();
        require(
            !output.isEmpty() && output != original_id,
            "refinement reserves a separate output"
        );
        evaluate(workspace, "moveCard(1); flush()");
        require(
            workspace.property("composition")
                    .toMap()
                    .value("cards")
                    .toList()
                    .last()
                    .toMap()
                    .value("id")
                == first,
            "card reorder retains stable identity during refinement"
        );
        {
            DesktopBackend pending_backend(
                shape::desktop::open_desktop_session(backend.bundlePath().toStdString())
            );
            CardKitController pending(pending_backend, settingsPath);
            require(
                pending.load(kit.draft().value("artifact_id").toString())
                    && pending.recoverArtworkEdit(first).value("stage").toMap().value(
                           "contextArtifactId"
                       ) == output,
                "fresh backend and controller recover the exact local card return after reordering"
            );
            const auto recovered = pending.recoverArtworkEdit(first);
            const auto clone_path = directory + "/cloned-cards.shape";
            std::filesystem::copy(
                backend.bundlePath().toStdString(),
                clone_path.toStdString(),
                std::filesystem::copy_options::recursive
            );
            DesktopBackend clone_backend(
                shape::desktop::open_desktop_session(clone_path.toStdString())
            );
            CardKitController clone(clone_backend, settingsPath);
            require(
                clone.load(kit.draft().value("artifact_id").toString())
                    && clone.rememberArtworkEdit(
                        first,
                        recovered.value("source").toMap(),
                        recovered.value("stage").toMap()
                    )
                    && !clone.recoverArtworkEdit(first).isEmpty()
                    && !pending.recoverArtworkEdit(first).isEmpty(),
                "copied bundles sharing all ids retain independent local artwork return locations"
            );
            QSettings migration(settingsPath, QSettings::IniFormat);
            QString old_record;
            for (const auto& key : migration.allKeys()) {
                if (!key.startsWith("card-kit/artwork-edits-by-bundle/"))
                    continue;
                const auto record = migration.value(key).toString();
                if (QJsonDocument::fromJson(record.toUtf8()).object()["project_path"].toString()
                    == backend.bundlePath()) {
                    old_record = record;
                    migration.remove(key);
                    break;
                }
            }
            require(
                !old_record.isEmpty(),
                "legacy migration fixture uses a real persisted stage record"
            );
            const auto legacy_key = QStringLiteral("card-kit/artwork-edits/%1/%2/%3")
                                        .arg(
                                            backend.projectId(),
                                            kit.draft().value("artifact_id").toString(),
                                            QString::fromLatin1(first.toUtf8().toHex())
                                        );
            migration.setValue(legacy_key, old_record);
            migration.sync();
            require(
                !pending.recoverArtworkEdit(first).isEmpty()
                    && !clone.recoverArtworkEdit(first).isEmpty(),
                "matching legacy return migrates without disturbing the copied bundle"
            );
            auto changed = pending.draft().value("composition").toMap();
            auto rows = changed.value("cards").toList();
            auto replacement = rows.last().toMap();
            replacement["artwork"] = QVariant();
            rows[rows.size() - 1] = replacement;
            changed["cards"] = rows;
            require(
                pending.save(changed) && pending.recoverArtworkEdit(first).isEmpty(),
                "changed card source cannot resume an old external return"
            );
            require(
                pending.save(workspace.property("composition").toMap()),
                "restore identical pending card after recovery boundary probe"
            );
        }
        require(
            kit.load(kit.draft().value("artifact_id").toString()),
            "reload current draft after external session checkpoint"
        );
        const auto count_before_resume = backend.operatorDrafts().size();
        evaluate(workspace, "clearArtwork(); resumeArtworkEdit()");
        require(
            workspace.property("editingStage").toMap().value("contextArtifactId") == output
                && workspace.property("artworkCardId").toString() == first
                && backend.operatorDrafts().size() == count_before_resume,
            "QML reopens the same stable card stage without generating another working copy"
        );
        const auto handoff_dir = directory + "/handoff";
        const auto ticket = backend.materialEditTicket(output).toStdString();
        (void)shape::desktop::export_material_copy(
            backend.bundlePath().toStdString(),
            ticket,
            handoff_dir.toStdString()
        );
        QImage returned = original.copy();
        returned.setPixelColor(300, 240, QColor("#ffbb66"));
        const auto returned_file = directory + "/returned.png";
        require(returned.save(returned_file), "write reviewed return fixture");
        auto candidate = shape::desktop::prepare_material_return(
            backend.bundlePath().toStdString(),
            ticket,
            (handoff_dir + "/handoff.json").toStdString(),
            returned_file.toStdString(),
            false
        );
        const auto candidate_id = backend.adoptMaterialReturn(std::move(candidate));
        require(!candidate_id.isEmpty(), "returned artwork remains a candidate");
        evaluate(workspace, "artworkCandidateId = " + literal(candidate_id));
        workspace.setProperty("artworkProject", "another-project");
        evaluate(workspace, "useArtwork()");
        require(
            !artifact(backend, output).value("hasAcceptedRevision").toBool(),
            "stale return cannot accept or update a card"
        );
        workspace.setProperty("artworkProject", backend.projectId());
        evaluate(workspace, "useArtwork()");
        auto authored = workspace.property("composition").toMap();
        require(
            card(authored, first).value("artwork").toMap().value("artifact_id") == output
                && card(authored, second).value("artwork").toMap().value("artifact_id")
                       == original_id
                && artifact(backend, original_id).value("acceptedRevisionId") == original_revision,
            "explicit return updates only the original stable card and preserves its source"
        );

        wideGlyphLayout(workspace, kit, *window, directory, first);
        click(workspace, "cardKitReview");
        require(
            kit.hasReview() && kit.preview().value("ready_to_export").toBool()
                && !kit.previewSource().isEmpty() && imageReady(workspace, "cardKitPreviewImage"),
            "production SVG review is ready for all complete cards"
        );
        capture(*window, directory + "-cards-en.png");
        click(workspace, "cardKitEnlargePreview");
        require(
            child(workspace, "cardKitLargePreview").property("visible").toBool()
                && imageReady(workspace, "cardKitLargeImage")
                && child(workspace, "cardKitLargeImage").property("width").toInt() == 750,
            "actual-size preview decodes the same SVG at its full canvas width"
        );
        capture(*window, directory + "-large-preview-en.png");
        click(workspace, "cardKitPreviewFit");
        require(
            !child(workspace, "cardKitLargePreview").property("actualSize").toBool(),
            "large preview can fit the whole card"
        );
        click(workspace, "cardKitPreviewClose");
        const auto before_language = workspace.property("composition").toMap();
        const auto wide_size = window->size();
        window->resize(1120, 720);
        settle();
        for (const auto* name :
             {"cardKitClose", "cardKitReview", "cardKitAccept", "cardKitPrint", "cardKitExport"}) {
            insideWindow(*window, workspace, name);
            insideDialog(workspace, name);
        }
        capture(*window, directory + "-cards-en-narrow.png");
        preferences.setLanguageMode("zh_CN");
        settle();
        require(
            child(workspace, "cardKitReview").property("text").toString()
                    == QString::fromUtf8("检查卡牌")
                && workspace.property("composition").toMap() == before_language,
            "live Chinese UI preserves authored text and stable card data"
        );
        for (const auto* name :
             {"cardKitClose", "cardKitReview", "cardKitAccept", "cardKitPrint", "cardKitExport"}) {
            insideWindow(*window, workspace, name);
            insideDialog(workspace, name);
        }
        auto* list = qobject_cast<QQuickItem*>(&child(workspace, "cardKitCardList"));
        auto* selected =
            qobject_cast<QQuickItem*>(&child(workspace, qPrintable("cardKitRow-" + first)));
        require(
            list && selected
                && list->mapRectToScene(list->boundingRect())
                       .contains(selected->mapRectToScene(selected->boundingRect())),
            "selected last card remains fully visible after reorder, recovery and narrowing the "
            "window"
        );
        capture(*window, directory + "-cards-zh-narrow.png");
        window->resize(wide_size);
        capture(*window, directory + "-cards-zh.png");
        click(workspace, "cardKitAccept");
        require(
            !kit.source().isEmpty() && kit.canExport(),
            "explicit review acceptance enables accepted export"
        );
        const auto zip = directory + "/accepted-cards.zip";
        require(kit.exportFiles(QUrl::fromLocalFile(zip)), "export inspected accepted card kit");
        QFile archive(zip);
        require(archive.open(QIODevice::ReadOnly) && archive.read(2) == "PK", "real ZIP output");
        const auto accepted_source = kit.source();
        const auto accepted_id = accepted_source.value("artifact_id").toString();
        click(workspace, qPrintable("cardKitRow-" + second));
        require(
            kit.preview().value("selected_card_id").toString() == second && !kit.hasReview()
                && kit.source() == accepted_source && imageReady(workspace, "cardKitPreviewImage"),
            "accepted card selection refreshes the exact displayed SVG without re-review or "
            "accepting"
        );
        click(workspace, qPrintable("cardKitRow-" + first));
        const auto project_path = backend.bundlePath();
        click(workspace, "cardKitClose");
        auto& surface = child(root, "workspaceSurface");
        int accepted_index = -1;
        for (int index = 0; index < backend.artifacts().size(); ++index)
            if (backend.artifacts()[index].toMap().value("id").toString() == accepted_id)
                accepted_index = index;
        require(accepted_index >= 0, "accepted kit in project navigation");
        evaluate(surface, QStringLiteral("activateScene(%1)").arg(accepted_index));
        settle();
        evaluate(surface, "openNode(" + literal("output." + accepted_id) + ")");
        require(
            workspace.property("visible").toBool()
                && workspace.property("composition").toMap() == authored,
            "graph output opens the concrete card workspace and restores exact authoring"
        );
        DesktopBackend reopened(shape::desktop::open_desktop_session(project_path.toStdString()));
        CardKitController restored(reopened);
        require(
            restored.load(accepted_id) && restored.draft().value("composition").toMap() == authored
                && restored.source() == accepted_source,
            "new session restores the persisted kit and accepted version"
        );
        require(
            kit.beginNew() && kit.save(authored) && kit.review(first) && kit.accept(),
            "accept a second printable kit for delayed export regression"
        );
        const auto wrong_zip = directory + "/wrong-kit.zip";
        evaluate(
            workspace,
            "exportKitFiles(" + literal(QUrl::fromLocalFile(wrong_zip).toString()) + ","
                + literal(backend.projectId()) + "," + literal(backend.bundlePath()) + ","
                + QString::fromUtf8(
                    QJsonDocument::fromVariant(accepted_source).toJson(QJsonDocument::Compact)
                )
                + ")"
        );
        require(
            !QFileInfo::exists(wrong_zip),
            "old ZIP callback cannot export another accepted kit"
        );
        require(kit.load(accepted_id), "return to original kit after delayed export probe");
        auto concurrent = authored;
        concurrent["brief"] = QStringLiteral("A collaborator updated this brief.");
        require(restored.save(concurrent), "parallel session saves a newer draft");
        evaluate(workspace, "setCardField('title', 'Locally unsaved title'); flush()");
        require(
            workspace.property("dirty").toBool() && !kit.error().isEmpty()
                && card(workspace.property("composition").toMap(), first).value("title")
                       == "Locally unsaved title",
            "stale save preserves exact local text for recovery"
        );
        click(workspace, "cardKitCopyDraft");
        require(
            QGuiApplication::clipboard()->text().contains("Locally unsaved title"),
            "complete conflict draft is copyable before explicit reload"
        );
        click(workspace, "cardKitReloadDraft");
        require(
            workspace.property("composition").toMap() == concurrent
                && !workspace.property("dirty").toBool() && kit.source() == accepted_source,
            "explicit reload adopts the saved draft without changing accepted history"
        );
        authored = concurrent;
        const auto second_project = directory + "/other";
        require(
            QDir().mkpath(second_project)
                && backend.createProject(QUrl::fromLocalFile(second_project), "Other project"),
            "switch project"
        );
        settle();
        require(
            !workspace.property("visible").toBool() && kit.draft().isEmpty(),
            "project switch clears scoped card state"
        );
        require(backend.openProject(QUrl::fromLocalFile(project_path)), "reopen original project");
        evaluate(workspace, "openWorkspace(" + literal(accepted_id) + ")");
        require(
            workspace.property("composition").toMap() == authored,
            "original card materials remain recoverable"
        );
        preferences.setLanguageMode("en");
        require(
            card_flex_smoke::verify(backend, kit, preferences, root, directory + "-flex"),
            "flexible-card native consumer"
        );
        std::cout << "card template pointer entry, typing, persisted draft, reorder-safe return, "
                     "review, acceptance, ZIP, language and project reopen passed"
                  << std::endl;
        return true;
    } catch (const std::exception& error) {
        std::cerr << "workflow smoke: " << error.what() << std::endl;
        return false;
    }
}

bool capturePreview(
    CardKitController& kit,
    UiPreferences& preferences,
    QObject& root,
    const QString& prefix
) {
    try {
        auto* window = qobject_cast<QQuickWindow*>(&root);
        require(window, "preview window");
        auto& workspace = child(root, "cardKitWorkspace");
        require(workspace.property("visible").toBool(), "preview card workspace");
        const auto cards = workspace.property("composition").toMap().value("cards").toList();
        require(!cards.isEmpty(), "preview cards");
        const auto wide_size = window->size();
        preferences.setLanguageMode("en");
        for (int index = 0; index < cards.size(); ++index) {
            const auto id = cards[index].toMap().value("id").toString();
            evaluate(workspace, "selectedCardId = " + literal(id));
            require(
                kit.review(id) && imageReady(workspace, "cardKitPreviewImage"),
                "real accepted artwork preview decoded"
            );
            capture(*window, prefix + QStringLiteral("-card-%1-en.png").arg(index + 1));
        }
        const auto id = cards.first().toMap().value("id").toString();
        evaluate(workspace, "selectedCardId = " + literal(id));
        require(kit.review(id), "first card preview");
        window->resize(1120, 720);
        settle();
        for (const auto* name :
             {"cardKitClose", "cardKitReview", "cardKitAccept", "cardKitPrint", "cardKitExport"}) {
            insideWindow(*window, workspace, name);
            insideDialog(workspace, name);
        }
        capture(*window, prefix + "-cards-en-narrow.png");
        preferences.setLanguageMode("zh_CN");
        settle();
        capture(*window, prefix + "-cards-zh-narrow.png");
        window->resize(wide_size);
        capture(*window, prefix + "-cards-zh.png");
        std::cout << "real card-kit previews captured; accepted history unchanged" << std::endl;
        return true;
    } catch (const std::exception& error) {
        std::cerr << "preview capture: " << error.what() << std::endl;
        return false;
    }
}
} // namespace workflow_smoke

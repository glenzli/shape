#include "picture_book_smoke.hpp"
#include "desktop_backend.hpp"
#include "infer_image_controller.hpp"
#include "material_edit_controller.hpp"
#include "picture_book_controller.hpp"
#include "picture_book_process_smoke.hpp"
#include "ui_preferences.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QScopeGuard>
#include <QStandardPaths>
#include <QTimer>
#include <filesystem>
#include <functional>
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
void pointerClick(QQuickWindow& window, QObject& object) {
    auto* item = qobject_cast<QQuickItem*>(&object);
    require(item && item->isVisible() && item->isEnabled(), "book page action visible to pointer");
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
    QEventLoop loop;
    QTimer::singleShot(200, &loop, &QEventLoop::quit);
    loop.exec();
}
bool waitUntil(const std::function<bool()>& done, int maximum = 45000) {
    QElapsedTimer timer;
    timer.start();
    while (!done() && timer.elapsed() < maximum) {
        QEventLoop loop;
        QTimer::singleShot(10, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return done();
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
void pageArtworkContinuation(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& workspace,
    const QImage& image,
    const QString& directory,
    QJsonObject& request
) {
    evaluate(workspace, QStringLiteral("choosePage(2)"));
    auto* editor = workspace.findChild<QObject*>("pictureBookScript");
    require(editor, "page return script editor");
    evaluate(*editor, QStringLiteral("insert(length, '\\n单页调色时保留这段草稿。')"));
    const auto script = workspace.property("script").toString();
    const auto before = QJsonObject::fromVariantMap(workspace.property("draft").toMap());
    const auto saved_book = book.source();
    click(workspace, "editPictureBookArtwork");
    const auto stage = workspace.property("artworkEdit").toMap();
    const auto output = stage.value("contextArtifactId").toString();
    require(
        workspace.property("editingArtwork").toBool() && !output.isEmpty(),
        "current page opens its own derived material stage inside book"
    );
    const auto ticket = backend.materialEditTicket(output).toStdString();
    (void)shape::desktop::export_material_copy(
        backend.bundlePath().toStdString(),
        ticket,
        directory.toStdString()
    );
    const auto returned = directory + "/returned.png";
    require(image.save(returned), "page artwork synthetic return");
    auto prepared = shape::desktop::prepare_material_return(
        backend.bundlePath().toStdString(),
        ticket,
        (directory + "/handoff.json").toStdString(),
        returned.toStdString(),
        false
    );
    const auto candidate = backend.adoptMaterialReturn(std::move(prepared));
    QCoreApplication::processEvents();
    require(
        !candidate.isEmpty() && artifactPin(backend, output)["revision_id"].toString().isEmpty()
            && workspace.property("draft").toMap() == before.toVariantMap()
            && book.source() == saved_book,
        "returned image is a candidate without accepting material or book"
    );
    require(
        !book.adoptArtwork(
                 before["pages"]
                     .toArray()[0]
                     .toObject()["source"]
                     .toObject()["artifact_id"]
                     .toString(),
                 candidate
        )
             .contains("artifact_id"),
        "another output cannot adopt the page candidate"
    );
    workspace.setProperty("artworkProject", "another-project");
    evaluate(workspace, QStringLiteral("adoptArtwork()"));
    require(
        artifactPin(backend, output)["revision_id"].toString().isEmpty(),
        "stale project return cannot accept material or change page"
    );
    workspace.setProperty("artworkProject", backend.projectId());
    auto displaced = before;
    auto displaced_pages = displaced["pages"].toArray();
    displaced_pages[2] = QJsonObject{{"kind", "blank"}};
    displaced["pages"] = displaced_pages;
    workspace.setProperty("draft", displaced.toVariantMap());
    evaluate(workspace, QStringLiteral("adoptArtwork()"));
    require(
        artifactPin(backend, output)["revision_id"].toString().isEmpty()
            && workspace.property("draft").toMap() == displaced.toVariantMap(),
        "changed page cannot be overwritten by late material return"
    );
    workspace.setProperty("draft", before.toVariantMap());
    click(workspace, "adoptPictureBookArtwork");
    const auto after = QJsonObject::fromVariantMap(workspace.property("draft").toMap());
    const auto pages = after["pages"].toArray();
    require(
        !workspace.property("editingArtwork").toBool()
            && pages[2].toObject()["source"].toObject() == artifactPin(backend, output)
            && pages[0] == before["pages"].toArray()[0] && after["languages"] == before["languages"]
            && book.source() == saved_book && !book.hasReview()
            && workspace.property("script").toString() == script
            && workspace.property("scriptDirty").toBool() && workspace.property("dirty").toBool()
            && workspace.property("activeLanguage").toString() == "zh-CN"
            && workspace.property("pageIndex").toInt() == 2,
        "explicit image adoption repins only current page and preserves pending language script"
    );
    evaluate(workspace, QStringLiteral("adoptArtwork()"));
    require(
        workspace.property("draft").toMap() == after.toVariantMap(),
        "repeated page adoption cannot append another image revision"
    );
    click(workspace, "adoptPictureBookScript");
    click(workspace, "reviewPictureBook");
    require(book.hasReview(), "page artwork requires a new book review");
    click(workspace, "acceptPictureBook");
    require(
        book.source() != saved_book && !workspace.property("dirty").toBool(),
        "book adoption is a distinct explicit durable action"
    );
    request = QJsonObject::fromVariantMap(workspace.property("draft").toMap());
}
void verifyDraftRecovery(DesktopBackend& backend, PictureBookController& book, QObject& workspace) {
    const auto accepted = book.source();
    const auto accepted_draft = workspace.property("draft").toMap();
    const auto original_script = workspace.property("script").toString();
    const auto count = backend.artifactCount();
    auto* editor = workspace.findChild<QObject*>("pictureBookScript");
    require(editor, "recoverable script editor");
    evaluate(
        *editor,
        QStringLiteral("insert(length, '\\n[Unknown]\\n未完成、未采用的恢复草稿。')")
    );
    evaluate(
        workspace,
        QStringLiteral("setField('name', 'Recover this unfinished book'); choosePage(2); flush()")
    );
    const auto typed = workspace.property("script").toString();
    const auto draft = book.draft();
    require(
        workspace.property("scriptDirty").toBool()
            && !workspace.property("checkpointDirty").toBool()
            && draft.value("state").toMap().value("script").toString() == typed
            && backend.artifactCount() == count && book.source() == accepted,
        "checkpoint persists unfinished syntax without adopting script or book"
    );
    require(
        backend.openProject(QUrl::fromLocalFile(backend.bundlePath()))
            && book.load(draft.value("artifact_id").toString()),
        "reopen durable book checkpoint"
    );
    require(
        workspace.property("script").toString() == typed
            && workspace.property("scriptDirty").toBool()
            && workspace.property("pageIndex").toInt() == 2
            && workspace.property("activeLanguage").toString() == "zh-CN"
            && workspace.property("draft").toMap().value("name").toString()
                   == "Recover this unfinished book"
            && book.source() == accepted,
        "reopened book restores page, language and exact unadopted script"
    );
    require(
        !book.review(workspace.property("draft").toMap()) && book.source() == accepted,
        "unfinished recovered script cannot be adopted through book review"
    );
    click(workspace, "discardPictureBookDraft");
    require(
        workspace.property("draft").toMap() == accepted_draft
            && workspace.property("script").toString() == original_script
            && !workspace.property("scriptDirty").toBool() && book.source() == accepted,
        "explicit discard restores accepted book after durable recovery"
    );
}
void verifyRefreshedStagePageFence(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& root,
    QObject& workspace,
    const QImage& image,
    const QString& directory
) {
    const auto accepted_book = book.source();
    evaluate(workspace, QStringLiteral("choosePage(2)"));
    const auto before = workspace.property("draft").toMap();
    const auto source =
        QJsonObject::fromVariantMap(before)["pages"].toArray()[2].toObject()["source"].toObject();
    click(workspace, "editPictureBookArtwork");
    const auto output =
        workspace.property("artworkEdit").toMap().value("contextArtifactId").toString();
    require(!output.isEmpty(), "source-refresh page stage");
    auto source_update = image.copy();
    source_update.setPixelColor(1, 1, QColor("#e1b78c"));
    const auto newer = acceptArtworkReturn(
        backend,
        source["artifact_id"].toString(),
        source_update,
        directory + "-source"
    );
    require(newer != source, "page source advanced in another material stage");
    auto* material = root.property("materialEdit").value<MaterialEditController*>();
    require(
        material && material->restart(backend.bundlePath(), output, true),
        "existing page stage explicitly refreshes its input"
    );
    const auto ticket = backend.materialEditTicket(output).toStdString();
    (void)shape::desktop::export_material_copy(
        backend.bundlePath().toStdString(),
        ticket,
        directory.toStdString()
    );
    const auto returned = directory + "/returned.png";
    auto returned_image = image.copy();
    returned_image.setPixelColor(0, 0, QColor("#bbaacc"));
    require(returned_image.save(returned), "refreshed-stage synthetic return");
    auto prepared = shape::desktop::prepare_material_return(
        backend.bundlePath().toStdString(),
        ticket,
        (directory + "/handoff.json").toStdString(),
        returned.toStdString(),
        false
    );
    require(
        !backend.adoptMaterialReturn(std::move(prepared)).isEmpty(),
        "refreshed stage creates its own reviewable candidate"
    );
    QCoreApplication::processEvents();
    evaluate(workspace, QStringLiteral("adoptArtwork()"));
    require(
        !workspace.property("artworkStageCurrent").toBool()
            && artifactPin(backend, output)["revision_id"].toString().isEmpty()
            && workspace.property("draft").toMap() == before && book.source() == accepted_book,
        "refreshed input candidate cannot accept into page bound to the previous source"
    );
    require(material->restart(backend.bundlePath(), output, false), "cancel refreshed return");
    click(workspace, "discardPictureBookDraft");
}
void verifyPageGeneration(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& root,
    QObject& workspace,
    QQuickWindow& window
) {
    auto* infer = root.property("inferImage").value<InferImageController*>();
    require(infer, "page generation shares existing image controller");
    const auto accepted = book.source();
    const auto pages = workspace.property("draft").toMap().value("pages");
    evaluate(workspace, QStringLiteral("choosePage(3)"));
    auto* script = workspace.findChild<QObject*>("pictureBookScript");
    require(script, "page generation script");
    evaluate(*script, QStringLiteral("insert(length, '\\n保留这段未采用的插画上下文。')"));
    const auto typed = workspace.property("script").toString();
    const QString lowercase_script = QStringLiteral(
        "[cover]\nCover\n[author]\nCredit\n[title]\nTitle\n[01]\nFirst body\n[2]\nSecond body"
    );
    workspace.setProperty("script", lowercase_script);
    workspace.setProperty("pageIndex", 2);
    QQmlExpression page_script(
        QQmlEngine::contextForObject(&workspace),
        &workspace,
        QStringLiteral("currentPageScript()")
    );
    require(
        page_script.evaluate().toString() == QStringLiteral("First body")
            && !page_script.hasError(),
        "page illustration context follows accepted case-insensitive role and numeric tags"
    );
    workspace.setProperty("script", typed);
    workspace.setProperty("pageIndex", 3);
    auto* button = workspace.findChild<QObject*>("generatePictureBookArtwork");
    require(button, "page illustration entry");
    pointerClick(window, *button);
    auto* dialog = workspace.findChild<QObject*>("pictureBookArtworkGenerationDialog");
    auto* prompt = workspace.findChild<QObject*>("pictureBookArtworkGenerationPrompt");
    require(
        dialog && dialog->property("visible").toBool() && prompt,
        "real pointer opens page generation workspace"
    );
    const auto description = prompt->property("text").toString();
    require(
        description.contains("保留这段未采用的插画上下文。") && description.contains('\n')
            && !description.contains(QStringLiteral("\\n"))
            && workspace.property("script").toString() == typed
            && workspace.property("scriptDirty").toBool(),
        "editable page prompt includes current unadopted stanza using actual newlines without "
        "altering script"
    );
    const auto book_id = book.draft().value("artifact_id").toString();
    require(
        workspace.property("generationBook").toString() == book_id
            && workspace.property("generationPage").toInt() == 3,
        "generation target pins exact book draft and page"
    );
    // The offline host supplies an empty isolated credential path. This exercises assembly
    // and the real controller's credential failure without any provider request or spend.
    evaluate(workspace, QStringLiteral("generateArtwork()"));
    require(
        waitUntil([&] { return !infer->running(); }, 10000),
        "offline generation failure returns control"
    );
    const auto output = workspace.property("generationArtifact").toString();
    const auto node = workspace.property("generationDraft").toString();
    require(
        !output.isEmpty() && !node.isEmpty()
            && artifactPin(backend, output)["revision_id"].toString().isEmpty(),
        "page generator uses exact returned unaccepted image node identity"
    );
    require(
        QMetaObject::invokeMethod(
            infer,
            "candidateCreated",
            Q_ARG(QString, QString("foreign-candidate")),
            Q_ARG(QString, QString("foreign-output"))
        ),
        "late foreign callback delivered to real connection"
    );
    require(
        workspace.property("generationCandidateId").toString().isEmpty(),
        "foreign output cannot select page candidate"
    );
    require(
        QMetaObject::invokeMethod(
            infer,
            "candidateCreated",
            Q_ARG(QString, QString("foreign-candidate")),
            Q_ARG(QString, output)
        ),
        "unknown candidate identity callback"
    );
    evaluate(workspace, QStringLiteral("adoptGeneratedArtwork()"));
    require(
        book.source() == accepted && workspace.property("draft").toMap().value("pages") == pages
            && artifactPin(backend, output)["revision_id"].toString().isEmpty()
            && workspace.property("script").toString() == typed,
        "unverified candidate callback cannot accept or overwrite the page or pending script"
    );
    evaluate(workspace, QStringLiteral("clearGeneration()"));
    click(workspace, "discardPictureBookDraft");
}
void verifyNativeLauncher(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& root,
    const QJsonObject& source,
    const QString& directory
) {
    const auto neutral = QStandardPaths::findExecutable(QStringLiteral("true"));
    if (!neutral.isEmpty()) {
        const auto unavailable_stage = book.beginArtworkEdit(source.toVariantMap());
        const auto unavailable_output = unavailable_stage.value("contextArtifactId").toString();
        MaterialEditController incomplete(backend, directory + "-incomplete.ini");
        require(
            incomplete.setShadowExecutable(QUrl::fromLocalFile(neutral)),
            "isolated executable selection"
        );
        incomplete.openShadow(backend.bundlePath(), unavailable_output);
        require(
            waitUntil([&] { return !incomplete.running() && !incomplete.shadowActive(); }, 10000)
                && incomplete.errorCode() == QStringLiteral("material_editor_incomplete"),
            "exited process without receipt releases waiting state and remains retryable"
        );
        require(
            artifactPin(backend, unavailable_output)["revision_id"].toString().isEmpty(),
            "process exit cannot accept image"
        );
        MaterialEditController reopened(backend, directory + "-incomplete.ini");
        require(
            !reopened.restoreShadowSession(backend.bundlePath(), unavailable_output)
                && !reopened.shadowActive(),
            "known incomplete process cannot restore into waiting after reopen"
        );
    }
    if (qEnvironmentVariable("SHAPE_SHADOW_BINARY").isEmpty())
        return;
    auto* controller = root.property("materialEdit").value<MaterialEditController*>();
    require(controller, "native Shadow controller reachable");
    const auto previous_action = qgetenv("SHADOW_PIPELINE_SMOKE_ACTION");
    const auto previous_profile = qgetenv("SHADOW_DESKTOP_DATA_ROOT");
    const auto previous_brush = qgetenv("SHADOW_PAINT_BRUSH_SETTINGS_FILE");
    const auto restore = qScopeGuard([&] {
        const auto reset = [](const char* key, const QByteArray& value) {
            if (value.isNull())
                qunsetenv(key);
            else
                qputenv(key, value);
        };
        reset("SHADOW_PIPELINE_SMOKE_ACTION", previous_action);
        reset("SHADOW_DESKTOP_DATA_ROOT", previous_profile);
        reset("SHADOW_PAINT_BRUSH_SETTINGS_FILE", previous_brush);
    });
    qputenv("SHADOW_PIPELINE_SMOKE_ACTION", "complete");
    qputenv("SHADOW_DESKTOP_DATA_ROOT", (directory + "-profile").toUtf8());
    qputenv("SHADOW_PAINT_BRUSH_SETTINGS_FILE", (directory + "-brush.ini").toUtf8());
    const auto wait = [](const std::function<bool()>& done) {
        QElapsedTimer timer;
        timer.start();
        while (!done() && timer.elapsed() < 45000) {
            QEventLoop loop;
            QTimer::singleShot(10, &loop, &QEventLoop::quit);
            loop.exec();
        }
        return done();
    };
    const auto stage = book.beginArtworkEdit(source.toVariantMap());
    const auto output = stage.value("contextArtifactId").toString();
    require(!output.isEmpty(), "native launcher source stage");
    controller->openShadow(backend.bundlePath(), output);
    require(
        wait([&] { return !controller->running() && !controller->shadowActive(); })
            && controller->errorCode().isEmpty(),
        "actual native Shadow completion reviewed"
    );
    QString candidate;
    for (const auto& value : backend.candidates())
        if (value.toMap().value("artifactId") == output)
            candidate = value.toMap().value("id").toString();
    require(
        !candidate.isEmpty() && artifactPin(backend, output)["revision_id"].toString().isEmpty(),
        "native launch and human-owner return create only candidate"
    );
    require(controller->restart(backend.bundlePath(), output, false), "native candidate cancelled");
    controller->openShadow(backend.bundlePath(), output);
    require(
        wait([&] {
            return controller->shadowActive()
                   || (!controller->running() && !controller->errorCode().isEmpty());
        }) && controller->shadowActive(),
        "second native Shadow window launched"
    );
    const auto handoff = controller->handoffPath();
    const auto result = QFileInfo(handoff).dir().filePath("shadow-result.json");
    require(
        controller->restart(backend.bundlePath(), output, false),
        "native waiting session cancelled"
    );
    require(
        wait([&] { return QFileInfo::exists(result); }),
        "late native producer completed after cancellation"
    );
    for (const auto& value : backend.candidates())
        require(value.toMap().value("artifactId") != output, "late native callback ignored");
    require(
        artifactPin(backend, output)["revision_id"].toString().isEmpty()
            && artifactPin(backend, source["artifact_id"].toString()) == source,
        "native cancellation preserves output and original heads"
    );
    const auto resume_stage = book.beginArtworkEdit(source.toVariantMap());
    const auto resume_output = resume_stage.value("contextArtifactId").toString();
    const auto resume_settings = directory + "-resume.ini";
    QString resumed_handoff;
    {
        MaterialEditController closing(backend, resume_settings);
        closing.openShadow(backend.bundlePath(), resume_output);
        require(
            waitUntil([&] {
                return closing.shadowActive()
                       || (!closing.running() && !closing.errorCode().isEmpty());
            }) && closing.shadowActive(),
            "native window launched before Shape observer closes"
        );
        resumed_handoff = closing.handoffPath();
    }
    // Destroying the observer must leave the provider alive and its exact local continuation
    // intact.
    require(
        waitUntil([&] {
            return QFileInfo::exists(
                QFileInfo(resumed_handoff).dir().filePath("shadow-result.json")
            );
        }),
        "independent Shadow completes after Shape observer destruction"
    );
    MaterialEditController restored(backend, resume_settings);
    require(
        restored.restoreShadowSession(backend.bundlePath(), resume_output) && waitUntil([&] {
            return !restored.running() && !restored.shadowActive();
        }) && restored.errorCode().isEmpty(),
        "reopened local settings recover handoff and strictly rebuild candidate"
    );
    bool resumed_candidate = false;
    for (const auto& value : backend.candidates())
        if (value.toMap().value("artifactId") == resume_output)
            resumed_candidate = true;
    require(
        resumed_candidate
            && artifactPin(backend, resume_output)["revision_id"].toString().isEmpty(),
        "resumed provider receipt stays unaccepted"
    );
    require(
        restored.restart(backend.bundlePath(), resume_output, false),
        "resumed candidate explicitly cancelled"
    );
    QFile report(directory + "-native-launch.json");
    require(report.open(QIODevice::WriteOnly), "native launcher evidence");
    report.write(QJsonDocument(
                     QJsonObject{
                         {"shadow_executable", controller->shadowExecutable()},
                         {"completed_candidate", candidate},
                         {"accepted", false},
                         {"late_result", result},
                         {"cancelled_return_ignored", true},
                         {"source_unchanged", true},
                         {"process_exit_without_receipt_retryable", true},
                         {"observer_close_preserves_shadow", true},
                         {"settings_resume_rebuilds_candidate", true},
                         {"resumed_handoff", resumed_handoff}
                     }
    ).toJson());
}
void deliverExportCallback(
    QObject& workspace,
    const QString& name,
    const QString& destination,
    const QJsonObject& context
) {
    auto* dialog = workspace.findChild<QObject*>(name);
    require(dialog, "packaged book export dialog");
    require(
        dialog->setProperty("requestContext", context.toVariantMap())
            && dialog->setProperty("selectedFile", QUrl::fromLocalFile(destination)),
        "captured book export callback arguments"
    );
    if (name == QStringLiteral("pictureBookPdfExportDialog")) {
        dialog->setProperty("language", "zh-CN");
        dialog->setProperty("includePrintOnly", false);
    }
    require(QMetaObject::invokeMethod(dialog, "accepted"), "delayed native save callback");
    QCoreApplication::processEvents();
}
void verifyExportContextFences(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& workspace,
    const QVariantMap& older_source,
    const QString& directory
) {
    const auto source = QJsonObject::fromVariantMap(book.source());
    const auto project = backend.projectId(), bundle = backend.bundlePath();
    const auto book_id = source["artifact_id"].toString();
    const auto page_source =
        QJsonObject::fromVariantMap(workspace.property("draft").toMap())["pages"]
            .toArray()[2]
            .toObject()["source"]
            .toObject();
    const auto locator_stage = book.beginArtworkEdit(page_source.toVariantMap());
    const auto locator_output = locator_stage.value("contextArtifactId").toString();
    require(!locator_output.isEmpty(), "same-id locator stage fixture");
    const auto locator_ticket = backend.materialEditTicket(locator_output);
    QJsonObject context{{"project", project}, {"bundle", bundle}, {"source", source}};
    auto old_context = context;
    old_context["source"] = QJsonObject::fromVariantMap(older_source);
    const auto stale_native = directory + "/stale-book.scproj";
    const auto stale_pdf = directory + "/stale-book.pdf";
    deliverExportCallback(workspace, "pictureBookNativeExportDialog", stale_native, old_context);
    deliverExportCallback(workspace, "pictureBookPdfExportDialog", stale_pdf, old_context);
    require(
        !QFileInfo::exists(stale_native) && !QFileInfo::exists(stale_pdf),
        "changed accepted book revision rejects delayed exports before file creation"
    );
    const auto interlude = directory + "/interlude";
    require(QDir().mkpath(interlude), "bundle fence fixture parent");
    require(
        backend.createProject(QUrl::fromLocalFile(interlude), "Unrelated fixture"),
        "close original bundle before copying its database"
    );
    const auto copied = directory + "/same-identities.shape";
    std::error_code error;
    std::filesystem::copy(
        bundle.toStdString(),
        copied.toStdString(),
        std::filesystem::copy_options::recursive,
        error
    );
    require(!error, "copy closed project with exact project/book/material identities");
    require(backend.openProject(QUrl::fromLocalFile(bundle)), "return to originating bundle");
    evaluate(
        workspace,
        QStringLiteral("openWorkspace('%1'); choosePage(2); openArtworkGeneration()").arg(book_id)
    );
    require(
        workspace.property("generationTargetCurrent").toBool(),
        "original bundle page generation context"
    );
    require(
        backend.openProject(QUrl::fromLocalFile(copied)) && backend.projectId() == project,
        "switch directly to distinct bundle with identical project id"
    );
    require(
        waitUntil([&] { return !workspace.property("visible").toBool(); })
            && !workspace.property("sessionCurrent").toBool()
            && workspace.property("generationBook").toString().isEmpty(),
        "same-id different-path switch closes and invalidates old book generation context"
    );
    evaluate(workspace, QStringLiteral("openWorkspace('%1')").arg(book_id));
    require(book.source() == source.toVariantMap(), "copy keeps exact accepted book pin");
    const auto foreign_native = directory + "/foreign-bundle.scproj";
    const auto foreign_pdf = directory + "/foreign-bundle.pdf";
    deliverExportCallback(workspace, "pictureBookNativeExportDialog", foreign_native, context);
    deliverExportCallback(workspace, "pictureBookPdfExportDialog", foreign_pdf, context);
    require(
        !QFileInfo::exists(foreign_native) && !QFileInfo::exists(foreign_pdf),
        "original save callbacks cannot export from a same-id project copy"
    );
    context["bundle"] = backend.bundlePath();
    const auto current_native = directory + "/current-bundle.scproj";
    const auto current_pdf = directory + "/current-bundle.pdf";
    deliverExportCallback(workspace, "pictureBookNativeExportDialog", current_native, context);
    deliverExportCallback(workspace, "pictureBookPdfExportDialog", current_pdf, context);
    require(
        QFileInfo::exists(current_native) && QFileInfo::exists(current_pdf),
        "current captured bundle and book version still export through actual dialog callbacks"
    );
    verifyBundleScopedShadowLocators(
        backend,
        bundle,
        copied,
        locator_output,
        locator_ticket,
        directory
    );
    require(backend.openProject(QUrl::fromLocalFile(bundle)), "restore original fixture project");
    evaluate(workspace, QStringLiteral("openWorkspace('%1')").arg(book_id));
    require(
        book.source() == source.toVariantMap(),
        "export guards preserve original accepted book"
    );
}
} // namespace
bool verify(
    DesktopBackend& backend,
    PictureBookController& book,
    QObject& root,
    const QString& directory
) {
    try {
        const auto restart_phase = qEnvironmentVariable("SHAPE_BOOK_PROCESS_RESTART_PHASE");
        if (!restart_phase.isEmpty())
            return processRestart(backend, book, directory, restart_phase);
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
        evaluate(*workspace, QStringLiteral("flush()"));
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
        pageArtworkContinuation(
            backend,
            book,
            *workspace,
            artwork,
            directory + "/page-artwork-return",
            request
        );
        verifyDraftRecovery(backend, book, *workspace);
        verifyPageGeneration(backend, book, root, *workspace, *window);
        verifyNativeLauncher(backend, book, root, original_image, directory + "/shadow-launch");
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
        const auto original_size = window->size();
        window->resize(1120, 720);
        preferences->setLanguageMode(QStringLiteral("en"));
        capture(*window, directory + "/shape-book-narrow-en.png");
        preferences->setLanguageMode(QStringLiteral("zh_CN"));
        capture(*window, directory + "/shape-book-narrow-zh.png");
        preferences->setLanguageMode(QStringLiteral("en"));
        window->resize(original_size);
        verifyExportContextFences(backend, book, *workspace, original_source, directory);
        verifyRefreshedStagePageFence(
            backend,
            book,
            root,
            *workspace,
            artwork,
            directory + "/source-refresh-fence"
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
                 "typed script protected from material/language replacement",
                 "typed script survives return to book",
                 "packaged English/Chinese guidance preserves typed script",
                 "failed script adoption retains input for retry",
                 "explicit script adoption preserves authored text",
                 "explicit discard restores adopted book head",
                 "no-op refresh preserves review and clean state",
                 "repeated artwork pages refresh exact accepted stage output",
                 "continued book re-reviewed and adopted",
                 "current-page Shadow stage stays inside book context",
                 "image Candidate requires explicit page adoption",
                 "page adoption preserves other pages and pending language script",
                 "refreshed stage source cannot adopt into a page bound to the prior source",
                 "image and book acceptance remain separate",
                 "unfinished script checkpoint survives real project reopen",
                 "page generation opens through actual pointer with editable stanza prompt",
                 "page generation uses exact new node and rejects foreign callbacks",
                 "offline page generation uses isolated missing credential and makes no provider "
                 "request",
                 "1120x720 bilingual book captures",
                 "lower-case role and numeric page tags select exact generation stanza",
                 "same-id project-copy switch invalidates workspace generation context",
                 "native and PDF save callbacks fence bundle and accepted book revision",
                 "copied bundles retain independent automatic Shadow locators and cancellation",
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

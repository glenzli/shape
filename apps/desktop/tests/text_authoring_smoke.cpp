//! General writing keeps visible editing, recovery and exact Candidate/Accept contracts.
#include "text_authoring_smoke.hpp"
#include "desktop_backend.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTimer>
#include <QVariant>
#include <functional>
#include <iostream>

namespace {
bool check(bool condition, const char* label) {
    if (!condition)
        std::cerr << "text authoring smoke failed: " << label << std::endl;
    return condition;
}
bool waitUntil(const std::function<bool()>& ready, int timeout = 5000) {
    QElapsedTimer timer;
    timer.start();
    while (!ready() && timer.elapsed() < timeout) {
        QEventLoop loop;
        QTimer::singleShot(20, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return ready();
}

QVariant evaluate(QObject& object, const QString& source) {
    QQmlExpression expression(QQmlEngine::contextForObject(&object), &object, source);
    const auto value = expression.evaluate();
    if (expression.hasError())
        std::cerr << expression.error().toString().toStdString() << std::endl;
    return expression.hasError() ? QVariant() : value;
}
QQuickItem* visualChild(QQuickItem* root, const QString& name) {
    if (!root)
        return nullptr;
    if (root->objectName() == name)
        return root;
    for (auto* child : root->childItems())
        if (auto* found = visualChild(child, name))
            return found;
    return nullptr;
}
bool click(QObject& owner, const char* name) {
    QObject* item = owner.findChild<QObject*>(QString::fromLatin1(name));
    if (!item)
        item = visualChild(qobject_cast<QQuickItem*>(&owner), QString::fromLatin1(name));
    return check(
        item && item->property("enabled").toBool()
            && QMetaObject::invokeMethod(item, "clicked", Qt::DirectConnection),
        name
    );
}
} // namespace

bool text_authoring_smoke::verify(DesktopBackend& backend, QObject& root) {
    auto* host = root.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (!check(host, "workspace host"))
        return false;
    const auto current = [&] { return qvariant_cast<QObject*>(host->property("loadedWorkspace")); };
    const auto before = backend.artifactCount();
    evaluate(root, QStringLiteral("startTextAuthoring('plain')"));
    if (!check(
            waitUntil([&] {
                return current() && current()->objectName() == "textAuthoringWorkspace";
            }) && backend.artifactCount() == before + 1,
            "plain writing opens directly"
        ))
        return false;
    auto* writer = current();
    if (!check(
            writer->property("profile") == "plain"
                && !writer->findChild<QObject*>("speechScriptHelpDialog")
                && !root.findChild<QObject*>("audioSpeechOperatorWorkspace"),
            "new plain writing has no production script UI"
        ))
        return false;
    auto* modelPicker = writer->findChild<QObject*>(QStringLiteral("writingModelPicker"));
    auto* modelChoice =
        modelPicker ? modelPicker->findChild<QObject*>(QStringLiteral("aiModelChoice")) : nullptr;
    auto* effortChoice =
        modelPicker ? modelPicker->findChild<QObject*>(QStringLiteral("aiEffortChoice")) : nullptr;
    if (!check(
            modelChoice
                && writer->property("selectedModelKey").toString()
                       == writer->property("defaultTextModel").toString()
                && QMetaObject::invokeMethod(
                    modelChoice,
                    "activated",
                    Qt::DirectConnection,
                    Q_ARG(int, 3)
                )
                && writer->property("selectedModelKey").toString() == QStringLiteral("gpt_6_sol")
                && effortChoice
                && QMetaObject::invokeMethod(
                    effortChoice,
                    "activated",
                    Qt::DirectConnection,
                    Q_ARG(int, 3)
                )
                && writer->property("selectedEffortKey").toString() == QStringLiteral("high")
                && QMetaObject::invokeMethod(
                    modelChoice,
                    "activated",
                    Qt::DirectConnection,
                    Q_ARG(int, 0)
                )
                && writer->property("selectedModelKey").toString()
                       == writer->property("defaultTextModel").toString()
                && writer->property("selectedEffortKey").toString().isEmpty(),
            "writing model and effort can override for one run and return to local default"
        ))
        return false;

    auto* prompt = writer->findChild<QObject*>("writingInstructionEditor");
    auto* generate = writer->findChild<QObject*>("writingGenerateButton");
    if (!check(
            prompt && generate && prompt->property("text").toString().isEmpty()
                && !generate->property("enabled").toBool()
                && !writer->property("hasAcceptedRevision").toBool(),
            "empty draft is not an accepted placeholder"
        ))
        return false;
    if (!click(*writer, "writingMode_manual"))
        return false;
    auto* editor = writer->findChild<QObject*>("writingManualEditor");
    if (!check(editor, "manual editor"))
        return false;
    const auto oldSize = QSize(root.property("width").toInt(), root.property("height").toInt());
    root.setProperty("width", 1120);
    root.setProperty("height", 740);
    editor->setProperty(
        "text",
        QStringLiteral("A long bilingual paragraph. 中英文混合。\n").repeated(350)
    );
    if (!check(
            waitUntil([&] { return editor->property("canScroll").toBool(); }),
            "long text scrolls inside its editor"
        ))
        return false;
    evaluate(*editor, QStringLiteral("focusEditor()"));
    editor->setProperty("cursorPosition", editor->property("text").toString().size());
    if (!check(
            waitUntil([&] {
                return evaluate(
                           *editor,
                           QStringLiteral(
                               "contentItem.contentY > 0 && contentItem.contentY + availableHeight "
                               ">= contentHeight - 24"
                           )
                )
                    .toBool();
            }),
            "typing at the end keeps the caret in view"
        ))
        return false;
    auto* scroll = writer->findChild<QObject*>(QStringLiteral("writingScroll"));
    const auto originalScrollHeight = scroll ? scroll->property("height") : QVariant();
    if (scroll)
        scroll->setProperty("height", 600);
    QCoreApplication::processEvents();
    if (!check(
            scroll && editor->property("height").toDouble() <= 380
                && scroll->property("contentHeight").toDouble()
                       > scroll->property("availableHeight").toDouble(),
            "bounded editor and independently scrollable form"
        ))
        return false;
    auto* form = writer->findChild<QObject*>(QStringLiteral("writingForm"));
    if (!check(
            waitUntil([&] {
                return form
                       && form->property("width").toDouble()
                              <= scroll->property("availableWidth").toDouble();
            }),
            "form fits the viewport horizontally"
        ))
        return false;
    evaluate(
        *scroll,
        QStringLiteral("contentItem.contentY = Math.max(0, contentHeight - availableHeight)")
    );
    std::cout << "layout viewport=" << scroll->property("availableWidth").toDouble()
              << " form=" << form->property("width").toDouble()
              << " editor=" << editor->property("height").toDouble() << std::endl;
    QEventLoop settle;
    QTimer::singleShot(200, &settle, &QEventLoop::quit);
    settle.exec();
    evaluate(
        *scroll,
        QStringLiteral("contentItem.contentY = Math.max(0, contentHeight - availableHeight)")
    );

    scroll->setProperty("height", originalScrollHeight);
    root.setProperty("width", oldSize.width());
    root.setProperty("height", oldSize.height());
    const auto longText = editor->property("text").toString();
    const auto artifactId = writer->property("artifactId").toString();
    const auto draftId = writer->property("draftId").toString();
    if (!check(evaluate(*writer, "checkpoint()").toBool(), "save long ordinary draft"))
        return false;
    evaluate(root, "workspaceSurface.showGraph()");
    evaluate(root, "workspaceSurface.openOperatorDraft('" + draftId + "')");
    if (!check(
            waitUntil([&] { return current() && current()->property("draftId") == draftId; }),
            "reopen stable writing draft"
        ))
        return false;
    writer = current();
    editor = writer->findChild<QObject*>("writingManualEditor");
    if (!check(
            editor && editor->property("text").toString() == longText,
            "long draft exact text survives reopening"
        ))
        return false;
    const QString authored = QStringLiteral("A quiet first draft.\n原样保留换行与文字。");
    editor->setProperty("text", authored);
    evaluate(*writer, "updatePreview(); checkpoint()");
    if (!check(
            backend.proposeAuthoredText(draftId) && backend.hasCandidate()
                && backend.textAuthoringContent(artifactId, backend.candidateId()) == authored
                && backend.textAuthoringContent(artifactId, {}).isEmpty(),
            "manual result remains a Candidate before explicit adoption"
        ))
        return false;
    if (!click(*writer, "writingAdoptButton"))
        return false;
    if (!check(
            backend.textAuthoringContent(artifactId, {}) == authored,
            "visible Use this text adopts exact authored bytes"
        ))
        return false;
    const auto count = backend.artifactCount();
    evaluate(root, "startTextAuthoring('script')");
    if (!check(backend.artifactCount() == count, "specialist creation is unavailable"))
        return false;
    const auto branchDraft = backend.beginOperatorDraft(artifactId, "text.summarize");
    if (!check(!branchDraft.isEmpty(), "accepted original can create separate summary"))
        return false;
    QString branchContext;
    for (const auto& item : backend.operatorDrafts())
        if (item.toMap()["id"].toString() == branchDraft)
            branchContext = item.toMap()["contextArtifactId"].toString();
    evaluate(root, "selectedArtifactIndex = artifactIndex('" + branchContext + "')");
    evaluate(root, "workspaceSurface.openOperatorDraft('" + branchDraft + "')");
    if (!check(
            waitUntil([&] { return current() && current()->property("draftId") == branchDraft; }),
            "summary opens same general editor"
        ))
        return false;
    writer = current();
    const auto branchId = writer->property("artifactId").toString();
    if (!click(*writer, "writingMode_manual"))
        return false;
    editor = writer->findChild<QObject*>("writingManualEditor");
    const QString branchText = QStringLiteral("Reviewed branch. 已审阅的分支内容。");
    if (!check(
            editor && writer->property("inputCurrent").toBool() && branchId != artifactId,
            "branch starts on an exact original pin"
        ))
        return false;
    editor->setProperty("text", branchText);
    evaluate(*writer, "updatePreview(); checkpoint()");
    auto* adopt = writer->findChild<QObject*>("writingAdoptButton");
    if (!check(
            adopt && adopt->property("enabled").toBool(),
            "valid branch can initially be adopted"
        ))
        return false;
    if (!click(*writer, "writingPreviewButton"))
        return false;
    const auto updatedOriginal =
        authored + QStringLiteral("\nAnother contributor updated this original.");
    if (!check(
            backend.proposeTextCandidate(artifactId, updatedOriginal)
                && backend.acceptCandidate(backend.candidateId()),
            "real concurrent accepted-head advance"
        ))
        return false;
    if (!check(
            waitUntil([&] { return !writer->property("inputCurrent").toBool(); })
                && !adopt->property("enabled").toBool()
                && backend.textAuthoringContent(branchId, {}).isEmpty(),
            "stale input blocks branch acceptance"
        ))
        return false;
    auto* impact = writer->findChild<QObject*>("writingRefreshImpact");
    if (!check(
            impact && impact->property("visible").toBool()
                && !impact->property("text").toString().isEmpty(),
            "refresh consequences are visible"
        ))
        return false;
    if (!click(*writer, "writingRefreshInputButton"))
        return false;
    if (!check(
            !writer->property("reviewing").toBool() && waitUntil([&] {
                return writer->property("inputCurrent").toBool()
                       && adopt->property("enabled").toBool();
            }) && editor->property("text").toString() == branchText,
            "explicit refresh keeps authored branch and recovers acceptance"
        ))
        return false;
    if (!click(*writer, "writingAdoptButton"))
        return false;
    return check(
        backend.textAuthoringContent(branchId, {}) == branchText
            && backend.textAuthoringContent(artifactId, {}) == updatedOriginal,
        "adopted branch does not rewrite the newer original"
    );
}

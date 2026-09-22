//! Packaged documentation and clipboard checks; preserves the user's clipboard formats.
#include "speech_script_help_smoke.hpp"
#include <QClipboard>
#include <QCoreApplication>
#include <QFile>
#include <QGuiApplication>
#include <QMetaObject>
#include <QMimeData>
#include <QObject>
#include <QVariant>
#include <iostream>
#include <memory>

namespace {
struct RestoreClipboard {
    QClipboard* clipboard = QGuiApplication::clipboard();
    std::unique_ptr<QMimeData> original = std::make_unique<QMimeData>();
    RestoreClipboard() {
        if (const auto* data = clipboard->mimeData())
            for (const auto& format : data->formats())
                original->setData(format, data->data(format));
    }
    ~RestoreClipboard() {
        clipboard->setMimeData(original.release());
    }
};
bool check(bool passed, const char* label) {
    if (!passed)
        std::cerr << "script help smoke failed: " << label << std::endl;
    return passed;
}
} // namespace

bool speech_script_help_smoke::verify(QObject& root) {
    QCoreApplication::processEvents();
    auto* host = root.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    auto* workspace = host ? qvariant_cast<QObject*>(host->property("loadedWorkspace")) : nullptr;
    if (!check(workspace != nullptr, "current speech workspace"))
        return false;
    auto* button = workspace->findChild<QObject*>(QStringLiteral("speechScriptRulesButton"));
    if (!check(
            button && QMetaObject::invokeMethod(button, "clicked", Qt::DirectConnection),
            "open from speech workspace"
        ))
        return false;
    QCoreApplication::processEvents();
    auto* dialog = workspace->findChild<QObject*>(QStringLiteral("speechScriptHelpDialog"));
    auto* tabs = workspace->findChild<QObject*>(QStringLiteral("speechScriptHelpTabs"));
    auto* copy = workspace->findChild<QObject*>(QStringLiteral("copySpeechScriptDocumentButton"));
    if (!check(
            dialog && tabs && copy && dialog->property("visible").toBool(),
            "packaged guide visible"
        ))
        return false;
    RestoreClipboard restore;
    const QStringList paths{
        QStringLiteral(":/shape/help/SPEECH_SCRIPT.md"),
        QStringLiteral(":/shape/help/SPEECH_SCRIPT_PROMPT.md")
    };
    for (int index = 0; index < paths.size(); ++index) {
        tabs->setProperty("currentIndex", index);
        QCoreApplication::processEvents();
        QFile file(paths[index]);
        if (!check(file.open(QIODevice::ReadOnly), "canonical document bundled"))
            return false;
        const QString expected = QString::fromUtf8(file.readAll());
        if (!check(
                expected.contains(QStringLiteral("20260922.2"))
                    && expected.contains(QStringLiteral("[end-repeat]"))
                    && dialog->property("documentText").toString() == expected,
                "full document shown"
            ))
            return false;
        if (!check(
                QMetaObject::invokeMethod(copy, "clicked", Qt::DirectConnection),
                "copy button connected"
            ))
            return false;
        QCoreApplication::processEvents();
        if (!check(
                restore.clipboard->text() == expected && dialog->property("copied").toBool(),
                "exact Markdown copied"
            ))
            return false;
    }
    return check(QMetaObject::invokeMethod(dialog, "close", Qt::DirectConnection), "close guide");
}

#include "speech_script_documentation.hpp"
#include <QClipboard>
#include <QFile>
#include <QGuiApplication>

namespace {
QString readDocument(const QString& path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || file.size() > 64 * 1024)
        return QString();
    return QString::fromUtf8(file.readAll());
}
} // namespace

SpeechScriptDocumentation::SpeechScriptDocumentation(QObject* parent)
    : QObject(parent), rules_(readDocument(QStringLiteral(":/shape/help/SPEECH_SCRIPT.md"))),
      writing_prompt_(readDocument(QStringLiteral(":/shape/help/SPEECH_SCRIPT_PROMPT.md"))) {}

QString SpeechScriptDocumentation::rules() const {
    return rules_;
}
QString SpeechScriptDocumentation::writingPrompt() const {
    return writing_prompt_;
}

bool SpeechScriptDocumentation::copyDocument(bool writingPrompt) {
    const QString& text = writingPrompt ? writing_prompt_ : rules_;
    if (text.isEmpty())
        return false;
    QGuiApplication::clipboard()->setText(text);
    return true;
}

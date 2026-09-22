//! Offline narration documentation, packaged from the canonical Markdown files.
#pragma once
#include <QObject>
#include <QString>
#include <QtQml/qqmlregistration.h>

class SpeechScriptDocumentation : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON
    Q_PROPERTY(QString rules READ rules CONSTANT)
    Q_PROPERTY(QString writingPrompt READ writingPrompt CONSTANT)

  public:
    explicit SpeechScriptDocumentation(QObject* parent = nullptr);
    [[nodiscard]] QString rules() const;
    [[nodiscard]] QString writingPrompt() const;
    Q_INVOKABLE bool copyDocument(bool writingPrompt);

  private:
    QString rules_;
    QString writing_prompt_;
};

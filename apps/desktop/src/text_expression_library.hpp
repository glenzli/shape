//! Personal reusable text-expression presets. Project drafts copy the selected
//! preset's authored content, so changing this library never mutates history.

#pragma once

#include <QObject>
#include <QSettings>
#include <QString>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include <memory>

class TextExpressionLibrary : public QObject {
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(QVariantList presets READ presets NOTIFY presetsChanged)

  public:
    explicit TextExpressionLibrary(QObject* parent = nullptr);

    [[nodiscard]] QVariantList presets() const;

    Q_INVOKABLE QString savePreset(
        const QString& presetId,
        const QString& name,
        const QString& instruction,
        const QString& example,
        const QString& visualKey
    );
    Q_INVOKABLE bool removePreset(const QString& presetId);

  signals:
    void presetsChanged();

  private:
    void load();
    void persist() const;

    std::unique_ptr<QSettings> settings_;
    QVariantList presets_;
};

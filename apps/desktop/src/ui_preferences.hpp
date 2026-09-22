//! Process-level UI preferences. This owner keeps appearance and language
//! lifecycle separate from project data and makes both settings live-update.

#pragma once

#include <QGuiApplication>
#include <QObject>
#include <QSettings>
#include <QString>
#include <QTranslator>
#include <QtQml/qqmlregistration.h>

#include <memory>

class QQmlEngine;

class UiPreferences final : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("UiPreferences is created by the host application")
    Q_PROPERTY(
        int appearanceMode READ appearanceMode WRITE setAppearanceMode NOTIFY appearanceModeChanged
    )
    Q_PROPERTY(bool dark READ dark NOTIFY darkChanged)
    Q_PROPERTY(
        QString languageMode READ languageMode WRITE setLanguageMode NOTIFY languageModeChanged
    )
    Q_PROPERTY(QString textModel READ textModel WRITE setTextModel NOTIFY textModelChanged)
    Q_PROPERTY(QString imageModel READ imageModel WRITE setImageModel NOTIFY imageModelChanged)

  public:
    enum class AppearanceMode {
        System = 0,
        Light = 1,
        Dark = 2,
    };
    Q_ENUM(AppearanceMode)

    explicit UiPreferences(QGuiApplication& application, QObject* parent = nullptr);

    int appearanceMode() const;
    void setAppearanceMode(int mode);

    bool dark() const;

    QString languageMode() const;
    void setLanguageMode(const QString& mode);
    QString textModel() const;
    void setTextModel(const QString& model);
    QString imageModel() const;
    void setImageModel(const QString& model);

    void attachEngine(QQmlEngine& engine);

  signals:
    void appearanceModeChanged();
    void darkChanged();
    void languageModeChanged();
    void textModelChanged();
    void imageModelChanged();

  private:
    void refreshEffectiveAppearance();
    void applyLanguage();

    QGuiApplication& application_;
    std::unique_ptr<QSettings> settings_;
    QQmlEngine* engine_ = nullptr;
    std::unique_ptr<QTranslator> translator_;
    int appearance_mode_ = static_cast<int>(AppearanceMode::System);
    bool effective_dark_ = true;
    QString language_mode_ = QStringLiteral("system");
    QString text_model_ = QStringLiteral("local_qwen");
    QString image_model_ = QStringLiteral("gpt_5_6_luna");
};

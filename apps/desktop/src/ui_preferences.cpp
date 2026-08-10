#include "ui_preferences.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QLocale>
#include <QPalette>
#include <QQmlEngine>
#include <QStandardPaths>
#include <QStyleHints>

namespace {

constexpr auto kAppearanceSettingsKey = "ui/appearanceMode";
constexpr auto kLanguageSettingsKey = "ui/language";

int normalizeAppearanceMode(const int mode) {
    if (mode >= static_cast<int>(UiPreferences::AppearanceMode::System)
        && mode <= static_cast<int>(UiPreferences::AppearanceMode::Dark)) {
        return mode;
    }
    return static_cast<int>(UiPreferences::AppearanceMode::System);
}

QString normalizeLanguageMode(const QString& mode) {
    if (mode == QStringLiteral("zh_CN") || mode == QStringLiteral("en")) {
        return mode;
    }
    return QStringLiteral("system");
}

QString systemLanguage() {
    for (const QString& language : QLocale::system().uiLanguages()) {
        if (language.startsWith(QStringLiteral("zh"))) {
            return QStringLiteral("zh_CN");
        }
    }
    return QStringLiteral("en");
}

} // namespace

UiPreferences::UiPreferences(QGuiApplication& application, QObject* parent)
    : QObject(parent), application_(application) {
    const QString config_directory =
        QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(config_directory);
    settings_ = std::make_unique<QSettings>(
        config_directory + QStringLiteral("/shape.conf"),
        QSettings::IniFormat
    );
    appearance_mode_ = normalizeAppearanceMode(
        settings_->value(QString::fromLatin1(kAppearanceSettingsKey), 0).toInt()
    );
    language_mode_ = normalizeLanguageMode(
        settings_->value(QString::fromLatin1(kLanguageSettingsKey), QStringLiteral("system"))
            .toString()
    );

    QObject::connect(
        application_.styleHints(),
        &QStyleHints::colorSchemeChanged,
        this,
        [this](const Qt::ColorScheme) { refreshEffectiveAppearance(); }
    );
    refreshEffectiveAppearance();
    applyLanguage();
}

int UiPreferences::appearanceMode() const {
    return appearance_mode_;
}

void UiPreferences::setAppearanceMode(const int mode) {
    const int normalized = normalizeAppearanceMode(mode);
    if (normalized == appearance_mode_) {
        return;
    }
    appearance_mode_ = normalized;
    settings_->setValue(QString::fromLatin1(kAppearanceSettingsKey), appearance_mode_);
    emit appearanceModeChanged();
    refreshEffectiveAppearance();
}

bool UiPreferences::dark() const {
    return effective_dark_;
}

void UiPreferences::refreshEffectiveAppearance() {
    bool next_dark = false;
    switch (static_cast<AppearanceMode>(appearance_mode_)) {
    case AppearanceMode::Light:
        next_dark = false;
        break;
    case AppearanceMode::Dark:
        next_dark = true;
        break;
    case AppearanceMode::System: {
        const Qt::ColorScheme scheme = application_.styleHints()->colorScheme();
        next_dark = scheme == Qt::ColorScheme::Dark
                    || (scheme == Qt::ColorScheme::Unknown
                        && application_.palette().color(QPalette::Window).lightness() < 128);
        break;
    }
    }
    if (next_dark != effective_dark_) {
        effective_dark_ = next_dark;
        emit darkChanged();
    }
}

QString UiPreferences::languageMode() const {
    return language_mode_;
}

void UiPreferences::setLanguageMode(const QString& mode) {
    const QString normalized = normalizeLanguageMode(mode);
    if (normalized == language_mode_) {
        return;
    }
    language_mode_ = normalized;
    settings_->setValue(QString::fromLatin1(kLanguageSettingsKey), language_mode_);
    applyLanguage();
    emit languageModeChanged();
}

void UiPreferences::attachEngine(QQmlEngine& engine) {
    engine_ = &engine;
    engine_->retranslate();
}

void UiPreferences::applyLanguage() {
    const QString effective_language =
        language_mode_ == QStringLiteral("system") ? systemLanguage() : language_mode_;
    auto next_translator = std::make_unique<QTranslator>();
    const bool install_translator =
        effective_language == QStringLiteral("zh_CN")
        && next_translator->load(QStringLiteral(":/translations/shape_zh_CN.qm"));

    if (translator_ != nullptr) {
        QCoreApplication::removeTranslator(translator_.get());
    }
    translator_ = std::move(next_translator);
    if (install_translator) {
        QCoreApplication::installTranslator(translator_.get());
    }

    QLocale::setDefault(QLocale(
        effective_language == QStringLiteral("zh_CN") ? QStringLiteral("zh_CN")
                                                      : QStringLiteral("en_US")
    ));
    if (engine_ != nullptr) {
        engine_->retranslate();
    }
}

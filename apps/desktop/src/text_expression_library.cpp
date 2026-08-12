#include "text_expression_library.hpp"

#include <QDir>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QUuid>
#include <QVariantMap>

#include <algorithm>
#include <array>

namespace {

constexpr auto kCustomTonesSettingsKey = "textExpression/customTones";
constexpr int kMaximumPresetCount = 24;
constexpr int kMaximumNameBytes = 64;
constexpr int kMaximumInstructionBytes = 512;
constexpr int kMaximumExampleBytes = 256;

bool hasValidAuthoredText(const QString& value, const int maximumBytes) {
    if (value.trimmed().isEmpty() || value.toUtf8().size() > maximumBytes) {
        return false;
    }
    return std::none_of(value.cbegin(), value.cend(), [](const QChar character) {
        return character.isNull()
               || (character.category() == QChar::Other_Control && character != QChar('\n')
                   && character != QChar('\t'));
    });
}

bool hasValidPresetId(const QString& value) {
    if (value.isEmpty() || value.size() > 80) {
        return false;
    }
    return std::all_of(value.cbegin(), value.cend(), [](const QChar character) {
        return character.isLetterOrNumber() || character == QChar('-') || character == QChar('_');
    });
}

bool hasValidVisualKey(const QString& value) {
    static constexpr std::array<const char*, 8> kVisualKeys = {
        "ripple",
        "glow",
        "embrace",
        "ascent",
        "spark",
        "frame",
        "pillar",
        "pulse",
    };
    return std::any_of(kVisualKeys.cbegin(), kVisualKeys.cend(), [&value](const char* key) {
        return value == QString::fromLatin1(key);
    });
}

QVariantMap validatedPreset(
    const QString& id,
    const QString& name,
    const QString& instruction,
    const QString& example,
    const bool allowMissingExample,
    const QString& visualKey
) {
    if (!hasValidPresetId(id) || !hasValidAuthoredText(name, kMaximumNameBytes)
        || !hasValidAuthoredText(instruction, kMaximumInstructionBytes)
        || (!allowMissingExample && !hasValidAuthoredText(example, kMaximumExampleBytes))
        || (!example.trimmed().isEmpty() && !hasValidAuthoredText(example, kMaximumExampleBytes))
        || !hasValidVisualKey(visualKey)) {
        return {};
    }
    return {
        {QStringLiteral("id"), id},
        {QStringLiteral("name"), name.trimmed()},
        {QStringLiteral("instruction"), instruction.trimmed()},
        {QStringLiteral("example"), example.trimmed()},
        {QStringLiteral("visualKey"), visualKey},
    };
}

} // namespace

TextExpressionLibrary::TextExpressionLibrary(QObject* parent) : QObject(parent) {
    const QString configDirectory =
        QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(configDirectory);
    settings_ = std::make_unique<QSettings>(
        configDirectory + QStringLiteral("/shape.conf"),
        QSettings::IniFormat
    );
    load();
}

QVariantList TextExpressionLibrary::presets() const {
    return presets_;
}

QString TextExpressionLibrary::savePreset(
    const QString& presetId,
    const QString& name,
    const QString& instruction,
    const QString& example,
    const QString& visualKey
) {
    const QString id = presetId.isEmpty() ? QStringLiteral("custom-tone-")
                                                + QUuid::createUuid().toString(QUuid::WithoutBraces)
                                          : presetId;
    const QVariantMap preset = validatedPreset(id, name, instruction, example, false, visualKey);
    if (preset.isEmpty()) {
        return {};
    }

    const auto existing =
        std::find_if(presets_.cbegin(), presets_.cend(), [&id](const QVariant& candidate) {
            return candidate.toMap().value(QStringLiteral("id")).toString() == id;
        });
    if (existing == presets_.cend()) {
        if (presets_.size() >= kMaximumPresetCount) {
            return {};
        }
        presets_.prepend(preset);
    } else {
        presets_[static_cast<qsizetype>(std::distance(presets_.cbegin(), existing))] = preset;
    }
    persist();
    emit presetsChanged();
    return id;
}

bool TextExpressionLibrary::removePreset(const QString& presetId) {
    const auto existing =
        std::find_if(presets_.cbegin(), presets_.cend(), [&presetId](const QVariant& candidate) {
            return candidate.toMap().value(QStringLiteral("id")).toString() == presetId;
        });
    if (existing == presets_.cend()) {
        return false;
    }
    presets_.removeAt(static_cast<qsizetype>(std::distance(presets_.cbegin(), existing)));
    persist();
    emit presetsChanged();
    return true;
}

void TextExpressionLibrary::load() {
    presets_.clear();
    const QByteArray encoded =
        settings_->value(QString::fromLatin1(kCustomTonesSettingsKey)).toByteArray();
    if (encoded.isEmpty()) {
        return;
    }
    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(encoded, &error);
    if (error.error != QJsonParseError::NoError || !document.isArray()) {
        return;
    }
    for (const QJsonValue& value : document.array()) {
        if (!value.isObject() || presets_.size() >= kMaximumPresetCount) {
            continue;
        }
        const QJsonObject object = value.toObject();
        if (object.size() != 4 && object.size() != 5) {
            continue;
        }
        const QString example = object.value(QStringLiteral("example")).toString();
        const QVariantMap preset = validatedPreset(
            object.value(QStringLiteral("id")).toString(),
            object.value(QStringLiteral("name")).toString(),
            object.value(QStringLiteral("instruction")).toString(),
            example,
            object.size() == 4,
            object.value(QStringLiteral("visualKey")).toString()
        );
        if (!preset.isEmpty()) {
            presets_.append(preset);
        }
    }
}

void TextExpressionLibrary::persist() const {
    QJsonArray encoded;
    for (const QVariant& preset : presets_) {
        encoded.append(QJsonObject::fromVariantMap(preset.toMap()));
    }
    settings_->setValue(
        QString::fromLatin1(kCustomTonesSettingsKey),
        QJsonDocument(encoded).toJson(QJsonDocument::Compact)
    );
}

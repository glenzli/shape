//! Small shared boundary helpers for authenticated Infer desktop controllers.

#pragma once

#include <QByteArray>
#include <QString>

#include <cstddef>
#include <string>

#include "rust/cxx.h"

namespace infer_controller_support {

inline std::string toUtf8(const QString& value) {
    const QByteArray bytes = value.toUtf8();
    return std::string(bytes.constData(), static_cast<std::size_t>(bytes.size()));
}

inline QString stableErrorCode(const rust::Error& error) {
    const QString code = QString::fromUtf8(error.what());
    for (const QChar character : code) {
        const char16_t value = character.unicode();
        if (!((value >= u'a' && value <= u'z') || (value >= u'0' && value <= u'9')
              || value == u'_')) {
            return QStringLiteral("generation_failed");
        }
    }
    return code.isEmpty() || code.size() > 96 ? QStringLiteral("generation_failed") : code;
}

} // namespace infer_controller_support

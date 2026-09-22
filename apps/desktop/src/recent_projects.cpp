#include "recent_projects.hpp"

#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>
#include <QUrl>
#include <QVariantMap>

#include <algorithm>

namespace {
constexpr auto kRecentProjectsKey = "projects/recent";
constexpr qsizetype kMaximumRecentProjects = 8;

QString normalized_bundle_path(const QString& path) {
    const QFileInfo file(path);
    if (!file.isDir() || file.suffix() != QStringLiteral("shape")) {
        return {};
    }
    return file.canonicalFilePath();
}
} // namespace

RecentProjects::RecentProjects(QObject* parent)
    : RecentProjects(
          QDir(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation))
              .filePath(QStringLiteral("shape.conf")),
          parent
      ) {}

RecentProjects::RecentProjects(const QString& settings_path, QObject* parent) : QObject(parent) {
    QDir().mkpath(QFileInfo(settings_path).absolutePath());
    settings_ = std::make_unique<QSettings>(settings_path, QSettings::IniFormat);
    const int count = settings_->beginReadArray(QString::fromLatin1(kRecentProjectsKey));
    for (int index = 0; index < count && entries_.size() < kMaximumRecentProjects; ++index) {
        settings_->setArrayIndex(index);
        const QString path = settings_->value(QStringLiteral("path")).toString();
        const QString name = settings_->value(QStringLiteral("name")).toString();
        if (path.endsWith(QStringLiteral(".shape"), Qt::CaseInsensitive)
            && !name.trimmed().isEmpty()
            && std::none_of(entries_.cbegin(), entries_.cend(), [&path](const Entry& entry) {
                   return entry.path == path;
               })) {
            entries_.append({path, name});
        }
    }
    settings_->endArray();
}

QVariantList RecentProjects::entries() const {
    QVariantList result;
    for (const Entry& entry : entries_) {
        const QFileInfo bundle(entry.path);
        result.append(QVariantMap{
            {QStringLiteral("name"), entry.name},
            {QStringLiteral("path"), entry.path},
            {QStringLiteral("url"), QUrl::fromLocalFile(entry.path)},
            {QStringLiteral("available"), bundle.isDir()},
        });
    }
    return result;
}

void RecentProjects::record(const QString& bundle_path, const QString& project_name) {
    const QString path = normalized_bundle_path(bundle_path);
    const QString name = project_name.trimmed();
    if (path.isEmpty() || name.isEmpty()) {
        return;
    }
    if (!entries_.isEmpty() && entries_.first().path == path && entries_.first().name == name) {
        return;
    }
    entries_.removeIf([&path](const Entry& entry) { return entry.path == path; });
    entries_.prepend({path, name});
    while (entries_.size() > kMaximumRecentProjects) {
        entries_.removeLast();
    }
    save();
    emit entriesChanged();
}

void RecentProjects::save() {
    settings_->remove(QString::fromLatin1(kRecentProjectsKey));
    settings_->beginWriteArray(QString::fromLatin1(kRecentProjectsKey));
    for (qsizetype index = 0; index < entries_.size(); ++index) {
        settings_->setArrayIndex(static_cast<int>(index));
        settings_->setValue(QStringLiteral("path"), entries_[index].path);
        settings_->setValue(QStringLiteral("name"), entries_[index].name);
    }
    settings_->endArray();
    settings_->sync();
}

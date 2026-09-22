//! Process-local index of successfully opened project bundles.

#pragma once

#include <QObject>
#include <QSettings>
#include <QString>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include <memory>

class RecentProjects : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("RecentProjects is created by the host application")
    Q_PROPERTY(QVariantList entries READ entries NOTIFY entriesChanged)

  public:
    explicit RecentProjects(QObject* parent = nullptr);
    explicit RecentProjects(const QString& settings_path, QObject* parent = nullptr);

    [[nodiscard]] QVariantList entries() const;
    void record(const QString& bundle_path, const QString& project_name);

  signals:
    void entriesChanged();

  private:
    struct Entry {
        QString path;
        QString name;
    };

    void save();

    std::unique_ptr<QSettings> settings_;
    QList<Entry> entries_;
};

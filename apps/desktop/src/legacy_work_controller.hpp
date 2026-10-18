//! Read-only specialist snapshots and concrete Storybook/Vox export handoffs.
#pragma once
#include <QJsonObject>
#include <QObject>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>
#include <memory>
class DesktopBackend;
class QSettings;
class LegacyWorkController final : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Created by the Shape host")
    Q_PROPERTY(QVariantMap work READ work NOTIFY changed)
    Q_PROPERTY(QVariantList snapshots READ snapshots NOTIFY changed)
    Q_PROPERTY(QVariantList materials READ materials NOTIFY changed)
    Q_PROPERTY(QString snapshotKey READ snapshotKey NOTIFY changed)
    Q_PROPERTY(QString selectionToken READ selectionToken NOTIFY changed)
    Q_PROPERTY(bool hasVoxTransfer READ hasVoxTransfer NOTIFY changed)
    Q_PROPERTY(QString error READ error NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
    Q_PROPERTY(QString lastExport READ lastExport NOTIFY changed)
    Q_PROPERTY(QString storybookExecutable READ storybookExecutable NOTIFY changed)
    Q_PROPERTY(QString voxExecutable READ voxExecutable NOTIFY changed)
  public:
    LegacyWorkController(DesktopBackend&, const QString& settingsPath, QObject* = nullptr);
    ~LegacyWorkController() override;
    QVariantMap work() const {
        return work_.toVariantMap();
    }
    QVariantList snapshots() const {
        return snapshots_;
    }
    QVariantList materials() const {
        return materials_;
    }
    QString snapshotKey() const {
        return snapshot_key_;
    }
    bool hasVoxTransfer() const {
        return !vox_result_.isEmpty() && exported_token_ == selectionToken();
    }
    QString selectionToken() const;
    QString error() const {
        return error_;
    }
    QString status() const {
        return status_;
    }
    QString lastExport() const {
        return last_export_;
    }
    QString storybookExecutable() const;
    QString voxExecutable() const;
    Q_INVOKABLE bool openWork(const QString& artifactId, const QString& draftId = {});
    Q_INVOKABLE bool openText(const QString& artifactId, const QString& expectedHead);
    Q_INVOKABLE bool selectSnapshot(const QString& key);
    Q_INVOKABLE bool exportBook(const QUrl& destination, const QString& token);
    Q_INVOKABLE bool exportVox(const QUrl& parentDirectory, const QString& token);
    Q_INVOKABLE bool openStorybook();
    Q_INVOKABLE bool openVox(const QUrl& newProject, const QString& token);
    Q_INVOKABLE bool chooseStorybook(const QUrl& path);
    Q_INVOKABLE bool chooseVox(const QUrl& path);
    Q_INVOKABLE bool revealExport();
    void retranslate();
  signals:
    void changed();

  private:
    bool projectCurrent() const;
    bool fail(const QString&);
    void addMaterial(const QJsonObject& pin, const QString& role);
    DesktopBackend& backend_;
    std::unique_ptr<QSettings> settings_;
    QJsonObject catalog_, work_, selected_, draft_, composition_, vox_result_;
    QVariantList snapshots_, materials_;
    QString bundle_, project_id_, artifact_id_, draft_id_, snapshot_key_, error_, status_;
    QString last_export_, exported_token_;
};

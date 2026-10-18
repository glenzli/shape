//! Asynchronous working-copy publication and return review, scoped to the originating stage.
#pragma once
#include <QFutureWatcher>
#include <QMutex>
#include <QObject>
#include <QString>
#include <QTimer>
#include <QUrl>
#include <QtQml/qqmlregistration.h>
#include <memory>
class DesktopBackend;
class QSettings;

class MaterialEditController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("MaterialEditController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)
    Q_PROPERTY(QString requestProjectPath READ requestProjectPath NOTIFY statusChanged)
    Q_PROPERTY(QString requestArtifactId READ requestArtifactId NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)
    Q_PROPERTY(QString handoffPath READ handoffPath NOTIFY statusChanged)
    Q_PROPERTY(bool shadowActive READ shadowActive NOTIFY statusChanged)
    Q_PROPERTY(QString shadowExecutable READ shadowExecutable NOTIFY statusChanged)
  public:
    explicit MaterialEditController(DesktopBackend& backend, QObject* parent = nullptr);
    MaterialEditController(
        DesktopBackend& backend,
        const QString& settingsPath,
        QObject* parent = nullptr
    );
    ~MaterialEditController() override;
    [[nodiscard]] bool running() const {
        return running_;
    }
    [[nodiscard]] QString requestProjectPath() const {
        return project_path_;
    }
    [[nodiscard]] QString requestArtifactId() const {
        return artifact_id_;
    }
    [[nodiscard]] QString errorCode() const {
        return error_code_;
    }
    [[nodiscard]] QString handoffPath() const {
        return handoff_path_;
    }
    [[nodiscard]] bool shadowActive() const {
        return shadow_active_;
    }
    [[nodiscard]] QString shadowExecutable() const;
    Q_INVOKABLE bool setShadowExecutable(const QUrl& executable);
    Q_INVOKABLE void openShadow(const QString& projectPath, const QString& outputId);
    Q_INVOKABLE bool restoreShadowSession(const QString& projectPath, const QString& outputId);
    Q_INVOKABLE void
    exportCopy(const QString& projectPath, const QString& outputId, const QString& parentFolder);
    Q_INVOKABLE void review(
        const QString& projectPath,
        const QString& outputId,
        const QString& handoffFile,
        const QString& returnedFile,
        bool allowPcm24Conversion = false
    );
    Q_INVOKABLE bool
    restart(const QString& projectPath, const QString& outputId, bool refreshInput);
  signals:
    void statusChanged();
    void candidateCreated(const QString& candidateId, const QString& artifactId);
    void copyCreated(const QString& manifestPath, const QString& artifactId);

  private:
    struct Result;
    void start(
        const QString& projectPath,
        const QString& outputId,
        const QString& firstPath,
        const QString& secondPath,
        bool exporting,
        bool allowPcm24Conversion = false
    );
    void finish();
    void launchShadow();
    void inspectShadowResult();
    QString shadowSessionKey(const QString& projectPath, const QString& outputId) const;
    void saveShadowSession();
    void clearShadowSession();
    DesktopBackend& backend_;
    std::unique_ptr<QSettings> settings_;
    QFutureWatcher<void> watcher_;
    QMutex mutex_;
    std::unique_ptr<Result> pending_;
    quint64 generation_ = 0;
    bool running_ = false;
    bool launch_after_export_ = false, shadow_active_ = false;
    quint64 shadow_generation_ = 0;
    QTimer shadow_poll_;
    QString shadow_ticket_, shadow_result_, shadow_executable_;
    QString project_path_, artifact_id_, error_code_, handoff_path_;
};

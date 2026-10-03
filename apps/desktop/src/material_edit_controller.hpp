//! Asynchronous working-copy publication and return review, scoped to the originating stage.
#pragma once
#include <QFutureWatcher>
#include <QMutex>
#include <QObject>
#include <QString>
#include <QtQml/qqmlregistration.h>
#include <memory>
class DesktopBackend;

class MaterialEditController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("MaterialEditController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)
    Q_PROPERTY(QString requestProjectPath READ requestProjectPath NOTIFY statusChanged)
    Q_PROPERTY(QString requestArtifactId READ requestArtifactId NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)
    Q_PROPERTY(QString handoffPath READ handoffPath NOTIFY statusChanged)
  public:
    explicit MaterialEditController(DesktopBackend& backend, QObject* parent = nullptr);
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
    DesktopBackend& backend_;
    QFutureWatcher<void> watcher_;
    QMutex mutex_;
    std::unique_ptr<Result> pending_;
    quint64 generation_ = 0;
    bool running_ = false;
    QString project_path_, artifact_id_, error_code_, handoff_path_;
};

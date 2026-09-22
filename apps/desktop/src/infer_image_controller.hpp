//! Asynchronous zero-input AI Image generation lifecycle.

#pragma once

#include <QFutureWatcher>
#include <QMutex>
#include <QObject>
#include <QString>
#include <QtQml/qqmlregistration.h>

#include <memory>

#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"

class DesktopBackend;

class InferImageController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("InferImageController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)
    Q_PROPERTY(int requestedCount READ requestedCount NOTIFY statusChanged)
    Q_PROPERTY(int completedCount READ completedCount NOTIFY statusChanged)

  public:
    InferImageController(
        DesktopBackend& backend,
        QString credentialPath,
        QObject* parent = nullptr
    );
    ~InferImageController() override;

    [[nodiscard]] bool running() const;
    [[nodiscard]] QString errorCode() const;
    [[nodiscard]] int requestedCount() const { return requested_count_; }
    [[nodiscard]] int completedCount() const { return completed_count_; }

    Q_INVOKABLE void
    generate(const QString& projectPath, const QString& artifactId, const QString& draftId,
             const QString& modelKey, const QString& effortKey, int candidateCount = 1);

  signals:
    void statusChanged();
    void candidateCreated(const QString& candidateId, const QString& artifactId);

  private:
    struct GenerationResult;

    void finishGeneration();
    void setErrorCode(const QString& code);

    DesktopBackend& backend_;
    QString credential_path_;
    QString explicit_override_;
    QFutureWatcher<void> watcher_;
    QMutex result_mutex_;
    std::unique_ptr<GenerationResult> pending_result_;
    quint64 generation_ = 0;
    bool running_ = false;
    QString error_code_;
    int requested_count_ = 1;
    int completed_count_ = 0;
};

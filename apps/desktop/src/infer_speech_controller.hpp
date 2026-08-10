//! Asynchronous preset speech synthesis lifecycle.

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

class InferSpeechController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("InferSpeechController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)

  public:
    InferSpeechController(
        DesktopBackend& backend,
        QString credentialPath,
        QObject* parent = nullptr
    );
    ~InferSpeechController() override;

    [[nodiscard]] bool running() const;
    [[nodiscard]] QString errorCode() const;

    Q_INVOKABLE void generate(
        const QString& projectPath,
        const QString& sourceArtifactId,
        const QString& draftId,
        const QString& artifactName
    );

  signals:
    void statusChanged();
    void candidateCreated(const QString& candidateId, const QString& sourceArtifactId);

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
};

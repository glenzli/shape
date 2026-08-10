//! Asynchronous authenticated Infer text-generation lifecycle.

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

class InferTextController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("InferTextController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)
    Q_PROPERTY(bool credentialConfigured READ credentialConfigured NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)

  public:
    InferTextController(DesktopBackend& backend, QString credentialPath, QObject* parent = nullptr);
    ~InferTextController() override;

    [[nodiscard]] bool running() const;
    [[nodiscard]] bool credentialConfigured() const;
    [[nodiscard]] QString errorCode() const;

    Q_INVOKABLE void
    generate(const QString& projectPath, const QString& artifactId, const QString& prompt);
    Q_INVOKABLE bool installCredential(const QString& token);
    Q_INVOKABLE void refreshCredentialStatus();

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
    bool credential_configured_ = false;
    QString error_code_;
};

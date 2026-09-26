//! Background Sound prompt/audio task lifecycle; UI-thread adoption remains in DesktopBackend.

#pragma once

#include <QFutureWatcher>
#include <QMutex>
#include <QObject>
#include <QString>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

#include <memory>

#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"

class DesktopBackend;

class InferSoundController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("InferSoundController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)
    Q_PROPERTY(int stage READ stage NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)

  public:
    InferSoundController(
        DesktopBackend& backend,
        QString credentialPath,
        QObject* parent = nullptr
    );
    ~InferSoundController() override;

    [[nodiscard]] bool running() const;
    [[nodiscard]] QString errorCode() const;
    [[nodiscard]] int stage() const;
    Q_INVOKABLE void stop();
    Q_INVOKABLE void
    generate(const QString& projectPath, const QString& artifactId, const QString& draftId);

  signals:
    void statusChanged();
    void candidateCreated(const QString& candidateId, const QString& sourceArtifactId);

  private:
    struct GenerationResult;
    void finishGeneration();
    void setErrorCode(const QString& code);

    rust::Box<shape::desktop::SoundGenerationControl> control_;
    QTimer stage_timer_;
    DesktopBackend& backend_;
    QString credential_path_;
    QString explicit_override_;
    QFutureWatcher<void> watcher_;
    QMutex result_mutex_;
    std::unique_ptr<GenerationResult> pending_result_;
    quint64 generation_ = 0;
    bool running_ = false;
    bool stopped_ = false;
    QString error_code_;
};

//! Atomic local WAV export of one verified accepted revision or selected Candidate.
#pragma once

#include <QFutureWatcher>
#include <QObject>
#include <QUrl>
#include <QtQml/qqmlregistration.h>

class DesktopBackend;

class AudioExportController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("AudioExportController is created by the host application")
    Q_PROPERTY(bool running READ running NOTIFY statusChanged)

  public:
    explicit AudioExportController(DesktopBackend& backend, QObject* parent = nullptr);
    ~AudioExportController() override;
    [[nodiscard]] bool running() const;
    Q_INVOKABLE void
    exportAudio(const QString& artifactId, const QString& candidateId, const QUrl& destination);

  signals:
    void statusChanged();
    void finished(bool success, const QString& destination, const QString& errorCode);

  private:
    DesktopBackend& backend_;
    QFutureWatcher<QString> watcher_;
    QString destination_;
    bool running_ = false;
};

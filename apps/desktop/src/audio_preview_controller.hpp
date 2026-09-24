//! Selected-only in-memory WAV playback lifecycle for accepted audio and Candidates.

#pragma once

#include <QAudioOutput>
#include <QBuffer>
#include <QMediaPlayer>
#include <QObject>
#include <QString>
#include <QtQml/qqmlregistration.h>

class DesktopBackend;

class AudioPreviewController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("AudioPreviewController is created by the host application")
    Q_PROPERTY(bool hasAudio READ hasAudio NOTIFY statusChanged)
    Q_PROPERTY(bool playing READ playing NOTIFY statusChanged)
    Q_PROPERTY(qint64 positionMillis READ positionMillis NOTIFY statusChanged)
    Q_PROPERTY(qint64 durationMillis READ durationMillis NOTIFY statusChanged)
    Q_PROPERTY(int sampleRateHz READ sampleRateHz NOTIFY statusChanged)
    Q_PROPERTY(int channels READ channels NOTIFY statusChanged)
    Q_PROPERTY(QString identity READ identity NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)

  public:
    explicit AudioPreviewController(DesktopBackend& backend, QObject* parent = nullptr);

    [[nodiscard]] bool hasAudio() const;
    [[nodiscard]] bool playing() const;
    [[nodiscard]] qint64 positionMillis() const;
    [[nodiscard]] qint64 durationMillis() const;
    [[nodiscard]] int sampleRateHz() const;
    [[nodiscard]] int channels() const;
    [[nodiscard]] QString identity() const;
    [[nodiscard]] QString errorCode() const;

    Q_INVOKABLE bool loadPreview(
        const QString& artifactId,
        const QString& candidateId = QString(),
        const QString& revisionId = QString()
    );
    Q_INVOKABLE void togglePlayback();
    Q_INVOKABLE void seekTo(qint64 positionMillis);
    Q_INVOKABLE void clear();

  signals:
    void statusChanged();

  private:
    void setErrorCode(const QString& code);

    DesktopBackend& backend_;
    QMediaPlayer player_;
    QAudioOutput audio_output_;
    QBuffer buffer_;
    QByteArray wav_bytes_;
    QString identity_;
    QString error_code_;
    qint64 duration_millis_ = 0;
    int sample_rate_hz_ = 0;
    int channels_ = 0;
    bool has_audio_ = false;
};

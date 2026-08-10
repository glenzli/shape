#include "audio_preview_controller.hpp"

#include "desktop_backend.hpp"

#include <QUrl>

#include <algorithm>
#include <utility>

AudioPreviewController::AudioPreviewController(DesktopBackend& backend, QObject* parent)
    : QObject(parent), backend_(backend), player_(this), audio_output_(this), buffer_(this) {
    audio_output_.setVolume(0.85F);
    player_.setAudioOutput(&audio_output_);
    connect(&player_, &QMediaPlayer::playbackStateChanged, this, [this]() {
        emit statusChanged();
    });
    connect(&player_, &QMediaPlayer::positionChanged, this, [this]() { emit statusChanged(); });
    connect(
        &player_,
        &QMediaPlayer::errorOccurred,
        this,
        [this](QMediaPlayer::Error error, const QString&) {
            if (error != QMediaPlayer::NoError) {
                setErrorCode(QStringLiteral("playback_failed"));
            }
        }
    );
}

bool AudioPreviewController::hasAudio() const {
    return has_audio_;
}

bool AudioPreviewController::playing() const {
    return player_.isPlaying();
}

qint64 AudioPreviewController::positionMillis() const {
    return player_.position();
}

qint64 AudioPreviewController::durationMillis() const {
    return duration_millis_;
}

int AudioPreviewController::sampleRateHz() const {
    return sample_rate_hz_;
}

int AudioPreviewController::channels() const {
    return channels_;
}

QString AudioPreviewController::identity() const {
    return identity_;
}

QString AudioPreviewController::errorCode() const {
    return error_code_;
}

bool AudioPreviewController::loadPreview(const QString& artifactId, const QString& candidateId) {
    const auto preview = backend_.audioPreview(artifactId, candidateId);
    if (!preview.has_value()) {
        setErrorCode(QStringLiteral("preview_unavailable"));
        return false;
    }
    if (preview->identity == identity_ && has_audio_) {
        error_code_.clear();
        emit statusChanged();
        return true;
    }
    if (preview->wav_bytes.size() < 44 || !preview->wav_bytes.startsWith("RIFF")
        || preview->wav_bytes.mid(8, 4) != QByteArrayLiteral("WAVE")
        || preview->duration_millis <= 0 || preview->sample_rate_hz <= 0
        || preview->channels <= 0) {
        setErrorCode(QStringLiteral("preview_invalid"));
        return false;
    }

    player_.stop();
    player_.setSourceDevice(nullptr);
    buffer_.close();
    wav_bytes_ = std::move(preview->wav_bytes);
    buffer_.setData(wav_bytes_);
    if (!buffer_.open(QIODevice::ReadOnly)) {
        clear();
        setErrorCode(QStringLiteral("preview_unavailable"));
        return false;
    }
    identity_ = preview->identity;
    duration_millis_ = preview->duration_millis;
    sample_rate_hz_ = preview->sample_rate_hz;
    channels_ = preview->channels;
    has_audio_ = true;
    error_code_.clear();
    player_.setSourceDevice(&buffer_, QUrl(QStringLiteral("memory://shape/%1.wav").arg(identity_)));
    emit statusChanged();
    return true;
}

void AudioPreviewController::togglePlayback() {
    if (!has_audio_) {
        setErrorCode(QStringLiteral("no_audio_preview"));
        return;
    }
    error_code_.clear();
    if (player_.isPlaying()) {
        player_.pause();
    } else {
        if (player_.mediaStatus() == QMediaPlayer::EndOfMedia) {
            player_.setPosition(0);
        }
        player_.play();
    }
    emit statusChanged();
}

void AudioPreviewController::seekTo(qint64 positionMillis) {
    if (!has_audio_) {
        return;
    }
    player_.setPosition(std::clamp(positionMillis, 0LL, duration_millis_));
}

void AudioPreviewController::clear() {
    player_.stop();
    player_.setSourceDevice(nullptr);
    buffer_.close();
    wav_bytes_.clear();
    identity_.clear();
    error_code_.clear();
    duration_millis_ = 0;
    sample_rate_hz_ = 0;
    channels_ = 0;
    has_audio_ = false;
    emit statusChanged();
}

void AudioPreviewController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

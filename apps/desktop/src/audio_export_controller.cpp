#include "audio_export_controller.hpp"
#include "desktop_backend.hpp"

#include <QDir>
#include <QFileInfo>
#include <QSaveFile>
#include <QtConcurrent/QtConcurrentRun>

AudioExportController::AudioExportController(DesktopBackend& backend, QObject* parent)
    : QObject(parent), backend_(backend) {
    connect(&watcher_, &QFutureWatcher<QString>::finished, this, [this]() {
        running_ = false;
        emit statusChanged();
        const QString error = watcher_.result();
        emit finished(error.isEmpty(), destination_, error);
    });
}

AudioExportController::~AudioExportController() {
    watcher_.waitForFinished();
}
bool AudioExportController::running() const {
    return running_;
}

void AudioExportController::exportAudio(
    const QString& artifactId,
    const QString& candidateId,
    const QUrl& destination
) {
    if (running_) {
        emit finished(false, QString(), QStringLiteral("export_busy"));
        return;
    }
    const QFileInfo target(destination.toLocalFile());
    if (!destination.isLocalFile() || !target.isAbsolute()
        || target.suffix().compare(QStringLiteral("wav"), Qt::CaseInsensitive) != 0
        || !target.dir().exists() || target.isSymLink() || target.isDir()) {
        emit finished(false, QString(), QStringLiteral("export_invalid_path"));
        return;
    }
    // Export never writes inside a project bundle, including via a directory symlink.
    QDir parent(target.dir().canonicalPath());
    const QString project = QFileInfo(backend_.bundlePath()).canonicalFilePath();
    do {
        if (parent.absolutePath() == project
            || parent.dirName().endsWith(QStringLiteral(".shape"), Qt::CaseInsensitive)) {
            emit finished(false, QString(), QStringLiteral("export_inside_project"));
            return;
        }
    } while (parent.cdUp());
    auto preview = backend_.audioPreview(artifactId, candidateId);
    if (!preview.has_value()) {
        emit finished(false, QString(), QStringLiteral("export_audio_unavailable"));
        return;
    }
    destination_ = target.absoluteFilePath();
    const QString path = destination_;
    running_ = true;
    emit statusChanged();
    watcher_.setFuture(QtConcurrent::run([path, bytes = std::move(preview->wav_bytes)]() {
        QSaveFile file(path);
        file.setDirectWriteFallback(false);
        if (!file.open(QIODevice::WriteOnly))
            return QStringLiteral("export_write_failed");
        if (file.write(bytes) != bytes.size()) {
            file.cancelWriting();
            return QStringLiteral("export_write_failed");
        }
        if (!file.commit())
            return QStringLiteral("export_write_failed");
        return QString();
    }));
}

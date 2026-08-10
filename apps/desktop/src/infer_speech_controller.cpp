#include "infer_speech_controller.hpp"

#include "desktop_backend.hpp"
#include "infer_controller_support.hpp"

#include <QMutexLocker>
#include <QtConcurrent/QtConcurrentRun>

#include <cstdint>
#include <optional>
#include <utility>

using infer_controller_support::stableErrorCode;
using infer_controller_support::toUtf8;

struct InferSpeechController::GenerationResult {
    quint64 generation = 0;
    QString source_artifact_id;
    QString error_code;
    std::optional<rust::Box<shape::desktop::InferSpeechCandidate>> candidate;
};

InferSpeechController::InferSpeechController(
    DesktopBackend& backend,
    QString credentialPath,
    QObject* parent
)
    : QObject(parent), backend_(backend), credential_path_(std::move(credentialPath)),
      explicit_override_(qEnvironmentVariable("SHAPE_INFER_RUNTIME_URL")) {
    connect(
        &watcher_,
        &QFutureWatcher<void>::finished,
        this,
        &InferSpeechController::finishGeneration
    );
}

InferSpeechController::~InferSpeechController() {
    if (watcher_.isRunning()) {
        watcher_.waitForFinished();
    }
}

bool InferSpeechController::running() const {
    return running_;
}

QString InferSpeechController::errorCode() const {
    return error_code_;
}

void InferSpeechController::generate(
    const QString& projectPath,
    const QString& sourceArtifactId,
    const QString& artifactName,
    int speedMilli
) {
    if (running_) {
        setErrorCode(QStringLiteral("generation_busy"));
        return;
    }
    if (projectPath.isEmpty() || sourceArtifactId.isEmpty() || artifactName.trimmed().isEmpty()
        || speedMilli < 250 || speedMilli > 4'000) {
        setErrorCode(QStringLiteral("invalid_speech_request"));
        return;
    }

    running_ = true;
    error_code_.clear();
    ++generation_;
    const quint64 request_generation = generation_;
    const QString credential_path = credential_path_;
    const QString explicit_override = explicit_override_;
    emit statusChanged();
    watcher_.setFuture(
        QtConcurrent::run([this,
                           request_generation,
                           projectPath,
                           sourceArtifactId,
                           artifactName,
                           speedMilli,
                           credential_path,
                           explicit_override]() mutable {
            GenerationResult result;
            result.generation = request_generation;
            result.source_artifact_id = sourceArtifactId;
            try {
                result.candidate.emplace(
                    shape::desktop::generate_infer_speech_candidate(
                        toUtf8(projectPath),
                        toUtf8(sourceArtifactId),
                        toUtf8(artifactName.trimmed()),
                        static_cast<std::uint16_t>(speedMilli),
                        toUtf8(credential_path),
                        toUtf8(explicit_override)
                    )
                );
            } catch (const rust::Error& error) {
                result.error_code = stableErrorCode(error);
            }
            const QMutexLocker lock(&result_mutex_);
            pending_result_ = std::make_unique<GenerationResult>(std::move(result));
        })
    );
}

void InferSpeechController::finishGeneration() {
    std::unique_ptr<GenerationResult> result;
    {
        const QMutexLocker lock(&result_mutex_);
        result = std::move(pending_result_);
    }
    if (result == nullptr || result->generation != generation_) {
        return;
    }

    running_ = false;
    if (!result->error_code.isEmpty()) {
        error_code_ = result->error_code;
        emit statusChanged();
        return;
    }
    if (!result->candidate.has_value()) {
        setErrorCode(QStringLiteral("generation_failed"));
        return;
    }
    try {
        const QString candidate_id =
            backend_.adoptInferSpeechCandidate(std::move(*result->candidate));
        if (candidate_id.isEmpty()) {
            setErrorCode(QStringLiteral("project_unavailable"));
            return;
        }
        error_code_.clear();
        emit statusChanged();
        emit candidateCreated(candidate_id, result->source_artifact_id);
    } catch (const rust::Error& error) {
        setErrorCode(stableErrorCode(error));
    }
}

void InferSpeechController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

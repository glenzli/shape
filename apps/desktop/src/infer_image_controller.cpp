#include "infer_image_controller.hpp"

#include "desktop_backend.hpp"
#include "infer_controller_support.hpp"

#include <QMutexLocker>
#include <QtConcurrent/QtConcurrentRun>

#include <optional>
#include <utility>

using infer_controller_support::stableErrorCode;
using infer_controller_support::toUtf8;

struct InferImageController::GenerationResult {
    quint64 generation = 0;
    QString error_code;
    std::optional<rust::Box<shape::desktop::InferImageBatch>> batch;
    int completed_count = 0;
    QString partial_error_code;
};

InferImageController::InferImageController(
    DesktopBackend& backend,
    QString credentialPath,
    QObject* parent
)
    : QObject(parent), backend_(backend), credential_path_(std::move(credentialPath)),
      explicit_override_(qEnvironmentVariable("SHAPE_INFER_RUNTIME_URL")) {
    progress_timer_.setInterval(100);
    connect(&progress_timer_, &QTimer::timeout, this, &InferImageController::drainReadyCandidates);
    connect(
        &watcher_,
        &QFutureWatcher<void>::finished,
        this,
        &InferImageController::finishGeneration
    );
}

InferImageController::~InferImageController() {
    if (watcher_.isRunning()) {
        watcher_.waitForFinished();
    }
}

bool InferImageController::running() const {
    return running_;
}

QString InferImageController::errorCode() const {
    return error_code_;
}

void InferImageController::generate(
    const QString& projectPath,
    const QString& artifactId,
    const QString& draftId,
    const QString& modelKey,
    const QString& effortKey,
    int candidateCount
) {
    if (running_) {
        setErrorCode(QStringLiteral("generation_busy"));
        return;
    }
    if (projectPath.isEmpty() || artifactId.isEmpty() || draftId.isEmpty()
        || candidateCount < 1 || candidateCount > 4) {
        setErrorCode(QStringLiteral("invalid_image_request"));
        return;
    }

    running_ = true;
    requested_count_ = candidateCount;
    completed_count_ = 0;
    stop_requested_ = false;
    adoption_error_code_.clear();
    active_artifact_id_ = artifactId;
    control_.emplace(shape::desktop::new_image_generation_control());
    error_code_.clear();
    ++generation_;
    const quint64 request_generation = generation_;
    const QString credential_path = credential_path_;
    const QString explicit_override = explicit_override_;
    emit statusChanged();
    progress_timer_.start();
    watcher_.setFuture(
        QtConcurrent::run([this,
                           request_generation,
                           projectPath,
                           artifactId,
                           draftId,
                           modelKey,
                           effortKey,
                           candidateCount,
                           credential_path,
                           explicit_override]() mutable {
            GenerationResult result;
            result.generation = request_generation;
            try {
                result.batch.emplace(
                    shape::desktop::generate_infer_image_batch_controlled(
                        toUtf8(projectPath),
                        toUtf8(artifactId),
                        toUtf8(draftId),
                        toUtf8(credential_path),
                        toUtf8(explicit_override),
                        toUtf8(modelKey),
                        toUtf8(effortKey),
                        static_cast<std::uint8_t>(candidateCount),
                        **control_
                    )
                );
                result.completed_count = shape::desktop::infer_image_batch_completed_count(
                    **result.batch
                );
                const auto failure = shape::desktop::infer_image_batch_failure_code(**result.batch);
                result.partial_error_code = QString::fromUtf8(failure.data(), failure.size());
            } catch (const rust::Error& error) {
                result.error_code = stableErrorCode(error);
            }
            const QMutexLocker lock(&result_mutex_);
            pending_result_ = std::make_unique<GenerationResult>(std::move(result));
        })
    );
}

void InferImageController::stopAfterCurrent() {
    if (!running_ || stop_requested_ || !control_.has_value()) {
        return;
    }
    stop_requested_ = true;
    shape::desktop::image_generation_control_cancel(**control_);
    emit statusChanged();
}

void InferImageController::drainReadyCandidates() {
    if (!control_.has_value() || !adoption_error_code_.isEmpty()) {
        return;
    }
    try {
        while (shape::desktop::image_generation_control_has_candidate(**control_)) {
            auto candidate = shape::desktop::image_generation_control_take_candidate(**control_);
            const QString candidate_id = backend_.adoptInferImageCandidate(std::move(candidate));
            if (candidate_id.isEmpty()) {
                adoption_error_code_ = QStringLiteral("project_unavailable");
                shape::desktop::image_generation_control_cancel(**control_);
                break;
            }
            ++completed_count_;
            emit statusChanged();
            emit candidateCreated(candidate_id, active_artifact_id_);
        }
    } catch (const rust::Error& error) {
        adoption_error_code_ = stableErrorCode(error);
        shape::desktop::image_generation_control_cancel(**control_);
    }
}

void InferImageController::finishGeneration() {
    progress_timer_.stop();
    drainReadyCandidates();
    std::unique_ptr<GenerationResult> result;
    {
        const QMutexLocker lock(&result_mutex_);
        result = std::move(pending_result_);
    }
    if (result == nullptr || result->generation != generation_) {
        return;
    }

    running_ = false;
    control_.reset();
    if (!adoption_error_code_.isEmpty()) {
        error_code_ = adoption_error_code_;
        emit statusChanged();
        return;
    }
    if (!result->error_code.isEmpty()) {
        error_code_ = result->error_code;
        emit statusChanged();
        return;
    }
    if (!result->batch.has_value()) {
        setErrorCode(QStringLiteral("generation_failed"));
        return;
    }
    if (completed_count_ != result->completed_count) {
        setErrorCode(QStringLiteral("project_unavailable"));
        return;
    }
    error_code_ = result->partial_error_code.isEmpty()
                      ? QString()
                      : result->partial_error_code == QStringLiteral("generation_cancelled")
                        ? QStringLiteral("generation_cancelled")
                        : QStringLiteral("partial_generation_failed");
    emit statusChanged();
}

void InferImageController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

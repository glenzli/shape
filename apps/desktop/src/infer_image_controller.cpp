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
    QString artifact_id;
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
                           artifactId,
                           draftId,
                           modelKey,
                           effortKey,
                           candidateCount,
                           credential_path,
                           explicit_override]() mutable {
            GenerationResult result;
            result.generation = request_generation;
            result.artifact_id = artifactId;
            try {
                result.batch.emplace(
                    shape::desktop::generate_infer_image_batch(
                        toUtf8(projectPath),
                        toUtf8(artifactId),
                        toUtf8(draftId),
                        toUtf8(credential_path),
                        toUtf8(explicit_override),
                        toUtf8(modelKey),
                        toUtf8(effortKey),
                        static_cast<std::uint8_t>(candidateCount)
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

void InferImageController::finishGeneration() {
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
    if (!result->batch.has_value()) {
        setErrorCode(QStringLiteral("generation_failed"));
        return;
    }
    try {
        const QStringList candidate_ids = backend_.adoptInferImageBatch(std::move(*result->batch));
        if (candidate_ids.isEmpty()) {
            setErrorCode(QStringLiteral("project_unavailable"));
            return;
        }
        completed_count_ = result->completed_count;
        error_code_ = result->partial_error_code.isEmpty()
                          ? QString() : QStringLiteral("partial_generation_failed");
        emit statusChanged();
        emit candidateCreated(candidate_ids.last(), result->artifact_id);
    } catch (const rust::Error& error) {
        setErrorCode(stableErrorCode(error));
    }
}

void InferImageController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

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
    std::optional<rust::Box<shape::desktop::InferImageCandidate>> candidate;
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
    const QString& draftId
) {
    if (running_) {
        setErrorCode(QStringLiteral("generation_busy"));
        return;
    }
    if (projectPath.isEmpty() || artifactId.isEmpty() || draftId.isEmpty()) {
        setErrorCode(QStringLiteral("invalid_image_request"));
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
                           artifactId,
                           draftId,
                           credential_path,
                           explicit_override]() mutable {
            GenerationResult result;
            result.generation = request_generation;
            result.artifact_id = artifactId;
            try {
                result.candidate.emplace(
                    shape::desktop::generate_infer_image_candidate(
                        toUtf8(projectPath),
                        toUtf8(artifactId),
                        toUtf8(draftId),
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
    if (!result->candidate.has_value()) {
        setErrorCode(QStringLiteral("generation_failed"));
        return;
    }
    try {
        const QString candidate_id =
            backend_.adoptInferImageCandidate(std::move(*result->candidate));
        if (candidate_id.isEmpty()) {
            setErrorCode(QStringLiteral("project_unavailable"));
            return;
        }
        error_code_.clear();
        emit statusChanged();
        emit candidateCreated(candidate_id, result->artifact_id);
    } catch (const rust::Error& error) {
        setErrorCode(stableErrorCode(error));
    }
}

void InferImageController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

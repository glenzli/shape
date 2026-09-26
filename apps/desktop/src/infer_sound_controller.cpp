#include "infer_sound_controller.hpp"

#include "desktop_backend.hpp"
#include "infer_controller_support.hpp"

#include <QMutexLocker>
#include <QtConcurrent/QtConcurrentRun>

#include <optional>
#include <string>
#include <utility>

using infer_controller_support::stableErrorCode;
using infer_controller_support::toUtf8;

struct InferSoundController::GenerationResult {
    quint64 generation = 0;
    QString artifact_id;
    QString error_code;
    std::optional<rust::Box<shape::desktop::InferSoundCandidate>> candidate;
};

InferSoundController::InferSoundController(
    DesktopBackend& backend,
    QString credentialPath,
    QObject* parent
)
    : QObject(parent), control_(shape::desktop::new_sound_control()), backend_(backend),
      credential_path_(std::move(credentialPath)),
      explicit_override_(qEnvironmentVariable("SHAPE_INFER_RUNTIME_URL")) {
    stage_timer_.setInterval(200);
    connect(&stage_timer_, &QTimer::timeout, this, &InferSoundController::statusChanged);
    connect(&backend_, &DesktopBackend::projectChanged, this, [this] {
        if (running_)
            stop();
    });
    connect(
        &watcher_,
        &QFutureWatcher<void>::finished,
        this,
        &InferSoundController::finishGeneration
    );
}

InferSoundController::~InferSoundController() {
    shape::desktop::sound_control_cancel(*control_);
    if (watcher_.isRunning())
        watcher_.waitForFinished();
}

bool InferSoundController::running() const {
    return running_;
}
QString InferSoundController::errorCode() const {
    return error_code_;
}

int InferSoundController::stage() const {
    return shape::desktop::sound_control_stage(*control_);
}
void InferSoundController::stop() {
    if (running_) {
        stopped_ = true;
        shape::desktop::sound_control_cancel(*control_);
    }
}

void InferSoundController::generate(
    const QString& projectPath,
    const QString& artifactId,
    const QString& draftId
) {
    if (running_) {
        setErrorCode(QStringLiteral("generation_busy"));
        return;
    }
    if (projectPath.isEmpty() || artifactId.isEmpty() || draftId.isEmpty()) {
        setErrorCode(QStringLiteral("invalid_prompt"));
        return;
    }
    shape::desktop::sound_control_resume(*control_);
    stage_timer_.start();
    stopped_ = false;
    running_ = true;
    error_code_.clear();
    const quint64 request_generation = ++generation_;
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
                    shape::desktop::generate_infer_sound_candidate(
                        toUtf8(projectPath),
                        toUtf8(artifactId),
                        toUtf8(draftId),
                        toUtf8(credential_path),
                        toUtf8(explicit_override),
                        *control_
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

void InferSoundController::finishGeneration() {
    std::unique_ptr<GenerationResult> result;
    {
        const QMutexLocker lock(&result_mutex_);
        result = std::move(pending_result_);
    }
    if (result == nullptr || result->generation != generation_)
        return;
    running_ = false;
    stage_timer_.stop();
    if (stopped_) {
        setErrorCode(QStringLiteral("generation_cancelled"));
        return;
    }
    if (!result->error_code.isEmpty()) {
        setErrorCode(result->error_code);
        return;
    }
    if (!result->candidate.has_value()) {
        setErrorCode(QStringLiteral("generation_failed"));
        return;
    }
    try {
        const QString candidate_id =
            backend_.adoptInferSoundCandidate(std::move(*result->candidate));
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

void InferSoundController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

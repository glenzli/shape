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
namespace {
QString fromRust(const rust::String& value) {
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}
} // namespace

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
      explicit_override_(qEnvironmentVariable("SHAPE_INFER_RUNTIME_URL")),
      control_(shape::desktop::new_speech_control()) {
    progress_timer_.setInterval(200);
    connect(&progress_timer_, &QTimer::timeout, this, &InferSpeechController::statusChanged);
    connect(
        &watcher_,
        &QFutureWatcher<void>::finished,
        this,
        &InferSpeechController::finishGeneration
    );
}

InferSpeechController::~InferSpeechController() {
    if (watcher_.isRunning()) {
        shape::desktop::speech_control_cancel(*control_);
        watcher_.waitForFinished();
    }
}

QVariantList InferSpeechController::presets() const {
    QVariantList result;
    for (const auto& preset : shape::desktop::speech_presets()) {
        result.append(
            QVariantMap{
                {QStringLiteral("key"), fromRust(preset.key)},
                {QStringLiteral("alias"), fromRust(preset.alias)},
                {QStringLiteral("language"), fromRust(preset.language)},
                {QStringLiteral("catalogRevision"), fromRust(preset.catalog_revision)}
            }
        );
    }
    return result;
}
int InferSpeechController::completedSegments() const {
    return static_cast<int>(shape::desktop::speech_control_completed(*control_));
}
int InferSpeechController::totalSegments() const {
    return static_cast<int>(shape::desktop::speech_control_total(*control_));
}
bool InferSpeechController::cancelling() const {
    return cancelling_;
}
void InferSpeechController::cancel() {
    if (!running_)
        return;
    cancelling_ = true;
    shape::desktop::speech_control_cancel(*control_);
    emit statusChanged();
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
    const QString& draftId,
    const QString& artifactName
) {
    if (running_) {
        setErrorCode(QStringLiteral("generation_busy"));
        return;
    }
    if (projectPath.isEmpty() || sourceArtifactId.isEmpty() || draftId.isEmpty()
        || artifactName.trimmed().isEmpty()) {
        setErrorCode(QStringLiteral("invalid_speech_request"));
        return;
    }

    shape::desktop::speech_control_resume(*control_);
    cancelling_ = false;
    progress_timer_.start();
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
                           draftId,
                           artifactName,
                           credential_path,
                           explicit_override]() mutable {
            GenerationResult result;
            result.generation = request_generation;
            result.source_artifact_id = sourceArtifactId;
            try {
                result.candidate.emplace(
                    shape::desktop::generate_infer_speech_candidate_controlled(
                        toUtf8(projectPath),
                        toUtf8(sourceArtifactId),
                        toUtf8(draftId),
                        toUtf8(artifactName.trimmed()),
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

void InferSpeechController::finishGeneration() {
    std::unique_ptr<GenerationResult> result;
    {
        const QMutexLocker lock(&result_mutex_);
        result = std::move(pending_result_);
    }
    if (result == nullptr || result->generation != generation_) {
        return;
    }

    progress_timer_.stop();
    cancelling_ = false;
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
        emit candidateCreated(candidate_id, backend_.candidateArtifactId());
    } catch (const rust::Error& error) {
        setErrorCode(stableErrorCode(error));
    }
}

void InferSpeechController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

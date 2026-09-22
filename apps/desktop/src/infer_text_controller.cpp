#include "infer_text_controller.hpp"

#include "desktop_backend.hpp"
#include "infer_controller_support.hpp"

#include <QMutexLocker>
#include <QtConcurrent/QtConcurrentRun>

#include <algorithm>
#include <optional>
#include <string>
#include <utility>

using infer_controller_support::stableErrorCode;
using infer_controller_support::toUtf8;

struct InferTextController::GenerationResult {
    quint64 generation = 0;
    QString artifact_id;
    QString error_code;
    std::optional<rust::Box<shape::desktop::InferTextCandidate>> candidate;
};

InferTextController::InferTextController(
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
        &InferTextController::finishGeneration
    );
    refreshCredentialStatus();
}

InferTextController::~InferTextController() {
    if (watcher_.isRunning()) {
        watcher_.waitForFinished();
    }
}

bool InferTextController::running() const {
    return running_;
}

bool InferTextController::credentialConfigured() const {
    return credential_configured_;
}

QString InferTextController::errorCode() const {
    return error_code_;
}

void InferTextController::generate(
    const QString& projectPath,
    const QString& artifactId,
    const QString& draftId,
    const QString& modelKey
) {
    if (running_) {
        setErrorCode(QStringLiteral("generation_busy"));
        return;
    }
    if (!credential_configured_) {
        setErrorCode(QStringLiteral("credential_missing"));
        return;
    }
    if (projectPath.isEmpty() || artifactId.isEmpty() || draftId.isEmpty()) {
        setErrorCode(QStringLiteral("invalid_prompt"));
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
                           modelKey,
                           credential_path,
                           explicit_override]() mutable {
            GenerationResult result;
            result.generation = request_generation;
            result.artifact_id = artifactId;
            try {
                result.candidate.emplace(
                    shape::desktop::generate_infer_text_candidate(
                        toUtf8(projectPath),
                        toUtf8(artifactId),
                        toUtf8(draftId),
                        toUtf8(credential_path),
                        toUtf8(explicit_override),
                        toUtf8(modelKey)
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

bool InferTextController::installCredential(const QString& token) {
    if (running_ || token.isEmpty()) {
        setErrorCode(
            running_ ? QStringLiteral("generation_busy") : QStringLiteral("credential_invalid")
        );
        return false;
    }
    QByteArray token_bytes = token.toUtf8();
    std::string token_utf8(token_bytes.constData(), static_cast<std::size_t>(token_bytes.size()));
    try {
        shape::desktop::install_infer_runtime_credential(toUtf8(credential_path_), token_utf8);
        std::fill(token_utf8.begin(), token_utf8.end(), '\0');
        token_bytes.fill('\0');
        credential_configured_ = true;
        error_code_.clear();
        emit statusChanged();
        return true;
    } catch (const rust::Error& error) {
        std::fill(token_utf8.begin(), token_utf8.end(), '\0');
        token_bytes.fill('\0');
        credential_configured_ = false;
        setErrorCode(stableErrorCode(error));
        return false;
    }
}

void InferTextController::refreshCredentialStatus() {
    const shape::desktop::InferRuntimeCredentialStatusWire status =
        shape::desktop::infer_runtime_credential_status(toUtf8(credential_path_));
    credential_configured_ = status.configured;
    error_code_ = QString::fromUtf8(
        status.error_code.data(),
        static_cast<qsizetype>(status.error_code.size())
    );
    emit statusChanged();
}

void InferTextController::finishGeneration() {
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
            backend_.adoptInferTextCandidate(std::move(*result->candidate));
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

void InferTextController::setErrorCode(const QString& code) {
    error_code_ = code;
    emit statusChanged();
}

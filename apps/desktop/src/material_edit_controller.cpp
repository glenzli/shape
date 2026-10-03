#include "material_edit_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_controller_support.hpp"
#include <QDir>
#include <QMutexLocker>
#include <QUrl>
#include <QUuid>
#include <QtConcurrent/QtConcurrentRun>
#include <optional>
#include <utility>

namespace {
QString localPath(const QString& input) {
    if (QDir::isAbsolutePath(input))
        return input;
    const QUrl url(input);
    return url.isLocalFile() ? url.toLocalFile() : QString();
}
QString materialErrorCode(const rust::Error& error) {
    const QString code = QString::fromUtf8(error.what());
    if (code == "material_session_changed" || code == "material_unchanged"
        || code == "material_file_unavailable")
        return code;
    return QStringLiteral("material_return_invalid");
}
} // namespace

struct MaterialEditController::Result {
    quint64 generation;
    QString error, manifest;
    std::optional<rust::Box<shape::desktop::PreparedMaterialReturn>> candidate;
};

MaterialEditController::MaterialEditController(DesktopBackend& backend, QObject* parent)
    : QObject(parent), backend_(backend) {
    connect(&watcher_, &QFutureWatcher<void>::finished, this, &MaterialEditController::finish);
}
MaterialEditController::~MaterialEditController() {
    watcher_.waitForFinished();
}

void MaterialEditController::exportCopy(
    const QString& project,
    const QString& output,
    const QString& parent
) {
    const auto folder = localPath(parent);
    if (folder.isEmpty())
        return;
    const auto directory = QDir(folder).filePath(
        QStringLiteral("shape-edit-%1").arg(QUuid::createUuid().toString(QUuid::WithoutBraces))
    );
    start(project, output, directory, {}, true);
}
void MaterialEditController::review(
    const QString& project,
    const QString& output,
    const QString& handoff,
    const QString& returned,
    bool allowPcm24Conversion
) {
    start(project, output, localPath(handoff), localPath(returned), false, allowPcm24Conversion);
}

void MaterialEditController::start(
    const QString& project,
    const QString& output,
    const QString& first,
    const QString& second,
    bool exporting,
    bool allowPcm24Conversion
) {
    // A second stage must not replace the owner or feedback of the running operation.
    if (running_ || project != backend_.bundlePath() || output.isEmpty())
        return;
    project_path_ = project;
    artifact_id_ = output;
    error_code_.clear();
    handoff_path_.clear();
    QString ticket;
    try {
        ticket = backend_.materialEditTicket(output);
    } catch (const rust::Error& error) {
        error_code_ = materialErrorCode(error);
        emit statusChanged();
        return;
    }
    if (first.isEmpty() || (!exporting && second.isEmpty())) {
        error_code_ = QStringLiteral("material_file_unavailable");
        emit statusChanged();
        return;
    }
    running_ = true;
    const auto generation = ++generation_;
    emit statusChanged();
    watcher_.setFuture(
        QtConcurrent::run(
            [this, generation, project, ticket, first, second, exporting, allowPcm24Conversion] {
                auto result = std::make_unique<Result>();
                result->generation = generation;
                try {
                    using infer_controller_support::toUtf8;
                    if (exporting) {
                        const auto receipt = shape::desktop::export_material_copy(
                            toUtf8(project),
                            toUtf8(ticket),
                            toUtf8(first)
                        );
                        (void)receipt;
                        result->manifest = QDir(first).filePath(QStringLiteral("handoff.json"));
                    } else {
                        result->candidate.emplace(
                            shape::desktop::prepare_material_return(
                                toUtf8(project),
                                toUtf8(ticket),
                                toUtf8(first),
                                toUtf8(second),
                                allowPcm24Conversion
                            )
                        );
                    }
                } catch (const rust::Error& error) {
                    result->error = materialErrorCode(error);
                }
                QMutexLocker lock(&mutex_);
                pending_ = std::move(result);
            }
        )
    );
}

void MaterialEditController::finish() {
    std::unique_ptr<Result> result;
    {
        QMutexLocker lock(&mutex_);
        result = std::move(pending_);
    }
    running_ = false;
    if (!result || result->generation != generation_) {
        emit statusChanged();
        return;
    }
    if (backend_.bundlePath() != project_path_) {
        error_code_ = QStringLiteral("material_session_changed");
        emit statusChanged();
        return;
    }
    error_code_ = result->error;
    if (result->candidate && error_code_.isEmpty()) {
        try {
            const auto id = backend_.adoptMaterialReturn(std::move(*result->candidate));
            emit candidateCreated(id, artifact_id_);
        } catch (const rust::Error& error) {
            error_code_ = materialErrorCode(error);
        }
    }
    if (!result->manifest.isEmpty() && error_code_.isEmpty()) {
        handoff_path_ = result->manifest;
        emit copyCreated(handoff_path_, artifact_id_);
    }
    emit statusChanged();
}

bool MaterialEditController::restart(const QString& project, const QString& output, bool refresh) {
    if (project != backend_.bundlePath())
        return false;
    const bool owns = project_path_ == project && artifact_id_ == output;
    try {
        backend_.restartMaterialEdit(backend_.materialEditTicket(output), refresh);
    } catch (const rust::Error& error) {
        if (owns || !running_) {
            if (!running_) {
                project_path_ = project;
                artifact_id_ = output;
            }
            error_code_ = materialErrorCode(error);
            emit statusChanged();
        }
        return false;
    }
    if (owns || !running_) {
        ++generation_;
        project_path_ = project;
        artifact_id_ = output;
        handoff_path_.clear();
        error_code_ = QStringLiteral("material_session_restarted");
        emit statusChanged();
    }
    return true;
}

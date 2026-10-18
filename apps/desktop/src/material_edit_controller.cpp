#include "material_edit_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_controller_support.hpp"
#include <QCryptographicHash>
#include <QDir>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMutexLocker>
#include <QProcess>
#include <QSettings>
#include <QStandardPaths>
#include <QStringList>
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
    QString error, manifest, ticket;
    std::optional<rust::Box<shape::desktop::PreparedMaterialReturn>> candidate;
};

MaterialEditController::MaterialEditController(DesktopBackend& backend, QObject* parent)
    : MaterialEditController(backend, QString(), parent) {}
MaterialEditController::MaterialEditController(
    DesktopBackend& backend,
    const QString& settingsPath,
    QObject* parent
)
    : QObject(parent), backend_(backend),
      settings_(
          settingsPath.isEmpty() ? std::make_unique<QSettings>()
                                 : std::make_unique<QSettings>(settingsPath, QSettings::IniFormat)
      ) {
    connect(&watcher_, &QFutureWatcher<void>::finished, this, &MaterialEditController::finish);
    shadow_executable_ =
        settings_->value(QStringLiteral("material-edit/shadowExecutable")).toString();
    shadow_poll_.setInterval(500);
    connect(&shadow_poll_, &QTimer::timeout, this, &MaterialEditController::inspectShadowResult);
}
MaterialEditController::~MaterialEditController() {
    shadow_poll_.stop();
    watcher_.waitForFinished();
}

QString MaterialEditController::shadowExecutable() const {
    const auto resolve = [](const QString& path) {
        QFileInfo info(path);
#ifdef Q_OS_MACOS
        if (info.isDir() && info.suffix() == QStringLiteral("app"))
            info.setFile(QDir(path).filePath(QStringLiteral("Contents/MacOS/Shadow")));
#endif
        return info.isAbsolute() && info.isFile() && info.isExecutable() ? info.canonicalFilePath()
                                                                         : QString();
    };
    // An explicit selection fails closed if it disappears; never launch a different editor.
    if (!shadow_executable_.isEmpty())
        return resolve(shadow_executable_);
    const auto environment = qEnvironmentVariable("SHAPE_SHADOW_BINARY");
    if (!environment.isEmpty())
        return resolve(environment);
    const auto executable = QStandardPaths::findExecutable(QStringLiteral("Shadow"));
    if (!executable.isEmpty())
        return resolve(executable);
#ifdef Q_OS_MACOS
    return resolve(QStringLiteral("/Applications/Shadow.app"));
#else
    return {};
#endif
}

bool MaterialEditController::setShadowExecutable(const QUrl& executable) {
    if (running_ || shadow_active_ || !executable.isLocalFile())
        return false;
    const auto previous = shadow_executable_;
    shadow_executable_ = executable.toLocalFile();
    const auto resolved = shadowExecutable();
    if (resolved.isEmpty()) {
        shadow_executable_ = previous;
        error_code_ = QStringLiteral("material_editor_unavailable");
        emit statusChanged();
        return false;
    }
    shadow_executable_ = resolved;
    settings_->setValue(QStringLiteral("material-edit/shadowExecutable"), resolved);
    emit statusChanged();
    return true;
}

void MaterialEditController::openShadow(const QString& project, const QString& output) {
    if (running_ || shadow_active_ || project != backend_.bundlePath() || output.isEmpty())
        return;
    project_path_ = project;
    artifact_id_ = output;
    if (shadowExecutable().isEmpty()) {
        error_code_ = QStringLiteral("material_editor_unavailable");
        emit statusChanged();
        return;
    }
    launch_after_export_ = true;
    start(
        project,
        output,
        QDir::temp().filePath(QStringLiteral("shape-shadow-%1")
                                  .arg(QUuid::createUuid().toString(QUuid::WithoutBraces))),
        {},
        true
    );
    if (!running_)
        launch_after_export_ = false;
}
QString
MaterialEditController::shadowSessionKey(const QString& project, const QString& output) const {
    const auto bundle = QFileInfo(project).canonicalFilePath();
    const auto digest =
        QCryptographicHash::hash(bundle.toUtf8(), QCryptographicHash::Sha256).toHex();
    return QStringLiteral("material-edit/shadow-sessions/by-bundle/%1/%2")
        .arg(QString::fromLatin1(digest), output);
}
void MaterialEditController::saveShadowSession() {
    const QJsonObject record{
        {"schema", "shape.shadow-local-session/1"},
        {"project_path", QFileInfo(project_path_).canonicalFilePath()},
        {"project_id", backend_.projectId()},
        {"output", artifact_id_},
        {"ticket", shadow_ticket_},
        {"handoff", handoff_path_},
        {"result", shadow_result_}
    };
    settings_->setValue(
        shadowSessionKey(project_path_, artifact_id_),
        QString::fromUtf8(QJsonDocument(record).toJson(QJsonDocument::Compact))
    );
    settings_->sync();
}
void MaterialEditController::clearShadowSession() {
    const auto project_id =
        QJsonDocument::fromJson(shadow_ticket_.toUtf8()).object()["project_id"].toString();
    const QStringList keys{
        shadowSessionKey(project_path_, artifact_id_),
        QStringLiteral("material-edit/shadow-sessions/%1/%2").arg(project_id, artifact_id_)
    };
    const auto bundle = QFileInfo(project_path_).canonicalFilePath();
    for (const auto& key : keys) {
        const auto record =
            QJsonDocument::fromJson(settings_->value(key).toString().toUtf8()).object();
        if (!bundle.isEmpty() && record["ticket"].toString() == shadow_ticket_
            && QFileInfo(record["project_path"].toString()).canonicalFilePath() == bundle)
            settings_->remove(key);
    }
    settings_->sync();
}
bool MaterialEditController::restoreShadowSession(const QString& project, const QString& output) {
    if (project != backend_.bundlePath() || output.isEmpty())
        return false;
    if (running_ || shadow_active_)
        return project_path_ == project && artifact_id_ == output;
    settings_->sync();
    auto saved = settings_->value(shadowSessionKey(project, output)).toString();
    const bool legacy = saved.isEmpty();
    if (legacy)
        saved = settings_
                    ->value(QStringLiteral("material-edit/shadow-sessions/%1/%2")
                                .arg(backend_.projectId(), output))
                    .toString();
    const auto record = QJsonDocument::fromJson(saved.toUtf8()).object();
    if (record["schema"] != "shape.shadow-local-session/1"
        || QFileInfo(record["project_path"].toString()).canonicalFilePath()
               != QFileInfo(project).canonicalFilePath()
        || record["project_id"].toString() != backend_.projectId()
        || record["output"].toString() != output)
        return false;
    try {
        const auto ticket = backend_.materialEditTicket(output);
        if (record["ticket"].toString() != ticket)
            return false;
        const auto handoff = record["handoff"].toString(), result = record["result"].toString();
        const QFileInfo manifest(handoff);
        if (!manifest.isAbsolute() || !manifest.isFile() || manifest.isSymLink()
            || manifest.fileName() != "handoff.json"
            || result != manifest.dir().filePath(QStringLiteral("shadow-result.json")))
            return false;
        project_path_ = project;
        artifact_id_ = output;
        handoff_path_ = handoff;
        shadow_ticket_ = ticket;
        shadow_result_ = result;
        shadow_generation_ = ++generation_;
        shadow_active_ = true;
        error_code_.clear();
        if (legacy)
            saveShadowSession();
        shadow_poll_.start();
        emit copyCreated(handoff, output);
        emit statusChanged();
        // The core rechecks manifest ticket, original bytes and native receipt before any
        // candidate.
        QTimer::singleShot(0, this, &MaterialEditController::inspectShadowResult);
        return true;
    } catch (const rust::Error&) {
        return false;
    }
}

void MaterialEditController::launchShadow() {
    try {
        shadow_ticket_ = backend_.materialEditTicket(artifact_id_);
        shadow_generation_ = generation_;
        const QDir folder = QFileInfo(handoff_path_).dir();
        shadow_result_ = folder.filePath(QStringLiteral("shadow-result.json"));
        const auto request = folder.filePath(QStringLiteral("shadow-request.json"));
        if (!QFileInfo::exists(request)) {
            error_code_ = QStringLiteral("material_editor_unavailable");
            return;
        }
        // The independent editor must survive Shape closing or cancelling its wait.
        // This unparented process deletes itself only after exit; destroying the controller
        // disconnects observation without destroying QProcess (which would kill the editor).
        auto* process = new QProcess;
        process->setStandardOutputFile(QProcess::nullDevice());
        process->setStandardErrorFile(folder.filePath(QStringLiteral("shadow-stderr.log")));
        const auto generation = shadow_generation_;
        connect(
            process,
            qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
            process,
            &QObject::deleteLater
        );
        connect(
            process,
            &QProcess::errorOccurred,
            process,
            [process](QProcess::ProcessError error) {
                if (error == QProcess::FailedToStart)
                    process->deleteLater();
            }
        );
        const auto ended = [this, generation](bool couldStart) {
            if (!shadow_active_ || generation != shadow_generation_ || generation != generation_)
                return;
            if (QFileInfo::exists(shadow_result_)) {
                inspectShadowResult();
                return;
            }
            shadow_poll_.stop();
            shadow_active_ = false;
            clearShadowSession();
            error_code_ = couldStart ? QStringLiteral("material_editor_incomplete")
                                     : QStringLiteral("material_editor_unavailable");
            emit statusChanged();
        };
        connect(
            process,
            qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
            this,
            [ended](int, QProcess::ExitStatus) { ended(true); }
        );
        connect(process, &QProcess::errorOccurred, this, [ended](QProcess::ProcessError error) {
            if (error == QProcess::FailedToStart)
                ended(false);
        });
        shadow_active_ = true;
        saveShadowSession();
        shadow_poll_.start();
        process->start(
            shadowExecutable(),
            {QStringLiteral("--pipeline-edit"),
             QStringLiteral("--request"),
             request,
             QStringLiteral("--result"),
             shadow_result_},
            QIODevice::NotOpen
        );
    } catch (const rust::Error& error) {
        error_code_ = materialErrorCode(error);
    }
}

void MaterialEditController::inspectShadowResult() {
    try {
        if (shadow_generation_ != generation_ || backend_.bundlePath() != project_path_
            || backend_.materialEditTicket(artifact_id_) != shadow_ticket_) {
            shadow_poll_.stop();
            shadow_active_ = false;
            error_code_ = QStringLiteral("material_session_changed");
            emit statusChanged();
            return;
        }
        if (!QFileInfo::exists(shadow_result_))
            return;
        shadow_poll_.stop();
        shadow_active_ = false;
        // Completion still enters the ordinary strict receipt verifier and transient shelf.
        review(project_path_, artifact_id_, handoff_path_, shadow_result_);
    } catch (const rust::Error& error) {
        shadow_poll_.stop();
        shadow_active_ = false;
        error_code_ = materialErrorCode(error);
        emit statusChanged();
    }
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
    if (running_ || shadow_active_ || project != backend_.bundlePath() || output.isEmpty())
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
                result->ticket = ticket;
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
        launch_after_export_ = false;
        emit statusChanged();
        return;
    }
    if (backend_.bundlePath() != project_path_) {
        launch_after_export_ = false;
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
        try {
            if (backend_.materialEditTicket(artifact_id_) != result->ticket)
                error_code_ = QStringLiteral("material_session_changed");
            else {
                handoff_path_ = result->manifest;
                emit copyCreated(handoff_path_, artifact_id_);
                if (launch_after_export_)
                    launchShadow();
            }
        } catch (const rust::Error& error) {
            error_code_ = materialErrorCode(error);
        }
    }
    launch_after_export_ = false;
    emit statusChanged();
}

bool MaterialEditController::restart(const QString& project, const QString& output, bool refresh) {
    if (project != backend_.bundlePath())
        return false;
    const bool owns = project_path_ == project && artifact_id_ == output;
    try {
        backend_.restartMaterialEdit(backend_.materialEditTicket(output), refresh);
    } catch (const rust::Error& error) {
        if (owns || (!running_ && !shadow_active_)) {
            if (!running_ && !shadow_active_) {
                project_path_ = project;
                artifact_id_ = output;
            }
            error_code_ = materialErrorCode(error);
            emit statusChanged();
        }
        return false;
    }
    if (owns || (!running_ && !shadow_active_)) {
        ++generation_;
        if (owns) {
            clearShadowSession();
            shadow_poll_.stop();
            shadow_active_ = false;
            launch_after_export_ = false;
        }
        project_path_ = project;
        artifact_id_ = output;
        handoff_path_.clear();
        error_code_ = QStringLiteral("material_session_restarted");
        emit statusChanged();
    }
    return true;
}

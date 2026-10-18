#include "deck_material_task_controller.hpp"
#include "desktop_backend.hpp"
#include "infer_image_controller.hpp"
#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"
#include <QDir>
#include <QFileInfo>
#include <QJsonDocument>
#include <QSettings>
#include <QStandardPaths>
#include <QUrl>
#include <QtConcurrent/QtConcurrentRun>

namespace {
std::string utf8(const QString& value) {
    return value.toUtf8().toStdString();
}
QString fromRust(const rust::String& value) {
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}
QJsonObject object(const QString& value) {
    return QJsonDocument::fromJson(value.toUtf8()).object();
}
} // namespace
DeckMaterialTaskController::DeckMaterialTaskController(
    DesktopBackend& backend,
    InferImageController& image,
    QObject* parent
)
    : QObject(parent), backend_(backend), image_(image) {
    connect(
        &watcher_,
        &QFutureWatcher<QJsonObject>::finished,
        this,
        &DeckMaterialTaskController::finish
    );
    connect(&backend_, &DesktopBackend::projectChanged, this, &DeckMaterialTaskController::changed);
    connect(
        &image_,
        &InferImageController::statusChanged,
        this,
        &DeckMaterialTaskController::changed
    );
}
DeckMaterialTaskController::~DeckMaterialTaskController() {
    watcher_.waitForFinished();
}
bool DeckMaterialTaskController::currentProject() const {
    return !project_path_.isEmpty() && backend_.projectId() == project_id_
           && QFileInfo(backend_.bundlePath()).canonicalFilePath() == project_path_;
}
bool DeckMaterialTaskController::canReturn() const {
    if (!active_ || busy() || image_.running() || !currentProject())
        return false;
    for (const auto& item : backend_.artifacts()) {
        const auto artifact = item.toMap();
        if (artifact.value("id").toString() == artifact_id_
            && artifact.value("kindKey").toString() == "image_raster"
            && artifact.value("hasAcceptedRevision").toBool())
            return true;
    }
    return false;
}
void DeckMaterialTaskController::startupFailed(const QString& reason) {
    error_ = reason;
    if (!request_.isEmpty()) {
        try {
            shape::desktop::finish_deck_material_task(
                "",
                utf8(request_path_),
                utf8(result_path_),
                "",
                utf8(request_digest_),
                "failed",
                utf8(reason)
            );
        } catch (
            const rust::Error&
        ) { /* Invalid/previous result destinations are never overwritten. */
        }
    }
    active_ = false;
    emit changed();
}
bool DeckMaterialTaskController::initialize(
    const QString& requestPath,
    const QString& resultPath,
    const QString& taskRoot
) {
    if (active_ || busy() || !QDir::isAbsolutePath(requestPath)
        || !QDir::isAbsolutePath(resultPath))
        return false;
    request_ = {};
    request_digest_.clear();
    artifact_id_.clear();
    draft_id_.clear();
    project_path_.clear();
    project_id_.clear();
    error_.clear();
    request_path_ = requestPath;
    result_path_ = resultPath;
    try {
        const auto prepared = object(
            fromRust(shape::desktop::read_deck_material_task(utf8(requestPath), utf8(resultPath)))
        );
        request_ = prepared.value("request").toObject();
        request_digest_ = prepared.value("request_blake3").toString();
    } catch (const rust::Error&) {
        startupFailed(tr(
            "This image task or its new result location is invalid. Ask Deck to start a new task."
        ));
        return false;
    }
    const auto base = taskRoot.isEmpty()
                          ? QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation)
                                + "/deck-material-tasks"
                          : taskRoot;
    const auto directory = base + '/' + request_digest_;
    if (!QDir().mkpath(directory)) {
        startupFailed(QString::fromUtf8(QT_TR_NOOP("The image task folder could not be created.")));
        return false;
    }
    QSettings saved(directory + "/task.ini", QSettings::IniFormat);
    if (!saved.value("request_blake3").toString().isEmpty()
        && saved.value("request_blake3").toString() != request_digest_) {
        startupFailed(
            QString::fromUtf8(
                QT_TR_NOOP("The saved image task identity changed. Its files have been kept.")
            )
        );
        return false;
    }
    auto existing = saved.value("project").toString();
    const auto expectedProject = directory + "/Deck image.shape";
    if (existing.isEmpty() && QFileInfo(expectedProject).isDir())
        existing = expectedProject;
    if (!existing.isEmpty()) {
        if (QFileInfo(existing).canonicalFilePath()
                != QFileInfo(directory + "/Deck image.shape").canonicalFilePath()
            || !backend_.openProject(QUrl::fromLocalFile(existing))) {
            startupFailed(
                QString::fromUtf8(QT_TR_NOOP(
                    "The saved image task could not be reopened. Its files have been kept."
                ))
            );
            return false;
        }
        artifact_id_ = saved.value("artifact").toString();
        draft_id_ = saved.value("draft").toString();
        bool found = false;
        for (const auto& value : backend_.artifacts()) {
            const auto item = value.toMap();
            if (item.value("id").toString() == artifact_id_
                && item.value("kindKey").toString() == "image_raster")
                found = true;
        }
        if (!found && artifact_id_.isEmpty()) {
            const auto drafts = backend_.operatorDrafts();
            if (drafts.size() == 1) {
                const auto item = drafts.first().toMap();
                if (item.value("operatorTypeKey").toString() == "image.generate") {
                    artifact_id_ = item.value("contextArtifactId").toString();
                    draft_id_ = item.value("id").toString();
                    found = !artifact_id_.isEmpty();
                }
            }
            if (!found && backend_.artifacts().isEmpty())
                found = true;
        }
        if (!found) {
            startupFailed(
                QString::fromUtf8(QT_TR_NOOP(
                    "The saved image task no longer contains its image. Its files have been kept."
                ))
            );
            return false;
        }
    } else {
        if (!backend_.createProject(QUrl::fromLocalFile(directory), QStringLiteral("Deck image"))) {
            startupFailed(
                QString::fromUtf8(QT_TR_NOOP("The isolated image project could not be created."))
            );
            return false;
        }
    }
    saved.setValue("project", backend_.bundlePath());
    saved.setValue("request_blake3", request_digest_);
    saved.sync();
    if (artifact_id_.isEmpty()) {
        const auto canvas = request_.value("canvas").toObject();
        const auto draft = backend_.beginAiImageDraft(
            QString::fromUtf8(QT_TR_NOOP("Image for Deck")),
            request_.value("prompt").toString(),
            canvas.value("width").toInt(),
            canvas.value("height").toInt()
        );
        artifact_id_ = draft.value("contextArtifactId").toString();
        draft_id_ = draft.value("id").toString();
        if (artifact_id_.isEmpty() || draft_id_.isEmpty()) {
            startupFailed(QString::fromUtf8(QT_TR_NOOP("The image task could not be prepared.")));
            return false;
        }
        saved.setValue("project", backend_.bundlePath());
        saved.setValue("artifact", artifact_id_);
        saved.setValue("draft", draft_id_);
        saved.setValue("request_blake3", request_digest_);
        saved.sync();
    }
    saved.setValue("artifact", artifact_id_);
    saved.setValue("draft", draft_id_);
    saved.sync();
    project_path_ = QFileInfo(backend_.bundlePath()).canonicalFilePath();
    project_id_ = backend_.projectId();
    active_ = true;
    error_.clear();
    emit changed();
    return true;
}
void DeckMaterialTaskController::returnImage() {
    if (!canReturn())
        return;
    QJsonObject source;
    for (const auto& value : backend_.artifacts()) {
        const auto item = value.toMap();
        if (item.value("id").toString() == artifact_id_)
            source = {
                {"artifact_id", artifact_id_},
                {"revision_id", item.value("acceptedRevisionId").toString()}
            };
    }
    if (source.value("revision_id").toString().isEmpty())
        return;
    const auto project = project_path_, request = request_path_, result = result_path_,
               digest = request_digest_;
    const auto pin = QString::fromUtf8(QJsonDocument(source).toJson(QJsonDocument::Compact));
    error_.clear();
    finishing_ = true;
    watcher_.setFuture(QtConcurrent::run([project, request, result, pin, digest] {
        try {
            return object(fromRust(
                shape::desktop::finish_deck_material_task(
                    utf8(project),
                    utf8(request),
                    utf8(result),
                    utf8(pin),
                    utf8(digest),
                    "completed",
                    ""
                )
            ));
        } catch (const rust::Error& error) {
            return QJsonObject{{"error", QString::fromUtf8(error.what())}};
        }
    }));
    emit changed();
}
void DeckMaterialTaskController::finish() {
    finishing_ = false;
    if (watcher_.result().contains("error")) {
        error_ = QString::fromUtf8(QT_TR_NOOP(
            "The selected image could not be returned. Its accepted version is saved; retry "
            "returning it."
        ));
        emit changed();
        return;
    }
    active_ = false;
    emit changed();
    emit finished();
}
bool DeckMaterialTaskController::cancel() {
    if (!active_)
        return true;
    if (busy())
        return false;
    if (image_.running())
        image_.stopAfterCurrent();
    try {
        shape::desktop::finish_deck_material_task(
            "",
            utf8(request_path_),
            utf8(result_path_),
            "",
            utf8(request_digest_),
            "cancelled",
            ""
        );
    } catch (const rust::Error&) {
        error_ = QString::fromUtf8(QT_TR_NOOP(
            "The cancellation receipt could not be written. Keep this window open and retry."
        ));
        emit changed();
        return false;
    }
    active_ = false;
    emit changed();
    emit finished();
    return true;
}

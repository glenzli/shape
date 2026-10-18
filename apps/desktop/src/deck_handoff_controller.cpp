#include "deck_handoff_controller.hpp"
#include "desktop_backend.hpp"
#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"
#include <QCryptographicHash>
#include <QDesktopServices>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QProcess>
#include <QSettings>
#include <QStandardPaths>
#include <QUuid>
#include <QtConcurrent/QtConcurrentRun>
#ifdef Q_OS_WIN
#include <windows.h>
#elif defined(Q_OS_MACOS)
#include <libproc.h>
#endif

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
QString resolveExecutable(const QString& path) {
    QFileInfo info(path);
#ifdef Q_OS_MACOS
    if (info.isDir() && info.suffix() == "app") {
        QProcess plist;
        plist.start(
            "/usr/bin/plutil",
            {"-extract",
             "CFBundleExecutable",
             "raw",
             "-o",
             "-",
             info.filePath() + "/Contents/Info.plist"}
        );
        if (!plist.waitForStarted(1000))
            return {};
        if (!plist.waitForFinished(1000)) {
            plist.kill();
            plist.waitForFinished(1000);
            return {};
        }
        if (plist.exitCode() != 0)
            return {};
        const auto name = QString::fromUtf8(plist.readAllStandardOutput()).trimmed();
        if (name.isEmpty() || name != QFileInfo(name).fileName())
            return {};
        info.setFile(info.filePath() + "/Contents/MacOS/" + name);
    }
#endif
    return info.isAbsolute() && info.isFile() && info.isExecutable() ? info.canonicalFilePath()
                                                                     : QString();
}
bool externalProcessAlive(qint64 pid, const QString& executable) {
    if (pid <= 0 || executable.isEmpty())
        return false;
    QString actual;
#ifdef Q_OS_WIN
    const auto handle =
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, static_cast<DWORD>(pid));
    if (!handle)
        return false;
    wchar_t buffer[32768];
    DWORD size = 32768;
    if (QueryFullProcessImageNameW(handle, 0, buffer, &size))
        actual = QString::fromWCharArray(buffer, static_cast<qsizetype>(size));
    CloseHandle(handle);
#elif defined(Q_OS_MACOS)
    char buffer[PROC_PIDPATHINFO_MAXSIZE];
    if (proc_pidpath(static_cast<int>(pid), buffer, sizeof(buffer)) > 0)
        actual = QString::fromUtf8(buffer);
#else
    actual = QFileInfo(QString("/proc/%1/exe").arg(pid)).symLinkTarget();
#endif
    return !actual.isEmpty()
           && QFileInfo(actual).canonicalFilePath() == QFileInfo(executable).canonicalFilePath();
}
bool supportsDeck(const QString& executable) {
    QProcess probe;
    probe.start(executable, {QStringLiteral("--handoff-capabilities")});
    if (!probe.waitForStarted(3000))
        return false;
    if (!probe.waitForFinished(3000)) {
        probe.kill();
        probe.waitForFinished(1000);
        return false;
    }
    if (probe.exitStatus() != QProcess::NormalExit || probe.exitCode() != 0
        || probe.bytesAvailable() > 8192)
        return false;
    const auto result = QJsonDocument::fromJson(probe.readAllStandardOutput()).object();
    return result.value("schema") == "deck.handoff-capabilities/1"
           && result.value("external_edit") == "deck.external-edit/1";
}
} // namespace

DeckHandoffController::DeckHandoffController(
    DesktopBackend& backend,
    const QString& settingsPath,
    QObject* parent
)
    : QObject(parent), backend_(backend),
      settings_(std::make_unique<QSettings>(settingsPath, QSettings::IniFormat)) {
    configured_executable_ = settings_->value("deck/executable").toString();
    handoff_root_ = QFileInfo(settingsPath).absolutePath() + "/deck-handoffs";
    connect(
        &watcher_,
        &QFutureWatcher<QJsonObject>::finished,
        this,
        &DeckHandoffController::finish
    );
    poll_.setInterval(500);
    connect(&poll_, &QTimer::timeout, this, &DeckHandoffController::poll);
    connect(&backend_, &DesktopBackend::projectChanged, this, [this] {
        if (!project_path_.isEmpty() && !contextCurrent()) {
            ++generation_;
            pending_open_index_ = -1;
            waiting_ = false;
            poll_.stop();
            record_ = {};
            kit_id_.clear();
            emit changed();
        }
    });
}
DeckHandoffController::~DeckHandoffController() {
    poll_.stop();
    watcher_.waitForFinished();
}
QString DeckHandoffController::executable() const {
    if (!configured_executable_.isEmpty())
        return resolveExecutable(configured_executable_);
    const auto environment = qEnvironmentVariable("SHAPE_DECK_BINARY");
    if (!environment.isEmpty())
        return resolveExecutable(environment);
    auto installed = QStandardPaths::findExecutable("deck-desktop");
    if (installed.isEmpty())
        installed = QStandardPaths::findExecutable("Deck");
    if (!installed.isEmpty())
        return resolveExecutable(installed);
#ifdef Q_OS_MACOS
    return resolveExecutable("/Applications/Deck.app");
#else
    return {};
#endif
}
bool DeckHandoffController::setExecutable(const QUrl& path) {
    if (busy() || waiting_ || !path.isLocalFile())
        return false;
    const auto choice = QDir::cleanPath(QFileInfo(path.toLocalFile()).absoluteFilePath());
    const auto resolved = resolveExecutable(choice);
    if (resolved.isEmpty()) {
        fail(QString::fromUtf8(QT_TR_NOOP("Choose the Deck application or its executable.")));
        return false;
    }
    configured_executable_ = choice;
    settings_->setValue("deck/executable", choice);
    error_.clear();
    emit changed();
    return true;
}
QString DeckHandoffController::key(const QString& kitId) const {
    const auto origin =
        QFileInfo(backend_.bundlePath()).canonicalFilePath() + '\n' + backend_.projectId();
    return "deck/links/"
           + QString::fromLatin1(
               QCryptographicHash::hash(origin.toUtf8(), QCryptographicHash::Sha256).toHex()
           )
           + '/' + kitId;
}
bool DeckHandoffController::contextCurrent() const {
    return backend_.projectId() == project_id_
           && QFileInfo(backend_.bundlePath()).canonicalFilePath() == project_path_;
}
QVariantList DeckHandoffController::outputs() const {
    return record_.value("outputs").toArray().toVariantList();
}
bool DeckHandoffController::canRetry() const {
    return !busy() && !waiting_ && !requestPath().isEmpty()
           && !QFileInfo::exists(record_.value("result_path").toString());
}
void DeckHandoffController::saveRecord() {
    if (key_.isEmpty())
        return;
    record_["project_path"] = project_path_;
    record_["project_id"] = project_id_;
    record_["kit_id"] = kit_id_;
    record_["waiting"] = waiting_;
    settings_->setValue(key_, QJsonDocument(record_).toJson(QJsonDocument::Compact));
    settings_->sync();
}
bool DeckHandoffController::attach(const QString& kitId) {
    if (busy() || kitId.isEmpty() || !backend_.projectOpen())
        return false;
    if (kit_id_ == kitId && contextCurrent())
        return true;
    ++generation_;
    pending_open_index_ = -1;
    poll_.stop();
    waiting_ = false;
    project_path_ = QFileInfo(backend_.bundlePath()).canonicalFilePath();
    project_id_ = backend_.projectId();
    kit_id_ = kitId;
    key_ = key(kitId);
    record_ = QJsonDocument::fromJson(settings_->value(key_).toByteArray()).object();
    error_.clear();
    status_.clear();
    if (!record_.isEmpty()) {
        QFile request(record_.value("request_path").toString());
        if (record_.value("project_path") != project_path_
            || record_.value("project_id") != project_id_ || record_.value("kit_id") != kitId
            || !request.open(QIODevice::ReadOnly) || request.size() > 64 * 1024)
            record_ = {};
        else {
            const auto parsed = QJsonDocument::fromJson(request.readAll()).object();
            if (parsed.value("schema") != "deck.external-edit/1"
                || parsed.value("origin").toObject().value("project_id") != project_id_
                || parsed.value("origin").toObject().value("kit_id") != kitId
                || parsed.value("request_id") != record_.value("request_id"))
                record_ = {};
        }
        if (!record_.isEmpty()) {
            waiting_ = record_.value("waiting").toBool();
            if (waiting_ && !QFileInfo::exists(record_.value("result_path").toString())
                && !externalProcessAlive(
                    static_cast<qint64>(record_.value("pid").toDouble()),
                    record_.value("executable").toString()
                )) {
                waiting_ = false;
                record_["local_state"] = "incomplete";
                saveRecord();
                error_ = QString::fromUtf8(QT_TR_NOOP(
                    "Deck is no longer running and did not return files. Retry or read a saved "
                    "result."
                ));
            }
            if (waiting_)
                poll_.start();
        }
    }
    emit changed();
    if (!record_.isEmpty() && QFileInfo::exists(record_.value("result_path").toString()))
        readReturnedOutputs();
    return true;
}
void DeckHandoffController::fail(const QString& message) {
    error_ = message;
    status_.clear();
    emit changed();
}
void DeckHandoffController::openKit(
    const QString& kitId,
    const QString& digest,
    bool freshSnapshot
) {
    if (busy() || waiting_ || !attach(kitId))
        return;
    if (busy())
        return; // A recovered receipt is being verified first.
    const auto binary = executable();
    if (binary.isEmpty()) {
        fail(
            QString::fromUtf8(
                QT_TR_NOOP("Deck is not installed. Choose its application to continue.")
            )
        );
        return;
    }
    error_.clear();
    status_ = QString::fromUtf8(QT_TR_NOOP("Preparing this saved card set for Deck…"));
    const auto base = handoff_root_;
    QDir().mkpath(base);
    const auto directory = base + '/' + QUuid::createUuid().toString(QUuid::WithoutBraces);
    const auto requestId = QUuid::createUuid().toString(QUuid::WithoutBraces);
    const auto project = project_path_;
    const auto document =
        freshSnapshot || !hasDocument()
            ? QString()
            : QString::fromUtf8(
                  QJsonDocument(record_.value("document").toObject()).toJson(QJsonDocument::Compact)
              );
    work_ = Work::Prepare;
    work_generation_ = generation_;
    watcher_.setFuture(
        QtConcurrent::run([project, kitId, digest, requestId, directory, document, binary] {
            if (!supportsDeck(binary))
                return QJsonObject{{"error", "unsupported"}};
            try {
                return object(fromRust(
                    shape::desktop::prepare_deck_handoff(
                        utf8(project),
                        utf8(kitId),
                        utf8(digest),
                        utf8(requestId),
                        utf8(directory),
                        utf8(document)
                    )
                ));
            } catch (const rust::Error& error) {
                return QJsonObject{{"error", QString::fromUtf8(error.what())}};
            }
        })
    );
    emit changed();
}
void DeckHandoffController::finish() {
    const auto result = watcher_.result();
    const auto work = work_;
    work_ = Work::None;
    if (work_generation_ != generation_ || (work != Work::Standalone && !contextCurrent())) {
        pending_open_index_ = -1;
        emit changed();
        return;
    }
    if (result.contains("error")) {
        pending_open_index_ = -1;
        waiting_ = false;
        if (work == Work::Verify)
            record_["outputs"] = QJsonArray{};
        record_["local_state"] = "incomplete";
        saveRecord();
        fail(
            result.value("error") == "unsupported"
                ? QString::fromUtf8(QT_TR_NOOP(
                      "This Deck version does not support this handoff. Choose an updated "
                      "application."
                  ))
                : QString::fromUtf8(QT_TR_NOOP(
                      "The Deck handoff could not be completed. Your Shape draft is unchanged."
                  ))
        );
        return;
    }
    if (work == Work::Standalone) {
        if (!QProcess::startDetached(executable(), {}))
            fail(
                QString::fromUtf8(
                    QT_TR_NOOP("Deck could not be started. Choose its application and retry.")
                )
            );
        else {
            status_ = QString::fromUtf8(QT_TR_NOOP("Deck opened for a new card set."));
            emit changed();
        }
        return;
    }
    if (work == Work::Relaunch) {
        waiting_ = true;
        record_["local_state"] = "starting";
        saveRecord();
        launch();
        return;
    }
    if (work == Work::Prepare) {
        const auto document = record_.value("document");
        record_ = result;
        if (document.isObject())
            record_["document"] = document;
        record_["request_id"] = result.value("request").toObject().value("request_id");
        record_["local_state"] = "starting";
        waiting_ = true;
        saveRecord();
        launch();
        return;
    }
    record_["document"] = result.value("document");
    record_["outputs"] = result.value("outputs");
    record_["local_state"] = "returned";
    waiting_ = false;
    poll_.stop();
    saveRecord();
    const auto outcome = result.value("outcome").toString();
    error_.clear();
    status_ =
        outcome == "completed"
            ? QString::fromUtf8(QT_TR_NOOP("Returned files verified. Open them below."))
        : outcome == "cancelled"
            ? QString::fromUtf8(
                  QT_TR_NOOP("Deck editing was cancelled. Saved Deck work can be continued.")
              )
            : QString::fromUtf8(QT_TR_NOOP("Deck reported an error. Saved work can be continued."));
    emit changed();
    if (pending_open_index_ >= 0) {
        const auto list = result.value("outputs").toArray();
        const int index = pending_open_index_;
        pending_open_index_ = -1;
        if (index < list.size()) {
            const auto path = list[index].toObject().value("path").toString();
            QDesktopServices::openUrl(
                QUrl::fromLocalFile(pending_reveal_ ? QFileInfo(path).absolutePath() : path)
            );
        }
    }
}
void DeckHandoffController::launch() {
    const auto binary = executable();
    auto* process = new QProcess;
    process->setStandardOutputFile(QProcess::nullDevice());
    process->setStandardErrorFile(QProcess::nullDevice());
    connect(
        process,
        qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
        process,
        &QObject::deleteLater
    );
    connect(process, &QProcess::errorOccurred, process, [process](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart)
            process->deleteLater();
    });
    const auto generation = generation_;
    const auto recordKey = key_;
    const auto requestId = record_.value("request_id");
    const auto ended = [this, generation, recordKey, requestId] {
        auto saved = QJsonDocument::fromJson(settings_->value(recordKey).toByteArray()).object();
        if (saved.value("request_id") != requestId)
            return;
        if (QFileInfo::exists(saved.value("result_path").toString())) {
            if (generation == generation_ && contextCurrent())
                poll();
            return;
        }
        saved["waiting"] = false;
        saved["local_state"] = "incomplete";
        settings_->setValue(recordKey, QJsonDocument(saved).toJson(QJsonDocument::Compact));
        settings_->sync();
        if (generation != generation_ || !contextCurrent())
            return;
        waiting_ = false;
        poll_.stop();
        record_["waiting"] = false;
        fail(
            QString::fromUtf8(QT_TR_NOOP(
                "Deck closed without returning files. Continue the saved work or retry the handoff."
            ))
        );
    };
    connect(
        process,
        qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
        this,
        [ended](int, QProcess::ExitStatus) { ended(); }
    );
    connect(process, &QProcess::errorOccurred, this, [ended](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart)
            ended();
    });
    connect(process, &QProcess::started, this, [this, process, generation, binary] {
        if (generation != generation_ || !contextCurrent())
            return;
        record_["pid"] = static_cast<double>(process->processId());
        record_["executable"] = binary;
        record_["local_state"] = "waiting";
        saveRecord();
    });
    process->start(
        binary,
        {"--external-edit", requestPath(), "--result", record_.value("result_path").toString()}
    );
    status_ = QString::fromUtf8(QT_TR_NOOP("Editing in Deck. Return outputs there when ready."));
    poll_.start();
    emit changed();
}
void DeckHandoffController::poll() {
    if (!waiting_ || busy() || !contextCurrent())
        return;
    if (QFileInfo::exists(record_.value("result_path").toString())) {
        readReturnedOutputs();
        return;
    }
    if (record_.value("local_state") == "waiting"
        && !externalProcessAlive(
            static_cast<qint64>(record_.value("pid").toDouble()),
            record_.value("executable").toString()
        )) {
        waiting_ = false;
        poll_.stop();
        record_["local_state"] = "incomplete";
        saveRecord();
        fail(
            QString::fromUtf8(QT_TR_NOOP(
                "Deck closed without returning files. Continue the saved work or retry the handoff."
            ))
        );
    }
}
void DeckHandoffController::readReturnedOutputs() {
    if (busy() || record_.isEmpty() || !contextCurrent())
        return;
    const auto request = requestPath();
    const auto result = record_.value("result_path").toString();
    if (!QFileInfo::exists(result)) {
        fail(
            QString::fromUtf8(QT_TR_NOOP(
                "Deck has not returned a result yet. Keep editing there, then choose Return "
                "outputs."
            ))
        );
        return;
    }
    work_ = Work::Verify;
    work_generation_ = generation_;
    poll_.stop();
    const auto digest = record_.value("request_blake3").toString();
    watcher_.setFuture(QtConcurrent::run([request, result, digest] {
        try {
            return object(fromRust(
                shape::desktop::verify_deck_output_return(utf8(request), utf8(result), utf8(digest))
            ));
        } catch (const rust::Error& error) {
            return QJsonObject{{"error", QString::fromUtf8(error.what())}};
        }
    }));
    emit changed();
}
void DeckHandoffController::stopWaiting() {
    if (busy())
        return;
    ++generation_;
    waiting_ = false;
    poll_.stop();
    record_["local_state"] = "stopped";
    saveRecord();
    status_ = QString::fromUtf8(
        QT_TR_NOOP("Stopped waiting. Deck remains open; read its returned files here later.")
    );
    emit changed();
}
void DeckHandoffController::retryHandoff() {
    if (!canRetry() || !contextCurrent())
        return;
    const auto binary = executable();
    if (binary.isEmpty()) {
        fail(
            QString::fromUtf8(
                QT_TR_NOOP("Deck is not installed. Choose its application to continue.")
            )
        );
        return;
    }
    work_ = Work::Relaunch;
    work_generation_ = generation_;
    error_.clear();
    watcher_.setFuture(QtConcurrent::run([binary] {
        return supportsDeck(binary) ? QJsonObject{} : QJsonObject{{"error", "unsupported"}};
    }));
    emit changed();
}
void DeckHandoffController::openNewDeck() {
    if (busy())
        return;
    const auto binary = executable();
    if (binary.isEmpty()) {
        fail(
            QString::fromUtf8(
                QT_TR_NOOP("Deck is not installed. Choose its application to continue.")
            )
        );
        return;
    }
    work_ = Work::Standalone;
    work_generation_ = generation_;
    error_.clear();
    watcher_.setFuture(QtConcurrent::run([binary] {
        return supportsDeck(binary) ? QJsonObject{} : QJsonObject{{"error", "unsupported"}};
    }));
    emit changed();
}
bool DeckHandoffController::openStorybook() {
#ifdef Q_OS_MACOS
    const auto binary = resolveExecutable("/Applications/storybook-co-editor.app");
    if (!binary.isEmpty() && QProcess::startDetached(binary, {}))
        return true;
#endif
    fail(
        QString::fromUtf8(QT_TR_NOOP(
            "Install Storybook Co Editor to make a new picture book. Existing Shape books remain "
            "available."
        ))
    );
    return false;
}
bool DeckHandoffController::openOutput(int index, bool reveal) {
    if (busy() || !contextCurrent())
        return false;
    const auto list = record_.value("outputs").toArray();
    if (index < 0 || index >= list.size())
        return false;
    pending_open_index_ = index;
    pending_reveal_ = reveal;
    readReturnedOutputs();
    return busy();
}

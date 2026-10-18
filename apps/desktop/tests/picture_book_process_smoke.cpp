#include "picture_book_process_smoke.hpp"
#include "desktop_backend.hpp"
#include "material_edit_controller.hpp"
#include "picture_book_controller.hpp"
#include <QCryptographicHash>
#include <QDir>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QFileInfo>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSettings>
#include <QTimer>
#include <QUrl>
#include <functional>
#include <iostream>
#include <stdexcept>

namespace picture_book_smoke {
namespace {
void require(bool condition, const char* message) {
    if (!condition)
        throw std::runtime_error(message);
}
bool waitFor(const std::function<bool()>& condition) {
    QElapsedTimer timer;
    timer.start();
    while (!condition() && timer.elapsed() < 45000) {
        QEventLoop loop;
        QTimer::singleShot(10, &loop, &QEventLoop::quit);
        loop.exec();
    }
    return condition();
}
void writeJson(const QString& path, const QJsonObject& value) {
    QFile file(path);
    require(file.open(QIODevice::WriteOnly | QIODevice::NewOnly), "new restart evidence file");
    const auto bytes = QJsonDocument(value).toJson();
    require(file.write(bytes) == bytes.size(), "complete restart evidence write");
}
QJsonObject pin(DesktopBackend& backend, const QString& id) {
    for (const auto& value : backend.artifacts()) {
        const auto item = value.toMap();
        if (item.value("id").toString() == id)
            return {
                {"artifact_id", id},
                {"revision_id", item.value("acceptedRevisionId").toString()}
            };
    }
    throw std::runtime_error("restart source/output artifact missing");
}
QString sourceHash(DesktopBackend& backend, const QJsonObject& source) {
    const auto bytes = backend.bookMaterial(
        QString::fromUtf8(QJsonDocument(source).toJson(QJsonDocument::Compact)),
        QStringLiteral("image")
    );
    return QString::fromLatin1(QCryptographicHash::hash(bytes, QCryptographicHash::Sha256).toHex());
}
} // namespace

void verifyBundleScopedShadowLocators(
    DesktopBackend& backend,
    const QString& original_bundle,
    const QString& copied_bundle,
    const QString& output,
    const QString& ticket,
    const QString& directory
) {
    const auto original_handoff = directory + "/locator-original/handoff.json";
    const auto copied_handoff = directory + "/locator-copy/handoff.json";
    (void)shape::desktop::export_material_copy(
        original_bundle.toStdString(),
        ticket.toStdString(),
        (directory + "/locator-original").toStdString()
    );
    (void)shape::desktop::export_material_copy(
        copied_bundle.toStdString(),
        ticket.toStdString(),
        (directory + "/locator-copy").toStdString()
    );
    const auto settings_path = directory + "/bundle-scoped-locators.ini";
    QSettings settings(settings_path, QSettings::IniFormat);
    const auto project_id = backend.projectId();
    const auto legacy_key =
        QStringLiteral("material-edit/shadow-sessions/%1/%2").arg(project_id, output);
    const auto seed_legacy = [&](const QString& bundle, const QString& handoff) {
        const QJsonObject record{
            {"schema", "shape.shadow-local-session/1"},
            {"project_path", bundle},
            {"project_id", project_id},
            {"output", output},
            {"ticket", ticket},
            {"handoff", handoff},
            {"result", QFileInfo(handoff).dir().filePath("shadow-result.json")}
        };
        settings.setValue(
            legacy_key,
            QString::fromUtf8(QJsonDocument(record).toJson(QJsonDocument::Compact))
        );
        settings.sync();
    };
    require(
        backend.openProject(QUrl::fromLocalFile(original_bundle)),
        "open original locator fixture"
    );
    seed_legacy(original_bundle, original_handoff);
    {
        MaterialEditController editor(backend, settings_path);
        require(
            editor.restoreShadowSession(original_bundle, output)
                && editor.handoffPath() == original_handoff,
            "matching legacy locator migrates to original bundle scope"
        );
    }
    require(backend.openProject(QUrl::fromLocalFile(copied_bundle)), "open copied locator fixture");
    seed_legacy(copied_bundle, copied_handoff);
    {
        MaterialEditController editor(backend, settings_path);
        require(
            editor.restoreShadowSession(copied_bundle, output)
                && editor.handoffPath() == copied_handoff,
            "identical project and stage ids save a separate copied-bundle locator"
        );
    }
    settings.remove(legacy_key);
    settings.sync();
    require(
        backend.openProject(QUrl::fromLocalFile(original_bundle)),
        "reopen original scoped locator"
    );
    {
        MaterialEditController editor(backend, settings_path);
        require(
            editor.restoreShadowSession(original_bundle, output)
                && editor.handoffPath() == original_handoff,
            "copied bundle cannot overwrite original automatic handoff path"
        );
        require(
            editor.restart(original_bundle, output, false),
            "cancel only original scoped locator"
        );
    }
    require(
        backend.openProject(QUrl::fromLocalFile(copied_bundle)),
        "reopen copied scoped locator"
    );
    {
        MaterialEditController editor(backend, settings_path);
        require(
            editor.restoreShadowSession(copied_bundle, output)
                && editor.handoffPath() == copied_handoff,
            "clearing original ticket cannot clear copied bundle continuation"
        );
        require(editor.restart(copied_bundle, output, false), "cancel copied scoped locator");
    }
    writeJson(
        directory + "/bundle-scoped-locators.json",
        {{"passed", true},
         {"legacy_migration", true},
         {"identical_project_and_stage_ids", true},
         {"independent_paths", true},
         {"independent_cancel", true},
         {"original_handoff", original_handoff},
         {"copied_handoff", copied_handoff}}
    );
}

bool processRestart(
    DesktopBackend& backend,
    PictureBookController& book,
    const QString& directory,
    const QString& phase
) {
    require(
        (phase == "launch" || phase == "resume")
            && !qEnvironmentVariable("SHAPE_SHADOW_BINARY").isEmpty()
            && QDir::isAbsolutePath(directory),
        "whole-process restart requires explicit phase, Shadow binary and absolute fixture "
        "directory"
    );
    require(QDir().mkpath(directory), "restart fixture directory");
    const auto record_path = directory + "/restart-session.json";
    const auto settings = directory + "/shadow-session.ini";
    if (phase == "launch") {
        require(!QFileInfo::exists(record_path), "restart launch evidence must be new");
        require(
            backend.createProject(QUrl::fromLocalFile(directory), "Shadow process restart"),
            "new process restart project"
        );
        QImage image(640, 426, QImage::Format_RGB32);
        image.fill(QColor("#254a70"));
        const auto image_path = directory + "/source.png";
        require(
            image.save(image_path) && backend.importRaster(QUrl::fromLocalFile(image_path)),
            "restart source import"
        );
        QJsonObject source;
        for (const auto& value : backend.artifacts()) {
            const auto item = value.toMap();
            if (item.value("kindKey") == "image_raster")
                source = pin(backend, item.value("id").toString());
        }
        const auto stage = book.beginArtworkEdit(source.toVariantMap());
        const auto output = stage.value("contextArtifactId").toString();
        require(!output.isEmpty(), "restart exact material stage");
        book.beginNew();
        auto state = QJsonObject::fromVariantMap(book.draft().value("state").toMap());
        auto composition = state["composition"].toObject();
        composition["name"] = "Whole-process pending book";
        composition["pages"] = QJsonArray{
            QJsonObject{{"kind", "image"}, {"source", source}},
            QJsonObject{{"kind", "blank"}}
        };
        state["composition"] = composition;
        state["active_language"] = "zh-CN";
        state["script"] = "[封面]\n跨进程仍待采用的脚本。\n[Unknown]\n保留未完成语法。";
        state["script_dirty"] = true;
        state["artwork_edit"] =
            QJsonObject{{"output_artifact_id", output}, {"page_index", 0}, {"source", source}};
        require(
            book.saveState(state.toVariantMap()),
            "checkpoint pending book before process exit"
        );
        state = QJsonObject::fromVariantMap(book.draft().value("state").toMap());
        qputenv("SHADOW_PIPELINE_SMOKE_ACTION", "complete");
        qputenv("SHADOW_DESKTOP_DATA_ROOT", (directory + "/shadow-profile").toUtf8());
        qputenv("SHADOW_PAINT_BRUSH_SETTINGS_FILE", (directory + "/shadow-brush.ini").toUtf8());
        MaterialEditController editor(backend, settings);
        require(
            editor.setShadowExecutable(
                QUrl::fromLocalFile(qEnvironmentVariable("SHAPE_SHADOW_BINARY"))
            ),
            "restart selects explicit Shadow binary"
        );
        editor.openShadow(backend.bundlePath(), output);
        require(
            waitFor([&] {
                return editor.shadowActive()
                       || (!editor.running() && !editor.errorCode().isEmpty());
            }) && editor.shadowActive()
                && !editor.handoffPath().isEmpty(),
            "real independent Shadow launched before complete Shape process exit"
        );
        const auto result = QFileInfo(editor.handoffPath()).dir().filePath("shadow-result.json");
        require(
            !QFileInfo::exists(result),
            "Shape exits while the real Shadow return is still pending"
        );
        writeJson(
            record_path,
            {{"schema", "shape.book-shadow-process-restart/1"},
             {"project", backend.bundlePath()},
             {"book", book.draft().value("artifact_id").toString()},
             {"state", state},
             {"source", source},
             {"source_sha256", sourceHash(backend, source)},
             {"output", output},
             {"handoff", editor.handoffPath()},
             {"result", result}}
        );
        // Returning from this opt-in smoke exits main normally. The outer runner waits for the
        // application PID to exit, then for Shadow's result before starting a fresh Shape PID.
        std::cout << "whole Shape process may exit with independent Shadow pending" << std::endl;
        return true;
    }
    QFile file(record_path);
    require(
        file.open(QIODevice::ReadOnly) && file.size() < 64 * 1024,
        "saved process restart evidence"
    );
    const auto record = QJsonDocument::fromJson(file.readAll()).object();
    require(record["schema"] == "shape.book-shadow-process-restart/1", "process restart schema");
    require(
        backend.openProject(QUrl::fromLocalFile(record["project"].toString()))
            && book.load(record["book"].toString()),
        "fresh Shape PID opens exact project and book checkpoint"
    );
    require(
        QJsonObject::fromVariantMap(book.draft().value("state").toMap())
                == record["state"].toObject()
            && book.source().isEmpty(),
        "whole-process restart preserves unfinished script, page binding and unaccepted book"
    );
    const auto output = record["output"].toString();
    MaterialEditController editor(backend, settings);
    require(
        editor.restoreShadowSession(backend.bundlePath(), output) && waitFor([&] {
            return !editor.running() && !editor.shadowActive();
        }) && editor.errorCode().isEmpty(),
        "fresh Shape PID restores local handoff and verifies real native completion"
    );
    QString candidate;
    for (const auto& value : backend.candidates())
        if (value.toMap().value("artifactId") == output)
            candidate = value.toMap().value("id").toString();
    require(
        !candidate.isEmpty() && pin(backend, output)["revision_id"].toString().isEmpty()
            && pin(backend, record["source"].toObject()["artifact_id"].toString())
                   == record["source"].toObject()
            && sourceHash(backend, record["source"].toObject())
                   == record["source_sha256"].toString(),
        "process restart creates only candidate and preserves exact original"
    );
    writeJson(
        directory + "/process-restart-report.json",
        {{"schema", "shape.book-shadow-process-restart-evidence/1"},
         {"passed", true},
         {"candidate", candidate},
         {"accepted", false},
         {"source_unchanged", true},
         {"unfinished_book_checkpoint_recovered", true},
         {"local_handoff_recovered", true},
         {"record", record_path},
         {"handoff", record["handoff"]},
         {"result", record["result"]}}
    );
    require(
        editor.restart(backend.bundlePath(), output, false),
        "restart candidate explicitly cancelled"
    );
    std::cout << "fresh Shape process recovered real Shadow completion as candidate" << std::endl;
    return true;
}
} // namespace picture_book_smoke

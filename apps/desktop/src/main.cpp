#include "audio_export_controller.hpp"
#include "audio_preview_controller.hpp"
#include "desktop_backend.hpp"
#include "image_live_smoke.hpp"
#include "image_preview_provider.hpp"
#include "recent_projects.hpp"
#include "infer_image_controller.hpp"
#include "infer_runtime_controller.hpp"
#include "infer_speech_controller.hpp"
#include "infer_text_controller.hpp"
#include "speech_live_smoke.hpp"
#include "text_authoring_smoke.hpp"
#include "ui_preferences.hpp"
#include "workspace_host_smoke.hpp"

#if defined(Q_OS_MACOS)
#include "mac_titlebar.hpp"
#endif

#include "rust/cxx.h"

#include <QColor>
#include <QDir>
#include <QFile>
#include <QGuiApplication>
#include <QImage>
#include <QQmlApplicationEngine>
#include <QQmlExpression>
#include <QQuickWindow>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QTimer>
#include <QUrl>
#include <QVariant>

#include <algorithm>
#include <iostream>
#include <memory>
#include <optional>
#include <string>

namespace {

struct Arguments {
    std::optional<std::string> project_path;
    bool smoke_exit = false;
    bool smoke_text_cycle = false;
    bool smoke_raster_cycle = false;
    std::optional<QString> speech_live_directory;
    std::optional<QString> speech_script_directory;
    std::optional<QString> text_authoring_directory;
    std::optional<QString> image_live_directory;
};

std::optional<Arguments> parse_arguments(int argc, char* argv[]) {
    Arguments arguments;
    for (int index = 1; index < argc; ++index) {
        const std::string argument(argv[index]);
        if (argument == "--project") {
            if (index + 1 >= argc) {
                return std::nullopt;
            }
            arguments.project_path = std::string(argv[++index]);
        } else if (argument == "--smoke-exit") {
            arguments.smoke_exit = true;
        } else if (argument == "--smoke-text-cycle") {
            arguments.smoke_text_cycle = true;
        } else if (argument == "--smoke-raster-cycle") {
            arguments.smoke_raster_cycle = true;
        } else if (argument == "--smoke-speech-script" && index + 1 < argc) {
            arguments.speech_script_directory = QString::fromLocal8Bit(argv[++index]);
        } else if (argument == "--smoke-speech-live" && index + 1 < argc) {
            arguments.speech_live_directory = QString::fromLocal8Bit(argv[++index]);
        } else if (argument == "--smoke-image-generation" && index + 1 < argc) {
            arguments.image_live_directory = QString::fromLocal8Bit(argv[++index]);
        } else if (argument == "--smoke-text-authoring" && index + 1 < argc) {
            arguments.text_authoring_directory = QString::fromLocal8Bit(argv[++index]);
        } else {
            return std::nullopt;
        }
    }
    return arguments;
}

bool run_smoke_raster_cycle(
    DesktopBackend& backend,
    QObject& root_object,
    const std::string& project_path
) {
    QTemporaryDir fixture_directory;
    if (!fixture_directory.isValid()) {
        std::cerr << "desktop raster smoke could not create fixture directory" << std::endl;
        return false;
    }
    const QString fixture_path = fixture_directory.filePath(QStringLiteral("fixture.png"));
    QImage fixture(8, 6, QImage::Format_RGBA8888);
    for (int y = 0; y < fixture.height(); ++y) {
        for (int x = 0; x < fixture.width(); ++x) {
            fixture.setPixelColor(x, y, QColor(x * 24, y * 32, 96, 255));
        }
    }
    if (!fixture.save(fixture_path, "PNG")
        || !backend.importRaster(QUrl::fromLocalFile(fixture_path))) {
        std::cerr << "desktop raster smoke could not import fixture" << std::endl;
        return false;
    }

    const QVariantList imported_artifacts = backend.artifacts();
    const auto imported = std::find_if(
        imported_artifacts.cbegin(),
        imported_artifacts.cend(),
        [](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("kindKey")).toString()
                   == QStringLiteral("image_raster");
        }
    );
    if (imported == imported_artifacts.cend()) {
        std::cerr << "desktop raster smoke lost imported artifact" << std::endl;
        return false;
    }
    const QVariantMap imported_artifact = imported->toMap();
    const QString artifact_id = imported_artifact.value(QStringLiteral("id")).toString();
    const QString imported_head =
        imported_artifact.value(QStringLiteral("acceptedRevisionId")).toString();
    const QString exported_png = fixture_directory.filePath(QStringLiteral("accepted.png"));
    if (!backend.exportAcceptedMaterial(artifact_id, QUrl::fromLocalFile(exported_png))
        || QImage(exported_png).size() != QSize(8, 6)) {
        std::cerr << "desktop raster smoke could not export accepted PNG" << std::endl;
        return false;
    }
    const int artifact_index =
        static_cast<int>(std::distance(imported_artifacts.cbegin(), imported));
    root_object.setProperty("selectedArtifactIndex", artifact_index);
    QCoreApplication::processEvents();
    if (imported_artifact.value(QStringLiteral("imageWidth")).toInt() != 8
        || imported_artifact.value(QStringLiteral("imageHeight")).toInt() != 6
        || !backend.prepareImagePreviews(artifact_id) || backend.acceptedImageSource().isEmpty()) {
        std::cerr << "desktop raster smoke could not present accepted image" << std::endl;
        return false;
    }

    QObject* const image_palette =
        root_object.findChild<QObject*>(QStringLiteral("operatorPalette"));
    QObject* const workspace_surface =
        root_object.findChild<QObject*>(QStringLiteral("workspaceSurface"));
    if (image_palette == nullptr || workspace_surface == nullptr
        || image_palette->property("compatibleOperatorCount").toInt() != 3) {
        std::cerr << "desktop raster smoke did not expose the universal text editor beside the "
                     "unified image editor"
                  << std::endl;
        return false;
    }
    QQmlExpression choose_image_editor(
        QQmlEngine::contextForObject(image_palette),
        image_palette,
        QStringLiteral("chooseOperator('image.edit')")
    );
    choose_image_editor.evaluate();
    QCoreApplication::processEvents();
    if (choose_image_editor.hasError()
        || workspace_surface->property("workspaceRouteKey").toString()
               != QStringLiteral("operator.image.edit")
        || workspace_surface->property("loadedWorkspaceObjectName").toString()
               != QStringLiteral("imageEditorWorkspace")) {
        std::cerr << "desktop raster smoke could not open the unified image editor" << std::endl;
        return false;
    }
    QObject* image_editor = root_object.findChild<QObject*>(QStringLiteral("imageEditorWorkspace"));
    if (image_editor == nullptr) {
        std::cerr << "desktop raster smoke lost the unified image editor" << std::endl;
        return false;
    }
    QQmlExpression choose_size_tool(
        QQmlEngine::contextForObject(image_editor),
        image_editor,
        QStringLiteral("chooseTool('size')")
    );
    choose_size_tool.evaluate();
    QCoreApplication::processEvents();
    QCoreApplication::processEvents();
    image_editor = root_object.findChild<QObject*>(QStringLiteral("imageEditorWorkspace"));
    QObject* const resize_tool =
        image_editor == nullptr
            ? nullptr
            : image_editor->findChild<QObject*>(QStringLiteral("rasterResizeOperatorWorkspace"));
    QObject* const tool_stack =
        image_editor == nullptr
            ? nullptr
            : image_editor->findChild<QObject*>(QStringLiteral("imageEditorToolStack"));
    const QVariantList image_edit_drafts = backend.operatorDrafts();
    if (choose_size_tool.hasError() || image_editor == nullptr || resize_tool == nullptr
        || tool_stack == nullptr || tool_stack->property("currentIndex").toInt() != 1
        || !resize_tool->property("visible").toBool()
        || image_editor->property("selectedToolKey").toString() != QStringLiteral("size")
        || image_edit_drafts.size() != 1
        || image_edit_drafts.first().toMap().value(QStringLiteral("operatorTypeKey")).toString()
               != QStringLiteral("image.resize")) {
        std::cerr << "desktop raster smoke could not switch to the Size tool" << std::endl;
        return false;
    }
    if (!QMetaObject::invokeMethod(workspace_surface, "showGraph", Qt::DirectConnection)) {
        return false;
    }
    QCoreApplication::processEvents();

    if (!backend.proposeRasterCrop(artifact_id, 1, 1, 4, 3) || !backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not create crop candidate" << std::endl;
        return false;
    }
    const QString candidate_id = backend.candidateId();
    if (backend.artifacts()[artifact_index]
                .toMap()
                .value(QStringLiteral("acceptedRevisionId"))
                .toString()
            != imported_head
        || !backend.prepareImagePreviews(artifact_id, candidate_id)
        || backend.candidateImageSource().isEmpty()) {
        std::cerr << "desktop raster candidate changed history or failed preview" << std::endl;
        return false;
    }
    root_object.setProperty("compareMode", true);
    QCoreApplication::processEvents();

    if (!backend.acceptCandidate(candidate_id) || backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not accept crop candidate" << std::endl;
        return false;
    }
    const QVariantMap accepted = backend.artifacts()[artifact_index].toMap();
    const QVariantList operator_nodes = accepted.value(QStringLiteral("operatorNodes")).toList();
    const bool has_crop_operator =
        std::any_of(operator_nodes.cbegin(), operator_nodes.cend(), [](const QVariant& node) {
            const QVariantMap projected = node.toMap();
            return projected.value(QStringLiteral("roleKey")).toString()
                       == QStringLiteral("operator")
                   && projected.value(QStringLiteral("operatorTypeKey")).toString()
                          == QStringLiteral("image.crop");
        });
    if (accepted.value(QStringLiteral("acceptedRevisionId")).toString() == imported_head
        || accepted.value(QStringLiteral("imageWidth")).toInt() != 4
        || accepted.value(QStringLiteral("imageHeight")).toInt() != 3 || !has_crop_operator) {
        std::cerr << "desktop raster smoke committed an unexpected crop" << std::endl;
        return false;
    }
    root_object.setProperty("compareMode", false);
    QCoreApplication::processEvents();
    if (!backend.prepareImagePreviews(artifact_id)) {
        std::cerr << "desktop raster smoke could not refresh the accepted image preview"
                  << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    QObject* const scene_graph =
        root_object.findChild<QObject*>(QStringLiteral("sceneOperatorGraphWorkspace"));
    if (scene_graph == nullptr
        || !scene_graph->property("currentResultPreviewAvailable").toBool()) {
        std::cerr << "desktop raster smoke found no preview in the current Result node"
                  << std::endl;
        return false;
    }
    if (!workspace_host_smoke::verifyOperatorRoute(
            root_object,
            QStringLiteral("image.crop"),
            QStringLiteral("operator.image.crop"),
            QStringLiteral("imageEditorWorkspace")
        )) {
        std::cerr << "desktop raster smoke did not route image.crop" << std::endl;
        return false;
    }

    const QVariantList compatible_operators = backend.compatibleOperators(artifact_id);
    const bool has_resize_descriptor = std::any_of(
        compatible_operators.cbegin(),
        compatible_operators.cend(),
        [](const QVariant& descriptor) {
            return descriptor.toMap().value(QStringLiteral("typeKey")).toString()
                   == QStringLiteral("image.resize");
        }
    );
    const QString resize_draft_id =
        backend.beginOperatorDraft(artifact_id, QStringLiteral("image.resize"));
    if (!has_resize_descriptor) {
        std::cerr << "desktop raster smoke found no executable resize descriptor" << std::endl;
        return false;
    }
    if (resize_draft_id.isEmpty()) {
        std::cerr << "desktop raster smoke could not begin the resize draft: "
                  << backend.lastError().toStdString() << std::endl;
        return false;
    }
    if (!backend.updateImageResizeDraft(
            resize_draft_id,
            2,
            2,
            QStringLiteral("fit_within"),
            QStringLiteral("lanczos3")
        )) {
        std::cerr << "desktop raster smoke could not save the resize draft: "
                  << backend.lastError().toStdString() << std::endl;
        return false;
    }
    if (!backend.openProject(QUrl::fromLocalFile(QString::fromStdString(project_path)))) {
        std::cerr << "desktop raster smoke could not reopen the resize draft: "
                  << backend.lastError().toStdString() << std::endl;
        return false;
    }

    const QVariantList reopened_drafts = backend.operatorDrafts();
    const auto reopened_resize_draft = std::find_if(
        reopened_drafts.cbegin(),
        reopened_drafts.cend(),
        [&resize_draft_id](const QVariant& draft) {
            return draft.toMap().value(QStringLiteral("id")).toString() == resize_draft_id;
        }
    );
    if (reopened_resize_draft == reopened_drafts.cend()
        || reopened_resize_draft->toMap().value(QStringLiteral("imageResizeTargetWidth")).toInt()
               != 2
        || reopened_resize_draft->toMap().value(QStringLiteral("imageResizeTargetHeight")).toInt()
               != 2
        || reopened_resize_draft->toMap()
                   .value(QStringLiteral("imageResizeAspectPolicy"))
                   .toString()
               != QStringLiteral("fit_within")
        || reopened_resize_draft->toMap().value(QStringLiteral("imageResizeResampling")).toString()
               != QStringLiteral("lanczos3")) {
        std::cerr << "desktop raster smoke reopened a different resize draft" << std::endl;
        return false;
    }

    const QVariantList reopened_artifacts = backend.artifacts();
    const auto current_artifact = std::find_if(
        reopened_artifacts.cbegin(),
        reopened_artifacts.cend(),
        [&artifact_id](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("id")).toString() == artifact_id;
        }
    );
    if (current_artifact == reopened_artifacts.cend()) {
        std::cerr << "desktop raster smoke lost the resized draft source" << std::endl;
        return false;
    }
    const int reopened_artifact_index =
        static_cast<int>(std::distance(reopened_artifacts.cbegin(), current_artifact));
    root_object.setProperty("selectedArtifactIndex", reopened_artifact_index);
    QCoreApplication::processEvents();

    QQmlExpression open_resize_draft(
        QQmlEngine::contextForObject(workspace_surface),
        workspace_surface,
        QStringLiteral("openOperatorDraft(\"") + resize_draft_id + QStringLiteral("\")")
    );
    const QVariant opened = open_resize_draft.evaluate();
    QCoreApplication::processEvents();
    QCoreApplication::processEvents();
    QObject* const workspace_host =
        workspace_surface->findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    QObject* const loaded_resize_workspace =
        workspace_host == nullptr
            ? nullptr
            : qvariant_cast<QObject*>(workspace_host->property("loadedWorkspace"));
    QObject* const resize_controls = loaded_resize_workspace == nullptr
                                         ? nullptr
                                         : loaded_resize_workspace->findChild<QObject*>(
                                               QStringLiteral("rasterResizeOperatorWorkspace")
                                           );
    if (open_resize_draft.hasError() || !opened.toBool() || workspace_host == nullptr
        || workspace_surface->property("workspaceRouteKey").toString()
               != QStringLiteral("operator.image.resize")
        || workspace_surface->property("loadedWorkspaceObjectName").toString()
               != QStringLiteral("imageEditorWorkspace")
        || resize_controls == nullptr
        || resize_controls->property("operatorDraftId").toString() != resize_draft_id
        || resize_controls->property("pendingWidth").toInt() != 2
        || resize_controls->property("pendingHeight").toInt() != 2) {
        std::cerr << "desktop raster smoke did not restore the Resize workspace" << std::endl;
        return false;
    }

    if (!backend.proposeRasterResize(artifact_id, resize_draft_id) || !backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not create resize candidate" << std::endl;
        return false;
    }
    const QString resize_candidate_id = backend.candidateId();
    if (backend.artifacts()[reopened_artifact_index]
                .toMap()
                .value(QStringLiteral("acceptedRevisionId"))
                .toString()
            != accepted.value(QStringLiteral("acceptedRevisionId")).toString()
        || !backend.prepareImagePreviews(artifact_id, resize_candidate_id)
        || backend.candidateImageSource().isEmpty()) {
        std::cerr << "desktop raster resize changed history or failed preview" << std::endl;
        return false;
    }
    root_object.setProperty("compareMode", true);
    QCoreApplication::processEvents();
    if (!backend.acceptCandidate(resize_candidate_id) || backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not accept resize candidate" << std::endl;
        return false;
    }
    const QVariantMap resized = backend.artifacts()[reopened_artifact_index].toMap();
    const QVariantList resized_operator_nodes =
        resized.value(QStringLiteral("operatorNodes")).toList();
    const bool has_resize_operator = std::any_of(
        resized_operator_nodes.cbegin(),
        resized_operator_nodes.cend(),
        [](const QVariant& node) {
            const QVariantMap projected = node.toMap();
            return projected.value(QStringLiteral("roleKey")).toString()
                       == QStringLiteral("operator")
                   && projected.value(QStringLiteral("operatorTypeKey")).toString()
                          == QStringLiteral("image.resize");
        }
    );
    if (resized.value(QStringLiteral("imageWidth")).toInt() != 2
        || resized.value(QStringLiteral("imageHeight")).toInt() != 2 || !has_resize_operator
        || !workspace_host_smoke::verifyOperatorRoute(
            root_object,
            QStringLiteral("image.resize"),
            QStringLiteral("operator.image.resize"),
            QStringLiteral("imageEditorWorkspace")
        )) {
        std::cerr << "desktop raster smoke committed or routed an unexpected resize" << std::endl;
        return false;
    }

    if (!backend.proposeRasterTransform(artifact_id, QStringLiteral("rotate90_clockwise"))
        || !backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not create transform candidate" << std::endl;
        return false;
    }
    const QString transform_candidate_id = backend.candidateId();
    if (!backend.prepareImagePreviews(artifact_id, transform_candidate_id)
        || backend.candidateImageSource().isEmpty()
        || !backend.acceptCandidate(transform_candidate_id)) {
        std::cerr << "desktop raster transform failed preview or acceptance" << std::endl;
        return false;
    }

    if (!backend.proposeRasterBlur(artifact_id, 1) || !backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not create blur candidate" << std::endl;
        return false;
    }
    const QString blur_candidate_id = backend.candidateId();
    if (!backend.prepareImagePreviews(artifact_id, blur_candidate_id)
        || backend.candidateImageSource().isEmpty()
        || !backend.acceptCandidate(blur_candidate_id)) {
        std::cerr << "desktop raster blur failed preview or acceptance" << std::endl;
        return false;
    }

    if (!backend.proposeRasterUnsharpMask(artifact_id, 1, 1250, 4) || !backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not create unsharp-mask candidate" << std::endl;
        return false;
    }
    const QString unsharp_candidate_id = backend.candidateId();
    if (!backend.prepareImagePreviews(artifact_id, unsharp_candidate_id)
        || backend.candidateImageSource().isEmpty()
        || !backend.acceptCandidate(unsharp_candidate_id)) {
        std::cerr << "desktop raster unsharp mask failed preview or acceptance" << std::endl;
        return false;
    }
    if (!workspace_host_smoke::verifyOperatorRoute(
            root_object,
            QStringLiteral("image.unsharp_mask"),
            QStringLiteral("operator.image.unsharp_mask"),
            QStringLiteral("imageEditorWorkspace"),
            QString(),
            QStringLiteral("rasterEffectsWorkspace"),
            QStringLiteral("effects")
        )) {
        std::cerr << "desktop raster smoke did not route unsharp mask into Image Editing"
                  << std::endl;
        return false;
    }

    if (!backend.proposeRasterDropShadow(artifact_id, 1, 0, 0, 0, 0, 0, 128)
        || !backend.hasCandidate()) {
        std::cerr << "desktop raster smoke could not create drop-shadow candidate" << std::endl;
        return false;
    }
    const QString shadow_candidate_id = backend.candidateId();
    if (!backend.prepareImagePreviews(artifact_id, shadow_candidate_id)
        || backend.candidateImageSource().isEmpty()
        || !backend.acceptCandidate(shadow_candidate_id)) {
        std::cerr << "desktop raster drop shadow failed preview or acceptance" << std::endl;
        return false;
    }
    if (!workspace_host_smoke::verifyOperatorRoute(
            root_object,
            QStringLiteral("image.drop_shadow"),
            QStringLiteral("operator.image.drop_shadow"),
            QStringLiteral("imageEditorWorkspace"),
            QString(),
            QStringLiteral("rasterEffectsWorkspace"),
            QStringLiteral("effects")
        )) {
        std::cerr << "desktop raster smoke did not route drop shadow into Image Editing"
                  << std::endl;
        return false;
    }

    const auto reopened = shape::desktop::load_project_snapshot(project_path);
    const auto reopened_artifact = std::find_if(
        reopened.artifacts.begin(),
        reopened.artifacts.end(),
        [&artifact_id](const shape::desktop::ArtifactSummaryWire& artifact) {
            return QString::fromUtf8(artifact.id.data(), static_cast<qsizetype>(artifact.id.size()))
                   == artifact_id;
        }
    );
    return reopened_artifact != reopened.artifacts.end() && reopened_artifact->has_image_preview
           && reopened_artifact->image_width == 3 && reopened_artifact->image_height == 2;
}

bool run_smoke_text_cycle(DesktopBackend& backend) {
    const QVariantList artifacts = backend.artifacts();
    const auto text_artifact =
        std::find_if(artifacts.cbegin(), artifacts.cend(), [](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("kindKey")).toString()
                   == QStringLiteral("text_document");
        });
    if (text_artifact == artifacts.cend()) {
        std::cerr << "desktop text smoke found no text document" << std::endl;
        return false;
    }

    const QVariantMap accepted = text_artifact->toMap();
    const QString artifact_id = accepted.value(QStringLiteral("id")).toString();
    const QString accepted_revision =
        accepted.value(QStringLiteral("acceptedRevisionId")).toString();
    const QString replacement = QStringLiteral("A desktop candidate accepted after review.");

    if (!backend.proposeTextCandidate(
            artifact_id,
            QStringLiteral("A desktop candidate kept as another option.")
        )) {
        std::cerr << "desktop text smoke could not create first shelf candidate" << std::endl;
        return false;
    }
    const QString first_candidate_id = backend.candidateId();
    if (!backend.proposeTextCandidate(artifact_id, replacement) || !backend.hasCandidate()
        || backend.candidateCount() != 2 || backend.candidates().size() != 2) {
        std::cerr << "desktop text smoke could not accumulate candidate shelf" << std::endl;
        return false;
    }
    const QString accepted_candidate_id = backend.candidateId();
    if (accepted_candidate_id == first_candidate_id
        || backend.candidates().first().toMap().value(QStringLiteral("id")).toString()
               != accepted_candidate_id) {
        std::cerr << "desktop text smoke did not select the newest candidate" << std::endl;
        return false;
    }
    if (!backend.discardCandidate(first_candidate_id) || backend.candidateCount() != 1
        || backend.candidateId() != accepted_candidate_id) {
        std::cerr << "desktop text smoke could not discard an exact shelf candidate" << std::endl;
        return false;
    }

    const QVariantList before_accept_artifacts = backend.artifacts();
    const auto before_accept_artifact = std::find_if(
        before_accept_artifacts.cbegin(),
        before_accept_artifacts.cend(),
        [&artifact_id](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("id")).toString() == artifact_id;
        }
    );
    if (before_accept_artifact == before_accept_artifacts.cend()
        || before_accept_artifact->toMap().value(QStringLiteral("acceptedRevisionId")).toString()
               != accepted_revision) {
        std::cerr << "desktop candidate changed accepted history before acceptance" << std::endl;
        return false;
    }

    if (!backend.acceptCandidate(accepted_candidate_id) || backend.hasCandidate()
        || backend.candidateCount() != 0) {
        std::cerr << "desktop text smoke could not accept candidate" << std::endl;
        return false;
    }

    const QVariantList committed_artifacts = backend.artifacts();
    const auto committed_artifact = std::find_if(
        committed_artifacts.cbegin(),
        committed_artifacts.cend(),
        [&artifact_id](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("id")).toString() == artifact_id;
        }
    );
    if (committed_artifact == committed_artifacts.cend()) {
        std::cerr << "desktop text smoke lost committed artifact" << std::endl;
        return false;
    }

    const QVariantMap committed = committed_artifact->toMap();
    const QString committed_revision =
        committed.value(QStringLiteral("acceptedRevisionId")).toString();
    if (committed_revision == accepted_revision
        || committed.value(QStringLiteral("textPreview")).toString() != replacement) {
        std::cerr << "desktop text smoke committed an unexpected revision" << std::endl;
        return false;
    }

    const QString branch_replacement =
        QStringLiteral("A desktop candidate accepted as a separate artifact.");
    if (!backend.proposeTextCandidate(
            artifact_id,
            QStringLiteral("A sibling candidate that remains on the shelf.")
        )) {
        std::cerr << "desktop text smoke could not create branch sibling" << std::endl;
        return false;
    }
    const QString branch_sibling_id = backend.candidateId();
    if (!backend.proposeTextCandidate(artifact_id, branch_replacement) || !backend.hasCandidate()
        || backend.candidateCount() != 2) {
        std::cerr << "desktop text smoke could not create branch candidate" << std::endl;
        return false;
    }
    const QString branch_candidate_id = backend.candidateId();
    const QString branch_name =
        committed.value(QStringLiteral("name")).toString() + QStringLiteral(" — Branch");
    if (!backend.branchCandidate(branch_candidate_id, branch_name) || backend.candidateCount() != 1
        || backend.candidateId() != branch_sibling_id) {
        std::cerr << "desktop text smoke could not branch candidate" << std::endl;
        return false;
    }

    const QVariantList branched_artifacts = backend.artifacts();
    const auto branched_source = std::find_if(
        branched_artifacts.cbegin(),
        branched_artifacts.cend(),
        [&artifact_id](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("id")).toString() == artifact_id;
        }
    );
    const auto branch = std::find_if(
        branched_artifacts.cbegin(),
        branched_artifacts.cend(),
        [&branch_name](const QVariant& artifact) {
            return artifact.toMap().value(QStringLiteral("name")).toString() == branch_name;
        }
    );
    if (branched_source == branched_artifacts.cend() || branch == branched_artifacts.cend()
        || branched_source->toMap().value(QStringLiteral("acceptedRevisionId")).toString()
               != committed_revision) {
        std::cerr << "desktop text smoke changed or lost the branch source" << std::endl;
        return false;
    }
    const QVariantMap branch_artifact = branch->toMap();
    const QVariantList branch_inputs =
        branch_artifact.value(QStringLiteral("transformationInputArtifactIds")).toList();
    const QVariantList graph_edges = backend.graphEdges();
    if (branch_artifact.value(QStringLiteral("textPreview")).toString() != branch_replacement
        || !branch_artifact.value(QStringLiteral("acceptedParentRevisionIds")).toList().isEmpty()
        || branch_inputs.size() != 1 || branch_inputs.first().toString() != artifact_id
        || graph_edges.size() != 1
        || graph_edges.first().toMap().value(QStringLiteral("sourceArtifactId")).toString()
               != artifact_id
        || graph_edges.first().toMap().value(QStringLiteral("targetArtifactId")).toString()
               != branch_artifact.value(QStringLiteral("id")).toString()) {
        std::cerr << "desktop text smoke projected invalid branch lineage" << std::endl;
        return false;
    }
    if (!backend.discardCandidate(branch_sibling_id) || backend.hasCandidate()) {
        std::cerr << "desktop text smoke could not clear remaining shelf candidate" << std::endl;
        return false;
    }

    return true;
}

bool verify_candidate_shelf_interaction(DesktopBackend& backend, QObject& root_object) {
    const int selected_index = root_object.property("selectedArtifactIndex").toInt();
    const QVariantList artifacts = backend.artifacts();
    if (selected_index < 0 || selected_index >= artifacts.size()) {
        std::cerr << "desktop shelf smoke has no selected artifact" << std::endl;
        return false;
    }
    const QString artifact_id =
        artifacts[selected_index].toMap().value(QStringLiteral("id")).toString();
    if (!backend.proposeTextCandidate(artifact_id, QStringLiteral("Candidate shelf option A."))) {
        std::cerr << "desktop shelf smoke could not create first option" << std::endl;
        return false;
    }
    const QString first_candidate_id = backend.candidateId();
    if (!backend.proposeTextCandidate(artifact_id, QStringLiteral("Candidate shelf option B."))) {
        std::cerr << "desktop shelf smoke could not create second option" << std::endl;
        return false;
    }
    const QString second_candidate_id = backend.candidateId();
    QCoreApplication::processEvents();

    QObject* const shelf = root_object.findChild<QObject*>(QStringLiteral("candidateFilmstrip"));
    if (shelf == nullptr) {
        std::cerr << "desktop shelf smoke could not find packaged shelf" << std::endl;
        return false;
    }
    QQmlExpression selection(
        QQmlEngine::contextForObject(shelf),
        shelf,
        QStringLiteral("select(\"%1\")").arg(first_candidate_id)
    );
    selection.evaluate();
    if (selection.hasError()) {
        std::cerr << "desktop shelf smoke could not select first option: "
                  << selection.error().toString().toStdString() << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    if (backend.candidateId() != first_candidate_id
        || root_object.property("selectedCandidateId").toString() != first_candidate_id) {
        std::cerr << "desktop shelf smoke did not synchronize candidate selection" << std::endl;
        return false;
    }
    if (!workspace_host_smoke::verifyOperatorRoute(
            root_object,
            QStringLiteral("text.edit"),
            QStringLiteral("operator.text.edit"),
            QStringLiteral("textAuthoringWorkspace"),
            first_candidate_id
        )) {
        std::cerr << "desktop shelf smoke did not pass Candidate selection into the Host"
                  << std::endl;
        return false;
    }
    if (!backend.discardCandidate(second_candidate_id)
        || !backend.discardCandidate(first_candidate_id) || backend.hasCandidate()) {
        std::cerr << "desktop shelf smoke could not clear transient options" << std::endl;
        return false;
    }
    return true;
}

bool verify_infer_runtime_surface(QObject& root_object) {
    if (root_object.findChild<QObject*>(QStringLiteral("inferCredentialField")) == nullptr) {
        std::cerr << "desktop runtime smoke could not find packaged access controls" << std::endl;
        return false;
    }
    return true;
}

bool run_smoke_project_authoring(DesktopBackend& backend, QObject& root_object) {
    QTemporaryDir project_parent;
    if (!project_parent.isValid()
        || !backend.createProject(
            QUrl::fromLocalFile(project_parent.path()),
            QStringLiteral("Desktop Authoring")
        )
        || backend.artifactCount() != 0) {
        std::cerr << "desktop authoring smoke could not create its empty project" << std::endl;
        return false;
    }

    QObject* const creative_start =
        root_object.findChild<QObject*>(QStringLiteral("createSceneTypeDialog"));
    if (creative_start == nullptr
        || !QMetaObject::invokeMethod(creative_start, "openForCreation", Qt::DirectConnection)) {
        std::cerr << "desktop authoring smoke could not open the creative start chooser"
                  << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    if (!creative_start->property("visible").toBool()
        || root_object.findChild<QObject*>(QStringLiteral("createTextSceneTypeButton")) == nullptr
        || root_object.findChild<QObject*>(QStringLiteral("createAiImageSceneTypeButton"))
               == nullptr
        || root_object.findChild<QObject*>(QStringLiteral("importMaterialStartButton")) == nullptr) {
        std::cerr << "desktop authoring smoke found an incomplete creative start chooser"
                  << std::endl;
        return false;
    }
    QMetaObject::invokeMethod(creative_start, "close", Qt::DirectConnection);

    if (!backend.createTextScene(
            QStringLiteral("Opening"),
            QStringLiteral("A first accepted Scene source.")
        )
        || backend.artifactCount() != 1) {
        std::cerr << "desktop authoring smoke could not create its project and first Scene"
                  << std::endl;
        return false;
    }
    root_object.setProperty("selectedArtifactIndex", 0);
    QCoreApplication::processEvents();
    QObject* const text_creation =
        root_object.findChild<QObject*>(QStringLiteral("createTextSceneDialog"));
    QObject* const workspace_surface =
        root_object.findChild<QObject*>(QStringLiteral("workspaceSurface"));
    if (text_creation == nullptr || workspace_surface == nullptr
        || !QMetaObject::invokeMethod(text_creation, "sceneCreated", Qt::DirectConnection)) {
        std::cerr << "desktop authoring smoke could not complete the text creation handoff"
                  << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    QCoreApplication::processEvents();
    const QVariantList automatic_drafts = backend.operatorDrafts();
    if (automatic_drafts.size() != 1
        || automatic_drafts.first().toMap().value(QStringLiteral("operatorTypeKey")).toString()
               != QStringLiteral("text.edit")
        || workspace_surface->property("workspaceRouteKey").toString()
               != QStringLiteral("operator.text.edit")) {
        std::cerr << "desktop authoring smoke did not enter the writing workspace directly"
                  << std::endl;
        return false;
    }
    if (!backend.discardOperatorDraft(
            automatic_drafts.first().toMap().value(QStringLiteral("id")).toString()
        )
        || !QMetaObject::invokeMethod(workspace_surface, "showGraph", Qt::DirectConnection)) {
        return false;
    }
    QCoreApplication::processEvents();
    const QString bundle_path = backend.bundlePath();
    if (!workspace_host_smoke::verifyOperatorDraftRoute(root_object, backend)
        || !backend.openProject(QUrl::fromLocalFile(bundle_path)) || backend.artifactCount() != 1
        || !backend.operatorDrafts().isEmpty()) {
        std::cerr << "desktop authoring smoke did not reopen without the discarded draft"
                  << std::endl;
        return false;
    }
    if (!backend.createAiImageScene(
            QStringLiteral("Concept image"),
            QStringLiteral("A cobalt glass bird on a quiet grey background"),
            1536,
            1024
        )
        || backend.artifactCount() != 2 || backend.operatorDrafts().size() != 1) {
        std::cerr << "desktop authoring smoke could not create the zero-input AI image Scene"
                  << std::endl;
        return false;
    }
    const QVariantMap image_draft = backend.operatorDrafts().first().toMap();
    const QString image_draft_id = image_draft.value(QStringLiteral("id")).toString();
    if (image_draft.value(QStringLiteral("operatorTypeKey")).toString()
            != QStringLiteral("image.generate")
        || image_draft.value(QStringLiteral("hasInputDataType")).toBool()
        || image_draft.value(QStringLiteral("aiImageOutputWidth")).toInt() != 1536
        || image_draft.value(QStringLiteral("aiImageOutputHeight")).toInt() != 1024
        || image_draft.value(QStringLiteral("aiImageCandidateCount")).toInt() != 1) {
        std::cerr << "desktop authoring smoke projected a different AI image draft" << std::endl;
        return false;
    }
    root_object.setProperty("selectedArtifactIndex", 1);
    QCoreApplication::processEvents();
    QQmlExpression open_image_draft(
        QQmlEngine::contextForObject(workspace_surface),
        workspace_surface,
        QStringLiteral("openOperatorDraft(\"") + image_draft_id + QStringLiteral("\")")
    );
    if (!open_image_draft.evaluate().toBool() || open_image_draft.hasError()
        || workspace_surface->property("workspaceRouteKey").toString()
               != QStringLiteral("operator.image.generate")
        || workspace_surface->property("loadedWorkspaceObjectName").toString()
               != QStringLiteral("aiImageOperatorWorkspace")
        || !workspace_surface->property("aiImageIntentActive").toBool()
        || root_object.findChild<QObject*>(QStringLiteral("operatorIntentSidebar")) == nullptr
        || root_object.findChild<QObject*>(QStringLiteral("aiImageCanvasPicker")) == nullptr) {
        std::cerr << "desktop authoring smoke did not open the AI image workbench" << std::endl;
        return false;
    }
    auto* image_intent = root_object.findChild<QObject*>(QStringLiteral("operatorIntentSidebar"));
    auto* image_count_picker = image_intent
                                   ? image_intent->findChild<QObject*>(
                                         QStringLiteral("imageCandidateCountPicker")
                                     )
                                   : nullptr;
    if (!image_count_picker
        || !QMetaObject::invokeMethod(
            image_count_picker, "activated", Qt::DirectConnection, Q_ARG(int, 2)
        )
        || image_intent->property("selectedCandidateCount").toInt() != 3
        || backend.operatorDrafts().first().toMap().value(QStringLiteral("aiImageCandidateCount"))
               .toInt() != 3) {
        std::cerr << "desktop authoring smoke did not persist the image candidate count"
                  << std::endl;
        return false;
    }
    auto* image_model_picker = image_intent->findChild<QObject*>(QStringLiteral("imageModelPicker"));
    auto* image_model_choice = image_model_picker
                                   ? image_model_picker->findChild<QObject*>(QStringLiteral("aiModelChoice"))
                                   : nullptr;
    auto* image_effort_choice = image_model_picker
                                    ? image_model_picker->findChild<QObject*>(QStringLiteral("aiEffortChoice"))
                                    : nullptr;
    if (!image_model_choice
        || image_intent->property("selectedModelKey").toString()
               != image_intent->property("defaultModelKey").toString()
        || !QMetaObject::invokeMethod(
            image_model_choice, "activated", Qt::DirectConnection, Q_ARG(int, 3)
        )
        || image_intent->property("selectedModelKey").toString() != QStringLiteral("gpt_6_sol")
        || !image_effort_choice
        || !QMetaObject::invokeMethod(
            image_effort_choice, "activated", Qt::DirectConnection, Q_ARG(int, 4)
        )
        || image_intent->property("selectedEffortKey").toString() != QStringLiteral("high")
        || !QMetaObject::invokeMethod(
            image_model_choice, "activated", Qt::DirectConnection, Q_ARG(int, 0)
        )
        || image_intent->property("selectedModelKey").toString()
               != image_intent->property("defaultModelKey").toString()
        || image_intent->property("selectedEffortKey").toString() != QStringLiteral("high")) {
        std::cerr << "desktop authoring smoke did not switch the image model and effort locally"
                  << std::endl;
        return false;
    }
    if (!backend.openProject(QUrl::fromLocalFile(bundle_path)) || backend.artifactCount() != 2
        || backend.operatorDrafts().size() != 1
        || backend.operatorDrafts().first().toMap().value(QStringLiteral("id")).toString()
               != image_draft_id
        || backend.operatorDrafts().first().toMap()
                   .value(QStringLiteral("aiImageCandidateCount"))
                   .toInt() != 3) {
        std::cerr << "desktop authoring smoke did not reopen the exact AI image source draft"
                  << std::endl;
        return false;
    }
    if (!workspace_host_smoke::verifyProjectedDraftRemoval(root_object, backend)
        || !workspace_host_smoke::verifyManualSourceAcrossSpeechSelection(root_object, backend)) {
        return false;
    }
    QTemporaryDir material_directory;
    if (!material_directory.isValid()) return false;
    const QString code_path = material_directory.filePath(QStringLiteral("animation.js"));
    const QString wav_path = material_directory.filePath(QStringLiteral("voice.wav"));
    const QByteArray code("export const frame = time => time * 2;\n");
    const QByteArray wav = QByteArray::fromHex(
        "524946462800000057415645666d74201000000001000100c05d000080bb0000"
        "02001000646174610400000000000000"
    );
    QFile code_file(code_path);
    QFile wav_file(wav_path);
    if (!code_file.open(QIODevice::WriteOnly) || code_file.write(code) != code.size()
        || !wav_file.open(QIODevice::WriteOnly) || wav_file.write(wav) != wav.size()) {
        return false;
    }
    code_file.close();
    wav_file.close();
    const int prior_count = backend.artifactCount();
    if (!backend.importMaterial(QUrl::fromLocalFile(code_path))
        || backend.artifactCount() != prior_count + 1
        || backend.artifacts().last().toMap().value(QStringLiteral("kindKey")).toString()
               != QStringLiteral("text_document")
        || !backend.importMaterial(QUrl::fromLocalFile(wav_path))
        || backend.artifactCount() != prior_count + 2) {
        std::cerr << "desktop authoring smoke could not import external material" << std::endl;
        return false;
    }
    const QString text_id = backend.artifacts()[prior_count].toMap().value(QStringLiteral("id")).toString();
    const QString exported_text = material_directory.filePath(QStringLiteral("accepted.txt"));
    QFile exported_text_file(exported_text);
    if (!backend.exportAcceptedMaterial(text_id, QUrl::fromLocalFile(exported_text))
        || !exported_text_file.open(QIODevice::ReadOnly)
        || exported_text_file.readAll() != code) {
        std::cerr << "desktop authoring smoke could not export accepted UTF-8 text" << std::endl;
        return false;
    }
    const QVariantMap audio = backend.artifacts().last().toMap();
    const QString audio_id = audio.value(QStringLiteral("id")).toString();
    const auto preview = backend.audioPreview(audio_id);
    if (audio.value(QStringLiteral("kindKey")).toString() != QStringLiteral("audio_clip")
        || audio.value(QStringLiteral("audioOriginKey")).toString()
               != QStringLiteral("imported_unverified")
        || !preview.has_value() || preview->wav_bytes != wav
        || !backend.openProject(QUrl::fromLocalFile(bundle_path))
        || !backend.audioPreview(audio_id).has_value()) {
        std::cerr << "desktop authoring smoke lost imported WAV provenance or playback"
                  << std::endl;
        return false;
    }
    return true;
}

} // namespace

int main(int argc, char* argv[]) {
    const auto arguments = parse_arguments(argc, argv);
    if (!arguments.has_value()) {
        std::cerr << "usage: shape-desktop [--project PROJECT.shape] [--smoke-exit] "
                     "[--smoke-text-cycle] [--smoke-raster-cycle]"
                  << std::endl;
        return 2;
    }

    qputenv("QT_QUICK_CONTROLS_STYLE", "Basic");
    QGuiApplication application(argc, argv);
    application.setApplicationName(QStringLiteral("Shape"));
    application.setOrganizationName(QStringLiteral("Shape"));
    std::unique_ptr<DesktopBackend> backend;
    try {
        if (arguments->project_path.has_value()) {
            backend = std::make_unique<DesktopBackend>(
                shape::desktop::open_desktop_session(*arguments->project_path)
            );
        } else {
            backend = std::make_unique<DesktopBackend>();
        }
    } catch (const rust::Error& error) {
        std::cerr << "cannot open Shape project: " << error.what() << std::endl;
        return 1;
    }

    InferRuntimeController infer_runtime(&application);
    const QString infer_credential_path =
        QDir(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation))
            .filePath(QStringLiteral("secrets/infer-runtime.token"));
    InferTextController infer_text(*backend, infer_credential_path, &application);
    InferSpeechController infer_speech(*backend, infer_credential_path, &application);
    InferImageController infer_image(*backend, infer_credential_path, &application);
    AudioPreviewController audio_preview(*backend, &application);
    AudioExportController audio_export(*backend, &application);
    UiPreferences ui_preferences(application);
    RecentProjects recent_projects;
    const bool smoke_mode = arguments->smoke_exit || arguments->smoke_text_cycle
                            || arguments->smoke_raster_cycle
                            || arguments->speech_live_directory.has_value()
                            || arguments->speech_script_directory.has_value()
                            || arguments->text_authoring_directory.has_value()
                            || arguments->image_live_directory.has_value();
    if (!smoke_mode) {
        const auto record_project = [&recent_projects, project = backend.get()] {
            if (project->projectOpen()) {
                recent_projects.record(project->bundlePath(), project->projectName());
            }
        };
        QObject::connect(backend.get(), &DesktopBackend::projectChanged, &recent_projects,
                         record_project);
        record_project();
    }
    QObject::connect(
        &ui_preferences,
        &UiPreferences::languageModeChanged,
        backend.get(),
        &DesktopBackend::retranslate
    );
    backend->retranslate();
    QQmlApplicationEngine engine;
    engine.addImageProvider(
        QStringLiteral("shape-preview"),
        new ImagePreviewProvider(backend->imagePreviewStore())
    );
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &application,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection
    );
    ui_preferences.attachEngine(engine);
    engine.setInitialProperties({
        {QStringLiteral("backend"), QVariant::fromValue(backend.get())},
        {QStringLiteral("inferRuntime"), QVariant::fromValue(&infer_runtime)},
        {QStringLiteral("inferText"), QVariant::fromValue(&infer_text)},
        {QStringLiteral("inferSpeech"), QVariant::fromValue(&infer_speech)},
        {QStringLiteral("inferImage"), QVariant::fromValue(&infer_image)},
        {QStringLiteral("audioPreview"), QVariant::fromValue(&audio_preview)},
        {QStringLiteral("audioExport"), QVariant::fromValue(&audio_export)},
        {QStringLiteral("uiPreferences"), QVariant::fromValue(&ui_preferences)},
        {QStringLiteral("recentProjects"), QVariant::fromValue(&recent_projects)},
    });
    infer_runtime.refresh();
    engine.loadFromModule(QStringLiteral("Shape.Desktop"), QStringLiteral("Main"));

    if (engine.rootObjects().isEmpty()) {
        std::cerr << "Shape QML shell failed to load" << std::endl;
        return 1;
    }

    QObject* const root_object = engine.rootObjects().first();

#if defined(Q_OS_MACOS)
    QObject* const title_toolbar = root_object->findChild<QObject*>(QStringLiteral("titleToolBar"));
    const int title_bar_height =
        title_toolbar == nullptr ? 44 : qRound(title_toolbar->property("height").toReal());
    installMacTitleBarAlignment(qobject_cast<QQuickWindow*>(root_object), title_bar_height);
#endif

    if (arguments->image_live_directory.has_value()) {
        QTimer::singleShot(0, &application, [&]() {
            const bool passed = image_live_smoke::run(
                *backend,
                infer_image,
                *root_object,
                *arguments->image_live_directory
            );
            QCoreApplication::exit(passed ? 0 : 8);
        });
    }
    if (arguments->speech_script_directory.has_value()) {
        QTimer::singleShot(0, &application, [&]() {
            const bool passed = speech_live_smoke::run_script(
                *backend,
                infer_speech,
                audio_preview,
                audio_export,
                *root_object,
                *arguments->speech_script_directory
            );
            QCoreApplication::exit(passed ? 0 : 6);
        });
    }
    if (arguments->text_authoring_directory.has_value()) {
        QTimer::singleShot(0, &application, [&]() {
            const bool passed = text_authoring_smoke::runLive(
                *backend,
                infer_text,
                infer_speech,
                audio_export,
                *root_object,
                *arguments->text_authoring_directory
            );
            QCoreApplication::exit(passed ? 0 : 7);
        });
    }
    if (arguments->speech_live_directory.has_value()) {
        QTimer::singleShot(0, &application, [&]() {
            const bool passed = speech_live_smoke::run(
                *backend,
                infer_speech,
                audio_preview,
                audio_export,
                *root_object,
                *arguments->speech_live_directory
            );
            QCoreApplication::exit(passed ? 0 : 6);
        });
    }
    if (arguments->smoke_exit) {
        const bool began_without_project = !backend->projectOpen();
        if (!workspace_host_smoke::verifyRecentProjects()
            || (began_without_project && !workspace_host_smoke::verifyProjectWelcome(*root_object))
            || !verify_infer_runtime_surface(*root_object)
            || !workspace_host_smoke::verifyLocalization(*root_object, ui_preferences)) {
            return 3;
        }
        if (began_without_project && !run_smoke_project_authoring(*backend, *root_object)) {
            return 3;
        }
        if (arguments->project_path.has_value()
            && (!backend->projectOpen() || backend->artifactCount() == 0)) {
            std::cerr << "Shape project smoke loaded no artifacts" << std::endl;
            return 3;
        }
        if (arguments->smoke_text_cycle
            && (!run_smoke_text_cycle(*backend)
                || !workspace_host_smoke::verifySceneGraphRoutes(*root_object)
                || !verify_candidate_shelf_interaction(*backend, *root_object))) {
            return 4;
        }
        if (arguments->smoke_raster_cycle
            && (!arguments->project_path.has_value()
                || !run_smoke_raster_cycle(*backend, *root_object, *arguments->project_path))) {
            return 5;
        }
        if (arguments->smoke_text_cycle && !text_authoring_smoke::verify(*backend, *root_object)) {
            return 4;
        }
        QTimer::singleShot(0, &application, &QCoreApplication::quit);
    }
    return application.exec();
}

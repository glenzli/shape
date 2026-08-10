#include "audio_preview_controller.hpp"
#include "desktop_backend.hpp"
#include "image_preview_provider.hpp"
#include "infer_runtime_controller.hpp"
#include "infer_speech_controller.hpp"
#include "infer_text_controller.hpp"
#include "ui_preferences.hpp"
#include "workspace_host_smoke.hpp"

#if defined(Q_OS_MACOS)
#include "mac_titlebar.hpp"
#endif

#include "rust/cxx.h"

#include <QColor>
#include <QDir>
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
    if (!backend.prepareImagePreviews(artifact_id)
        || !workspace_host_smoke::verifyOperatorRoute(
            root_object,
            QStringLiteral("image.crop"),
            QStringLiteral("operator.image.crop"),
            QStringLiteral("imageCropOperatorWorkspace")
        )) {
        std::cerr << "desktop raster smoke did not route image.crop" << std::endl;
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
           && reopened_artifact->image_width == 4 && reopened_artifact->image_height == 3;
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

    QObject* const shelf = root_object.findChild<QObject*>(QStringLiteral("candidateShelf"));
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
            QStringLiteral("textEditOperatorWorkspace"),
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
    if (root_object.findChild<QObject*>(QStringLiteral("inferRuntimeStatusButton")) == nullptr
        || root_object.findChild<QObject*>(QStringLiteral("inferGenerateButton")) == nullptr
        || root_object.findChild<QObject*>(QStringLiteral("inferCredentialField")) == nullptr) {
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
        || backend.artifactCount() != 0
        || !backend.createTextScene(
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
    const QString bundle_path = backend.bundlePath();
    if (!workspace_host_smoke::verifyOperatorDraftRoute(root_object, backend)
        || !backend.openProject(QUrl::fromLocalFile(bundle_path))
        || backend.artifactCount() != 1 || !backend.operatorDrafts().isEmpty()) {
        std::cerr << "desktop authoring smoke did not reopen without the discarded draft"
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
    AudioPreviewController audio_preview(*backend, &application);
    UiPreferences ui_preferences(application);
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
        {QStringLiteral("audioPreview"), QVariant::fromValue(&audio_preview)},
        {QStringLiteral("uiPreferences"), QVariant::fromValue(&ui_preferences)},
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

    if (arguments->smoke_exit) {
        const bool began_without_project = !backend->projectOpen();
        if ((began_without_project && !workspace_host_smoke::verifyProjectWelcome(*root_object))
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
        QTimer::singleShot(0, &application, &QCoreApplication::quit);
    }
    return application.exec();
}

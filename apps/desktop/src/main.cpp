#include "desktop_backend.hpp"
#include "ui_preferences.hpp"

#if defined(Q_OS_MACOS)
#include "mac_titlebar.hpp"
#endif

#include "rust/cxx.h"

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickWindow>
#include <QTimer>
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
        } else {
            return std::nullopt;
        }
    }
    return arguments;
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

    if (!backend.proposeTextCandidate(artifact_id, replacement) || !backend.hasCandidate()) {
        std::cerr << "desktop text smoke could not create candidate" << std::endl;
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

    if (!backend.acceptCandidate() || backend.hasCandidate()) {
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
    if (committed.value(QStringLiteral("acceptedRevisionId")).toString() == accepted_revision
        || committed.value(QStringLiteral("textPreview")).toString() != replacement) {
        std::cerr << "desktop text smoke committed an unexpected revision" << std::endl;
        return false;
    }

    return true;
}

} // namespace

int main(int argc, char* argv[]) {
    const auto arguments = parse_arguments(argc, argv);
    if (!arguments.has_value()) {
        std::cerr << "usage: shape-desktop [--project PROJECT.shape] [--smoke-exit] "
                     "[--smoke-text-cycle]"
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

    UiPreferences ui_preferences(application);
    QObject::connect(
        &ui_preferences,
        &UiPreferences::languageModeChanged,
        backend.get(),
        &DesktopBackend::retranslate
    );
    backend->retranslate();
    QQmlApplicationEngine engine;
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
        {QStringLiteral("uiPreferences"), QVariant::fromValue(&ui_preferences)},
    });
    engine.loadFromModule(QStringLiteral("Shape.Desktop"), QStringLiteral("Main"));

    if (engine.rootObjects().isEmpty()) {
        std::cerr << "Shape QML shell failed to load" << std::endl;
        return 1;
    }

#if defined(Q_OS_MACOS)
    QObject* const root_object = engine.rootObjects().first();
    QObject* const title_toolbar = root_object->findChild<QObject*>(QStringLiteral("titleToolBar"));
    const int title_bar_height =
        title_toolbar == nullptr ? 44 : qRound(title_toolbar->property("height").toReal());
    installMacTitleBarAlignment(qobject_cast<QQuickWindow*>(root_object), title_bar_height);
#endif

    if (arguments->smoke_exit) {
        if (arguments->project_path.has_value()
            && (!backend->projectOpen() || backend->artifactCount() == 0)) {
            std::cerr << "Shape project smoke loaded no artifacts" << std::endl;
            return 3;
        }
        if (arguments->smoke_text_cycle && !run_smoke_text_cycle(*backend)) {
            return 4;
        }
        QTimer::singleShot(0, &application, &QCoreApplication::quit);
    }
    return application.exec();
}

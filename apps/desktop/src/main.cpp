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

#include <iostream>
#include <memory>
#include <optional>
#include <string>

namespace {

struct Arguments {
    std::optional<std::string> project_path;
    bool smoke_exit = false;
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
        } else {
            return std::nullopt;
        }
    }
    return arguments;
}

} // namespace

int main(int argc, char* argv[]) {
    const auto arguments = parse_arguments(argc, argv);
    if (!arguments.has_value()) {
        std::cerr << "usage: shape-desktop [--project PROJECT.shape] [--smoke-exit]" << std::endl;
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
                shape::desktop::load_project_snapshot(*arguments->project_path)
            );
        } else {
            backend = std::make_unique<DesktopBackend>();
        }
    } catch (const rust::Error& error) {
        std::cerr << "cannot open Shape project: " << error.what() << std::endl;
        return 1;
    }

    UiPreferences ui_preferences(application);
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
        QTimer::singleShot(0, &application, &QCoreApplication::quit);
    }
    return application.exec();
}

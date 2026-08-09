#include <QGuiApplication>
#include <QLocale>
#include <QQmlApplicationEngine>
#include <QTimer>
#include <QTranslator>

int main(int argc, char* argv[]) {
  QGuiApplication application(argc, argv);
  application.setApplicationName(QStringLiteral("Shape"));
  application.setOrganizationName(QStringLiteral("Shape"));

  QTranslator translator;
  if (translator.load(QLocale(), QStringLiteral("shape"), QStringLiteral("_"),
                      QStringLiteral(":/translations"))) {
    application.installTranslator(&translator);
  }

  QQmlApplicationEngine engine;
  QObject::connect(
      &engine, &QQmlApplicationEngine::objectCreationFailed, &application,
      []() { QCoreApplication::exit(1); }, Qt::QueuedConnection);
  engine.loadFromModule(QStringLiteral("Shape.Desktop"), QStringLiteral("Main"));

  if (application.arguments().contains(QStringLiteral("--smoke-exit"))) {
    QTimer::singleShot(0, &application, &QCoreApplication::quit);
  }
  return application.exec();
}

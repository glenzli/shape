//! Exact external Deck edit lifecycle and verified local output links.
#pragma once
#include <QFutureWatcher>
#include <QJsonObject>
#include <QObject>
#include <QTimer>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>
#include <memory>
class DesktopBackend;
class QSettings;

class DeckHandoffController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Created by the Shape host")
    Q_PROPERTY(bool busy READ busy NOTIFY changed)
    Q_PROPERTY(bool waiting READ waiting NOTIFY changed)
    Q_PROPERTY(QString executable READ executable NOTIFY changed)
    Q_PROPERTY(QString error READ error NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
    Q_PROPERTY(QString kitId READ kitId NOTIFY changed)
    Q_PROPERTY(QString requestPath READ requestPath NOTIFY changed)
    Q_PROPERTY(QVariantList outputs READ outputs NOTIFY changed)
    Q_PROPERTY(bool hasDocument READ hasDocument NOTIFY changed)
    Q_PROPERTY(bool canRetry READ canRetry NOTIFY changed)
  public:
    DeckHandoffController(
        DesktopBackend& backend,
        const QString& settingsPath,
        QObject* parent = nullptr
    );
    ~DeckHandoffController() override;
    bool busy() const {
        return work_ != Work::None;
    }
    bool waiting() const {
        return waiting_;
    }
    QString executable() const;
    QString error() const {
        return tr(error_.toUtf8().constData());
    }
    QString status() const {
        return tr(status_.toUtf8().constData());
    }
    QString kitId() const {
        return kit_id_;
    }
    QString requestPath() const {
        return record_.value("request_path").toString();
    }
    QVariantList outputs() const;
    bool hasDocument() const {
        return record_.value("document").isObject();
    }
    bool canRetry() const;
    Q_INVOKABLE bool setExecutable(const QUrl& path);
    Q_INVOKABLE bool attach(const QString& kitId);
    Q_INVOKABLE void
    openKit(const QString& kitId, const QString& draftDigest, bool freshSnapshot = false);
    Q_INVOKABLE void openNewDeck();
    Q_INVOKABLE void readReturnedOutputs();
    Q_INVOKABLE void stopWaiting();
    Q_INVOKABLE void retryHandoff();
    Q_INVOKABLE bool openOutput(int index, bool reveal = false);
    void retranslate() {
        emit changed();
    }
  signals:
    void changed();

  private:
    void finish();
    void launch();
    void poll();
    void saveRecord();
    QString key(const QString& kitId) const;
    bool contextCurrent() const;
    void fail(const QString& message);
    DesktopBackend& backend_;
    std::unique_ptr<QSettings> settings_;
    QFutureWatcher<QJsonObject> watcher_;
    QTimer poll_;
    QString configured_executable_, project_path_, project_id_, kit_id_, key_, error_, status_;
    QJsonObject record_;
    quint64 generation_ = 0;
    quint64 work_generation_ = 0;
    enum class Work { None, Prepare, Verify, Standalone, Relaunch } work_ = Work::None;
    bool waiting_ = false;
    QString handoff_root_;
    int pending_open_index_ = -1;
    bool pending_reveal_ = false;
};

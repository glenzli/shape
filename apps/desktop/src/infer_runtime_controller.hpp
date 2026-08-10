//! Asynchronous desktop lifecycle for Infer Runtime contract availability.

#pragma once

#include <QFutureWatcher>
#include <QObject>
#include <QString>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class InferRuntimeController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("InferRuntimeController is created by the host application")
    Q_PROPERTY(bool probing READ probing NOTIFY statusChanged)
    Q_PROPERTY(bool reachable READ reachable NOTIFY statusChanged)
    Q_PROPERTY(bool compatible READ compatible NOTIFY statusChanged)
    Q_PROPERTY(bool available READ available NOTIFY statusChanged)
    Q_PROPERTY(QString contractVersion READ contractVersion NOTIFY statusChanged)
    Q_PROPERTY(QString errorCode READ errorCode NOTIFY statusChanged)
    Q_PROPERTY(QString endpointOrigin READ endpointOrigin NOTIFY statusChanged)
    Q_PROPERTY(QString endpointSource READ endpointSource NOTIFY statusChanged)
    Q_PROPERTY(QString runtimeInstanceId READ runtimeInstanceId NOTIFY statusChanged)
    Q_PROPERTY(QString runtimeGeneration READ runtimeGeneration NOTIFY statusChanged)

  public:
    explicit InferRuntimeController(QObject* parent = nullptr);
    ~InferRuntimeController() override;

    [[nodiscard]] bool probing() const;
    [[nodiscard]] bool reachable() const;
    [[nodiscard]] bool compatible() const;
    [[nodiscard]] bool available() const;
    [[nodiscard]] QString contractVersion() const;
    [[nodiscard]] QString errorCode() const;
    [[nodiscard]] QString endpointOrigin() const;
    [[nodiscard]] QString endpointSource() const;
    [[nodiscard]] QString runtimeInstanceId() const;
    [[nodiscard]] QString runtimeGeneration() const;

    /// Coalesces refresh requests while one bounded probe is active.
    Q_INVOKABLE void refresh();

  signals:
    void statusChanged();

  private:
    void finishProbe();

    QString explicit_override_;
    QFutureWatcher<QVariantMap> watcher_;
    quint64 generation_ = 0;
    bool refresh_queued_ = false;
    bool probing_ = false;
    bool reachable_ = false;
    bool compatible_ = false;
    QString contract_version_;
    QString error_code_;
    QString endpoint_origin_;
    QString endpoint_source_;
    QString runtime_instance_id_;
    QString runtime_generation_;
};

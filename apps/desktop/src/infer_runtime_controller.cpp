#include "infer_runtime_controller.hpp"

#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"

#include <QtConcurrent/QtConcurrentRun>

#include <cstddef>
#include <string>

namespace {

QString from_rust(const rust::String& value) {
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}

std::string to_utf8(const QString& value) {
    const QByteArray bytes = value.toUtf8();
    return std::string(bytes.constData(), static_cast<std::size_t>(bytes.size()));
}

QVariantMap probe_projection(const QString& explicitOverride, quint64 generation) {
    const shape::desktop::InferRuntimeProbeWire probe =
        shape::desktop::probe_infer_runtime(to_utf8(explicitOverride));
    return {
        {QStringLiteral("generation"), QVariant::fromValue<qulonglong>(generation)},
        {QStringLiteral("reachable"), probe.reachable},
        {QStringLiteral("compatible"), probe.compatible},
        {QStringLiteral("contractVersion"), from_rust(probe.contract_version)},
        {QStringLiteral("errorCode"), from_rust(probe.error_code)},
        {QStringLiteral("endpointOrigin"), from_rust(probe.endpoint_origin)},
        {QStringLiteral("endpointSource"), from_rust(probe.endpoint_source)},
        {QStringLiteral("runtimeInstanceId"), from_rust(probe.runtime_instance_id)},
        {QStringLiteral("runtimeGeneration"), from_rust(probe.runtime_generation)},
    };
}

} // namespace

InferRuntimeController::InferRuntimeController(QObject* parent)
    : QObject(parent), explicit_override_(qEnvironmentVariable("SHAPE_INFER_RUNTIME_URL")) {
    connect(
        &watcher_,
        &QFutureWatcher<QVariantMap>::finished,
        this,
        &InferRuntimeController::finishProbe
    );
}

InferRuntimeController::~InferRuntimeController() {
    if (watcher_.isRunning()) {
        watcher_.waitForFinished();
    }
}

bool InferRuntimeController::probing() const {
    return probing_;
}

bool InferRuntimeController::reachable() const {
    return reachable_;
}

bool InferRuntimeController::compatible() const {
    return compatible_;
}

bool InferRuntimeController::available() const {
    return reachable_ && compatible_;
}

QString InferRuntimeController::contractVersion() const {
    return contract_version_;
}

QString InferRuntimeController::errorCode() const {
    return error_code_;
}

QString InferRuntimeController::endpointOrigin() const {
    return endpoint_origin_;
}

QString InferRuntimeController::endpointSource() const {
    return endpoint_source_;
}

QString InferRuntimeController::runtimeInstanceId() const {
    return runtime_instance_id_;
}

QString InferRuntimeController::runtimeGeneration() const {
    return runtime_generation_;
}

void InferRuntimeController::refresh() {
    if (probing_) {
        refresh_queued_ = true;
        return;
    }
    probing_ = true;
    ++generation_;
    emit statusChanged();
    watcher_.setFuture(QtConcurrent::run(probe_projection, explicit_override_, generation_));
}

void InferRuntimeController::finishProbe() {
    const QVariantMap result = watcher_.result();
    const quint64 result_generation = result.value(QStringLiteral("generation")).toULongLong();
    if (result_generation == generation_) {
        reachable_ = result.value(QStringLiteral("reachable")).toBool();
        compatible_ = result.value(QStringLiteral("compatible")).toBool();
        contract_version_ = result.value(QStringLiteral("contractVersion")).toString();
        error_code_ = result.value(QStringLiteral("errorCode")).toString();
        endpoint_origin_ = result.value(QStringLiteral("endpointOrigin")).toString();
        endpoint_source_ = result.value(QStringLiteral("endpointSource")).toString();
        runtime_instance_id_ = result.value(QStringLiteral("runtimeInstanceId")).toString();
        runtime_generation_ = result.value(QStringLiteral("runtimeGeneration")).toString();
    }
    probing_ = false;
    emit statusChanged();
    if (refresh_queued_) {
        refresh_queued_ = false;
        refresh();
    }
}

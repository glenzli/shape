#include "desktop_backend.hpp"

#include <QVariantMap>

#include <cstdint>
#include <utility>

namespace {

QString from_rust(const rust::String& value) {
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}

QString artifact_kind_label(const QString& key) {
    if (key == QStringLiteral("text_document")) {
        return DesktopBackend::tr("Text document");
    }
    if (key == QStringLiteral("image_raster")) {
        return DesktopBackend::tr("Raster image");
    }
    if (key == QStringLiteral("image_composite")) {
        return DesktopBackend::tr("Image composition");
    }
    if (key == QStringLiteral("reference_set")) {
        return DesktopBackend::tr("Reference set");
    }
    return DesktopBackend::tr("Unknown artifact");
}

QVariantMap artifact_projection(const shape::desktop::ArtifactSummaryWire& artifact) {
    const QString kind_key = from_rust(artifact.kind_key);
    QVariantMap projected;
    projected.insert(QStringLiteral("id"), from_rust(artifact.id));
    projected.insert(QStringLiteral("name"), from_rust(artifact.name));
    projected.insert(QStringLiteral("kindKey"), kind_key);
    projected.insert(QStringLiteral("kindLabel"), artifact_kind_label(kind_key));
    projected.insert(QStringLiteral("hasAcceptedRevision"), artifact.has_accepted_revision);
    projected.insert(
        QStringLiteral("acceptedRevisionId"),
        from_rust(artifact.accepted_revision_id)
    );
    projected.insert(QStringLiteral("hasContent"), artifact.has_content);
    projected.insert(QStringLiteral("contentDigest"), from_rust(artifact.content_digest));
    projected.insert(QStringLiteral("mediaType"), from_rust(artifact.media_type));
    projected.insert(QStringLiteral("byteLength"), static_cast<qulonglong>(artifact.byte_length));
    projected.insert(QStringLiteral("hasTextPreview"), artifact.has_text_preview);
    projected.insert(QStringLiteral("textPreview"), from_rust(artifact.text_preview));
    projected.insert(QStringLiteral("textPreviewTruncated"), artifact.text_preview_truncated);
    return projected;
}

} // namespace

DesktopBackend::DesktopBackend(QObject* parent) : QObject(parent) {}

DesktopBackend::DesktopBackend(shape::desktop::ProjectSnapshotWire snapshot, QObject* parent)
    : QObject(parent), project_open_(true), project_id_(from_rust(snapshot.project_id)),
      project_name_(from_rust(snapshot.project_name)),
      schema_revision_(from_rust(snapshot.schema_revision)),
      bundle_path_(from_rust(snapshot.bundle_path)) {
    for (const auto& artifact : snapshot.artifacts) {
        artifacts_.append(artifact_projection(artifact));
    }
}

bool DesktopBackend::projectOpen() const {
    return project_open_;
}

QString DesktopBackend::projectId() const {
    return project_id_;
}

QString DesktopBackend::projectName() const {
    return project_name_;
}

QString DesktopBackend::schemaRevision() const {
    return schema_revision_;
}

QString DesktopBackend::bundlePath() const {
    return bundle_path_;
}

int DesktopBackend::artifactCount() const {
    return static_cast<int>(artifacts_.size());
}

QVariantList DesktopBackend::artifacts() const {
    return artifacts_;
}

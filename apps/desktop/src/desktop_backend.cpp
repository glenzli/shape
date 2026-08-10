#include "desktop_backend.hpp"

#include <QDebug>
#include <QVariantMap>

#include <cstdint>
#include <utility>

namespace {

QString from_rust(const rust::String& value) {
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}

std::string to_utf8(const QString& value) {
    const QByteArray bytes = value.toUtf8();
    return std::string(bytes.constData(), static_cast<std::size_t>(bytes.size()));
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

QString transformation_kind_label(const QString& key) {
    if (key == QStringLiteral("import")) {
        return DesktopBackend::tr("Imported origin");
    }
    if (key == QStringLiteral("text_rewrite")) {
        return DesktopBackend::tr("Text revision");
    }
    if (key == QStringLiteral("deterministic_edit")) {
        return DesktopBackend::tr("Deterministic edit");
    }
    if (key == QStringLiteral("generative_edit")) {
        return DesktopBackend::tr("Generative edit");
    }
    if (key == QStringLiteral("composite")) {
        return DesktopBackend::tr("Composite");
    }
    if (key == QStringLiteral("external_round_trip")) {
        return DesktopBackend::tr("External round trip");
    }
    return DesktopBackend::tr("Unknown transformation");
}

QString transformation_intent_label(const QString& intent) {
    if (intent == QStringLiteral("Replace text with a user-authored draft")) {
        return DesktopBackend::tr("Direct text edit");
    }
    return intent;
}

QVariantList string_list_projection(const rust::Vec<rust::String>& values) {
    QVariantList projected;
    projected.reserve(static_cast<qsizetype>(values.size()));
    for (const auto& value : values) {
        projected.append(from_rust(value));
    }
    return projected;
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
    projected.insert(
        QStringLiteral("acceptedParentRevisionIds"),
        string_list_projection(artifact.accepted_parent_revision_ids)
    );
    projected.insert(QStringLiteral("transformationId"), from_rust(artifact.transformation_id));
    const QString transformation_kind_key = from_rust(artifact.transformation_kind_key);
    projected.insert(QStringLiteral("transformationKindKey"), transformation_kind_key);
    projected.insert(
        QStringLiteral("transformationKindLabel"),
        transformation_kind_label(transformation_kind_key)
    );
    projected.insert(
        QStringLiteral("transformationIntent"),
        transformation_intent_label(from_rust(artifact.transformation_intent))
    );
    projected.insert(
        QStringLiteral("transformationInputRevisionIds"),
        string_list_projection(artifact.transformation_input_revision_ids)
    );
    projected.insert(
        QStringLiteral("constraintCount"),
        static_cast<qulonglong>(artifact.constraint_count)
    );
    projected.insert(
        QStringLiteral("referenceCount"),
        static_cast<qulonglong>(artifact.reference_count)
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

struct DesktopBackend::SessionState {
    explicit SessionState(rust::Box<shape::desktop::DesktopSession> value)
        : session(std::move(value)) {}

    rust::Box<shape::desktop::DesktopSession> session;
};

DesktopBackend::DesktopBackend(QObject* parent) : QObject(parent) {}

DesktopBackend::DesktopBackend(rust::Box<shape::desktop::DesktopSession> session, QObject* parent)
    : QObject(parent), session_(std::make_unique<SessionState>(std::move(session))) {
    applySnapshot(session_->session->session_snapshot());
}

DesktopBackend::~DesktopBackend() = default;

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

bool DesktopBackend::hasCandidate() const {
    return has_candidate_;
}

QString DesktopBackend::candidateId() const {
    return candidate_id_;
}

QString DesktopBackend::candidateArtifactId() const {
    return candidate_artifact_id_;
}

QString DesktopBackend::candidateText() const {
    return candidate_text_;
}

bool DesktopBackend::candidateTextTruncated() const {
    return candidate_text_truncated_;
}

QString DesktopBackend::lastError() const {
    return last_error_;
}

bool DesktopBackend::proposeTextCandidate(
    const QString& artifactId,
    const QString& replacementText
) {
    if (session_ == nullptr) {
        setLastError(tr("Open a project before creating a candidate."));
        return false;
    }
    try {
        applyCandidate(
            session_->session->session_propose_text(to_utf8(artifactId), to_utf8(replacementText))
        );
        setLastError(QString());
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create desktop candidate:" << error.what();
        setLastError(tr("Could not create candidate."));
        return false;
    }
}

bool DesktopBackend::acceptCandidate() {
    if (session_ == nullptr || !has_candidate_) {
        setLastError(tr("There is no candidate to accept."));
        return false;
    }
    try {
        applySnapshot(session_->session->session_accept_text());
        clearCandidate();
        setLastError(QString());
        emit projectChanged();
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not accept desktop candidate:" << error.what();
        setLastError(tr("Could not accept candidate."));
        return false;
    }
}

void DesktopBackend::discardCandidate() {
    if (session_ == nullptr || !has_candidate_) {
        return;
    }
    session_->session->session_discard_text();
    clearCandidate();
    setLastError(QString());
    emit candidateChanged();
}

void DesktopBackend::retranslate() {
    if (session_ == nullptr) {
        return;
    }
    try {
        applySnapshot(session_->session->session_snapshot());
        setLastError(QString());
        emit projectChanged();
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not refresh desktop project:" << error.what();
        setLastError(tr("Could not refresh project."));
    }
}

void DesktopBackend::applySnapshot(shape::desktop::ProjectSnapshotWire snapshot) {
    QVariantList artifacts;
    artifacts.reserve(static_cast<qsizetype>(snapshot.artifacts.size()));
    for (const auto& artifact : snapshot.artifacts) {
        artifacts.append(artifact_projection(artifact));
    }
    project_open_ = true;
    project_id_ = from_rust(snapshot.project_id);
    project_name_ = from_rust(snapshot.project_name);
    schema_revision_ = from_rust(snapshot.schema_revision);
    bundle_path_ = from_rust(snapshot.bundle_path);
    artifacts_ = std::move(artifacts);
}

void DesktopBackend::applyCandidate(shape::desktop::TextCandidateWire candidate) {
    has_candidate_ = true;
    candidate_id_ = from_rust(candidate.candidate_id);
    candidate_artifact_id_ = from_rust(candidate.artifact_id);
    candidate_text_ = from_rust(candidate.text_preview);
    candidate_text_truncated_ = candidate.text_preview_truncated;
}

void DesktopBackend::clearCandidate() {
    has_candidate_ = false;
    candidate_id_.clear();
    candidate_artifact_id_.clear();
    candidate_text_.clear();
    candidate_text_truncated_ = false;
}

void DesktopBackend::setLastError(const QString& message) {
    if (message == last_error_) {
        return;
    }
    last_error_ = message;
    emit lastErrorChanged();
}

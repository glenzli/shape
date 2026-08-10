#include "desktop_backend.hpp"

#include <QDebug>
#include <QVariantMap>

#include <algorithm>
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
        QStringLiteral("transformationInputArtifactIds"),
        string_list_projection(artifact.transformation_input_artifact_ids)
    );
    projected.insert(
        QStringLiteral("transformationInputArtifactNames"),
        string_list_projection(artifact.transformation_input_artifact_names)
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

QVariantMap graph_edge_projection(const shape::desktop::ProjectGraphEdgeWire& edge) {
    const QString transformation_kind_key = from_rust(edge.transformation_kind_key);
    QVariantMap projected;
    projected.insert(QStringLiteral("sourceArtifactId"), from_rust(edge.source_artifact_id));
    projected.insert(QStringLiteral("targetArtifactId"), from_rust(edge.target_artifact_id));
    projected.insert(QStringLiteral("sourceRevisionId"), from_rust(edge.source_revision_id));
    projected.insert(QStringLiteral("targetRevisionId"), from_rust(edge.target_revision_id));
    projected.insert(QStringLiteral("transformationId"), from_rust(edge.transformation_id));
    projected.insert(QStringLiteral("transformationKindKey"), transformation_kind_key);
    projected.insert(
        QStringLiteral("transformationKindLabel"),
        transformation_kind_label(transformation_kind_key)
    );
    return projected;
}

QVariantMap candidate_projection(const shape::desktop::TextCandidateWire& candidate) {
    QVariantMap projected;
    projected.insert(QStringLiteral("id"), from_rust(candidate.candidate_id));
    projected.insert(QStringLiteral("artifactId"), from_rust(candidate.artifact_id));
    projected.insert(QStringLiteral("hasExpectedHead"), candidate.has_expected_head);
    projected.insert(QStringLiteral("expectedHead"), from_rust(candidate.expected_head));
    projected.insert(QStringLiteral("text"), from_rust(candidate.text_preview));
    projected.insert(QStringLiteral("textTruncated"), candidate.text_preview_truncated);
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
    applyCandidates(session_->session->session_text_candidates());
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

QVariantList DesktopBackend::graphEdges() const {
    return graph_edges_;
}

int DesktopBackend::candidateCount() const {
    return static_cast<int>(candidates_.size());
}

QVariantList DesktopBackend::candidates() const {
    return candidates_;
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
        const auto candidate =
            session_->session->session_propose_text(to_utf8(artifactId), to_utf8(replacementText));
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_text_candidates(), candidate_id);
        setLastError(QString());
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create desktop candidate:" << error.what();
        setLastError(tr("Could not create candidate."));
        return false;
    }
}

bool DesktopBackend::selectCandidate(const QString& candidateId) {
    if (!applyCandidateSelection(candidateId)) {
        return false;
    }
    setLastError(QString());
    emit candidateChanged();
    return true;
}

bool DesktopBackend::acceptCandidate(const QString& candidateId) {
    if (session_ == nullptr || candidateId.isEmpty()) {
        setLastError(tr("There is no candidate to accept."));
        return false;
    }
    try {
        applySnapshot(session_->session->session_accept_text(to_utf8(candidateId)));
        applyCandidates(session_->session->session_text_candidates());
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

bool DesktopBackend::branchCandidate(const QString& candidateId, const QString& artifactName) {
    if (session_ == nullptr || candidateId.isEmpty()) {
        setLastError(tr("There is no candidate to branch."));
        return false;
    }
    try {
        applySnapshot(
            session_->session->session_branch_text(to_utf8(candidateId), to_utf8(artifactName))
        );
        applyCandidates(session_->session->session_text_candidates());
        setLastError(QString());
        emit projectChanged();
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not branch desktop candidate:" << error.what();
        setLastError(tr("Could not branch candidate."));
        return false;
    }
}

bool DesktopBackend::discardCandidate(const QString& candidateId) {
    if (session_ == nullptr || candidateId.isEmpty()) {
        return false;
    }
    try {
        session_->session->session_discard_text(to_utf8(candidateId));
        applyCandidates(session_->session->session_text_candidates());
        setLastError(QString());
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not discard desktop candidate:" << error.what();
        setLastError(tr("Could not discard candidate."));
        return false;
    }
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
    QVariantList graph_edges;
    graph_edges.reserve(static_cast<qsizetype>(snapshot.graph_edges.size()));
    for (const auto& edge : snapshot.graph_edges) {
        graph_edges.append(graph_edge_projection(edge));
    }
    project_open_ = true;
    project_id_ = from_rust(snapshot.project_id);
    project_name_ = from_rust(snapshot.project_name);
    schema_revision_ = from_rust(snapshot.schema_revision);
    bundle_path_ = from_rust(snapshot.bundle_path);
    artifacts_ = std::move(artifacts);
    graph_edges_ = std::move(graph_edges);
}

void DesktopBackend::applyCandidates(
    rust::Vec<shape::desktop::TextCandidateWire> candidates,
    const QString& preferredCandidateId
) {
    QVariantList projected;
    projected.reserve(static_cast<qsizetype>(candidates.size()));
    for (const auto& candidate : candidates) {
        projected.append(candidate_projection(candidate));
    }
    const QString previous_candidate_id = candidate_id_;
    candidates_ = std::move(projected);
    if (!preferredCandidateId.isEmpty() && applyCandidateSelection(preferredCandidateId)) {
        return;
    }
    if (!previous_candidate_id.isEmpty() && applyCandidateSelection(previous_candidate_id)) {
        return;
    }
    if (!candidates_.isEmpty()) {
        applyCandidateSelection(candidates_.first().toMap().value(QStringLiteral("id")).toString());
        return;
    }
    clearCandidateSelection();
}

bool DesktopBackend::applyCandidateSelection(const QString& candidateId) {
    const auto selected = std::find_if(
        candidates_.cbegin(),
        candidates_.cend(),
        [&candidateId](const QVariant& candidate) {
            return candidate.toMap().value(QStringLiteral("id")).toString() == candidateId;
        }
    );
    if (selected == candidates_.cend()) {
        return false;
    }
    const QVariantMap candidate = selected->toMap();
    has_candidate_ = true;
    candidate_id_ = candidate.value(QStringLiteral("id")).toString();
    candidate_artifact_id_ = candidate.value(QStringLiteral("artifactId")).toString();
    candidate_text_ = candidate.value(QStringLiteral("text")).toString();
    candidate_text_truncated_ = candidate.value(QStringLiteral("textTruncated")).toBool();
    return true;
}

void DesktopBackend::clearCandidateSelection() {
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

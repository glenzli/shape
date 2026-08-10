#include "desktop_backend.hpp"
#include "image_preview_provider.hpp"

#include <QByteArray>
#include <QDebug>
#include <QFileInfo>
#include <QImage>
#include <QVariantMap>

#include <algorithm>
#include <cstdint>
#include <stdexcept>
#include <utility>

namespace {

constexpr int kMaximumPreviewDimension = 4096;

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
    if (key == QStringLiteral("audio_clip")) {
        return DesktopBackend::tr("Audio clip");
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
    if (intent == QStringLiteral("Replace text with a user-authored draft")
        || intent == QStringLiteral("Calibrate text with a user-authored replacement")) {
        return DesktopBackend::tr("Deterministic text calibration");
    }
    return intent;
}

QString operator_role_label(const QString& key) {
    if (key == QStringLiteral("source")) {
        return DesktopBackend::tr("Source");
    }
    if (key == QStringLiteral("operator")) {
        return DesktopBackend::tr("Operator");
    }
    if (key == QStringLiteral("output")) {
        return DesktopBackend::tr("Output");
    }
    return DesktopBackend::tr("Unknown node");
}

QString operator_type_label(const QString& key) {
    if (key.startsWith(QStringLiteral("source."))) {
        return DesktopBackend::tr("Source input");
    }
    if (key == QStringLiteral("image.crop")) {
        return DesktopBackend::tr("Crop");
    }
    if (key == QStringLiteral("text.edit")) {
        return DesktopBackend::tr("Text calibration");
    }
    if (key == QStringLiteral("text.transform")) {
        return DesktopBackend::tr("AI text transform");
    }
    if (key == QStringLiteral("audio.speech_synthesize")) {
        return DesktopBackend::tr("Speech synthesis");
    }
    if (key == QStringLiteral("creative.generate")) {
        return DesktopBackend::tr("Generative edit");
    }
    if (key == QStringLiteral("creative.deterministic_edit")) {
        return DesktopBackend::tr("Deterministic edit");
    }
    if (key == QStringLiteral("creative.composite")) {
        return DesktopBackend::tr("Composite");
    }
    if (key == QStringLiteral("external.round_trip")) {
        return DesktopBackend::tr("External round trip");
    }
    if (key.startsWith(QStringLiteral("output."))) {
        return DesktopBackend::tr("Scene output");
    }
    return DesktopBackend::tr("Unknown operator");
}

QVariantList string_list_projection(const rust::Vec<rust::String>& values) {
    QVariantList projected;
    projected.reserve(static_cast<qsizetype>(values.size()));
    for (const auto& value : values) {
        projected.append(from_rust(value));
    }
    return projected;
}

QVariantList operator_port_projection(const rust::Vec<shape::desktop::OperatorPortWire>& ports) {
    QVariantList projected;
    projected.reserve(static_cast<qsizetype>(ports.size()));
    for (const auto& port : ports) {
        QVariantMap item;
        item.insert(QStringLiteral("id"), from_rust(port.port_id));
        item.insert(QStringLiteral("dataTypeKey"), from_rust(port.data_type_key));
        projected.append(item);
    }
    return projected;
}

QVariantMap operator_node_projection(const shape::desktop::OperatorGraphNodeWire& node) {
    const QString role_key = from_rust(node.role_key);
    const QString operator_type_key = from_rust(node.operator_type_key);
    QVariantMap projected;
    projected.insert(QStringLiteral("id"), from_rust(node.node_id));
    projected.insert(QStringLiteral("roleKey"), role_key);
    projected.insert(QStringLiteral("roleLabel"), operator_role_label(role_key));
    projected.insert(QStringLiteral("operatorTypeKey"), operator_type_key);
    projected.insert(QStringLiteral("operatorTypeLabel"), operator_type_label(operator_type_key));
    projected.insert(QStringLiteral("artifactId"), from_rust(node.artifact_id));
    projected.insert(QStringLiteral("artifactName"), from_rust(node.artifact_name));
    projected.insert(QStringLiteral("revisionId"), from_rust(node.revision_id));
    projected.insert(QStringLiteral("transformationId"), from_rust(node.transformation_id));
    projected.insert(QStringLiteral("intent"), transformation_intent_label(from_rust(node.intent)));
    projected.insert(QStringLiteral("inputPorts"), operator_port_projection(node.input_ports));
    projected.insert(QStringLiteral("outputPorts"), operator_port_projection(node.output_ports));
    return projected;
}

QVariantMap operator_edge_projection(const shape::desktop::OperatorGraphEdgeWire& edge) {
    QVariantMap projected;
    projected.insert(QStringLiteral("sourceNodeId"), from_rust(edge.source_node_id));
    projected.insert(QStringLiteral("sourcePortId"), from_rust(edge.source_port_id));
    projected.insert(QStringLiteral("targetNodeId"), from_rust(edge.target_node_id));
    projected.insert(QStringLiteral("targetPortId"), from_rust(edge.target_port_id));
    projected.insert(QStringLiteral("dataTypeKey"), from_rust(edge.data_type_key));
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
    projected.insert(QStringLiteral("hasImagePreview"), artifact.has_image_preview);
    projected.insert(QStringLiteral("imageWidth"), static_cast<qulonglong>(artifact.image_width));
    projected.insert(QStringLiteral("imageHeight"), static_cast<qulonglong>(artifact.image_height));
    projected.insert(QStringLiteral("hasAudioPreview"), artifact.has_audio_preview);
    projected.insert(
        QStringLiteral("audioDurationMillis"),
        static_cast<qulonglong>(artifact.audio_duration_millis)
    );
    projected.insert(
        QStringLiteral("audioSampleRateHz"),
        static_cast<qulonglong>(artifact.audio_sample_rate_hz)
    );
    projected.insert(QStringLiteral("audioChannels"), artifact.audio_channels);
    projected.insert(QStringLiteral("audioOriginKey"), from_rust(artifact.audio_origin_key));
    QVariantList operator_nodes;
    operator_nodes.reserve(static_cast<qsizetype>(artifact.operator_graph_nodes.size()));
    for (const auto& node : artifact.operator_graph_nodes) {
        operator_nodes.append(operator_node_projection(node));
    }
    QVariantList operator_edges;
    operator_edges.reserve(static_cast<qsizetype>(artifact.operator_graph_edges.size()));
    for (const auto& edge : artifact.operator_graph_edges) {
        operator_edges.append(operator_edge_projection(edge));
    }
    projected.insert(QStringLiteral("operatorNodes"), operator_nodes);
    projected.insert(QStringLiteral("operatorEdges"), operator_edges);
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

QVariantMap candidate_projection(const shape::desktop::CandidateWire& candidate) {
    QVariantMap projected;
    projected.insert(QStringLiteral("id"), from_rust(candidate.candidate_id));
    projected.insert(QStringLiteral("artifactId"), from_rust(candidate.artifact_id));
    projected.insert(QStringLiteral("contextArtifactId"), from_rust(candidate.context_artifact_id));
    projected.insert(QStringLiteral("artifactName"), from_rust(candidate.artifact_name));
    projected.insert(QStringLiteral("kindKey"), from_rust(candidate.kind_key));
    projected.insert(QStringLiteral("hasExpectedHead"), candidate.has_expected_head);
    projected.insert(QStringLiteral("expectedHead"), from_rust(candidate.expected_head));
    projected.insert(QStringLiteral("canBranch"), candidate.can_branch);
    projected.insert(QStringLiteral("hasTextPreview"), candidate.has_text_preview);
    projected.insert(QStringLiteral("text"), from_rust(candidate.text_preview));
    projected.insert(QStringLiteral("textTruncated"), candidate.text_preview_truncated);
    projected.insert(QStringLiteral("hasImagePreview"), candidate.has_image_preview);
    projected.insert(QStringLiteral("imageWidth"), static_cast<qulonglong>(candidate.image_width));
    projected.insert(
        QStringLiteral("imageHeight"),
        static_cast<qulonglong>(candidate.image_height)
    );
    projected.insert(QStringLiteral("hasAudioPreview"), candidate.has_audio_preview);
    projected.insert(
        QStringLiteral("audioDurationMillis"),
        static_cast<qulonglong>(candidate.audio_duration_millis)
    );
    projected.insert(
        QStringLiteral("audioSampleRateHz"),
        static_cast<qulonglong>(candidate.audio_sample_rate_hz)
    );
    projected.insert(QStringLiteral("audioChannels"), candidate.audio_channels);
    projected.insert(QStringLiteral("audioOriginKey"), from_rust(candidate.audio_origin_key));
    return projected;
}

} // namespace

struct DesktopBackend::SessionState {
    explicit SessionState(rust::Box<shape::desktop::DesktopSession> value)
        : session(std::move(value)) {}

    rust::Box<shape::desktop::DesktopSession> session;
};

DesktopBackend::DesktopBackend(QObject* parent)
    : QObject(parent), image_preview_store_(std::make_shared<ImagePreviewStore>()) {}

DesktopBackend::DesktopBackend(rust::Box<shape::desktop::DesktopSession> session, QObject* parent)
    : QObject(parent), session_(std::make_unique<SessionState>(std::move(session))),
      image_preview_store_(std::make_shared<ImagePreviewStore>()) {
    applySnapshot(session_->session->session_snapshot());
    applyCandidates(session_->session->session_candidates());
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

QString DesktopBackend::acceptedImageSource() const {
    return accepted_image_source_;
}

QString DesktopBackend::candidateImageSource() const {
    return candidate_image_source_;
}

QString DesktopBackend::lastError() const {
    return last_error_;
}

bool DesktopBackend::importRaster(const QUrl& sourceUrl) {
    if (session_ == nullptr || !sourceUrl.isLocalFile()) {
        setLastError(tr("Choose a local PNG or JPEG image."));
        return false;
    }
    const QString source_path = sourceUrl.toLocalFile();
    const QString artifact_name = QFileInfo(source_path).completeBaseName().trimmed();
    if (artifact_name.isEmpty()) {
        setLastError(tr("The image needs a usable file name."));
        return false;
    }
    try {
        applySnapshot(
            session_->session->session_import_raster(to_utf8(source_path), to_utf8(artifact_name))
        );
        applyCandidates(session_->session->session_candidates());
        setLastError(QString());
        emit projectChanged();
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not import raster image:" << error.what();
        setLastError(
            tr("Could not import this image. Use an 8-bit PNG or JPEG within the local size limit.")
        );
        return false;
    }
}

bool DesktopBackend::proposeRasterCrop(
    const QString& artifactId,
    int x,
    int y,
    int width,
    int height
) {
    if (session_ == nullptr || x < 0 || y < 0 || width <= 0 || height <= 0) {
        setLastError(tr("Choose a valid crop area."));
        return false;
    }
    try {
        const auto candidate = session_->session->session_propose_raster_crop(
            to_utf8(artifactId),
            static_cast<std::uint32_t>(x),
            static_cast<std::uint32_t>(y),
            static_cast<std::uint32_t>(width),
            static_cast<std::uint32_t>(height)
        );
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_candidates(), candidate_id);
        setLastError(QString());
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster crop candidate:" << error.what();
        setLastError(tr("Could not create the crop candidate."));
        return false;
    }
}

bool DesktopBackend::prepareImagePreviews(const QString& artifactId, const QString& candidateId) {
    if (session_ == nullptr || artifactId.isEmpty()) {
        return false;
    }
    try {
        const QString accepted_source = cacheImagePreview(
            session_->session->session_image_preview(to_utf8(artifactId), std::string())
        );
        QString candidate_source;
        if (!candidateId.isEmpty()) {
            candidate_source = cacheImagePreview(
                session_->session->session_image_preview(to_utf8(artifactId), to_utf8(candidateId))
            );
        }
        const bool changed = accepted_image_source_ != accepted_source
                             || candidate_image_source_ != candidate_source;
        accepted_image_source_ = accepted_source;
        candidate_image_source_ = candidate_source;
        if (changed) {
            emit imagePreviewChanged();
        }
        setLastError(QString());
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not prepare raster preview:" << error.what();
        accepted_image_source_.clear();
        candidate_image_source_.clear();
        emit imagePreviewChanged();
        setLastError(tr("Could not load the verified image preview."));
        return false;
    }
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
        applyCandidates(session_->session->session_candidates(), candidate_id);
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
        applySnapshot(session_->session->session_accept_candidate(to_utf8(candidateId)));
        applyCandidates(session_->session->session_candidates());
        image_preview_store_->remove(candidateId);
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
            session_->session->session_branch_candidate(to_utf8(candidateId), to_utf8(artifactName))
        );
        applyCandidates(session_->session->session_candidates());
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
        session_->session->session_discard_candidate(to_utf8(candidateId));
        applyCandidates(session_->session->session_candidates());
        image_preview_store_->remove(candidateId);
        setLastError(QString());
        emit candidateChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not discard desktop candidate:" << error.what();
        setLastError(tr("Could not discard candidate."));
        return false;
    }
}

QString
DesktopBackend::adoptInferTextCandidate(rust::Box<shape::desktop::InferTextCandidate> candidate) {
    if (session_ == nullptr) {
        return QString();
    }
    const auto adopted = session_->session->session_adopt_infer_text(std::move(candidate));
    const QString candidate_id = from_rust(adopted.candidate_id);
    applyCandidates(session_->session->session_candidates(), candidate_id);
    setLastError(QString());
    emit candidateChanged();
    return candidate_id;
}

QString DesktopBackend::adoptInferSpeechCandidate(
    rust::Box<shape::desktop::InferSpeechCandidate> candidate
) {
    if (session_ == nullptr) {
        return QString();
    }
    const auto adopted = session_->session->session_adopt_infer_speech(std::move(candidate));
    const QString candidate_id = from_rust(adopted.candidate_id);
    applyCandidates(session_->session->session_candidates(), candidate_id);
    setLastError(QString());
    emit candidateChanged();
    return candidate_id;
}

std::optional<AudioPreviewData>
DesktopBackend::audioPreview(const QString& artifactId, const QString& candidateId) {
    if (session_ == nullptr || artifactId.isEmpty()) {
        setLastError(tr("Open a project before previewing audio."));
        return std::nullopt;
    }
    try {
        auto preview =
            session_->session->session_audio_preview(to_utf8(artifactId), to_utf8(candidateId));
        AudioPreviewData projected;
        projected.identity = from_rust(preview.identity);
        projected.wav_bytes = QByteArray(
            reinterpret_cast<const char*>(preview.wav_bytes.data()),
            static_cast<qsizetype>(preview.wav_bytes.size())
        );
        projected.duration_millis = static_cast<qint64>(preview.duration_millis);
        projected.sample_rate_hz = static_cast<int>(preview.sample_rate_hz);
        projected.channels = static_cast<int>(preview.channels);
        setLastError(QString());
        return projected;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not prepare audio preview:" << error.what();
        setLastError(tr("Could not load the verified audio preview."));
        return std::nullopt;
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
    rust::Vec<shape::desktop::CandidateWire> candidates,
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

QString DesktopBackend::cacheImagePreview(shape::desktop::ImagePreviewWire preview) {
    if (preview.png_bytes.empty()) {
        return QString();
    }
    const QByteArray encoded = QByteArray::fromRawData(
        reinterpret_cast<const char*>(preview.png_bytes.data()),
        static_cast<qsizetype>(preview.png_bytes.size())
    );
    QImage image = QImage::fromData(encoded, "PNG");
    if (image.isNull() || image.width() != static_cast<int>(preview.width)
        || image.height() != static_cast<int>(preview.height)) {
        throw std::runtime_error("raster preview dimensions do not match its contract");
    }
    if (image.width() > kMaximumPreviewDimension || image.height() > kMaximumPreviewDimension) {
        image = image.scaled(
            kMaximumPreviewDimension,
            kMaximumPreviewDimension,
            Qt::KeepAspectRatio,
            Qt::SmoothTransformation
        );
    }
    const QString identity = from_rust(preview.identity);
    if (!image_preview_store_->put(identity, std::move(image))) {
        throw std::runtime_error("raster preview exceeds the resident cache budget");
    }
    return QStringLiteral("image://shape-preview/") + identity;
}

std::shared_ptr<ImagePreviewStore> DesktopBackend::imagePreviewStore() const {
    return image_preview_store_;
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

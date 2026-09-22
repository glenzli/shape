#include "desktop_backend.hpp"
#include "image_preview_provider.hpp"

#include <QByteArray>
#include <QDebug>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFutureWatcher>
#include <QImage>
#include <QVariantMap>
#include <QtConcurrent/QtConcurrentRun>

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
        return DesktopBackend::tr("Image editing");
    }
    if (key == QStringLiteral("image.resize")) {
        return DesktopBackend::tr("Image editing");
    }
    if (key == QStringLiteral("text.create")) {
        return DesktopBackend::tr("Text creation");
    }
    if (key == QStringLiteral("text.edit")) {
        return DesktopBackend::tr("Text editing");
    }
    if (key == QStringLiteral("text.transform")) {
        return DesktopBackend::tr("AI text editor");
    }
    if (key == QStringLiteral("audio.speech_synthesize")) {
        return DesktopBackend::tr("Speech synthesis");
    }
    if (key == QStringLiteral("image.generate")) {
        return DesktopBackend::tr("AI image creation");
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
    projected.insert(QStringLiteral("mediaType"), from_rust(node.media_type));
    projected.insert(QStringLiteral("byteLength"), static_cast<qulonglong>(node.byte_length));
    projected.insert(QStringLiteral("hasTextPreview"), node.has_text_preview);
    projected.insert(QStringLiteral("textPreview"), from_rust(node.text_preview));
    projected.insert(QStringLiteral("textPreviewTruncated"), node.text_preview_truncated);
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

QVariantMap operator_draft_projection(const shape::desktop::OperatorDraftWire& draft) {
    const QString operator_type_key = from_rust(draft.operator_type_key);
    QVariantMap projected;
    projected.insert(QStringLiteral("id"), from_rust(draft.draft_id));
    projected.insert(QStringLiteral("contextArtifactId"), from_rust(draft.context_artifact_id));
    projected.insert(QStringLiteral("inputArtifactId"), from_rust(draft.input_artifact_id));
    projected.insert(QStringLiteral("inputRevisionId"), from_rust(draft.input_revision_id));
    projected.insert(QStringLiteral("operatorTypeKey"), operator_type_key);
    projected.insert(QStringLiteral("operatorTypeLabel"), operator_type_label(operator_type_key));
    projected.insert(QStringLiteral("hasInputDataType"), draft.has_input_data_type);
    projected.insert(QStringLiteral("inputDataTypeKey"), from_rust(draft.input_data_type_key));
    projected.insert(QStringLiteral("outputDataTypeKey"), from_rust(draft.output_data_type_key));
    projected.insert(QStringLiteral("configurationSchema"), from_rust(draft.configuration_schema));
    projected.insert(QStringLiteral("textAuthoringJson"), from_rust(draft.text_authoring_json));
    projected.insert(QStringLiteral("textTransformMode"), from_rust(draft.text_transform_mode));
    projected.insert(
        QStringLiteral("textTransformInstruction"),
        from_rust(draft.text_transform_instruction)
    );
    projected.insert(QStringLiteral("textTransformTone"), from_rust(draft.text_transform_tone));
    projected.insert(
        QStringLiteral("textTransformExpressionJson"),
        from_rust(draft.text_transform_expression_json)
    );
    projected.insert(QStringLiteral("textTransformStyle"), from_rust(draft.text_transform_style));
    projected.insert(
        QStringLiteral("textTransformVariantCount"),
        static_cast<int>(draft.text_transform_variant_count)
    );
    projected.insert(
        QStringLiteral("audioSpeechPresetAlias"),
        from_rust(draft.audio_speech_preset_alias)
    );
    projected.insert(
        QStringLiteral("audioSpeechPresetCatalogRevision"),
        from_rust(draft.audio_speech_preset_catalog_revision)
    );
    projected.insert(
        QStringLiteral("audioSpeechScriptJson"),
        from_rust(draft.audio_speech_script_json)
    );
    projected.insert(QStringLiteral("audioSpeechLanguage"), from_rust(draft.audio_speech_language));
    projected.insert(
        QStringLiteral("audioSpeechSpeedMilli"),
        static_cast<int>(draft.audio_speech_speed_milli)
    );
    projected.insert(
        QStringLiteral("audioSpeechDisclosureRequired"),
        draft.audio_speech_disclosure_required
    );
    projected.insert(
        QStringLiteral("imageResizeTargetWidth"),
        static_cast<int>(draft.image_resize_target_width)
    );
    projected.insert(
        QStringLiteral("imageResizeTargetHeight"),
        static_cast<int>(draft.image_resize_target_height)
    );
    projected.insert(
        QStringLiteral("imageResizeAspectPolicy"),
        from_rust(draft.image_resize_aspect_policy)
    );
    projected.insert(
        QStringLiteral("imageResizeResampling"),
        from_rust(draft.image_resize_resampling)
    );
    projected.insert(QStringLiteral("aiImageInstruction"), from_rust(draft.ai_image_instruction));
    projected.insert(
        QStringLiteral("aiImageOutputWidth"),
        static_cast<int>(draft.ai_image_output_width)
    );
    projected.insert(
        QStringLiteral("aiImageOutputHeight"),
        static_cast<int>(draft.ai_image_output_height)
    );
    return projected;
}

QVariantMap
operator_descriptor_projection(const shape::desktop::OperatorDescriptorWire& descriptor) {
    QVariantMap projected;
    projected.insert(QStringLiteral("typeKey"), from_rust(descriptor.operator_type));
    projected.insert(QStringLiteral("inputDataTypeKey"), from_rust(descriptor.input_data_type));
    projected.insert(QStringLiteral("outputDataTypeKey"), from_rust(descriptor.output_data_type));
    projected.insert(QStringLiteral("categoryKey"), from_rust(descriptor.category));
    projected.insert(QStringLiteral("iconKey"), from_rust(descriptor.icon));
    return projected;
}

QString portable_project_directory_name(const QString& project_name) {
    QString result;
    result.reserve(project_name.size());
    for (const QChar character : project_name.trimmed()) {
        if (character.isLetterOrNumber() || character == QLatin1Char(' ')
            || character == QLatin1Char('-') || character == QLatin1Char('_')) {
            result.append(character);
        } else {
            result.append(QLatin1Char('-'));
        }
    }
    while (result.contains(QStringLiteral("--"))) {
        result.replace(QStringLiteral("--"), QStringLiteral("-"));
    }
    return result.trimmed();
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
    projected.insert(QStringLiteral("textFormat"), from_rust(artifact.text_format));
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
    : QObject(parent), image_preview_store_(std::make_shared<ImagePreviewStore>()) {
    replaceSession(std::move(session));
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

QVariantList DesktopBackend::operatorDrafts() const {
    return operator_drafts_;
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

bool DesktopBackend::createProject(const QUrl& parentDirectory, const QString& projectName) {
    const QString name = projectName.trimmed();
    const QString directory_name = portable_project_directory_name(name);
    if (!parentDirectory.isLocalFile() || name.isEmpty() || directory_name.isEmpty()) {
        setLastError(tr("Choose a local folder and enter a project name."));
        return false;
    }
    const QFileInfo parent(parentDirectory.toLocalFile());
    if (!parent.isDir()) {
        setLastError(tr("The selected project location is not a folder."));
        return false;
    }
    const QString bundle_path =
        QDir(parent.absoluteFilePath()).filePath(directory_name + QStringLiteral(".shape"));
    if (QFileInfo::exists(bundle_path)) {
        setLastError(tr("A project with this name already exists in that folder."));
        return false;
    }
    try {
        replaceSession(shape::desktop::create_desktop_project(to_utf8(bundle_path), to_utf8(name)));
        setLastError(QString());
        emit projectChanged();
        emit candidateChanged();
        emit operatorDraftsChanged();
        emit imagePreviewChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create Shape project:" << error.what();
        setLastError(tr("Could not create the project in this location."));
        return false;
    }
}

bool DesktopBackend::openProject(const QUrl& bundleUrl) {
    if (!bundleUrl.isLocalFile()) {
        setLastError(tr("Choose a local .shape project folder."));
        return false;
    }
    const QFileInfo bundle(bundleUrl.toLocalFile());
    if (!bundle.isDir()) {
        setLastError(tr("The selected Shape project is not a folder."));
        return false;
    }
    try {
        replaceSession(shape::desktop::open_desktop_session(to_utf8(bundle.absoluteFilePath())));
        setLastError(QString());
        emit projectChanged();
        emit candidateChanged();
        emit operatorDraftsChanged();
        emit imagePreviewChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not open Shape project:" << error.what();
        setLastError(tr("Could not open this Shape project."));
        return false;
    }
}

bool DesktopBackend::createTextScene(const QString& sceneName, const QString& initialText) {
    if (session_ == nullptr || sceneName.trimmed().isEmpty() || initialText.trimmed().isEmpty()) {
        setLastError(tr("Enter a Scene name and some starting text."));
        return false;
    }
    try {
        applySnapshot(session_->session->session_create_text_document(
            to_utf8(sceneName.trimmed()),
            to_utf8(initialText)
        ));
        setLastError(QString());
        emit projectChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create text Scene:" << error.what();
        setLastError(tr("Could not create the text Scene."));
        return false;
    }
}

bool DesktopBackend::createAiImageScene(
    const QString& sceneName,
    const QString& instruction,
    int outputWidth,
    int outputHeight
) {
    constexpr int kMaximumImageDimension = 4096;
    if (session_ == nullptr || sceneName.trimmed().isEmpty() || instruction.trimmed().isEmpty()
        || outputWidth <= 0 || outputHeight <= 0 || outputWidth > kMaximumImageDimension
        || outputHeight > kMaximumImageDimension) {
        setLastError(tr("Enter a Scene name, an image description, and valid dimensions."));
        return false;
    }
    try {
        applySnapshot(session_->session->session_create_ai_image_draft(
            to_utf8(sceneName.trimmed()),
            to_utf8(instruction),
            static_cast<std::uint32_t>(outputWidth),
            static_cast<std::uint32_t>(outputHeight)
        ));
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create AI image Scene:" << error.what();
        setLastError(tr("Could not create the AI image Scene."));
        return false;
    }
}

bool DesktopBackend::createDetachedTextEditor(const QString& nodeName) {
    if (session_ == nullptr || nodeName.trimmed().isEmpty()) {
        return false;
    }
    try {
        applySnapshot(session_->session->session_create_detached_text_editor(to_utf8(nodeName)));
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create detached AI text editor:" << error.what();
        setLastError(tr("Could not add the AI text editor node."));
        return false;
    }
}

bool DesktopBackend::createTextAuthoring(const QString& name, const QString& profile) {
    if (!session_)
        return false;
    try {
        applySnapshot(
            session_->session->session_create_text_authoring(to_utf8(name), to_utf8(profile))
        );
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error&) {
        setLastError(tr("Could not create the writing draft."));
        return false;
    }
}

QString DesktopBackend::beginTextAuthoring(const QString& artifactId, const QString& profile) {
    if (!session_)
        return {};
    try {
        const auto draft =
            session_->session->session_begin_text_authoring(to_utf8(artifactId), to_utf8(profile));
        if (std::none_of(artifacts_.cbegin(), artifacts_.cend(), [&draft](const QVariant& a) {
                return a.toMap().value(QStringLiteral("id")).toString()
                       == from_rust(draft.context_artifact_id);
            })) {
            session_->session->session_rename_artifact(
                draft.context_artifact_id,
                to_utf8(tr("Edited text"))
            );
        }
        applySnapshot(session_->session->session_snapshot());
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit operatorDraftsChanged();
        return from_rust(draft.draft_id);
    } catch (const rust::Error&) {
        setLastError(tr("Could not open the writing workspace."));
        return {};
    }
}

bool DesktopBackend::renameArtifact(const QString& artifactId, const QString& name) {
    if (!session_)
        return false;
    try {
        session_->session->session_rename_artifact(to_utf8(artifactId), to_utf8(name.trimmed()));
        applySnapshot(session_->session->session_snapshot());
        setLastError({});
        emit projectChanged();
        return true;
    } catch (const rust::Error&) {
        setLastError(tr("Could not rename the output."));
        return false;
    }
}

QString DesktopBackend::textNodeInput(const QString& draftId) {
    if (!session_)
        return {};
    try {
        return from_rust(session_->session->session_text_node_input(to_utf8(draftId)));
    } catch (const rust::Error&) {
        setLastError(tr("Could not read the original text."));
        return {};
    }
}

bool DesktopBackend::refreshTextInput(const QString& draftId) {
    if (!session_)
        return false;
    try {
        session_->session->session_refresh_text_input(to_utf8(draftId));
        applyOperatorDrafts(session_->session->session_operator_drafts());
        applyCandidates(session_->session->session_candidates());
        setLastError({});
        emit operatorDraftsChanged();
        emit candidateChanged();
        return true;
    } catch (const rust::Error&) {
        setLastError(tr("Could not update the original text input."));
        return false;
    }
}

bool DesktopBackend::updateTextAuthoring(const QString& draftId, const QString& json) {
    if (!session_)
        return false;
    try {
        session_->session->session_update_text_authoring(to_utf8(draftId), to_utf8(json));
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        const QString code = QString::fromUtf8(error.what());
        setLastError(
            code == QStringLiteral("writing_draft_too_large")
                ? tr("This draft is too large to save. Keep the draft and reference text under 48 "
                     "KiB in total.")
                : tr("Could not save the writing draft. Your text is still in the editor.")
        );
        return false;
    }
}

QString
DesktopBackend::textAuthoringContent(const QString& artifactId, const QString& candidateId) {
    if (!session_)
        return {};
    try {
        return from_rust(session_->session->session_text_authoring_content(
            to_utf8(artifactId),
            to_utf8(candidateId)
        ));
    } catch (const rust::Error&) {
        setLastError(tr("Could not read the complete text."));
        return {};
    }
}

QString DesktopBackend::textAuthoringPreview(const QString& profile, const QString& text) {
    if (!session_)
        return {};
    try {
        return from_rust(shape::desktop::text_authoring_preview(to_utf8(profile), to_utf8(text)));
    } catch (const rust::Error&) {
        return {};
    }
}

QString
DesktopBackend::textAuthoringConfiguredPreview(const QString& settings, const QString& text) {
    if (!session_)
        return {};
    try {
        return from_rust(
            shape::desktop::text_authoring_configured_preview(to_utf8(settings), to_utf8(text))
        );
    } catch (const rust::Error&) {
        return {};
    }
}

bool DesktopBackend::proposeAuthoredText(const QString& draftId) {
    if (!session_)
        return false;
    try {
        const auto candidate = session_->session->session_propose_authored_text(to_utf8(draftId));
        applyCandidates(session_->session->session_candidates(), from_rust(candidate.candidate_id));
        setLastError(QString());
        emit candidateChanged();
        return true;
    } catch (const rust::Error&) {
        setLastError(tr("Check the text and script instructions before adopting this draft."));
        return false;
    }
}

QString DesktopBackend::beginAuthoringSpeech(const QString& artifactId) {
    if (!session_)
        return {};
    try {
        const auto draft = session_->session->session_begin_authoring_speech(to_utf8(artifactId));
        if (std::none_of(artifacts_.cbegin(), artifacts_.cend(), [&draft](const QVariant& a) {
                return a.toMap().value(QStringLiteral("id")).toString()
                       == from_rust(draft.context_artifact_id);
            })) {
            QString sourceName;
            for (const auto& item : artifacts_) {
                const auto source = item.toMap();
                if (source.value(QStringLiteral("id")).toString() == artifactId)
                    sourceName = source.value(QStringLiteral("name")).toString();
            }
            session_->session->session_rename_artifact(
                draft.context_artifact_id,
                to_utf8(tr("%1 · Audio").arg(sourceName.left(24)))
            );
        }
        applySnapshot(session_->session->session_snapshot());
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit operatorDraftsChanged();
        return from_rust(draft.draft_id);
    } catch (const rust::Error&) {
        setLastError(tr("Adopt a valid script before choosing voices."));
        return {};
    }
}

QString
DesktopBackend::beginOperatorDraft(const QString& artifactId, const QString& operatorTypeKey) {
    if (session_ == nullptr || artifactId.isEmpty() || operatorTypeKey.isEmpty()) {
        setLastError(tr("Choose a Scene before adding an Operator."));
        return QString();
    }
    try {
        const auto draft = session_->session->session_begin_operator_draft(
            to_utf8(artifactId),
            to_utf8(operatorTypeKey)
        );
        if (operatorTypeKey.startsWith(QStringLiteral("text."))
            || operatorTypeKey == QStringLiteral("audio.speech_synthesize")) {
            QString sourceName;
            for (const auto& item : artifacts_) {
                const auto source = item.toMap();
                if (source.value(QStringLiteral("id")).toString() == artifactId)
                    sourceName = source.value(QStringLiteral("name")).toString();
            }
            const QHash<QString, QString> labels{
                {QStringLiteral("text.translate"), tr("Translated text")},
                {QStringLiteral("text.summarize"), tr("Summary")},
                {QStringLiteral("text.polish"), tr("Polished text")},
                {QStringLiteral("text.expand"), tr("Expanded text")},
                {QStringLiteral("text.outline"), tr("Outline")},
                {QStringLiteral("text.prepare_script"), tr("Narration script")}
            };
            const auto name =
                labels.contains(operatorTypeKey)
                    ? sourceName.left(24) + QStringLiteral(" · ") + labels.value(operatorTypeKey)
                : operatorTypeKey == QStringLiteral("audio.speech_synthesize")
                    ? tr("%1 · Audio").arg(sourceName.left(24))
                    : tr("%1 · Edited text").arg(sourceName.left(24));
            session_->session->session_rename_artifact(draft.context_artifact_id, to_utf8(name));
        }
        applySnapshot(session_->session->session_snapshot());
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit operatorDraftsChanged();
        return from_rust(draft.draft_id);
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not begin Operator draft:" << error.what();
        setLastError(tr("This Operator cannot use the selected Scene source."));
        return QString();
    }
}

QVariantList DesktopBackend::compatibleOperators(const QString& artifactId) {
    if (session_ == nullptr) {
        return {};
    }
    try {
        const auto descriptors =
            session_->session->session_operator_descriptors(to_utf8(artifactId));
        QVariantList projected;
        projected.reserve(static_cast<qsizetype>(descriptors.size()));
        for (const auto& descriptor : descriptors) {
            projected.append(operator_descriptor_projection(descriptor));
        }
        return projected;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not load compatible Operators:" << error.what();
        return {};
    }
}

bool DesktopBackend::updateTextTransformDraft(
    const QString& draftId,
    const QString& modeKey,
    const QString& instruction,
    const QString& expressionJson,
    const QString& styleKey,
    const int variantCount
) {
    if (session_ == nullptr || draftId.isEmpty() || modeKey.isEmpty() || expressionJson.isEmpty()
        || styleKey.isEmpty() || variantCount < 1 || variantCount > 4) {
        return false;
    }
    try {
        session_->session->session_update_text_expression_draft(
            to_utf8(draftId),
            to_utf8(modeKey),
            to_utf8(instruction),
            to_utf8(expressionJson),
            to_utf8(styleKey),
            static_cast<std::uint8_t>(variantCount)
        );
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not save text transform draft:" << error.what();
        setLastError(tr("Could not save the Operator draft."));
        return false;
    }
}

bool DesktopBackend::updateAudioSpeechDraft(
    const QString& draftId,
    const QString& presetAlias,
    const QString& presetCatalogRevision,
    const QString& language,
    int speedMilli,
    bool syntheticDisclosureRequired
) {
    if (session_ == nullptr || draftId.isEmpty() || speedMilli < 0 || speedMilli > 65'535) {
        return false;
    }
    try {
        session_->session->session_update_audio_speech_draft(
            to_utf8(draftId),
            to_utf8(presetAlias),
            to_utf8(presetCatalogRevision),
            to_utf8(language),
            static_cast<std::uint16_t>(speedMilli),
            syntheticDisclosureRequired
        );
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not save audio speech draft:" << error.what();
        setLastError(tr("Could not save the Operator draft."));
        return false;
    }
}

bool DesktopBackend::updateSpeechScript(const QString& draftId, const QString& optionsJson) {
    if (!session_ || speech_cue_importing_)
        return false;
    try {
        session_->session->session_update_speech_script(to_utf8(draftId), to_utf8(optionsJson));
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error&) {
        setLastError(tr("Could not save the narration script settings."));
        return false;
    }
}

QString DesktopBackend::speechScriptPreview(const QString& draftId) {
    if (!session_ || draftId.isEmpty())
        return QString();
    try {
        return from_rust(session_->session->session_speech_script_preview(to_utf8(draftId)));
    } catch (const rust::Error&) {
        return QString();
    }
}

void DesktopBackend::importSpeechCue(
    const QString& draftId,
    const QString& label,
    const QUrl& source
) {
    if (!session_ || speech_cue_importing_ || !source.isLocalFile())
        return;
    const QString project = project_id_;
    const QString bundle = bundle_path_;
    const QString preview = speechScriptPreview(draftId);
    auto* watcher = new QFutureWatcher<QByteArray>(this);
    speech_cue_importing_ = true;
    emit speechCueImportingChanged();
    connect(
        watcher,
        &QFutureWatcher<QByteArray>::finished,
        this,
        [this, watcher, project, bundle, preview, draftId, label]() {
            const QByteArray bytes = watcher->result();
            watcher->deleteLater();
            speech_cue_importing_ = false;
            emit speechCueImportingChanged();
            if (!session_ || project_id_ != project || bundle_path_ != bundle
                || speechScriptPreview(draftId) != preview)
                return;
            try {
                if (bytes.isEmpty())
                    throw std::runtime_error("invalid cue file");
                session_->session->session_import_speech_cue(
                    to_utf8(draftId),
                    to_utf8(label),
                    rust::Slice<const std::uint8_t>(
                        reinterpret_cast<const std::uint8_t*>(bytes.constData()),
                        static_cast<std::size_t>(bytes.size())
                    )
                );
                applyOperatorDrafts(session_->session->session_operator_drafts());
                setLastError(QString());
                emit operatorDraftsChanged();
            } catch (const std::exception&) {
                setLastError(tr("Choose a 24 kHz mono PCM16 WAV file up to 8 MiB."));
            }
        }
    );
    watcher->setFuture(QtConcurrent::run([path = source.toLocalFile()]() {
        constexpr qint64 limit = 8 * 1024 * 1024;
        const QFileInfo info(path);
        if (!info.isFile() || info.size() <= 44 || info.size() > limit)
            return QByteArray();
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly))
            return QByteArray();
        QByteArray bytes = file.read(limit + 1);
        return bytes.size() <= limit ? bytes : QByteArray();
    }));
}

bool DesktopBackend::updateImageResizeDraft(
    const QString& draftId,
    int targetWidth,
    int targetHeight,
    const QString& aspectPolicyKey,
    const QString& resamplingKey
) {
    constexpr int kMaximumResizeDimension = 32'768;
    if (session_ == nullptr || draftId.isEmpty() || targetWidth <= 0 || targetHeight <= 0
        || targetWidth > kMaximumResizeDimension || targetHeight > kMaximumResizeDimension
        || aspectPolicyKey.isEmpty() || resamplingKey.isEmpty()) {
        setLastError(tr("Choose valid resize dimensions and policies."));
        return false;
    }
    try {
        session_->session->session_update_image_resize_draft(
            to_utf8(draftId),
            static_cast<std::uint32_t>(targetWidth),
            static_cast<std::uint32_t>(targetHeight),
            to_utf8(aspectPolicyKey),
            to_utf8(resamplingKey)
        );
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not save image resize draft:" << error.what();
        setLastError(tr("Could not save the Operator draft."));
        return false;
    }
}

bool DesktopBackend::updateAiImageDraft(
    const QString& draftId,
    const QString& instruction,
    int outputWidth,
    int outputHeight
) {
    constexpr int kMaximumImageDimension = 4096;
    if (session_ == nullptr || draftId.isEmpty() || instruction.trimmed().isEmpty()
        || outputWidth <= 0 || outputHeight <= 0 || outputWidth > kMaximumImageDimension
        || outputHeight > kMaximumImageDimension) {
        setLastError(tr("Enter an image description and valid dimensions."));
        return false;
    }
    try {
        session_->session->session_update_ai_image_draft(
            to_utf8(draftId),
            to_utf8(instruction),
            static_cast<std::uint32_t>(outputWidth),
            static_cast<std::uint32_t>(outputHeight)
        );
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not save AI image draft:" << error.what();
        setLastError(tr("Could not save the AI image draft."));
        return false;
    }
}

bool DesktopBackend::discardOperatorDraft(const QString& draftId) {
    if (session_ == nullptr || draftId.isEmpty()) {
        return false;
    }
    try {
        session_->session->session_discard_operator_draft(to_utf8(draftId));
        applySnapshot(session_->session->session_snapshot());
        applyCandidates(session_->session->session_candidates());
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit projectChanged();
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not discard Operator draft:" << error.what();
        setLastError(tr("Could not remove the Operator draft."));
        return false;
    }
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
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster crop candidate:" << error.what();
        setLastError(tr("Could not create the crop candidate."));
        return false;
    }
}

bool DesktopBackend::proposeRasterResize(const QString& artifactId, const QString& draftId) {
    if (session_ == nullptr || artifactId.isEmpty() || draftId.isEmpty()) {
        setLastError(tr("Open a valid Resize Operator draft."));
        return false;
    }
    try {
        const auto candidate =
            session_->session->session_propose_raster_resize(to_utf8(artifactId), to_utf8(draftId));
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_candidates(), candidate_id);
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster resize candidate:" << error.what();
        setLastError(tr("Could not create the resize candidate."));
        return false;
    }
}

bool DesktopBackend::proposeRasterTransform(
    const QString& artifactId,
    const QString& transformKey
) {
    if (session_ == nullptr || artifactId.isEmpty() || transformKey.isEmpty()) {
        setLastError(tr("Choose a valid image transform."));
        return false;
    }
    try {
        const auto candidate = session_->session->session_propose_raster_transform(
            to_utf8(artifactId),
            to_utf8(transformKey)
        );
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_candidates(), candidate_id);
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster transform candidate:" << error.what();
        setLastError(tr("Could not create the transform candidate."));
        return false;
    }
}

bool DesktopBackend::proposeRasterBlur(const QString& artifactId, int radius) {
    if (session_ == nullptr || artifactId.isEmpty() || radius < 1 || radius > 64) {
        setLastError(tr("Choose a blur radius from 1 to 64 pixels."));
        return false;
    }
    try {
        const auto candidate = session_->session->session_propose_raster_blur(
            to_utf8(artifactId),
            static_cast<std::uint16_t>(radius)
        );
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_candidates(), candidate_id);
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster blur candidate:" << error.what();
        setLastError(tr(
            "Could not create the blur candidate. Embedded color profiles are not supported yet."
        ));
        return false;
    }
}

bool DesktopBackend::proposeRasterUnsharpMask(
    const QString& artifactId,
    int radius,
    int amountMilli,
    int threshold
) {
    if (session_ == nullptr || artifactId.isEmpty() || radius < 1 || radius > 64 || amountMilli < 1
        || amountMilli > 4000 || threshold < 0 || threshold > 255) {
        setLastError(tr("Choose valid sharpen settings."));
        return false;
    }
    try {
        const auto candidate = session_->session->session_propose_raster_unsharp_mask(
            to_utf8(artifactId),
            static_cast<std::uint16_t>(radius),
            static_cast<std::uint16_t>(amountMilli),
            static_cast<std::uint8_t>(threshold)
        );
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_candidates(), candidate_id);
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster unsharp-mask candidate:" << error.what();
        setLastError(
            tr("Could not create the sharpen candidate. Embedded color profiles are not supported "
               "yet.")
        );
        return false;
    }
}

bool DesktopBackend::proposeRasterDropShadow(
    const QString& artifactId,
    int offsetX,
    int offsetY,
    int blurRadius,
    int red,
    int green,
    int blue,
    int alpha
) {
    const auto valid_channel = [](int value) { return value >= 0 && value <= 255; };
    if (session_ == nullptr || artifactId.isEmpty() || offsetX < -4096 || offsetX > 4096
        || offsetY < -4096 || offsetY > 4096 || blurRadius < 0 || blurRadius > 64
        || !valid_channel(red) || !valid_channel(green) || !valid_channel(blue) || alpha < 1
        || alpha > 255) {
        setLastError(tr("Choose valid drop-shadow settings."));
        return false;
    }
    try {
        const auto candidate = session_->session->session_propose_raster_drop_shadow(
            to_utf8(artifactId),
            static_cast<std::int32_t>(offsetX),
            static_cast<std::int32_t>(offsetY),
            static_cast<std::uint16_t>(blurRadius),
            static_cast<std::uint8_t>(red),
            static_cast<std::uint8_t>(green),
            static_cast<std::uint8_t>(blue),
            static_cast<std::uint8_t>(alpha)
        );
        const QString candidate_id = from_rust(candidate.candidate_id);
        applyCandidates(session_->session->session_candidates(), candidate_id);
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning().noquote() << "could not create raster drop-shadow candidate:" << error.what();
        setLastError(
            tr("Could not create the drop-shadow candidate. The image or expanded canvas may be "
               "too large.")
        );
        return false;
    }
}

bool DesktopBackend::prepareImagePreviews(const QString& artifactId, const QString& candidateId) {
    if (session_ == nullptr || artifactId.isEmpty()) {
        return false;
    }
    try {
        const auto artifact = std::find_if(
            artifacts_.cbegin(),
            artifacts_.cend(),
            [&artifactId](const QVariant& value) {
                return value.toMap().value(QStringLiteral("id")).toString() == artifactId;
            }
        );
        if (artifact == artifacts_.cend()) {
            return false;
        }
        QString accepted_source;
        if (artifact->toMap().value(QStringLiteral("hasAcceptedRevision")).toBool()) {
            accepted_source = cacheImagePreview(
                session_->session->session_image_preview(to_utf8(artifactId), std::string())
            );
        }
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
        applyOperatorDrafts(session_->session->session_operator_drafts());
        setLastError(QString());
        emit candidateChanged();
        emit operatorDraftsChanged();
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
    applyOperatorDrafts(session_->session->session_operator_drafts());
    setLastError(QString());
    emit candidateChanged();
    emit operatorDraftsChanged();
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
    applyOperatorDrafts(session_->session->session_operator_drafts());
    setLastError(QString());
    emit candidateChanged();
    emit operatorDraftsChanged();
    return candidate_id;
}

QString
DesktopBackend::adoptInferImageCandidate(rust::Box<shape::desktop::InferImageCandidate> candidate) {
    if (session_ == nullptr) {
        return QString();
    }
    const auto adopted = session_->session->session_adopt_infer_image(std::move(candidate));
    const QString candidate_id = from_rust(adopted.candidate_id);
    applyCandidates(session_->session->session_candidates(), candidate_id);
    applyOperatorDrafts(session_->session->session_operator_drafts());
    setLastError(QString());
    emit candidateChanged();
    emit operatorDraftsChanged();
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

void DesktopBackend::applyOperatorDrafts(rust::Vec<shape::desktop::OperatorDraftWire> drafts) {
    QVariantList projected;
    projected.reserve(static_cast<qsizetype>(drafts.size()));
    for (const auto& draft : drafts) {
        projected.append(operator_draft_projection(draft));
    }
    operator_drafts_ = std::move(projected);
}

void DesktopBackend::replaceSession(rust::Box<shape::desktop::DesktopSession> session) {
    session_ = std::make_unique<SessionState>(std::move(session));
    image_preview_store_->clear();
    accepted_image_source_.clear();
    candidate_image_source_.clear();
    applySnapshot(session_->session->session_snapshot());
    applyCandidates(session_->session->session_candidates());
    applyOperatorDrafts(session_->session->session_operator_drafts());
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

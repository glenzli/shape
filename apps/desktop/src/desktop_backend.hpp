//! Qt presentation facade over one optional Rust-owned desktop session.

#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include <memory>
#include <optional>

#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"

class ImagePreviewStore;

struct AudioPreviewData {
    QString identity;
    QByteArray wav_bytes;
    qint64 duration_millis = 0;
    int sample_rate_hz = 0;
    int channels = 0;
};

class DesktopBackend : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("DesktopBackend is created by the host application")
    Q_PROPERTY(bool projectOpen READ projectOpen NOTIFY projectChanged)
    Q_PROPERTY(QString projectId READ projectId NOTIFY projectChanged)
    Q_PROPERTY(QString projectName READ projectName NOTIFY projectChanged)
    Q_PROPERTY(QString schemaRevision READ schemaRevision NOTIFY projectChanged)
    Q_PROPERTY(QString bundlePath READ bundlePath NOTIFY projectChanged)
    Q_PROPERTY(int artifactCount READ artifactCount NOTIFY projectChanged)
    Q_PROPERTY(QVariantList artifacts READ artifacts NOTIFY projectChanged)
    Q_PROPERTY(QVariantList graphEdges READ graphEdges NOTIFY projectChanged)
    Q_PROPERTY(QVariantList operatorDrafts READ operatorDrafts NOTIFY operatorDraftsChanged)
    Q_PROPERTY(int candidateCount READ candidateCount NOTIFY candidateChanged)
    Q_PROPERTY(QVariantList candidates READ candidates NOTIFY candidateChanged)
    Q_PROPERTY(bool hasCandidate READ hasCandidate NOTIFY candidateChanged)
    Q_PROPERTY(QString candidateId READ candidateId NOTIFY candidateChanged)
    Q_PROPERTY(QString candidateArtifactId READ candidateArtifactId NOTIFY candidateChanged)
    Q_PROPERTY(QString candidateText READ candidateText NOTIFY candidateChanged)
    Q_PROPERTY(bool candidateTextTruncated READ candidateTextTruncated NOTIFY candidateChanged)
    Q_PROPERTY(QString acceptedImageSource READ acceptedImageSource NOTIFY imagePreviewChanged)
    Q_PROPERTY(QString candidateImageSource READ candidateImageSource NOTIFY imagePreviewChanged)
    Q_PROPERTY(bool speechCueImporting READ speechCueImporting NOTIFY speechCueImportingChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)

  public:
    explicit DesktopBackend(QObject* parent = nullptr);
    explicit DesktopBackend(
        rust::Box<shape::desktop::DesktopSession> session,
        QObject* parent = nullptr
    );
    ~DesktopBackend() override;

    [[nodiscard]] bool projectOpen() const;
    [[nodiscard]] QString projectId() const;
    [[nodiscard]] QString projectName() const;
    [[nodiscard]] QString schemaRevision() const;
    [[nodiscard]] QString bundlePath() const;
    [[nodiscard]] int artifactCount() const;
    [[nodiscard]] QVariantList artifacts() const;
    [[nodiscard]] QVariantList graphEdges() const;
    [[nodiscard]] QVariantList operatorDrafts() const;

    [[nodiscard]] int candidateCount() const;
    [[nodiscard]] QVariantList candidates() const;
    [[nodiscard]] bool hasCandidate() const;
    [[nodiscard]] QString candidateId() const;
    [[nodiscard]] QString candidateArtifactId() const;
    [[nodiscard]] QString candidateText() const;
    [[nodiscard]] bool candidateTextTruncated() const;
    [[nodiscard]] QString acceptedImageSource() const;
    [[nodiscard]] QString candidateImageSource() const;
    [[nodiscard]] QString lastError() const;

    Q_INVOKABLE bool createProject(const QUrl& parentDirectory, const QString& projectName);
    Q_INVOKABLE bool openProject(const QUrl& bundleUrl);
    Q_INVOKABLE bool createTextScene(const QString& sceneName, const QString& initialText);
    Q_INVOKABLE bool createAiImageScene(
        const QString& sceneName,
        const QString& instruction,
        int outputWidth,
        int outputHeight
    );
    Q_INVOKABLE bool createDetachedTextEditor(const QString& nodeName);
    Q_INVOKABLE bool createTextAuthoring(const QString& name, const QString& profile);
    Q_INVOKABLE QString beginTextAuthoring(const QString& artifactId, const QString& profile);
    Q_INVOKABLE bool updateTextAuthoring(const QString& draftId, const QString& json);
    Q_INVOKABLE QString textAuthoringContent(const QString& artifactId, const QString& candidateId);
    Q_INVOKABLE QString sourceRevisionText(const QString& revisionId);
    Q_INVOKABLE QString
    textAuthoringConfiguredPreview(const QString& settings, const QString& text);
    Q_INVOKABLE QString textAuthoringPreview(const QString& profile, const QString& text);
    Q_INVOKABLE QString textNodeInput(const QString& draftId);
    Q_INVOKABLE bool refreshTextInput(const QString& draftId);
    Q_INVOKABLE bool renameArtifact(const QString& artifactId, const QString& name);
    Q_INVOKABLE bool proposeAuthoredText(const QString& draftId);
    Q_INVOKABLE QString beginAuthoringSpeech(const QString& artifactId);
    Q_INVOKABLE QString
    beginOperatorDraft(const QString& artifactId, const QString& operatorTypeKey);
    Q_INVOKABLE QVariantList compatibleOperators(const QString& artifactId);
    Q_INVOKABLE bool updateTextTransformDraft(
        const QString& draftId,
        const QString& modeKey,
        const QString& instruction,
        const QString& expressionJson,
        const QString& styleKey,
        int variantCount
    );
    Q_INVOKABLE bool updateAudioSpeechDraft(
        const QString& draftId,
        const QString& presetAlias,
        const QString& presetCatalogRevision,
        const QString& language,
        int speedMilli,
        bool syntheticDisclosureRequired
    );
    [[nodiscard]] bool speechCueImporting() const {
        return speech_cue_importing_;
    }
    Q_INVOKABLE bool updateSpeechScript(const QString& draftId, const QString& optionsJson);
    Q_INVOKABLE QString speechScriptPreview(const QString& draftId);
    Q_INVOKABLE void
    importSpeechCue(const QString& draftId, const QString& label, const QUrl& source);
    Q_INVOKABLE bool updateImageResizeDraft(
        const QString& draftId,
        int targetWidth,
        int targetHeight,
        const QString& aspectPolicyKey,
        const QString& resamplingKey
    );
    Q_INVOKABLE bool updateAiImageDraft(
        const QString& draftId,
        const QString& instruction,
        int outputWidth,
        int outputHeight,
        int candidateCount
    );
    Q_INVOKABLE bool discardOperatorDraft(const QString& draftId);
    Q_INVOKABLE bool
    proposeTextCandidate(const QString& artifactId, const QString& replacementText);
    Q_INVOKABLE bool importRaster(const QUrl& sourceUrl);
    Q_INVOKABLE bool importMaterial(const QUrl& sourceUrl);
    Q_INVOKABLE bool pasteMaterial();
    Q_INVOKABLE bool exportAcceptedMaterial(const QString& artifactId, const QUrl& targetUrl);
    Q_INVOKABLE bool
    proposeRasterCrop(const QString& artifactId, int x, int y, int width, int height);
    Q_INVOKABLE bool proposeRasterResize(const QString& artifactId, const QString& draftId);
    Q_INVOKABLE bool proposeRasterTransform(const QString& artifactId, const QString& transformKey);
    Q_INVOKABLE bool proposeRasterBlur(const QString& artifactId, int radius);
    Q_INVOKABLE bool
    proposeRasterUnsharpMask(const QString& artifactId, int radius, int amountMilli, int threshold);
    Q_INVOKABLE bool proposeRasterDropShadow(
        const QString& artifactId,
        int offsetX,
        int offsetY,
        int blurRadius,
        int red,
        int green,
        int blue,
        int alpha
    );
    Q_INVOKABLE bool
    prepareImagePreviews(const QString& artifactId, const QString& candidateId = QString());
    Q_INVOKABLE QString candidateThumbnailSource(const QString& candidateId) const;
    Q_INVOKABLE bool selectCandidate(const QString& candidateId);
    Q_INVOKABLE bool acceptCandidate(const QString& candidateId);
    Q_INVOKABLE bool branchCandidate(const QString& candidateId, const QString& artifactName);
    Q_INVOKABLE bool discardCandidate(const QString& candidateId);

    /// Adopts one fully executed background Infer candidate on the UI thread.
    /// Stale-head validation remains authoritative in the Rust session.
    [[nodiscard]] QString
    adoptInferTextCandidate(rust::Box<shape::desktop::InferTextCandidate> candidate);
    /// Adopts one background speech result on the UI thread after Rust revalidates its source.
    [[nodiscard]] QString
    adoptInferSpeechCandidate(rust::Box<shape::desktop::InferSpeechCandidate> candidate);
    /// Adopts one background source-less image result after exact draft revalidation.
    [[nodiscard]] QString
    adoptInferImageCandidate(rust::Box<shape::desktop::InferImageCandidate> candidate);
    [[nodiscard]] QStringList
    adoptInferImageBatch(rust::Box<shape::desktop::InferImageBatch> batch);

    /// Fetches exact WAV bytes only for the preview selected by the audio controller.
    [[nodiscard]] std::optional<AudioPreviewData>
    audioPreview(
        const QString& artifactId,
        const QString& candidateId = QString(),
        const QString& revisionId = QString()
    );

    /// Rebuilds translated presentation values after a runtime locale change.
    void retranslate();

    [[nodiscard]] std::shared_ptr<ImagePreviewStore> imagePreviewStore() const;

  signals:
    void speechCueImportingChanged();
    void projectChanged();
    void candidateChanged();
    void operatorDraftsChanged();
    void lastErrorChanged();
    void imagePreviewChanged();

  private:
    struct SessionState;

    void applySnapshot(shape::desktop::ProjectSnapshotWire snapshot);
    void applyOperatorDrafts(rust::Vec<shape::desktop::OperatorDraftWire> drafts);
    void applyCandidates(
        rust::Vec<shape::desktop::CandidateWire> candidates,
        const QString& preferredCandidateId = QString()
    );
    bool applyCandidateSelection(const QString& candidateId);
    void replaceSession(rust::Box<shape::desktop::DesktopSession> session);
    void clearCandidateSelection();
    void setLastError(const QString& message);
    [[nodiscard]] QString cacheImagePreview(shape::desktop::ImagePreviewWire preview, bool thumbnail = false);

    std::unique_ptr<SessionState> session_;
    bool speech_cue_importing_ = false;
    bool project_open_ = false;
    QString project_id_;
    QString project_name_;
    QString schema_revision_;
    QString bundle_path_;
    QVariantList artifacts_;
    QVariantList graph_edges_;
    QVariantList operator_drafts_;
    QVariantList candidates_;

    bool has_candidate_ = false;
    QString candidate_id_;
    QString candidate_artifact_id_;
    QString candidate_text_;
    bool candidate_text_truncated_ = false;
    QString last_error_;
    std::shared_ptr<ImagePreviewStore> image_preview_store_;
    QString accepted_image_source_;
    QString candidate_image_source_;
};

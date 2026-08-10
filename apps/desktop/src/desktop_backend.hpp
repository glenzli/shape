//! Qt presentation facade over one optional Rust-owned desktop session.

#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>
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
    Q_INVOKABLE bool
    createTextScene(const QString& sceneName, const QString& initialText);
    Q_INVOKABLE QString
    beginOperatorDraft(const QString& artifactId, const QString& operatorTypeKey);
    Q_INVOKABLE bool discardOperatorDraft(const QString& draftId);
    Q_INVOKABLE bool
    proposeTextCandidate(const QString& artifactId, const QString& replacementText);
    Q_INVOKABLE bool importRaster(const QUrl& sourceUrl);
    Q_INVOKABLE bool
    proposeRasterCrop(const QString& artifactId, int x, int y, int width, int height);
    Q_INVOKABLE bool
    prepareImagePreviews(const QString& artifactId, const QString& candidateId = QString());
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

    /// Fetches exact WAV bytes only for the preview selected by the audio controller.
    [[nodiscard]] std::optional<AudioPreviewData>
    audioPreview(const QString& artifactId, const QString& candidateId = QString());

    /// Rebuilds translated presentation values after a runtime locale change.
    void retranslate();

    [[nodiscard]] std::shared_ptr<ImagePreviewStore> imagePreviewStore() const;

  signals:
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
    [[nodiscard]] QString cacheImagePreview(shape::desktop::ImagePreviewWire preview);

    std::unique_ptr<SessionState> session_;
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

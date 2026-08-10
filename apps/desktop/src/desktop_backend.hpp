//! Qt presentation facade over one optional Rust-owned desktop session.

#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include <memory>

#include "rust/cxx.h"
#include "shape-desktop-bridge/src/lib.rs.h"

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
    Q_PROPERTY(bool hasCandidate READ hasCandidate NOTIFY candidateChanged)
    Q_PROPERTY(QString candidateId READ candidateId NOTIFY candidateChanged)
    Q_PROPERTY(QString candidateArtifactId READ candidateArtifactId NOTIFY candidateChanged)
    Q_PROPERTY(QString candidateText READ candidateText NOTIFY candidateChanged)
    Q_PROPERTY(bool candidateTextTruncated READ candidateTextTruncated NOTIFY candidateChanged)
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

    [[nodiscard]] bool hasCandidate() const;
    [[nodiscard]] QString candidateId() const;
    [[nodiscard]] QString candidateArtifactId() const;
    [[nodiscard]] QString candidateText() const;
    [[nodiscard]] bool candidateTextTruncated() const;
    [[nodiscard]] QString lastError() const;

    Q_INVOKABLE bool
    proposeTextCandidate(const QString& artifactId, const QString& replacementText);
    Q_INVOKABLE bool acceptCandidate();
    Q_INVOKABLE bool branchCandidate(const QString& artifactName);
    Q_INVOKABLE void discardCandidate();

    /// Rebuilds translated presentation values after a runtime locale change.
    void retranslate();

  signals:
    void projectChanged();
    void candidateChanged();
    void lastErrorChanged();

  private:
    struct SessionState;

    void applySnapshot(shape::desktop::ProjectSnapshotWire snapshot);
    void applyCandidate(shape::desktop::TextCandidateWire candidate);
    void clearCandidate();
    void setLastError(const QString& message);

    std::unique_ptr<SessionState> session_;
    bool project_open_ = false;
    QString project_id_;
    QString project_name_;
    QString schema_revision_;
    QString bundle_path_;
    QVariantList artifacts_;
    QVariantList graph_edges_;

    bool has_candidate_ = false;
    QString candidate_id_;
    QString candidate_artifact_id_;
    QString candidate_text_;
    bool candidate_text_truncated_ = false;
    QString last_error_;
};

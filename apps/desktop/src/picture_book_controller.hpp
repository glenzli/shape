#pragma once
#include "picture_book_renderer.hpp"
#include <QJsonObject>
#include <QObject>
#include <QUrl>
#include <QVariantList>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>
class DesktopBackend;
class PictureBookController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Created by the Shape host")
    Q_PROPERTY(QVariantMap composition READ composition NOTIFY documentChanged)
    Q_PROPERTY(QVariantMap source READ source NOTIFY documentChanged)
    Q_PROPERTY(QVariantMap draft READ draft NOTIFY draftChanged)
    Q_PROPERTY(QVariantList drafts READ drafts NOTIFY draftsChanged)
    Q_PROPERTY(bool hasDraftChanges READ hasDraftChanges NOTIFY documentChanged)
    Q_PROPERTY(QString previewSource READ previewSource NOTIFY previewChanged)
    Q_PROPERTY(QString status READ status NOTIFY previewChanged)
    Q_PROPERTY(QString error READ error NOTIFY previewChanged)
    Q_PROPERTY(bool hasTitlePage READ hasTitlePage NOTIFY previewChanged)
    Q_PROPERTY(bool hasReview READ hasReview NOTIFY reviewChanged)
    Q_PROPERTY(bool hasAccepted READ hasAccepted NOTIFY documentChanged)
    Q_PROPERTY(QString candidateId READ candidateId NOTIFY reviewChanged)
    Q_PROPERTY(QUrl nativeExport READ nativeExport NOTIFY previewChanged)
  public:
    explicit PictureBookController(DesktopBackend& backend, QObject* parent = nullptr);
    QVariantMap composition() const {
        return composition_.toVariantMap();
    }
    QVariantMap source() const {
        return source_.toVariantMap();
    }
    QVariantMap draft() const {
        return draft_.toVariantMap();
    }
    QVariantList drafts() const {
        return drafts_;
    }
    bool hasDraftChanges() const {
        return draft_changes_;
    }
    QString previewSource() const {
        return preview_source_;
    }
    QString status() const {
        return status_;
    }
    QString error() const {
        return error_;
    }
    bool hasTitlePage() const;
    bool hasReview() const {
        return !candidate_id_.isEmpty();
    }
    bool hasAccepted() const {
        return !source_.isEmpty();
    }
    QString candidateId() const {
        return candidate_id_;
    }
    QUrl nativeExport() const {
        return native_export_;
    }
    Q_INVOKABLE void beginNew();
    Q_INVOKABLE bool refresh();
    Q_INVOKABLE bool saveState(const QVariantMap& state);
    Q_INVOKABLE bool discardDraft();
    Q_INVOKABLE bool load(const QString& artifactId);
    Q_INVOKABLE void invalidate();
    Q_INVOKABLE bool review(const QVariantMap& composition);
    Q_INVOKABLE bool accept();
    Q_INVOKABLE bool showPage(const QString& language, int page);
    Q_INVOKABLE QString scriptText(const QVariantMap& source);
    Q_INVOKABLE QVariantMap
    adoptScript(const QString& name, const QVariantMap& source, const QString& text);
    Q_INVOKABLE QVariantMap refreshMaterials(const QVariantMap& composition) const;
    Q_INVOKABLE QVariantMap beginArtworkEdit(const QVariantMap& source);
    Q_INVOKABLE QVariantMap adoptArtwork(const QString& outputId, const QString& candidateId);
    Q_INVOKABLE QString
    artworkCandidatePreview(const QString& outputId, const QString& candidateId);
    Q_INVOKABLE QString artworkPreview(const QVariantMap& source);
    Q_INVOKABLE bool exportNative(const QUrl& destination);
    Q_INVOKABLE bool revealNativeExport();
    Q_INVOKABLE bool
    exportPdf(const QUrl& destination, const QString& language, bool includePrintOnly);
    // Production-linked smoke can inspect exact rendered pages without a screenshot substitute.
    QImage
    pageImage(const QString& language, int page, picture_book::LayoutResult* result = nullptr);
  signals:
    void documentChanged();
    void draftChanged();
    void draftsChanged();
    void reviewChanged();
    void previewChanged();

  private:
    void fail(const QString& message, const std::exception& error);
    QJsonObject draftAction(const QJsonObject& request);
    void setDraft(const QJsonObject& value);
    QJsonObject preview(const QJsonObject& composition);
    picture_book::Page page(const QJsonObject& preview, const QString& language, int index);
    DesktopBackend& backend_;
    QString project_id_, project_path_;
    QJsonObject source_, composition_, preview_;
    QJsonObject draft_, accepted_composition_;
    QVariantList drafts_;
    bool draft_changes_ = true;
    QString candidate_id_, candidate_artifact_id_, preview_source_, status_, error_;
    QUrl native_export_;
};

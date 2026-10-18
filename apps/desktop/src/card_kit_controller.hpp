//! Recoverable card-kit authoring and explicit reviewed acceptance.
#pragma once
#include <QJsonObject>
#include <QObject>
#include <QStringList>
#include <QUrl>
#include <QVariantList>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>
#include <exception>
#include <memory>
class DesktopBackend;
class QSettings;

class CardKitController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Created by the Shape host")
    Q_PROPERTY(QVariantMap draft READ draft NOTIFY documentChanged)
    Q_PROPERTY(QVariantList drafts READ drafts NOTIFY draftsChanged)
    Q_PROPERTY(QStringList templateKeys READ templateKeys NOTIFY draftsChanged)
    Q_PROPERTY(QVariantMap source READ source NOTIFY documentChanged)
    Q_PROPERTY(QVariantMap preview READ preview NOTIFY previewChanged)
    Q_PROPERTY(QString previewSource READ previewSource NOTIFY previewChanged)
    Q_PROPERTY(QString error READ error NOTIFY statusChanged)
    Q_PROPERTY(QString status READ status NOTIFY statusChanged)
    Q_PROPERTY(bool hasReview READ hasReview NOTIFY previewChanged)
    Q_PROPERTY(bool canExport READ canExport NOTIFY previewChanged)
  public:
    explicit CardKitController(DesktopBackend& backend, QObject* parent = nullptr);
    CardKitController(
        DesktopBackend& backend,
        const QString& settingsPath,
        QObject* parent = nullptr
    );
    ~CardKitController() override;
    QVariantMap draft() const {
        return draft_.toVariantMap();
    }
    QVariantList drafts() const {
        return drafts_;
    }
    QVariantMap source() const {
        return source_.toVariantMap();
    }
    QVariantMap preview() const {
        return preview_.toVariantMap();
    }
    QString previewSource() const {
        return preview_source_;
    }
    QString error() const {
        return tr(error_.toUtf8().constData());
    }
    QString status() const {
        return tr(status_.toUtf8().constData());
    }
    QStringList templateKeys() const {
        return template_keys_;
    }
    bool hasReview() const {
        return !review_digest_.isEmpty();
    }
    bool canExport() const;
    Q_INVOKABLE bool refresh();
    Q_INVOKABLE bool beginNew();
    Q_INVOKABLE bool load(const QString& artifactId);
    Q_INVOKABLE bool save(const QVariantMap& composition);
    Q_INVOKABLE void invalidateReview();
    Q_INVOKABLE bool review(const QString& cardId);
    Q_INVOKABLE bool showCard(const QString& cardId);
    Q_INVOKABLE bool accept();
    Q_INVOKABLE bool exportFiles(const QUrl& destination);
    Q_INVOKABLE void copyDraft(const QVariantMap& composition);
    Q_INVOKABLE bool
    rememberArtworkEdit(const QString& cardId, const QVariantMap& source, const QVariantMap& stage);
    Q_INVOKABLE QVariantMap recoverArtworkEdit(const QString& cardId);
    Q_INVOKABLE void forgetArtworkEdit(const QString& cardId);
    void retranslate() {
        emit statusChanged();
    }
  signals:
    void documentChanged();
    void draftsChanged();
    void previewChanged();
    void statusChanged();

  private:
    QJsonObject action(QJsonObject request);
    void setDraft(const QJsonObject& value);
    void updateSource();
    void clear();
    bool fail(const char* message, const std::exception& error);
    QString artworkEditKey(const QString& cardId) const;
    QString legacyArtworkEditKey(const QString& cardId) const;
    DesktopBackend& backend_;
    std::unique_ptr<QSettings> settings_;
    QString project_id_, bundle_path_;
    QJsonObject draft_, source_, preview_;
    QVariantList drafts_;
    QStringList template_keys_;
    QString preview_source_, review_digest_, error_, status_;
    bool accepted_current_ = false;
};

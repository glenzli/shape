//! One isolated Shape image-generation task requested by a stable Deck card.
#pragma once
#include <QFutureWatcher>
#include <QJsonObject>
#include <QObject>
#include <QtQml/qqmlregistration.h>
class DesktopBackend;
class InferImageController;

class DeckMaterialTaskController : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Created by the Shape host")
    Q_PROPERTY(bool active READ active NOTIFY changed)
    Q_PROPERTY(bool busy READ busy NOTIFY changed)
    Q_PROPERTY(bool canReturn READ canReturn NOTIFY changed)
    Q_PROPERTY(QString cardId READ cardId NOTIFY changed)
    Q_PROPERTY(QString cardTitle READ cardTitle NOTIFY changed)
    Q_PROPERTY(QString error READ error NOTIFY changed)
    Q_PROPERTY(QString artifactId READ artifactId NOTIFY changed)
    Q_PROPERTY(QString draftId READ draftId NOTIFY changed)
  public:
    DeckMaterialTaskController(
        DesktopBackend& backend,
        InferImageController& image,
        QObject* parent = nullptr
    );
    ~DeckMaterialTaskController() override;
    bool
    initialize(const QString& requestPath, const QString& resultPath, const QString& taskRoot = {});
    bool active() const {
        return active_;
    }
    bool busy() const {
        return finishing_;
    }
    bool canReturn() const;
    QString cardId() const {
        return request_.value("target").toObject().value("card_id").toString();
    }
    QString cardTitle() const {
        const auto title = request_.value("target")
                               .toObject()
                               .value("card_snapshot")
                               .toObject()
                               .value("title")
                               .toString();
        return title.isEmpty() ? tr("Untitled card") : title;
    }
    QString artifactId() const {
        return artifact_id_;
    }
    QString draftId() const {
        return draft_id_;
    }
    QString error() const {
        return tr(error_.toUtf8().constData());
    }
    void retranslate() {
        emit changed();
    }
    Q_INVOKABLE void returnImage();
    Q_INVOKABLE bool cancel();
  signals:
    void changed();
    void finished();

  private:
    void finish();
    void startupFailed(const QString& reason);
    bool currentProject() const;
    DesktopBackend& backend_;
    InferImageController& image_;
    QFutureWatcher<QJsonObject> watcher_;
    QJsonObject request_;
    QString request_path_, result_path_, request_digest_, project_path_, project_id_, artifact_id_,
        draft_id_, error_;
    bool active_ = false;
    bool finishing_ = false;
};

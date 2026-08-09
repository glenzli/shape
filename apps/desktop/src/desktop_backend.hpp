//! Qt-owned presentation projection over the generated Rust project snapshot.

#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include "shape-desktop-bridge/src/lib.rs.h"

class DesktopBackend : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("DesktopBackend is created by the host application")
    Q_PROPERTY(bool projectOpen READ projectOpen CONSTANT)
    Q_PROPERTY(QString projectId READ projectId CONSTANT)
    Q_PROPERTY(QString projectName READ projectName CONSTANT)
    Q_PROPERTY(QString schemaRevision READ schemaRevision CONSTANT)
    Q_PROPERTY(QString bundlePath READ bundlePath CONSTANT)
    Q_PROPERTY(int artifactCount READ artifactCount CONSTANT)
    Q_PROPERTY(QVariantList artifacts READ artifacts CONSTANT)

  public:
    explicit DesktopBackend(QObject* parent = nullptr);
    explicit DesktopBackend(
        shape::desktop::ProjectSnapshotWire snapshot,
        QObject* parent = nullptr
    );

    [[nodiscard]] bool projectOpen() const;
    [[nodiscard]] QString projectId() const;
    [[nodiscard]] QString projectName() const;
    [[nodiscard]] QString schemaRevision() const;
    [[nodiscard]] QString bundlePath() const;
    [[nodiscard]] int artifactCount() const;
    [[nodiscard]] QVariantList artifacts() const;

  private:
    bool project_open_ = false;
    QString project_id_;
    QString project_name_;
    QString schema_revision_;
    QString bundle_path_;
    QVariantList artifacts_;
};

//! Bounded decoded-image cache and QML image provider for selected previews.

#pragma once

#include <QHash>
#include <QImage>
#include <QMutex>
#include <QQuickImageProvider>
#include <QStringList>

#include <memory>

class ImagePreviewStore {
  public:
    [[nodiscard]] bool put(const QString& identity, QImage image);
    [[nodiscard]] QImage image(const QString& identity) const;
    void remove(const QString& identity);
    void clear();

  private:
    void evictToBudget();

    static constexpr qsizetype maximumResidentBytes = 256 * 1024 * 1024;
    mutable QMutex mutex_;
    QHash<QString, QImage> images_;
    QStringList insertion_order_;
    qsizetype resident_bytes_ = 0;
};

class ImagePreviewProvider final : public QQuickImageProvider {
  public:
    explicit ImagePreviewProvider(std::shared_ptr<ImagePreviewStore> store);

    [[nodiscard]] QImage
    requestImage(const QString& id, QSize* size, const QSize& requestedSize) override;

  private:
    std::shared_ptr<ImagePreviewStore> store_;
};

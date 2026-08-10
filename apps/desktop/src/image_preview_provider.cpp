#include "image_preview_provider.hpp"

#include <QMutexLocker>

bool ImagePreviewStore::put(const QString& identity, QImage image) {
    if (identity.isEmpty() || image.isNull() || image.sizeInBytes() > maximumResidentBytes) {
        return false;
    }
    const QMutexLocker lock(&mutex_);
    if (const auto existing = images_.find(identity); existing != images_.end()) {
        resident_bytes_ -= existing->sizeInBytes();
        insertion_order_.removeAll(identity);
    }
    resident_bytes_ += image.sizeInBytes();
    images_.insert(identity, std::move(image));
    insertion_order_.append(identity);
    evictToBudget();
    return images_.contains(identity);
}

QImage ImagePreviewStore::image(const QString& identity) const {
    const QMutexLocker lock(&mutex_);
    return images_.value(identity);
}

void ImagePreviewStore::remove(const QString& identity) {
    const QMutexLocker lock(&mutex_);
    const auto found = images_.find(identity);
    if (found == images_.end()) {
        return;
    }
    resident_bytes_ -= found->sizeInBytes();
    images_.erase(found);
    insertion_order_.removeAll(identity);
}

void ImagePreviewStore::clear() {
    const QMutexLocker lock(&mutex_);
    images_.clear();
    insertion_order_.clear();
    resident_bytes_ = 0;
}

void ImagePreviewStore::evictToBudget() {
    while (resident_bytes_ > maximumResidentBytes && insertion_order_.size() > 1) {
        const QString identity = insertion_order_.takeFirst();
        const auto found = images_.find(identity);
        if (found == images_.end()) {
            continue;
        }
        resident_bytes_ -= found->sizeInBytes();
        images_.erase(found);
    }
}

ImagePreviewProvider::ImagePreviewProvider(std::shared_ptr<ImagePreviewStore> store)
    : QQuickImageProvider(QQuickImageProvider::Image), store_(std::move(store)) {}

QImage
ImagePreviewProvider::requestImage(const QString& id, QSize* size, const QSize& requestedSize) {
    QImage image = store_->image(id);
    if (size != nullptr) {
        *size = image.size();
    }
    if (!image.isNull() && requestedSize.isValid()
        && (image.width() > requestedSize.width() || image.height() > requestedSize.height())) {
        image = image.scaled(requestedSize, Qt::KeepAspectRatio, Qt::SmoothTransformation);
    }
    return image;
}

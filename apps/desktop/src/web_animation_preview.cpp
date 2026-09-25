#include "web_animation_preview.hpp"

#include <QWebEngineUrlRequestInfo>

void WebAnimationPreviewProfile::OfflineRequests::interceptRequest(
    QWebEngineUrlRequestInfo& request
) {
    // Inline data and generated blob URLs are enough for a self-contained
    // animation. Everything else, including file and network URLs, is denied.
    const QString scheme = request.requestUrl().scheme().toLower();
    if (scheme != QStringLiteral("data") && scheme != QStringLiteral("blob")) {
        request.block(true);
    }
}

WebAnimationPreviewProfile::WebAnimationPreviewProfile() {
    profile_.setOffTheRecord(true);
    profile_.setHttpCacheType(QQuickWebEngineProfile::NoCache);
    profile_.setPersistentCookiesPolicy(QQuickWebEngineProfile::NoPersistentCookies);
    profile_.setPersistentPermissionsPolicy(
        QQuickWebEngineProfile::PersistentPermissionsPolicy::AskEveryTime
    );
    profile_.setUrlRequestInterceptor(&offline_requests_);
}

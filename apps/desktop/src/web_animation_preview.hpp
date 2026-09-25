#pragma once

#include <QQuickWebEngineProfile>
#include <QWebEngineUrlRequestInterceptor>

// The HTML preview is a separate, memory-only browser context. No project
// path or desktop file URL is ever passed into the rendered document.
class WebAnimationPreviewProfile final {
  public:
    WebAnimationPreviewProfile();

    QQuickWebEngineProfile* profile() {
        return &profile_;
    }

  private:
    class OfflineRequests final : public QWebEngineUrlRequestInterceptor {
      public:
        using QWebEngineUrlRequestInterceptor::QWebEngineUrlRequestInterceptor;
        void interceptRequest(QWebEngineUrlRequestInfo& request) override;
    };

    OfflineRequests offline_requests_;
    QQuickWebEngineProfile profile_;
};

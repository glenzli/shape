pragma ComponentBehavior: Bound

//! Ephemeral, offline playback of an accepted, self-contained HTML source.
//! The native profile denies file and network requests; closing this view
//! destroys its script context. The accepted source is never rewritten.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Shape.Desktop

Rectangle {
    id: preview
    objectName: "webAnimationPreview"
    required property string html
    required property var previewProfile
    readonly property bool fitsWebEngine: encodeURIComponent(html).length < 1500000
    property string loadError: ""

    radius: Theme.radiusMedium
    color: Theme.raised
    border.color: Theme.border
    clip: true

    function previewHtml(): string {
        // Applied only to the in-memory preview; export uses the accepted bytes.
        const policy = '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; script-src \'unsafe-inline\' blob: data:; style-src \'unsafe-inline\'; img-src data: blob:; media-src data: blob:; font-src data:; connect-src \'none\'; frame-src \'none\'; object-src \'none\'; base-uri \'none\'; form-action \'none\'">';
        return /<head(?:\s[^>]*)?>/i.test(html)
                ? html.replace(/<head(?:\s[^>]*)?>/i, "$&" + policy)
                : policy + html;
    }

    WebEngineView {
        id: browser
        objectName: "webAnimationPreviewBrowser"
        anchors.fill: parent
        anchors.margins: 1
        visible: preview.fitsWebEngine && preview.loadError.length === 0
        profile: preview.previewProfile
        backgroundColor: "white"
        settings.localContentCanAccessFileUrls: false
        settings.localContentCanAccessRemoteUrls: false
        settings.localStorageEnabled: false
        settings.javascriptCanAccessClipboard: false
        settings.javascriptCanPaste: false
        settings.javascriptCanOpenWindows: false
        settings.screenCaptureEnabled: false
        settings.navigateOnDropEnabled: false
        settings.pluginsEnabled: false
        settings.dnsPrefetchEnabled: false
        settings.unknownUrlSchemePolicy: WebEngineSettings.DisallowUnknownUrlSchemes

        onNavigationRequested: function(request) {
            const scheme = request.url.toString().split(":", 1)[0].toLowerCase();
            if (scheme !== "data" && scheme !== "blob" && scheme !== "about")
                request.reject();
        }
        onPermissionRequested: function(request) { request.deny(); }
        // No new-window handler means popup requests fail without a target.
        Component.onCompleted: {
            if (preview.fitsWebEngine)
                browser.loadHtml(preview.previewHtml(), "about:blank");
        }
        onLoadingChanged: function(loadRequest) {
            if (loadRequest.status === WebEngineView.LoadFailedStatus)
                preview.loadError = qsTr("This HTML could not be previewed.");
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 48, 420)
        visible: !preview.fitsWebEngine || preview.loadError.length > 0
        Text {
            Layout.fillWidth: true
            text: preview.fitsWebEngine ? preview.loadError
                                        : qsTr("This HTML is too large for the in-app preview. You can still inspect and export the original file.")
            color: Theme.muted
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }
}

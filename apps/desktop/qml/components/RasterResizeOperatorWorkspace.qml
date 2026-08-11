pragma ComponentBehavior: Bound

//! Authored-state workspace for the deterministic `image.resize` Operator.
//! Exact pixels stay in Rust; QML owns controls and bounded display geometry.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: workspace
    objectName: "rasterResizeOperatorWorkspace"

    property string operatorDraftId: ""
    property string artifactId: ""
    property url source
    property int sourceWidth: 0
    property int sourceHeight: 0
    property int targetWidth: 0
    property int targetHeight: 0
    property string aspectPolicyKey: "fit_within"
    property string resamplingKey: "lanczos3"

    property int pendingWidth: targetWidth > 0 ? targetWidth : Math.max(1, sourceWidth)
    property int pendingHeight: targetHeight > 0 ? targetHeight : Math.max(1, sourceHeight)
    property string pendingAspectPolicyKey: aspectPolicyKey.length > 0
                                                   ? aspectPolicyKey : "fit_within"
    property string pendingResamplingKey: resamplingKey.length > 0
                                               ? resamplingKey : "lanczos3"

    readonly property size resolvedSize: resolveOutputSize()
    readonly property bool hasExecutableResize: operatorDraftId.length > 0
                                                 && sourceWidth > 0 && sourceHeight > 0
                                                 && pendingWidth > 0 && pendingHeight > 0
                                                 && (resolvedSize.width !== sourceWidth
                                                     || resolvedSize.height !== sourceHeight)

    signal draftSaveRequested(string draftId, int targetWidth, int targetHeight,
                              string aspectPolicyKey, string resamplingKey)
    signal resizeRequested(string artifactId, string draftId,
                           int targetWidth, int targetHeight,
                           string aspectPolicyKey, string resamplingKey)

    function resolveOutputSize() : size {
        const width = Math.max(1, pendingWidth)
        const height = Math.max(1, pendingHeight)
        if (pendingAspectPolicyKey === "stretch" || sourceWidth <= 0 || sourceHeight <= 0) {
            return Qt.size(width, height)
        }
        const widthLimited = width * sourceHeight <= height * sourceWidth
        if (widthLimited) {
            return Qt.size(width, Math.max(1, Math.floor((sourceHeight * width
                                                        + sourceWidth / 2) / sourceWidth)))
        }
        return Qt.size(Math.max(1, Math.floor((sourceWidth * height
                                              + sourceHeight / 2) / sourceHeight)), height)
    }

    function synchronizeDraftConfiguration() : void {
        saveTimer.stop()
        pendingWidth = targetWidth > 0 ? targetWidth : Math.max(1, sourceWidth)
        pendingHeight = targetHeight > 0 ? targetHeight : Math.max(1, sourceHeight)
        pendingAspectPolicyKey = aspectPolicyKey.length > 0
                                 ? aspectPolicyKey : "fit_within"
        pendingResamplingKey = resamplingKey.length > 0 ? resamplingKey : "lanczos3"
        widthInput.value = pendingWidth
        heightInput.value = pendingHeight
    }

    function persistDraftConfiguration() : void {
        saveTimer.stop()
        if (operatorDraftId.length === 0) return
        if (pendingWidth === targetWidth && pendingHeight === targetHeight
                && pendingAspectPolicyKey === aspectPolicyKey
                && pendingResamplingKey === resamplingKey) {
            return
        }
        draftSaveRequested(operatorDraftId, pendingWidth, pendingHeight,
                           pendingAspectPolicyKey, pendingResamplingKey)
    }

    function requestResize() : void {
        saveTimer.stop()
        resizeRequested(artifactId, operatorDraftId, pendingWidth, pendingHeight,
                        pendingAspectPolicyKey, pendingResamplingKey)
    }

    onOperatorDraftIdChanged: Qt.callLater(workspace.synchronizeDraftConfiguration)
    onTargetWidthChanged: Qt.callLater(workspace.synchronizeDraftConfiguration)
    onTargetHeightChanged: Qt.callLater(workspace.synchronizeDraftConfiguration)
    onAspectPolicyKeyChanged: Qt.callLater(workspace.synchronizeDraftConfiguration)
    onResamplingKeyChanged: Qt.callLater(workspace.synchronizeDraftConfiguration)
    Component.onCompleted: synchronizeDraftConfiguration()
    Component.onDestruction: {
        if (saveTimer.running) persistDraftConfiguration()
    }

    Timer {
        id: saveTimer
        interval: 350
        repeat: false
        onTriggered: workspace.persistDraftConfiguration()
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 16

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 280
            radius: Theme.radiusLarge
            color: Theme.effectiveDark ? "#151719" : "#e7e9eb"
            border.color: Theme.border
            clip: true

            Image {
                anchors.fill: parent
                anchors.margins: 28
                source: workspace.source
                fillMode: Image.PreserveAspectFit
                asynchronous: true
                cache: false
            }

            Rectangle {
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: 14
                width: resultLabel.implicitWidth + 18
                height: 28
                radius: 14
                color: Theme.surface
                border.color: Theme.borderStrong

                Text {
                    id: resultLabel
                    anchors.centerIn: parent
                    text: qsTr("%1 × %2 px").arg(workspace.resolvedSize.width)
                                              .arg(workspace.resolvedSize.height)
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 286
            Layout.fillHeight: true
            radius: Theme.radiusLarge
            color: Theme.surface
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 14

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        text: qsTr("RESIZE IMAGE")
                        color: Theme.text
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Author portable output dimensions. Exact pixels are produced only when you create a Candidate.")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WordWrap
                        lineHeight: 1.25
                    }
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: 10
                    rowSpacing: 7

                    Text { text: qsTr("Width"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                    SpinBox {
                        id: widthInput
                        objectName: "resizeWidthInput"
                        Layout.fillWidth: true
                        from: 1
                        to: 32768
                        editable: true
                        value: workspace.pendingWidth
                        Accessible.name: qsTr("Resize width")
                        onValueModified: {
                            workspace.pendingWidth = value
                            saveTimer.restart()
                        }
                    }

                    Text { text: qsTr("Height"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                    SpinBox {
                        id: heightInput
                        objectName: "resizeHeightInput"
                        Layout.fillWidth: true
                        from: 1
                        to: 32768
                        editable: true
                        value: workspace.pendingHeight
                        Accessible.name: qsTr("Resize height")
                        onValueModified: {
                            workspace.pendingHeight = value
                            saveTimer.restart()
                        }
                    }
                }

                Text {
                    text: qsTr("Aspect")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    ShapeButton {
                        objectName: "resizeFitWithinButton"
                        Layout.fillWidth: true
                        text: qsTr("Fit within")
                        selected: workspace.pendingAspectPolicyKey === "fit_within"
                        onClicked: {
                            workspace.pendingAspectPolicyKey = "fit_within"
                            workspace.persistDraftConfiguration()
                        }
                    }

                    ShapeButton {
                        objectName: "resizeStretchButton"
                        Layout.fillWidth: true
                        text: qsTr("Stretch")
                        selected: workspace.pendingAspectPolicyKey === "stretch"
                        onClicked: {
                            workspace.pendingAspectPolicyKey = "stretch"
                            workspace.persistDraftConfiguration()
                        }
                    }
                }

                Text {
                    text: qsTr("Resampling")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    rowSpacing: 6
                    columnSpacing: 6

                    Repeater {
                        model: [
                            { "key": "lanczos3", "label": qsTr("Lanczos") },
                            { "key": "catmull_rom", "label": qsTr("Catmull–Rom") },
                            { "key": "triangle", "label": qsTr("Triangle") },
                            { "key": "nearest", "label": qsTr("Nearest") }
                        ]

                        ShapeButton {
                            required property var modelData
                            Layout.fillWidth: true
                            text: modelData.label
                            selected: workspace.pendingResamplingKey === modelData.key
                            onClicked: {
                                workspace.pendingResamplingKey = modelData.key
                                workspace.persistDraftConfiguration()
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Text {
                    Layout.fillWidth: true
                    visible: !workspace.hasExecutableResize
                    text: qsTr("Choose dimensions that change the accepted image.")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }

                ShapeButton {
                    objectName: "createResizeCandidateButton"
                    Layout.fillWidth: true
                    primary: true
                    enabled: workspace.hasExecutableResize
                    text: qsTr("Create resize Candidate")
                    iconSource: "qrc:/qt/qml/Shape/Desktop/icons/fit.svg"
                    Accessible.name: qsTr("Create resize Candidate")
                    onClicked: workspace.requestResize()
                }
            }
        }
    }
}

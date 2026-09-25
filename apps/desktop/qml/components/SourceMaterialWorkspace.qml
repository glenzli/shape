pragma ComponentBehavior: Bound

//! Content-first viewer for the exact immutable revision bound to one Source
//! node. It deliberately omits internal identities from the primary UI.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: workspace
    objectName: "sourceMaterialWorkspace"

    required property var nodeData
    required property string artifactId
    required property string revisionId
    required property var backend
    required property var audioPreview
    required property string audioOriginKey

    readonly property var outputPorts: nodeData.outputPorts || []
    readonly property string dataTypeKey: outputPorts.length > 0
                                                  ? outputPorts[0].dataTypeKey : ""
    readonly property bool isText: dataTypeKey === "text.document"
    readonly property bool isImage: dataTypeKey === "image.raster"
    readonly property bool isAudio: dataTypeKey === "audio.clip"
    readonly property bool isCode: /\.(js|mjs|html|css|json|svg)$/i.test(nodeData.artifactName || "")
    readonly property bool isHtml: isText && /\.html?$/i.test(nodeData.artifactName || "")
    readonly property string textPreview: nodeData.hasTextPreview === true
                                                   ? nodeData.textPreview : ""
    property string completeText: ""
    property bool previewMode: false

    Component.onCompleted: {
        if (isText && revisionId.length > 0 && backend !== null)
            completeText = backend.sourceRevisionText(revisionId)
        if (isAudio && revisionId.length > 0 && audioPreview !== null)
            audioPreview.loadPreview(artifactId, "", revisionId)
    }

    function materialTitle() : string {
        if (isText) return isCode ? qsTr("Original code") : qsTr("Original text")
        if (isImage) return qsTr("Original image")
        if (isAudio) return qsTr("Original audio")
        return qsTr("Source material")
    }

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Rectangle {
                Layout.preferredWidth: 42
                Layout.preferredHeight: 42
                radius: 12
                color: Theme.raised
                border.color: Theme.borderStrong

                ShapeIcon {
                    anchors.centerIn: parent
                    source: workspace.isAudio
                            ? "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                            : "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                    color: Theme.muted
                    size: 20
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("SOURCE MATERIAL")
                    color: Theme.muted
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.6
                }
                Text {
                    Layout.fillWidth: true
                    text: workspace.materialTitle()
                    color: Theme.text
                    font.pixelSize: 20
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }

            ShapeButton {
                objectName: "htmlAnimationPreviewButton"
                visible: workspace.isHtml && workspace.completeText.length > 0
                text: workspace.previewMode ? qsTr("View HTML source") : qsTr("Preview HTML animation")
                selected: workspace.previewMode
                onClicked: workspace.previewMode = !workspace.previewMode
            }

            Rectangle {
                Layout.preferredWidth: lockedLabel.implicitWidth + 22
                Layout.preferredHeight: 30
                radius: 15
                color: Theme.raised
                border.color: Theme.border

                Text {
                    id: lockedLabel
                    anchors.centerIn: parent
                    text: qsTr("LOCKED INPUT")
                    color: Theme.textSoft
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: workspace.isHtml
                  ? qsTr("Preview runs this self-contained HTML in an offline browser. Export preserves the accepted file exactly.")
                  : qsTr("This is the exact material connected to the workflow. Add an editing node after it to create a new version.")
            color: Theme.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }

        ShapeTextEditor {
            id: sourceText
            objectName: "sourceMaterialText"
            visible: workspace.isText && !workspace.previewMode
            Layout.fillWidth: true
            Layout.fillHeight: true
            readOnly: true
            textFormat: TextEdit.PlainText
            selectByMouse: true
            text: workspace.completeText.length > 0 ? workspace.completeText
                                                     : workspace.textPreview.length > 0
                                                       ? workspace.textPreview : qsTr("Empty text")
            color: Theme.text
            font.pixelSize: workspace.isCode ? 14 : 17
            font.family: workspace.isCode ? "monospace" : ""
            wrapMode: TextEdit.Wrap
            leftPadding: 18
            rightPadding: 18
            topPadding: 16
            bottomPadding: 16
            background: Rectangle {
                radius: Theme.radiusMedium
                color: Theme.raised
                border.color: Theme.border
            }
        }

        Loader {
            visible: workspace.isHtml && workspace.previewMode
            active: visible
            Layout.fillWidth: true
            Layout.fillHeight: true
            sourceComponent: Component {
                WebAnimationPreview {
                    html: workspace.completeText
                    previewProfile: webAnimationPreviewProfile
                }
            }
        }

        Rectangle {
            visible: !workspace.isText
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 48, 420)
                spacing: 10

                ShapeIcon {
                    Layout.alignment: Qt.AlignHCenter
                    source: workspace.isAudio
                            ? "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                            : "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                    color: Theme.muted
                    size: 34
                }
                Text {
                    Layout.fillWidth: true
                    text: workspace.isImage ? qsTr("Image material")
                                            : workspace.isAudio ? qsTr("Audio material")
                                                                : qsTr("Creative material")
                    color: Theme.text
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    horizontalAlignment: Text.AlignHCenter
                }
                Text {
                    Layout.fillWidth: true
                    text: workspace.isAudio
                          && workspace.audioOriginKey === "imported_unverified"
                          ? qsTr("Imported audio is preserved as selected. Its recording or generation origin has not been verified.")
                          : qsTr("This material is preserved exactly as the node input.")
                    color: Theme.muted
                    font.pixelSize: 10
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                }
                ShapeButton {
                    visible: workspace.isAudio
                    Layout.alignment: Qt.AlignHCenter
                    text: workspace.audioPreview !== null && workspace.audioPreview.playing
                          ? qsTr("Pause preview") : qsTr("Play preview")
                    enabled: workspace.audioPreview !== null && workspace.audioPreview.hasAudio
                    onClicked: workspace.audioPreview.togglePlayback()
                }
            }
        }

        Text {
            visible: workspace.nodeData.textPreviewTruncated === true
                     && workspace.completeText.length === 0
            Layout.fillWidth: true
            text: qsTr("Preview shortened · the complete source remains preserved")
            color: Theme.muted
            font.pixelSize: 9
            horizontalAlignment: Text.AlignRight
        }
    }
}

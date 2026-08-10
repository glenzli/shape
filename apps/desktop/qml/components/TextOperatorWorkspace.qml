//! Text-node presentation for accepted input, transient Candidate preview,
//! and the immutable Revision boundary. Routing remains with the host.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: workspace
    objectName: "textOperatorWorkspace"

    property string acceptedText: ""
    property string candidateText: ""
    property bool hasAcceptedRevision: false
    property bool hasTextPreview: false
    property bool textPreviewTruncated: false
    property bool hasCandidate: false
    property bool compareMode: false

    signal speechWorkspaceRequested()

    readonly property bool reviewingCandidate: compareMode && hasCandidate

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: workspace.reviewingCandidate ? Theme.accent : Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 10

                Text {
                    text: qsTr("TEXT OPERATOR WORKSPACE")
                    color: Theme.text
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }

                Rectangle {
                    Layout.preferredWidth: inputType.implicitWidth + 16
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.surface
                    border.color: Theme.border

                    Text {
                        id: inputType
                        anchors.centerIn: parent
                        text: "text.document"
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }

                Text {
                    text: "→"
                    color: Theme.muted
                    font.pixelSize: 12
                }

                Rectangle {
                    Layout.preferredWidth: operatorType.implicitWidth + 16
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.accentSoft
                    border.color: Theme.accent

                    Text {
                        id: operatorType
                        anchors.centerIn: parent
                        text: workspace.reviewingCandidate ? qsTr("CANDIDATE")
                                                           : qsTr("ACCEPTED")
                        color: Theme.accent
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                    }
                }

                Text {
                    text: "→"
                    color: Theme.muted
                    font.pixelSize: 12
                }

                Rectangle {
                    Layout.preferredWidth: outputType.implicitWidth + 16
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.surface
                    border.color: Theme.border

                    Text {
                        id: outputType
                        anchors.centerIn: parent
                        text: "text.document"
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: workspace.reviewingCandidate
                          ? qsTr("Transient preview") : qsTr("Immutable revision")
                    color: workspace.reviewingCandidate ? Theme.accent : Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Rectangle {
                visible: !workspace.reviewingCandidate
                anchors.centerIn: parent
                width: Math.min(parent.width - 48, 680)
                height: Math.min(parent.height - 30, 360)
                radius: Theme.radiusLarge
                color: Theme.raised
                border.color: Theme.border

                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    width: 2
                    height: Math.min(parent.height - 56, 96)
                    radius: 1
                    color: workspace.hasAcceptedRevision ? Theme.accent : Theme.borderStrong
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 42
                    anchors.rightMargin: 38
                    anchors.topMargin: 30
                    anchors.bottomMargin: 30
                    spacing: 16

                    Item { Layout.fillHeight: true }

                    Text {
                        Layout.fillWidth: true
                        text: workspace.hasTextPreview
                              ? workspace.acceptedText : qsTr("No accepted text content")
                        color: Theme.text
                        font.pixelSize: workspace.hasTextPreview ? 28 : 19
                        font.weight: Font.Light
                        lineHeight: 1.25
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Text {
                        Layout.fillWidth: true
                        text: workspace.hasAcceptedRevision
                              ? qsTr("This verified text.document is the immutable input to the next Text Operator.")
                              : qsTr("This artifact has no accepted revision yet.")
                        color: Theme.muted
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        font.pixelSize: 11
                        lineHeight: 1.3
                    }

                    Text {
                        visible: workspace.textPreviewTruncated
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Preview truncated")
                        color: Theme.accent
                        font.pixelSize: 10
                    }

                    Item { Layout.fillHeight: true }
                }
            }

            TextCompareWorkspace {
                anchors.fill: parent
                visible: workspace.reviewingCandidate
                acceptedText: workspace.acceptedText
                candidateText: workspace.candidateText
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Text {
                Layout.fillWidth: true
                text: workspace.reviewingCandidate
                      ? qsTr("Preview is transient. Accept explicitly to create a new immutable Revision.")
                      : qsTr("text.edit calibrates deterministically; text.transform keeps AI execution behind creative intent.")
                color: Theme.muted
                font.pixelSize: 10
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }

            ShapeButton {
                objectName: "openSpeechWorkspaceButton"
                visible: workspace.hasAcceptedRevision && workspace.hasTextPreview
                text: qsTr("Create narration →")
                onClicked: workspace.speechWorkspaceRequested()
            }
        }
    }
}

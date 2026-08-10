import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: workspace

    property string artifactName: ""
    property string artifactKind: ""
    property string artifactText: ""
    property bool hasAcceptedRevision: false
    property bool hasTextPreview: false
    property bool textPreviewTruncated: false
    property bool hasCandidate: false
    property string candidateText: ""
    property bool compareMode: false

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 54
            Layout.leftMargin: 18
            Layout.rightMargin: 16
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    Layout.fillWidth: true
                    text: workspace.artifactName.length > 0
                          ? workspace.artifactName : qsTr("Current artifact")
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: workspace.artifactKind.length > 0
                          ? workspace.artifactKind : qsTr("Creative workspace")
                    color: Theme.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }

            Rectangle {
                Layout.preferredWidth: stateLabel.implicitWidth + 20
                Layout.preferredHeight: 26
                radius: 13
                color: workspace.compareMode && workspace.hasCandidate
                       ? Theme.accentSoft
                       : workspace.hasAcceptedRevision
                       ? (Theme.effectiveDark ? "#203126" : "#e2f0e4") : Theme.raised
                border.color: workspace.compareMode && workspace.hasCandidate
                              ? Theme.accent
                              : workspace.hasAcceptedRevision
                              ? (Theme.effectiveDark ? "#35553e" : "#bed8c3") : Theme.border

                Text {
                    id: stateLabel
                    anchors.centerIn: parent
                    text: workspace.compareMode && workspace.hasCandidate
                          ? qsTr("Comparing candidate")
                          : workspace.hasAcceptedRevision ? qsTr("Accepted head")
                                                          : qsTr("No accepted head")
                    color: workspace.compareMode && workspace.hasCandidate
                           ? Theme.accent
                           : workspace.hasAcceptedRevision ? Theme.success : Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Rectangle {
                visible: !workspace.compareMode || !workspace.hasCandidate
                anchors.centerIn: parent
                width: Math.min(parent.width - 72, 680)
                height: Math.min(parent.height - 54, 360)
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
                              ? workspace.artifactText : qsTr("No accepted text content")
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
                              ? qsTr("Loaded from the verified accepted revision. Executor output stays a candidate until you accept it.")
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
                anchors.margins: 24
                visible: workspace.compareMode && workspace.hasCandidate
                acceptedText: workspace.artifactText
                candidateText: workspace.candidateText
            }
        }
    }
}

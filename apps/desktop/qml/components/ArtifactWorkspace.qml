import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Frame {
    id: workspace
    property string artifactName: ""
    property string artifactKind: ""
    property string artifactText: ""
    property bool hasAcceptedRevision: false
    property bool hasTextPreview: false
    property bool textPreviewTruncated: false
    padding: 0
    background: Rectangle {
        color: Theme.surface
        radius: Theme.radiusLarge
        border.color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 14
            Label {
                text: workspace.artifactName.length > 0
                      ? workspace.artifactName
                      : qsTr("CURRENT ARTIFACT")
                color: Theme.muted
                font.pixelSize: 11
                font.bold: true
            }
            Item { Layout.fillWidth: true }
            Label {
                text: workspace.hasAcceptedRevision ? qsTr("Accepted head") : qsTr("No accepted head")
                color: workspace.hasAcceptedRevision ? Theme.success : Theme.muted
                font.pixelSize: 12
            }
        }

        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 80, 620)
                spacing: 18
                Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: workspace.hasTextPreview
                          ? workspace.artifactText
                          : qsTr("No accepted text content")
                    color: Theme.text
                    font.pixelSize: workspace.hasTextPreview ? 30 : 20
                    font.weight: Font.Light
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                    Layout.fillWidth: true
                }
                Label {
                    Layout.fillWidth: true
                    text: workspace.hasAcceptedRevision
                          ? qsTr("Loaded from the verified accepted revision. Executor output remains a candidate until you explicitly accept it.")
                          : qsTr("This artifact has no accepted revision yet.")
                    color: Theme.muted
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    font.pixelSize: 13
                }
                Label {
                    visible: workspace.textPreviewTruncated
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("Preview truncated")
                    color: Theme.accent
                    font.pixelSize: 11
                }
            }
        }
    }
}

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop
import "components"

ApplicationWindow {
    id: window
    width: 1440
    height: 900
    minimumWidth: 1024
    minimumHeight: 700
    visible: true
    title: qsTr("Shape — Untitled Project")
    color: Theme.background

    header: ToolBar {
        implicitHeight: 58
        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 14

            Label {
                text: qsTr("SHAPE")
                color: Theme.accent
                font.pixelSize: 17
                font.bold: true
                font.letterSpacing: 2
            }
            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 24
                color: Theme.border
            }
            Label {
                text: qsTr("Untitled Project")
                color: Theme.text
                font.pixelSize: 15
            }
            Label {
                text: qsTr("Saved locally")
                color: Theme.muted
                font.pixelSize: 12
            }
            Item { Layout.fillWidth: true }
            Button {
                text: qsTr("Compare")
                enabled: false
                ToolTip.text: qsTr("Compare becomes available when variants exist")
                ToolTip.visible: hovered
            }
            Button {
                text: qsTr("Export")
                enabled: false
                ToolTip.text: qsTr("Export is not connected in the foundation build")
                ToolTip.visible: hovered
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Frame {
            Layout.preferredWidth: 210
            Layout.fillHeight: true
            padding: 12
            background: Rectangle {
                color: Theme.surface
                radius: Theme.radiusLarge
                border.color: Theme.border
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: 10
                Label {
                    text: qsTr("ARTIFACTS")
                    color: Theme.muted
                    font.pixelSize: 11
                    font.bold: true
                }
                ItemDelegate {
                    Layout.fillWidth: true
                    highlighted: true
                    text: qsTr("Story")
                    icon.name: "text-x-generic"
                }
                Item { Layout.fillHeight: true }
                Label {
                    Layout.fillWidth: true
                    text: qsTr("A project contains stable creative artifacts. Accepted revisions remain immutable.")
                    color: Theme.muted
                    wrapMode: Text.WordWrap
                    font.pixelSize: 12
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            ArtifactWorkspace {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            IntentPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: 178
            }
        }

        ColumnLayout {
            Layout.preferredWidth: 310
            Layout.fillHeight: true
            spacing: 10

            VariantsPanel {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
            SemanticHistory {
                Layout.fillWidth: true
                Layout.preferredHeight: 260
            }
        }
    }
}

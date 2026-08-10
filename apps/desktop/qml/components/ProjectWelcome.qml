pragma ComponentBehavior: Bound

//! Empty-launch entry for a real project session.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: welcome
    objectName: "projectWelcome"

    property string errorMessage: ""
    signal newProjectRequested()
    signal openProjectRequested()

    color: Theme.background

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(560, parent.width - 56)
        implicitHeight: welcomeContent.implicitHeight + 56
        radius: Theme.radiusLarge
        color: Theme.surface
        border.color: Theme.border

        ColumnLayout {
            id: welcomeContent
            anchors.fill: parent
            anchors.margins: 28
            spacing: 14

            Rectangle {
                Layout.preferredWidth: 46
                Layout.preferredHeight: 46
                Layout.alignment: Qt.AlignHCenter
                radius: 14
                color: Theme.accentSoft
                border.color: Theme.accent

                Text {
                    anchors.centerIn: parent
                    text: "◇"
                    color: Theme.accent
                    font.pixelSize: 23
                    font.weight: Font.DemiBold
                }
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Start shaping")
                color: Theme.text
                font.pixelSize: 22
                font.weight: Font.DemiBold
                horizontalAlignment: Text.AlignHCenter
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Create a project, add a Scene source, then place an Operator on the graph.")
                color: Theme.muted
                font.pixelSize: 12
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                lineHeight: 1.35
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 6
                spacing: 10

                ShapeButton {
                    objectName: "newProjectButton"
                    text: qsTr("New project")
                    primary: true
                    onClicked: welcome.newProjectRequested()
                }

                ShapeButton {
                    objectName: "openProjectButton"
                    text: qsTr("Open project…")
                    onClicked: welcome.openProjectRequested()
                }
            }

            Text {
                visible: welcome.errorMessage.length > 0
                Layout.fillWidth: true
                text: welcome.errorMessage
                color: Theme.danger
                font.pixelSize: 10
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }
        }
    }
}

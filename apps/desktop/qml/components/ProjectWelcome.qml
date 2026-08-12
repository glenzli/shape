pragma ComponentBehavior: Bound

//! Empty-launch entry for a real project session.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: welcome
    objectName: "projectWelcome"

    property string errorMessage: ""
    signal newProjectRequested()
    signal openProjectRequested()

    color: Theme.canvas

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(620, parent.width - 64)
        implicitHeight: welcomeContent.implicitHeight + 64
        radius: Theme.cardRadius
        color: Theme.panelRaised
        border.color: Theme.border

        ColumnLayout {
            id: welcomeContent
            anchors.fill: parent
            anchors.margins: 32
            spacing: 16

            Rectangle {
                Layout.preferredWidth: 52
                Layout.preferredHeight: 52
                Layout.alignment: Qt.AlignHCenter
                radius: 15
                color: Theme.accentSurfaceQuiet
                border.color: Theme.accentBorder

                ShapeIcon {
                    anchors.centerIn: parent
                    source: "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                    size: 22
                    color: Theme.accent
                }
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("What do you want to make?")
                color: Theme.text
                font.pixelSize: 24
                font.weight: Font.DemiBold
                horizontalAlignment: Text.AlignHCenter
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Start a project, then choose a creative goal. Shape will build the workflow for you and keep every adopted version safe.")
                color: Theme.muted
                font.pixelSize: 13
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
                    text: qsTr("Start a new project")
                    iconSource: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
                    primary: true
                    onClicked: welcome.newProjectRequested()
                }

                ShapeButton {
                    objectName: "openProjectButton"
                    text: qsTr("Continue a project…")
                    iconSource: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                    quiet: false
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

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
    property var recentProjects: []
    signal newProjectRequested()
    signal openProjectRequested()
    signal recentProjectRequested(url projectUrl)

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
                text: qsTr("Your projects")
                color: Theme.text
                font.pixelSize: 24
                font.weight: Font.DemiBold
                horizontalAlignment: Text.AlignHCenter
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Create a project or continue where you left off.")
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

            ColumnLayout {
                visible: welcome.recentProjects.length > 0
                Layout.fillWidth: true
                Layout.topMargin: 8
                spacing: 6

                Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }
                Label {
                    text: qsTr("Recent projects")
                    color: Theme.textSoft
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    Layout.topMargin: 8
                }
                Repeater {
                    model: welcome.recentProjects.slice(0, 5)
                    delegate: ItemDelegate {
                        required property var modelData
                        required property int index
                        objectName: "recentProjectEntry-" + index
                        Layout.fillWidth: true
                        implicitHeight: 52
                        enabled: modelData.available
                        onClicked: welcome.recentProjectRequested(modelData.url)
                        background: Rectangle {
                            radius: Theme.controlRadius
                            color: parent.hovered ? Theme.buttonHoverSurface : Theme.surface
                            border.color: Theme.border
                        }
                        contentItem: ColumnLayout {
                            spacing: 2
                            Label {
                                Layout.fillWidth: true
                                text: modelData.name + (modelData.available ? "" : " · " + qsTr("Unavailable"))
                                color: modelData.available ? Theme.text : Theme.disabled
                                font.weight: Font.DemiBold
                                elide: Text.ElideMiddle
                            }
                            Label {
                                Layout.fillWidth: true
                                text: modelData.path
                                color: Theme.muted
                                font.pixelSize: 11
                                elide: Text.ElideMiddle
                            }
                        }
                    }
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

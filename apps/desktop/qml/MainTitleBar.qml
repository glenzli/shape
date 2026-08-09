//! Compact one-row window chrome aligned with Shadow's 44 px title bar.
//! Project identity remains textual; universal actions use symbols and tips.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Shape.Desktop
import "components"

ToolBar {
    id: titleBar

    required property var hostWindow
    required property bool projectOpen
    required property string projectName

    signal compareRequested()
    signal exportRequested()
    signal settingsRequested()

    objectName: "titleToolBar"
    Accessible.name: qsTr("Shape toolbar")
    implicitHeight: Theme.toolbarHeight
    topPadding: 0
    bottomPadding: 0
    leftPadding: Math.max(
        SafeArea.margins.left,
        Qt.platform.os === "osx" && hostWindow.visibility !== Window.FullScreen ? 96 : 16
    )
    rightPadding: Math.max(
        SafeArea.margins.right,
        Qt.platform.os === "windows" ? 152 : 16
    )

    background: Rectangle {
        color: Theme.chrome

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }
    }

    contentItem: Item {
        Item {
            anchors.fill: parent

            DragHandler {
                target: null
                acceptedButtons: Qt.LeftButton
                onActiveChanged: if (active) titleBar.hostWindow.startSystemMove()
            }
        }

        RowLayout {
            anchors.fill: parent
            spacing: 10

            Text {
                text: "SHAPE"
                color: Theme.text
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.letterSpacing: 2.5
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 18
                color: Theme.border
            }

            Text {
                Layout.fillWidth: true
                text: titleBar.projectOpen ? titleBar.projectName : qsTr("No project open")
                color: Theme.textSoft
                font.pixelSize: 12
                font.weight: Font.Medium
                elide: Text.ElideRight
            }

            Item {
                visible: titleBar.projectOpen
                Layout.preferredWidth: 26
                Layout.preferredHeight: 26

                HoverHandler { id: verifiedHover }

                ShapeIcon {
                    anchors.centerIn: parent
                    source: "qrc:/qt/qml/Shape/Desktop/icons/verified.svg"
                    size: 16
                    color: Theme.success
                }

                ToolTip {
                    id: verifiedToolTip
                    visible: verifiedHover.hovered
                    delay: 450
                    text: qsTr("Local · verified")
                    y: parent.height + 6

                    contentItem: Text {
                        text: verifiedToolTip.text
                        color: Theme.text
                        font.pixelSize: Theme.fontMeta
                    }

                    background: Rectangle {
                        radius: Theme.radiusSmall
                        color: Theme.raised
                        border.color: Theme.borderStrong
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 18
                color: Theme.border
            }

            ShapeIconButton {
                source: "qrc:/qt/qml/Shape/Desktop/icons/compare.svg"
                toolTipText: qsTr("Compare")
                accessibleName: toolTipText
                enabled: false
                onClicked: titleBar.compareRequested()
            }

            ShapeIconButton {
                source: "qrc:/qt/qml/Shape/Desktop/icons/export.svg"
                toolTipText: qsTr("Export")
                accessibleName: toolTipText
                enabled: false
                onClicked: titleBar.exportRequested()
            }

            ShapeIconButton {
                source: "qrc:/qt/qml/Shape/Desktop/icons/settings.svg"
                toolTipText: qsTr("Settings")
                accessibleName: qsTr("Open settings")
                onClicked: titleBar.settingsRequested()
            }
        }
    }
}

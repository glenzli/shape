pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Dialog {
    id: dialog
    objectName: "createSceneTypeDialog"

    signal textSceneRequested()
    signal aiImageSceneRequested()
    signal importImageRequested()

    function openForCreation() : void { open() }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(640, parent.width - 48)
    modal: true
    title: qsTr("What do you want to make?")
    closePolicy: Popup.CloseOnEscape

    contentItem: ColumnLayout {
        spacing: 12

        Text {
            Layout.fillWidth: true
            text: qsTr("Choose a starting point. Shape creates the work and its node flow automatically; you can inspect or extend it whenever you need.")
            color: Theme.muted
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }

        ItemDelegate {
            id: textChoice
            objectName: "createTextSceneTypeButton"
            Layout.fillWidth: true
            implicitHeight: 76
            leftPadding: 14
            rightPadding: 14
            Accessible.name: qsTr("Write or revise text")
            onClicked: {
                dialog.close()
                dialog.textSceneRequested()
            }

            background: Rectangle {
                radius: Theme.radiusMedium
                color: textChoice.hovered ? Theme.raisedHover : Theme.raised
                border.color: textChoice.hovered ? Theme.accent : Theme.border
            }

            contentItem: RowLayout {
                spacing: 12

                Rectangle {
                    Layout.preferredWidth: 42
                    Layout.preferredHeight: 42
                    radius: 11
                    color: Theme.accentSoft

                    ShapeIcon {
                        anchors.centerIn: parent
                        source: "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
                        size: 20
                        color: Theme.accent
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: qsTr("Write or revise text")
                        color: Theme.text
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Begin with your own words, then edit, rewrite, or turn them into speech.")
                        color: Theme.muted
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }
                }

                Text { text: "›"; color: Theme.muted; font.pixelSize: 18 }
            }
        }

        ItemDelegate {
            id: aiImageChoice
            objectName: "createAiImageSceneTypeButton"
            Layout.fillWidth: true
            implicitHeight: 76
            leftPadding: 14
            rightPadding: 14
            Accessible.name: qsTr("Generate an image with AI")
            onClicked: {
                dialog.close()
                dialog.aiImageSceneRequested()
            }

            background: Rectangle {
                radius: Theme.radiusMedium
                color: aiImageChoice.hovered ? Theme.accentSoft : Theme.raised
                border.color: aiImageChoice.hovered ? Theme.accent : Theme.border
            }

            contentItem: RowLayout {
                spacing: 12

                Rectangle {
                    Layout.preferredWidth: 42
                    Layout.preferredHeight: 42
                    radius: 11
                    color: Theme.accentSoft

                    ShapeIcon {
                        anchors.centerIn: parent
                        source: "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                        size: 20
                        color: Theme.accent
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: qsTr("Generate an image with AI")
                        color: Theme.text
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Describe the result you want and compare generated versions before choosing one.")
                        color: Theme.muted
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }
                }

                Text { text: "›"; color: Theme.muted; font.pixelSize: 18 }
            }
        }

        ItemDelegate {
            id: importImageChoice
            objectName: "importImageStartButton"
            Layout.fillWidth: true
            implicitHeight: 76
            leftPadding: 14
            rightPadding: 14
            Accessible.name: qsTr("Import an image")
            onClicked: {
                dialog.close()
                dialog.importImageRequested()
            }

            background: Rectangle {
                radius: Theme.radiusMedium
                color: importImageChoice.hovered ? Theme.raisedHover : Theme.raised
                border.color: importImageChoice.hovered ? Theme.accent : Theme.border
            }

            contentItem: RowLayout {
                spacing: 12

                Rectangle {
                    Layout.preferredWidth: 42
                    Layout.preferredHeight: 42
                    radius: 11
                    color: Theme.raised
                    border.color: Theme.border

                    ShapeIcon {
                        anchors.centerIn: parent
                        source: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                        size: 20
                        color: Theme.textSoft
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: qsTr("Import an image")
                        color: Theme.text
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Use an existing image as the starting point for cropping, resizing, or future Shadow edits.")
                        color: Theme.muted
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }
                }

                Text { text: "›"; color: Theme.muted; font.pixelSize: 18 }
            }
        }
    }

    footer: DialogButtonBox {
        Button {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
        }
        onRejected: dialog.close()
    }
}

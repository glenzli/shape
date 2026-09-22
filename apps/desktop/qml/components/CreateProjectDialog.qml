pragma ComponentBehavior: Bound

//! Project name and local bundle-location admission.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import Shape.Desktop

ShapeDialog {
    id: dialog
    objectName: "createProjectDialog"

    required property DesktopBackend backend
    property url selectedFolder: ""
    signal projectCreated()

    function openForCreation() : void {
        projectNameField.text = qsTr("Untitled Project")
        selectedFolder = ""
        open()
        projectNameField.forceActiveFocus()
        projectNameField.selectAll()
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(520, parent.width - 48)
    modal: true
    title: qsTr("New Shape project")
    closePolicy: Popup.CloseOnEscape

    FolderDialog {
        id: folderPicker
        title: qsTr("Choose where to create the project")
        onAccepted: dialog.selectedFolder = selectedFolder
    }

    contentItem: ColumnLayout {
        spacing: 12

        Text {
            Layout.fillWidth: true
            text: qsTr("PROJECT NAME")
            color: Theme.textSoft
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.6
        }

        ShapeTextField {
            id: projectNameField
            objectName: "projectNameField"
            Layout.fillWidth: true
            placeholderText: qsTr("Project name")
            selectByMouse: true
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("LOCATION")
            color: Theme.textSoft
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.6
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ShapeTextField {
                Layout.fillWidth: true
                readOnly: true
                text: dialog.selectedFolder.toString().replace("file://", "")
                placeholderText: qsTr("Choose a local folder")
            }

            ShapeButton {
                objectName: "chooseProjectLocationButton"
                text: qsTr("Choose…")
                onClicked: folderPicker.open()
            }
        }

        Text {
            visible: dialog.backend.lastError.length > 0
            Layout.fillWidth: true
            text: dialog.backend.lastError
            color: Theme.danger
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }
    }

    footer: Pane {
        topPadding: 0
        leftPadding: 20
        rightPadding: 20
        bottomPadding: 20
        background: Item {}
        contentItem: RowLayout {
            spacing: 8
            Item { Layout.fillWidth: true }
            ShapeButton {
                text: qsTr("Cancel")
                onClicked: dialog.close()
            }

            ShapeButton {
                objectName: "confirmCreateProjectButton"
                text: qsTr("Create project")
                highlighted: true
                enabled: projectNameField.text.trim().length > 0
                         && dialog.selectedFolder.toString().length > 0
                onClicked: {
                    if (dialog.backend.createProject(dialog.selectedFolder, projectNameField.text)) {
                        dialog.close()
                        dialog.projectCreated()
                    }
                }
            }
        }
    }
}

pragma ComponentBehavior: Bound

//! Human-facing text-work creation over one atomic accepted origin.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

ShapeDialog {
    id: dialog
    objectName: "createTextSceneDialog"

    required property DesktopBackend backend
    signal sceneCreated()

    function openForCreation() : void {
        sceneNameField.text = qsTr("Untitled text")
        initialTextArea.text = ""
        open()
        sceneNameField.forceActiveFocus()
        sceneNameField.selectAll()
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(560, parent.width - 48)
    modal: true
    title: qsTr("Start with text")
    closePolicy: Popup.CloseOnEscape

    contentItem: ColumnLayout {
        spacing: 10

        ShapeTextField {
            id: sceneNameField
            objectName: "sceneNameField"
            Layout.fillWidth: true
            placeholderText: qsTr("Name this work")
            selectByMouse: true
        }

        ShapeTextEditor {
            id: initialTextArea
            objectName: "initialSceneTextField"
            Layout.fillWidth: true
            Layout.preferredHeight: 150
            placeholderText: qsTr("Start writing…")
            wrapMode: TextEdit.Wrap
            selectByMouse: true
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Shape saves this as the starting version and opens a writing workspace. You can add AI rewriting or speech later.")
            color: Theme.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
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

    footer: DialogButtonBox {
        ShapeButton {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
        }

        ShapeButton {
            objectName: "confirmCreateTextSceneButton"
            text: qsTr("Start writing")
            highlighted: true
            enabled: sceneNameField.text.trim().length > 0
                     && initialTextArea.text.trim().length > 0
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
        }

        onAccepted: {
            if (dialog.backend.createTextScene(sceneNameField.text, initialTextArea.text)) {
                dialog.close()
                dialog.sceneCreated()
            }
        }
        onRejected: dialog.close()
    }
}

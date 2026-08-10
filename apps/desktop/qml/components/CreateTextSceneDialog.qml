pragma ComponentBehavior: Bound

//! First compatibility Scene source creation over one atomic text origin.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Dialog {
    id: dialog
    objectName: "createTextSceneDialog"

    required property DesktopBackend backend
    signal sceneCreated()

    function openForCreation() : void {
        sceneNameField.text = qsTr("Opening")
        initialTextArea.text = ""
        open()
        sceneNameField.forceActiveFocus()
        sceneNameField.selectAll()
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(560, parent.width - 48)
    modal: true
    title: qsTr("Create text Scene")
    closePolicy: Popup.CloseOnEscape

    contentItem: ColumnLayout {
        spacing: 10

        TextField {
            id: sceneNameField
            objectName: "sceneNameField"
            Layout.fillWidth: true
            placeholderText: qsTr("Scene name")
            selectByMouse: true
        }

        TextArea {
            id: initialTextArea
            objectName: "initialSceneTextField"
            Layout.fillWidth: true
            Layout.preferredHeight: 150
            placeholderText: qsTr("Enter the first accepted text source…")
            wrapMode: TextEdit.Wrap
            selectByMouse: true
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("This creates a Source and Main Output. Add an Operator from the Scene Graph next.")
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
        Button {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
        }

        Button {
            objectName: "confirmCreateTextSceneButton"
            text: qsTr("Create Scene")
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

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

ShapeDialog {
    id: dialog
    objectName: "createAiImageSceneDialog"

    required property DesktopBackend backend
    signal sceneCreated()

    readonly property var canvasPresets: [
        { "label": qsTr("Square · 1:1"), "width": 1024, "height": 1024 },
        { "label": qsTr("Landscape · 3:2"), "width": 1536, "height": 1024 },
        { "label": qsTr("Portrait · 2:3"), "width": 1024, "height": 1536 }
    ]

    function openForCreation() : void {
        sceneNameField.text = qsTr("Untitled image")
        instructionArea.text = ""
        canvasPreset.currentIndex = 0
        open()
        sceneNameField.forceActiveFocus()
        sceneNameField.selectAll()
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(600, parent.width - 48)
    modal: true
    title: qsTr("Create an image")
    closePolicy: Popup.CloseOnEscape

    contentItem: ColumnLayout {
        spacing: 10

        ShapeTextField {
            id: sceneNameField
            objectName: "aiImageSceneNameField"
            Layout.fillWidth: true
            placeholderText: qsTr("Name this image")
            selectByMouse: true
        }

        Label {
            Layout.fillWidth: true
            text: qsTr("Codex Luna · image generation through Infer")
            color: Theme.accent
            font.pixelSize: 12
            wrapMode: Text.WordWrap
        }

        ShapeTextEditor {
            id: instructionArea
            objectName: "aiImageInstructionField"
            Layout.fillWidth: true
            Layout.preferredHeight: 150
            placeholderText: qsTr("Describe the image you want to create…")
            wrapMode: TextEdit.Wrap
            selectByMouse: true
        }

        ShapeComboBox {
            id: canvasPreset
            objectName: "aiImageCanvasPreset"
            Layout.fillWidth: true
            model: dialog.canvasPresets
            textRole: "label"
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Your prompt stays with this work. Generated versions remain optional until you choose one to use.")
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
            objectName: "confirmCreateAiImageSceneButton"
            text: qsTr("Open image studio")
            highlighted: true
            enabled: sceneNameField.text.trim().length > 0
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
        }

        onAccepted: {
            const preset = dialog.canvasPresets[canvasPreset.currentIndex]
            if (dialog.backend.createAiImageScene(
                    sceneNameField.text, instructionArea.text,
                    preset.width, preset.height)) {
                dialog.close()
                dialog.sceneCreated()
            }
        }
        onRejected: dialog.close()
    }
}

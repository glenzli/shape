pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

ShapeDialog {
    id: dialog
    objectName: "createSceneTypeDialog"
    signal textAuthoringRequested(string preset)
    signal aiImageSceneRequested()
    signal soundSceneRequested()
    signal importMaterialRequested()
    signal pasteMaterialRequested()
    function openForCreation() : void { open() }
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(740, parent.width - 48)
    height: Math.min(570, parent.height - 48)
    modal: true
    title: qsTr("Create material")
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    contentItem: ScrollView {
        id: scroll
        objectName: "createTaskScroll"
        contentWidth: availableWidth
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
            width: scroll.availableWidth
            spacing: 16
            Label {
                Layout.fillWidth: true
                text: qsTr("Write, generate or bring in a source. Review each result before using it.")
                color: Theme.muted
                wrapMode: Text.WordWrap
            }
            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 14
                rowSpacing: 14
                Repeater {
                    model: [
                        {key: "text", title: qsTr("Writing"), detail: qsTr("Start with an idea, edit existing text, or write on your own."), action: qsTr("Start writing"), name: "createTextSceneTypeButton", card: "createFreeWritingCard"},
                        {key: "image", title: qsTr("Images"), detail: qsTr("Describe an image and choose from the generated results."), action: qsTr("Generate an image"), name: "createAiImageSceneTypeButton", card: "createImageCard"},
                        {key: "sound", title: qsTr("Sound"), detail: qsTr("Generate a sound effect or a short piece of music."), action: qsTr("Create sound"), name: "createSoundSceneTypeButton", card: "createSoundCard"},
                        {key: "import", title: qsTr("Your materials"), detail: qsTr("Import an image, text, code or audio file to keep working with it."), action: qsTr("Import material…"), name: "importMaterialStartButton", card: "createImportCard"}
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        objectName: modelData.card
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        Layout.preferredHeight: 174
                        radius: Theme.panelRadius
                        color: Theme.panel
                        border.color: Theme.border
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 10
                            Label { text: modelData.title; color: Theme.text; font.pixelSize: 17; font.weight: Font.DemiBold }
                            Label { Layout.fillWidth: true; text: modelData.detail; color: Theme.muted; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                            Item { Layout.fillHeight: true }
                            ShapeButton {
                                objectName: modelData.name
                                text: modelData.action
                                onClicked: {
                                    dialog.close()
                                    if (modelData.key === "text") dialog.textAuthoringRequested("plain")
                                    else if (modelData.key === "image") dialog.aiImageSceneRequested()
                                    else if (modelData.key === "sound") dialog.soundSceneRequested()
                                    else dialog.importMaterialRequested()
                                }
                            }
                        }
                    }
                }
            }
            ShapeButton {
                objectName: "pasteMaterialStartButton"
                text: qsTr("Paste material")
                quiet: true
                onClicked: { dialog.close(); dialog.pasteMaterialRequested() }
            }
        }
    }
}

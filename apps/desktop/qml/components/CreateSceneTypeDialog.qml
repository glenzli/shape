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
    function start(preset) : void { close(); textAuthoringRequested(preset) }
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(700, parent.width - 48)
    modal: true
    title: qsTr("What do you want to make?")
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    contentItem: ColumnLayout {
        spacing: 16
        Label {
            Layout.fillWidth: true
            text: qsTr("Choose a format. Start with an idea, existing material, or an empty page.")
            wrapMode: Text.WordWrap
            color: Theme.muted
            font.pixelSize: 13
        }
        Label { text: qsTr("Text"); color: Theme.muted; font.pixelSize: 12 }
        RowLayout {
            Layout.fillWidth: true
            spacing: 14
            Rectangle {
                objectName: "createFreeWritingCard"
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.preferredHeight: 210
                color: Theme.panel
                border.color: Theme.border
                radius: Theme.panelRadius
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 12
                    ShapeIcon { source: "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"; color: Theme.accent; size: 22 }
                    Label { text: qsTr("Free writing"); color: Theme.text; font.pixelSize: 17; font.weight: Font.DemiBold }
                    Label { Layout.fillWidth: true; text: qsTr("Write with AI, revise existing text, or begin on your own."); wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 12 }
                    Item { Layout.fillHeight: true }
                    ShapeButton { objectName: "createTextSceneTypeButton"; text: qsTr("Start writing"); onClicked: dialog.start("plain") }
                }
            }
            Rectangle {
                objectName: "createScriptCard"
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.preferredHeight: 210
                color: Theme.panel
                border.color: Theme.border
                radius: Theme.panelRadius
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 6
                    ShapeIcon { source: "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"; color: Theme.accent; size: 22 }
                    Label { text: qsTr("Production script"); color: Theme.text; font.pixelSize: 17; font.weight: Font.DemiBold }
                    Label { Layout.fillWidth: true; text: qsTr("Write spoken lines and place roles, pauses, cues and repeats in the script itself."); wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 12 }
                    Item { Layout.fillHeight: true }
                    ShapeButton { objectName: "createProductionScriptButton"; text: qsTr("Start a script"); primary: true; onClicked: dialog.start("script") }
                }
            }
        }
        ShapeButton { objectName: "createSoundSceneTypeButton"; text: qsTr("Create sound effects or music"); onClicked: { dialog.close(); dialog.soundSceneRequested() } }
        Label { text: qsTr("Image and external material"); color: Theme.muted; font.pixelSize: 12 }
        RowLayout {
            Layout.fillWidth: true
            ShapeButton { objectName: "createAiImageSceneTypeButton"; text: qsTr("Generate an image with AI"); onClicked: { dialog.close(); dialog.aiImageSceneRequested() } }
            ShapeButton { objectName: "importMaterialStartButton"; text: qsTr("Import material…"); onClicked: { dialog.close(); dialog.importMaterialRequested() } }
            ShapeButton { objectName: "pasteMaterialStartButton"; text: qsTr("Paste material"); onClicked: { dialog.close(); dialog.pasteMaterialRequested() } }
            Item { Layout.fillWidth: true }
        }
        Label {
            Layout.fillWidth: true
            text: qsTr("Self-contained HTML can be previewed offline and exported. Other code files remain text sources.")
            color: Theme.muted
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }
    }
}

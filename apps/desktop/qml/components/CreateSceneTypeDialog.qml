pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

ShapeDialog {
    id: dialog
    objectName: "createSceneTypeDialog"
    property var templateKeys: ["picture_book", "card_kit", "material_edit"]
    signal textAuthoringRequested(string preset)
    signal aiImageSceneRequested()
    signal soundSceneRequested()
    signal pictureBookRequested()
    signal cardKitRequested()
    signal imageRefinementRequested()
    signal importMaterialRequested()
    signal pasteMaterialRequested()
    function openForCreation() : void { open() }
    function start(preset) : void { close(); textAuthoringRequested(preset) }
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(840, parent.width - 48)
    height: Math.min(760, parent.height - 48)
    modal: true
    title: qsTr("What do you want to make?")
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    contentItem: ScrollView {
        objectName: "createTaskScroll"
        contentWidth: availableWidth
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
        width: parent.width
        spacing: 14
        Label {
            Layout.fillWidth: true
            text: qsTr("Start with a complete task, or choose a single creative tool below.")
            wrapMode: Text.WordWrap
            color: Theme.muted
            font.pixelSize: 13
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Repeater {
                model: [
                    {title: qsTr("Picture book"), detail: qsTr("Write a story, prepare page artwork, then review the book."), action: qsTr("Create or continue a picture book"), name: "createPictureBookButton", task: "picture_book"},
                    {title: qsTr("Cards"), detail: qsTr("Make image or text cards, or arrange existing card images for printing."), action: qsTr("Make or arrange cards"), name: "createCardKitButton", task: "card_kit"},
                    {title: qsTr("Refine an image"), detail: qsTr("Choose an image, edit it in Shadow, then review the returned version."), action: qsTr("Choose an image"), name: "createImageRefinementButton", task: "material_edit"}
                ]
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 185
                    radius: Theme.panelRadius
                    color: Theme.accentSurfaceQuiet
                    border.color: Theme.accentBorder
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 10
                        Label { Layout.fillWidth: true; text: modelData.title; color: Theme.text; font.pixelSize: 17; font.weight: Font.DemiBold }
                        Label { Layout.fillWidth: true; text: modelData.detail; color: Theme.muted; font.pixelSize: 12; wrapMode: Text.WordWrap }
                        Item { Layout.fillHeight: true }
                        ShapeButton {
                            objectName: modelData.name
                            Layout.fillWidth: true
                            enabled: dialog.templateKeys.indexOf(modelData.task) >= 0
                            text: modelData.action
                            primary: true
                            onClicked: {
                                dialog.close()
                                if (modelData.task === "picture_book") dialog.pictureBookRequested()
                                else if (modelData.task === "card_kit") dialog.cardKitRequested()
                                else dialog.imageRefinementRequested()
                            }
                        }
                    }
                }
            }
        }
        Label { text: qsTr("Writing tools"); color: Theme.muted; font.pixelSize: 12 }
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
}

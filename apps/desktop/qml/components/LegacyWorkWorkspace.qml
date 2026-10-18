pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import Shape.Desktop

Item {
    id: page
    objectName: "legacyWorkWorkspace"
    required property LegacyWorkController controller
    readonly property var work: controller.work
    readonly property string kind: work.kind || ""
    signal deckRequested(string artifactId, string selection)
    signal materialRequested(string artifactId, string revisionId)
    signal writingRequested(string artifactId)
    signal closeRequested()
    function beginVox() : void {
        if (kind !== "speech") return
        if (controller.voxExecutable.length === 0) {
            voxChoice.continueAfterSelection = true
            voxChoice.selection = controller.selectionToken
            voxChoice.open()
        } else {
            voxProject.selection = controller.selectionToken
            voxProject.open()
        }
    }
    function caption() : string {
        if (kind === "card_kit") return qsTr("Continue making cards in Deck")
        if (kind === "picture_book") return qsTr("Continue your book in Storybook Co Editor")
        return qsTr("Continue voice production in Vox")
    }
    ScrollView {
        id: scroll
        anchors.fill: parent
        anchors.margins: 24
        contentWidth: availableWidth
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
            width: scroll.availableWidth
            spacing: 18
            Item { Layout.preferredHeight: 8 }
            RowLayout {
                Layout.fillWidth: true
                Label { text: page.caption(); color: Theme.accent; font.pixelSize: 13; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                ShapeButton { objectName: "legacyCloseButton"; text: qsTr("Back to materials"); quiet: true; onClicked: page.closeRequested() }
            }
            Label { text: page.work.name || ""; color: Theme.text; font.pixelSize: 25; font.weight: Font.DemiBold; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            Label {
                Layout.fillWidth: true
                text: page.kind === "speech"
                      ? qsTr("Continue from this text in Vox. Its accepted version and saved writing draft remain separate, and the Shape project stays unchanged.")
                      : qsTr("This saved Shape work is available for reading and transfer. Continue production in its dedicated editor; the original project stays unchanged.")
                color: Theme.muted
                wrapMode: Text.WordWrap
            }
            RowLayout {
                visible: page.kind !== "speech"
                Layout.fillWidth: true
                Label { text: qsTr("Snapshot"); color: Theme.text }
                ShapeComboBox {
                    objectName: "legacySnapshotChoice"
                    Layout.fillWidth: true
                    model: page.controller.snapshots
                    textRole: "label"
                    currentIndex: page.controller.snapshots.findIndex(s => s.key === page.controller.snapshotKey)
                    onActivated: index => page.controller.selectSnapshot(page.controller.snapshots[index].key)
                }
            }
            Label {
                objectName: "legacySnapshotStatus"
                Layout.fillWidth: true
                text: page.kind === "speech"
                      ? qsTr("The transfer keeps the script, saved voice settings and any selected recording separate. Existing audio keeps its original script and origin.")
                      : page.controller.snapshotKey === "draft"
                        ? qsTr("This draft has not been accepted. Its unfinished content will be identified as a draft in the receiving project.")
                        : qsTr("This is the accepted version. Saved unfinished changes remain available separately.")
                color: Theme.muted
                wrapMode: Text.WordWrap
            }
            Label {
                visible: page.kind === "card_kit" || page.kind === "picture_book"
                text: page.kind === "card_kit" ? qsTr("%1 card designs").arg(page.work.item_count || 0) : qsTr("%1 pages").arg(page.work.item_count || 0)
                color: Theme.text
            }
            Label { visible: (page.work.brief || "").length > 0; text: page.work.brief || ""; color: Theme.text; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            RowLayout {
                Layout.fillWidth: true
                ShapeButton {
                    objectName: "legacyContinueDeckButton"
                    visible: page.kind === "card_kit"
                    text: qsTr("Continue in Deck…")
                    primary: true
                    enabled: (page.work.deck_selection || "").length > 0
                    onClicked: page.deckRequested(page.work.artifact_id, page.work.deck_selection)
                }
                ShapeButton {
                    objectName: "legacyExportBookButton"
                    visible: page.kind === "picture_book"
                    text: qsTr("Export for Storybook…")
                    primary: true
                    onClicked: { bookExport.selection = page.controller.selectionToken; bookExport.open() }
                }
                ShapeButton {
                    objectName: "legacyContinueVoxButton"
                    visible: page.kind === "speech"
                    text: qsTr("Continue in Vox…")
                    primary: true
                    onClicked: page.beginVox()
                }
                Item { Layout.fillWidth: true }
            }
            RowLayout {
                Layout.fillWidth: true
                ShapeButton { visible: page.kind === "picture_book"; text: qsTr("Open Storybook"); onClicked: page.controller.openStorybook() }
                ShapeButton { visible: page.kind === "picture_book"; text: qsTr("Choose Storybook…"); quiet: true; onClicked: storybookChoice.open() }
                ShapeButton { objectName: "legacyExportVoxButton"; visible: page.kind === "speech"; text: qsTr("Export transfer files only…"); quiet: true; onClicked: { voxFolder.selection = page.controller.selectionToken; voxFolder.open() } }
                ShapeButton { visible: page.kind === "speech"; text: qsTr("Choose Vox…"); quiet: true; onClicked: { voxChoice.continueAfterSelection = false; voxChoice.open() } }
                ShapeButton { visible: page.controller.lastExport.length > 0; text: qsTr("Show exported files"); quiet: true; onClicked: page.controller.revealExport() }
                Item { Layout.fillWidth: true }
            }
            Label { visible: page.controller.status.length > 0; text: page.controller.status; color: Theme.muted; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Label { visible: page.controller.error.length > 0; text: page.controller.error; color: Theme.danger; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            ShapeTextEditor {
                objectName: "legacyCardContents"
                visible: page.kind === "card_kit" && (page.work.summary || "").trim().length > 0
                readOnly: true
                text: page.work.summary || ""
                Layout.fillWidth: true
                Layout.preferredHeight: 200
            }
            Label { text: qsTr("Original materials"); color: Theme.text; font.weight: Font.DemiBold }
            Repeater {
                model: page.controller.materials
                delegate: ShapeButton {
                    required property var modelData
                    objectName: "legacyMaterial_" + modelData.source.revision_id
                    Layout.fillWidth: true
                    text: modelData.role + " · " + (modelData.name || qsTr("Saved material"))
                    quiet: true
                    onClicked: page.materialRequested(modelData.source.artifact_id, modelData.source.revision_id)
                }
            }
            Label { visible: page.controller.materials.length === 0; text: qsTr("No material has been bound to this snapshot yet."); color: Theme.muted; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Label { visible: (page.work.unfinished_script || "").length > 0; text: qsTr("Saved unfinished script"); color: Theme.text; font.weight: Font.DemiBold }
            ShapeTextEditor {
                objectName: "legacyUnfinishedScript"
                visible: (page.work.unfinished_script || "").length > 0
                readOnly: true
                text: page.work.unfinished_script || ""
                Layout.fillWidth: true
                Layout.preferredHeight: 240
            }
            Item { Layout.preferredHeight: 12 }
        }
    }
    FileDialog { id: bookExport; objectName: "legacyBookExportDialog"; property string selection: ""; title: qsTr("Export for Storybook"); fileMode: FileDialog.SaveFile; nameFilters: [qsTr("Storybook project (*.scproj)")]; defaultSuffix: "scproj"; onAccepted: page.controller.exportBook(selectedFile, selection) }
    FolderDialog { id: voxFolder; objectName: "legacyVoxExportDialog"; property string selection: ""; title: qsTr("Choose a folder for the Vox transfer"); onAccepted: page.controller.exportVox(selectedFolder, selection) }
    FileDialog { id: voxProject; objectName: "legacyVoxProjectDialog"; property string selection: ""; title: qsTr("Create a separate Vox project"); fileMode: FileDialog.SaveFile; nameFilters: [qsTr("Vox project (*.vox)")]; defaultSuffix: "vox"; onAccepted: page.controller.openVox(selectedFile, selection) }
    FileDialog { id: storybookChoice; title: qsTr("Choose Storybook Co Editor"); fileMode: FileDialog.OpenFile; onAccepted: page.controller.chooseStorybook(selectedFile) }
    FileDialog { id: voxChoice; property bool continueAfterSelection: false; property string selection: ""; title: qsTr("Choose Vox"); fileMode: FileDialog.OpenFile; onAccepted: { if (page.controller.chooseVox(selectedFile) && continueAfterSelection && selection === page.controller.selectionToken) { voxProject.selection = selection; voxProject.open() } } }
}

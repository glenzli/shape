pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import Shape.Desktop

ShapeDialog {
    id: root
    objectName: "deckHandoffDialog"
    required property DeckHandoffController controller
    required property DesktopBackend backend
    property string kitId: ""
    property string draftDigest: ""
    property string projectId: ""
    property string projectPath: ""
    readonly property bool contextCurrent: !kitId.length || (projectId === backend.projectId && projectPath === backend.bundlePath)
    readonly property bool boundToKit: contextCurrent && kitId.length > 0 && controller.kitId === kitId
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(620, parent.width - 48)
    modal: true
    title: kitId.length ? qsTr("Continue cards in Deck") : qsTr("Make cards in Deck")
    function openForKit(artifactId, digest) {
        kitId = artifactId; draftDigest = digest
        projectId = backend.projectId; projectPath = backend.bundlePath
        if (kitId.length) controller.attach(kitId)
        open()
    }
    contentItem: ColumnLayout {
        spacing: 12
        Label {
            Layout.fillWidth: true
            text: root.kitId.length
                  ? qsTr("Send this saved card set and its chosen materials to an independent Deck project. Return exported PDF or ZIP files here when ready.")
                  : qsTr("Deck is the dedicated card editor for creating, arranging and printing cards. Shape can make images for it.")
            color: Theme.textSoft; wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true; visible: root.controller.executable.length === 0
            text: qsTr("Choose the installed Deck application to begin."); color: Theme.muted; wrapMode: Text.WordWrap
        }
        RowLayout {
            Layout.fillWidth: true
            ShapeButton { objectName: "deckChooseApplication"; text: qsTr("Choose Deck…"); enabled: !root.controller.busy && !root.controller.waiting; onClicked: applicationPicker.open() }
            Item { Layout.fillWidth: true }
            ShapeButton {
                objectName: "deckOpen"
                text: root.kitId.length ? qsTr("Send selected snapshot") : qsTr("Open Deck")
                primary: true; enabled: root.contextCurrent && !root.controller.busy && !root.controller.waiting && root.controller.executable.length > 0
                onClicked: root.kitId.length ? root.controller.openKit(root.kitId, root.draftDigest, true) : root.controller.openNewDeck()
            }
        }
        ShapeButton {
            objectName: "deckContinueLinkedProject"; visible: root.boundToKit && root.controller.hasDocument
            text: qsTr("Continue the linked Deck project")
            enabled: root.contextCurrent && !root.controller.busy && !root.controller.waiting
            onClicked: root.controller.openKit(root.kitId, root.draftDigest, false)
        }
        Label { Layout.fillWidth: true; visible: root.controller.status.length > 0; text: root.controller.status; wrapMode: Text.WordWrap; color: Theme.muted }
        Label { Layout.fillWidth: true; visible: root.controller.error.length > 0; text: root.controller.error; wrapMode: Text.WordWrap; color: Theme.warning }
        Repeater {
            model: root.boundToKit ? root.controller.outputs : []
            delegate: RowLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                Label { Layout.fillWidth: true; text: modelData.kind === "print_pdf" ? qsTr("Verified print PDF") : qsTr("Verified materials ZIP"); color: Theme.text; elide: Text.ElideRight }
                ShapeButton { text: qsTr("Open"); enabled: !root.controller.busy; onClicked: root.controller.openOutput(index, false) }
                ShapeButton { text: qsTr("Show folder"); enabled: !root.controller.busy; onClicked: root.controller.openOutput(index, true) }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            ShapeButton { text: qsTr("Close"); onClicked: root.close() }
            Item { Layout.fillWidth: true }
            ShapeButton { objectName: "deckReadResult"; visible: root.boundToKit && root.controller.requestPath.length > 0; text: qsTr("Read returned files"); enabled: !root.controller.busy; onClicked: root.controller.readReturnedOutputs() }
            ShapeButton { objectName: "deckRetryHandoff"; visible: root.boundToKit && root.controller.canRetry; text: qsTr("Retry this handoff"); enabled: root.contextCurrent; onClicked: root.controller.retryHandoff() }
            ShapeButton { objectName: "deckStopWaiting"; visible: root.controller.waiting; text: qsTr("Stop waiting"); enabled: !root.controller.busy; onClicked: root.controller.stopWaiting() }
        }
    }
    Connections { target: root.backend; function onProjectChanged() {if (!root.contextCurrent)root.close()} }
    FileDialog {
        id: applicationPicker
        title: qsTr("Choose Deck")
        fileMode: FileDialog.OpenFile
        onAccepted: root.controller.setExecutable(selectedFile)
    }
}

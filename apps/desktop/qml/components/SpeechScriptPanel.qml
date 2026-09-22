pragma ComponentBehavior: Bound

//! Authored role/cue bindings and a preview of the complete accepted script.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import Shape.Desktop
import "ScriptPresentation.js" as ScriptPresentation

ListView {
    id: panel
    objectName: "speechScriptPanel"
    required property DesktopBackend backend
    required property string draftId
    required property string optionsJson
    required property var preview
    required property var voices
    readonly property var options: optionsJson.length > 0 ? JSON.parse(optionsJson) : ({roles: {}, cues: {}})
    property string selectedCue: ""
    clip: true
    model: (preview.plan?.events || []).filter(event => ["speech", "pause", "cue", "heading", "scene", "repeat_start", "repeat_end"].indexOf(event.kind) >= 0)
    spacing: 6
    delegate: Label {
        required property var modelData
        width: panel.width
        text: panel.eventText(modelData)
        textFormat: Text.PlainText
        color: modelData.kind === "speech" ? Theme.text : Theme.muted
        font.pixelSize: 12
        wrapMode: Text.WordWrap
        padding: 4
    }
    ScrollBar.vertical: ScrollBar {}

    function currentOptions() : var {
        const draft = backend.operatorDrafts.find(item => item.id === draftId)
        return JSON.parse(draft ? draft.audioSpeechScriptJson : optionsJson)
    }
    function setRole(role, index) : void {
        const next = currentOptions()
        if (index === 0) delete next.roles[role]
        else {
            const voice = voices[index - 1]
            next.roles[role] = {alias: voice.alias, catalog_revision: voice.catalogRevision}
        }
        backend.updateSpeechScript(draftId, JSON.stringify(next))
    }
    function setCue(label, index) : void {
        if (index === 3) { selectedCue = label; cueFile.open(); return }
        const next = currentOptions()
        if (index === 0) delete next.cues[label]
        else next.cues[label] = index === 4 ? {action: "beep", milliseconds: 200, level: "soft"} : {action: index === 1 ? "chime" : "skip"}
        backend.updateSpeechScript(draftId, JSON.stringify(next))
    }
    function cueIndex(label) : int {
        const cue = options.cues[label]
        if (!cue) return 0
        return cue.action === "chime" ? 1 : cue.action === "skip" ? 2 : cue.action === "beep" ? 4 : 3
    }
    function eventText(event) : string {
        return (event.kind === "speech" ? (event.role || qsTr("Default voice")) + "\n" : "") + ScriptPresentation.eventText(event)
    }
    function issueText(issue) : string { return qsTr("Line %1: %2").arg(issue.line).arg(ScriptPresentation.issue(issue.code)) }

    FileDialog {
        id: cueFile
        title: qsTr("Choose a script audio cue")
        nameFilters: [qsTr("WAV audio (*.wav)")]
        fileMode: FileDialog.OpenFile
        onAccepted: panel.backend.importSpeechCue(panel.draftId, panel.selectedCue, selectedFile)
    }

    header: ColumnLayout {
        width: panel.width
        spacing: 10
        Label {
            Layout.fillWidth: true
            text: qsTr("Choose voices for the roles in your adopted script.")
            color: Theme.textSoft
            wrapMode: Text.WordWrap
            font.pixelSize: 11
        }
        Label {
            Layout.fillWidth: true
            text: qsTr("Pauses: %1 s · Sound cues: %2").arg((panel.preview.pause_ms || 0) / 1000).arg((panel.preview.cues || []).length)
            color: Theme.muted
            wrapMode: Text.WordWrap
            font.pixelSize: 11
        }
        Label {
            Layout.fillWidth: true
            text: qsTr("Delivery · %1").arg(ScriptPresentation.delivery(panel.preview.plan?.delivery))
            color: Theme.accent; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        Label {
            visible: (panel.preview.shared_voices || []).length > 0
            Layout.fillWidth: true
            text: qsTr("Some characters share a voice. For dialogue, choose distinguishable voices if they represent different people.")
            color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap
        }
        Repeater {
            model: panel.preview.roles || []
            delegate: ColumnLayout {
                required property string modelData
                Layout.fillWidth: true
                Label { text: parent.modelData; color: Theme.text; textFormat: Text.PlainText }
                Label {
                    Layout.fillWidth: true
                    text: qsTr("%1 · %2").arg(ScriptPresentation.delivery(panel.preview.plan?.roles[parent.modelData]?.delivery || panel.preview.plan?.delivery)).arg(panel.preview.plan?.roles[parent.modelData]?.description || panel.preview.plan?.roles[parent.modelData]?.language || "auto")
                    visible: text.length > 0
                    color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap
                }
                ShapeComboBox {
                    objectName: "scriptRoleVoice"
                    property string role: parent.modelData
                    Layout.fillWidth: true
                    model: [qsTr("Choose a voice…")].concat(panel.voices.map(v => v.label))
                    currentIndex: panel.options.roles[role] ? panel.voices.findIndex(v => v.alias === panel.options.roles[role].alias) + 1 : 0
                    onActivated: index => panel.setRole(role, index)
                }
            }
        }
        Repeater {
            model: panel.preview.cues || []
            delegate: ColumnLayout {
                required property string modelData
                Layout.fillWidth: true
                Label { text: qsTr("Audio cue: %1").arg(parent.modelData); color: Theme.text; textFormat: Text.PlainText }
                ShapeComboBox {
                    objectName: "scriptCueChoice"
                    property string cue: parent.modelData
                    Layout.fillWidth: true
                    model: [panel.preview.plan?.cues[cue] ? qsTr("Use script definition") : qsTr("Choose an action…"), qsTr("Built-in ding-dong"), qsTr("Skip this cue"), qsTr("Choose WAV file…"), qsTr("Soft beep · 0.2 seconds")]
                    currentIndex: panel.cueIndex(cue)
                    onActivated: index => panel.setCue(cue, index)
                }
                Label {
                    Layout.fillWidth: true
                    visible: panel.options.cues[parent.modelData]?.action === "audio"
                    text: qsTr("WAV stored in this project")
                    color: Theme.accent
                    font.pixelSize: 10
                }
            }
        }
        Repeater {
            model: panel.preview.plan?.issues || []
            delegate: Label {
                required property var modelData
                Layout.fillWidth: true
                text: panel.issueText(modelData)
                color: Theme.danger
                textFormat: Text.PlainText
                wrapMode: Text.WordWrap
            }
        }
        Label {
            Layout.fillWidth: true
            text: panel.preview.ready ? qsTr("Ready to synthesize") : qsTr("Choose a voice for every role and resolve all external cues.")
            color: panel.preview.ready ? Theme.accent : Theme.danger
            wrapMode: Text.WordWrap
        }
        Label { text: qsTr("PLAYBACK ORDER"); color: Theme.muted; font.pixelSize: 10 }
    }
}

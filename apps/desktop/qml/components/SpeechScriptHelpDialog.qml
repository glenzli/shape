pragma ComponentBehavior: Bound

//! Offline rules and an exact, copyable writing prompt from packaged documentation.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

ShapeDialog {
    id: dialog
    objectName: "speechScriptHelpDialog"
    property bool copied: false
    readonly property bool showingPrompt: tabs.currentIndex === 1
    readonly property bool showingDocument: tabs.currentIndex !== 0
    readonly property string documentText: showingPrompt ? SpeechScriptDocumentation.writingPrompt
                                                        : SpeechScriptDocumentation.rules

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(860, parent.width - 48)
    height: Math.min(720, parent.height - 48)
    modal: true
    title: qsTr("Production script guide")
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    onOpened: copied = false

    function copyCurrentDocument() : void {
        copied = SpeechScriptDocumentation.copyDocument(showingPrompt)
    }

    contentItem: ColumnLayout {
        spacing: 12
        RowLayout {
            id: tabs
            objectName: "speechScriptHelpTabs"
            property int currentIndex: 0
            Layout.fillWidth: true
            spacing: 8
            ShapeButton { text: qsTr("Script rules"); selected: tabs.currentIndex === 0; onClicked: tabs.currentIndex = 0 }
            ShapeButton { text: qsTr("AI writing prompt"); selected: tabs.currentIndex === 1; onClicked: tabs.currentIndex = 1 }
            ShapeButton { text: qsTr("Full specification"); selected: tabs.currentIndex === 2; onClicked: tabs.currentIndex = 2 }
            Item { Layout.fillWidth: true }
            onCurrentIndexChanged: dialog.copied = false
        }
        Label {
            Layout.fillWidth: true
            text: dialog.showingPrompt
                  ? qsTr("Copy this prompt, add your content requirements, and give it to your writing AI.")
                  : qsTr("These rules and examples are included with Shape and available offline.")
            color: Theme.textSoft
            wrapMode: Text.WordWrap
        }
        ScrollView {
            id: documentScroll
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: contentHeight > availableHeight ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: documentScroll.availableWidth - 12
                spacing: 12
                Rectangle {
                    visible: !dialog.showingDocument
                    Layout.fillWidth: true
                    implicitHeight: introduction.implicitHeight + 32
                    radius: Theme.radiusLarge
                    color: Theme.accentSoft
                    Label {
                        id: introduction
                        anchors.fill: parent
                        anchors.margins: 16
                        text: qsTr("Write the content first. Use instructions on their own lines to control delivery. Shape reads spoken text and inserts pauses and sound cues during synthesis.")
                        color: Theme.text
                        font.pixelSize: 14
                        wrapMode: Text.WordWrap
                    }
                }
                Repeater {
                    model: dialog.showingDocument ? [] : [
                        {title: qsTr("Production and delivery"), example: "[production: narration]\n[delivery: warm]", explanation: qsTr("Production labels describe the purpose. Overall delivery can be neutral, clear, warm or lively for any script; writing style and spoken delivery are separate.")},
                        {title: qsTr("Declare stable characters"), example: "[role: Narrator; language: auto]\n[role: Reader; language: auto]\n[speaker: Narrator]\n请听录音。Number one.", explanation: qsTr("Declare roles before playback begins, then bind every named role to a voice. Switching languages does not create a new character.")},
                        {title: qsTr("Repeat the same recording"), example: "[cue: turn; sound: beep]\nNumber one.\n[repeat: 2; gap: 2s; cue: turn]\nLook at the boy.\n[end-repeat]\n[pause: 5s]", explanation: qsTr("The question number plays once. The body is rendered once and replayed with identical audio. The cue plays only between the two plays, followed by 2 seconds of silence; 5 seconds follow the final play. A complete dialogue can also be repeated as one block.")},
                        {title: qsTr("Reusable beeps and sound cues"), example: "[cue: answer; sound: beep; duration: 0.2s; level: soft]\nHello.\n[audio: answer]\n[pause: 5s]", explanation: qsTr("Declare each cue once, then reference its name. Built-in cues are ready to play. External cues require a stored WAV or an explicit skip. The cue plays before the answering silence.")},
                        {title: qsTr("Scenes and local delivery"), example: "[production: narration]\n[scene: opening]\n[performance: warm]\nWelcome.\n[performance: default]\nLet us begin.", explanation: qsTr("Scenes organize the production without being spoken. Local delivery works in any script; repeat blocks cannot be nested.")},
                        {title: qsTr("Spoken text and silent directions"), example: "# Weekend plans\n[note: Check with the teacher]\nLook at the boy.", explanation: qsTr("Ordinary paragraphs are spoken. Headings and notes are silent. Executable directions must use a supported control; a note alone does not change the sound.")}
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: ruleBody.implicitHeight + 28
                        radius: Theme.radiusMedium
                        color: Theme.surface
                        border.color: Theme.border
                        ColumnLayout {
                            id: ruleBody
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 10
                            Label { text: modelData.title; color: Theme.text; font.pixelSize: 14; font.weight: Font.DemiBold }
                            ShapeTextArea {
                                Layout.fillWidth: true
                                readOnly: true
                                text: modelData.example
                                font.family: "monospace"
                                font.pixelSize: 12
                                wrapMode: TextEdit.Wrap
                                implicitHeight: contentHeight + 24
                            }
                            Label { Layout.fillWidth: true; text: modelData.explanation; color: Theme.muted; font.pixelSize: 12; wrapMode: Text.WordWrap }
                        }
                    }
                }
                ShapeTextArea {
                    objectName: "speechScriptHelpText"
                    visible: dialog.showingDocument
                    Layout.fillWidth: true
                    text: dialog.documentText
                    textFormat: TextEdit.PlainText
                    readOnly: true
                    selectByMouse: true
                    wrapMode: TextEdit.Wrap
                    font.pixelSize: 13
                    implicitHeight: contentHeight + 28
                }
            }
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
            ShapeButton {
                objectName: "copySpeechScriptDocumentButton"
                text: dialog.copied ? qsTr("Copied") : dialog.showingPrompt
                      ? qsTr("Copy AI writing prompt") : qsTr("Copy script rules")
                enabled: dialog.documentText.length > 0
                onClicked: dialog.copyCurrentDocument()
            }
            Item { Layout.fillWidth: true }
            ShapeButton {
                text: qsTr("Close")
                onClicked: dialog.close()
            }
        }
    }
}

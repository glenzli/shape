import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: panel

    property string artifactId: ""
    property string artifactKindKey: ""
    property string acceptedText: ""
    property bool candidatePending: false
    property string errorMessage: ""

    readonly property bool canEditText: artifactId.length > 0
                                         && artifactKindKey === "text_document"

    signal candidateRequested(string artifactId, string replacementText)

    function resetDraft() : void {
        draftEditor.text = acceptedText
    }

    onArtifactIdChanged: resetDraft()
    onAcceptedTextChanged: if (!candidatePending && !draftEditor.activeFocus) resetDraft()
    onCandidatePendingChanged: if (!candidatePending) resetDraft()

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: qsTr("TEXT DRAFT")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
                font.letterSpacing: 0.7
            }

            Item { Layout.fillWidth: true }

            Text {
                text: qsTr("Direct edit · candidate before commit")
                color: Theme.muted
                font.pixelSize: 10
            }
        }

        TextArea {
            id: draftEditor

            Layout.fillWidth: true
            Layout.fillHeight: true
            leftPadding: 12
            rightPadding: 12
            topPadding: 10
            bottomPadding: 10
            enabled: panel.canEditText && !panel.candidatePending
            placeholderText: panel.canEditText
                             ? qsTr("Write the next text revision…")
                             : qsTr("Select a text document to begin")
            color: Theme.text
            placeholderTextColor: Theme.muted
            selectionColor: Theme.accentSoft
            selectedTextColor: Theme.text
            wrapMode: TextEdit.Wrap
            font.pixelSize: Theme.fontBody
            Accessible.name: qsTr("Text draft")

            background: Rectangle {
                color: Theme.raised
                radius: Theme.radiusSmall
                border.color: parent.activeFocus ? Theme.accent : Theme.border
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Text {
                Layout.fillWidth: true
                text: panel.errorMessage.length > 0
                      ? panel.errorMessage
                      : panel.candidatePending
                        ? qsTr("A candidate is ready. Compare, discard, or accept it.")
                        : qsTr("Accepted history stays unchanged until you accept the candidate.")
                color: panel.errorMessage.length > 0 ? Theme.danger
                                                     : panel.candidatePending ? Theme.accent
                                                                              : Theme.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }

            ShapeButton {
                text: qsTr("Create candidate")
                primary: true
                enabled: panel.canEditText
                         && !panel.candidatePending
                         && draftEditor.text.trim().length > 0
                         && draftEditor.text !== panel.acceptedText
                onClicked: panel.candidateRequested(panel.artifactId, draftEditor.text)
            }
        }
    }

    Component.onCompleted: resetDraft()
}

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
    property var candidates: []
    property string errorMessage: ""
    property bool runtimeProbing: false
    property bool runtimeReachable: false
    property bool runtimeCompatible: false
    property string runtimeContractVersion: ""

    readonly property bool canEditText: artifactId.length > 0
                                         && artifactKindKey === "text_document"

    signal candidateRequested(string artifactId, string replacementText)
    signal runtimeRefreshRequested()

    function resetDraft() : void {
        draftEditor.text = acceptedText
    }

    function draftMatchesCandidate() : bool {
        for (let index = 0; index < candidates.length; ++index) {
            if (candidates[index].text === draftEditor.text) {
                return true
            }
        }
        return false
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

            ShapeButton {
                objectName: "inferRuntimeStatusButton"
                implicitHeight: 26
                text: panel.runtimeProbing ? qsTr("Checking AI…")
                      : panel.runtimeCompatible ? qsTr("AI ready")
                      : panel.runtimeReachable ? qsTr("AI contract mismatch")
                      : qsTr("AI offline")
                selected: panel.runtimeCompatible
                enabled: !panel.runtimeProbing
                Accessible.name: panel.runtimeCompatible
                                 ? qsTr("Infer Runtime ready, contract %1").arg(
                                       panel.runtimeContractVersion)
                                 : text
                onClicked: panel.runtimeRefreshRequested()
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
            enabled: panel.canEditText
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
                        ? qsTr("Keep editing to add another option, or review the shelf.")
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
                         && draftEditor.text.trim().length > 0
                         && draftEditor.text !== panel.acceptedText
                         && !panel.draftMatchesCandidate()
                onClicked: panel.candidateRequested(panel.artifactId, draftEditor.text)
            }
        }
    }

    Component.onCompleted: resetDraft()
}

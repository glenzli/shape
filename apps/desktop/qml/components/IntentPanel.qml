import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: panel
    objectName: "intentPanel"

    property string artifactId: ""
    property string artifactKindKey: ""
    property string acceptedText: ""
    property bool hasAcceptedRevision: false
    property bool candidatePending: false
    property var candidates: []
    property string errorMessage: ""
    property bool runtimeProbing: false
    property bool runtimeReachable: false
    property bool runtimeCompatible: false
    property string runtimeContractVersion: ""
    property string runtimeEndpointSource: ""
    property bool generationRunning: false
    property bool credentialConfigured: false
    property string generationErrorCode: ""
    property string operatorDraftId: ""
    property string operatorTypeKey: ""
    property string textTransformMode: "rewrite"
    property string textTransformInstruction: ""
    property string pendingTextTransformMode: "rewrite"

    readonly property bool selectedTextDocument: artifactId.length > 0
                                                && artifactKindKey === "text_document"
    readonly property bool canEditText: selectedTextDocument && hasAcceptedRevision
    readonly property bool editingText: operatorTypeKey === "text.edit"
    readonly property bool transformingText: operatorTypeKey === "text.transform"

    signal candidateRequested(string artifactId, string replacementText)
    signal inferCandidateRequested(string artifactId, string draftId,
                                   string modeKey, string instruction)
    signal textTransformDraftConfigurationSaveRequested(string draftId,
                                                        string modeKey,
                                                        string instruction)
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

    function generationErrorText() : string {
        switch (generationErrorCode) {
        case "": return ""
        case "credential_missing": return qsTr("Add the Shape credential from Infer Console in Settings.")
        case "credential_invalid":
        case "credential_unsafe": return qsTr("The saved Infer credential is invalid or unsafe.")
        case "invalid_api_key": return qsTr("Infer rejected the Shape credential. Rotate it in Infer Console.")
        case "intent_forbidden": return qsTr("Shape is not allowed to use assistant.general.")
        case "policy_violation": return qsTr("Infer rejected Shape's local-only execution policy.")
        case "no_candidate": return qsTr("No local model currently satisfies this request.")
        case "provider_unavailable":
        case "infer_unavailable": return qsTr("The local Infer provider is unavailable.")
        case "stale_candidate": return qsTr("The accepted revision changed while AI was working. Try again.")
        case "invalid_prompt": return qsTr("Enter a short creative instruction for AI.")
        default: return qsTr("AI could not create a candidate.")
        }
    }

    function synchronizeTransformInstruction() : void {
        instructionSaveTimer.stop()
        if (generationPrompt.text !== textTransformInstruction) {
            generationPrompt.text = textTransformInstruction
        }
    }

    function normalizeTransformMode(modeKey) : string {
        switch (modeKey) {
        case "rewrite":
        case "expand":
        case "polish":
        case "shorten": return modeKey
        default: return "rewrite"
        }
    }

    function transformModeLabel(modeKey) : string {
        switch (modeKey) {
        case "expand": return qsTr("Expand")
        case "polish": return qsTr("Polish")
        case "shorten": return qsTr("Shorten")
        default: return qsTr("Rewrite")
        }
    }

    function synchronizeTransformMode() : void {
        pendingTextTransformMode = normalizeTransformMode(textTransformMode)
    }

    function selectTransformMode(modeKey) : void {
        pendingTextTransformMode = normalizeTransformMode(modeKey)
        instructionSaveTimer.stop()
        if (generationPrompt.text.trim().length > 0) {
            persistTransformConfiguration()
        }
    }

    function persistTransformConfiguration() : void {
        if (!transformingText || operatorDraftId.length === 0
                || (generationPrompt.text === textTransformInstruction
                    && pendingTextTransformMode
                       === normalizeTransformMode(textTransformMode))) {
            return
        }
        textTransformDraftConfigurationSaveRequested(
                    operatorDraftId, pendingTextTransformMode,
                    generationPrompt.text)
    }

    onArtifactIdChanged: resetDraft()
    onAcceptedTextChanged: if (!candidatePending && !draftEditor.activeFocus) resetDraft()
    onCandidatePendingChanged: if (!candidatePending) resetDraft()
    onOperatorDraftIdChanged: synchronizeTransformInstruction()
    onTextTransformModeChanged: synchronizeTransformMode()
    onTextTransformInstructionChanged: synchronizeTransformInstruction()
    onVisibleChanged: {
        if (!visible && instructionSaveTimer.running) {
            instructionSaveTimer.stop()
            persistTransformConfiguration()
        }
    }

    Timer {
        id: instructionSaveTimer
        interval: 350
        repeat: false
        onTriggered: panel.persistTransformConfiguration()
    }

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
                text: panel.artifactKindKey === "image_raster"
                      ? qsTr("IMAGE CROP") : qsTr("TEXT OPERATOR")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
                font.letterSpacing: 0.7
            }

            Item { Layout.fillWidth: true }

            Text {
                text: panel.artifactKindKey === "image_raster"
                      ? qsTr("Direct manipulation · candidate before commit")
                      : qsTr("Deterministic calibration · candidate before accept")
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
                                 ? panel.runtimeEndpointSource === "discovery"
                                   ? qsTr("Infer Runtime ready through Infra Discovery, contract %1").arg(
                                         panel.runtimeContractVersion)
                                   : panel.runtimeEndpointSource === "explicit_override"
                                     ? qsTr("Infer Runtime ready through an explicit endpoint, contract %1").arg(
                                           panel.runtimeContractVersion)
                                     : panel.runtimeEndpointSource === "compatibility_fallback"
                                       ? qsTr("Infer Runtime ready through the compatibility fallback, contract %1").arg(
                                             panel.runtimeContractVersion)
                                       : qsTr("Infer Runtime ready, contract %1").arg(
                                             panel.runtimeContractVersion)
                                 : text
                onClicked: panel.runtimeRefreshRequested()
            }
        }

        TextArea {
            id: draftEditor

            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: panel.canEditText && panel.editingText
            leftPadding: 12
            rightPadding: 12
            topPadding: 10
            bottomPadding: 10
            enabled: panel.canEditText
            placeholderText: panel.canEditText
                             ? qsTr("Write the next text revision…")
                             : panel.selectedTextDocument
                               ? qsTr("This text document needs an accepted origin")
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
            spacing: 6
            visible: panel.canEditText && panel.transformingText

            Text {
                text: qsTr("Mode")
                color: Theme.muted
                font.pixelSize: 10
            }

            ShapeButton {
                objectName: "textTransformModeRewriteButton"
                implicitHeight: 26
                text: panel.transformModeLabel("rewrite")
                selected: panel.pendingTextTransformMode === "rewrite"
                enabled: panel.canEditText && !panel.generationRunning
                Accessible.name: text
                onClicked: panel.selectTransformMode("rewrite")
            }

            ShapeButton {
                objectName: "textTransformModeExpandButton"
                implicitHeight: 26
                text: panel.transformModeLabel("expand")
                selected: panel.pendingTextTransformMode === "expand"
                enabled: panel.canEditText && !panel.generationRunning
                Accessible.name: text
                onClicked: panel.selectTransformMode("expand")
            }

            ShapeButton {
                objectName: "textTransformModePolishButton"
                implicitHeight: 26
                text: panel.transformModeLabel("polish")
                selected: panel.pendingTextTransformMode === "polish"
                enabled: panel.canEditText && !panel.generationRunning
                Accessible.name: text
                onClicked: panel.selectTransformMode("polish")
            }

            ShapeButton {
                objectName: "textTransformModeShortenButton"
                implicitHeight: 26
                text: panel.transformModeLabel("shorten")
                selected: panel.pendingTextTransformMode === "shorten"
                enabled: panel.canEditText && !panel.generationRunning
                Accessible.name: text
                onClicked: panel.selectTransformMode("shorten")
            }

            Item { Layout.fillWidth: true }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            visible: panel.canEditText && panel.transformingText

            TextField {
                id: generationPrompt
                objectName: "textTransformInstructionField"

                Layout.fillWidth: true
                implicitHeight: 30
                enabled: panel.canEditText && !panel.generationRunning
                placeholderText: panel.credentialConfigured
                                 ? qsTr("Describe how AI should transform this text…")
                                 : qsTr("Add an Infer credential in Settings to use AI")
                color: Theme.text
                placeholderTextColor: Theme.muted
                selectionColor: Theme.accentSoft
                selectedTextColor: Theme.text
                font.pixelSize: 11
                Accessible.name: qsTr("AI transform instruction")
                onTextEdited: instructionSaveTimer.restart()
                onEditingFinished: {
                    instructionSaveTimer.stop()
                    panel.persistTransformConfiguration()
                }

                background: Rectangle {
                    color: Theme.raised
                    radius: Theme.radiusSmall
                    border.color: parent.activeFocus ? Theme.accent : Theme.border
                }
            }

            ShapeButton {
                objectName: "inferGenerateButton"
                implicitHeight: 30
                text: panel.generationRunning ? qsTr("Transforming…") : qsTr("Transform with AI")
                selected: panel.generationRunning
                enabled: panel.canEditText
                         && panel.runtimeCompatible
                         && panel.credentialConfigured
                         && !panel.generationRunning
                         && panel.operatorDraftId.length > 0
                         && generationPrompt.text.trim().length > 0
                onClicked: panel.inferCandidateRequested(
                               panel.artifactId, panel.operatorDraftId,
                               panel.pendingTextTransformMode,
                               generationPrompt.text)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            visible: panel.canEditText

            Text {
                Layout.fillWidth: true
                text: panel.generationErrorText().length > 0
                      ? panel.generationErrorText()
                      : panel.errorMessage.length > 0
                      ? panel.errorMessage
                      : panel.candidatePending
                        ? qsTr("Keep editing to add another option, or review the shelf.")
                        : qsTr("Accepted history stays unchanged until you accept the candidate.")
                color: panel.generationErrorText().length > 0
                       || panel.errorMessage.length > 0 ? Theme.danger
                                                     : panel.candidatePending ? Theme.accent
                                                                              : Theme.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }

            ShapeButton {
                text: qsTr("Preview calibration")
                primary: true
                visible: panel.editingText
                enabled: panel.canEditText
                         && draftEditor.text.trim().length > 0
                         && draftEditor.text !== panel.acceptedText
                         && !panel.draftMatchesCandidate()
                onClicked: panel.candidateRequested(panel.artifactId, draftEditor.text)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: panel.artifactKindKey === "image_raster"
            radius: Theme.radiusSmall
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 32, 520)
                spacing: 8

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Shape the crop directly on the image")
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    Layout.fillWidth: true
                    text: panel.errorMessage.length > 0
                          ? panel.errorMessage
                          : qsTr("Drag the frame to move it, use the lower-right handle to resize it, then create a transient candidate.")
                    color: panel.errorMessage.length > 0 ? Theme.danger : Theme.muted
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }

    Component.onCompleted: {
        resetDraft()
        synchronizeTransformInstruction()
        synchronizeTransformMode()
    }
}

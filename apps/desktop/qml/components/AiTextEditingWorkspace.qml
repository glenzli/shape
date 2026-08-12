pragma ComponentBehavior: Bound

//! Complete interaction owner for one reusable AI text-editing node. Graph
//! inputs remain outside the workspace; this component owns authored intent,
//! batch generation pacing, candidate review, selection refinement, and the
//! explicit lock-output affordance.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: workspace
    objectName: "aiTextEditingWorkspace"

    property string artifactId: ""
    property string artifactName: ""
    property string operatorDraftId: ""
    property string acceptedText: ""
    property bool hasAcceptedRevision: false
    property var candidates: []
    property string selectedCandidateId: ""
    property bool generationRunning: false
    property string generationErrorCode: ""
    property bool runtimeCompatible: false
    property bool credentialConfigured: false
    property string persistedMode: "rewrite"
    property string persistedInstruction: ""
    property string persistedExpressionJson: ""
    property string persistedStyle: "natural"
    property int persistedVariantCount: 1

    property string pendingMode: normalizeMode(persistedMode)
    property string pendingExpressionJson: normalizeExpressionJson(persistedExpressionJson)
    property string pendingStyle: normalizeStyle(persistedStyle)
    property int pendingVariantCount: persistedVariantCount === 3 ? 3 : 1
    property int queuedGenerationCount: 0
    property int observedCandidateCount: candidates.length
    property bool batchActive: false

    readonly property var selectedCandidate: candidateForId(selectedCandidateId)
    readonly property int materialCount: hasAcceptedRevision ? 1 : 0
    readonly property string outputText: selectedCandidate !== null ? selectedCandidate.text : acceptedText
    readonly property bool canGenerate: hasAcceptedRevision && operatorDraftId.length > 0 && runtimeCompatible && credentialConfigured && expressionPalette.expressionComplete && !generationRunning && !batchActive
    readonly property bool canUsePrimaryAction: hasAcceptedRevision && (operatorDraftId.length === 0 || runtimeCompatible) && expressionPalette.expressionComplete && !generationRunning && !batchActive
    readonly property bool canLockOutput: selectedCandidate !== null

    signal draftSaveRequested(string draftId, string modeKey, string instruction, string expressionJson, string styleKey, int variantCount)
    signal generationRequested(string artifactId, string draftId, string modeKey, string instruction, string expressionJson, string styleKey, int variantCount)
    signal candidateSelected(string candidateId)
    signal candidateLockRequested(string candidateId)
    signal setupRequested
    signal newDraftRequested

    function candidateForId(candidateId): var {
        for (let index = 0; index < candidates.length; ++index) {
            if (candidates[index].id === candidateId)
                return candidates[index];
        }
        return null;
    }

    function normalizeMode(value): string {
        switch (value) {
        case "rewrite":
        case "expand":
        case "polish":
        case "shorten":
        case "summarize":
            return value;
        default:
            return "rewrite";
        }
    }

    function normalizeExpressionJson(value): string {
        if (value.length > 0) {
            try {
                const decoded = JSON.parse(value);
                if (decoded.tones && decoded.tones.length > 0)
                    return JSON.stringify(decoded);
            } catch (error) {
                // The Rust codec remains authoritative; a malformed projection
                // falls back visibly instead of becoming authored state.
            }
        }
        return JSON.stringify({
            "tones": [
                {
                    "kind": "preset",
                    "preset": "neutral"
                }
            ],
            "intensity": "balanced",
            "audience": {
                "kind": "preset",
                "preset": "general"
            }
        });
    }

    function normalizeStyle(value): string {
        switch (value) {
        case "natural":
        case "concise":
        case "professional":
        case "literary":
        case "casual":
            return value;
        default:
            return "natural";
        }
    }

    function modeLabel(value): string {
        switch (value) {
        case "expand":
            return qsTr("Expand");
        case "polish":
            return qsTr("Polish");
        case "shorten":
            return qsTr("Shorten");
        case "summarize":
            return qsTr("Summarize");
        default:
            return qsTr("Rewrite");
        }
    }

    function styleLabel(value): string {
        switch (value) {
        case "concise":
            return qsTr("Concise");
        case "professional":
            return qsTr("Professional");
        case "literary":
            return qsTr("Literary");
        case "casual":
            return qsTr("Casual");
        default:
            return qsTr("Natural");
        }
    }

    function persistIntent(): void {
        if (operatorDraftId.length === 0)
            return;
        draftSaveRequested(operatorDraftId, pendingMode, promptEditor.text, pendingExpressionJson, pendingStyle, pendingVariantCount);
    }

    function chooseMode(modeKey): void {
        pendingMode = normalizeMode(modeKey);
        persistTimer.restart();
    }

    function chooseStyle(styleKey): void {
        pendingStyle = normalizeStyle(styleKey);
        persistTimer.restart();
    }

    function startGeneration(): void {
        if (!canGenerate)
            return;
        persistTimer.stop();
        persistIntent();
        queuedGenerationCount = pendingVariantCount;
        observedCandidateCount = candidates.length;
        batchActive = true;
        requestNextCandidate();
    }

    function triggerPrimaryAction(): void {
        if (!canUsePrimaryAction)
            return;
        if (operatorDraftId.length === 0) {
            newDraftRequested();
            return;
        }
        if (!credentialConfigured) {
            setupRequested();
            return;
        }
        startGeneration();
    }

    function primaryActionText(): string {
        if (generationRunning || batchActive) {
            return qsTr("Generating %1…").arg(Math.max(1, queuedGenerationCount + 1));
        }
        if (operatorDraftId.length === 0)
            return qsTr("Edit again");
        if (!runtimeCompatible)
            return qsTr("AI runtime unavailable");
        if (!credentialConfigured)
            return qsTr("Set up AI access");
        return pendingVariantCount === 1 ? qsTr("Generate") : qsTr("Generate %1 versions").arg(pendingVariantCount);
    }

    function requestNextCandidate(): void {
        if (!batchActive || generationRunning || queuedGenerationCount <= 0)
            return;
        queuedGenerationCount -= 1;
        generationRequested(artifactId, operatorDraftId, pendingMode, promptEditor.text, pendingExpressionJson, pendingStyle, pendingVariantCount);
    }

    function stopBatch(): void {
        queuedGenerationCount = 0;
        batchActive = false;
    }

    function prepareSelectionRevision(): void {
        const selection = outputEditor.selectedText.trim();
        if (selection.length === 0)
            return;
        pendingMode = "polish";
        promptEditor.text = qsTr("Revise only this selected passage while preserving the surrounding text: “%1”").arg(selection);
        promptEditor.forceActiveFocus();
        persistTimer.restart();
    }

    function generationErrorText(): string {
        switch (generationErrorCode) {
        case "":
            return "";
        case "credential_missing":
            return qsTr("Add the Shape credential in Settings.");
        case "invalid_api_key":
            return qsTr("Infer rejected the Shape credential.");
        case "intent_forbidden":
            return qsTr("Shape is not allowed to use text generation.");
        case "no_candidate":
            return qsTr("No available model can execute this intent.");
        case "provider_unavailable":
        case "infer_unavailable":
            return qsTr("The AI runtime is currently unavailable.");
        case "stale_candidate":
            return qsTr("The locked input changed. Reopen this node and try again.");
        default:
            return qsTr("AI could not generate a text candidate.");
        }
    }

    onPersistedModeChanged: pendingMode = normalizeMode(persistedMode)
    onPersistedExpressionJsonChanged: pendingExpressionJson = normalizeExpressionJson(persistedExpressionJson)
    onPersistedStyleChanged: pendingStyle = normalizeStyle(persistedStyle)
    onPersistedVariantCountChanged: pendingVariantCount = persistedVariantCount === 3 ? 3 : 1
    onPersistedInstructionChanged: {
        if (!promptEditor.activeFocus && promptEditor.text !== persistedInstruction) {
            promptEditor.text = persistedInstruction;
        }
    }
    onGenerationErrorCodeChanged: if (generationErrorCode.length > 0)
        stopBatch()
    onCandidatesChanged: {
        if (!batchActive || candidates.length <= observedCandidateCount)
            return;
        if (generationRunning)
            return;
        observedCandidateCount = candidates.length;
        if (queuedGenerationCount <= 0)
            stopBatch();
        else if (!generationRunning)
            Qt.callLater(workspace.requestNextCandidate);
    }
    onGenerationRunningChanged: {
        if (!generationRunning && batchActive && generationErrorCode.length === 0 && candidates.length > observedCandidateCount) {
            observedCandidateCount = candidates.length;
            if (queuedGenerationCount <= 0)
                stopBatch();
            else
                Qt.callLater(workspace.requestNextCandidate);
        }
    }

    Timer {
        id: persistTimer
        interval: 350
        repeat: false
        onTriggered: workspace.persistIntent()
    }

    component ChoiceChip: Button {
        id: chip

        property bool current: false

        implicitWidth: Math.max(54, chipLabel.implicitWidth + 18)
        implicitHeight: Theme.compactControlHeight
        leftPadding: 9
        rightPadding: 9
        focusPolicy: Qt.StrongFocus

        background: Rectangle {
            radius: Theme.compactControlRadius
            color: {
                if (!chip.enabled)
                    return Theme.controlQuiet;
                if (chip.current)
                    return Theme.accentSoft;
                if (chip.down)
                    return Theme.buttonGhostPressed;
                if (chip.hovered)
                    return Theme.buttonGhostHover;
                return Theme.control;
            }
            border.width: chip.current || chip.visualFocus ? 1 : 0
            border.color: chip.visualFocus ? Theme.focusRing : Theme.accentBorder
        }

        contentItem: Text {
            id: chipLabel
            text: chip.text
            color: chip.enabled ? chip.current ? Theme.accent : Theme.textSoft : Theme.disabled
            font.pixelSize: Theme.fontSection
            font.weight: chip.current ? Font.DemiBold : Font.Normal
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 460
                color: Theme.panelRaised

                Rectangle {
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    width: 1
                    color: Theme.border
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 16

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Source text")
                            color: Theme.text
                            font.pixelSize: Theme.fontHeading
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Rectangle {
                            Layout.preferredWidth: sourceBadgeText.implicitWidth + 20
                            Layout.preferredHeight: 26
                            radius: 13
                            color: workspace.hasAcceptedRevision ? Theme.surfaceSelected : Theme.surfaceSubtle

                            Text {
                                id: sourceBadgeText
                                anchors.centerIn: parent
                                text: workspace.hasAcceptedRevision ? qsTr("Locked source") : qsTr("Source missing")
                                color: workspace.hasAcceptedRevision ? Theme.textSoft : Theme.muted
                                font.pixelSize: Theme.fontMeta
                                font.weight: Font.DemiBold
                            }
                        }
                    }

                    TextArea {
                        id: sourceEditor
                        objectName: "aiTextSourceEditor"
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(180, Math.max(104, contentHeight + 28))
                        readOnly: true
                        selectByMouse: true
                        text: workspace.acceptedText.length > 0 ? workspace.acceptedText : qsTr("Connect a text source to start editing.")
                        color: workspace.hasAcceptedRevision ? Theme.text : Theme.muted
                        wrapMode: TextEdit.Wrap
                        font.pixelSize: 14
                        selectionColor: Theme.accentSoft
                        selectedTextColor: Theme.text
                        leftPadding: 14
                        rightPadding: 14
                        topPadding: 12
                        bottomPadding: 12
                        background: Rectangle {
                            radius: Theme.controlRadius
                            color: Theme.panelInset
                            border.color: sourceEditor.activeFocus ? Theme.focusRing : Theme.border
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.border
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Text {
                                text: qsTr("New version")
                                color: Theme.text
                                font.pixelSize: Theme.fontHeading
                                font.weight: Font.DemiBold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: workspace.selectedCandidate !== null ? qsTr("Review, refine, then use it as this node's output.") : qsTr("Your source stays untouched until you choose a version.")
                                color: Theme.muted
                                font.pixelSize: Theme.fontMeta
                                elide: Text.ElideRight
                            }
                        }

                        Rectangle {
                            Layout.preferredWidth: candidateCountText.implicitWidth + 18
                            Layout.preferredHeight: 26
                            radius: 13
                            color: Theme.surfaceSubtle
                            Text {
                                id: candidateCountText
                                anchors.centerIn: parent
                                text: qsTr("%1 versions").arg(workspace.candidates.length)
                                color: Theme.textSoft
                                font.pixelSize: Theme.fontMeta
                            }
                        }
                    }

                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.minimumHeight: 210

                        TextArea {
                            id: outputEditor
                            objectName: "aiTextOutputEditor"
                            anchors.fill: parent
                            visible: workspace.selectedCandidate !== null
                            readOnly: true
                            selectByMouse: true
                            text: workspace.selectedCandidate !== null ? workspace.selectedCandidate.text : ""
                            color: Theme.text
                            wrapMode: TextEdit.Wrap
                            font.pixelSize: 15
                            selectionColor: Theme.accentSoft
                            selectedTextColor: Theme.text
                            leftPadding: 16
                            rightPadding: 16
                            topPadding: 14
                            bottomPadding: 14
                            background: Rectangle {
                                radius: Theme.controlRadius
                                color: Theme.panelInset
                                border.color: outputEditor.activeFocus ? Theme.focusRing : Theme.border
                            }
                        }

                        ColumnLayout {
                            anchors.centerIn: parent
                            width: Math.min(parent.width - 48, 360)
                            visible: workspace.selectedCandidate === null
                            spacing: 10

                            Rectangle {
                                Layout.alignment: Qt.AlignHCenter
                                Layout.preferredWidth: 46
                                Layout.preferredHeight: 46
                                radius: 12
                                color: Theme.accentSurfaceQuiet

                                ShapeIcon {
                                    anchors.centerIn: parent
                                    source: "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                                    color: Theme.accent
                                    size: 20
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: workspace.hasAcceptedRevision ? qsTr("Ready for a new version") : qsTr("Connect a source first")
                                color: Theme.text
                                font.pixelSize: Theme.fontHeading
                                font.weight: Font.DemiBold
                                horizontalAlignment: Text.AlignHCenter
                            }

                            Text {
                                Layout.fillWidth: true
                                text: workspace.hasAcceptedRevision ? qsTr("Choose an edit direction and generate one or more alternatives.") : qsTr("This node keeps your editing intent while it waits for material.")
                                color: Theme.muted
                                font.pixelSize: Theme.fontMeta
                                wrapMode: Text.WordWrap
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            Layout.fillWidth: true
                            text: outputEditor.selectedText.trim().length > 0 ? qsTr("Selected text can be revised again") : qsTr("Select a passage to refine it again")
                            color: Theme.muted
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        ShapeIconButton {
                            objectName: "refineSelectedTextButton"
                            source: "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
                            toolTipText: qsTr("Refine selected text")
                            accessibleName: toolTipText
                            buttonSize: 32
                            enabled: outputEditor.selectedText.trim().length > 0
                            onClicked: workspace.prepareSelectionRevision()
                        }

                        ShapeButton {
                            objectName: "lockTextCandidateButton"
                            primary: true
                            iconSource: "qrc:/qt/qml/Shape/Desktop/icons/verified.svg"
                            text: qsTr("Use version")
                            enabled: workspace.canLockOutput
                            onClicked: workspace.candidateLockRequested(workspace.selectedCandidateId)
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 348
                Layout.maximumWidth: 372
                Layout.fillHeight: true
                color: Theme.panel

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 14

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            text: qsTr("Edit direction")
                            color: Theme.text
                            font.pixelSize: Theme.fontHeading
                            font.weight: Font.DemiBold
                        }
                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Describe the change; Shape keeps the source outside this node.")
                            color: Theme.muted
                            font.pixelSize: Theme.fontMeta
                            wrapMode: Text.WordWrap
                        }
                    }

                    Text {
                        text: qsTr("PROMPT · OPTIONAL")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    TextArea {
                        id: promptEditor
                        objectName: "aiTextPromptEditor"
                        Layout.fillWidth: true
                        Layout.preferredHeight: 72
                        text: workspace.persistedInstruction
                        placeholderText: qsTr("Add specific instructions, facts to preserve, or a direction for this edit…")
                        color: Theme.text
                        placeholderTextColor: Theme.muted
                        wrapMode: TextEdit.Wrap
                        selectByMouse: true
                        leftPadding: 12
                        rightPadding: 12
                        topPadding: 10
                        bottomPadding: 10
                        onTextChanged: if (activeFocus)
                            persistTimer.restart()
                        background: Rectangle {
                            color: Theme.control
                            radius: Theme.controlRadius
                            border.color: promptEditor.activeFocus ? Theme.focusRing : Theme.border
                        }
                    }

                    ScrollView {
                        id: intentScroll
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true

                        ColumnLayout {
                            width: Math.max(0, intentScroll.availableWidth)
                            spacing: 11

                            Text {
                                text: qsTr("QUICK ACTION")
                                color: Theme.muted
                                font.pixelSize: Theme.fontMicro
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.6
                            }

                            Flow {
                                Layout.fillWidth: true
                                Layout.preferredHeight: childrenRect.height
                                spacing: 6

                                Repeater {
                                    model: ["rewrite", "expand", "polish", "shorten", "summarize"]
                                    delegate: ChoiceChip {
                                        required property string modelData
                                        objectName: "textAction-" + modelData
                                        text: workspace.modeLabel(modelData)
                                        current: workspace.pendingMode === modelData
                                        onClicked: workspace.chooseMode(modelData)
                                    }
                                }
                            }

                            TextExpressionPalette {
                                id: expressionPalette
                                Layout.fillWidth: true
                                persistedExpressionJson: workspace.persistedExpressionJson
                                styleKey: workspace.pendingStyle
                                onExpressionEdited: expressionJson => {
                                    workspace.pendingExpressionJson = expressionJson;
                                    persistTimer.restart();
                                }
                            }

                            Text {
                                text: qsTr("STYLE")
                                color: Theme.muted
                                font.pixelSize: Theme.fontMicro
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.6
                            }

                            Flow {
                                Layout.fillWidth: true
                                Layout.preferredHeight: childrenRect.height
                                spacing: 6

                                Repeater {
                                    model: ["natural", "concise", "professional", "literary", "casual"]
                                    delegate: ChoiceChip {
                                        required property string modelData
                                        objectName: "textStyle-" + modelData
                                        text: workspace.styleLabel(modelData)
                                        current: workspace.pendingStyle === modelData
                                        onClicked: workspace.chooseStyle(modelData)
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.border
                    }

                    Text {
                        Layout.fillWidth: true
                        text: workspace.generationErrorText().length > 0 ? workspace.generationErrorText() : !workspace.hasAcceptedRevision ? qsTr("Connect a text, image, audio, or file material before generating.") : workspace.operatorDraftId.length === 0 ? qsTr("Start a new draft to edit this saved result again.") : !workspace.runtimeCompatible ? qsTr("AI runtime is not ready.") : !workspace.credentialConfigured ? qsTr("Add an Infer credential in Settings.") : qsTr("The source remains unchanged until you choose a generated version.")
                        color: workspace.generationErrorText().length > 0 ? Theme.danger : Theme.muted
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WordWrap
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: qsTr("Versions")
                            color: Theme.muted
                            font.pixelSize: 10
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        ChoiceChip {
                            objectName: "generateOneCandidateButton"
                            implicitWidth: 42
                            text: "1"
                            current: workspace.pendingVariantCount === 1
                            onClicked: {
                                workspace.pendingVariantCount = 1;
                                persistTimer.restart();
                            }
                        }
                        ChoiceChip {
                            objectName: "generateThreeCandidatesButton"
                            implicitWidth: 42
                            text: "3"
                            current: workspace.pendingVariantCount === 3
                            onClicked: {
                                workspace.pendingVariantCount = 3;
                                persistTimer.restart();
                            }
                        }
                    }

                    ShapeButton {
                        objectName: "aiTextGenerateButton"
                        Layout.fillWidth: true
                        primary: true
                        iconSource: workspace.operatorDraftId.length === 0 ? "qrc:/qt/qml/Shape/Desktop/icons/edit.svg" : workspace.credentialConfigured ? "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg" : "qrc:/qt/qml/Shape/Desktop/icons/settings.svg"
                        text: workspace.primaryActionText()
                        enabled: workspace.canUsePrimaryAction
                        onClicked: workspace.triggerPrimaryAction()
                    }
                }
            }
        }
    }

    Component.onCompleted: {
        pendingMode = normalizeMode(persistedMode);
        pendingExpressionJson = normalizeExpressionJson(persistedExpressionJson);
        pendingStyle = normalizeStyle(persistedStyle);
        pendingVariantCount = persistedVariantCount === 3 ? 3 : 1;
        observedCandidateCount = candidates.length;
    }
}

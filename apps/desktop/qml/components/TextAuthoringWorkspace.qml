pragma ComponentBehavior: Bound
//! Empty-first authoring, grammar review and explicit text-to-speech handoff.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: workspace
    objectName: "textAuthoringWorkspace"
    required property DesktopBackend backend
    property string artifactId: ""
    property string artifactName: ""
    property string inputArtifactId: ""
    property string inputRevisionId: ""
    readonly property bool editing: inputArtifactId.length > 0
    readonly property var inputArtifact: backend.artifacts.find(a => a.id === inputArtifactId) || null
    readonly property bool inputCurrent: !editing || (inputArtifact !== null && inputArtifact.acceptedRevisionId === inputRevisionId)
    property bool showExpression: false
    property string expressionJson: ""
    property string style: "natural"
    property string mode: "rewrite"
    property string draftId: ""
    property string draftJson: ""
    property string selectedCandidateId: ""
    property bool hasAcceptedRevision: false
    property bool generationRunning: false
    property bool runtimeCompatible: false
    property bool credentialConfigured: false
    property string generationErrorCode: ""
    property string defaultTextModel: "local_qwen"
    property string modelOverride: ""
    readonly property string selectedModelKey: modelPicker.effectiveModelKey
    property bool initialized: false
    property bool loading: false
    property bool dirty: false
    property bool saveFailed: false
    property bool reviewing: false
    property bool showReference: false
    property bool showSource: false
    property string profile: "plain"
    // Only retained when a legacy requirement cannot fit in the instruction buffer.
    property string unmigratedExample: ""
    property string entry: "generate"
    property string outputText: ""
    property string repairFeedback: ""
    property var preview: ({valid: false, plan: {events: [], issues: []}})
    readonly property bool scriptMode: profile !== "plain"
    readonly property string reviewText: entry === "manual" ? manualEditor.text : outputText
    readonly property bool canGenerate: inputCurrent && !generationRunning && runtimeCompatible && credentialConfigured
                                         && (editing ? true : entry === "adapt" ? materialEditor.text.trim().length > 0
                                             : instructionEditor.text.trim().length > 0 || materialEditor.text.trim().length > 0)
    signal generationRequested(string artifactId, string draftId, string modelKey)
    signal candidateSelected(string candidateId)
    signal speechRequested(string artifactId)
    signal setupRequested()

    function utf8Length(value) : int {
        return encodeURIComponent(value).replace(/%[0-9A-Fa-f]{2}/g, "x").length
    }

    function restoreDraft() : void {
        if (!initialized || dirty || draftJson.length === 0) return
        const saved = JSON.parse(draftJson)
        loading = true
        profile = saved.profile === "plain" ? "plain" : "script"
        const oldExample = saved.profile === "listening" ? "listening"
                           : saved.profile === "dialogue" ? "dialogue" : saved.example || "general"
        unmigratedExample = ""
        mode = saved.mode || "rewrite"
        expressionJson = JSON.stringify(saved.expression || {tones: [{kind: "preset", preset: "neutral"}], intensity: "balanced", audience: {kind: "preset", preset: "general"}})
        style = saved.style || "natural"
        entry = saved.entry
        let instruction = saved.instruction || ""
        const legacySettings = saved.profile !== "plain" && saved.entry !== "manual"
                               && saved.repeat_count !== undefined
        const migrateExample = saved.profile !== "plain" && saved.entry !== "manual"
                               && oldExample !== "general"
                               && (instruction.trim().length > 0 || (saved.material || "").trim().length > 0 || saved.entry === "adapt")
        const requirements = []
        if (migrateExample)
            requirements.push(oldExample === "listening" ? qsTr("Create a listening exercise.") : qsTr("Write a dialogue."))
        if (legacySettings) {
            if ((saved.cast || "").trim().length > 0)
                requirements.push(qsTr("Use these roles: %1.").arg(saved.cast))
            requirements.push(qsTr("Use overall delivery: %1.").arg(saved.delivery || "neutral"))
            if (saved.profile === "listening") {
                requirements.push(qsTr("For each question, repeat %1 times with %2 seconds between plays; pause %3 seconds afterwards.")
                                  .arg(saved.repeat_count).arg(saved.gap_seconds ?? 2).arg(saved.pause_seconds))
                if (saved.answer_beep)
                    requirements.push(qsTr("Declare a beep cue and play it before that pause."))
            }
        }
        const imported = requirements.length > 0
                         ? (instruction.trim().length > 0 ? "\n\n" : "")
                           + qsTr("Imported requirements from this older draft:") + "\n"
                           + requirements.join(" ") : ""
        if (utf8Length(instruction + imported) <= 4096)
            instruction += imported
        else {
            if (legacySettings) instruction += imported // Keep the existing save failure visible instead of discarding old settings.
            else if (migrateExample) unmigratedExample = oldExample
        }
        instructionEditor.text = instruction
        repairFeedback = saved.repair_feedback || ""
        materialEditor.text = saved.material
        manualEditor.text = saved.text
        showReference = saved.material.length > 0
        loading = false
        if (legacySettings || migrateExample || oldExample !== "general") changed()
        refreshOutput()
    }
    function stateJson() : string {
        const state = {profile: profile, mode: mode, expression: JSON.parse(expressionJson), style: style, entry: entry,
            instruction: instructionEditor.text, repair_feedback: repairFeedback, material: materialEditor.text,
            text: manualEditor.text}
        if (unmigratedExample.length > 0) state.example = unmigratedExample
        return JSON.stringify(state)
    }
    function changed() : void {
        if (!initialized || loading) return
        dirty = true
        saveTimer.restart()
        previewTimer.restart()
    }
    function checkpoint() : bool {
        saveTimer.stop()
        if (!dirty) return !saveFailed
        if (draftId.length === 0) return false
        const saved = backend.updateTextAuthoring(draftId, stateJson())
        saveFailed = !saved
        if (saved) dirty = false
        return saved
    }
    function refreshOutput() : void {
        if (!initialized || artifactId.length === 0) return
        const candidateExists = backend.candidates.some(c => c.id === selectedCandidateId && c.hasTextPreview)
        outputText = backend.textAuthoringContent(artifactId, candidateExists ? selectedCandidateId : "")
        if (selectedCandidateId.length > 0) reviewing = true
        previewTimer.restart()
    }
    function updatePreview() : void {
        const result = backend.textAuthoringConfiguredPreview(stateJson(), reviewText)
        preview = result.length > 0 ? JSON.parse(result) : {valid: false, plan: {events: [], issues: []}}
    }
    function generate() : void {
        const modelKey = selectedModelKey
        if (canGenerate && checkpoint()) generationRequested(artifactId, draftId, modelKey)
    }
    function editOutput() : void {
        const textToEdit = reviewText
        manualEditor.text = textToEdit
        entry = "manual"
        reviewing = false
        changed()
        manualEditor.focusEditor()
    }
    function repairScript() : void {
        materialEditor.text = reviewText
        repairFeedback = JSON.stringify((preview.plan?.issues || []).slice(0, 32).map(issue => ({line: issue.line, code: issue.code})))
        entry = "adapt"
        showReference = true
        reviewing = false
        changed()
    }
    function adopt(goToSpeech) : void {
        updatePreview()
        if (!inputCurrent || !preview.valid || !checkpoint()) return
        let candidateId = selectedCandidateId
        if (entry === "manual") {
            if (!backend.proposeAuthoredText(draftId)) return
            candidateId = backend.candidateId
        }
        if (candidateId.length > 0 && !backend.acceptCandidate(candidateId)) return
        candidateSelected("")
        refreshOutput()
        reviewing = true
        if (goToSpeech) speechRequested(artifactId)
    }
    onDraftJsonChanged: restoreDraft()
    onSelectedCandidateIdChanged: refreshOutput()
    onHasAcceptedRevisionChanged: refreshOutput()
    onReviewTextChanged: previewTimer.restart()
    Component.onCompleted: { initialized = true; restoreDraft(); reviewing = hasAcceptedRevision || selectedCandidateId.length > 0 }
    Component.onDestruction: { if (dirty && backend.projectOpen) checkpoint() }
    Timer { id: saveTimer; interval: 600; onTriggered: workspace.checkpoint() }
    Timer { id: previewTimer; interval: 180; onTriggered: workspace.updatePreview() }
    Connections {
        target: workspace.backend
        function onProjectChanged() : void { workspace.refreshOutput() }
    }
    SpeechScriptHelpDialog { id: helpDialog }

    ScrollView {
        id: writingScroll
        objectName: "writingScroll"
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ScrollBar.vertical.policy: contentHeight > availableHeight ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
        ScrollBar.vertical.interactive: true
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        ColumnLayout {
            width: writingScroll.availableWidth
            spacing: 0
            Item { Layout.preferredHeight: 22 }
            ColumnLayout {
                Layout.fillWidth: true
                objectName: "writingForm"
                Layout.minimumWidth: 0
                Layout.maximumWidth: Math.min(920, writingScroll.availableWidth - 48)
                Layout.alignment: Qt.AlignHCenter
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                spacing: 18
                RowLayout {
                    Layout.fillWidth: true
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 5
                        Label { text: workspace.editing ? qsTr("Text editing") : qsTr("Text creation"); color: Theme.accent; font.pixelSize: 12 }
                        Label { text: workspace.artifactName; color: Theme.text; font.pixelSize: 21; font.weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                    }
                    ShapeButton {
                        objectName: "writingScriptRulesButton"
                        text: qsTr("Format and examples")
                        quiet: true
                        onClicked: helpDialog.open()
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    ShapeButton { text: qsTr("Write"); selected: true; onClicked: workspace.reviewing = false }
                    Label { text: "›"; color: Theme.muted }
                    ShapeButton {
                        text: qsTr("Voices and audition")
                        quiet: true
                        enabled: workspace.hasAcceptedRevision && !workspace.generationRunning && !workspace.dirty
                        onClicked: workspace.speechRequested(workspace.artifactId)
                    }
                    Item { Layout.fillWidth: true }
                    Label { text: workspace.saveFailed ? qsTr("Not saved") : workspace.dirty ? qsTr("Saving…") : qsTr("Draft saved"); color: workspace.saveFailed ? Theme.danger : Theme.muted; font.pixelSize: 11 }
                }
                TextFormatPanel {
                    Layout.fillWidth: true
                    profile: workspace.profile
                    enabled: !workspace.generationRunning
                    onProfileSelected: profile => {
                        workspace.profile = profile
                        workspace.changed()
                    }
                    onGuideRequested: helpDialog.open()
                }
                Label {
                    visible: workspace.scriptMode && !workspace.reviewing
                    Layout.fillWidth: true
                    text: qsTr("The script controls playback. Declare roles and cues before spoken text; place pauses, delivery changes and repeat blocks where they belong.")
                    color: Theme.muted; font.pixelSize: 12; wrapMode: Text.WordWrap
                }
                Rectangle {
                    visible: workspace.editing
                    Layout.fillWidth: true
                    implicitHeight: inputLayout.implicitHeight + 28
                    radius: Theme.radiusMedium
                    color: Theme.surface
                    border.color: workspace.inputCurrent ? Theme.border : Theme.danger
                    ColumnLayout {
                        id: inputLayout
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 8
                        Label { text: qsTr("Original") + " · " + (workspace.inputArtifact ? workspace.inputArtifact.name : qsTr("Missing source")); color: Theme.text; font.weight: Font.DemiBold }
                        Label { Layout.fillWidth: true; text: workspace.inputCurrent ? qsTr("This node produces a separate document. The original stays available for other branches.") : qsTr("The original changed. Update the input before generating or adopting a result."); color: Theme.muted; wrapMode: Text.WordWrap }
                        ShapeButton { visible: !workspace.inputCurrent && workspace.inputArtifact !== null; text: qsTr("Use latest original"); onClicked: workspace.backend.refreshTextInput(workspace.draftId) }
                        ShapeButton { text: workspace.showSource ? qsTr("Hide original") : qsTr("View original"); quiet: true; onClicked: workspace.showSource = !workspace.showSource }
                        ShapeTextEditor {
                            visible: workspace.showSource
                            readOnly: true
                            Layout.fillWidth: true
                            Layout.preferredHeight: 160
                            text: workspace.inputRevisionId.length > 0 ? workspace.backend.textNodeInput(workspace.draftId) : ""
                        }
                        ShapeComboBox {
                            objectName: "textEditMode"
                            property var keys: ["rewrite", "translate", "summarize", "polish", "expand", "outline", "prepare_script"]
                            model: [qsTr("Rewrite"), qsTr("Translate"), qsTr("Summarize"), qsTr("Polish"), qsTr("Expand"), qsTr("Outline"), qsTr("Prepare a script")]
                            Layout.fillWidth: true
                            currentIndex: keys.indexOf(workspace.mode)
                            onActivated: index => {
                                workspace.mode = keys[index]
                                if (workspace.mode === "prepare_script" && !workspace.scriptMode) workspace.profile = "script"
                                workspace.changed()
                            }
                        }
                    }
                }
                RowLayout {
                    visible: !workspace.reviewing
                    Layout.fillWidth: true
                    spacing: 8
                    Repeater {
                        model: workspace.editing ? [{key: "adapt", label: qsTr("Let AI edit")}, {key: "manual", label: qsTr("Edit manually")}] : [{key: "generate", label: qsTr("Let AI write")}, {key: "adapt", label: qsTr("Adapt pasted text")}, {key: "manual", label: qsTr("Write from scratch")}]
                        delegate: ShapeButton {
                            required property var modelData
                            objectName: "writingMode_" + modelData.key
                            text: modelData.label
                            selected: workspace.entry === modelData.key
                            quiet: !selected
                            enabled: !workspace.generationRunning
                            onClicked: {
                                if (modelData.key === "manual" && workspace.editing && manualEditor.text.length === 0)
                                    manualEditor.text = workspace.backend.textNodeInput(workspace.draftId)
                                workspace.entry = modelData.key; workspace.changed()
                            }
                        }
                    }
                    Item { Layout.fillWidth: true }
                }
                AiModelPicker {
                    id: modelPicker
                    objectName: "writingModelPicker"
                    visible: !workspace.reviewing && workspace.entry !== "manual"
                    family: "text"
                    defaultModelKey: workspace.defaultTextModel
                    overrideKey: workspace.modelOverride
                    enabled: !workspace.generationRunning
                    onChoiceSelected: key => workspace.modelOverride = key
                }
                ColumnLayout {
                    visible: !workspace.reviewing
                    Layout.fillWidth: true
                    spacing: 12
                    Label {
                        text: workspace.entry === "manual" ? qsTr("Your draft") : workspace.entry === "adapt" ? qsTr("What should change? (optional)") : qsTr("Tell AI what you want to write")
                        color: Theme.text
                        font.pixelSize: 13
                    }
                    ShapeTextEditor {
                        id: instructionEditor
                        objectName: "writingInstructionEditor"
                        visible: workspace.entry !== "manual"
                        Layout.fillWidth: true
                        Layout.preferredHeight: 150
                        enabled: !workspace.generationRunning
                        placeholderText: qsTr("Describe the subject, audience, length and what you want to say…")
                        onTextChanged: workspace.changed()
                    }
                    ShapeTextEditor {
                        id: manualEditor
                        objectName: "writingManualEditor"
                        visible: workspace.entry === "manual"
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.max(220, Math.min(380, workspace.height * 0.52))
                        enabled: !workspace.generationRunning
                        placeholderText: workspace.scriptMode ? qsTr("Write spoken lines here. Use Format and examples whenever you need a role, pause or audio cue.") : qsTr("Start writing whenever you are ready…")
                        onTextChanged: workspace.changed()
                    }
                    RowLayout {
                        visible: workspace.entry !== "manual"
                        Layout.fillWidth: true
                        ShapeButton {
                            text: qsTr("Add reference text")
                            quiet: true
                            enabled: !workspace.generationRunning
                            onClicked: workspace.showReference = !workspace.showReference
                        }
                        Item { Layout.fillWidth: true }
                    }
                    Label { visible: (!workspace.editing && workspace.entry === "adapt") || workspace.showReference; text: qsTr("Reference text"); color: Theme.text; font.pixelSize: 13 }
                    ShapeTextEditor {
                        id: materialEditor
                        objectName: "writingMaterialEditor"
                        visible: (!workspace.editing && workspace.entry === "adapt") || workspace.showReference
                        Layout.fillWidth: true
                        Layout.preferredHeight: 200
                        enabled: !workspace.generationRunning
                        placeholderText: qsTr("Paste ordinary text, vocabulary or existing questions. AI will apply the script format.")
                        onTextChanged: workspace.changed()
                    }
                    Label {
                        visible: workspace.scriptMode
                        Layout.fillWidth: true
                        text: qsTr("Shape applies the script rules automatically. You can review and edit every spoken line before choosing voices.")
                        color: Theme.muted
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                    }
                }
                ShapeButton {
                    visible: !workspace.reviewing
                    text: qsTr("Tone, audience and style")
                    quiet: true
                    onClicked: workspace.showExpression = !workspace.showExpression
                }
                TextExpressionPalette {
                    visible: workspace.showExpression && !workspace.reviewing
                    Layout.fillWidth: true
                    persistedExpressionJson: workspace.expressionJson
                    styleKey: workspace.style
                    onExpressionEdited: json => { workspace.expressionJson = json; workspace.changed() }
                    onStyleKeyChanged: { workspace.style = styleKey; workspace.changed() }
                }
                ColumnLayout {
                    visible: workspace.reviewing
                    Layout.fillWidth: true
                    spacing: 12
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: workspace.selectedCandidateId.length > 0 ? qsTr("Candidate · not adopted") : qsTr("Current text"); color: Theme.muted; Layout.fillWidth: true }
                        ShapeButton { text: qsTr("Read"); selected: !workspace.showSource; onClicked: workspace.showSource = false }
                        ShapeButton { text: qsTr("Script source"); selected: workspace.showSource; onClicked: workspace.showSource = true }
                    }
                    ShapeTextEditor {
                        visible: workspace.showSource || !workspace.scriptMode
                        readOnly: true
                        text: workspace.reviewText
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.max(220, Math.min(420, workspace.height * 0.6))
                    }
                    SpeechScriptReadingView {
                        visible: !workspace.showSource && workspace.scriptMode
                        Layout.fillWidth: true
                        preview: workspace.preview
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        ShapeButton { text: qsTr("Adjust writing request"); enabled: !workspace.generationRunning; onClicked: workspace.reviewing = false }
                        ShapeButton { text: qsTr("Edit this text"); enabled: !workspace.generationRunning; onClicked: workspace.editOutput() }
                        ShapeButton { visible: workspace.scriptMode && !workspace.preview.valid; text: qsTr("Prepare AI format repair"); enabled: !workspace.generationRunning; onClicked: workspace.repairScript() }
                        Item { Layout.fillWidth: true }
                    }
                }
                Label {
                    visible: !workspace.runtimeCompatible || !workspace.credentialConfigured
                    Layout.fillWidth: true
                    text: qsTr("AI writing needs a connected Infer Runtime. You can still write and save a draft.")
                    color: Theme.muted
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }
                ShapeButton { visible: !workspace.credentialConfigured; text: qsTr("Set up AI access"); onClicked: workspace.setupRequested() }
                Label {
                    visible: workspace.generationErrorCode.length > 0
                    Layout.fillWidth: true
                    text: workspace.generationErrorCode === "stale_candidate"
                          ? qsTr("The text changed while AI was writing. Generate again using the current draft.")
                          : qsTr("AI could not finish this draft. Your request is saved; try again or write manually.")
                    color: Theme.danger
                    wrapMode: Text.WordWrap
                }
                Label { visible: workspace.backend.lastError.length > 0; text: workspace.backend.lastError; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.danger }
                Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }
                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        font.pixelSize: 12
                        color: Theme.muted
                        text: workspace.generationRunning ? qsTr("AI is writing. This may take a moment…")
                            : workspace.entry === "manual" || workspace.reviewing
                              ? workspace.preview.valid ? qsTr("Ready to adopt") : qsTr("Add spoken text and check the script instructions before continuing.")
                              : qsTr("Generated text remains a candidate until you adopt it.")
                    }
                    ShapeButton {
                        objectName: "writingGenerateButton"
                        visible: !workspace.reviewing && workspace.entry !== "manual"
                        text: workspace.entry === "adapt" ? qsTr("Adapt text") : workspace.scriptMode ? qsTr("Generate script") : qsTr("Generate text")
                        primary: true
                        busy: workspace.generationRunning
                        enabled: workspace.canGenerate
                        onClicked: workspace.generate()
                    }
                    ShapeButton {
                        objectName: "writingPreviewButton"
                        visible: !workspace.reviewing && workspace.entry === "manual"
                        text: qsTr("Preview")
                        enabled: manualEditor.text.trim().length > 0
                        onClicked: { workspace.updatePreview(); workspace.reviewing = true }
                    }
                    ShapeButton {
                        objectName: "writingAdoptButton"
                        visible: workspace.reviewing || workspace.entry === "manual"
                        text: workspace.scriptMode ? qsTr("Adopt script and choose voices") : qsTr("Adopt text")
                        primary: true
                        enabled: workspace.preview.valid === true && !workspace.generationRunning
                        onClicked: workspace.adopt(workspace.scriptMode)
                    }
                }
            }
            Item { Layout.preferredHeight: 24 }
        }
    }
}

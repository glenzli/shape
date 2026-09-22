pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import QtQuick.Window
import Shape.Desktop
import "components"

ApplicationWindow {
    id: window

    required property DesktopBackend backend
    required property InferRuntimeController inferRuntime
    required property InferTextController inferText
    required property InferSpeechController inferSpeech
    required property InferImageController inferImage
    required property AudioPreviewController audioPreview
    required property AudioExportController audioExport
    required property UiPreferences uiPreferences
    required property RecentProjects recentProjects

    property int selectedArtifactIndex: 0
    property string selectedCandidateId: ""
    property string imageBatchAutofocusArtifactId: ""
    property bool compareMode: false
    property string workbenchSection: "scenes"
    readonly property var selectedArtifact: window.backend.artifacts.length > selectedArtifactIndex
                                            ? window.backend.artifacts[selectedArtifactIndex]
                                            : null
    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    readonly property var artifactCandidates: hasSelectedArtifact
                                              ? candidatesForArtifact(selectedArtifact.id) : []
    readonly property var artifactDrafts: hasSelectedArtifact
                                          ? draftsForArtifact(selectedArtifact.id) : []
    readonly property var selectedCandidate: {
        for (let index = 0; index < artifactCandidates.length; ++index) {
            if (artifactCandidates[index].id === selectedCandidateId) {
                return artifactCandidates[index]
            }
        }
        return artifactCandidates.length > 0 ? artifactCandidates[0] : null
    }
    readonly property bool candidateForSelected: selectedCandidate !== null
    readonly property var railAssets: {
        const matches = []
        for (let index = 0; index < backend.artifacts.length; ++index) {
            const artifact = backend.artifacts[index]
            if (artifact.kindKey === "image_raster"
                    || artifact.kindKey === "audio_clip"
                    || artifact.kindKey === "image_composite") {
                matches.push(artifact)
            }
        }
        return matches
    }

    function candidatesForArtifact(artifactId) : var {
        const matches = []
        for (let index = 0; index < backend.candidates.length; ++index) {
            if (backend.candidates[index].contextArtifactId === artifactId) {
                matches.push(backend.candidates[index])
            }
        }
        return matches
    }

    function startTextAuthoring(preset) : void {
        if (preset !== "plain" && preset !== "script") return
        const name = preset === "script" ? qsTr("Untitled production script") : qsTr("Untitled text")
        if (!backend.createTextAuthoring(name, preset)) return
        selectedArtifactIndex = Math.max(0, backend.artifactCount - 1)
        selectedCandidateId = ""
        compareMode = false
        Qt.callLater(function() {
            if (!window.hasSelectedArtifact) return
            const drafts = window.draftsForArtifact(window.selectedArtifact.id)
            if (drafts.length > 0) workspaceSurface.openOperatorDraft(drafts[0].id)
        })
    }

    function openAuthoringSpeech(artifactId) : void {
        openDraftTarget(backend.beginAuthoringSpeech(artifactId))
    }

    function openProject(projectUrl) : void {
        if (!window.backend.openProject(projectUrl)) return
        window.selectedArtifactIndex = 0
        window.selectedCandidateId = ""
        window.compareMode = false
        workspaceSurface.showGraph()
    }

    function returnToWriting(artifactId, scriptMode) : void {
        const index = artifactIndex(artifactId)
        if (index < 0) return
        const draftId = backend.beginTextAuthoring(artifactId, scriptMode ? "script" : "plain")
        if (draftId.length === 0) return
        openDraftTarget(draftId)
    }

    function draftsForArtifact(artifactId) : var {
        const matches = []
        for (let index = 0; index < backend.operatorDrafts.length; ++index) {
            if (backend.operatorDrafts[index].contextArtifactId === artifactId) {
                matches.push(backend.operatorDrafts[index])
            }
        }
        return matches
    }

    function artifactIndex(artifactId) : int {
        for (let index = 0; index < backend.artifacts.length; ++index) {
            if (backend.artifacts[index].id === artifactId) return index
        }
        return -1
    }

    function openDraftTarget(draftId) : void {
        const draft = backend.operatorDrafts.find(d => d.id === draftId)
        if (!draft) return
        selectedArtifactIndex = artifactIndex(draft.contextArtifactId)
        selectedCandidateId = ""
        compareMode = false
        Qt.callLater(() => workspaceSurface.openOperatorDraft(draftId))
    }

    function addOrOpenTextEditorNode() : void {
        if (!hasSelectedArtifact || !selectedArtifact.hasAcceptedRevision
                || selectedArtifact.kindKey !== "text_document") return
        openDraftTarget(backend.beginOperatorDraft(selectedArtifact.id, "text.edit"))
    }

    function activateCandidate(candidateId) : bool {
        if (backend.selectCandidate(candidateId)) {
            selectedCandidateId = candidateId
            return true
        }
        return false
    }

    function reviewCandidate(candidateId) : void {
        if (activateCandidate(candidateId)) {
            compareMode = true
            workspaceSurface.showArtifact()
        }
    }

    function toggleComparison() : void {
        if (!candidateForSelected) {
            return
        }
        compareMode = !compareMode
        if (compareMode) {
            workspaceSurface.showArtifact()
        }
    }

    function discardCandidate(candidateId) : void {
        if (backend.discardCandidate(candidateId)) compareMode = false
    }

    function acceptCandidate(candidateId) : void {
        if (!activateCandidate(candidateId) || selectedCandidate === null) return
        const targetArtifactId = selectedCandidate.artifactId
        const contextArtifactId = selectedCandidate.contextArtifactId
        if (!backend.acceptCandidate(candidateId)) return
        compareMode = false
        if (targetArtifactId.length > 0 && targetArtifactId !== contextArtifactId) {
            const acceptedIndex = artifactIndex(targetArtifactId)
            if (acceptedIndex >= 0) {
                selectedArtifactIndex = acceptedIndex
                workspaceSurface.showGraph()
            }
        }
    }

    function openCandidateBranch(candidateId) : void {
        if (!activateCandidate(candidateId) || selectedArtifact === null) return
        branchDialog.openFor(selectedArtifact.name, candidateId)
    }

    function refreshSelectedImage() : void {
        if (!hasSelectedArtifact || selectedArtifact.kindKey !== "image_raster") {
            return
        }
        const imageCandidateId = selectedCandidate !== null
                                 && selectedCandidate.hasImagePreview
                                 ? selectedCandidate.id : ""
        if (!selectedArtifact.hasAcceptedRevision && imageCandidateId.length === 0) return
        backend.prepareImagePreviews(selectedArtifact.id, imageCandidateId)
    }

    function imageGenerationStatus(errorCode) : string {
        if (window.inferImage.running) {
            if (window.inferImage.stopRequested) {
                return qsTr("Finishing the current image, then stopping. %1 of %2 ready.")
                    .arg(window.inferImage.completedCount).arg(window.inferImage.requestedCount)
            }
            return window.inferImage.requestedCount > 1
                   ? qsTr("%1 of %2 image versions are ready.")
                       .arg(window.inferImage.completedCount).arg(window.inferImage.requestedCount)
                   : qsTr("AI is creating a new image version…")
        }
        if (errorCode === "generation_cancelled") {
            return qsTr("Stopped. %1 of %2 image versions are ready.")
                .arg(window.inferImage.completedCount).arg(window.inferImage.requestedCount)
        }
        if (errorCode === "partial_generation_failed") {
            return qsTr("%1 of %2 versions are ready. Review them before retrying.")
                .arg(window.inferImage.completedCount).arg(window.inferImage.requestedCount)
        }
        if (!window.inferText.credentialConfigured) {
            return qsTr("Add the Shape Infer credential in Settings before generating.")
        }
        if (errorCode === "intent_forbidden" || errorCode === "policy_violation") {
            return qsTr("Infer has not granted Shape cloud image generation permission yet.")
        }
        if (errorCode === "no_candidate") {
            return qsTr("Infer could not route this request to Codex Luna. Check Luna availability and Shape's cloud text permission in Infer Console, then retry.")
        }
        if (errorCode === "provider_unavailable" || errorCode === "upstream_unavailable") {
            return qsTr("No image generation provider is available right now.")
        }
        if (errorCode.length > 0) {
            return qsTr("Image generation failed safely: %1").arg(errorCode)
        }
        return qsTr("Each run creates a reviewable version. Your chosen result changes only when you use it.")
    }

    function synchronizeSelectedArtifact() : void {
        compareMode = false
        const candidates = hasSelectedArtifact ? candidatesForArtifact(selectedArtifact.id) : []
        selectedCandidateId = candidates.length > 0 ? candidates[0].id : ""
        if (selectedCandidateId.length > 0) {
            backend.selectCandidate(selectedCandidateId)
        }
        Qt.callLater(workspaceSurface.synchronizeNodeSelection)
        Qt.callLater(window.refreshSelectedImage)
    }

    onSelectedArtifactIndexChanged: Qt.callLater(window.synchronizeSelectedArtifact)

    onSelectedCandidateIdChanged: Qt.callLater(window.refreshSelectedImage)

    width: 1440
    height: 900
    minimumWidth: 1120
    minimumHeight: 720
    visible: true
    title: Qt.platform.os === "osx" ? ""
           : window.backend.projectOpen
             ? qsTr("Shape — %1").arg(window.backend.projectName)
             : qsTr("Shape — No Project")
    flags: Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint
    color: Theme.background

    Binding {
        target: Theme
        property: "mode"
        value: window.uiPreferences.appearanceMode
    }

    Binding {
        target: Theme
        property: "systemDark"
        value: window.uiPreferences.dark
    }

    ShapeSettingsDialog {
        id: settingsDialog
        objectName: "shapeSettingsDialog"
        uiPreferences: window.uiPreferences
        inferText: window.inferText
    }

    AudioExportDialog {
        id: audioExportDialog
        controller: window.audioExport
    }

    CreateProjectDialog {
        id: createProjectDialog
        backend: window.backend
        onProjectCreated: {
            window.selectedArtifactIndex = 0
            window.selectedCandidateId = ""
            window.compareMode = false
            workspaceSurface.showGraph()
            Qt.callLater(createSceneTypeDialog.openForCreation)
        }
    }

    CreateSceneTypeDialog {
        id: createSceneTypeDialog
        onTextAuthoringRequested: profile => window.startTextAuthoring(profile)
        onAiImageSceneRequested: createAiImageSceneDialog.openForCreation()
        onImportImageRequested: imageImportDialog.open()
    }

    CreateTextSceneDialog {
        id: createTextSceneDialog
        backend: window.backend
        onSceneCreated: {
            window.selectedArtifactIndex = Math.max(0, window.backend.artifactCount - 1)
            window.selectedCandidateId = ""
            window.compareMode = false
            Qt.callLater(function() {
                if (!window.hasSelectedArtifact) {
                    workspaceSurface.showGraph()
                    return
                }
                const draftId = window.backend.beginOperatorDraft(
                    window.selectedArtifact.id, "text.edit")
                if (draftId.length > 0) window.openDraftTarget(draftId)
                else workspaceSurface.showGraph()
            })
        }
    }

    CreateAiImageSceneDialog {
        id: createAiImageSceneDialog
        backend: window.backend
        onSceneCreated: {
            window.selectedArtifactIndex = Math.max(0, window.backend.artifactCount - 1)
            window.selectedCandidateId = ""
            window.compareMode = false
            Qt.callLater(function() {
                const drafts = window.draftsForArtifact(window.selectedArtifact.id)
                if (drafts.length > 0) workspaceSurface.openOperatorDraft(drafts[0].id)
            })
        }
    }

    FolderDialog {
        id: projectOpenDialog
        title: qsTr("Open Shape project folder")
        onAccepted: window.openProject(selectedFolder)
    }

    FileDialog {
        id: imageImportDialog
        title: qsTr("Choose an image to work with")
        fileMode: FileDialog.OpenFile
        nameFilters: [qsTr("Images (*.png *.jpg *.jpeg)")]
        onAccepted: {
            if (window.backend.importRaster(selectedFile)) {
                window.selectedArtifactIndex = Math.max(0, window.backend.artifactCount - 1)
                window.compareMode = false
                workspaceSurface.showGraph()
                Qt.callLater(window.refreshSelectedImage)
            }
        }
    }

    MessageDialog {
        id: componentPreviewDialog
        title: qsTr("Components")
        text: qsTr("Reusable components are coming next. This section is reserved so larger projects can organize repeated creative work without changing today's project structure.")
        buttons: MessageDialog.Ok
    }

    BranchArtifactDialog {
        id: branchDialog
        backend: window.backend
        onBranchCreated: {
            window.selectedArtifactIndex = Math.max(0, window.backend.artifactCount - 1)
            window.compareMode = false
            workspaceSurface.showGraph()
        }
    }

    header: MainTitleBar {
        exportAvailable: !window.audioExport.running && ((window.selectedCandidate !== null && window.selectedCandidate.hasAudioPreview) || (window.hasSelectedArtifact && window.selectedArtifact.kindKey === "audio_clip" && window.selectedArtifact.hasAcceptedRevision))
        onExportRequested: {
            if (window.selectedCandidate !== null && window.selectedCandidate.hasAudioPreview)
                audioExportDialog.openForAudio(window.selectedArtifact.id, window.selectedCandidate.id)
            else if (window.hasSelectedArtifact)
                audioExportDialog.openForAudio(window.selectedArtifact.id, "")
        }
        hostWindow: window
        projectOpen: window.backend.projectOpen
        projectName: window.backend.projectName
        compareAvailable: window.candidateForSelected
        compareActive: window.compareMode
        onCompareRequested: window.toggleComparison()
        onSettingsRequested: settingsDialog.open()
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        WorkbenchProjectRail {
            Layout.minimumWidth: 216
            Layout.preferredWidth: 216
            Layout.maximumWidth: 216
            Layout.fillHeight: true
            projectOpen: window.backend.projectOpen
            projectName: window.backend.projectName
            scenes: window.backend.artifacts
            components: []
            assets: window.railAssets
            candidates: window.backend.candidates
            currentSection: window.workbenchSection
            selectedSceneId: window.hasSelectedArtifact ? window.selectedArtifact.id : ""
            selectedAssetId: window.hasSelectedArtifact ? window.selectedArtifact.id : ""
            onSectionRequested: sectionKey => window.workbenchSection = sectionKey
            onSceneSelected: (sceneId, index) => workspaceSurface.activateScene(index)
            onSceneOpened: (sceneId, index) => workspaceSurface.activateScene(index)
            onAssetSelected: (assetId, index) => {
                const targetIndex = window.artifactIndex(assetId)
                if (targetIndex >= 0) workspaceSurface.activateScene(targetIndex)
            }
            onAssetOpened: (assetId, index) => {
                const targetIndex = window.artifactIndex(assetId)
                if (targetIndex >= 0) workspaceSurface.activateScene(targetIndex)
            }
            onNewProjectRequested: createProjectDialog.openForCreation()
            onOpenProjectRequested: projectOpenDialog.open()
            onCreateSceneRequested: createSceneTypeDialog.openForCreation()
            onCreateComponentRequested: componentPreviewDialog.open()
            onImportAssetRequested: imageImportDialog.open()
        }

        ColumnLayout {
            Layout.minimumWidth: 500
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                WorkspaceSurface {
                    id: workspaceSurface

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    projectName: window.backend.projectName
                    allArtifacts: window.backend.artifacts
                    selectedArtifact: window.selectedArtifact
                    candidates: window.artifactCandidates
                    operatorDrafts: window.artifactDrafts
                    allOperatorDrafts: window.backend.operatorDrafts
                    operatorDescriptors: window.hasSelectedArtifact
                                         ? window.backend.compatibleOperators(
                                               window.selectedArtifact.id) : window.backend.compatibleOperators("")
                    selectedCandidate: window.selectedCandidate
                    selectedCandidateId: window.selectedCandidateId
                    defaultTextModel: window.uiPreferences.textModel
                    defaultTextEffort: window.uiPreferences.textEffort
                    compareMode: window.compareMode
                    acceptedImageSource: window.backend.acceptedImageSource
                    candidateImageSource: window.backend.candidateImageSource
                    inferImage: window.inferImage
                    backend: window.backend
                    inferSpeech: window.inferSpeech
                    audioPreview: window.audioPreview
                    projectPath: window.backend.bundlePath
                    inferCredentialConfigured: window.inferText.credentialConfigured
                    inferTextRunning: window.inferText.running
                    inferTextErrorCode: window.inferText.errorCode
                    inferRuntimeCompatible: window.inferRuntime.compatible
                    onSceneSelected: index => window.selectedArtifactIndex = index
                    onCandidateSelected: candidateId => window.activateCandidate(candidateId)
                    onCandidateReviewRequested: candidateId => window.reviewCandidate(candidateId)
                    onCropRequested: (artifactId, x, y, width, height) => {
                        if (window.backend.proposeRasterCrop(
                                artifactId, x, y, width, height)) {
                            window.selectedCandidateId = window.backend.candidateId
                            window.compareMode = true
                            window.backend.prepareImagePreviews(
                                artifactId, window.selectedCandidateId)
                        }
                    }
                    onResizeDraftSaveRequested: (draftId, targetWidth, targetHeight,
                                                 aspectPolicyKey, resamplingKey) => {
                        window.backend.updateImageResizeDraft(
                            draftId, targetWidth, targetHeight,
                            aspectPolicyKey, resamplingKey)
                    }
                    onResizeRequested: (artifactId, draftId, targetWidth, targetHeight,
                                        aspectPolicyKey, resamplingKey) => {
                        if (window.backend.updateImageResizeDraft(
                                draftId, targetWidth, targetHeight,
                                aspectPolicyKey, resamplingKey)
                                && window.backend.proposeRasterResize(artifactId, draftId)) {
                            window.selectedCandidateId = window.backend.candidateId
                            window.compareMode = true
                            window.backend.prepareImagePreviews(
                                artifactId, window.selectedCandidateId)
                        }
                    }
                    onRasterTransformRequested: (artifactId, transformKey) => {
                        if (window.backend.proposeRasterTransform(artifactId, transformKey)) {
                            window.selectedCandidateId = window.backend.candidateId
                            window.compareMode = true
                            window.backend.prepareImagePreviews(
                                artifactId, window.selectedCandidateId)
                        }
                    }
                    onRasterBlurRequested: (artifactId, radius) => {
                        if (window.backend.proposeRasterBlur(artifactId, radius)) {
                            window.selectedCandidateId = window.backend.candidateId
                            window.compareMode = true
                            window.backend.prepareImagePreviews(
                                artifactId, window.selectedCandidateId)
                        }
                    }
                    onRasterUnsharpMaskRequested: (artifactId, radius,
                                                   amountMilli, threshold) => {
                        if (window.backend.proposeRasterUnsharpMask(
                                artifactId, radius, amountMilli, threshold)) {
                            window.selectedCandidateId = window.backend.candidateId
                            window.compareMode = true
                            window.backend.prepareImagePreviews(
                                artifactId, window.selectedCandidateId)
                        }
                    }
                    onRasterDropShadowRequested: (artifactId, offsetX, offsetY,
                                                  radius, red, green, blue, alpha) => {
                        if (window.backend.proposeRasterDropShadow(
                                artifactId, offsetX, offsetY, radius,
                                red, green, blue, alpha)) {
                            window.selectedCandidateId = window.backend.candidateId
                            window.compareMode = true
                            window.backend.prepareImagePreviews(
                                artifactId, window.selectedCandidateId)
                        }
                    }
                    onSpeechDraftSaveRequested: (draftId, presetAlias,
                                                 presetCatalogRevision, language,
                                                 speedMilli,
                                                 syntheticDisclosureRequired) => {
                        window.backend.updateAudioSpeechDraft(
                            draftId, presetAlias, presetCatalogRevision,
                            language, speedMilli, syntheticDisclosureRequired)
                    }
                    onSpeechSynthesisRequested: (sourceArtifactId, draftId,
                                                 artifactName, presetAlias,
                                                 presetCatalogRevision, language,
                                                 speedMilli,
                                                 syntheticDisclosureRequired) => {
                        if (window.backend.updateAudioSpeechDraft(
                                draftId, presetAlias, presetCatalogRevision,
                                language, speedMilli,
                                syntheticDisclosureRequired)) {
                            window.inferSpeech.generate(
                                window.backend.bundlePath, sourceArtifactId,
                                draftId, artifactName)
                        }
                    }
                    onAiImageDraftSaveRequested: (draftId, instruction,
                                                  outputWidth, outputHeight, candidateCount) => {
                        window.backend.updateAiImageDraft(
                            draftId, instruction, outputWidth, outputHeight, candidateCount)
                    }
                    onOperatorDraftRequested: operatorTypeKey => {
                        if (operatorTypeKey === "text.create") { window.startTextAuthoring("plain"); return }
                        if (operatorTypeKey === "image.generate") { createAiImageSceneDialog.openForCreation(); return }
                        if (operatorTypeKey === "text.edit"
                                || operatorTypeKey === "text.transform") {
                            window.addOrOpenTextEditorNode()
                            return
                        }
                        if (!window.hasSelectedArtifact) return
                        const draftId = window.backend.beginOperatorDraft(
                            window.selectedArtifact.id, operatorTypeKey)
                        if (draftId.length > 0) window.openDraftTarget(draftId)
                    }
                    onOperatorDraftDiscardRequested: draftId => {
                        if (window.backend.discardOperatorDraft(draftId)) {
                            window.selectedArtifactIndex = Math.max(0, Math.min(
                                window.selectedArtifactIndex, window.backend.artifactCount - 1))
                            workspaceSurface.showGraph()
                        }
                    }
                    onTextStudioDraftSaveRequested: (draftId, modeKey, instruction,
                                                     expressionJson, styleKey, variantCount) => {
                        window.backend.updateTextTransformDraft(
                            draftId, modeKey, instruction, expressionJson,
                            styleKey, variantCount)
                    }
                    onTextStudioGenerationRequested: (artifactId, draftId, modeKey,
                                                       instruction, expressionJson, styleKey,
                                                       variantCount) => {
                        if (window.backend.updateTextTransformDraft(
                                draftId, modeKey, instruction, expressionJson,
                                styleKey, variantCount)) {
                            window.inferText.generate(
                                window.backend.bundlePath, artifactId, draftId,
                                window.uiPreferences.textModel,
                                window.uiPreferences.textModel === "local_qwen"
                                    ? "" : window.uiPreferences.textEffort)
                        }
                    }
                    onTextCandidateLockRequested: candidateId =>
                                                      window.acceptCandidate(candidateId)
                    onAuthoringGenerationRequested: (artifactId, draftId, modelKey, effortKey) =>
                        window.inferText.generate(window.backend.bundlePath, artifactId, draftId,
                                                  modelKey, effortKey)
                    onAuthoringSpeechRequested: artifactId => window.openAuthoringSpeech(artifactId)
                    onWritingRequested: (artifactId, scriptMode) => window.returnToWriting(artifactId, scriptMode)
                    onAudioExportRequested: (artifactId, candidateId) => audioExportDialog.openForAudio(artifactId, candidateId)
                    onInferAccessSetupRequested: settingsDialog.open()
                }

                OperatorIntentSidebar {
                    id: aiImageIntent
                    Layout.minimumWidth: visible ? 320 : 0
                    Layout.preferredWidth: visible ? 320 : 0
                    Layout.maximumWidth: visible ? 320 : 0
                    Layout.fillHeight: true
                    visible: workspaceSurface.aiImageIntentActive
                    defaultModelKey: window.uiPreferences.imageModel
                    defaultEffortKey: window.uiPreferences.imageEffort
                    modelContextId: workspaceSurface.selectedDraft !== null
                                    ? workspaceSurface.selectedDraft.id : ""
                    defaultCandidateCount: workspaceSurface.selectedDraft !== null
                                           ? workspaceSurface.selectedDraft.aiImageCandidateCount : 1
                    allowCandidateCount: true
                    operatorTitle: qsTr("Create an image")
                    operatorKindLabel: qsTr("STARTING POINT · AI GENERATED")
                    intentText: workspaceSurface.selectedDraft !== null
                                ? workspaceSurface.selectedDraft.aiImageInstruction : ""
                    intentPlaceholder: qsTr("Describe the image, composition, light, and mood…")
                    changeItems: workspaceSurface.selectedDraft !== null ? [
                        qsTr("Create a new image from your description"),
                        qsTr("Image size %1 × %2")
                            .arg(workspaceSurface.selectedDraft.aiImageOutputWidth)
                            .arg(workspaceSurface.selectedDraft.aiImageOutputHeight)
                    ] : []
                    preserveItems: [qsTr("Your chosen result until you use another version")]
                    references: []
                    allowReferences: false
                    referencesEmptyText: qsTr("This starting point does not need an existing image")
                    editable: window.backend.projectOpen
                    running: window.inferImage.running
                    stopRequested: window.inferImage.stopRequested
                    statusText: window.imageGenerationStatus(window.inferImage.errorCode)
                    primaryActionText: aiImageIntent.selectedCandidateCount > 1
                                       ? qsTr("Generate %1 versions").arg(aiImageIntent.selectedCandidateCount)
                                       : qsTr("Generate a version")
                    primaryActionEnabled: window.inferText.credentialConfigured
                                          && workspaceSurface.selectedDraft !== null
                                          && editedIntentText.trim().length > 0
                    onCandidateCountSelected: count => {
                        const draft = workspaceSurface.selectedDraft
                        if (draft !== null) {
                            window.backend.updateAiImageDraft(
                                draft.id, editedIntentText,
                                draft.aiImageOutputWidth, draft.aiImageOutputHeight, count)
                        }
                    }
                    onIntentCommitRequested: text => {
                        const draft = workspaceSurface.selectedDraft
                        if (draft !== null) {
                            window.backend.updateAiImageDraft(
                                draft.id, text,
                                draft.aiImageOutputWidth, draft.aiImageOutputHeight,
                                aiImageIntent.selectedCandidateCount)
                        }
                    }
                    onPrimaryActionRequested: {
                        const draft = workspaceSurface.selectedDraft
                        const modelKey = aiImageIntent.selectedModelKey
                        const effortKey = aiImageIntent.selectedEffortKey
                        if (draft !== null && window.backend.updateAiImageDraft(
                                draft.id, editedIntentText,
                                draft.aiImageOutputWidth, draft.aiImageOutputHeight,
                                aiImageIntent.selectedCandidateCount)) {
                            window.imageBatchAutofocusArtifactId = window.hasSelectedArtifact
                                    && window.selectedArtifact.id === draft.contextArtifactId
                                    && window.selectedCandidateId.length === 0
                                    ? draft.contextArtifactId : ""
                            window.inferImage.generate(
                                window.backend.bundlePath,
                                draft.contextArtifactId, draft.id,
                                modelKey, effortKey, aiImageIntent.selectedCandidateCount)
                        }
                    }
                    onStopActionRequested: window.inferImage.stopAfterCurrent()
                }
            }

            CandidateFilmstrip {
                Layout.fillWidth: true
                Layout.preferredHeight: visible ? 150 : 0
                visible: workspaceSurface.focusActive
                         && (!workspaceSurface.guidedAuthoringActive || window.artifactCandidates.length > 0)
                candidates: window.artifactCandidates
                selectedCandidateId: window.selectedCandidateId
                acceptedRevisionId: window.hasSelectedArtifact
                                    ? window.selectedArtifact.acceptedRevisionId : ""
                selectedPreviewSource: window.backend.candidateImageSource
                candidateThumbnailSource: candidateId => window.backend.candidateThumbnailSource(candidateId)
                mutationEnabled: window.backend.projectOpen && !window.inferImage.running
                compareAvailable: window.candidateForSelected
                onCandidateSelected: candidateId => window.activateCandidate(candidateId)
                onCandidateReviewRequested: candidateId => window.reviewCandidate(candidateId)
                onCompareRequested: candidateId => window.reviewCandidate(candidateId)
                onDiscardRequested: candidateId => window.discardCandidate(candidateId)
                onAcceptRequested: candidateId => window.acceptCandidate(candidateId)
                onBranchRequested: candidateId => window.openCandidateBranch(candidateId)
            }
        }

    }

    Connections {
        target: window.inferImage

        function onCandidateCreated(candidateId, artifactId) : void {
            if (window.inferImage.completedCount !== 1
                    || window.imageBatchAutofocusArtifactId !== artifactId
                    || !window.hasSelectedArtifact
                    || window.selectedArtifact.id !== artifactId) return
            window.imageBatchAutofocusArtifactId = ""
            window.activateCandidate(candidateId)
            window.compareMode = false
            window.backend.prepareImagePreviews(artifactId, candidateId)
        }
    }

    Connections {
        target: window.backend

        function onCandidateChanged() : void {
            if (window.selectedCandidate !== null
                    && window.selectedCandidateId !== window.selectedCandidate.id) {
                window.selectedCandidateId = window.selectedCandidate.id
            } else if (window.selectedCandidate === null
                       && window.selectedCandidateId.length > 0) {
                window.selectedCandidateId = ""
            }
            if (!window.candidateForSelected) {
                window.compareMode = false
            }
            Qt.callLater(window.refreshSelectedImage)
        }

        function onProjectChanged() : void {
            Qt.callLater(window.refreshSelectedImage)
        }

        function onOperatorDraftsChanged() : void {
            Qt.callLater(workspaceSurface.synchronizeNodeSelection)
        }
    }

    Connections {
        target: window.inferText

        function onCandidateCreated(candidateId, artifactId) : void {
            if (window.hasSelectedArtifact && window.selectedArtifact.id === artifactId) {
                window.reviewCandidate(candidateId)
            }
        }
    }

    Connections {
        target: window.inferSpeech

        function onCandidateCreated(candidateId, sourceArtifactId) : void {
            if (window.hasSelectedArtifact
                    && window.selectedArtifact.id === sourceArtifactId
                    && window.activateCandidate(candidateId)) {
                window.compareMode = true
                workspaceSurface.openSpeechWorkspace()
            }
        }
    }

    ProjectWelcome {
        anchors.fill: parent
        z: 100
        visible: !window.backend.projectOpen
        errorMessage: window.backend.lastError
        recentProjects: window.recentProjects.entries
        onNewProjectRequested: createProjectDialog.openForCreation()
        onOpenProjectRequested: projectOpenDialog.open()
        onRecentProjectRequested: projectUrl => window.openProject(projectUrl)
    }
}

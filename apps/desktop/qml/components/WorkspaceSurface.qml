pragma ComponentBehavior: Bound

//! Scene-graph-first navigation assembly. OperatorWorkspaceHost owns the
//! focused lifecycle and routing; graph and media owners keep their own state.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: surface
    objectName: "workspaceSurface"

    property string projectName: ""
    property var allArtifacts: []
    property var selectedArtifact: null
    property var candidates: []
    property var operatorDrafts: []
    property var operatorDescriptors: []
    property var selectedCandidate: null
    property string selectedCandidateId: ""
    property bool compareMode: false
    property string acceptedImageSource: ""
    property string candidateImageSource: ""
    required property InferImageController inferImage
    required property DesktopBackend backend
    required property InferSpeechController inferSpeech
    required property AudioPreviewController audioPreview
    property string projectPath: ""
    property bool inferCredentialConfigured: false
    property bool inferTextRunning: false
    property string inferTextErrorCode: ""
    property bool inferRuntimeCompatible: false
    property int currentMode: 0
    property string selectedNodeId: ""

    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    readonly property var graphNodes: hasSelectedArtifact
                                      ? (selectedArtifact.operatorNodes || []) : []
    readonly property var graphEdges: hasSelectedArtifact
                                      ? (selectedArtifact.operatorEdges || []) : []
    readonly property bool graphActive: currentMode === 0
    readonly property bool focusActive: currentMode === 1
    readonly property bool guidedAuthoringActive: focusActive && (authoringRoute
        || (workspaceRouteKey === "operator.audio.speech_synthesize"
            && speechDraftForArtifact(operatorWorkspaceHost.openedArtifactId) !== null))
    readonly property bool editWorkspaceActive: focusActive
                                                && operatorWorkspaceHost.openedRoleKey
                                                   === "operator"
                                                && operatorWorkspaceHost
                                                   .hasRegisteredOperatorWorkspace
    readonly property bool intentWorkspaceActive: editWorkspaceActive
                                                  && (workspaceRouteKey === "operator.text.edit"
                                                      || workspaceRouteKey
                                                         === "operator.text.transform")
    readonly property bool aiImageWorkspaceActive: focusActive
                                                   && workspaceRouteKey
                                                      === "operator.image.generate"
    readonly property bool aiImageIntentActive: aiImageWorkspaceActive
                                                && selectedDraft !== null
                                                && selectedDraft.operatorTypeKey
                                                   === "image.generate"
    readonly property string workspaceRouteKey: operatorWorkspaceHost.routeKey
    readonly property string loadedWorkspaceObjectName: operatorWorkspaceHost
                                                         .loadedWorkspaceObjectName
    readonly property bool candidateForSelected: hasSelectedArtifact
                                                  && selectedCandidate !== null
                                                  && selectedCandidate.contextArtifactId
                                                     === selectedArtifact.id
    readonly property var selectedNode: {
        for (let index = 0; index < graphNodes.length; ++index) {
            if (graphNodes[index].id === selectedNodeId) return graphNodes[index]
        }
        return null
    }
    readonly property var selectedDraft: {
        const draftId = selectedNodeId
        const drafts = operatorDrafts || []
        for (let index = 0; index < drafts.length; ++index) {
            if (drafts[index].id === draftId) return drafts[index]
        }
        return null
    }

    signal sceneSelected(int index)
    signal candidateSelected(string candidateId)
    signal candidateReviewRequested(string candidateId)
    signal cropRequested(string artifactId, int x, int y, int width, int height)
    signal resizeDraftSaveRequested(string draftId, int targetWidth, int targetHeight,
                                    string aspectPolicyKey, string resamplingKey)
    signal resizeRequested(string artifactId, string draftId,
                           int targetWidth, int targetHeight,
                           string aspectPolicyKey, string resamplingKey)
    signal rasterTransformRequested(string artifactId, string transformKey)
    signal rasterBlurRequested(string artifactId, int radius)
    signal rasterUnsharpMaskRequested(string artifactId, int radius,
                                      int amountMilli, int threshold)
    signal rasterDropShadowRequested(string artifactId, int offsetX, int offsetY,
                                     int blurRadius, int red, int green, int blue, int alpha)
    signal speechDraftSaveRequested(string draftId, string presetAlias,
                                    string presetCatalogRevision, string language,
                                    int speedMilli, bool syntheticDisclosureRequired)
    signal speechSynthesisRequested(string sourceArtifactId, string draftId,
                                    string artifactName, string presetAlias,
                                    string presetCatalogRevision, string language,
                                    int speedMilli, bool syntheticDisclosureRequired)
    signal aiImageDraftSaveRequested(string draftId, string instruction,
                                     int outputWidth, int outputHeight)
    signal operatorDraftRequested(string operatorTypeKey)
    signal operatorDraftDiscardRequested(string draftId)
    signal textStudioDraftSaveRequested(string draftId, string modeKey,
                                        string instruction, string expressionJson,
                                        string styleKey, int variantCount)
    signal textStudioGenerationRequested(string artifactId, string draftId,
                                         string modeKey, string instruction,
                                         string expressionJson, string styleKey,
                                         int variantCount)
    signal textCandidateLockRequested(string candidateId)
    signal inferAccessSetupRequested()
    signal authoringGenerationRequested(string artifactId, string draftId)
    signal authoringSpeechRequested(string artifactId)
    signal writingRequested(string artifactId, bool scriptMode)
    signal audioExportRequested(string artifactId, string candidateId)
    readonly property var openedTextDraft: textDraftForArtifact(operatorWorkspaceHost.openedArtifactId)
    readonly property bool authoringRoute: openedTextDraft !== null && (openedTextDraft.textAuthoringJson || "").length > 0

    function showArtifact() : bool {
        if (selectedCandidate !== null && selectedCandidate.hasAudioPreview) {
            return openSpeechWorkspace()
        }
        if (selectedCandidate !== null && selectedCandidate.hasImagePreview) {
            const drafts = operatorDrafts || []
            for (let index = 0; index < drafts.length; ++index) {
                if (drafts[index].operatorTypeKey === "image.generate") {
                    return openOperatorDraft(drafts[index].id)
                }
            }
        }
        synchronizeNodeSelection()
        if (selectedNode !== null && selectedNode.roleKey === "operator") {
            return openNode(selectedNode.id)
        }
        for (let index = graphNodes.length - 1; index >= 0; --index) {
            if (graphNodes[index].roleKey === "operator") {
                return openNode(graphNodes[index].id)
            }
        }
        return false
    }

    function artifactForId(artifactId) : var {
        for (let index = 0; index < allArtifacts.length; ++index) {
            if (allArtifacts[index].id === artifactId) return allArtifacts[index]
        }
        return null
    }

    function openSpeechWorkspace() : bool {
        if (!hasSelectedArtifact || selectedArtifact.kindKey !== "text_document"
                || !selectedArtifact.hasAcceptedRevision) {
            return false
        }
        const speechDraft = speechDraftForArtifact(selectedArtifact.id)
        const nodeId = speechDraft !== null
                       ? speechDraft.id
                       : "draft.audio.speech." + selectedArtifact.id
        if (!operatorWorkspaceHost.openWorkspace(
                nodeId,
                "operator", "audio.speech_synthesize", selectedArtifact.id,
                selectedArtifact.acceptedRevisionId, "")) {
            return false
        }
        currentMode = 1
        Qt.callLater(surface.refreshSpeechDraftProjection)
        return true
    }

    function showGraph() : void {
        operatorWorkspaceHost.closeWorkspace()
        currentMode = 0
    }

    function activateScene(index) : void {
        sceneSelected(index)
        showGraph()
    }

    function selectNode(nodeId) : void {
        selectedNodeId = nodeId
    }

    function nodeForId(nodeId) : var {
        for (let index = 0; index < graphNodes.length; ++index) {
            if (graphNodes[index].id === nodeId) return graphNodes[index]
        }
        return null
    }

    function draftForId(draftId) : var {
        const drafts = operatorDrafts || []
        for (let index = 0; index < drafts.length; ++index) {
            if (drafts[index].id === draftId) return drafts[index]
        }
        return null
    }

    function speechDraftForArtifact(artifactId) : var {
        const drafts = operatorDrafts || []
        for (let index = 0; index < drafts.length; ++index) {
            const draft = drafts[index]
            if (draft.contextArtifactId === artifactId
                    && draft.operatorTypeKey === "audio.speech_synthesize") {
                return draft
            }
        }
        return null
    }

    function textDraftForArtifact(artifactId) : var {
        const drafts = operatorDrafts || []
        for (let index = 0; index < drafts.length; ++index) {
            const draft = drafts[index]
            if (draft.contextArtifactId === artifactId
                    && (draft.operatorTypeKey === "text.create" || draft.operatorTypeKey === "text.edit"
                        || draft.operatorTypeKey === "text.transform")) {
                return draft
            }
        }
        return null
    }

    function imageResizeDraftForArtifact(artifactId) : var {
        const drafts = operatorDrafts || []
        for (let index = 0; index < drafts.length; ++index) {
            const draft = drafts[index]
            if (draft.contextArtifactId === artifactId
                    && draft.operatorTypeKey === "image.resize") return draft
        }
        return null
    }

    function openImageEditor() : bool {
        if (!hasSelectedArtifact || selectedArtifact.kindKey !== "image_raster"
                || !selectedArtifact.hasAcceptedRevision) return false
        selectedNodeId = "workspace.image.edit." + selectedArtifact.id
        if (!operatorWorkspaceHost.openWorkspace(
                selectedNodeId, "operator", "image.edit", selectedArtifact.id,
                selectedArtifact.acceptedRevisionId, "")) return false
        currentMode = 1
        return true
    }

    function projectSpeechDraft(workspace) : void {
        if (!workspace || workspace.objectName !== "audioSpeechOperatorWorkspace") {
            return
        }
        const draft = draftForId(operatorWorkspaceHost.openedNodeId)
        if (draft === null
                || draft.operatorTypeKey !== "audio.speech_synthesize") {
            workspace.operatorDraftId = ""
            workspace.presetAlias = ""
            workspace.presetCatalogRevision = ""
            workspace.language = ""
            workspace.speedMilli = 0
            workspace.syntheticDisclosureRequired = false
            return
        }
        workspace.operatorDraftId = draft.id
        workspace.scriptJson = draft.audioSpeechScriptJson
        workspace.presetAlias = draft.audioSpeechPresetAlias
        workspace.presetCatalogRevision = draft.audioSpeechPresetCatalogRevision
        workspace.language = draft.audioSpeechLanguage
        workspace.speedMilli = draft.audioSpeechSpeedMilli
        workspace.syntheticDisclosureRequired = draft.audioSpeechDisclosureRequired
    }

    function refreshSpeechDraftProjection() : void {
        projectSpeechDraft(operatorWorkspaceHost.loadedWorkspace)
    }

    function openNode(nodeId) : bool {
        if (draftForId(nodeId) !== null) return openOperatorDraft(nodeId)
        const node = nodeForId(nodeId)
        if (node === null) return false
        selectedNodeId = nodeId
        if (!operatorWorkspaceHost.openWorkspace(
                node.id, node.roleKey, node.operatorTypeKey,
                node.artifactId, node.revisionId, node.transformationId, node)) {
            return false
        }
        currentMode = 1
        return true
    }

    function openOperatorDraft(draftId) : bool {
        const draft = draftForId(draftId)
        if (draft === null || !hasSelectedArtifact
                || draft.contextArtifactId !== selectedArtifact.id) {
            return false
        }
        selectedNodeId = draftId
        if (!operatorWorkspaceHost.openWorkspace(
                draft.id, "operator", draft.operatorTypeKey,
                selectedArtifact.id, selectedArtifact.acceptedRevisionId, "")) {
            return false
        }
        currentMode = 1
        Qt.callLater(surface.refreshSpeechDraftProjection)
        return true
    }

    function synchronizeNodeSelection() : void {
        const nodes = graphNodes || []
        for (let index = 0; index < nodes.length; ++index) {
            if (nodes[index].id === selectedNodeId) return
        }
        if (draftForId(selectedNodeId) !== null) return
        selectedNodeId = ""
        for (let index = 0; index < nodes.length; ++index) {
            if (nodes[index].roleKey === "output") {
                selectedNodeId = nodes[index].id
                return
            }
        }
        const drafts = operatorDrafts || []
        if (drafts.length > 0) {
            selectedNodeId = drafts[0].id
            return
        }
        if (nodes.length > 0) selectedNodeId = nodes[0].id
    }

    onSelectedArtifactChanged: synchronizeNodeSelection()
    onGraphNodesChanged: synchronizeNodeSelection()
    onOperatorDraftsChanged: Qt.callLater(function() {
        surface.synchronizeNodeSelection()
        surface.refreshSpeechDraftProjection()
    })

    Connections {
        target: operatorWorkspaceHost

        function onWorkspaceLoaded(workspace) : void {
            surface.projectSpeechDraft(workspace)
        }

        function onOpenedNodeIdChanged() : void {
            surface.refreshSpeechDraftProjection()
        }
    }

    Component {
        id: textAuthoringWorkspace
        TextAuthoringWorkspace {
            backend: surface.backend
            artifactId: operatorWorkspaceHost.openedArtifactId
            artifactName: surface.hasSelectedArtifact ? surface.selectedArtifact.name : ""
            draftId: surface.openedTextDraft !== null ? surface.openedTextDraft.id : ""
            inputArtifactId: surface.openedTextDraft !== null ? (surface.openedTextDraft.inputArtifactId || "") : ""
            inputRevisionId: surface.openedTextDraft !== null ? (surface.openedTextDraft.inputRevisionId || "") : ""
            draftJson: surface.openedTextDraft !== null ? surface.openedTextDraft.textAuthoringJson : ""
            hasAcceptedRevision: surface.hasSelectedArtifact && surface.selectedArtifact.hasAcceptedRevision
            selectedCandidateId: surface.selectedCandidate !== null && surface.selectedCandidate.hasTextPreview ? surface.selectedCandidate.id : ""
            generationRunning: surface.inferTextRunning
            generationErrorCode: surface.inferTextErrorCode
            runtimeCompatible: surface.inferRuntimeCompatible
            credentialConfigured: surface.inferCredentialConfigured
            onGenerationRequested: (artifactId, draftId) => surface.authoringGenerationRequested(artifactId, draftId)
            onCandidateSelected: candidateId => surface.candidateSelected(candidateId)
            onSpeechRequested: artifactId => surface.authoringSpeechRequested(artifactId)
            onSetupRequested: surface.inferAccessSetupRequested()
        }
    }


    Component {
        id: imageEditorWorkspace

        ImageEditorWorkspace {
            property var resizeDraft: surface.imageResizeDraftForArtifact(
                                          operatorWorkspaceHost.openedArtifactId)
            openedOperatorTypeKey: operatorWorkspaceHost.openedOperatorTypeKey
            operatorDraftId: resizeDraft !== null ? resizeDraft.id : ""
            artifactId: operatorWorkspaceHost.openedArtifactId
            source: surface.acceptedImageSource
            sourceWidth: surface.hasSelectedArtifact ? surface.selectedArtifact.imageWidth : 0
            sourceHeight: surface.hasSelectedArtifact ? surface.selectedArtifact.imageHeight : 0
            targetWidth: resizeDraft !== null ? resizeDraft.imageResizeTargetWidth : 0
            targetHeight: resizeDraft !== null ? resizeDraft.imageResizeTargetHeight : 0
            aspectPolicyKey: resizeDraft !== null
                             ? resizeDraft.imageResizeAspectPolicy : "fit_within"
            resamplingKey: resizeDraft !== null
                           ? resizeDraft.imageResizeResampling : "lanczos3"
            compareMode: surface.compareMode
            candidatePending: surface.candidateForSelected
            acceptedSource: surface.acceptedImageSource
            candidateSource: surface.candidateImageSource
            onCropRequested: (x, y, width, height) => {
                if (surface.hasSelectedArtifact) {
                    surface.cropRequested(operatorWorkspaceHost.openedArtifactId,
                                          x, y, width, height)
                }
            }
            onResizeDraftRequested: surface.operatorDraftRequested("image.resize")
            onResizeDraftSaveRequested: (draftId, width, height, aspect, resampling) =>
                                            surface.resizeDraftSaveRequested(
                                                draftId, width, height, aspect, resampling)
            onResizeRequested: (artifactId, draftId, width, height, aspect, resampling) =>
                                   surface.resizeRequested(
                                       artifactId, draftId, width, height, aspect, resampling)
            onTransformRequested: transformKey =>
                                      surface.rasterTransformRequested(
                                          operatorWorkspaceHost.openedArtifactId, transformKey)
            onBlurRequested: radius =>
                                 surface.rasterBlurRequested(
                                     operatorWorkspaceHost.openedArtifactId, radius)
            onUnsharpMaskRequested: (radius, amountMilli, threshold) =>
                                        surface.rasterUnsharpMaskRequested(
                                            operatorWorkspaceHost.openedArtifactId,
                                            radius, amountMilli, threshold)
            onDropShadowRequested: (offsetX, offsetY, radius, red, green, blue, alpha) =>
                                       surface.rasterDropShadowRequested(
                                           operatorWorkspaceHost.openedArtifactId,
                                           offsetX, offsetY, radius,
                                           red, green, blue, alpha)
        }
    }

    Component {
        id: aiImageOperatorWorkspace

        AiImageOperatorWorkspace {
            property var imageDraft: surface.draftForId(operatorWorkspaceHost.openedNodeId)
            instruction: imageDraft !== null ? imageDraft.aiImageInstruction : ""
            outputWidth: imageDraft !== null ? imageDraft.aiImageOutputWidth
                         : surface.hasSelectedArtifact && surface.selectedArtifact.imageWidth > 0
                           ? surface.selectedArtifact.imageWidth : 1024
            outputHeight: imageDraft !== null ? imageDraft.aiImageOutputHeight
                          : surface.hasSelectedArtifact && surface.selectedArtifact.imageHeight > 0
                            ? surface.selectedArtifact.imageHeight : 1024
            acceptedSource: surface.acceptedImageSource
            candidateSource: surface.candidateImageSource
            running: surface.inferImage.running
            errorCode: surface.inferImage.errorCode
            onCanvasRequested: (width, height) => {
                if (imageDraft !== null) {
                    surface.aiImageDraftSaveRequested(
                        imageDraft.id, imageDraft.aiImageInstruction, width, height)
                }
            }
        }
    }

    Component {
        id: audioSpeechOperatorWorkspace

        AudioSpeechOperatorWorkspace {
            property bool selectedIsAcceptedAudio: surface.hasSelectedArtifact
                                                   && surface.selectedArtifact.kindKey
                                                      === "audio_clip"
            property var speechDraft: surface.speechDraftForArtifact(operatorWorkspaceHost.openedArtifactId)
            property string sourceId: speechDraft !== null ? speechDraft.inputArtifactId
                : selectedIsAcceptedAudio && surface.selectedArtifact.transformationInputArtifactIds.length > 0
                  ? surface.selectedArtifact.transformationInputArtifactIds[0] : operatorWorkspaceHost.openedArtifactId
            property var sourceArtifact: surface.artifactForId(sourceId)
            backend: surface.backend
            inferSpeech: surface.inferSpeech
            audioPreview: surface.audioPreview
            projectPath: surface.projectPath
            sourceArtifactId: sourceId
            outputArtifactId: operatorWorkspaceHost.openedArtifactId
            outputName: surface.hasSelectedArtifact ? surface.selectedArtifact.name : ""
            sourceName: sourceArtifact !== null ? sourceArtifact.name : ""
            sourceText: sourceArtifact !== null && sourceArtifact.hasTextPreview
                        ? sourceArtifact.textPreview : ""
            canGenerate: sourceArtifact !== null
                         && speechDraft !== null && speechDraft.inputRevisionId === sourceArtifact.acceptedRevisionId
                         && sourceArtifact.kindKey === "text_document"
                         && operatorDraftId.length > 0
            credentialConfigured: surface.inferCredentialConfigured
            acceptedAudioArtifactId: selectedIsAcceptedAudio && surface.selectedArtifact.hasAcceptedRevision
                                     ? surface.selectedArtifact.id : ""
            sourceChanged: selectedIsAcceptedAudio && sourceArtifact !== null
                           && surface.selectedArtifact.transformationInputRevisionIds.length > 0
                           && surface.selectedArtifact.transformationInputRevisionIds[0]
                              !== sourceArtifact.acceptedRevisionId
            acceptedDurationMillis: selectedIsAcceptedAudio
                                    ? surface.selectedArtifact.audioDurationMillis : 0
            acceptedSampleRateHz: selectedIsAcceptedAudio
                                  ? surface.selectedArtifact.audioSampleRateHz : 0
            acceptedChannels: selectedIsAcceptedAudio
                              ? surface.selectedArtifact.audioChannels : 0
            acceptedOriginKey: selectedIsAcceptedAudio
                               ? surface.selectedArtifact.audioOriginKey : ""
            onWritingRequested: surface.writingRequested(sourceId, scriptMode)
            onExportRequested: surface.audioExportRequested(
                operatorWorkspaceHost.openedArtifactId,
                candidate !== null ? candidate.id : "")
            candidate: surface.candidateForSelected
                       && surface.selectedCandidate.hasAudioPreview
                       ? surface.selectedCandidate : null

            onDraftSaveRequested: (draftId, presetAlias, presetCatalogRevision,
                                   language, speedMilli,
                                   syntheticDisclosureRequired) =>
                                      surface.speechDraftSaveRequested(
                                          draftId, presetAlias,
                                          presetCatalogRevision, language,
                                          speedMilli,
                                          syntheticDisclosureRequired)
            onSynthesizeRequested: (sourceArtifactId, draftId, artifactName,
                                    presetAlias, presetCatalogRevision,
                                    language, speedMilli,
                                    syntheticDisclosureRequired) =>
                                      surface.speechSynthesisRequested(
                                          sourceArtifactId, draftId, artifactName,
                                          presetAlias, presetCatalogRevision,
                                          language, speedMilli,
                                          syntheticDisclosureRequired)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: surface.currentMode

            SceneOperatorGraphWorkspace {
                projectName: surface.projectName
                sceneName: surface.hasSelectedArtifact ? surface.selectedArtifact.name : ""
                sceneKind: surface.hasSelectedArtifact
                           ? surface.selectedArtifact.kindLabel : ""
                nodes: surface.graphNodes
                edges: surface.graphEdges
                candidates: surface.candidates
                drafts: surface.operatorDrafts.filter(d => !surface.graphNodes.some(n => n.id === d.id))
                operatorDescriptors: surface.operatorDescriptors
                artifactKindKey: surface.hasSelectedArtifact
                                 ? surface.selectedArtifact.kindKey : ""
                artifactTextPreview: surface.hasSelectedArtifact
                                     ? surface.selectedArtifact.textPreview : ""
                artifactTextPreviewTruncated: surface.hasSelectedArtifact
                                              && surface.selectedArtifact.textPreviewTruncated
                acceptedImageSource: surface.acceptedImageSource
                artifactImageWidth: surface.hasSelectedArtifact
                                    ? surface.selectedArtifact.imageWidth : 0
                artifactImageHeight: surface.hasSelectedArtifact
                                     ? surface.selectedArtifact.imageHeight : 0
                artifactAudioDurationMillis: surface.hasSelectedArtifact
                                             ? surface.selectedArtifact.audioDurationMillis : 0
                artifactAudioSampleRateHz: surface.hasSelectedArtifact
                                           ? surface.selectedArtifact.audioSampleRateHz : 0
                artifactAudioChannels: surface.hasSelectedArtifact
                                       ? surface.selectedArtifact.audioChannels : 0
                hasAcceptedRevision: surface.hasSelectedArtifact
                                     && surface.selectedArtifact.hasAcceptedRevision
                selectedNodeId: surface.selectedNodeId
                selectedCandidateId: surface.selectedCandidateId
                onNodeSelected: nodeId => surface.selectNode(nodeId)
                onNodeOpened: nodeId => surface.openNode(nodeId)
                onDraftRequested: operatorTypeKey => {
                    if (operatorTypeKey === "image.edit") surface.openImageEditor()
                    else surface.operatorDraftRequested(operatorTypeKey)
                }
                onDraftSelected: draftId => surface.selectNode(draftId)
                onDraftOpened: draftId => surface.openOperatorDraft(draftId)
                onDraftDiscardRequested: draftId => surface.operatorDraftDiscardRequested(draftId)
                onCandidateSelected: candidateId => surface.candidateSelected(candidateId)
                onCandidateReviewRequested: candidateId => surface.candidateReviewRequested(
                                                candidateId)
            }

            OperatorWorkspaceHost {
                id: operatorWorkspaceHost
                compactNavigation: surface.guidedAuthoringActive
                selectedCandidateId: surface.selectedCandidateId
                operatorWorkspaces: ({
                    "text.create": textAuthoringWorkspace,
                    "text.edit": textAuthoringWorkspace,
                    "text.transform": textAuthoringWorkspace,
                    "image.edit": imageEditorWorkspace,
                    "image.crop": imageEditorWorkspace,
                    "image.resize": imageEditorWorkspace,
                    "image.transform": imageEditorWorkspace,
                    "image.blur": imageEditorWorkspace,
                    "image.unsharp_mask": imageEditorWorkspace,
                    "image.drop_shadow": imageEditorWorkspace,
                    "image.generate": aiImageOperatorWorkspace,
                    "audio.speech_synthesize": audioSpeechOperatorWorkspace
                })
                onReturnRequested: surface.showGraph()
            }
        }
    }
}

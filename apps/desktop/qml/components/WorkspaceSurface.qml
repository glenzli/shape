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
    required property InferSpeechController inferSpeech
    required property AudioPreviewController audioPreview
    property string projectPath: ""
    property bool inferCredentialConfigured: false
    property int currentMode: 0
    property string selectedNodeId: ""

    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    readonly property var graphNodes: hasSelectedArtifact
                                      ? (selectedArtifact.operatorNodes || []) : []
    readonly property var graphEdges: hasSelectedArtifact
                                      ? (selectedArtifact.operatorEdges || []) : []
    readonly property bool graphActive: currentMode === 0
    readonly property bool focusActive: currentMode === 1
    readonly property bool editWorkspaceActive: focusActive
                                                && operatorWorkspaceHost.openedRoleKey
                                                   === "operator"
                                                && operatorWorkspaceHost
                                                   .hasRegisteredOperatorWorkspace
    readonly property bool intentWorkspaceActive: editWorkspaceActive
                                                  && (workspaceRouteKey === "operator.text.edit"
                                                      || workspaceRouteKey
                                                         === "operator.text.transform"
                                                      || workspaceRouteKey
                                                         === "operator.image.crop")
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
    signal speechDraftSaveRequested(string draftId, string presetAlias,
                                    string presetCatalogRevision, string language,
                                    int speedMilli, bool syntheticDisclosureRequired)
    signal speechSynthesisRequested(string sourceArtifactId, string draftId,
                                    string artifactName, string presetAlias,
                                    string presetCatalogRevision, string language,
                                    int speedMilli, bool syntheticDisclosureRequired)
    signal operatorDraftRequested(string operatorTypeKey)
    signal operatorDraftDiscardRequested(string draftId)

    function showArtifact() : bool {
        if (selectedCandidate !== null && selectedCandidate.hasAudioPreview) {
            return openSpeechWorkspace()
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
        const node = nodeForId(nodeId)
        if (node === null) return false
        selectedNodeId = nodeId
        if (!operatorWorkspaceHost.openWorkspace(
                node.id, node.roleKey, node.operatorTypeKey,
                node.artifactId, node.revisionId, node.transformationId)) {
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
        if (nodes.length > 0) selectedNodeId = nodes[0].id
    }

    onSelectedArtifactChanged: synchronizeNodeSelection()
    onGraphNodesChanged: synchronizeNodeSelection()
    onOperatorDraftsChanged: {
        synchronizeNodeSelection()
        refreshSpeechDraftProjection()
    }

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
        id: textEditOperatorWorkspace

        TextOperatorWorkspace {
            objectName: "textEditOperatorWorkspace"
            property string nodeId: operatorWorkspaceHost.openedNodeId
            property string artifactId: operatorWorkspaceHost.openedArtifactId
            property string revisionId: operatorWorkspaceHost.openedRevisionId
            property string transformationId: operatorWorkspaceHost.openedTransformationId
            property string candidateId: operatorWorkspaceHost.selectedCandidateId

            acceptedText: surface.hasSelectedArtifact
                          ? surface.selectedArtifact.textPreview : ""
            candidateText: surface.candidateForSelected
                           ? surface.selectedCandidate.text : ""
            hasAcceptedRevision: surface.hasSelectedArtifact
                                 && surface.selectedArtifact.hasAcceptedRevision
            hasTextPreview: surface.hasSelectedArtifact
                            && surface.selectedArtifact.hasTextPreview
            textPreviewTruncated: surface.hasSelectedArtifact
                                  && surface.selectedArtifact.textPreviewTruncated
            hasCandidate: surface.candidateForSelected
            compareMode: surface.compareMode
            onSpeechWorkspaceRequested: surface.openSpeechWorkspace()
        }
    }

    Component {
        id: imageCropOperatorWorkspace

        Item {
            objectName: "imageCropOperatorWorkspace"
            property string nodeId: operatorWorkspaceHost.openedNodeId
            property string artifactId: operatorWorkspaceHost.openedArtifactId
            property string revisionId: operatorWorkspaceHost.openedRevisionId
            property string transformationId: operatorWorkspaceHost.openedTransformationId
            property string candidateId: operatorWorkspaceHost.selectedCandidateId

            RasterCropOperatorWorkspace {
                anchors.fill: parent
                anchors.margins: 24
                visible: !surface.compareMode || !surface.candidateForSelected
                artifactId: operatorWorkspaceHost.openedArtifactId
                source: surface.acceptedImageSource
                sourceWidth: surface.hasSelectedArtifact
                             ? surface.selectedArtifact.imageWidth : 0
                sourceHeight: surface.hasSelectedArtifact
                              ? surface.selectedArtifact.imageHeight : 0
                onCropRequested: (x, y, width, height) => {
                    if (surface.hasSelectedArtifact) {
                        surface.cropRequested(operatorWorkspaceHost.openedArtifactId,
                                              x, y, width, height)
                    }
                }
            }

            ImageCompareWorkspace {
                anchors.fill: parent
                anchors.margins: 24
                visible: surface.compareMode && surface.candidateForSelected
                acceptedSource: surface.acceptedImageSource
                candidateSource: surface.candidateImageSource
            }
        }
    }

    Component {
        id: audioSpeechOperatorWorkspace

        AudioSpeechOperatorWorkspace {
            property bool selectedIsAcceptedAudio: surface.hasSelectedArtifact
                                                   && surface.selectedArtifact.kindKey
                                                      === "audio_clip"
            property string sourceId: selectedIsAcceptedAudio
                                      && surface.selectedArtifact
                                                .transformationInputArtifactIds.length > 0
                                      ? surface.selectedArtifact
                                          .transformationInputArtifactIds[0]
                                      : operatorWorkspaceHost.openedArtifactId
            property var sourceArtifact: surface.artifactForId(sourceId)
            inferSpeech: surface.inferSpeech
            audioPreview: surface.audioPreview
            projectPath: surface.projectPath
            sourceArtifactId: sourceId
            sourceName: sourceArtifact !== null ? sourceArtifact.name : ""
            sourceText: sourceArtifact !== null && sourceArtifact.hasTextPreview
                        ? sourceArtifact.textPreview : ""
            canGenerate: !selectedIsAcceptedAudio && sourceArtifact !== null
                         && sourceArtifact.kindKey === "text_document"
                         && operatorDraftId.length > 0
            credentialConfigured: surface.inferCredentialConfigured
            acceptedAudioArtifactId: selectedIsAcceptedAudio
                                     ? surface.selectedArtifact.id : ""
            acceptedDurationMillis: selectedIsAcceptedAudio
                                    ? surface.selectedArtifact.audioDurationMillis : 0
            acceptedSampleRateHz: selectedIsAcceptedAudio
                                  ? surface.selectedArtifact.audioSampleRateHz : 0
            acceptedChannels: selectedIsAcceptedAudio
                              ? surface.selectedArtifact.audioChannels : 0
            acceptedOriginKey: selectedIsAcceptedAudio
                               ? surface.selectedArtifact.audioOriginKey : ""
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
        spacing: 8

        Rectangle {
            visible: surface.graphActive
            Layout.fillWidth: true
            Layout.preferredHeight: visible ? 46 : 0
            radius: Theme.radiusMedium
            color: Theme.surface
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 5
                spacing: 5

                ColumnLayout {
                    Layout.leftMargin: 8
                    Layout.fillWidth: true
                    spacing: 1

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("SCENE GRAPH")
                        color: Theme.text
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.7
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Connect Sources, Operators, and named Outputs.")
                        color: Theme.muted
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                }

                Rectangle {
                    Layout.rightMargin: 4
                    Layout.preferredWidth: primaryViewLabel.implicitWidth + 18
                    Layout.preferredHeight: 26
                    radius: 13
                    color: Theme.accentSoft
                    border.color: Theme.accent

                    Text {
                        id: primaryViewLabel
                        anchors.centerIn: parent
                        text: qsTr("SCENE HOME")
                        color: Theme.accent
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.4
                    }
                }
            }
        }

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
                drafts: surface.operatorDrafts
                operatorDescriptors: surface.operatorDescriptors
                selectedNodeId: surface.selectedNodeId
                selectedCandidateId: surface.selectedCandidateId
                onNodeSelected: nodeId => surface.selectNode(nodeId)
                onNodeOpened: nodeId => surface.openNode(nodeId)
                onDraftRequested: operatorTypeKey => surface.operatorDraftRequested(
                                      operatorTypeKey)
                onDraftSelected: draftId => surface.selectNode(draftId)
                onDraftOpened: draftId => surface.openOperatorDraft(draftId)
                onDraftDiscardRequested: draftId => surface.operatorDraftDiscardRequested(draftId)
                onCandidateSelected: candidateId => surface.candidateSelected(candidateId)
                onCandidateReviewRequested: candidateId => surface.candidateReviewRequested(
                                                candidateId)
            }

            OperatorWorkspaceHost {
                id: operatorWorkspaceHost
                selectedCandidateId: surface.selectedCandidateId
                operatorWorkspaces: ({
                    "text.edit": textEditOperatorWorkspace,
                    "text.transform": textEditOperatorWorkspace,
                    "image.crop": imageCropOperatorWorkspace,
                    "audio.speech_synthesize": audioSpeechOperatorWorkspace
                })
                onReturnRequested: surface.showGraph()
            }
        }
    }
}

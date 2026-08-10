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
    required property AudioPreviewController audioPreview
    required property UiPreferences uiPreferences

    property int selectedArtifactIndex: 0
    property string selectedCandidateId: ""
    property bool compareMode: false
    readonly property var selectedArtifact: window.backend.artifacts.length > selectedArtifactIndex
                                            ? window.backend.artifacts[selectedArtifactIndex]
                                            : null
    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    readonly property var artifactCandidates: hasSelectedArtifact
                                              ? candidatesForArtifact(selectedArtifact.id) : []
    readonly property var selectedCandidate: {
        for (let index = 0; index < artifactCandidates.length; ++index) {
            if (artifactCandidates[index].id === selectedCandidateId) {
                return artifactCandidates[index]
            }
        }
        return artifactCandidates.length > 0 ? artifactCandidates[0] : null
    }
    readonly property bool candidateForSelected: selectedCandidate !== null

    function candidatesForArtifact(artifactId) : var {
        const matches = []
        for (let index = 0; index < backend.candidates.length; ++index) {
            if (backend.candidates[index].contextArtifactId === artifactId) {
                matches.push(backend.candidates[index])
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
            contextInspector.currentPage = 0
        }
    }

    function toggleComparison() : void {
        if (!candidateForSelected) {
            return
        }
        compareMode = !compareMode
        if (compareMode) {
            workspaceSurface.showArtifact()
            contextInspector.currentPage = 0
        }
    }

    function refreshSelectedImage() : void {
        if (!hasSelectedArtifact || selectedArtifact.kindKey !== "image_raster"
                || !selectedArtifact.hasAcceptedRevision) {
            return
        }
        const imageCandidateId = selectedCandidate !== null
                                 && selectedCandidate.hasImagePreview
                                 ? selectedCandidate.id : ""
        backend.prepareImagePreviews(selectedArtifact.id, imageCandidateId)
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
        uiPreferences: window.uiPreferences
        inferText: window.inferText
    }

    FileDialog {
        id: imageImportDialog
        title: qsTr("Import raster image")
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

    BranchArtifactDialog {
        id: branchDialog
        backend: window.backend
        onBranchCreated: {
            window.selectedArtifactIndex = Math.max(0, window.backend.artifactCount - 1)
            window.compareMode = false
            workspaceSurface.showGraph()
            contextInspector.currentPage = 2
        }
    }

    header: MainTitleBar {
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
        anchors.margins: 14
        spacing: 12

        ProjectNavigator {
            Layout.minimumWidth: 232
            Layout.preferredWidth: 232
            Layout.maximumWidth: 232
            Layout.fillHeight: true
            projectOpen: window.backend.projectOpen
            artifacts: window.backend.artifacts
            selectedIndex: window.selectedArtifactIndex
            candidates: window.backend.candidates
            graphActive: workspaceSurface.graphActive
            onArtifactSelected: index => workspaceSurface.activateScene(index)
            onArtifactOpened: index => workspaceSurface.activateScene(index)
            onGraphRequested: workspaceSurface.showGraph()
            onImportImageRequested: imageImportDialog.open()
        }

        ColumnLayout {
            Layout.minimumWidth: 500
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            WorkspaceSurface {
                id: workspaceSurface

                Layout.fillWidth: true
                Layout.fillHeight: true
                projectName: window.backend.projectName
                allArtifacts: window.backend.artifacts
                selectedArtifact: window.selectedArtifact
                candidates: window.artifactCandidates
                selectedCandidate: window.selectedCandidate
                selectedCandidateId: window.selectedCandidateId
                compareMode: window.compareMode
                acceptedImageSource: window.backend.acceptedImageSource
                candidateImageSource: window.backend.candidateImageSource
                inferSpeech: window.inferSpeech
                audioPreview: window.audioPreview
                projectPath: window.backend.bundlePath
                inferCredentialConfigured: window.inferText.credentialConfigured
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
                        contextInspector.currentPage = 0
                    }
                }
                onSpeechSynthesisRequested: (sourceArtifactId, artifactName, speedMilli) => {
                    window.inferSpeech.generate(
                        window.backend.bundlePath, sourceArtifactId,
                        artifactName, speedMilli)
                }
            }

            IntentPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: visible ? 244 : 0
                visible: workspaceSurface.intentWorkspaceActive
                artifactId: window.hasSelectedArtifact ? window.selectedArtifact.id : ""
                artifactKindKey: window.hasSelectedArtifact
                                 ? window.selectedArtifact.kindKey : ""
                acceptedText: window.hasSelectedArtifact
                              ? window.selectedArtifact.textPreview : ""
                hasAcceptedRevision: window.hasSelectedArtifact
                                     && window.selectedArtifact.hasAcceptedRevision
                candidatePending: window.candidateForSelected
                candidates: window.artifactCandidates
                errorMessage: window.backend.lastError
                generationRunning: window.inferText.running
                credentialConfigured: window.inferText.credentialConfigured
                generationErrorCode: window.inferText.errorCode
                runtimeProbing: window.inferRuntime.probing
                runtimeReachable: window.inferRuntime.reachable
                runtimeCompatible: window.inferRuntime.compatible
                runtimeContractVersion: window.inferRuntime.contractVersion
                runtimeEndpointSource: window.inferRuntime.endpointSource
                onRuntimeRefreshRequested: window.inferRuntime.refresh()
                onInferCandidateRequested: (artifactId, prompt) => {
                    window.inferText.generate(
                        window.backend.bundlePath, artifactId, prompt)
                }
                onCandidateRequested: (artifactId, replacementText) => {
                    if (window.backend.proposeTextCandidate(artifactId, replacementText)) {
                        window.selectedCandidateId = window.backend.candidateId
                        window.compareMode = true
                        workspaceSurface.showArtifact()
                        contextInspector.currentPage = 0
                    }
                }
            }
        }

        ContextInspector {
            id: contextInspector

            Layout.minimumWidth: 328
            Layout.preferredWidth: 328
            Layout.maximumWidth: 328
            Layout.fillHeight: true
            artifact: window.selectedArtifact
            candidates: window.artifactCandidates
            selectedCandidateId: window.selectedCandidate !== null
                                 ? window.selectedCandidate.id : ""
            onCompareRequested: window.toggleComparison()
            onCandidateSelected: candidateId => window.activateCandidate(candidateId)
            onDiscardRequested: candidateId => {
                if (window.backend.discardCandidate(candidateId)) {
                    window.compareMode = false
                }
            }
            onAcceptRequested: candidateId => {
                const targetArtifactId = window.selectedCandidate !== null
                                         ? window.selectedCandidate.artifactId : ""
                const contextArtifactId = window.selectedCandidate !== null
                                          ? window.selectedCandidate.contextArtifactId : ""
                if (window.backend.acceptCandidate(candidateId)) {
                    window.compareMode = false
                    if (targetArtifactId.length > 0
                            && targetArtifactId !== contextArtifactId) {
                        const acceptedIndex = window.artifactIndex(targetArtifactId)
                        if (acceptedIndex >= 0) {
                            window.selectedArtifactIndex = acceptedIndex
                            workspaceSurface.showGraph()
                        }
                    }
                }
            }
            onBranchRequested: candidateId => branchDialog.openFor(
                                   window.selectedArtifact.name, candidateId)
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
                contextInspector.currentPage = 0
            }
        }
    }
}

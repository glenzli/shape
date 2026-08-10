pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Shape.Desktop
import "components"

ApplicationWindow {
    id: window

    required property DesktopBackend backend
    required property UiPreferences uiPreferences

    property int selectedArtifactIndex: 0
    property bool compareMode: false
    readonly property var selectedArtifact: window.backend.artifacts.length > selectedArtifactIndex
                                            ? window.backend.artifacts[selectedArtifactIndex]
                                            : null
    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    readonly property bool candidateForSelected: hasSelectedArtifact
                                                  && window.backend.hasCandidate
                                                  && window.backend.candidateArtifactId
                                                     === selectedArtifact.id

    onSelectedArtifactIndexChanged: compareMode = false

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
    }

    header: MainTitleBar {
        hostWindow: window
        projectOpen: window.backend.projectOpen
        projectName: window.backend.projectName
        compareAvailable: window.candidateForSelected
        compareActive: window.compareMode
        onCompareRequested: window.compareMode = !window.compareMode
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
            hasCandidate: window.backend.hasCandidate
            candidateArtifactId: window.backend.candidateArtifactId
            onArtifactSelected: index => window.selectedArtifactIndex = index
        }

        ColumnLayout {
            Layout.minimumWidth: 500
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            ArtifactWorkspace {
                Layout.fillWidth: true
                Layout.fillHeight: true
                projectName: window.backend.projectName
                artifactName: window.hasSelectedArtifact
                              ? window.selectedArtifact.name
                              : qsTr("No artifact selected")
                artifactKind: window.hasSelectedArtifact
                              ? window.selectedArtifact.kindLabel
                              : ""
                artifactText: window.hasSelectedArtifact
                              ? window.selectedArtifact.textPreview
                              : ""
                hasAcceptedRevision: window.hasSelectedArtifact
                                     && window.selectedArtifact.hasAcceptedRevision
                hasTextPreview: window.hasSelectedArtifact
                                && window.selectedArtifact.hasTextPreview
                textPreviewTruncated: window.hasSelectedArtifact
                                      && window.selectedArtifact.textPreviewTruncated
                hasCandidate: window.candidateForSelected
                candidateText: window.candidateForSelected ? window.backend.candidateText : ""
                compareMode: window.compareMode
            }

            IntentPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: 188
                artifactId: window.hasSelectedArtifact ? window.selectedArtifact.id : ""
                artifactKindKey: window.hasSelectedArtifact
                                 ? window.selectedArtifact.kindKey : ""
                acceptedText: window.hasSelectedArtifact
                              ? window.selectedArtifact.textPreview : ""
                candidatePending: window.candidateForSelected
                errorMessage: window.backend.lastError
                onCandidateRequested: (artifactId, replacementText) => {
                    if (window.backend.proposeTextCandidate(artifactId, replacementText)) {
                        window.compareMode = true
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
            hasCandidate: window.candidateForSelected
            candidateText: window.candidateForSelected ? window.backend.candidateText : ""
            candidateTextTruncated: window.candidateForSelected
                                    && window.backend.candidateTextTruncated
            onCompareRequested: window.compareMode = !window.compareMode
            onDiscardRequested: {
                window.backend.discardCandidate()
                window.compareMode = false
            }
            onAcceptRequested: {
                if (window.backend.acceptCandidate()) {
                    window.compareMode = false
                }
            }
        }
    }

    Connections {
        target: window.backend

        function onCandidateChanged() : void {
            if (!window.candidateForSelected) {
                window.compareMode = false
            }
        }
    }
}

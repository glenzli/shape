pragma ComponentBehavior: Bound

//! Central workspace navigation and composition. Media presentation remains in
//! ArtifactWorkspace; accepted project topology belongs to ProjectGraphWorkspace.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: surface
    objectName: "workspaceSurface"

    property string projectName: ""
    property var artifacts: []
    property var graphEdges: []
    property var selectedArtifact: null
    property int selectedIndex: 0
    property var candidates: []
    property var selectedCandidate: null
    property string selectedCandidateId: ""
    property bool compareMode: false
    property int currentMode: 0

    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    readonly property bool candidateForSelected: hasSelectedArtifact
                                                  && selectedCandidate !== null
                                                  && selectedCandidate.artifactId
                                                     === selectedArtifact.id

    signal artifactSelected(int index)
    signal candidateSelected(string candidateId)

    function showArtifact() : void {
        currentMode = 0
    }

    function showGraph() : void {
        currentMode = 1
    }

    function activateArtifact(index) : void {
        artifactSelected(index)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 44
            radius: Theme.radiusMedium
            color: Theme.surface
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 5
                spacing: 5

                Text {
                    Layout.leftMargin: 8
                    text: qsTr("WORKSPACE")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }

                Item { Layout.fillWidth: true }

                ShapeButton {
                    implicitHeight: 32
                    implicitWidth: 106
                    text: qsTr("Artifact")
                    selected: surface.currentMode === 0
                    onClicked: surface.showArtifact()
                }

                ShapeButton {
                    implicitHeight: 32
                    implicitWidth: 106
                    text: qsTr("Project graph")
                    selected: surface.currentMode === 1
                    onClicked: surface.showGraph()
                }
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: surface.currentMode

            ArtifactWorkspace {
                projectName: surface.projectName
                artifactName: surface.hasSelectedArtifact
                              ? surface.selectedArtifact.name
                              : qsTr("No artifact selected")
                artifactKind: surface.hasSelectedArtifact
                              ? surface.selectedArtifact.kindLabel : ""
                artifactText: surface.hasSelectedArtifact
                              ? surface.selectedArtifact.textPreview : ""
                hasAcceptedRevision: surface.hasSelectedArtifact
                                     && surface.selectedArtifact.hasAcceptedRevision
                hasTextPreview: surface.hasSelectedArtifact
                                && surface.selectedArtifact.hasTextPreview
                textPreviewTruncated: surface.hasSelectedArtifact
                                      && surface.selectedArtifact.textPreviewTruncated
                hasCandidate: surface.candidateForSelected
                candidateText: surface.candidateForSelected
                               ? surface.selectedCandidate.text : ""
                compareMode: surface.compareMode
            }

            ProjectGraphWorkspace {
                projectName: surface.projectName
                artifacts: surface.artifacts
                edges: surface.graphEdges
                selectedIndex: surface.selectedIndex
                candidates: surface.candidates
                selectedCandidateId: surface.selectedCandidateId
                onArtifactSelected: index => surface.activateArtifact(index)
                onCandidateSelected: candidateId => surface.candidateSelected(candidateId)
            }
        }
    }
}

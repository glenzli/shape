//! Right-side contextual navigation for exploration, artifact facts, and lineage.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: inspector

    property var artifact: null
    property var candidates: []
    property string selectedCandidateId: ""
    property int currentPage: 0

    readonly property bool hasArtifact: artifact !== null
    readonly property bool hasCandidate: candidates.length > 0

    signal compareRequested()
    signal candidateSelected(string candidateId)
    signal discardRequested(string candidateId)
    signal acceptRequested(string candidateId)
    signal branchRequested(string candidateId)

    ColumnLayout {
        anchors.fill: parent
        spacing: 10

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

                ShapeButton {
                    expanded: true
                    Layout.fillWidth: true
                    implicitHeight: 32
                    text: qsTr("Explore")
                    selected: inspector.currentPage === 0
                    onClicked: inspector.currentPage = 0
                }

                ShapeButton {
                    expanded: true
                    Layout.fillWidth: true
                    implicitHeight: 32
                    text: qsTr("Details")
                    selected: inspector.currentPage === 1
                    onClicked: inspector.currentPage = 1
                }

                ShapeButton {
                    expanded: true
                    Layout.fillWidth: true
                    implicitHeight: 32
                    text: qsTr("Lineage")
                    selected: inspector.currentPage === 2
                    onClicked: inspector.currentPage = 2
                }
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: inspector.currentPage

            VariantsPanel {
                artifactText: inspector.hasArtifact ? inspector.artifact.textPreview : ""
                hasAcceptedRevision: inspector.hasArtifact
                                     && inspector.artifact.hasAcceptedRevision
                hasTextPreview: inspector.hasArtifact && inspector.artifact.hasTextPreview
                artifactKindKey: inspector.hasArtifact ? inspector.artifact.kindKey : ""
                imageWidth: inspector.hasArtifact ? inspector.artifact.imageWidth : 0
                imageHeight: inspector.hasArtifact ? inspector.artifact.imageHeight : 0
                audioDurationMillis: inspector.hasArtifact
                                     ? inspector.artifact.audioDurationMillis : 0
                audioSampleRateHz: inspector.hasArtifact
                                   ? inspector.artifact.audioSampleRateHz : 0
                audioChannels: inspector.hasArtifact ? inspector.artifact.audioChannels : 0
                audioOriginKey: inspector.hasArtifact ? inspector.artifact.audioOriginKey : ""
                acceptedRevisionId: inspector.hasArtifact
                                    ? inspector.artifact.acceptedRevisionId : ""
                candidates: inspector.candidates
                selectedCandidateId: inspector.selectedCandidateId
                onCompareRequested: inspector.compareRequested()
                onCandidateSelected: candidateId => inspector.candidateSelected(candidateId)
                onDiscardRequested: candidateId => inspector.discardRequested(candidateId)
                onAcceptRequested: candidateId => inspector.acceptRequested(candidateId)
                onBranchRequested: candidateId => inspector.branchRequested(candidateId)
            }

            ArtifactDetailsPanel {
                artifactName: inspector.hasArtifact ? inspector.artifact.name : ""
                artifactKind: inspector.hasArtifact ? inspector.artifact.kindLabel : ""
                artifactId: inspector.hasArtifact ? inspector.artifact.id : ""
                hasAcceptedRevision: inspector.hasArtifact
                                     && inspector.artifact.hasAcceptedRevision
                hasCandidate: inspector.hasCandidate
                mediaType: inspector.hasArtifact ? inspector.artifact.mediaType : ""
                byteLength: inspector.hasArtifact ? inspector.artifact.byteLength : 0
                contentDigest: inspector.hasArtifact ? inspector.artifact.contentDigest : ""
            }

            ArtifactLineagePanel {
                hasAcceptedRevision: inspector.hasArtifact
                                     && inspector.artifact.hasAcceptedRevision
                revisionId: inspector.hasArtifact ? inspector.artifact.acceptedRevisionId : ""
                parentRevisionIds: inspector.hasArtifact
                                   ? inspector.artifact.acceptedParentRevisionIds : []
                transformationId: inspector.hasArtifact
                                  ? inspector.artifact.transformationId : ""
                transformationKind: inspector.hasArtifact
                                    ? inspector.artifact.transformationKindLabel : ""
                transformationIntent: inspector.hasArtifact
                                      ? inspector.artifact.transformationIntent : ""
                inputRevisionIds: inspector.hasArtifact
                                  ? inspector.artifact.transformationInputRevisionIds : []
                inputArtifactNames: inspector.hasArtifact
                                    ? inspector.artifact.transformationInputArtifactNames : []
                constraintCount: inspector.hasArtifact ? inspector.artifact.constraintCount : 0
                referenceCount: inspector.hasArtifact ? inspector.artifact.referenceCount : 0
            }
        }
    }
}

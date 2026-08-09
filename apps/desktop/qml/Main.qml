pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop
import "components"

ApplicationWindow {
    id: window
    required property DesktopBackend backend
    property int selectedArtifactIndex: 0
    readonly property var selectedArtifact: window.backend.artifacts.length > selectedArtifactIndex
                                            ? window.backend.artifacts[selectedArtifactIndex]
                                            : null
    readonly property bool hasSelectedArtifact: selectedArtifact !== null
    width: 1440
    height: 900
    minimumWidth: 1024
    minimumHeight: 700
    visible: true
    title: window.backend.projectOpen
           ? qsTr("Shape — %1").arg(window.backend.projectName)
           : qsTr("Shape — No Project")
    color: Theme.background

    header: ToolBar {
        implicitHeight: 58
        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 14

            Label {
                text: qsTr("SHAPE")
                color: Theme.accent
                font.pixelSize: 17
                font.bold: true
                font.letterSpacing: 2
            }
            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 24
                color: Theme.border
            }
            Label {
                text: window.backend.projectOpen
                      ? window.backend.projectName
                      : qsTr("No project open")
                color: Theme.text
                font.pixelSize: 15
            }
            Label {
                text: window.backend.projectOpen
                      ? qsTr("Saved locally")
                      : qsTr("Open a .shape project to begin")
                color: Theme.muted
                font.pixelSize: 12
            }
            Item { Layout.fillWidth: true }
            Button {
                text: qsTr("Compare")
                enabled: false
                ToolTip.text: qsTr("Compare becomes available when variants exist")
                ToolTip.visible: hovered
            }
            Button {
                text: qsTr("Export")
                enabled: false
                ToolTip.text: qsTr("Export is not connected in the foundation build")
                ToolTip.visible: hovered
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Frame {
            Layout.preferredWidth: 210
            Layout.fillHeight: true
            padding: 12
            background: Rectangle {
                color: Theme.surface
                radius: Theme.radiusLarge
                border.color: Theme.border
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: 10
                Label {
                    text: qsTr("ARTIFACTS")
                    color: Theme.muted
                    font.pixelSize: 11
                    font.bold: true
                }
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 4
                    model: window.backend.artifacts
                    delegate: ItemDelegate {
                        id: artifactDelegate
                        required property int index
                        required property var modelData
                        width: ListView.view.width
                        highlighted: window.selectedArtifactIndex === artifactDelegate.index
                        text: artifactDelegate.modelData.name
                        onClicked: window.selectedArtifactIndex = artifactDelegate.index
                        contentItem: Column {
                            spacing: 2
                            Label { text: artifactDelegate.modelData.name; color: Theme.text }
                            Label { text: artifactDelegate.modelData.kindLabel; color: Theme.muted; font.pixelSize: 11 }
                        }
                    }
                }
                Label {
                    visible: window.backend.artifactCount === 0
                    Layout.fillWidth: true
                    text: window.backend.projectOpen
                          ? qsTr("This project has no artifacts")
                          : qsTr("No project loaded")
                    color: Theme.muted
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    text: qsTr("A project contains stable creative artifacts. Accepted revisions remain immutable.")
                    color: Theme.muted
                    wrapMode: Text.WordWrap
                    font.pixelSize: 12
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            ArtifactWorkspace {
                Layout.fillWidth: true
                Layout.fillHeight: true
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
            }

            IntentPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: 178
            }
        }

        ColumnLayout {
            Layout.preferredWidth: 310
            Layout.fillHeight: true
            spacing: 10

            VariantsPanel {
                Layout.fillWidth: true
                Layout.fillHeight: true
                artifactText: window.hasSelectedArtifact
                              ? window.selectedArtifact.textPreview
                              : ""
                hasAcceptedRevision: window.hasSelectedArtifact
                                     && window.selectedArtifact.hasAcceptedRevision
                hasTextPreview: window.hasSelectedArtifact
                                && window.selectedArtifact.hasTextPreview
            }
            SemanticHistory {
                Layout.fillWidth: true
                Layout.preferredHeight: 260
                hasAcceptedRevision: window.hasSelectedArtifact
                                     && window.selectedArtifact.hasAcceptedRevision
                revisionId: window.hasSelectedArtifact
                            ? window.selectedArtifact.acceptedRevisionId
                            : ""
            }
        }
    }
}

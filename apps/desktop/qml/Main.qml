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
    readonly property var selectedArtifact: window.backend.artifacts.length > selectedArtifactIndex
                                            ? window.backend.artifacts[selectedArtifactIndex]
                                            : null
    readonly property bool hasSelectedArtifact: selectedArtifact !== null

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
        onSettingsRequested: settingsDialog.open()
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        Rectangle {
            Layout.minimumWidth: 224
            Layout.preferredWidth: 224
            Layout.maximumWidth: 224
            Layout.fillHeight: true
            radius: Theme.radiusLarge
            color: Theme.surface
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: qsTr("ARTIFACTS")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.8
                    }

                    Item { Layout.fillWidth: true }

                    Rectangle {
                        Layout.preferredWidth: countLabel.implicitWidth + 14
                        Layout.preferredHeight: 22
                        radius: 11
                        color: Theme.raised
                        border.color: Theme.border

                        Text {
                            id: countLabel
                            anchors.centerIn: parent
                            text: window.backend.artifactCount
                            color: Theme.muted
                            font.pixelSize: 10
                        }
                    }
                }

                ListView {
                    id: artifactList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 5
                    model: window.backend.artifacts

                    delegate: ItemDelegate {
                        id: artifactDelegate
                        required property int index
                        required property var modelData

                        width: ListView.view.width
                        height: 58
                        leftPadding: 12
                        rightPadding: 10
                        highlighted: window.selectedArtifactIndex === artifactDelegate.index
                        onClicked: window.selectedArtifactIndex = artifactDelegate.index

                        background: Rectangle {
                            radius: Theme.radiusMedium
                            color: artifactDelegate.highlighted
                                   ? Theme.selected
                                   : artifactDelegate.hovered ? Theme.raisedHover : "transparent"
                            border.width: artifactDelegate.highlighted ? 1 : 0
                            border.color: Theme.accent
                        }

                        contentItem: RowLayout {
                            spacing: 10

                            Rectangle {
                                Layout.preferredWidth: 28
                                Layout.preferredHeight: 28
                                radius: 8
                                color: artifactDelegate.highlighted ? Theme.accentSoft : Theme.raised
                                border.color: Theme.border

                                Text {
                                    anchors.centerIn: parent
                                    text: artifactDelegate.modelData.kindLabel.length > 0
                                          ? artifactDelegate.modelData.kindLabel.charAt(0).toUpperCase()
                                          : "·"
                                    color: artifactDelegate.highlighted ? Theme.accent : Theme.muted
                                    font.pixelSize: 11
                                    font.weight: Font.DemiBold
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    Layout.fillWidth: true
                                    text: artifactDelegate.modelData.name
                                    color: Theme.text
                                    font.pixelSize: 13
                                    font.weight: artifactDelegate.highlighted
                                                 ? Font.DemiBold : Font.Normal
                                    elide: Text.ElideRight
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: artifactDelegate.modelData.kindLabel
                                    color: Theme.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }

                Text {
                    visible: window.backend.artifactCount === 0
                    Layout.fillWidth: true
                    text: window.backend.projectOpen
                          ? qsTr("This project has no artifacts")
                          : qsTr("No project loaded")
                    color: Theme.muted
                    wrapMode: Text.WordWrap
                    font.pixelSize: Theme.fontBody
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.border
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Stable identities\nImmutable accepted revisions")
                    color: Theme.muted
                    lineHeight: 1.35
                    wrapMode: Text.WordWrap
                    font.pixelSize: 10
                }
            }
        }

        ColumnLayout {
            Layout.minimumWidth: 520
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

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
                Layout.preferredHeight: 188
            }
        }

        ColumnLayout {
            Layout.minimumWidth: 320
            Layout.preferredWidth: 320
            Layout.maximumWidth: 320
            Layout.fillHeight: true
            spacing: 12

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
                Layout.preferredHeight: 252
                hasAcceptedRevision: window.hasSelectedArtifact
                                     && window.selectedArtifact.hasAcceptedRevision
                revisionId: window.hasSelectedArtifact
                            ? window.selectedArtifact.acceptedRevisionId
                            : ""
            }
        }
    }
}

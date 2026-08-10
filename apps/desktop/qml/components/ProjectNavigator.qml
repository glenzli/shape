pragma ComponentBehavior: Bound

//! Project-level artifact navigation. Creative relationships remain summarized here;
//! media-internal structure belongs to the selected workspace.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: navigator

    property bool projectOpen: false
    property var artifacts: []
    property int selectedIndex: 0
    property bool hasCandidate: false
    property string candidateArtifactId: ""
    property bool graphActive: false

    readonly property int acceptedCount: {
        let count = 0
        for (let index = 0; index < artifacts.length; ++index) {
            if (artifacts[index].hasAcceptedRevision) {
                ++count
            }
        }
        return count
    }

    signal artifactSelected(int index)
    signal graphRequested()

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
                text: qsTr("PROJECT")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
                font.letterSpacing: 0.8
            }

            Item { Layout.fillWidth: true }

            Rectangle {
                Layout.preferredWidth: artifactCountLabel.implicitWidth + 14
                Layout.preferredHeight: 22
                radius: 11
                color: Theme.raised
                border.color: Theme.border

                Text {
                    id: artifactCountLabel
                    anchors.centerIn: parent
                    text: navigator.artifacts.length
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: qsTr("CREATIVE OBJECTS")
                color: Theme.textSoft
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 0.5
            }

            Item { Layout.fillWidth: true }

            Text {
                text: qsTr("Current heads")
                color: Theme.muted
                font.pixelSize: 9
            }
        }

        ListView {
            id: artifactList

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 6
            model: navigator.artifacts

            delegate: ItemDelegate {
                id: artifactDelegate

                required property int index
                required property var modelData
                readonly property bool candidatePending: navigator.hasCandidate
                                                         && navigator.candidateArtifactId
                                                            === modelData.id

                width: ListView.view.width
                height: 68
                leftPadding: 10
                rightPadding: 10
                highlighted: navigator.selectedIndex === index
                onClicked: navigator.artifactSelected(index)

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
                        Layout.preferredWidth: 32
                        Layout.preferredHeight: 32
                        radius: 9
                        color: artifactDelegate.highlighted ? Theme.accentSoft : Theme.raised
                        border.color: artifactDelegate.candidatePending ? Theme.accent : Theme.border

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
                        spacing: 3

                        Text {
                            Layout.fillWidth: true
                            text: artifactDelegate.modelData.name
                            color: Theme.text
                            font.pixelSize: 13
                            font.weight: artifactDelegate.highlighted ? Font.DemiBold : Font.Medium
                            elide: Text.ElideRight
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 5

                            Text {
                                text: "●"
                                color: artifactDelegate.candidatePending
                                       ? Theme.accent
                                       : artifactDelegate.modelData.hasAcceptedRevision
                                         ? Theme.success : Theme.muted
                                font.pixelSize: 7
                            }

                            Text {
                                Layout.fillWidth: true
                                text: artifactDelegate.candidatePending
                                      ? qsTr("Candidate pending")
                                      : artifactDelegate.modelData.hasAcceptedRevision
                                        ? qsTr("Accepted head")
                                        : qsTr("Awaiting first revision")
                                color: artifactDelegate.candidatePending ? Theme.accent : Theme.muted
                                font.pixelSize: 9
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }
        }

        Text {
            visible: navigator.artifacts.length === 0
            Layout.fillWidth: true
            text: navigator.projectOpen ? qsTr("This project has no creative objects")
                                        : qsTr("No project loaded")
            color: Theme.muted
            wrapMode: Text.WordWrap
            font.pixelSize: Theme.fontBody
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 82
            radius: Theme.radiusMedium
            color: navigator.graphActive ? Theme.accentSoft
                                         : graphMouse.hovered ? Theme.raisedHover : Theme.raised
            border.color: navigator.graphActive ? Theme.accent : Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: qsTr("CREATIVE GRAPH")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.5
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: qsTr("%1 accepted").arg(navigator.acceptedCount)
                        color: Theme.success
                        font.pixelSize: 9
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Candidates stay outside the graph until accepted.")
                    color: Theme.muted
                    font.pixelSize: 9
                    wrapMode: Text.WordWrap
                    lineHeight: 1.25
                }
            }

            MouseArea {
                id: graphMouse

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                Accessible.name: qsTr("Open project graph")
                Accessible.role: Accessible.Button
                onClicked: navigator.graphRequested()
            }
        }
    }
}

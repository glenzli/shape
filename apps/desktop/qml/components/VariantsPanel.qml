pragma ComponentBehavior: Bound

//! Artifact-scoped Candidate Shelf. Candidate identity and bytes come from the
//! Rust session; this component owns only selection and review interaction.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: variants
    objectName: "candidateShelf"

    property string artifactText: ""
    property bool hasAcceptedRevision: false
    property bool hasTextPreview: false
    property string acceptedRevisionId: ""
    property var candidates: []
    property string selectedCandidateId: ""

    readonly property var selectedCandidate: {
        for (let index = 0; index < candidates.length; ++index) {
            if (candidates[index].id === selectedCandidateId) {
                return candidates[index]
            }
        }
        return candidates.length > 0 ? candidates[0] : null
    }
    readonly property bool hasCandidate: selectedCandidate !== null

    signal compareRequested()
    signal candidateSelected(string candidateId)
    signal discardRequested(string candidateId)
    signal acceptRequested(string candidateId)
    signal branchRequested(string candidateId)

    function select(candidateId) : void {
        candidateSelected(candidateId)
    }

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    text: qsTr("CANDIDATE SHELF")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }

                Text {
                    text: qsTr("Explore before committing history")
                    color: Theme.disabled
                    font.pixelSize: 9
                }
            }

            Rectangle {
                Layout.preferredWidth: queueLabel.implicitWidth + 14
                Layout.preferredHeight: 22
                radius: 11
                color: variants.hasCandidate ? Theme.accentSoft : Theme.raised
                border.color: variants.hasCandidate ? Theme.accent : Theme.border

                Text {
                    id: queueLabel
                    anchors.centerIn: parent
                    text: qsTr("%1 pending").arg(variants.candidates.length)
                    color: variants.hasCandidate ? Theme.accent : Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 90
            visible: variants.hasAcceptedRevision
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: qsTr("CURRENT ACCEPTED")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: qsTr("Durable · verified")
                        color: Theme.success
                        font.pixelSize: 9
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    text: variants.hasTextPreview ? variants.artifactText
                                                  : qsTr("Accepted non-text content")
                    color: Theme.textSoft
                    font.pixelSize: 12
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: variants.hasCandidate

            Text {
                text: qsTr("TRANSIENT OPTIONS")
                color: Theme.textSoft
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.5
            }

            Item { Layout.fillWidth: true }

            ShapeButton {
                implicitHeight: 26
                text: qsTr("Compare")
                onClicked: variants.compareRequested()
            }
        }

        ListView {
            id: candidateList

            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: variants.hasCandidate
            clip: true
            spacing: 7
            model: variants.candidates

            ScrollBar.vertical: ScrollBar {
                policy: candidateList.contentHeight > candidateList.height
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: ItemDelegate {
                id: candidateDelegate

                required property int index
                required property var modelData
                readonly property bool selected: variants.selectedCandidate !== null
                                                 && variants.selectedCandidate.id === modelData.id
                readonly property bool currentHead: modelData.hasExpectedHead
                                                    ? modelData.expectedHead
                                                      === variants.acceptedRevisionId
                                                    : variants.acceptedRevisionId.length === 0

                width: ListView.view.width
                height: 86
                leftPadding: 11
                rightPadding: 11
                topPadding: 9
                bottomPadding: 9
                highlighted: selected
                Accessible.name: qsTr("Select candidate %1").arg(index + 1)
                onClicked: variants.select(modelData.id)

                background: Rectangle {
                    radius: Theme.radiusMedium
                    color: candidateDelegate.selected ? Theme.accentSoft
                                                      : candidateDelegate.hovered
                                                        ? Theme.raisedHover : Theme.raised
                    border.width: candidateDelegate.selected ? 2 : 1
                    border.color: candidateDelegate.selected ? Theme.accent : Theme.border
                }

                contentItem: ColumnLayout {
                    spacing: 4

                    RowLayout {
                        Layout.fillWidth: true

                        Text {
                            text: qsTr("OPTION %1").arg(candidateDelegate.index + 1)
                            color: candidateDelegate.selected ? Theme.accent : Theme.muted
                            font.pixelSize: 9
                            font.weight: Font.DemiBold
                            font.letterSpacing: 0.5
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            text: candidateDelegate.currentHead
                                  ? qsTr("Ready to accept") : qsTr("Earlier revision")
                            color: candidateDelegate.currentHead ? Theme.success : Theme.danger
                            font.pixelSize: 9
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: candidateDelegate.modelData.text
                        color: Theme.text
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                    }

                    Text {
                        Layout.fillWidth: true
                        text: candidateDelegate.modelData.textTruncated
                              ? qsTr("Preview truncated") : qsTr("Transient · not in history")
                        color: Theme.disabled
                        font.pixelSize: 8
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !variants.hasCandidate
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 5

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: variants.hasAcceptedRevision
                          ? qsTr("No pending candidates") : qsTr("No accepted or pending variants")
                    color: Theme.textSoft
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("Edit the text draft to create an option")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: variants.hasCandidate
            spacing: 8

            ShapeButton {
                Layout.fillWidth: true
                text: qsTr("Branch selected option")
                onClicked: variants.branchRequested(variants.selectedCandidate.id)
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                ShapeButton {
                    Layout.fillWidth: true
                    text: qsTr("Discard")
                    onClicked: variants.discardRequested(variants.selectedCandidate.id)
                }

                ShapeButton {
                    Layout.fillWidth: true
                    text: qsTr("Accept selected")
                    primary: true
                    onClicked: variants.acceptRequested(variants.selectedCandidate.id)
                }
            }
        }
    }
}

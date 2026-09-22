pragma ComponentBehavior: Bound

//! Horizontal version-review surface for the focused Operator. Candidate
//! identity, acceptance, branching, and persistence remain outside QML.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: filmstrip
    objectName: "candidateFilmstrip"

    property var candidates: []
    property string selectedCandidateId: ""
    property string acceptedRevisionId: ""
    property string selectedPreviewSource: ""
    property var candidateThumbnailSource: null
    property bool mutationEnabled: true
    property bool compareAvailable: selectedCandidate !== null

    readonly property var selectedCandidate: candidateForId(selectedCandidateId)

    signal candidateSelected(string candidateId)
    signal candidateReviewRequested(string candidateId)
    signal compareRequested(string candidateId)
    signal acceptRequested(string candidateId)
    signal discardRequested(string candidateId)
    signal branchRequested(string candidateId)

    function candidateForId(candidateId) : var {
        for (let index = 0; index < candidates.length; ++index) {
            if (String(candidates[index].id) === candidateId) return candidates[index]
        }
        return candidates.length > 0 ? candidates[0] : null
    }

    function select(candidateId) : void {
        candidateSelected(candidateId)
    }

    function isCurrentHead(candidate) : bool {
        if (!candidate) return false
        if (!candidate.hasExpectedHead) return acceptedRevisionId.length === 0
        return String(candidate.expectedHead) === acceptedRevisionId
    }

    function formatDuration(milliseconds) : string {
        const seconds = Math.max(0, Math.floor(Number(milliseconds) / 1000))
        const minutes = Math.floor(seconds / 60)
        const remainder = seconds % 60
        return minutes + ":" + (remainder < 10 ? "0" : "") + remainder
    }

    function candidateSummary(candidate, index) : string {
        if (candidate.hasImagePreview) {
            return qsTr("Image · %1 × %2").arg(candidate.imageWidth).arg(candidate.imageHeight)
        }
        if (candidate.hasAudioPreview) {
            return qsTr("Audio · %1").arg(formatDuration(candidate.audioDurationMillis))
        }
        if (candidate.text !== undefined && String(candidate.text).length > 0) {
            return String(candidate.text)
        }
        return qsTr("Version %1").arg(index + 1)
    }

    function reviewSelected() : void {
        if (selectedCandidate === null) return
        if (selectedCandidate.hasAudioPreview) {
            candidateReviewRequested(String(selectedCandidate.id))
        } else {
            compareRequested(String(selectedCandidate.id))
        }
    }

    implicitHeight: 154
    color: Theme.panel
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        anchors.topMargin: 10
        anchors.bottomMargin: 10
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: qsTr("NEW VERSIONS")
                color: Theme.textSoft
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
                font.letterSpacing: 0.7
            }

            Rectangle {
                Layout.preferredWidth: candidateCountLabel.implicitWidth + 12
                Layout.preferredHeight: 20
                radius: 10
                color: filmstrip.candidates.length > 0 ? Theme.accentSoft : Theme.raised

                Text {
                    id: candidateCountLabel
                    anchors.centerIn: parent
                    text: filmstrip.candidates.length
                    color: filmstrip.candidates.length > 0 ? Theme.accent : Theme.muted
                    font.pixelSize: 9
                }
            }

            Text {
                Layout.fillWidth: true
                text: filmstrip.candidates.length > 0
                      ? qsTr("Choose one only when it feels right")
                      : qsTr("Run this step to create a version")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }

            ShapeIconButton {
                source: filmstrip.selectedCandidate !== null
                        && filmstrip.selectedCandidate.hasAudioPreview
                        ? "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                        : "qrc:/qt/qml/Shape/Desktop/icons/compare.svg"
                toolTipText: filmstrip.selectedCandidate !== null
                             && filmstrip.selectedCandidate.hasAudioPreview
                             ? qsTr("Review this version") : qsTr("Compare this version")
                accessibleName: toolTipText
                buttonSize: 28
                enabled: filmstrip.compareAvailable
                onClicked: filmstrip.reviewSelected()
            }

            ShapeIconButton {
                source: "qrc:/qt/qml/Shape/Desktop/icons/trash.svg"
                toolTipText: qsTr("Remove this version")
                accessibleName: toolTipText
                enabled: filmstrip.mutationEnabled && filmstrip.selectedCandidate !== null
                onClicked: filmstrip.discardRequested(
                               String(filmstrip.selectedCandidate.id))
            }

            ShapeIconButton {
                source: "qrc:/qt/qml/Shape/Desktop/icons/export.svg"
                toolTipText: qsTr("Start a separate work from this version")
                accessibleName: toolTipText
                enabled: filmstrip.mutationEnabled && filmstrip.selectedCandidate !== null
                onClicked: filmstrip.branchRequested(
                               String(filmstrip.selectedCandidate.id))
            }

            ShapeIconButton {
                objectName: "filmstripAcceptButton"
                primary: true
                source: "qrc:/qt/qml/Shape/Desktop/icons/verified.svg"
                toolTipText: qsTr("Use this version")
                accessibleName: toolTipText
                buttonSize: 28
                enabled: filmstrip.mutationEnabled
                         && filmstrip.selectedCandidate !== null
                         && filmstrip.isCurrentHead(filmstrip.selectedCandidate)
                onClicked: filmstrip.acceptRequested(
                               String(filmstrip.selectedCandidate.id))
            }
        }

        ListView {
            id: candidateList

            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: ListView.Horizontal
            clip: true
            spacing: 7
            model: filmstrip.candidates

            ScrollBar.horizontal: ScrollBar {
                policy: candidateList.contentWidth > candidateList.width
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: ItemDelegate {
                id: candidateDelegate

                required property int index
                required property var modelData
                readonly property bool selected: filmstrip.selectedCandidate !== null
                                                 && String(filmstrip.selectedCandidate.id)
                                                    === String(modelData.id)
                readonly property bool currentHead: filmstrip.isCurrentHead(modelData)

                width: 196
                height: candidateList.height - 6
                leftPadding: 8
                rightPadding: 8
                topPadding: 7
                bottomPadding: 7
                highlighted: selected
                Accessible.name: qsTr("Select version %1").arg(index + 1)
                onClicked: filmstrip.select(String(modelData.id))
                onDoubleClicked: filmstrip.candidateReviewRequested(String(modelData.id))

                background: Rectangle {
                    radius: Theme.controlRadius
                    color: candidateDelegate.selected ? Theme.accentSoft
                                                      : candidateDelegate.hovered
                                                        ? Theme.raisedHover : Theme.raised
                    border.width: candidateDelegate.selected ? 2 : 1
                    border.color: candidateDelegate.selected ? Theme.accent : Theme.border
                }

                contentItem: RowLayout {
                    spacing: 8

                    Rectangle {
                        Layout.preferredWidth: 42
                        Layout.fillHeight: true
                        radius: 6
                        color: Theme.background
                        border.color: Theme.border
                        clip: true

                        Image {
                            id: candidateThumbnail
                            anchors.fill: parent
                            anchors.margins: 1
                            source: {
                                const refresh = filmstrip.selectedPreviewSource
                                const id = String(candidateDelegate.modelData.id)
                                const thumbnail = filmstrip.candidateThumbnailSource !== null
                                                  ? filmstrip.candidateThumbnailSource(id) : ""
                                return thumbnail.length > 0 ? thumbnail
                                       : candidateDelegate.selected ? refresh : ""
                            }
                            visible: source.toString().length > 0
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }

                        ShapeIcon {
                            anchors.centerIn: parent
                            visible: !candidateThumbnail.visible
                            source: candidateDelegate.modelData.hasAudioPreview
                                    ? "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                                    : candidateDelegate.modelData.hasImagePreview
                                      ? "qrc:/qt/qml/Shape/Desktop/icons/crop.svg"
                                      : "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
                            size: 17
                            color: candidateDelegate.selected ? Theme.accent : Theme.muted
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: qsTr("Option %1").arg(candidateDelegate.index + 1)
                                color: candidateDelegate.selected ? Theme.accent : Theme.textSoft
                            font.pixelSize: Theme.fontMeta
                                font.weight: Font.DemiBold
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: candidateDelegate.currentHead ? "●" : "○"
                                color: candidateDelegate.currentHead ? Theme.success : Theme.danger
                                font.pixelSize: 8
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            text: filmstrip.candidateSummary(candidateDelegate.modelData,
                                                             candidateDelegate.index)
                            color: Theme.muted
                            font.pixelSize: Theme.fontMicro
                            elide: Text.ElideRight
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                        }
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: filmstrip.candidates.length === 0
                text: qsTr("No new versions yet")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
            }
        }
    }
}

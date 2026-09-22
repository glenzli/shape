pragma ComponentBehavior: Bound

//! Compact presentation for one transient generated version. Full comparison
//! belongs to the review workspace rather than the graph.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: card

    required property var candidateData
    property int candidateIndex: 0
    property bool selected: false
    property bool hovered: false

    function candidateSummary(): string {
        if (candidateData.hasImagePreview) {
            return qsTr("Image · %1 × %2").arg(candidateData.imageWidth).arg(candidateData.imageHeight);
        }
        if (candidateData.hasAudioPreview)
            return qsTr("Audio ready for review");
        return candidateData.text;
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controlRadius
        color: card.selected ? Theme.accentSoft : card.hovered ? Theme.raisedHover : Theme.panelRaised
        border.width: card.selected ? 2 : 1
        border.color: Theme.accent
    }

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 3
        color: Theme.accent
    }

    Rectangle {
        anchors.left: parent.left
        anchors.leftMargin: -6
        anchors.verticalCenter: parent.verticalCenter
        width: 12
        height: 12
        radius: 6
        color: Theme.canvas
        border.width: 2
        border.color: Theme.accent
    }

    Item {
        anchors.fill: parent
        anchors.leftMargin: 13
        anchors.rightMargin: 12
        anchors.topMargin: 11
        anchors.bottomMargin: 10
        clip: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Rectangle {
                    Layout.preferredWidth: 26
                    Layout.preferredHeight: 26
                    radius: 8
                    color: Theme.accentSurfaceQuiet

                    ShapeIcon {
                        anchors.centerIn: parent
                        source: card.candidateData.hasAudioPreview ? "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg" : "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                        size: 14
                        color: Theme.accent
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("NEW VERSION")
                        color: Theme.accent
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.55
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Option %1").arg(card.candidateIndex + 1)
                        color: Theme.text
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                Layout.fillHeight: true
                text: card.candidateSummary()
                color: Theme.textSoft
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.Wrap
                elide: Text.ElideRight
                maximumLineCount: 2
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Review before using")
                color: Theme.accentSelectionText
                font.pixelSize: Theme.fontMicro
                font.weight: Font.Medium
                elide: Text.ElideRight
            }
        }
    }
}

pragma ComponentBehavior: Bound

//! Presentation for one transient generated version on the graph. Candidate
//! selection and acceptance remain outside this component.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: card

    required property var candidateData
    property int candidateIndex: 0
    property bool selected: false
    property bool hovered: false

    Rectangle {
        anchors.fill: parent
        radius: Theme.cardRadius
        color: card.selected ? Theme.accentSoft
                             : card.hovered ? Theme.raisedHover : Theme.panelRaised
        border.width: card.selected ? 2 : 1
        border.color: Theme.accent
        opacity: 0.98
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

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 17
        anchors.rightMargin: 14
        anchors.topMargin: 14
        anchors.bottomMargin: 13
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Rectangle {
                Layout.preferredWidth: 32
                Layout.preferredHeight: 32
                radius: 9
                color: Theme.accentSurfaceQuiet

                ShapeIcon {
                    anchors.centerIn: parent
                    source: card.candidateData.hasAudioPreview
                            ? "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                            : "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                    size: 17
                    color: Theme.accent
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    Layout.fillWidth: true
                    text: qsTr("NEW VERSION")
                    color: Theme.accent
                    font.pixelSize: Theme.fontMicro
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.65
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Option %1").arg(card.candidateIndex + 1)
                    color: Theme.text
                    font.pixelSize: Theme.fontHeading
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 54
            radius: Theme.controlRadius
            color: Theme.panelInset
            border.color: Theme.border

            Text {
                anchors.fill: parent
                anchors.margins: 11
                text: card.candidateData.hasImagePreview
                      ? qsTr("Image result · %1 × %2")
                          .arg(card.candidateData.imageWidth)
                          .arg(card.candidateData.imageHeight)
                      : card.candidateData.hasAudioPreview
                        ? qsTr("Audio result ready for review")
                        : card.candidateData.text
                color: Theme.textSoft
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.Wrap
                elide: Text.ElideRight
                maximumLineCount: 3
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Rectangle {
                Layout.preferredWidth: 6
                Layout.preferredHeight: 6
                radius: 3
                color: Theme.accent
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Review before using")
                color: Theme.accentSelectionText
                font.pixelSize: Theme.fontMeta
                font.weight: Font.Medium
            }
        }
    }
}

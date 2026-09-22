//! Side-by-side ownership for comparing accepted and transient text states.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: comparison

    property string acceptedText: ""
    property string candidateText: ""

    RowLayout {
        anchors.fill: parent
        spacing: 12

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 10

                Text {
                    text: qsTr("ACCEPTED")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                ShapeTextEditor {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    readOnly: true
                    text: comparison.acceptedText
                    color: Theme.textSoft
                    wrapMode: TextEdit.Wrap
                    font.pixelSize: 15
                    background: null
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: Theme.radiusMedium
            color: Theme.accentSoft
            border.color: Theme.accent

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 10

                Text {
                    text: qsTr("CANDIDATE")
                    color: Theme.accent
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                ShapeTextEditor {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    readOnly: true
                    text: comparison.candidateText
                    color: Theme.text
                    wrapMode: TextEdit.Wrap
                    font.pixelSize: 15
                    background: null
                }
            }
        }
    }
}

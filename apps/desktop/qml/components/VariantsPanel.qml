import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: variants

    property string artifactText: ""
    property bool hasAcceptedRevision: false
    property bool hasTextPreview: false
    property bool hasCandidate: false
    property string candidateText: ""
    property bool candidateTextTruncated: false

    signal compareRequested()
    signal discardRequested()
    signal acceptRequested()

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: qsTr("VARIANTS")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
                font.letterSpacing: 0.7
            }

            Item { Layout.fillWidth: true }

            Rectangle {
                Layout.preferredWidth: queueLabel.implicitWidth + 14
                Layout.preferredHeight: 22
                radius: 11
                color: Theme.raised
                border.color: Theme.border

                Text {
                    id: queueLabel
                    anchors.centerIn: parent
                    text: variants.hasCandidate ? qsTr("1 pending") : qsTr("0 pending")
                    color: variants.hasCandidate ? Theme.accent : Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 104
            visible: variants.hasAcceptedRevision
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 13
                spacing: 5

                Text {
                    text: qsTr("CURRENT ACCEPTED")
                    color: Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.6
                }

                Text {
                    Layout.fillWidth: true
                    text: variants.hasTextPreview ? variants.artifactText
                                                  : qsTr("Accepted non-text content")
                    color: Theme.text
                    font.pixelSize: 13
                    elide: Text.ElideRight
                }

                Item { Layout.fillHeight: true }

                Text { text: qsTr("Durable · verified"); color: Theme.success; font.pixelSize: 10 }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 116
            visible: variants.hasCandidate
            radius: Theme.radiusMedium
            color: Theme.accentSoft
            border.color: Theme.accent

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 13
                spacing: 5

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: qsTr("TEXT CANDIDATE")
                        color: Theme.accent
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    Item { Layout.fillWidth: true }

                    ShapeButton {
                        implicitHeight: 26
                        text: qsTr("Compare")
                        onClicked: variants.compareRequested()
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: variants.candidateText
                    color: Theme.text
                    font.pixelSize: 13
                    elide: Text.ElideRight
                }

                Item { Layout.fillHeight: true }

                Text {
                    text: variants.candidateTextTruncated ? qsTr("Preview truncated")
                                                          : qsTr("Transient · not in history")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 112
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
                    text: qsTr("Edit the text draft to create a candidate")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Item { Layout.fillHeight: true }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ShapeButton {
                visible: variants.hasCandidate
                Layout.fillWidth: true
                text: qsTr("Discard")
                onClicked: variants.discardRequested()
            }

            ShapeButton {
                Layout.fillWidth: true
                text: qsTr("Accept candidate")
                primary: true
                enabled: variants.hasCandidate
                onClicked: variants.acceptRequested()
            }
        }
    }
}

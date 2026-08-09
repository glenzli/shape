import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: variants

    property string artifactText: ""
    property bool hasAcceptedRevision: false
    property bool hasTextPreview: false

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
                    text: qsTr("0 pending")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 112
            visible: variants.hasAcceptedRevision
            radius: Theme.radiusMedium
            color: Theme.accentSoft
            border.color: Theme.accent

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 13
                spacing: 5

                Text {
                    text: qsTr("CURRENT ACCEPTED")
                    color: Theme.accent
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

                Text {
                    text: qsTr("Durable · verified")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 112
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
                    text: qsTr("Run an executor to explore alternatives")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Item { Layout.fillHeight: true }

        ShapeButton {
            Layout.fillWidth: true
            text: qsTr("Accept selected variant")
            primary: true
            enabled: false
        }
    }
}

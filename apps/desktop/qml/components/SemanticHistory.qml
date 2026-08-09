import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: history

    property bool hasAcceptedRevision: false
    property string revisionId: ""

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        Text {
            text: qsTr("SEMANTIC HISTORY")
            color: Theme.muted
            font.pixelSize: Theme.fontMeta
            font.weight: Font.DemiBold
            font.letterSpacing: 0.7
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 66
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 10

                Rectangle {
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    radius: 15
                    color: history.hasAcceptedRevision ? Theme.accentSoft : Theme.surface
                    border.color: history.hasAcceptedRevision ? Theme.accent : Theme.border

                    Text {
                        anchors.centerIn: parent
                        text: history.hasAcceptedRevision ? "✓" : "·"
                        color: history.hasAcceptedRevision ? Theme.accent : Theme.muted
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        Layout.fillWidth: true
                        text: history.hasAcceptedRevision
                              ? qsTr("Accepted revision") : qsTr("No accepted history")
                        color: Theme.text
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        text: history.hasAcceptedRevision ? history.revisionId : qsTr("Waiting for the first accepted change")
                        color: history.hasAcceptedRevision ? Theme.accent : Theme.muted
                        font.pixelSize: 10
                        elide: Text.ElideMiddle
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Every accepted transformation will form a durable, inspectable timeline.")
            color: Theme.muted
            font.pixelSize: 10
            lineHeight: 1.35
            wrapMode: Text.WordWrap
        }

        Item { Layout.fillHeight: true }
    }
}

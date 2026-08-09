import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Frame {
    id: history
    property bool hasAcceptedRevision: false
    property string revisionId: ""
    padding: 14
    background: Rectangle {
        color: Theme.surface
        radius: Theme.radiusLarge
        border.color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 10
        Label {
            text: qsTr("SEMANTIC HISTORY")
            color: Theme.muted
            font.pixelSize: 11
            font.bold: true
        }
        Label {
            text: history.hasAcceptedRevision ? qsTr("Accepted revision") : qsTr("No accepted history")
            color: Theme.text
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
        Label {
            visible: history.hasAcceptedRevision
            text: history.revisionId
            color: Theme.accent
            font.pixelSize: 11
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }
        Label {
            text: qsTr("Detailed transformation history will be connected through a separate bounded projection.")
            color: Theme.muted
            font.pixelSize: 11
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
        Item { Layout.fillHeight: true }
    }
}

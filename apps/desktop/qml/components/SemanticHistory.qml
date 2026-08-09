import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Frame {
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
        Label { text: qsTr("Make the atmosphere quiet"); color: Theme.text; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Label { text: qsTr("Preserved: summer afternoon subject"); color: Theme.accent; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Label { text: qsTr("Executed by Shape built-in text · accepted"); color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }
        Label { text: qsTr("Import the initial line"); color: Theme.text; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Label { text: qsTr("Imported · accepted"); color: Theme.muted; font.pixelSize: 11 }
        Item { Layout.fillHeight: true }
    }
}

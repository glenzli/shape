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
        spacing: 12
        Label {
            text: qsTr("VARIANTS")
            color: Theme.muted
            font.pixelSize: 11
            font.bold: true
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 92
            radius: Theme.radiusSmall
            color: Theme.accentSoft
            border.color: Theme.accent
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                Label { text: qsTr("Current accepted"); color: Theme.accent; font.bold: true }
                Label { text: qsTr("A quiet summer afternoon."); color: Theme.text }
                Label { text: qsTr("Durable · verified"); color: Theme.muted; font.pixelSize: 11 }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 92
            radius: Theme.radiusSmall
            color: Theme.raised
            border.color: Theme.border
            ColumnLayout {
                anchors.centerIn: parent
                Label { text: qsTr("No pending candidates"); color: Theme.text }
                Label { text: qsTr("Run an executor to explore alternatives"); color: Theme.muted; font.pixelSize: 11 }
            }
        }
        Item { Layout.fillHeight: true }
        Button {
            Layout.fillWidth: true
            text: qsTr("Accept selected variant")
            enabled: false
        }
    }
}

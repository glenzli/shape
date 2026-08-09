import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Frame {
    padding: 0
    background: Rectangle {
        color: Theme.surface
        radius: Theme.radiusLarge
        border.color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 14
            Label {
                text: qsTr("CURRENT ARTIFACT")
                color: Theme.muted
                font.pixelSize: 11
                font.bold: true
            }
            Item { Layout.fillWidth: true }
            Label {
                text: qsTr("Accepted head")
                color: Theme.success
                font.pixelSize: 12
            }
        }

        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 80, 620)
                spacing: 18
                Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("A quiet summer afternoon.")
                    color: Theme.text
                    font.pixelSize: 30
                    font.weight: Font.Light
                }
                Label {
                    Layout.fillWidth: true
                    text: qsTr("The workspace renders the accepted artifact. Executor output stays a candidate until you explicitly accept it.")
                    color: Theme.muted
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    font.pixelSize: 13
                }
            }
        }
    }
}

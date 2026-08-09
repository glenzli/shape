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
            text: qsTr("INTENT")
            color: Theme.muted
            font.pixelSize: 11
            font.bold: true
        }
        TextArea {
            Layout.fillWidth: true
            Layout.fillHeight: true
            placeholderText: qsTr("Describe what should change and what must remain…")
            color: Theme.text
            placeholderTextColor: Theme.muted
            wrapMode: TextEdit.Wrap
            background: Rectangle {
                color: Theme.raised
                radius: Theme.radiusSmall
                border.color: Theme.border
            }
        }
        RowLayout {
            Label {
                text: qsTr("Constraints will appear here")
                color: Theme.accent
                font.pixelSize: 12
            }
            Item { Layout.fillWidth: true }
            Button {
                text: qsTr("Generate variants")
                enabled: false
                ToolTip.text: qsTr("An executor bridge will enable this action")
                ToolTip.visible: hovered
            }
        }
    }
}

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: qsTr("CREATIVE INTENT")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
                font.letterSpacing: 0.7
            }

            Item { Layout.fillWidth: true }

            Text {
                text: qsTr("Describe change · protect invariants")
                color: Theme.muted
                font.pixelSize: 10
            }
        }

        TextArea {
            Layout.fillWidth: true
            Layout.fillHeight: true
            leftPadding: 12
            rightPadding: 12
            topPadding: 10
            bottomPadding: 10
            placeholderText: qsTr("Describe what should change and what must remain…")
            color: Theme.text
            placeholderTextColor: Theme.muted
            selectionColor: Theme.accentSoft
            selectedTextColor: Theme.text
            wrapMode: TextEdit.Wrap
            font.pixelSize: Theme.fontBody

            background: Rectangle {
                color: Theme.raised
                radius: Theme.radiusSmall
                border.color: parent.activeFocus ? Theme.accent : Theme.border
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: qsTr("Constraints will appear here")
                color: Theme.accent
                font.pixelSize: 11
            }

            Item { Layout.fillWidth: true }

            ShapeButton {
                text: qsTr("Generate variants")
                primary: true
                enabled: false
                ToolTip.text: qsTr("An executor bridge will enable this action")
                ToolTip.visible: hovered
            }
        }
    }
}

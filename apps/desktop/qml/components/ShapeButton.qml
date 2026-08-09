import QtQuick
import QtQuick.Controls
import Shape.Desktop

Button {
    id: control

    property bool primary: false
    property bool selected: false
    property bool quiet: !primary

    implicitHeight: Theme.controlHeight
    implicitWidth: Math.max(70, implicitContentWidth + 24)
    leftPadding: 12
    rightPadding: 12
    focusPolicy: Qt.StrongFocus

    background: Rectangle {
        radius: Theme.radiusSmall
        border.width: control.primary ? 0 : 1
        border.color: control.selected ? Theme.accent : Theme.border
        color: {
            if (!control.enabled) {
                return Theme.raised
            }
            if (control.primary) {
                return control.down ? Qt.darker(Theme.accent, 1.1)
                                    : control.hovered ? Theme.accentHover : Theme.accent
            }
            if (control.selected) {
                return Theme.accentSoft
            }
            return control.down ? Theme.selected
                                : control.hovered ? Theme.raisedHover : Theme.raised
        }

        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: parent.radius + 2
            color: "transparent"
            border.width: control.visualFocus ? 1 : 0
            border.color: Theme.accent
        }
    }

    contentItem: Text {
        text: control.text
        color: !control.enabled ? Theme.disabled
                                : control.primary ? Theme.accentText
                                                  : control.selected ? Theme.accent : Theme.textSoft
        font.pixelSize: Theme.fontBody
        font.weight: control.primary || control.selected ? Font.DemiBold : Font.Normal
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}

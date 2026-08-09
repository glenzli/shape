import QtQuick
import QtQuick.Controls
import Shape.Desktop

Button {
    id: control

    property url source
    property string toolTipText: ""
    property string accessibleName: toolTipText
    property bool selected: false
    property int buttonSize: 28
    property int iconSize: 17

    implicitWidth: buttonSize
    implicitHeight: buttonSize
    padding: 0
    hoverEnabled: true
    focusPolicy: enabled ? Qt.StrongFocus : Qt.NoFocus
    Accessible.name: accessibleName

    background: Rectangle {
        radius: 6
        color: {
            if (!control.enabled) return "transparent"
            if (control.selected) return Theme.accentSoft
            if (control.down) return Theme.selected
            if (control.hovered) return Theme.raisedHover
            return "transparent"
        }

        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            visible: control.enabled && control.visualFocus
            radius: parent.radius + 2
            color: "transparent"
            border.width: 1
            border.color: Theme.accent
        }
    }

    contentItem: ShapeIcon {
        anchors.centerIn: parent
        source: control.source
        size: control.iconSize
        opacity: control.enabled ? 1.0 : 0.34
        color: !control.enabled ? Theme.disabled
                                : control.selected ? Theme.accent
                                                   : control.hovered ? Theme.text : Theme.textSoft
    }

    ToolTip {
        id: toolTip
        parent: control
        visible: control.hovered && !control.down && control.toolTipText.length > 0
        delay: 450
        timeout: 3500
        text: control.toolTipText
        x: Math.round((control.width - width) / 2)
        y: control.height + 6

        contentItem: Text {
            text: toolTip.text
            color: Theme.text
            font.pixelSize: Theme.fontMeta
        }

        background: Rectangle {
            radius: Theme.radiusSmall
            color: Theme.raised
            border.color: Theme.borderStrong
        }
    }
}

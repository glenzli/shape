pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects
import Shape.Desktop

Item {
    id: root

    property url source
    property color color: Theme.textSoft
    property int size: 18

    implicitWidth: size
    implicitHeight: size

    Image {
        id: image

        anchors.centerIn: parent
        width: Math.min(root.width, root.size)
        height: Math.min(root.height, root.size)
        source: root.source
        sourceSize: Qt.size(
            Math.ceil(root.size * Screen.devicePixelRatio),
            Math.ceil(root.size * Screen.devicePixelRatio)
        )
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true

        layer.enabled: status === Image.Ready
        layer.smooth: true
        layer.effect: MultiEffect {
            colorization: 1.0
            colorizationColor: root.color
        }
    }

    Behavior on color {
        ColorAnimation { duration: 80 }
    }
}

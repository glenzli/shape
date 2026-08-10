//! Direct raster presentation and one bounded crop gesture lifecycle.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: raster
    objectName: "imageRasterWorkspace"

    property string artifactId: ""
    property string source: ""
    property int sourceWidth: 0
    property int sourceHeight: 0
    property int cropX: 0
    property int cropY: 0
    property int cropWidth: 0
    property int cropHeight: 0

    readonly property bool ready: source.length > 0 && sourceWidth > 0 && sourceHeight > 0
    readonly property real displayScale: ready && preview.paintedWidth > 0
                                         ? preview.paintedWidth / sourceWidth : 1
    readonly property real imageLeft: preview.x + (preview.width - preview.paintedWidth) / 2
    readonly property real imageTop: preview.y + (preview.height - preview.paintedHeight) / 2
    readonly property bool cropChangesRaster: ready && (cropX !== 0 || cropY !== 0
                                                        || cropWidth !== sourceWidth
                                                        || cropHeight !== sourceHeight)

    signal cropRequested(int x, int y, int width, int height)

    function resetCrop() : void {
        if (!ready) {
            cropX = 0
            cropY = 0
            cropWidth = 0
            cropHeight = 0
            return
        }
        const margin = Math.max(1, Math.floor(Math.min(sourceWidth, sourceHeight) * 0.08))
        cropX = Math.min(margin, sourceWidth - 1)
        cropY = Math.min(margin, sourceHeight - 1)
        cropWidth = Math.max(1, sourceWidth - cropX * 2)
        cropHeight = Math.max(1, sourceHeight - cropY * 2)
    }

    onArtifactIdChanged: resetCrop()
    onSourceWidthChanged: resetCrop()
    onSourceHeightChanged: resetCrop()
    onReadyChanged: {
        if (ready) {
            resetCrop()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 10

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: Theme.radiusMedium
            color: Theme.effectiveDark ? "#151719" : "#e7e9eb"
            border.color: Theme.border
            clip: true

            Image {
                id: preview
                anchors.fill: parent
                anchors.margins: 16
                source: raster.source
                fillMode: Image.PreserveAspectFit
                asynchronous: true
                cache: true
            }

            Rectangle {
                id: cropFrame
                visible: raster.ready && preview.status === Image.Ready
                x: raster.imageLeft + raster.cropX * raster.displayScale
                y: raster.imageTop + raster.cropY * raster.displayScale
                width: raster.cropWidth * raster.displayScale
                height: raster.cropHeight * raster.displayScale
                color: "transparent"
                border.width: 2
                border.color: Theme.accent

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 2
                    color: Theme.accent
                    opacity: 0.07
                }

                MouseArea {
                    id: moveArea
                    anchors.fill: parent
                    anchors.rightMargin: 18
                    anchors.bottomMargin: 18
                    cursorShape: Qt.SizeAllCursor
                    property real startPointerX: 0
                    property real startPointerY: 0
                    property int startCropX: 0
                    property int startCropY: 0
                    onPressed: mouse => {
                        const pointer = moveArea.mapToItem(raster, mouse.x, mouse.y)
                        startPointerX = pointer.x
                        startPointerY = pointer.y
                        startCropX = raster.cropX
                        startCropY = raster.cropY
                    }
                    onPositionChanged: mouse => {
                        if (!pressed) return
                        const pointer = moveArea.mapToItem(raster, mouse.x, mouse.y)
                        const dx = Math.round((pointer.x - startPointerX) / raster.displayScale)
                        const dy = Math.round((pointer.y - startPointerY) / raster.displayScale)
                        raster.cropX = Math.max(0, Math.min(
                            raster.sourceWidth - raster.cropWidth, startCropX + dx))
                        raster.cropY = Math.max(0, Math.min(
                            raster.sourceHeight - raster.cropHeight, startCropY + dy))
                    }
                }

                Rectangle {
                    width: 16
                    height: 16
                    radius: 4
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.rightMargin: -8
                    anchors.bottomMargin: -8
                    color: Theme.accent
                    border.color: Theme.surface

                    MouseArea {
                        id: resizeArea

                        anchors.fill: parent
                        anchors.margins: -8
                        cursorShape: Qt.SizeFDiagCursor
                        property real startPointerX: 0
                        property real startPointerY: 0
                        property int startWidth: 0
                        property int startHeight: 0
                        onPressed: mouse => {
                            const pointer = resizeArea.mapToItem(raster, mouse.x, mouse.y)
                            startPointerX = pointer.x
                            startPointerY = pointer.y
                            startWidth = raster.cropWidth
                            startHeight = raster.cropHeight
                        }
                        onPositionChanged: mouse => {
                            if (!pressed) return
                            const pointer = resizeArea.mapToItem(raster, mouse.x, mouse.y)
                            const dw = Math.round((pointer.x - startPointerX) / raster.displayScale)
                            const dh = Math.round((pointer.y - startPointerY) / raster.displayScale)
                            raster.cropWidth = Math.max(1, Math.min(
                                raster.sourceWidth - raster.cropX, startWidth + dw))
                            raster.cropHeight = Math.max(1, Math.min(
                                raster.sourceHeight - raster.cropY, startHeight + dh))
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Text {
                Layout.fillWidth: true
                text: raster.ready
                      ? qsTr("Crop %1 × %2 at %3, %4").arg(
                            raster.cropWidth).arg(raster.cropHeight).arg(
                            raster.cropX).arg(raster.cropY)
                      : qsTr("Loading verified raster…")
                color: Theme.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }

            ShapeButton {
                text: qsTr("Reset frame")
                enabled: raster.ready
                onClicked: raster.resetCrop()
            }

            ShapeButton {
                text: qsTr("Create crop candidate")
                primary: true
                enabled: raster.cropChangesRaster
                onClicked: raster.cropRequested(
                               raster.cropX, raster.cropY,
                               raster.cropWidth, raster.cropHeight)
            }
        }
    }
}

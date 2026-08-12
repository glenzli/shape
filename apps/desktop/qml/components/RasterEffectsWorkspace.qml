pragma ComponentBehavior: Bound

//! Deterministic transform, blur, and flattened drop-shadow tools. Authored
//! controls stay transient; exact accepted parameters live in Transformation.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: workspace
    objectName: "rasterEffectsWorkspace"

    property string artifactId: ""
    property url source
    property int blurRadius: 8
    property int sharpenRadius: 2
    property int sharpenAmountPercent: 100
    property int sharpenThreshold: 4
    property int shadowOffsetX: 12
    property int shadowOffsetY: 12
    property int shadowBlurRadius: 12
    property int shadowOpacity: 128

    signal transformRequested(string transformKey)
    signal blurRequested(int radius)
    signal unsharpMaskRequested(int radius, int amountMilli, int threshold)
    signal dropShadowRequested(int offsetX, int offsetY, int blurRadius,
                               int red, int green, int blue, int alpha)

    RowLayout {
        anchors.fill: parent
        spacing: 16

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 280
            radius: Theme.radiusLarge
            color: Theme.effectiveDark ? "#151719" : "#e7e9eb"
            border.color: Theme.border
            clip: true

            Image {
                anchors.fill: parent
                anchors.margins: 28
                source: workspace.source
                fillMode: Image.PreserveAspectFit
                asynchronous: true
                cache: false
            }

            Rectangle {
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: 14
                width: previewLabel.implicitWidth + 18
                height: 28
                radius: 14
                color: Theme.surface
                border.color: Theme.borderStrong

                Text {
                    id: previewLabel
                    anchors.centerIn: parent
                    text: qsTr("Accepted image")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 306
            Layout.fillHeight: true
            radius: Theme.radiusLarge
            color: Theme.surface
            border.color: Theme.border

            ScrollView {
                anchors.fill: parent
                anchors.margins: 18
                contentWidth: availableWidth
                clip: true

                ColumnLayout {
                    width: parent.width
                    spacing: 12

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("TRANSFORM & EFFECTS")
                        color: Theme.text
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Each action creates a reviewable Candidate. Accepted history changes only after you lock it.")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WordWrap
                        lineHeight: 1.25
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                    Text {
                        text: qsTr("Orientation")
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMeta
                        font.weight: Font.DemiBold
                    }

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 6
                        rowSpacing: 6

                        ShapeButton {
                            objectName: "rasterRotateLeftButton"
                            Layout.fillWidth: true
                            text: qsTr("Rotate left")
                            onClicked: workspace.transformRequested("rotate270_clockwise")
                        }
                        ShapeButton {
                            objectName: "rasterRotateRightButton"
                            Layout.fillWidth: true
                            text: qsTr("Rotate right")
                            onClicked: workspace.transformRequested("rotate90_clockwise")
                        }
                        ShapeButton {
                            objectName: "rasterFlipHorizontalButton"
                            Layout.fillWidth: true
                            text: qsTr("Flip horizontal")
                            onClicked: workspace.transformRequested("flip_horizontal")
                        }
                        ShapeButton {
                            objectName: "rasterFlipVerticalButton"
                            Layout.fillWidth: true
                            text: qsTr("Flip vertical")
                            onClicked: workspace.transformRequested("flip_vertical")
                        }
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                    Text {
                        text: qsTr("Gaussian blur")
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMeta
                        font.weight: Font.DemiBold
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        Text {
                            text: qsTr("Radius")
                            color: Theme.muted
                            font.pixelSize: Theme.fontMeta
                        }
                        SpinBox {
                            objectName: "rasterBlurRadiusInput"
                            Layout.fillWidth: true
                            from: 1
                            to: 64
                            editable: true
                            value: workspace.blurRadius
                            onValueModified: workspace.blurRadius = value
                        }
                    }

                    ShapeButton {
                        objectName: "rasterBlurCandidateButton"
                        Layout.fillWidth: true
                        text: qsTr("Create blur Candidate")
                        onClicked: workspace.blurRequested(workspace.blurRadius)
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                    Text {
                        text: qsTr("Unsharp mask")
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMeta
                        font.weight: Font.DemiBold
                    }

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 8
                        rowSpacing: 6

                        Text { text: qsTr("Radius"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterSharpenRadiusInput"
                            Layout.fillWidth: true
                            from: 1
                            to: 64
                            editable: true
                            value: workspace.sharpenRadius
                            onValueModified: workspace.sharpenRadius = value
                        }
                        Text { text: qsTr("Amount (%)"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterSharpenAmountInput"
                            Layout.fillWidth: true
                            from: 1
                            to: 400
                            editable: true
                            value: workspace.sharpenAmountPercent
                            onValueModified: workspace.sharpenAmountPercent = value
                        }
                        Text { text: qsTr("Threshold"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterSharpenThresholdInput"
                            Layout.fillWidth: true
                            from: 0
                            to: 255
                            editable: true
                            value: workspace.sharpenThreshold
                            onValueModified: workspace.sharpenThreshold = value
                        }
                    }

                    ShapeButton {
                        objectName: "rasterSharpenCandidateButton"
                        Layout.fillWidth: true
                        text: qsTr("Create sharpen Candidate")
                        onClicked: workspace.unsharpMaskRequested(
                                       workspace.sharpenRadius,
                                       workspace.sharpenAmountPercent * 10,
                                       workspace.sharpenThreshold)
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                    Text {
                        text: qsTr("Drop shadow")
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMeta
                        font.weight: Font.DemiBold
                    }

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 8
                        rowSpacing: 6

                        Text { text: qsTr("Horizontal offset"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterShadowOffsetXInput"
                            Layout.fillWidth: true
                            from: -4096
                            to: 4096
                            editable: true
                            value: workspace.shadowOffsetX
                            onValueModified: workspace.shadowOffsetX = value
                        }
                        Text { text: qsTr("Vertical offset"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterShadowOffsetYInput"
                            Layout.fillWidth: true
                            from: -4096
                            to: 4096
                            editable: true
                            value: workspace.shadowOffsetY
                            onValueModified: workspace.shadowOffsetY = value
                        }
                        Text { text: qsTr("Blur radius"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterShadowBlurInput"
                            Layout.fillWidth: true
                            from: 0
                            to: 64
                            editable: true
                            value: workspace.shadowBlurRadius
                            onValueModified: workspace.shadowBlurRadius = value
                        }
                        Text { text: qsTr("Opacity"); color: Theme.muted; font.pixelSize: Theme.fontMeta }
                        SpinBox {
                            objectName: "rasterShadowOpacityInput"
                            Layout.fillWidth: true
                            from: 1
                            to: 255
                            editable: true
                            value: workspace.shadowOpacity
                            onValueModified: workspace.shadowOpacity = value
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("The first desktop control uses a black shadow. The underlying operation already preserves exact RGBA tint parameters.")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WordWrap
                        lineHeight: 1.2
                    }

                    ShapeButton {
                        objectName: "rasterShadowCandidateButton"
                        Layout.fillWidth: true
                        text: qsTr("Create shadow Candidate")
                        onClicked: workspace.dropShadowRequested(
                                       workspace.shadowOffsetX,
                                       workspace.shadowOffsetY,
                                       workspace.shadowBlurRadius,
                                       0, 0, 0, workspace.shadowOpacity)
                    }
                }
            }
        }
    }
}

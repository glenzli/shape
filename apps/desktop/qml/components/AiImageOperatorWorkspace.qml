pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: workspace
    objectName: "aiImageOperatorWorkspace"

    property string instruction: ""
    property int outputWidth: 1024
    property int outputHeight: 1024
    property string acceptedSource: ""
    property string candidateSource: ""
    property bool running: false
    property string errorCode: ""

    signal canvasRequested(int width, int height)

    readonly property string previewSource: candidateSource.length > 0
                                            ? candidateSource : acceptedSource

    radius: Theme.radiusMedium
    color: Theme.raised
    border.color: Theme.border
    clip: true

    RowLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 14
        spacing: 8

        Rectangle {
            Layout.preferredWidth: sourceBadge.implicitWidth + 18
            Layout.preferredHeight: 26
            radius: 13
            color: Theme.accentSoft
            border.color: Theme.accent

            Text {
                id: sourceBadge
                anchors.centerIn: parent
                text: qsTr("AI IMAGE · STARTING POINT")
                color: Theme.accent
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.5
            }
        }

        Item { Layout.fillWidth: true }

        ShapeComboBox {
            id: canvasPicker
            objectName: "aiImageCanvasPicker"
            Layout.preferredWidth: 178
            enabled: !workspace.running
            model: [
                qsTr("Square · 1024"),
                qsTr("Landscape · 1536 × 1024"),
                qsTr("Portrait · 1024 × 1536")
            ]
            currentIndex: workspace.outputWidth === 1536 ? 1
                          : workspace.outputHeight === 1536 ? 2 : 0
            onActivated: index => {
                if (index === 1) workspace.canvasRequested(1536, 1024)
                else if (index === 2) workspace.canvasRequested(1024, 1536)
                else workspace.canvasRequested(1024, 1024)
            }
        }
    }

    Item {
        anchors.fill: parent
        anchors.leftMargin: 28
        anchors.rightMargin: 28
        anchors.topMargin: 58
        anchors.bottomMargin: 42

        Rectangle {
            id: canvas
            anchors.centerIn: parent
            width: {
                const availableRatio = parent.width / Math.max(1, parent.height)
                const canvasRatio = workspace.outputWidth / Math.max(1, workspace.outputHeight)
                return canvasRatio > availableRatio ? parent.width : parent.height * canvasRatio
            }
            height: width * workspace.outputHeight / Math.max(1, workspace.outputWidth)
            radius: Theme.radiusSmall
            color: Theme.surface
            border.color: Theme.borderStrong
            clip: true

            Image {
                anchors.fill: parent
                source: workspace.previewSource
                visible: source.toString().length > 0
                fillMode: Image.PreserveAspectFit
                smooth: true
                mipmap: true
            }

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(320, canvas.width - 32)
                visible: workspace.previewSource.length === 0
                spacing: 10

                ShapeIcon {
                    Layout.alignment: Qt.AlignHCenter
                    source: "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                    size: 30
                    color: Theme.accent
                }

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: workspace.running ? qsTr("Creating a new version…")
                                            : qsTr("Your generated image will appear here")
                    color: Theme.textSoft
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: qsTr("%1 × %2 · starts from your description")
                          .arg(workspace.outputWidth).arg(workspace.outputHeight)
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }
        }
    }

    Text {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 14
        text: workspace.candidateSource.length > 0
              ? qsTr("A new version is ready. Your current result has not changed.")
              : qsTr("Your description is saved before AI generation begins.")
        color: Theme.muted
        font.pixelSize: 9
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
    }
}

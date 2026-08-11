pragma ComponentBehavior: Bound

//! Media- and role-aware presentation for one accepted creative graph node.
//! The graph owner keeps layout and interaction; this owner makes Source,
//! creative steps, and the current Result visually mean different things.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: card
    objectName: "creativeGraphNodeCard-" + nodeData.roleKey

    required property var nodeData
    property bool selected: false
    property bool hovered: false
    property string artifactKindKey: ""
    property string artifactTextPreview: ""
    property bool artifactTextPreviewTruncated: false
    property url acceptedImageSource
    property int artifactImageWidth: 0
    property int artifactImageHeight: 0
    property int artifactAudioDurationMillis: 0
    property int artifactAudioSampleRateHz: 0
    property int artifactAudioChannels: 0
    property int stageNumber: 0

    signal outputNodeRequested()

    readonly property bool isSource: nodeData.roleKey === "source"
    readonly property bool isOutput: nodeData.roleKey === "output"
    readonly property bool isOperator: nodeData.roleKey === "operator"
    readonly property bool isImageEditor: isOperator
                                                  && (nodeData.operatorTypeKey === "image.crop"
                                                      || nodeData.operatorTypeKey
                                                         === "image.resize")
    readonly property bool isAiCreator: isOperator && nodeData.inputPorts.length === 0
                                                 && nodeData.operatorTypeKey.indexOf(".generate") > 0
    readonly property string boundDataType: portDataType()
    readonly property bool hasExactTextPreview: nodeData.hasTextPreview === true
    readonly property string exactTextPreview: hasExactTextPreview
                                                ? nodeData.textPreview : ""
    readonly property bool displaysMaterialPreview: (isSource || isOutput)
                                                    && (boundDataType === "text.document"
                                                        || boundDataType === "image.raster"
                                                        || boundDataType === "audio.clip")
    readonly property color semanticColor: isOutput ? Theme.success
                                                    : isOperator ? Theme.accent : Theme.muted

    function dataTypeLabel(dataTypeKey) : string {
        if (dataTypeKey === "text.document") return qsTr("Text")
        if (dataTypeKey === "image.raster") return qsTr("Image")
        if (dataTypeKey === "audio.clip") return qsTr("Audio")
        return qsTr("Creative content")
    }

    function roleLabel() : string {
        if (isSource) return qsTr("SOURCE")
        if (isOutput) return qsTr("CURRENT RESULT")
        if (isAiCreator) return qsTr("AI CREATION")
        return qsTr("EDITING STEP")
    }

    function title() : string {
        if (isSource) {
            if (boundDataType === "text.document") return qsTr("Text material")
            if (boundDataType === "image.raster") return qsTr("Image material")
            if (boundDataType === "audio.clip") return qsTr("Audio material")
            return qsTr("Starting material")
        }
        if (isOutput) {
            if (boundDataType === "text.document") return qsTr("Text result")
            if (boundDataType === "image.raster") return qsTr("Image result")
            if (boundDataType === "audio.clip") return qsTr("Audio result")
            return qsTr("Current result")
        }
        if (isImageEditor) return qsTr("Image editing")
        if (nodeData.operatorTypeKey === "text.edit"
                || nodeData.operatorTypeKey === "text.transform") return qsTr("AI text editor")
        if (nodeData.operatorTypeKey === "image.generate") return qsTr("AI image creation")
        return nodeData.operatorTypeLabel
    }

    function portDataType() : string {
        const ports = isSource ? nodeData.outputPorts : nodeData.inputPorts
        return ports.length > 0 ? ports[0].dataTypeKey : ""
    }

    function detail() : string {
        if (isOperator) {
            if (isImageEditor) return qsTr("Frame, size, and assisted adjustments")
            return nodeData.intent.length > 0 ? nodeData.intent : nodeData.operatorTypeLabel
        }
        return dataTypeLabel(portDataType())
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusLarge
        color: card.selected ? Theme.accentSoft
                             : card.hovered ? Theme.raisedHover : Theme.raised
        border.width: card.selected ? 2 : 1
        border.color: card.selected ? card.semanticColor : Theme.borderStrong
        clip: true

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 4
            color: card.semanticColor
        }

        Rectangle {
            visible: card.isOutput
            anchors.fill: parent
            color: Theme.success
            opacity: Theme.effectiveDark ? 0.045 : 0.035
        }
    }

    Rectangle {
        visible: card.nodeData.inputPorts.length > 0
        anchors.left: parent.left
        anchors.leftMargin: -5
        anchors.verticalCenter: parent.verticalCenter
        width: 10
        height: 10
        radius: 5
        color: Theme.surface
        border.width: 2
        border.color: card.semanticColor
    }

    Rectangle {
        visible: card.nodeData.outputPorts.length > 0
        anchors.right: parent.right
        anchors.rightMargin: -5
        anchors.verticalCenter: parent.verticalCenter
        width: 10
        height: 10
        radius: 5
        color: Theme.surface
        border.width: 2
        border.color: card.semanticColor
    }

    ShapeIconButton {
        objectName: "nodeOutputAddButton"
        visible: card.nodeData.outputPorts.length > 0 && (card.hovered || card.selected)
        anchors.right: parent.right
        anchors.rightMargin: -17
        anchors.verticalCenter: parent.verticalCenter
        width: 28
        height: 28
        source: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
        toolTipText: qsTr("Add a node from this output")
        accessibleName: toolTipText
        onClicked: card.outputNodeRequested()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 16
        anchors.rightMargin: 13
        anchors.topMargin: 12
        anchors.bottomMargin: 12
        spacing: 6

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                text: card.roleLabel()
                color: card.semanticColor
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.6
            }

            Item { Layout.fillWidth: true }

            Text {
                visible: card.isOperator
                text: qsTr("STEP %1").arg(card.stageNumber)
                color: Theme.disabled
                font.pixelSize: 8
            }
        }

        Text {
            Layout.fillWidth: true
            text: card.title()
            color: Theme.text
            font.pixelSize: 13
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }

        Rectangle {
            visible: card.displaysMaterialPreview
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 66
            radius: Theme.radiusSmall
            color: Theme.effectiveDark ? "#11151a" : "#eef1f4"
            border.color: Theme.border
            clip: true

            Image {
                visible: card.isOutput && card.boundDataType === "image.raster"
                         && card.acceptedImageSource.toString().length > 0
                anchors.fill: parent
                anchors.margins: 4
                source: card.acceptedImageSource
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                cache: false
            }

            Text {
                visible: card.boundDataType === "text.document"
                anchors.fill: parent
                anchors.margins: 9
                text: card.hasExactTextPreview
                      ? card.exactTextPreview
                      : card.isOutput && card.artifactTextPreview.length > 0
                        ? card.artifactTextPreview : qsTr("Empty text")
                color: Theme.textSoft
                font.pixelSize: 9
                wrapMode: Text.Wrap
                elide: Text.ElideRight
                maximumLineCount: 4
            }

            RowLayout {
                visible: card.boundDataType === "audio.clip"
                anchors.centerIn: parent
                spacing: 9

                ShapeIcon {
                    source: "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                    color: card.semanticColor
                    size: 22
                }

                ColumnLayout {
                    spacing: 1
                    Text {
                        text: card.isSource ? qsTr("Audio material") : qsTr("Audio preview")
                        color: Theme.text
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                    }
                    Text {
                        text: card.isOutput
                              ? qsTr("%1 s · %2 Hz").arg(
                                    (card.artifactAudioDurationMillis / 1000).toFixed(1)).arg(
                                    card.artifactAudioSampleRateHz)
                              : qsTr("Open to inspect this material")
                        color: Theme.muted
                        font.pixelSize: 8
                    }
                }
            }

            ColumnLayout {
                visible: card.boundDataType === "image.raster"
                         && (!card.isOutput
                             || card.acceptedImageSource.toString().length === 0)
                anchors.centerIn: parent
                spacing: 3

                ShapeIcon {
                    Layout.alignment: Qt.AlignHCenter
                    source: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                    color: card.semanticColor
                    size: 20
                }
                Text {
                    text: card.isSource ? qsTr("Image material") : qsTr("Image preview")
                    color: Theme.textSoft
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                }
            }
        }

        RowLayout {
            visible: card.isOperator
            Layout.fillWidth: true
            spacing: 5

            Rectangle {
                visible: card.isImageEditor
                Layout.preferredWidth: cropLabel.implicitWidth + 12
                Layout.preferredHeight: 20
                radius: 10
                color: Theme.surface
                border.color: Theme.border
                Text {
                    id: cropLabel
                    anchors.centerIn: parent
                    text: qsTr("Frame")
                    color: Theme.textSoft
                    font.pixelSize: 8
                }
            }

            Rectangle {
                visible: card.isImageEditor
                Layout.preferredWidth: sizeLabel.implicitWidth + 12
                Layout.preferredHeight: 20
                radius: 10
                color: Theme.surface
                border.color: Theme.border
                Text {
                    id: sizeLabel
                    anchors.centerIn: parent
                    text: qsTr("Size")
                    color: Theme.textSoft
                    font.pixelSize: 8
                }
            }

            Text {
                Layout.fillWidth: true
                text: card.detail()
                color: Theme.muted
                font.pixelSize: 9
                elide: Text.ElideRight
                horizontalAlignment: card.isImageEditor ? Text.AlignRight : Text.AlignLeft
            }
        }

        Text {
            visible: card.isOutput && card.boundDataType === "image.raster"
            Layout.fillWidth: true
            text: qsTr("%1 × %2 px").arg(card.artifactImageWidth)
                                      .arg(card.artifactImageHeight)
            color: Theme.muted
            font.pixelSize: 8
            elide: Text.ElideRight
        }
    }
}

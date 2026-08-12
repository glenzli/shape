pragma ComponentBehavior: Bound

//! Media- and role-aware presentation for one accepted creative graph node.
//! Layout belongs to SceneOperatorGraphWorkspace; this owner makes Source,
//! accepted creative steps, and the current Result visibly different.

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
        if (isSource) return qsTr("SOURCE MATERIAL")
        if (isOutput) return qsTr("CURRENT RESULT")
        if (isAiCreator) return qsTr("AI CREATION")
        return qsTr("EDITING STEP")
    }

    function roleIcon() : url {
        if (isOutput) return "qrc:/qt/qml/Shape/Desktop/icons/verified.svg"
        if (isOperator || isAiCreator) {
            return "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
        }
        if (boundDataType === "audio.clip") {
            return "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
        }
        return "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
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
        radius: Theme.cardRadius
        color: card.selected
               ? (card.isOutput ? Theme.successSoft : Theme.accentSoft)
               : card.hovered ? Theme.raisedHover : Theme.panelRaised
        border.width: card.selected ? 2 : 1
        border.color: card.selected ? card.semanticColor : Theme.borderStrong
        clip: true
    }

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 3
        color: card.semanticColor
    }

    Rectangle {
        visible: card.nodeData.inputPorts.length > 0
        anchors.left: parent.left
        anchors.leftMargin: -6
        anchors.verticalCenter: parent.verticalCenter
        width: 12
        height: 12
        radius: 6
        color: Theme.canvas
        border.width: 2
        border.color: card.semanticColor
    }

    Rectangle {
        visible: card.nodeData.outputPorts.length > 0
        anchors.right: parent.right
        anchors.rightMargin: -6
        anchors.verticalCenter: parent.verticalCenter
        width: 12
        height: 12
        radius: 6
        color: Theme.canvas
        border.width: 2
        border.color: card.semanticColor
    }

    ShapeIconButton {
        objectName: "nodeOutputAddButton"
        visible: card.nodeData.outputPorts.length > 0 && (card.hovered || card.selected)
        anchors.right: parent.right
        anchors.rightMargin: -17
        anchors.verticalCenter: parent.verticalCenter
        source: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
        toolTipText: qsTr("Add a node from this output")
        accessibleName: toolTipText
        buttonSize: 28
        iconSize: 15
        onClicked: card.outputNodeRequested()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 17
        anchors.rightMargin: 14
        anchors.topMargin: 14
        anchors.bottomMargin: 13
        spacing: 9

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Rectangle {
                Layout.preferredWidth: 32
                Layout.preferredHeight: 32
                radius: 9
                color: card.isOutput ? Theme.successSoft : Theme.panelInset

                ShapeIcon {
                    anchors.centerIn: parent
                    source: card.roleIcon()
                    size: 17
                    color: card.semanticColor
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    Layout.fillWidth: true
                    text: card.roleLabel()
                    color: card.semanticColor
                    font.pixelSize: Theme.fontMicro
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.65
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: card.title()
                    color: Theme.text
                    font.pixelSize: Theme.fontHeading
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }

            Text {
                visible: card.isOperator
                text: qsTr("STEP %1").arg(card.stageNumber)
                color: Theme.disabled
                font.pixelSize: Theme.fontMicro
            }
        }

        Rectangle {
            visible: card.displaysMaterialPreview
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 68
            radius: Theme.controlRadius
            color: Theme.panelInset
            border.color: Theme.border
            clip: true

            Image {
                visible: card.boundDataType === "image.raster"
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
                anchors.margins: 10
                text: card.hasExactTextPreview
                      ? card.exactTextPreview
                      : card.artifactTextPreview.length > 0
                        ? card.artifactTextPreview : qsTr("Empty text")
                color: Theme.textSoft
                font.pixelSize: Theme.fontMeta
                lineHeight: 1.2
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
                    spacing: 2

                    Text {
                        text: card.isSource ? qsTr("Audio material") : qsTr("Audio preview")
                        color: Theme.text
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                    }

                    Text {
                        text: card.isOutput
                              ? qsTr("%1 s · %2 Hz")
                                  .arg((card.artifactAudioDurationMillis / 1000).toFixed(1))
                                  .arg(card.artifactAudioSampleRateHz)
                              : qsTr("Open to inspect this material")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                    }
                }
            }

            ColumnLayout {
                visible: card.boundDataType === "image.raster"
                         && card.acceptedImageSource.toString().length === 0
                anchors.centerIn: parent
                spacing: 4

                ShapeIcon {
                    Layout.alignment: Qt.AlignHCenter
                    source: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                    color: card.semanticColor
                    size: 20
                }

                Text {
                    text: card.isSource ? qsTr("Image material") : qsTr("Image preview")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Rectangle {
                visible: card.isImageEditor
                Layout.preferredWidth: frameLabel.implicitWidth + 14
                Layout.preferredHeight: 22
                radius: 11
                color: Theme.panelInset

                Text {
                    id: frameLabel
                    anchors.centerIn: parent
                    text: qsTr("Frame")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMicro
                }
            }

            Rectangle {
                visible: card.isImageEditor
                Layout.preferredWidth: sizeLabel.implicitWidth + 14
                Layout.preferredHeight: 22
                radius: 11
                color: Theme.panelInset

                Text {
                    id: sizeLabel
                    anchors.centerIn: parent
                    text: qsTr("Size")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMicro
                }
            }

            Text {
                Layout.fillWidth: true
                text: card.isOutput && card.boundDataType === "image.raster"
                      ? qsTr("%1 × %2 px")
                          .arg(card.artifactImageWidth).arg(card.artifactImageHeight)
                      : card.detail()
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
                horizontalAlignment: card.isImageEditor ? Text.AlignRight : Text.AlignLeft
            }
        }
    }
}

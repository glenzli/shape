pragma ComponentBehavior: Bound

//! Media- and role-aware summary for one accepted creative graph node. The
//! graph is deliberately an index into content, not a miniature editor: full
//! material and authored intent open in their dedicated workspace.

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
    property bool hasAcceptedRevision: false
    property bool continueAvailable: false
    property int stageNumber: 0

    signal outputNodeRequested
    signal outputOpened

    readonly property bool isSource: nodeData.roleKey === "source"
    readonly property bool isOutput: nodeData.roleKey === "output"
    readonly property bool isOperator: nodeData.roleKey === "operator"
    readonly property bool hasOutputEndpoint: isOperator && !!nodeData.outputNodeId
    readonly property bool isImageEditor: isOperator && (nodeData.operatorTypeKey === "image.crop" || nodeData.operatorTypeKey === "image.resize")
    readonly property bool isAiCreator: isOperator && nodeData.inputPorts.length === 0 && nodeData.operatorTypeKey.indexOf(".generate") > 0
    readonly property string boundDataType: portDataType()
    readonly property bool hasExactTextPreview: nodeData.hasTextPreview === true
    readonly property string exactTextPreview: hasExactTextPreview ? nodeData.textPreview : ""
    readonly property bool displaysMaterialPreview: (isSource || isOutput) && (boundDataType === "text.document" || boundDataType === "image.raster" || boundDataType === "audio.clip")
    readonly property color semanticColor: isOutput && hasAcceptedRevision
                                           ? Theme.success : isOperator ? Theme.accent : Theme.muted

    function dataTypeLabel(dataTypeKey): string {
        if (dataTypeKey === "text.document")
            return qsTr("Text");
        if (dataTypeKey === "image.raster")
            return qsTr("Image");
        if (dataTypeKey === "audio.clip")
            return qsTr("Audio");
        return qsTr("Creative content");
    }

    function roleLabel(): string {
        if (isSource && nodeData.earlierInput === true)
            return qsTr("EARLIER SOURCE VERSION");
        if (isSource)
            return qsTr("SOURCE MATERIAL");
        if (isOutput)
            return hasAcceptedRevision ? qsTr("CURRENT RESULT") : qsTr("OUTPUT");
        if (isAiCreator)
            return qsTr("AI CREATION");
        return qsTr("EDITING STEP");
    }

    function roleIcon(): url {
        if (isOutput)
            return "qrc:/qt/qml/Shape/Desktop/icons/verified.svg";
        if (isOperator || isAiCreator)
            return "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg";
        if (boundDataType === "audio.clip")
            return "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg";
        return "qrc:/qt/qml/Shape/Desktop/icons/edit.svg";
    }

    function title(): string {
        if (isSource) {
            if (boundDataType === "text.document")
                return nodeData.artifactName.length > 0
                       ? nodeData.artifactName : qsTr("Text material");
            if (boundDataType === "image.raster")
                return qsTr("Image material");
            if (boundDataType === "audio.clip")
                return qsTr("Audio material");
            return qsTr("Starting material");
        }
        if (isOutput) {
            if (boundDataType === "text.document")
                return nodeData.artifactName.length > 0
                       ? nodeData.artifactName : qsTr("Text result");
            if (boundDataType === "image.raster")
                return qsTr("Image result");
            if (boundDataType === "audio.clip")
                return qsTr("Audio result");
            return qsTr("Current result");
        }
        if (isImageEditor)
            return qsTr("Image editing");
        if (nodeData.operatorTypeKey === "text.edit" || nodeData.operatorTypeKey === "text.transform")
            return qsTr("AI text editor");
        if (nodeData.operatorTypeKey === "audio.generate")
            return qsTr("Sound generation");
        if (nodeData.operatorTypeKey === "image.generate")
            return qsTr("AI image creation");
        return nodeData.operatorTypeLabel;
    }

    function portDataType(): string {
        const ports = isSource ? nodeData.outputPorts : nodeData.inputPorts;
        return ports.length > 0 ? ports[0].dataTypeKey : "";
    }

    function operatorSummary(): string {
        if (isImageEditor)
            return qsTr("Crop · Resize · Effects");
        if (nodeData.operatorTypeKey === "text.edit" || nodeData.operatorTypeKey === "text.transform") {
            return qsTr("Write · Expand · Polish");
        }
        if (nodeData.operatorTypeKey === "audio.generate")
            return qsTr("Description · Duration · Seed");
        if (isAiCreator)
            return qsTr("Prompt · Canvas · Candidates");
        return nodeData.operatorTypeLabel;
    }

    function materialPreviewText(): string {
        if (isOutput && !hasAcceptedRevision)
            return qsTr("No accepted version yet");
        if (hasExactTextPreview)
            return exactTextPreview;
        if (artifactTextPreview.length > 0)
            return artifactTextPreview;
        return qsTr("Empty text");
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controlRadius
        color: card.selected ? (card.isOutput && card.hasAcceptedRevision
                               ? Theme.successSoft : card.isOutput ? Theme.panelInset : Theme.accentSoft)
                             : card.hovered ? Theme.raisedHover : Theme.panelRaised
        border.width: card.selected ? 2 : 1
        border.color: card.selected ? card.semanticColor : Theme.borderStrong
    }

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 3
        radius: 1.5
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
        toolTipText: card.continueAvailable ? qsTr("Open next step")
                                            : qsTr("Add a node from this output")
        accessibleName: toolTipText
        buttonSize: 28
        iconSize: 15
        onClicked: card.outputNodeRequested()
    }

    Item {
        id: clippedContent
        anchors.fill: parent
        anchors.leftMargin: 13
        anchors.rightMargin: 12
        anchors.topMargin: 11
        anchors.bottomMargin: 10
        clip: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Rectangle {
                    Layout.preferredWidth: 26
                    Layout.preferredHeight: 26
                    radius: 8
                    color: card.isOutput && card.hasAcceptedRevision
                           ? Theme.successSoft : card.isOperator ? Theme.accentSurfaceQuiet : Theme.panelInset

                    ShapeIcon {
                        anchors.centerIn: parent
                        source: card.roleIcon()
                        size: 14
                        color: card.semanticColor
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Text {
                        Layout.fillWidth: true
                        text: card.roleLabel()
                        color: card.semanticColor
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.55
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        text: card.title()
                        color: Theme.text
                        font.pixelSize: Theme.fontBody
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
                Layout.minimumHeight: 44
                radius: Theme.compactControlRadius
                color: Theme.panelInset
                border.color: Theme.border
                clip: true

                Image {
                    visible: card.boundDataType === "image.raster" && card.hasAcceptedRevision && card.acceptedImageSource.toString().length > 0
                    anchors.fill: parent
                    anchors.margins: 3
                    source: card.acceptedImageSource
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    cache: false
                }

                Text {
                    visible: card.boundDataType === "text.document"
                    anchors.fill: parent
                    anchors.margins: 7
                    text: card.materialPreviewText()
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMicro
                    lineHeight: 1.15
                    wrapMode: Text.Wrap
                    elide: Text.ElideRight
                    maximumLineCount: 2
                }

                RowLayout {
                    visible: card.boundDataType === "audio.clip" && (!card.isOutput || card.hasAcceptedRevision)
                    anchors.centerIn: parent
                    spacing: 6

                    ShapeIcon {
                        source: "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                        color: card.semanticColor
                        size: 16
                    }

                    Text {
                        text: card.isOutput ? qsTr("%1 s · %2 Hz").arg((card.artifactAudioDurationMillis / 1000).toFixed(1)).arg(card.artifactAudioSampleRateHz) : qsTr("Open to inspect")
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMicro
                    }
                }

                Text {
                    visible: card.boundDataType === "image.raster" && card.hasAcceptedRevision && card.acceptedImageSource.toString().length === 0
                    anchors.centerIn: parent
                    text: qsTr("Open to inspect")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMicro
                }

                Text {
                    visible: card.isOutput && !card.hasAcceptedRevision
                             && (card.boundDataType === "audio.clip" || card.boundDataType === "image.raster")
                    anchors.centerIn: parent
                    text: qsTr("No accepted version yet")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMicro
                }
            }

            Rectangle {
                visible: card.isOperator
                Layout.fillWidth: true
                Layout.preferredHeight: card.hasOutputEndpoint ? 30 : 38
                radius: Theme.compactControlRadius
                color: Theme.panelInset
                border.color: Theme.border

                Text {
                    anchors.fill: parent
                    anchors.margins: 8
                    text: card.operatorSummary()
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.Medium
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Rectangle {
                visible: card.hasOutputEndpoint
                Layout.fillWidth: true
                Layout.preferredHeight: 32
                radius: Theme.compactControlRadius
                color: card.hasAcceptedRevision ? Theme.successSoft : Theme.panelInset
                border.color: card.hasAcceptedRevision ? Theme.success : Theme.border

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 9
                    anchors.rightMargin: 62
                    spacing: 6

                    Text {
                        text: qsTr("OUTPUT")
                        color: card.hasAcceptedRevision ? Theme.success : Theme.muted
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.fillWidth: true
                        text: card.hasAcceptedRevision
                              ? (card.nodeData.outputArtifactName || card.dataTypeLabel(card.nodeData.outputDataTypeKey))
                              : qsTr("No accepted version yet")
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMicro
                        elide: Text.ElideRight
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: card.outputOpened()
                }

                ShapeButton {
                    objectName: "acceptedGraphOutput-" + card.nodeData.outputNodeId
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: card.hasAcceptedRevision ? qsTr("View") : qsTr("Continue")
                    quiet: true
                    onClicked: card.outputOpened()
                }
            }

            Text {
                Layout.fillWidth: true
                visible: !card.isOperator
                text: card.isOutput && card.boundDataType === "image.raster" ? qsTr("%1 × %2 px").arg(card.artifactImageWidth).arg(card.artifactImageHeight) : card.dataTypeLabel(card.boundDataType)
                color: Theme.muted
                font.pixelSize: Theme.fontMicro
                elide: Text.ElideRight
            }
        }
    }
}

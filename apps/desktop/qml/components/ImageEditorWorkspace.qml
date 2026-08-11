pragma ComponentBehavior: Bound

//! Product-level image editing workspace. Crop and resize remain exact
//! executable Operators, but are presented as tools inside one creative step.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: editor
    objectName: "imageEditorWorkspace"

    property string openedOperatorTypeKey: "image.edit"
    property string operatorDraftId: ""
    property string artifactId: ""
    property url source
    property int sourceWidth: 0
    property int sourceHeight: 0
    property int targetWidth: 0
    property int targetHeight: 0
    property string aspectPolicyKey: "fit_within"
    property string resamplingKey: "lanczos3"
    property bool compareMode: false
    property bool candidatePending: false
    property url acceptedSource
    property url candidateSource
    property string selectedToolKey: "frame"

    signal cropRequested(int x, int y, int width, int height)
    signal resizeDraftRequested()
    signal resizeDraftSaveRequested(string draftId, int targetWidth, int targetHeight,
                                    string aspectPolicyKey, string resamplingKey)
    signal resizeRequested(string artifactId, string draftId,
                           int targetWidth, int targetHeight,
                           string aspectPolicyKey, string resamplingKey)

    function synchronizeTool() : void {
        if (openedOperatorTypeKey === "image.resize") selectedToolKey = "size"
        else if (openedOperatorTypeKey === "image.crop"
                 || openedOperatorTypeKey === "image.edit") selectedToolKey = "frame"
    }

    function chooseTool(toolKey) : void {
        selectedToolKey = toolKey
        if (toolKey === "size" && operatorDraftId.length === 0) {
            Qt.callLater(editor.resizeDraftRequested)
        }
    }

    onOpenedOperatorTypeKeyChanged: synchronizeTool()
    Component.onCompleted: synchronizeTool()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("Image editing")
                    color: Theme.text
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                }

                Text {
                    text: qsTr("Choose the kind of change; Shape keeps the exact operation in history.")
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }

            ButtonGroup { id: toolGroup }

            ShapeButton {
                objectName: "imageEditorFrameTool"
                text: qsTr("Frame")
                checkable: true
                checked: editor.selectedToolKey === "frame"
                ButtonGroup.group: toolGroup
                onClicked: editor.chooseTool("frame")
            }

            ShapeButton {
                objectName: "imageEditorSizeTool"
                text: qsTr("Size")
                checkable: true
                checked: editor.selectedToolKey === "size"
                ButtonGroup.group: toolGroup
                onClicked: editor.chooseTool("size")
            }

            ShapeButton {
                objectName: "imageEditorAiAssistTool"
                text: qsTr("AI assist · later")
                enabled: false
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Material-aware image editing is not available in the runtime yet.")
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        StackLayout {
            id: toolStack
            objectName: "imageEditorToolStack"
            visible: !editor.compareMode || !editor.candidatePending
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: editor.selectedToolKey === "size" ? 1 : 0

            RasterCropOperatorWorkspace {
                artifactId: editor.artifactId
                source: editor.source
                sourceWidth: editor.sourceWidth
                sourceHeight: editor.sourceHeight
                onCropRequested: (x, y, width, height) =>
                                     editor.cropRequested(x, y, width, height)
            }

            RasterResizeOperatorWorkspace {
                operatorDraftId: editor.operatorDraftId
                artifactId: editor.artifactId
                source: editor.source
                sourceWidth: editor.sourceWidth
                sourceHeight: editor.sourceHeight
                targetWidth: editor.targetWidth
                targetHeight: editor.targetHeight
                aspectPolicyKey: editor.aspectPolicyKey
                resamplingKey: editor.resamplingKey
                onDraftSaveRequested: (draftId, width, height, aspect, resampling) =>
                                          editor.resizeDraftSaveRequested(
                                              draftId, width, height, aspect, resampling)
                onResizeRequested: (artifactId, draftId, width, height,
                                    aspect, resampling) =>
                                       editor.resizeRequested(
                                           artifactId, draftId, width, height,
                                           aspect, resampling)
            }
        }

        ImageCompareWorkspace {
            visible: editor.compareMode && editor.candidatePending
            Layout.fillWidth: true
            Layout.fillHeight: true
            acceptedSource: editor.acceptedSource
            candidateSource: editor.candidateSource
        }
    }
}

pragma ComponentBehavior: Bound

//! Interactive Source -> Operator -> Output projection for one selected scene.
//! Rust owns node and port semantics; this component owns layout and interaction.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: graph
    objectName: "sceneOperatorGraphWorkspace"

    property string projectName: ""
    property string sceneName: ""
    property string sceneKind: ""
    property var nodes: []
    property var edges: []
    property var drafts: []
    property var candidates: []
    property var operatorDescriptors: []
    property string selectedNodeId: ""
    property string selectedCandidateId: ""
    property string artifactKindKey: ""
    property string artifactTextPreview: ""
    property bool artifactTextPreviewTruncated: false
    property url acceptedImageSource
    property int artifactImageWidth: 0
    property int artifactImageHeight: 0
    property int artifactAudioDurationMillis: 0
    property int artifactAudioSampleRateHz: 0
    property int artifactAudioChannels: 0
    property real zoomLevel: 1.0
    property string focusedSelectionKind: "node"

    readonly property real nodeWidth: 240
    readonly property real nodeHeight: 188
    readonly property real columnGap: 64
    readonly property real rowGap: 38
    readonly property real graphMargin: 34
    readonly property int acceptedMaxStage: maximumAcceptedStage()
    readonly property int projectedMaxStage: acceptedMaxStage
                                             + (drafts.length + candidates.length > 0 ? 1 : 0)
    readonly property int projectedNodeCount: nodes.length + drafts.length + candidates.length
    readonly property real minimumZoom: 0.65
    readonly property real maximumZoom: 1.5
    readonly property bool currentResultPreviewAvailable: (artifactKindKey === "image_raster"
                                                            && acceptedImageSource.toString().length
                                                               > 0)
                                                           || artifactKindKey === "text_document"
                                                           || artifactKindKey === "audio_clip"
    readonly property var inspectedNode: nodeForId(selectedNodeId)
    readonly property var inspectedDraft: draftForId(selectedNodeId)
    readonly property var inspectedCandidate: candidateForId(selectedCandidateId)
    readonly property string inspectionKind: {
        if (focusedSelectionKind === "candidate" && inspectedCandidate !== null) {
            return "candidate"
        }
        if (inspectedDraft !== null) return "draft"
        if (inspectedNode !== null) return "node"
        return "none"
    }
    readonly property int operatorCount: {
        let count = 0
        for (let index = 0; index < nodes.length; ++index) {
            if (nodes[index].roleKey === "operator") ++count
        }
        return count
    }
    readonly property string terminalNodeId: {
        let outputId = ""
        for (let index = 0; index < nodes.length; ++index) {
            if (nodes[index].roleKey === "output") {
                outputId = nodes[index].id
                break
            }
        }
        for (let index = 0; index < edges.length; ++index) {
            if (edges[index].targetNodeId === outputId) return edges[index].sourceNodeId
        }
        return nodes.length > 0 ? nodes[nodes.length - 1].id : ""
    }
    readonly property int maximumLaneCount: {
        let maximum = 1
        for (let stage = 0; stage <= acceptedMaxStage; ++stage) {
            maximum = Math.max(maximum, acceptedNodesAtStage(stage))
        }
        maximum = Math.max(maximum, drafts.length + candidates.length)
        return maximum
    }

    signal nodeSelected(string nodeId)
    signal nodeOpened(string nodeId)
    signal candidateSelected(string candidateId)
    signal candidateReviewRequested(string candidateId)
    signal draftRequested(string operatorTypeKey)
    signal draftSelected(string draftId)
    signal draftOpened(string draftId)
    signal draftDiscardRequested(string draftId)
    signal nodeOutputRequested(string nodeId)

    function nodeIndex(nodeId) : int {
        for (let index = 0; index < nodes.length; ++index) {
            if (nodes[index].id === nodeId) return index
        }
        return -1
    }

    function selectedNodeIndex() : int {
        return nodeIndex(selectedNodeId)
    }

    function selectedDraftIndex() : int {
        for (let index = 0; index < drafts.length; ++index) {
            if (drafts[index].id === selectedNodeId) return index
        }
        return -1
    }

    function nodeForId(nodeId) : var {
        const index = nodeIndex(nodeId)
        return index >= 0 ? nodes[index] : null
    }

    function draftForId(draftId) : var {
        for (let index = 0; index < drafts.length; ++index) {
            if (drafts[index].id === draftId) return drafts[index]
        }
        return null
    }

    function candidateForId(candidateId) : var {
        for (let index = 0; index < candidates.length; ++index) {
            if (candidates[index].id === candidateId) return candidates[index]
        }
        return null
    }

    function selectAcceptedNode(nodeId) : void {
        focusedSelectionKind = "node"
        nodeSelected(nodeId)
    }

    function selectDraft(draftId) : void {
        focusedSelectionKind = "draft"
        draftSelected(draftId)
    }

    function selectCandidate(candidateId) : void {
        focusedSelectionKind = "candidate"
        candidateSelected(candidateId)
    }

    function setZoom(nextZoom) : void {
        zoomLevel = Math.max(minimumZoom, Math.min(maximumZoom, nextZoom))
    }

    function fitGraph() : void {
        if (viewport.width <= 0 || viewport.height <= 0) return
        const horizontal = Math.max(minimumZoom,
                                    (viewport.width - 36) / contentGraphWidth())
        const vertical = Math.max(minimumZoom,
                                  (viewport.height - 36) / contentGraphHeight())
        setZoom(Math.min(1.0, horizontal, vertical))
        viewport.contentX = 0
        viewport.contentY = 0
    }

    function nodeStage(nodeId, trail) : int {
        if (trail[nodeId] === true) return 0
        const nextTrail = Object.assign({}, trail)
        nextTrail[nodeId] = true
        let stage = 0
        for (let index = 0; index < edges.length; ++index) {
            if (edges[index].targetNodeId === nodeId) {
                stage = Math.max(stage,
                                 nodeStage(edges[index].sourceNodeId, nextTrail) + 1)
            }
        }
        return stage
    }

    function maximumAcceptedStage() : int {
        let maximum = 0
        for (let index = 0; index < nodes.length; ++index) {
            maximum = Math.max(maximum, nodeStage(nodes[index].id, {}))
        }
        return maximum
    }

    function acceptedNodesAtStage(stage) : int {
        let count = 0
        for (let index = 0; index < nodes.length; ++index) {
            if (nodeStage(nodes[index].id, {}) === stage) ++count
        }
        return count
    }

    function acceptedNodeLane(index) : int {
        const stage = nodeStage(nodes[index].id, {})
        let lane = 0
        for (let cursor = 0; cursor < index; ++cursor) {
            if (nodeStage(nodes[cursor].id, {}) === stage) ++lane
        }
        return lane
    }

    function projectedStage(index) : int {
        if (index < nodes.length) {
            const stage = nodeStage(nodes[index].id, {})
            return nodes[index].roleKey === "output"
                    && drafts.length + candidates.length > 0 ? stage + 1 : stage
        }
        return acceptedMaxStage
    }

    function projectedLane(index) : int {
        if (index < nodes.length) return acceptedNodeLane(index)
        return index - nodes.length
    }

    function contentGraphWidth() : real {
        return graph.graphMargin * 2
               + (graph.projectedMaxStage + 1) * graph.nodeWidth
               + graph.projectedMaxStage * graph.columnGap
    }

    function contentGraphHeight() : real {
        return graph.graphMargin * 2
               + graph.maximumLaneCount * graph.nodeHeight
               + (graph.maximumLaneCount - 1) * graph.rowGap
    }

    function nodeX(index) : real {
        const available = Math.max(viewport.width / graph.zoomLevel, contentGraphWidth())
        const used = contentGraphWidth() - graph.graphMargin * 2
        const offset = Math.max(graph.graphMargin, (available - used) / 2)
        return offset + projectedStage(index) * (graph.nodeWidth + graph.columnGap)
    }

    function nodeY(index) : real {
        const available = Math.max(viewport.height / graph.zoomLevel,
                                   contentGraphHeight())
        const used = graph.maximumLaneCount * graph.nodeHeight
                     + (graph.maximumLaneCount - 1) * graph.rowGap
        const offset = Math.max(graph.graphMargin, (available - used) / 2)
        return offset + projectedLane(index) * (graph.nodeHeight + graph.rowGap)
    }

    function nodeTitle(node) : string {
        if (node.roleKey === "source") {
            return node.artifactName.length > 0 ? node.artifactName : qsTr("Starting material")
        }
        if (node.roleKey === "output") {
            return node.artifactName.length > 0
                    ? qsTr("%1 / Current").arg(node.artifactName) : qsTr("Current result")
        }
        return node.operatorTypeLabel
    }

    function dataTypeLabel(dataTypeKey) : string {
        if (dataTypeKey === "text.document") return qsTr("Text")
        if (dataTypeKey === "image.raster") return qsTr("Image")
        if (dataTypeKey === "audio.clip") return qsTr("Audio")
        return qsTr("Creative content")
    }

    function roleLabel(node) : string {
        if (node.roleKey === "source"
                || (node.roleKey === "operator" && node.inputPorts.length === 0)) {
            return qsTr("STARTING POINT")
        }
        if (node.roleKey === "output") return qsTr("CURRENT RESULT")
        return qsTr("CREATIVE STEP")
    }

    function draftRoleLabel(draft) : string {
        return draft.hasInputDataType ? qsTr("NEXT STEP") : qsTr("STARTING POINT")
    }

    function draftDetail(draft) : string {
        if (!draft.hasInputDataType) {
            return qsTr("Creates the first %1").arg(dataTypeLabel(draft.outputDataTypeKey))
        }
        return qsTr("%1 to %2").arg(dataTypeLabel(draft.inputDataTypeKey))
                .arg(dataTypeLabel(draft.outputDataTypeKey))
    }

    function nodeDetail(node) : string {
        if (node.roleKey === "operator") {
            return node.intent.length > 0 ? node.intent : node.operatorTypeLabel
        }
        const ports = node.roleKey === "source" ? node.outputPorts : node.inputPorts
        return ports.length > 0 ? dataTypeLabel(ports[0].dataTypeKey)
                                : qsTr("Creative content")
    }

    function roleColor(roleKey) : color {
        if (roleKey === "operator") return Theme.accent
        if (roleKey === "output") return Theme.success
        return Theme.muted
    }

    function roleIcon(roleKey) : url {
        if (roleKey === "operator") {
            return "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
        }
        if (roleKey === "output") {
            return "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
        }
        return "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
    }

    function inspectionTitle() : string {
        if (inspectionKind === "node") return nodeTitle(inspectedNode)
        if (inspectionKind === "draft") return inspectedDraft.operatorTypeLabel
        if (inspectionKind === "candidate") return qsTr("New version")
        return ""
    }

    function inspectionEyebrow() : string {
        if (inspectionKind === "node") return roleLabel(inspectedNode)
        if (inspectionKind === "draft") return draftRoleLabel(inspectedDraft)
        if (inspectionKind === "candidate") return qsTr("OPTIONAL VERSION")
        return ""
    }

    function inspectionDetail() : string {
        if (inspectionKind === "node") {
            if (inspectedNode.roleKey === "source") {
                return qsTr("The original material this work starts from · %1")
                        .arg(nodeDetail(inspectedNode))
            }
            if (inspectedNode.roleKey === "output") {
                return qsTr("The version currently used by this work · %1")
                        .arg(nodeDetail(inspectedNode))
            }
            return qsTr("A creative step in this work · %1").arg(nodeDetail(inspectedNode))
        }
        if (inspectionKind === "draft") {
            return qsTr("Ready to configure · %1").arg(draftDetail(inspectedDraft))
        }
        if (inspectionKind === "candidate") {
            return inspectedCandidate.hasImagePreview
                    ? qsTr("Image result · %1 × %2")
                          .arg(inspectedCandidate.imageWidth)
                          .arg(inspectedCandidate.imageHeight)
                    : inspectedCandidate.hasAudioPreview
                      ? qsTr("Audio result ready for review")
                      : inspectedCandidate.text
        }
        return ""
    }

    function inspectionIcon() : url {
        if (inspectionKind === "node") return roleIcon(inspectedNode.roleKey)
        if (inspectionKind === "draft") {
            return "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
        }
        if (inspectionKind === "candidate") {
            return inspectedCandidate.hasAudioPreview
                    ? "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
                    : "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
        }
        return ""
    }

    function inspectionColor() : color {
        if (inspectionKind === "node") return roleColor(inspectedNode.roleKey)
        if (inspectionKind === "candidate" || inspectionKind === "draft") return Theme.accent
        return Theme.muted
    }

    function paintConnection(context, sourceIndex, targetIndex, dashed) : void {
        if (sourceIndex < 0 || targetIndex < 0) return
        const startX = nodeX(sourceIndex) + nodeWidth
        const startY = nodeY(sourceIndex) + nodeHeight / 2
        const endX = nodeX(targetIndex)
        const endY = nodeY(targetIndex) + nodeHeight / 2
        const direction = endX >= startX ? 1 : -1
        const controlDistance = Math.max(36, Math.abs(endX - startX) * 0.48)

        context.save()
        context.strokeStyle = dashed ? Theme.accent : Theme.borderStrong
        context.fillStyle = dashed ? Theme.accent : Theme.borderStrong
        context.lineWidth = dashed ? 1.6 : 1.5
        context.setLineDash(dashed ? [7, 6] : [])
        context.beginPath()
        context.moveTo(startX, startY)
        context.bezierCurveTo(startX + direction * controlDistance, startY,
                              endX - direction * controlDistance, endY, endX, endY)
        context.stroke()
        context.setLineDash([])
        context.beginPath()
        context.moveTo(endX, endY)
        context.lineTo(endX - direction * 8, endY - 4)
        context.lineTo(endX - direction * 8, endY + 4)
        context.closePath()
        context.fill()
        context.restore()
    }

    color: Theme.panel

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        SceneGraphToolbar {
            id: graphToolbar
            Layout.fillWidth: true
            Layout.preferredHeight: 62
            projectName: graph.projectName
            sceneName: graph.sceneName
            sceneKind: graph.sceneKind
            nodeCount: graph.nodes.length
            operatorCount: graph.operatorCount
            draftCount: graph.drafts.length
            candidateCount: graph.candidates.length
            operatorDescriptors: graph.operatorDescriptors
            zoomLevel: graph.zoomLevel
            minimumZoom: graph.minimumZoom
            maximumZoom: graph.maximumZoom
            onZoomOutRequested: graph.setZoom(graph.zoomLevel - 0.1)
            onZoomInRequested: graph.setZoom(graph.zoomLevel + 0.1)
            onFitRequested: graph.fitGraph()
            onOperatorRequested: operatorTypeKey => graph.draftRequested(operatorTypeKey)
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Flickable {
            id: viewport

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            contentWidth: Math.max(width, graphCanvas.width * graph.zoomLevel)
            contentHeight: Math.max(height, graphCanvas.height * graph.zoomLevel)

            Rectangle {
                anchors.fill: parent
                color: Theme.canvas
            }

            ScrollBar.horizontal: ScrollBar {
                policy: viewport.contentWidth > viewport.width
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }
            ScrollBar.vertical: ScrollBar {
                policy: viewport.contentHeight > viewport.height
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            Item {
                id: graphCanvas

                width: Math.max(viewport.width / graph.zoomLevel,
                                graph.contentGraphWidth())
                height: Math.max(viewport.height / graph.zoomLevel,
                                 graph.contentGraphHeight())
                scale: graph.zoomLevel
                transformOrigin: Item.TopLeft

                Canvas {
                    id: gridCanvas

                    anchors.fill: parent

                    onPaint: {
                        const context = getContext("2d")
                        context.clearRect(0, 0, width, height)
                        context.fillStyle = Theme.canvasGrid
                        const step = 24
                        for (let y = step; y < height; y += step) {
                            for (let x = step; x < width; x += step) {
                                context.fillRect(x, y, 1, 1)
                            }
                        }
                    }

                    Connections {
                        target: graphCanvas
                        function onWidthChanged() : void { gridCanvas.requestPaint() }
                        function onHeightChanged() : void { gridCanvas.requestPaint() }
                    }

                    Connections {
                        target: Theme
                        function onEffectiveDarkChanged() : void {
                            gridCanvas.requestPaint()
                        }
                    }
                }

                Canvas {
                    id: edgeCanvas

                    anchors.fill: parent

                    onPaint: {
                        const context = getContext("2d")
                        context.clearRect(0, 0, width, height)
                        for (let index = 0; index < graph.edges.length; ++index) {
                            const edge = graph.edges[index]
                            graph.paintConnection(context,
                                                  graph.nodeIndex(edge.sourceNodeId),
                                                  graph.nodeIndex(edge.targetNodeId), false)
                        }
                        const terminalIndex = graph.nodeIndex(graph.terminalNodeId)
                        for (let index = 0; index < graph.drafts.length; ++index) {
                            graph.paintConnection(context, terminalIndex,
                                                  graph.nodes.length + index, true)
                        }
                        for (let index = 0; index < graph.candidates.length; ++index) {
                            graph.paintConnection(context, terminalIndex,
                                                  graph.nodes.length + graph.drafts.length + index,
                                                  true)
                        }
                    }

                    Connections {
                        target: graph
                        function onNodesChanged() : void { edgeCanvas.requestPaint() }
                        function onEdgesChanged() : void { edgeCanvas.requestPaint() }
                        function onDraftsChanged() : void { edgeCanvas.requestPaint() }
                        function onCandidatesChanged() : void { edgeCanvas.requestPaint() }
                        function onWidthChanged() : void { edgeCanvas.requestPaint() }
                        function onHeightChanged() : void { edgeCanvas.requestPaint() }
                        function onZoomLevelChanged() : void { edgeCanvas.requestPaint() }
                    }

                    Connections {
                        target: Theme
                        function onEffectiveDarkChanged() : void { edgeCanvas.requestPaint() }
                    }
                }

                Repeater {
                    model: graph.drafts

                    delegate: ItemDelegate {
                        id: draftNode
                        objectName: "draftGraphNode-" + index

                        required property int index
                        required property var modelData
                        readonly property bool selected: modelData.id === graph.selectedNodeId

                        x: graph.nodeX(graph.nodes.length + index)
                        y: graph.nodeY(graph.nodes.length + index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight
                        padding: 0
                        onClicked: graph.selectDraft(modelData.id)
                        onDoubleClicked: graph.draftOpened(modelData.id)

                        background: Item {}

                        contentItem: CreativeDraftNodeCard {
                            draftData: draftNode.modelData
                            selected: draftNode.selected
                            hovered: draftNode.hovered
                            stageNumber: graph.acceptedMaxStage
                            draftIndex: draftNode.index
                            onDiscardRequested: graph.draftDiscardRequested(
                                                    draftNode.modelData.id)
                        }
                    }
                }

                Repeater {
                    model: graph.nodes

                    delegate: ItemDelegate {
                        id: acceptedNode
                        objectName: "acceptedGraphNode-" + index

                        required property int index
                        required property var modelData
                        readonly property bool currentNode: modelData.id === graph.selectedNodeId
                        readonly property bool displaysCurrentPreview: modelData.roleKey === "output"
                                                                       && ((graph.artifactKindKey
                                                                            === "image_raster"
                                                                            && graph.acceptedImageSource
                                                                               .toString().length
                                                                               > 0)
                                                                           || graph.artifactKindKey
                                                                              === "text_document"
                                                                           || graph.artifactKindKey
                                                                              === "audio_clip")

                        x: graph.nodeX(index)
                        y: graph.nodeY(index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight
                        padding: 0
                        highlighted: currentNode
                        Accessible.name: qsTr("Open %1").arg(graph.nodeTitle(modelData))
                        onClicked: graph.selectAcceptedNode(modelData.id)
                        onDoubleClicked: graph.nodeOpened(modelData.id)

                        background: Item {}

                        contentItem: CreativeGraphNodeCard {
                            nodeData: acceptedNode.modelData
                            selected: acceptedNode.currentNode
                            hovered: acceptedNode.hovered
                            artifactKindKey: graph.artifactKindKey
                            artifactTextPreview: graph.artifactTextPreview
                            artifactTextPreviewTruncated: graph.artifactTextPreviewTruncated
                            acceptedImageSource: graph.acceptedImageSource
                            artifactImageWidth: graph.artifactImageWidth
                            artifactImageHeight: graph.artifactImageHeight
                            artifactAudioDurationMillis: graph.artifactAudioDurationMillis
                            artifactAudioSampleRateHz: graph.artifactAudioSampleRateHz
                            artifactAudioChannels: graph.artifactAudioChannels
                            stageNumber: graph.nodeStage(acceptedNode.modelData.id, {})
                            onOutputNodeRequested: {
                                graph.selectAcceptedNode(acceptedNode.modelData.id)
                                graph.nodeOutputRequested(acceptedNode.modelData.id)
                                graphToolbar.openNodeLibrary()
                            }
                        }
                    }
                }

                Repeater {
                    model: graph.candidates

                    delegate: Item {
                        id: candidateNode

                        required property int index
                        required property var modelData
                        readonly property bool selected: modelData.id
                                                         === graph.selectedCandidateId

                        x: graph.nodeX(graph.nodes.length + graph.drafts.length + index)
                        y: graph.nodeY(graph.nodes.length + graph.drafts.length + index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight

                        CreativeCandidateNodeCard {
                            anchors.fill: parent
                            candidateData: candidateNode.modelData
                            candidateIndex: candidateNode.index
                            selected: candidateNode.selected
                            hovered: candidateMouseArea.containsMouse
                        }

                        MouseArea {
                            id: candidateMouseArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            Accessible.name: qsTr("Review version %1").arg(
                                                     candidateNode.index + 1)
                            Accessible.role: Accessible.Button
                            onClicked: graph.selectCandidate(candidateNode.modelData.id)
                            onDoubleClicked: {
                                graph.selectCandidate(candidateNode.modelData.id)
                                graph.candidateReviewRequested(candidateNode.modelData.id)
                            }
                        }
                    }
                }

                ColumnLayout {
                    visible: graph.nodes.length === 0 && graph.drafts.length === 0
                    anchors.centerIn: parent
                    spacing: 6

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("This work has no starting point yet")
                        color: Theme.textSoft
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Create or import something and Shape will build the first step.")
                        color: Theme.muted
                        font.pixelSize: 10
                    }
                }
            }
        }

        GraphSelectionInspector {
            Layout.fillWidth: true
            selectionKind: graph.inspectionKind
            eyebrow: graph.inspectionEyebrow()
            title: graph.inspectionTitle()
            detail: graph.inspectionDetail()
            iconSource: graph.inspectionIcon()
            accentColor: graph.inspectionColor()
            openAvailable: graph.inspectionKind === "node"
                           || graph.inspectionKind === "draft"
            reviewAvailable: graph.inspectionKind === "candidate"
            discardAvailable: graph.inspectionKind === "draft"
                              && graph.inspectedDraft.hasInputDataType
            onOpenRequested: {
                if (graph.inspectionKind === "draft") {
                    graph.draftOpened(graph.selectedNodeId)
                } else if (graph.inspectionKind === "node") {
                    graph.nodeOpened(graph.selectedNodeId)
                }
            }
            onReviewRequested: graph.candidateReviewRequested(graph.selectedCandidateId)
            onDiscardRequested: graph.draftDiscardRequested(graph.selectedNodeId)
        }
    }
}

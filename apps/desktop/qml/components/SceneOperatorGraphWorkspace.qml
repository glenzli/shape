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
    property var candidates: []
    property string selectedNodeId: ""
    property string selectedCandidateId: ""

    readonly property real nodeWidth: 182
    readonly property real nodeHeight: 108
    readonly property real columnGap: 48
    readonly property real rowGap: 52
    readonly property real graphMargin: 24
    readonly property int acceptedMaxStage: maximumAcceptedStage()
    readonly property int projectedNodeCount: nodes.length + candidates.length
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
        maximum = Math.max(maximum,
                           acceptedNodesAtStage(acceptedMaxStage) + candidates.length)
        return maximum
    }

    signal nodeSelected(string nodeId)
    signal nodeOpened(string nodeId)
    signal candidateSelected(string candidateId)
    signal candidateReviewRequested(string candidateId)

    function nodeIndex(nodeId) : int {
        for (let index = 0; index < nodes.length; ++index) {
            if (nodes[index].id === nodeId) return index
        }
        return -1
    }

    function selectedNodeIndex() : int {
        return nodeIndex(selectedNodeId)
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
        return index < nodes.length ? nodeStage(nodes[index].id, {}) : acceptedMaxStage
    }

    function projectedLane(index) : int {
        if (index < nodes.length) return acceptedNodeLane(index)
        return acceptedNodesAtStage(acceptedMaxStage) + index - nodes.length
    }

    function contentGraphWidth() : real {
        return graph.graphMargin * 2
               + (graph.acceptedMaxStage + 1) * graph.nodeWidth
               + graph.acceptedMaxStage * graph.columnGap
    }

    function contentGraphHeight() : real {
        return graph.graphMargin * 2
               + graph.maximumLaneCount * graph.nodeHeight
               + (graph.maximumLaneCount - 1) * graph.rowGap
    }

    function nodeX(index) : real {
        const available = Math.max(viewport.width, contentGraphWidth())
        const used = contentGraphWidth() - graph.graphMargin * 2
        const offset = Math.max(graph.graphMargin, (available - used) / 2)
        return offset + projectedStage(index) * (graph.nodeWidth + graph.columnGap)
    }

    function nodeY(index) : real {
        return graph.graphMargin
               + projectedLane(index) * (graph.nodeHeight + graph.rowGap)
    }

    function nodeTitle(node) : string {
        if (node.roleKey === "source") {
            return node.artifactName.length > 0 ? node.artifactName : qsTr("Source")
        }
        if (node.roleKey === "output") {
            return node.artifactName.length > 0
                    ? qsTr("%1 / Main").arg(node.artifactName) : qsTr("Main output")
        }
        return node.operatorTypeLabel
    }

    function nodeDetail(node) : string {
        if (node.roleKey === "operator") {
            return node.intent.length > 0 ? node.intent : node.operatorTypeKey
        }
        const ports = node.roleKey === "source" ? node.outputPorts : node.inputPorts
        return ports.length > 0 ? ports[0].dataTypeKey : node.operatorTypeKey
    }

    function roleColor(roleKey) : color {
        if (roleKey === "operator") return Theme.accent
        if (roleKey === "output") return Theme.success
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

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 62
            Layout.leftMargin: 18
            Layout.rightMargin: 14
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    text: qsTr("SCENE OPERATOR GRAPH")
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("%1 › %2 › %3")
                          .arg(graph.projectName.length > 0
                               ? graph.projectName : qsTr("Project"))
                          .arg(graph.sceneName.length > 0
                               ? graph.sceneName : qsTr("Scene"))
                          .arg(graph.sceneKind)
                    color: Theme.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }

            Rectangle {
                Layout.preferredWidth: graphSummary.implicitWidth + 20
                Layout.preferredHeight: 26
                radius: 13
                color: Theme.raised
                border.color: Theme.border

                Text {
                    id: graphSummary
                    anchors.centerIn: parent
                    text: qsTr("%1 nodes · %2 operators · %3 candidates")
                          .arg(graph.nodes.length).arg(graph.operatorCount)
                          .arg(graph.candidates.length)
                    color: Theme.muted
                    font.pixelSize: 10
                }
            }

            ShapeButton {
                objectName: "openSelectedNodeButton"
                visible: graph.selectedNodeIndex() >= 0
                implicitHeight: 30
                text: qsTr("Open node")
                primary: true
                Accessible.name: qsTr("Open selected node workspace")
                onClicked: graph.nodeOpened(graph.selectedNodeId)
            }
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
            contentWidth: Math.max(width, graph.contentGraphWidth())
            contentHeight: Math.max(height, graph.contentGraphHeight())

            ScrollBar.horizontal: ScrollBar {
                policy: viewport.contentWidth > viewport.width
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }
            ScrollBar.vertical: ScrollBar {
                policy: viewport.contentHeight > viewport.height
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            Item {
                width: viewport.contentWidth
                height: viewport.contentHeight

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
                        for (let index = 0; index < graph.candidates.length; ++index) {
                            graph.paintConnection(context, terminalIndex,
                                                  graph.nodes.length + index, true)
                        }
                    }

                    Connections {
                        target: graph
                        function onNodesChanged() : void { edgeCanvas.requestPaint() }
                        function onEdgesChanged() : void { edgeCanvas.requestPaint() }
                        function onCandidatesChanged() : void { edgeCanvas.requestPaint() }
                        function onWidthChanged() : void { edgeCanvas.requestPaint() }
                    }

                    Connections {
                        target: Theme
                        function onEffectiveDarkChanged() : void { edgeCanvas.requestPaint() }
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
                        readonly property color semanticColor: graph.roleColor(modelData.roleKey)

                        x: graph.nodeX(index)
                        y: graph.nodeY(index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight
                        leftPadding: 14
                        rightPadding: 14
                        topPadding: 12
                        bottomPadding: 12
                        highlighted: currentNode
                        Accessible.name: qsTr("Open %1").arg(graph.nodeTitle(modelData))
                        onClicked: graph.nodeSelected(modelData.id)
                        onDoubleClicked: graph.nodeOpened(modelData.id)

                        background: Rectangle {
                            radius: Theme.radiusMedium
                            color: acceptedNode.currentNode ? Theme.accentSoft
                                                            : acceptedNode.hovered
                                                              ? Theme.raisedHover : Theme.raised
                            border.width: acceptedNode.currentNode ? 2 : 1
                            border.color: acceptedNode.currentNode
                                          ? acceptedNode.semanticColor : Theme.borderStrong

                            Rectangle {
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                width: 3
                                radius: 2
                                color: acceptedNode.semanticColor
                            }
                        }

                        Rectangle {
                            visible: acceptedNode.modelData.inputPorts.length > 0
                            anchors.left: parent.left
                            anchors.leftMargin: -5
                            anchors.verticalCenter: parent.verticalCenter
                            width: 10
                            height: 10
                            radius: 5
                            color: Theme.surface
                            border.width: 2
                            border.color: acceptedNode.semanticColor
                        }

                        Rectangle {
                            visible: acceptedNode.modelData.outputPorts.length > 0
                            anchors.right: parent.right
                            anchors.rightMargin: -5
                            anchors.verticalCenter: parent.verticalCenter
                            width: 10
                            height: 10
                            radius: 5
                            color: Theme.surface
                            border.width: 2
                            border.color: acceptedNode.semanticColor
                        }

                        contentItem: ColumnLayout {
                            spacing: 4

                            RowLayout {
                                Layout.fillWidth: true

                                Text {
                                    text: acceptedNode.modelData.roleLabel.toUpperCase()
                                    color: acceptedNode.semanticColor
                                    font.pixelSize: 9
                                    font.weight: Font.DemiBold
                                    font.letterSpacing: 0.5
                                }

                                Item { Layout.fillWidth: true }

                                Text {
                                    text: qsTr("%1 in · %2 out")
                                          .arg(acceptedNode.modelData.inputPorts.length)
                                          .arg(acceptedNode.modelData.outputPorts.length)
                                    color: Theme.disabled
                                    font.pixelSize: 8
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: graph.nodeTitle(acceptedNode.modelData)
                                color: Theme.text
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Text {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                text: graph.nodeDetail(acceptedNode.modelData)
                                color: Theme.muted
                                font.pixelSize: 9
                                elide: Text.ElideRight
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                    }
                }

                Repeater {
                    model: graph.candidates

                    delegate: Rectangle {
                        id: candidateNode

                        required property int index
                        required property var modelData
                        readonly property bool selected: modelData.id
                                                         === graph.selectedCandidateId

                        x: graph.nodeX(graph.nodes.length + index)
                        y: graph.nodeY(graph.nodes.length + index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight
                        radius: Theme.radiusMedium
                        color: selected ? Theme.accentSoft : Theme.raised
                        opacity: 0.95

                        Canvas {
                            id: candidateBorderCanvas
                            anchors.fill: parent
                            onPaint: {
                                const context = getContext("2d")
                                context.clearRect(0, 0, width, height)
                                context.strokeStyle = Theme.accent
                                context.lineWidth = candidateNode.selected ? 2.2 : 1.4
                                context.setLineDash([7, 5])
                                context.strokeRect(1, 1, width - 2, height - 2)
                            }
                            Connections {
                                target: candidateNode
                                function onSelectedChanged() : void {
                                    candidateBorderCanvas.requestPaint()
                                }
                            }
                            Connections {
                                target: Theme
                                function onEffectiveDarkChanged() : void {
                                    candidateBorderCanvas.requestPaint()
                                }
                            }
                        }

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 13
                            spacing: 4

                            Text {
                                text: qsTr("CANDIDATE OPERATOR")
                                color: Theme.accent
                                font.pixelSize: 9
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.5
                            }

                            Text {
                                Layout.fillWidth: true
                                text: qsTr("Pending option %1").arg(candidateNode.index + 1)
                                color: Theme.text
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Text {
                                Layout.fillWidth: true
                                text: candidateNode.modelData.hasImagePreview
                                      ? qsTr("Image result · %1 × %2").arg(
                                            candidateNode.modelData.imageWidth).arg(
                                            candidateNode.modelData.imageHeight)
                                      : candidateNode.modelData.text
                                color: Theme.muted
                                font.pixelSize: 9
                                elide: Text.ElideRight
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            Accessible.name: qsTr("Review candidate %1").arg(
                                                     candidateNode.index + 1)
                            Accessible.role: Accessible.Button
                            onClicked: graph.candidateSelected(candidateNode.modelData.id)
                            onDoubleClicked: {
                                graph.candidateSelected(candidateNode.modelData.id)
                                graph.candidateReviewRequested(candidateNode.modelData.id)
                            }
                        }
                    }
                }

                ColumnLayout {
                    visible: graph.nodes.length === 0
                    anchors.centerIn: parent
                    spacing: 6

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("This scene needs a Source")
                        color: Theme.textSoft
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Import or create content to begin its Operator Graph.")
                        color: Theme.muted
                        font.pixelSize: 10
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            color: Theme.raised
            radius: Theme.radiusLarge

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 12
                color: parent.color
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 8

                Text { text: "●"; color: Theme.muted; font.pixelSize: 8 }
                Text { text: qsTr("Source"); color: Theme.muted; font.pixelSize: 9 }
                Text { text: "●"; color: Theme.accent; font.pixelSize: 8 }
                Text { text: qsTr("Operator"); color: Theme.muted; font.pixelSize: 9 }
                Text { text: "●"; color: Theme.success; font.pixelSize: 8 }
                Text { text: qsTr("Output"); color: Theme.muted; font.pixelSize: 9 }
                Text { text: "- -"; color: Theme.accent; font.pixelSize: 10 }
                Text { text: qsTr("Candidate"); color: Theme.muted; font.pixelSize: 9 }

                Item { Layout.fillWidth: true }

                Text {
                    text: qsTr("Select a node · Double-click to open its workspace")
                    color: Theme.muted
                    font.pixelSize: 9
                }
            }
        }
    }
}

pragma ComponentBehavior: Bound

//! Interactive current-head projection of the accepted Creative Graph.
//! Layout and candidate visuals are presentation-only; graph edges come from Rust.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: graph

    property string projectName: ""
    property var artifacts: []
    property var edges: []
    property int selectedIndex: 0
    property var candidates: []
    property string selectedCandidateId: ""

    readonly property real nodeWidth: 196
    readonly property real nodeHeight: 88
    readonly property real columnGap: 82
    readonly property real rowGap: 76
    readonly property real graphMargin: 38
    readonly property int columnCount: Math.max(1, Math.floor(
                                                   (graph.width - graph.graphMargin * 2
                                                    + graph.columnGap)
                                                   / (graph.nodeWidth + graph.columnGap)))
    readonly property int projectedNodeCount: artifacts.length + candidates.length
    readonly property int rowCount: Math.max(1, Math.ceil(projectedNodeCount / columnCount))

    signal artifactSelected(int index)
    signal candidateSelected(string candidateId)

    function artifactIndex(artifactId) : int {
        for (let index = 0; index < artifacts.length; ++index) {
            if (artifacts[index].id === artifactId) {
                return index
            }
        }
        return -1
    }

    function artifactName(artifactId) : string {
        const index = artifactIndex(artifactId)
        return index >= 0 ? artifacts[index].name : qsTr("Unknown source")
    }

    function nodeX(index) : real {
        return graph.graphMargin
               + (index % graph.columnCount) * (graph.nodeWidth + graph.columnGap)
    }

    function nodeY(index) : real {
        return graph.graphMargin
               + Math.floor(index / graph.columnCount) * (graph.nodeHeight + graph.rowGap)
    }

    function hasIncomingEdge(artifactId) : bool {
        for (let index = 0; index < edges.length; ++index) {
            if (edges[index].targetArtifactId === artifactId) {
                return true
            }
        }
        return false
    }

    function nodeState(artifact) : string {
        if (hasIncomingEdge(artifact.id)) {
            return qsTr("Derived")
        }
        if (artifact.acceptedParentRevisionIds.length > 0) {
            return qsTr("Revised")
        }
        return artifact.hasAcceptedRevision ? qsTr("Origin") : qsTr("Awaiting revision")
    }

    function paintConnection(context, sourceIndex, targetIndex, dashed) : void {
        if (sourceIndex < 0 || targetIndex < 0) {
            return
        }
        const sourceX = nodeX(sourceIndex) + nodeWidth / 2
        const sourceY = nodeY(sourceIndex) + nodeHeight / 2
        const targetX = nodeX(targetIndex) + nodeWidth / 2
        const targetY = nodeY(targetIndex) + nodeHeight / 2
        const horizontal = Math.abs(targetX - sourceX) >= Math.abs(targetY - sourceY)
        const sameRow = Math.floor(sourceIndex / columnCount)
                        === Math.floor(targetIndex / columnCount)
        const crossesIntermediateNode = sameRow
                                        && Math.abs((sourceIndex % columnCount)
                                                    - (targetIndex % columnCount)) > 1
        let startX = sourceX
        let startY = sourceY
        let endX = targetX
        let endY = targetY
        let directionX = 0
        let directionY = 0

        if (horizontal) {
            directionX = targetX >= sourceX ? 1 : -1
            startX += directionX * nodeWidth / 2
            endX -= directionX * nodeWidth / 2
        } else {
            directionY = targetY >= sourceY ? 1 : -1
            startY += directionY * nodeHeight / 2
            endY -= directionY * nodeHeight / 2
        }

        context.save()
        context.strokeStyle = dashed ? Theme.accent : Theme.borderStrong
        context.fillStyle = dashed ? Theme.accent : Theme.borderStrong
        context.lineWidth = dashed ? 1.5 : 1.4
        context.setLineDash(dashed ? [6, 6] : [])
        context.beginPath()
        context.moveTo(startX, startY)
        if (horizontal) {
            if (crossesIntermediateNode) {
                const span = endX - startX
                const bendY = startY + nodeHeight / 2 + Math.min(36, rowGap * 0.45)
                context.bezierCurveTo(startX + span * 0.22, bendY,
                                      endX - span * 0.22, bendY, endX, endY)
            } else {
                const middleX = (startX + endX) / 2
                context.bezierCurveTo(middleX, startY, middleX, endY, endX, endY)
            }
        } else {
            const middleY = (startY + endY) / 2
            context.bezierCurveTo(startX, middleY, endX, middleY, endX, endY)
        }
        context.stroke()
        context.setLineDash([])
        context.beginPath()
        if (horizontal) {
            context.moveTo(endX, endY)
            context.lineTo(endX - directionX * 8, endY - 4)
            context.lineTo(endX - directionX * 8, endY + 4)
        } else {
            context.moveTo(endX, endY)
            context.lineTo(endX - 4, endY - directionY * 8)
            context.lineTo(endX + 4, endY - directionY * 8)
        }
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
            Layout.preferredHeight: 58
            Layout.leftMargin: 18
            Layout.rightMargin: 16
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    text: qsTr("PROJECT GRAPH")
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: graph.projectName.length > 0
                          ? graph.projectName : qsTr("Creative relationships")
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
                    text: qsTr("%1 artifacts · %2 derivations · %3 candidates")
                          .arg(graph.artifacts.length).arg(graph.edges.length)
                          .arg(graph.candidates.length)
                    color: Theme.muted
                    font.pixelSize: 10
                }
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
            contentWidth: width
            contentHeight: Math.max(height, graph.graphMargin * 2
                                    + graph.rowCount * graph.nodeHeight
                                    + (graph.rowCount - 1) * graph.rowGap)

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
                                                  graph.artifactIndex(edge.sourceArtifactId),
                                                  graph.artifactIndex(edge.targetArtifactId),
                                                  false)
                        }
                        for (let index = 0; index < graph.candidates.length; ++index) {
                            const candidate = graph.candidates[index]
                            graph.paintConnection(context,
                                                  graph.artifactIndex(candidate.artifactId),
                                                  graph.artifacts.length + index,
                                                  true)
                        }
                    }

                    Connections {
                        target: graph
                        function onArtifactsChanged() : void { edgeCanvas.requestPaint() }
                        function onEdgesChanged() : void { edgeCanvas.requestPaint() }
                        function onCandidatesChanged() : void { edgeCanvas.requestPaint() }
                        function onColumnCountChanged() : void { edgeCanvas.requestPaint() }
                    }

                    Connections {
                        target: Theme
                        function onEffectiveDarkChanged() : void { edgeCanvas.requestPaint() }
                    }
                }

                Repeater {
                    model: graph.artifacts

                    delegate: ItemDelegate {
                        id: artifactNode

                        required property int index
                        required property var modelData
                        readonly property bool currentNode: index === graph.selectedIndex

                        x: graph.nodeX(index)
                        y: graph.nodeY(index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight
                        leftPadding: 12
                        rightPadding: 12
                        topPadding: 12
                        bottomPadding: 12
                        highlighted: currentNode
                        Accessible.name: qsTr("Open %1").arg(modelData.name)
                        onClicked: activate()

                        function activate() : void {
                            graph.artifactSelected(index)
                        }

                        background: Rectangle {
                            radius: Theme.radiusMedium
                            color: artifactNode.currentNode ? Theme.accentSoft
                                                            : artifactNode.hovered
                                                              ? Theme.raisedHover : Theme.raised
                            border.width: artifactNode.currentNode ? 2 : 1
                            border.color: artifactNode.currentNode ? Theme.accent
                                                                   : Theme.borderStrong
                        }

                        contentItem: RowLayout {
                            spacing: 10

                            Rectangle {
                                Layout.preferredWidth: 34
                                Layout.preferredHeight: 34
                                radius: 10
                                color: artifactNode.currentNode ? Theme.raised : Theme.surface
                                border.color: artifactNode.currentNode ? Theme.accent : Theme.border

                                Text {
                                    anchors.centerIn: parent
                                    text: artifactNode.modelData.kindLabel.length > 0
                                          ? artifactNode.modelData.kindLabel.charAt(0).toUpperCase()
                                          : "·"
                                    color: artifactNode.currentNode ? Theme.accent : Theme.muted
                                    font.pixelSize: 11
                                    font.weight: Font.DemiBold
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    Layout.fillWidth: true
                                    text: artifactNode.modelData.name
                                    color: Theme.text
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: graph.nodeState(artifactNode.modelData)
                                    color: graph.hasIncomingEdge(artifactNode.modelData.id)
                                           ? Theme.accent : Theme.muted
                                    font.pixelSize: 9
                                    elide: Text.ElideRight
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: artifactNode.modelData.acceptedRevisionId
                                    color: Theme.disabled
                                    font.pixelSize: 8
                                    elide: Text.ElideMiddle
                                }
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

                        x: graph.nodeX(graph.artifacts.length + index)
                        y: graph.nodeY(graph.artifacts.length + index)
                        width: graph.nodeWidth
                        height: graph.nodeHeight
                        radius: Theme.radiusMedium
                        color: selected ? Theme.accentSoft : Theme.raised
                        opacity: 0.94

                        Canvas {
                            id: candidateBorderCanvas

                            anchors.fill: parent

                            onPaint: {
                                const context = getContext("2d")
                                context.clearRect(0, 0, width, height)
                                context.strokeStyle = Theme.accent
                                context.lineWidth = candidateNode.selected ? 2.2 : 1.4
                                context.setLineDash([6, 5])
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

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 10

                            Rectangle {
                                Layout.preferredWidth: 34
                                Layout.preferredHeight: 34
                                radius: 17
                                color: Theme.surface
                                border.color: Theme.accent

                                Text {
                                    anchors.centerIn: parent
                                    text: candidateNode.index + 1
                                    color: Theme.accent
                                    font.pixelSize: 11
                                    font.weight: Font.DemiBold
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("Pending option %1").arg(candidateNode.index + 1)
                                    color: Theme.accent
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("From %1").arg(graph.artifactName(
                                                                  candidateNode.modelData.artifactId))
                                    color: Theme.textSoft
                                    font.pixelSize: 9
                                    elide: Text.ElideRight
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: candidateNode.modelData.textTruncated
                                          ? qsTr("Preview truncated") : qsTr("Outside history")
                                    color: Theme.muted
                                    font.pixelSize: 8
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            Accessible.name: qsTr("Pending option from %1").arg(
                                                     graph.artifactName(
                                                         candidateNode.modelData.artifactId))
                            Accessible.role: Accessible.Button
                            onClicked: {
                                const sourceIndex = graph.artifactIndex(
                                                      candidateNode.modelData.artifactId)
                                if (sourceIndex >= 0) {
                                    graph.artifactSelected(sourceIndex)
                                    graph.candidateSelected(candidateNode.modelData.id)
                                }
                            }
                        }
                    }
                }

                ColumnLayout {
                    visible: graph.artifacts.length === 0
                    anchors.centerIn: parent
                    spacing: 6

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("No artifacts to graph")
                        color: Theme.textSoft
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Accepted creative objects will appear here.")
                        color: Theme.muted
                        font.pixelSize: 10
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
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

                Text {
                    text: "—"
                    color: Theme.borderStrong
                    font.pixelSize: 14
                }

                Text {
                    text: qsTr("Accepted derivation")
                    color: Theme.muted
                    font.pixelSize: 9
                }

                Text {
                    text: "- -"
                    color: Theme.accent
                    font.pixelSize: 11
                }

                Text {
                    text: qsTr("Transient candidate")
                    color: Theme.muted
                    font.pixelSize: 9
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: qsTr("Click a node to inspect it")
                    color: Theme.muted
                    font.pixelSize: 9
                }
            }
        }
    }
}

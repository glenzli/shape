pragma ComponentBehavior: Bound

//! Compact, read-only graph context for focused Operator workspaces. It
//! projects accepted nodes and mutable drafts without becoming graph authority.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: strip
    objectName: "sceneGraphContextStrip"

    property string sceneName: ""
    property var nodes: []
    property var drafts: []
    property int candidateCount: 0
    property string selectedNodeId: ""
    property bool showDrafts: true

    readonly property var projectedItems: buildProjection()

    signal graphRequested()
    signal nodeSelected(string nodeId)
    signal nodeOpened(string nodeId)
    signal draftSelected(string draftId)
    signal draftOpened(string draftId)

    function buildProjection() : var {
        const projection = []
        for (let index = 0; index < nodes.length; ++index) {
            const node = nodes[index]
            projection.push({
                "id": String(node.id),
                "title": nodeTitle(node),
                "detail": nodeDetail(node),
                "roleKey": String(node.roleKey),
                "draft": false
            })
        }
        if (showDrafts) {
            for (let index = 0; index < drafts.length; ++index) {
                const draft = drafts[index]
                projection.push({
                    "id": String(draft.id),
                    "title": String(draft.operatorTypeLabel),
                    "detail": qsTr("Ready to work on"),
                    "roleKey": "operator",
                    "draft": true
                })
            }
        }
        return projection
    }

    function nodeTitle(node) : string {
        if (String(node.roleKey) === "source") {
            return node.artifactName !== undefined && String(node.artifactName).length > 0
                    ? String(node.artifactName) : qsTr("Starting point")
        }
        if (String(node.roleKey) === "output") {
            return node.artifactName !== undefined && String(node.artifactName).length > 0
                    ? String(node.artifactName) : qsTr("Current result")
        }
        return node.operatorTypeLabel !== undefined
                ? String(node.operatorTypeLabel) : qsTr("Creative step")
    }

    function nodeDetail(node) : string {
        if (String(node.roleKey) === "operator") {
            if (node.intent !== undefined && String(node.intent).length > 0) {
                return String(node.intent)
            }
            return node.operatorTypeKey !== undefined ? String(node.operatorTypeKey) : ""
        }
        return node.roleLabel !== undefined ? String(node.roleLabel) : ""
    }

    function roleColor(roleKey, draft) : color {
        if (draft || roleKey === "operator") return Theme.accent
        if (roleKey === "output") return Theme.success
        return Theme.muted
    }

    function activate(item, open) : void {
        if (item.draft) {
            if (open) draftOpened(item.id)
            else draftSelected(item.id)
        } else {
            if (open) nodeOpened(item.id)
            else nodeSelected(item.id)
        }
    }

    implicitHeight: 82
    radius: Theme.radiusMedium
    color: Theme.surface
    border.color: Theme.border

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        spacing: 10

        ColumnLayout {
            Layout.preferredWidth: 126
            Layout.maximumWidth: 150
            spacing: 2

            Text {
                Layout.fillWidth: true
                text: strip.sceneName.length > 0 ? strip.sceneName : qsTr("Workflow")
                color: Theme.text
                font.pixelSize: 11
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            Text {
                text: qsTr("%1 step(s) · %2 new version(s)")
                      .arg(strip.nodes.length + (strip.showDrafts ? strip.drafts.length : 0))
                      .arg(strip.candidateCount)
                color: Theme.muted
                font.pixelSize: 8
            }

            ShapeIconButton {
                source: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                toolTipText: qsTr("See workflow")
                accessibleName: toolTipText
                buttonSize: 28
                onClicked: strip.graphRequested()
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            Layout.topMargin: 4
            Layout.bottomMargin: 4
            color: Theme.border
        }

        ListView {
            id: contextList

            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: ListView.Horizontal
            clip: true
            spacing: 20
            model: strip.projectedItems

            ScrollBar.horizontal: ScrollBar {
                policy: contextList.contentWidth > contextList.width
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: ItemDelegate {
                id: contextDelegate

                required property int index
                required property var modelData

                width: 138
                height: contextList.height - 7
                leftPadding: 9
                rightPadding: 9
                topPadding: 6
                bottomPadding: 6
                highlighted: strip.selectedNodeId === String(modelData.id)
                Accessible.name: String(modelData.title)
                onClicked: strip.activate(modelData, false)
                onDoubleClicked: strip.activate(modelData, true)

                background: Rectangle {
                    radius: Theme.radiusSmall
                    color: contextDelegate.highlighted ? Theme.selected
                                                       : contextDelegate.hovered
                                                         ? Theme.raisedHover : Theme.raised
                    border.width: contextDelegate.highlighted ? 1 : 0
                    border.color: Theme.borderStrong
                }

                contentItem: RowLayout {
                    spacing: 7

                    Rectangle {
                        Layout.preferredWidth: 5
                        Layout.fillHeight: true
                        Layout.topMargin: 3
                        Layout.bottomMargin: 3
                        radius: 3
                        color: strip.roleColor(String(contextDelegate.modelData.roleKey),
                                               Boolean(contextDelegate.modelData.draft))
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            Layout.fillWidth: true
                            text: String(contextDelegate.modelData.title)
                            color: Theme.text
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: String(contextDelegate.modelData.detail)
                            color: Boolean(contextDelegate.modelData.draft)
                                   ? Theme.accent : Theme.muted
                            font.pixelSize: 8
                            elide: Text.ElideRight
                        }
                    }
                }

                Text {
                    visible: contextDelegate.index < strip.projectedItems.length - 1
                    anchors.left: parent.right
                    anchors.leftMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    text: "›"
                    color: Theme.borderStrong
                    font.pixelSize: 17
                }
            }

            Text {
                anchors.centerIn: parent
                visible: strip.projectedItems.length === 0
                text: qsTr("No graph context available")
                color: Theme.muted
                font.pixelSize: 10
            }
        }
    }
}

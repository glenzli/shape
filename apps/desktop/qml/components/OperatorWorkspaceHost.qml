pragma ComponentBehavior: Bound

//! Stable node-workspace routing and one explicit open/return lifecycle. Media
//! state remains in the routed workspace components supplied by the shell.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: host
    objectName: "operatorWorkspaceHost"

    // The composition root registers only real packaged consumers. The Host
    // owns generic exact-key dispatch without becoming an Operator SDK.
    property var operatorWorkspaces: ({})
    property string selectedCandidateId: ""

    property bool active: false
    property string openedNodeId: ""
    property string openedRoleKey: ""
    property string openedOperatorTypeKey: ""
    property string openedArtifactId: ""
    property string openedRevisionId: ""
    property string openedTransformationId: ""

    readonly property string operatorFamilyKey: operatorFamily(openedOperatorTypeKey)
    readonly property bool hasRegisteredOperatorWorkspace: {
        const component = operatorWorkspaces[openedOperatorTypeKey]
        return component !== undefined && component !== null
    }

    readonly property string routeKey: {
        if (!active) return "none"
        if (openedRoleKey === "source") return "source.readonly"
        if (openedRoleKey === "output") return "output.readonly"
        if (openedRoleKey !== "operator") return "node.unknown"
        if (hasRegisteredOperatorWorkspace) {
            return "operator." + openedOperatorTypeKey
        }
        if (operatorFamilyKey.length > 0) {
            return "operator.family." + operatorFamilyKey
        }
        return "operator.unknown"
    }
    readonly property Component routedWorkspace: {
        if (routeKey === "source.readonly" || routeKey === "output.readonly") {
            return readOnlyWorkspace
        }
        if (hasRegisteredOperatorWorkspace) {
            return operatorWorkspaces[openedOperatorTypeKey]
        }
        return unknownWorkspace
    }
    readonly property string loadedWorkspaceObjectName: workspaceLoader.item !== null
                                                        ? workspaceLoader.item.objectName : ""
    readonly property var loadedWorkspace: workspaceLoader.item

    signal returnRequested()
    signal workspaceLoaded(var workspace)

    function openWorkspace(nodeId, roleKey, operatorTypeKey, artifactId,
                           revisionId, transformationId) : bool {
        if (nodeId.length === 0 || roleKey.length === 0) return false
        openedNodeId = nodeId
        openedRoleKey = roleKey
        openedOperatorTypeKey = operatorTypeKey
        openedArtifactId = artifactId
        openedRevisionId = revisionId
        openedTransformationId = transformationId
        active = true
        return true
    }

    function closeWorkspace() : void {
        active = false
        openedNodeId = ""
        openedRoleKey = ""
        openedOperatorTypeKey = ""
        openedArtifactId = ""
        openedRevisionId = ""
        openedTransformationId = ""
    }

    function requestReturn() : void {
        returnRequested()
    }

    function operatorFamily(operatorTypeKey) : string {
        const separator = operatorTypeKey.indexOf(".")
        const family = separator > 0 ? operatorTypeKey.slice(0, separator) : ""
        if (family === "text" || family === "image" || family === "audio") {
            return family
        }
        return ""
    }

    function operatorFamilyLabel(familyKey) : string {
        if (familyKey === "text") return qsTr("Text")
        if (familyKey === "image") return qsTr("Image")
        if (familyKey === "audio") return qsTr("Audio")
        return qsTr("Unknown")
    }

    function workspaceTitle() : string {
        if (routeKey === "source.readonly") return qsTr("Source viewer")
        if (routeKey === "output.readonly") return qsTr("Output viewer")
        if (hasRegisteredOperatorWorkspace) {
            return qsTr("Operator workspace · %1").arg(openedOperatorTypeKey)
        }
        if (operatorFamilyKey.length > 0) {
            return qsTr("%1 workspace unavailable").arg(
                        operatorFamilyLabel(operatorFamilyKey))
        }
        return qsTr("Unsupported operator")
    }

    radius: Theme.radiusLarge
    color: Theme.background

    Component {
        id: readOnlyWorkspace

        ReadOnlyNodeWorkspace {
            nodeId: host.openedNodeId
            roleKey: host.openedRoleKey
            operatorTypeKey: host.openedOperatorTypeKey
            artifactId: host.openedArtifactId
            revisionId: host.openedRevisionId
        }
    }

    Component {
        id: unknownWorkspace

        Rectangle {
            objectName: "unknownOperatorWorkspace"
            radius: Theme.radiusLarge
            color: Theme.surface
            border.color: Theme.border

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 64, 620)
                spacing: 14

                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: unknownBadge.implicitWidth + 22
                    Layout.preferredHeight: 28
                    radius: 14
                    color: Theme.raised
                    border.color: Theme.borderStrong

                    Text {
                        id: unknownBadge
                        anchors.centerIn: parent
                        text: qsTr("WORKSPACE UNAVAILABLE")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }
                }

                Text {
                    objectName: "unknownWorkspaceMessage"
                    Layout.fillWidth: true
                    text: host.operatorFamilyKey.length > 0
                          ? qsTr("The %1 operator family is recognized, but this build has no workspace for “%2”.")
                            .arg(host.operatorFamilyLabel(host.operatorFamilyKey))
                            .arg(host.openedOperatorTypeKey)
                          : qsTr("No workspace is registered for “%1”.").arg(
                                host.openedOperatorTypeKey.length > 0
                                ? host.openedOperatorTypeKey : qsTr("unknown operator"))
                    color: Theme.text
                    font.pixelSize: 20
                    font.weight: Font.Light
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("The Scene Graph remains available. Return to the graph and choose another node.")
                    color: Theme.muted
                    font.pixelSize: 11
                    lineHeight: 1.35
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    Layout.fillWidth: true
                    visible: host.openedTransformationId.length > 0
                    text: qsTr("Transformation: %1").arg(host.openedTransformationId)
                    color: Theme.disabled
                    font.pixelSize: 9
                    elide: Text.ElideMiddle
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 46
            radius: Theme.radiusMedium
            color: Theme.surface
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 5
                spacing: 5

                ShapeButton {
                    objectName: "returnToSceneGraphButton"
                    implicitHeight: 32
                    text: qsTr("← Scene graph")
                    Accessible.name: qsTr("Return to scene graph")
                    onClicked: host.requestReturn()
                }

                ColumnLayout {
                    Layout.leftMargin: 4
                    Layout.fillWidth: true
                    spacing: 1

                    Text {
                        Layout.fillWidth: true
                        text: host.workspaceTitle()
                        color: Theme.text
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        text: host.selectedCandidateId.length > 0
                              ? qsTr("Node workspace · candidate selected")
                              : qsTr("Node workspace · accepted revision")
                        color: Theme.muted
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                }
            }
        }

        Loader {
            id: workspaceLoader
            objectName: "nodeWorkspaceLoader"
            Layout.fillWidth: true
            Layout.fillHeight: true
            active: host.active
            sourceComponent: host.routedWorkspace
            onLoaded: host.workspaceLoaded(item)
        }
    }
}

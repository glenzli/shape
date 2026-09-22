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
    property bool compactNavigation: false

    property bool active: false
    property string openedNodeId: ""
    property string openedRoleKey: ""
    property string openedOperatorTypeKey: ""
    property string openedArtifactId: ""
    property string openedRevisionId: ""
    property string openedTransformationId: ""
    property var openedNodeData: ({})

    readonly property string operatorFamilyKey: operatorFamily(openedOperatorTypeKey)
    readonly property bool hasRegisteredOperatorWorkspace: {
        const component = operatorWorkspaces[openedOperatorTypeKey];
        return component !== undefined && component !== null;
    }

    readonly property string routeKey: {
        if (!active)
            return "none";
        if (openedRoleKey === "source")
            return "source.readonly";
        if (openedRoleKey === "output")
            return "output.readonly";
        if (openedRoleKey !== "operator")
            return "node.unknown";
        if (hasRegisteredOperatorWorkspace) {
            return "operator." + openedOperatorTypeKey;
        }
        if (operatorFamilyKey.length > 0) {
            return "operator.family." + operatorFamilyKey;
        }
        return "operator.unknown";
    }
    readonly property Component routedWorkspace: {
        if (routeKey === "source.readonly")
            return sourceMaterialWorkspace;
        if (routeKey === "output.readonly")
            return readOnlyWorkspace;
        if (hasRegisteredOperatorWorkspace) {
            return operatorWorkspaces[openedOperatorTypeKey];
        }
        return unknownWorkspace;
    }
    readonly property string loadedWorkspaceObjectName: workspaceLoader.item !== null ? workspaceLoader.item.objectName : ""
    readonly property var loadedWorkspace: workspaceLoader.item

    signal returnRequested
    signal workspaceLoaded(var workspace)

    function openWorkspace(nodeId, roleKey, operatorTypeKey, artifactId, revisionId, transformationId, nodeData): bool {
        if (nodeId.length === 0 || roleKey.length === 0)
            return false;
        openedNodeId = nodeId;
        openedRoleKey = roleKey;
        openedOperatorTypeKey = operatorTypeKey;
        openedArtifactId = artifactId;
        openedRevisionId = revisionId;
        openedTransformationId = transformationId;
        openedNodeData = nodeData || ({});
        active = true;
        return true;
    }

    function closeWorkspace(): void {
        active = false;
        openedNodeId = "";
        openedRoleKey = "";
        openedOperatorTypeKey = "";
        openedArtifactId = "";
        openedRevisionId = "";
        openedTransformationId = "";
        openedNodeData = ({});
    }

    function requestReturn(): void {
        returnRequested();
    }

    function operatorFamily(operatorTypeKey): string {
        const separator = operatorTypeKey.indexOf(".");
        const family = separator > 0 ? operatorTypeKey.slice(0, separator) : "";
        if (family === "text" || family === "image" || family === "audio") {
            return family;
        }
        return "";
    }

    function operatorFamilyLabel(familyKey): string {
        if (familyKey === "text")
            return qsTr("Text");
        if (familyKey === "image")
            return qsTr("Image");
        if (familyKey === "audio")
            return qsTr("Audio");
        return qsTr("Unknown");
    }

    function workspaceTitle(): string {
        if (routeKey === "source.readonly")
            return qsTr("Starting material");
        if (routeKey === "output.readonly")
            return qsTr("Current result");
        if (hasRegisteredOperatorWorkspace) {
            switch (openedOperatorTypeKey) {
            case "text.edit":
            case "text.transform":
                return qsTr("AI text editor");
            case "image.generate":
                return qsTr("Create an image");
            case "image.edit":
            case "image.crop":
            case "image.resize":
            case "image.transform":
            case "image.blur":
            case "image.unsharp_mask":
            case "image.drop_shadow":
                return qsTr("Image editing");
            case "audio.speech_synthesize":
                return qsTr("Turn text into speech");
            default:
                return qsTr("Creative workspace");
            }
        }
        if (operatorFamilyKey.length > 0) {
            return qsTr("%1 workspace unavailable").arg(operatorFamilyLabel(operatorFamilyKey));
        }
        return qsTr("Unsupported operator");
    }

    radius: Theme.radiusLarge
    color: Theme.background

    Component {
        id: sourceMaterialWorkspace

        SourceMaterialWorkspace {
            nodeData: host.openedNodeData
        }
    }

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
                    text: host.operatorFamilyKey.length > 0 ? qsTr("The %1 operator family is recognized, but this build has no workspace for “%2”.").arg(host.operatorFamilyLabel(host.operatorFamilyKey)).arg(host.openedOperatorTypeKey) : qsTr("No workspace is registered for “%1”.").arg(host.openedOperatorTypeKey.length > 0 ? host.openedOperatorTypeKey : qsTr("unknown operator"))
                    color: Theme.text
                    font.pixelSize: 20
                    font.weight: Font.Light
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Your workflow remains available. Return to it and choose another step.")
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
            Layout.preferredHeight: host.compactNavigation ? 34 : 50
            radius: 0
            color: "transparent"
            border.width: 0

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 2
                anchors.rightMargin: 4
                spacing: 8

                ShapeIconButton {
                    objectName: "returnToSceneGraphButton"
                    source: "qrc:/qt/qml/Shape/Desktop/icons/back.svg"
                    toolTipText: qsTr("Return to workflow")
                    accessibleName: toolTipText
                    buttonSize: 34
                    onClicked: host.requestReturn()
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1

                    Text {
                        Layout.fillWidth: true
                        text: host.compactNavigation ? qsTr("Return to workflow") : host.workspaceTitle()
                        color: Theme.text
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: !host.compactNavigation
                        text: host.selectedCandidateId.length > 0 ? qsTr("Reviewing a new version") : qsTr("Working on this creative step")
                        color: Theme.muted
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                }

                Rectangle {
                    visible: !host.compactNavigation
                    Layout.preferredWidth: workspaceStateText.implicitWidth + 20
                    Layout.preferredHeight: 26
                    radius: 13
                    color: host.selectedCandidateId.length > 0 ? Theme.accentSoft : Theme.raised

                    Text {
                        id: workspaceStateText
                        anchors.centerIn: parent
                        text: host.selectedCandidateId.length > 0 ? qsTr("Reviewing") : qsTr("Draft")
                        color: host.selectedCandidateId.length > 0 ? Theme.accent : Theme.textSoft
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
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

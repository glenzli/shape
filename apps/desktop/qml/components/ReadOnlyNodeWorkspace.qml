pragma ComponentBehavior: Bound

//! Read-only Source and Output node presentation. This owner deliberately
//! exposes immutable identities without borrowing an Operator's edit lifecycle.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: workspace

    required property string nodeId
    required property string roleKey
    required property string operatorTypeKey
    required property string artifactId
    required property string revisionId

    readonly property bool isSource: roleKey === "source"

    objectName: isSource ? "sourceReadOnlyWorkspace" : "outputReadOnlyWorkspace"
    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    function shortIdentity(identity) : string {
        if (identity.length <= 30) return identity
        return identity.slice(0, 14) + "…" + identity.slice(-10)
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 64, 620)
        spacing: 14

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: roleBadge.implicitWidth + 22
            Layout.preferredHeight: 28
            radius: 14
            color: Theme.raised
            border.color: Theme.borderStrong

            Text {
                id: roleBadge
                anchors.centerIn: parent
                text: workspace.isSource ? qsTr("READ-ONLY SOURCE")
                                         : qsTr("READ-ONLY OUTPUT")
                color: workspace.isSource ? Theme.muted : Theme.success
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.6
            }
        }

        Text {
            Layout.fillWidth: true
            text: workspace.isSource ? qsTr("Immutable scene input")
                                     : qsTr("Accepted scene result")
            color: Theme.text
            font.pixelSize: 22
            font.weight: Font.Light
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: workspace.isSource
                  ? qsTr("Source nodes expose an immutable input revision. Open an Operator to make a change.")
                  : qsTr("Output nodes expose an accepted Scene result. Editing happens in an upstream Operator.")
            color: Theme.muted
            font.pixelSize: 11
            lineHeight: 1.35
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: 8
            Layout.preferredHeight: identityRows.implicitHeight + 28
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                id: identityRows
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: 18
                anchors.rightMargin: 18
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: qsTr("Artifact")
                        color: Theme.muted
                        font.pixelSize: 10
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: workspace.shortIdentity(workspace.artifactId)
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: qsTr("Revision")
                        color: Theme.muted
                        font.pixelSize: 10
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: workspace.shortIdentity(workspace.revisionId)
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: qsTr("Node")
                        color: Theme.muted
                        font.pixelSize: 10
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: workspace.shortIdentity(workspace.nodeId)
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }
            }
        }
    }
}

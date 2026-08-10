pragma ComponentBehavior: Bound

//! Window-owned naming surface for accepting a pending candidate as a new artifact.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop
import "components"

Item {
    id: overlay

    required property DesktopBackend backend
    property string sourceName: ""
    property string candidateId: ""

    signal branchCreated()

    anchors.fill: parent
    z: 1001
    visible: false
    focus: visible

    function openFor(name, selectedCandidateId) : void {
        sourceName = name
        candidateId = selectedCandidateId
        nameEditor.text = qsTr("%1 — Branch").arg(name)
        visible = true
        nameEditor.forceActiveFocus()
        nameEditor.selectAll()
    }

    function close() : void {
        visible = false
    }

    function createBranch() : void {
        const name = nameEditor.text.trim()
        if (name.length === 0) {
            return
        }
        if (backend.branchCandidate(candidateId, name)) {
            close()
            branchCreated()
        }
    }

    Keys.onEscapePressed: event => {
        overlay.close()
        event.accepted = true
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.effectiveDark ? "#99000000" : "#550d1720"

        MouseArea {
            anchors.fill: parent
            onClicked: overlay.close()
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(500, overlay.width - 48)
        height: 310
        radius: Theme.radiusLarge
        color: Theme.raised
        border.color: Theme.borderStrong

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.AllButtons
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 76
                color: Theme.chrome
                radius: Theme.radiusLarge

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 16
                    color: parent.color
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: Theme.border
                }

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 24
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3

                    Text {
                        text: qsTr("Branch as new artifact")
                        color: Theme.text
                        font.pixelSize: 17
                        font.weight: Font.DemiBold
                    }

                    Text {
                        text: qsTr("Keep the current artifact unchanged and create a connected alternative.")
                        color: Theme.muted
                        font.pixelSize: 11
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 24
                spacing: 8

                Text {
                    text: qsTr("Artifact name")
                    color: Theme.text
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }

                TextField {
                    id: nameEditor

                    Layout.fillWidth: true
                    implicitHeight: 40
                    leftPadding: 12
                    rightPadding: 12
                    color: Theme.text
                    placeholderText: qsTr("Name this alternative")
                    placeholderTextColor: Theme.muted
                    selectionColor: Theme.accentSoft
                    selectedTextColor: Theme.text
                    font.pixelSize: Theme.fontBody
                    Accessible.name: qsTr("Artifact name")
                    onAccepted: overlay.createBranch()

                    background: Rectangle {
                        color: Theme.surface
                        radius: Theme.radiusSmall
                        border.color: nameEditor.activeFocus ? Theme.accent : Theme.border
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: overlay.backend.lastError.length > 0
                          ? overlay.backend.lastError
                          : qsTr("The candidate becomes the first accepted revision of the new artifact. Its source remains visible in Lineage.")
                    color: overlay.backend.lastError.length > 0 ? Theme.danger : Theme.muted
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                }

                Item { Layout.fillHeight: true }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 60
                color: Theme.chrome
                radius: Theme.radiusLarge

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 16
                    color: parent.color
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 1
                    color: Theme.border
                }

                RowLayout {
                    anchors.right: parent.right
                    anchors.rightMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 8

                    ShapeButton {
                        text: qsTr("Cancel")
                        onClicked: overlay.close()
                    }

                    ShapeButton {
                        text: qsTr("Create branch")
                        primary: true
                        enabled: nameEditor.text.trim().length > 0
                        onClicked: overlay.createBranch()
                    }
                }
            }
        }
    }
}

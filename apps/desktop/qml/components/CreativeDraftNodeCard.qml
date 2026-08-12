pragma ComponentBehavior: Bound

//! Presentation for one mutable, not-yet-accepted creative step. The graph
//! workspace owns selection and geometry; this card owns the draft affordance
//! and its visual distinction from accepted history.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: card

    required property var draftData
    property bool selected: false
    property bool hovered: false
    property int stageNumber: 0
    property int draftIndex: 0

    signal discardRequested()

    function dataTypeLabel(dataTypeKey) : string {
        if (dataTypeKey === "text.document") return qsTr("Text")
        if (dataTypeKey === "image.raster") return qsTr("Image")
        if (dataTypeKey === "audio.clip") return qsTr("Audio")
        return qsTr("Creative content")
    }

    readonly property string detailText: draftData.hasInputDataType
                                                 ? qsTr("%1 → %2")
                                                     .arg(dataTypeLabel(
                                                         draftData.inputDataTypeKey))
                                                     .arg(dataTypeLabel(
                                                         draftData.outputDataTypeKey))
                                                 : qsTr("Creates the first %1")
                                                     .arg(dataTypeLabel(
                                                         draftData.outputDataTypeKey))

    Rectangle {
        anchors.fill: parent
        radius: Theme.cardRadius
        color: card.selected ? Theme.accentSoft
                             : card.hovered ? Theme.raisedHover : Theme.panelRaised
        border.width: card.selected ? 2 : 1
        border.color: card.selected ? Theme.accent : Theme.accentBorder
    }

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 3
        radius: 1.5
        color: Theme.accent
    }

    Rectangle {
        visible: card.draftData.hasInputDataType
        anchors.left: parent.left
        anchors.leftMargin: -6
        anchors.verticalCenter: parent.verticalCenter
        width: 12
        height: 12
        radius: 6
        color: Theme.canvas
        border.width: 2
        border.color: Theme.accent
    }

    Rectangle {
        anchors.right: parent.right
        anchors.rightMargin: -6
        anchors.verticalCenter: parent.verticalCenter
        width: 12
        height: 12
        radius: 6
        color: Theme.canvas
        border.width: 2
        border.color: Theme.accent
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 17
        anchors.rightMargin: 14
        anchors.topMargin: 14
        anchors.bottomMargin: 13
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Rectangle {
                Layout.preferredWidth: 32
                Layout.preferredHeight: 32
                radius: 9
                color: Theme.accentSurfaceQuiet

                ShapeIcon {
                    anchors.centerIn: parent
                    source: "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
                    size: 17
                    color: Theme.accent
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    Layout.fillWidth: true
                    text: card.draftData.hasInputDataType
                          ? qsTr("DRAFT STEP") : qsTr("STARTING POINT")
                    color: Theme.accent
                    font.pixelSize: Theme.fontMicro
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.65
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: card.draftData.operatorTypeLabel
                    color: Theme.text
                    font.pixelSize: Theme.fontHeading
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }

            ShapeIconButton {
                objectName: "discardDraftButton-" + card.draftIndex
                visible: card.draftData.hasInputDataType
                source: "qrc:/qt/qml/Shape/Desktop/icons/trash.svg"
                toolTipText: qsTr("Remove unfinished step")
                accessibleName: toolTipText
                buttonSize: 28
                iconSize: 15
                onClicked: card.discardRequested()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 54
            radius: Theme.controlRadius
            color: Theme.panelInset
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 4

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Ready to configure")
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.Medium
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: card.detailText
                    color: Theme.muted
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideRight
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Rectangle {
                Layout.preferredWidth: 6
                Layout.preferredHeight: 6
                radius: 3
                color: Theme.accent
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Open to shape this step")
                color: Theme.accentSelectionText
                font.pixelSize: Theme.fontMeta
                font.weight: Font.Medium
                elide: Text.ElideRight
            }

            Text {
                text: qsTr("STEP %1").arg(card.stageNumber)
                color: Theme.disabled
                font.pixelSize: Theme.fontMicro
            }
        }
    }
}

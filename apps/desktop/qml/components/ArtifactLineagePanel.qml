//! Accepted revision lineage and creative transformation provenance.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: lineage

    property bool hasAcceptedRevision: false
    property string revisionId: ""
    property var parentRevisionIds: []
    property string transformationId: ""
    property string transformationKind: ""
    property string transformationIntent: ""
    property var inputRevisionIds: []
    property var inputArtifactNames: []
    property double constraintCount: 0
    property double referenceCount: 0

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 11

        Text {
            text: qsTr("LINEAGE")
            color: Theme.muted
            font.pixelSize: Theme.fontMeta
            font.weight: Font.DemiBold
            font.letterSpacing: 0.7
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 76
            radius: Theme.radiusMedium
            color: lineage.hasAcceptedRevision ? Theme.accentSoft : Theme.raised
            border.color: lineage.hasAcceptedRevision ? Theme.accent : Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

                Rectangle {
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    radius: 15
                    color: lineage.hasAcceptedRevision ? Theme.raised : Theme.surface
                    border.color: lineage.hasAcceptedRevision ? Theme.accent : Theme.border

                    Text {
                        anchors.centerIn: parent
                        text: lineage.hasAcceptedRevision ? "✓" : "·"
                        color: lineage.hasAcceptedRevision ? Theme.accent : Theme.muted
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        text: lineage.hasAcceptedRevision ? qsTr("Current accepted head")
                                                          : qsTr("No accepted history")
                        color: Theme.text
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.fillWidth: true
                        text: lineage.hasAcceptedRevision ? lineage.revisionId
                                                          : qsTr("Waiting for the first accepted change")
                        color: lineage.hasAcceptedRevision ? Theme.accent : Theme.muted
                        font.pixelSize: 9
                        elide: Text.ElideMiddle
                    }
                }
            }
        }

        Rectangle {
            visible: lineage.hasAcceptedRevision
            Layout.fillWidth: true
            Layout.preferredHeight: 112
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 5

                Text {
                    text: qsTr("TRANSFORMATION")
                    color: Theme.muted
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.5
                }

                Text {
                    Layout.fillWidth: true
                    text: lineage.transformationKind
                    color: Theme.text
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    text: lineage.transformationIntent
                    color: Theme.textSoft
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                    elide: Text.ElideRight
                }
            }
        }

        RowLayout {
            visible: lineage.hasAcceptedRevision
            Layout.fillWidth: true
            spacing: 7

            Repeater {
                model: [
                    { "value": lineage.inputRevisionIds.length, "label": qsTr("Inputs") },
                    { "value": lineage.constraintCount, "label": qsTr("Constraints") },
                    { "value": lineage.referenceCount, "label": qsTr("References") }
                ]

                delegate: Rectangle {
                    id: metricCard

                    required property var modelData

                    Layout.fillWidth: true
                    Layout.preferredHeight: 58
                    radius: Theme.radiusSmall
                    color: Theme.raised
                    border.color: Theme.border

                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 2

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: metricCard.modelData.value
                            color: Theme.text
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                        }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: metricCard.modelData.label
                            color: Theme.muted
                            font.pixelSize: 8
                        }
                    }
                }
            }
        }

        Text {
            visible: lineage.hasAcceptedRevision && lineage.inputArtifactNames.length > 0
            text: qsTr("SOURCE ARTIFACTS")
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.5
        }

        Rectangle {
            visible: lineage.hasAcceptedRevision && lineage.inputArtifactNames.length > 0
            Layout.fillWidth: true
            Layout.preferredHeight: 58
            radius: Theme.radiusMedium
            color: Theme.accentSoft
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 9

                Rectangle {
                    Layout.preferredWidth: 26
                    Layout.preferredHeight: 26
                    radius: 13
                    color: Theme.raised
                    border.color: Theme.accent

                    Text {
                        anchors.centerIn: parent
                        text: "↗"
                        color: Theme.accent
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: qsTr("Transformation input")
                        color: Theme.muted
                        font.pixelSize: 9
                    }

                    Text {
                        Layout.fillWidth: true
                        text: lineage.inputArtifactNames.join(", ")
                        color: Theme.text
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                }
            }
        }

        Text {
            visible: lineage.hasAcceptedRevision
            text: qsTr("PARENT REVISION")
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.5
        }

        Rectangle {
            visible: lineage.hasAcceptedRevision
            Layout.fillWidth: true
            Layout.preferredHeight: 72
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 4

                Text {
                    Layout.fillWidth: true
                    text: lineage.parentRevisionIds.length > 0
                          ? qsTr("Previous accepted state") : qsTr("Origin revision")
                    color: Theme.text
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: lineage.parentRevisionIds.length > 0
                          ? lineage.parentRevisionIds[0]
                          : qsTr("This head starts the artifact history")
                    color: Theme.muted
                    font.pixelSize: 9
                    elide: Text.ElideMiddle
                }
            }
        }

        Item { Layout.fillHeight: true }

        Text {
            Layout.fillWidth: true
            text: qsTr("Only accepted revisions enter the durable creative lineage.")
            color: Theme.muted
            font.pixelSize: 9
            wrapMode: Text.WordWrap
        }
    }
}

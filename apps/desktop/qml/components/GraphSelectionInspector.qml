pragma ComponentBehavior: Bound

//! Compact graph-local selection summary. It exposes user intent but owns no
//! node semantics, draft lifecycle, Candidate state, or persistence.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: inspector
    objectName: "graphSelectionInspector"

    property string selectionKind: "none"
    property string eyebrow: ""
    property string title: ""
    property string detail: ""
    property url iconSource
    property color accentColor: Theme.muted
    property bool openAvailable: false
    property bool reviewAvailable: false
    property bool discardAvailable: false

    signal openRequested()
    signal reviewRequested()
    signal discardRequested()

    implicitHeight: 66
    color: Theme.panelRaised
    border.color: Theme.border

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 18
        anchors.rightMargin: 16
        spacing: 12

        Rectangle {
            Layout.preferredWidth: 36
            Layout.preferredHeight: 36
            radius: 10
            color: inspector.selectionKind === "none"
                   ? Theme.panelInset : Theme.accentSurfaceQuiet

            ShapeIcon {
                visible: inspector.iconSource.toString().length > 0
                anchors.centerIn: parent
                source: inspector.iconSource
                size: 18
                color: inspector.accentColor
            }

            Text {
                visible: inspector.iconSource.toString().length === 0
                anchors.centerIn: parent
                text: "◇"
                color: Theme.muted
                font.pixelSize: 14
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            RowLayout {
                Layout.fillWidth: true
                spacing: 7

                Text {
                    visible: inspector.eyebrow.length > 0
                    text: inspector.eyebrow
                    color: inspector.accentColor
                    font.pixelSize: Theme.fontMicro
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.45
                }

                Text {
                    Layout.fillWidth: true
                    text: inspector.title.length > 0
                          ? inspector.title : qsTr("Select a step to understand what it does")
                    color: inspector.title.length > 0 ? Theme.text : Theme.textSoft
                    font.pixelSize: 13
                    font.weight: inspector.title.length > 0 ? Font.DemiBold : Font.Normal
                    elide: Text.ElideRight
                }
            }

            Text {
                Layout.fillWidth: true
                text: inspector.detail.length > 0
                      ? inspector.detail
                      : qsTr("Open a step to work on it, or add the next step from above.")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        ShapeIconButton {
            objectName: "discardSelectedDraftButton"
            visible: inspector.discardAvailable
            source: "qrc:/qt/qml/Shape/Desktop/icons/trash.svg"
            toolTipText: qsTr("Remove this unfinished step")
            accessibleName: toolTipText
            onClicked: inspector.discardRequested()
        }

        ShapeButton {
            objectName: inspector.reviewAvailable ? "reviewSelectedCandidateButton"
                                                  : "openSelectedNodeButton"
            visible: inspector.openAvailable || inspector.reviewAvailable
            primary: true
            text: inspector.reviewAvailable
                         ? qsTr("Review selected version")
                         : qsTr("Open selected step")
            onClicked: {
                if (inspector.reviewAvailable) inspector.reviewRequested()
                else inspector.openRequested()
            }
        }
    }
}

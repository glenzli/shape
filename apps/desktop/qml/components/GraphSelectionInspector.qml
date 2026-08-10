pragma ComponentBehavior: Bound

//! Compact graph-local selection summary. It exposes user intent but owns no
//! node semantics, draft lifecycle, Candidate state, or persistence.

import QtQuick
import QtQuick.Controls
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

    implicitHeight: 58
    color: Theme.raised
    border.color: Theme.border

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 12
        spacing: 10

        Rectangle {
            Layout.preferredWidth: 34
            Layout.preferredHeight: 34
            radius: 9
            color: inspector.selectionKind === "none" ? Theme.surface : Theme.accentSoft

            ShapeIcon {
                visible: inspector.iconSource.toString().length > 0
                anchors.centerIn: parent
                source: inspector.iconSource
                size: 17
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
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.45
                }

                Text {
                    Layout.fillWidth: true
                    text: inspector.title.length > 0
                          ? inspector.title : qsTr("Select a node to inspect its role and ports")
                    color: inspector.title.length > 0 ? Theme.text : Theme.textSoft
                    font.pixelSize: Theme.fontBody
                    font.weight: inspector.title.length > 0 ? Font.DemiBold : Font.Normal
                    elide: Text.ElideRight
                }
            }

            Text {
                Layout.fillWidth: true
                text: inspector.detail.length > 0
                      ? inspector.detail
                      : qsTr("Double-click a node to enter its dedicated workspace.")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        ShapeIconButton {
            objectName: "discardSelectedDraftButton"
            visible: inspector.discardAvailable
            source: "qrc:/qt/qml/Shape/Desktop/icons/trash.svg"
            toolTipText: qsTr("Discard draft")
            accessibleName: toolTipText
            onClicked: inspector.discardRequested()
        }

        ShapeButton {
            objectName: inspector.reviewAvailable ? "reviewSelectedCandidateButton"
                                                  : "openSelectedNodeButton"
            visible: inspector.openAvailable || inspector.reviewAvailable
            implicitHeight: 30
            iconSource: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
            text: inspector.reviewAvailable ? qsTr("Review") : qsTr("Open")
            primary: true
            Accessible.name: inspector.reviewAvailable
                             ? qsTr("Review selected Candidate")
                             : qsTr("Open selected node workspace")
            onClicked: {
                if (inspector.reviewAvailable) inspector.reviewRequested()
                else inspector.openRequested()
            }
        }
    }
}

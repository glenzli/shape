pragma ComponentBehavior: Bound

//! Scene identity, graph scale controls, and Operator-palette entry. Graph
//! geometry and draft authority remain outside this presentation owner.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: toolbar
    objectName: "sceneGraphToolbar"

    property string projectName: ""
    property string sceneName: ""
    property string sceneKind: ""
    property int nodeCount: 0
    property int operatorCount: 0
    property int draftCount: 0
    property int candidateCount: 0
    property var operatorDescriptors: []
    property real zoomLevel: 1.0
    property real minimumZoom: 0.65
    property real maximumZoom: 1.5

    signal zoomOutRequested()
    signal zoomInRequested()
    signal fitRequested()
    signal operatorRequested(string operatorTypeKey)

    function openNodeLibrary() : void {
        operatorPalette.openFor(addOperatorButton)
    }

    implicitHeight: 62

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 18
        anchors.rightMargin: 14
        spacing: 10

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Text {
                text: qsTr("SCENE NODE GRAPH")
                color: Theme.text
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("%1 › %2 › %3")
                      .arg(toolbar.projectName.length > 0
                           ? toolbar.projectName : qsTr("Project"))
                      .arg(toolbar.sceneName.length > 0
                           ? toolbar.sceneName : qsTr("Work"))
                      .arg(toolbar.sceneKind)
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
                text: qsTr("%1 steps · %2 ready · %3 new versions")
                      .arg(toolbar.nodeCount)
                      .arg(toolbar.draftCount).arg(toolbar.candidateCount)
                color: Theme.muted
                font.pixelSize: 10
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        ShapeIconButton {
            objectName: "graphZoomOutButton"
            source: "qrc:/qt/qml/Shape/Desktop/icons/zoom-out.svg"
            toolTipText: qsTr("Zoom out")
            accessibleName: toolTipText
            enabled: toolbar.zoomLevel > toolbar.minimumZoom
            onClicked: toolbar.zoomOutRequested()
        }

        Text {
            Layout.preferredWidth: 36
            text: Math.round(toolbar.zoomLevel * 100) + "%"
            color: Theme.muted
            font.pixelSize: 9
            horizontalAlignment: Text.AlignHCenter
        }

        ShapeIconButton {
            objectName: "graphZoomInButton"
            source: "qrc:/qt/qml/Shape/Desktop/icons/zoom-in.svg"
            toolTipText: qsTr("Zoom in")
            accessibleName: toolTipText
            enabled: toolbar.zoomLevel < toolbar.maximumZoom
            onClicked: toolbar.zoomInRequested()
        }

        ShapeIconButton {
            objectName: "graphFitButton"
            source: "qrc:/qt/qml/Shape/Desktop/icons/fit.svg"
            toolTipText: qsTr("Fit graph")
            accessibleName: toolTipText
            onClicked: toolbar.fitRequested()
        }

        ShapeButton {
            id: addOperatorButton
            objectName: "addOperatorButton"
            implicitHeight: 30
            iconSource: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
            text: qsTr("Add node")
            primary: toolbar.draftCount === 0 && toolbar.operatorCount === 0
            Accessible.name: qsTr("Open the node library")
            onClicked: operatorPalette.openFor(addOperatorButton)
        }
    }

    OperatorPalette {
        id: operatorPalette
        operators: toolbar.operatorDescriptors
        onOperatorRequested: operatorTypeKey => toolbar.operatorRequested(operatorTypeKey)
    }
}

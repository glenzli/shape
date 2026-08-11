pragma ComponentBehavior: Bound

//! Compact project navigation for the workbench shell. Models and selection
//! remain authoritative in the application; this owner derives only the
//! currently visible section and emits user intent.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: rail
    objectName: "workbenchProjectRail"

    property bool projectOpen: false
    property string projectName: ""
    property var scenes: []
    property var components: []
    property var assets: []
    property var candidates: []
    property string currentSection: "scenes"
    property string selectedSceneId: ""
    property string selectedComponentId: ""
    property string selectedAssetId: ""

    readonly property var currentItems: currentSection === "components"
                                        ? components
                                        : currentSection === "assets" ? assets : scenes
    readonly property string selectedItemId: currentSection === "components"
                                             ? selectedComponentId
                                             : currentSection === "assets"
                                               ? selectedAssetId : selectedSceneId

    signal sectionRequested(string sectionKey)
    signal sceneSelected(string sceneId, int index)
    signal sceneOpened(string sceneId, int index)
    signal componentSelected(string componentId, int index)
    signal componentOpened(string componentId, int index)
    signal assetSelected(string assetId, int index)
    signal assetOpened(string assetId, int index)
    signal newProjectRequested()
    signal openProjectRequested()
    signal createSceneRequested()
    signal createComponentRequested()
    signal importAssetRequested()

    function itemId(item) : string {
        if (!item) return ""
        if (item.sceneId !== undefined && String(item.sceneId).length > 0) {
            return String(item.sceneId)
        }
        return item.id !== undefined ? String(item.id) : ""
    }

    function itemName(item) : string {
        if (!item) return ""
        if (item.name !== undefined) return String(item.name)
        if (item.title !== undefined) return String(item.title)
        return itemId(item)
    }

    function itemKind(item) : string {
        if (!item) return ""
        if (item.kindLabel !== undefined) return String(item.kindLabel)
        if (item.kind !== undefined) return String(item.kind)
        return ""
    }

    function candidateCount(itemId) : int {
        let count = 0
        for (let index = 0; index < candidates.length; ++index) {
            if (String(candidates[index].contextArtifactId) === itemId) ++count
        }
        return count
    }

    function activate(itemId, index, open) : void {
        if (currentSection === "components") {
            if (open) componentOpened(itemId, index)
            else componentSelected(itemId, index)
        } else if (currentSection === "assets") {
            if (open) assetOpened(itemId, index)
            else assetSelected(itemId, index)
        } else {
            if (open) sceneOpened(itemId, index)
            else sceneSelected(itemId, index)
        }
    }

    function requestCreation() : void {
        if (currentSection === "components") createComponentRequested()
        else if (currentSection === "assets") importAssetRequested()
        else createSceneRequested()
    }

    function sectionLabel(sectionKey) : string {
        if (sectionKey === "components") return qsTr("Components")
        if (sectionKey === "assets") return qsTr("Assets")
        return qsTr("Works")
    }

    function emptyLabel(sectionKey) : string {
        if (!projectOpen) return qsTr("Open a project to begin")
        if (sectionKey === "components") return qsTr("No Components yet")
        if (sectionKey === "assets") return qsTr("No Assets yet")
        return qsTr("No works yet")
    }

    function createLabel(sectionKey) : string {
        if (sectionKey === "components") return qsTr("New Component")
        if (sectionKey === "assets") return qsTr("Import Asset")
        return qsTr("New content")
    }

    implicitWidth: 196
    radius: Theme.radiusMedium
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    Layout.fillWidth: true
                    text: rail.projectOpen && rail.projectName.length > 0
                          ? rail.projectName : qsTr("Shape Project")
                    color: Theme.text
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                text: rail.projectOpen ? qsTr("YOUR PROJECT")
                                           : qsTr("NO PROJECT")
                    color: Theme.muted
                    font.pixelSize: 8
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }
            }

            ShapeIconButton {
                objectName: "railNewProjectButton"
                source: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
                toolTipText: qsTr("New project")
                accessibleName: toolTipText
                onClicked: rail.newProjectRequested()
            }

            ShapeIconButton {
                objectName: "railOpenProjectButton"
                source: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                toolTipText: qsTr("Open project")
                accessibleName: toolTipText
                onClicked: rail.openProjectRequested()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 2

            Repeater {
                model: ["scenes", "components", "assets"]

                delegate: Button {
                    id: sectionButton

                    required property string modelData

                    Layout.fillWidth: true
                    implicitHeight: 28
                    padding: 4
                    checked: rail.currentSection === modelData
                    Accessible.name: rail.sectionLabel(modelData)
                    onClicked: rail.sectionRequested(modelData)

                    background: Rectangle {
                        radius: Theme.radiusSmall
                        color: sectionButton.checked ? Theme.selected
                                                     : sectionButton.hovered
                                                       ? Theme.raisedHover : "transparent"
                    }

                    contentItem: Text {
                        text: rail.sectionLabel(sectionButton.modelData)
                        color: sectionButton.checked ? Theme.text : Theme.muted
                        font.pixelSize: 9
                        font.weight: sectionButton.checked ? Font.DemiBold : Font.Normal
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: rail.sectionLabel(rail.currentSection).toUpperCase()
                color: Theme.muted
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.6
            }

            Item { Layout.fillWidth: true }

            Text {
                text: rail.currentItems.length
                color: Theme.disabled
                font.pixelSize: 9
            }
        }

        ListView {
            id: itemList

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 3
            model: rail.currentItems

            ScrollBar.vertical: ScrollBar {
                policy: itemList.contentHeight > itemList.height
                        ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: ItemDelegate {
                id: itemDelegate

                required property int index
                required property var modelData
                readonly property string stableId: rail.itemId(modelData)
                readonly property int pendingCount: rail.candidateCount(stableId)

                width: ListView.view.width
                height: 50
                leftPadding: 8
                rightPadding: 8
                highlighted: rail.selectedItemId === stableId
                Accessible.name: rail.itemName(modelData)
                onClicked: rail.activate(stableId, index, false)
                onDoubleClicked: rail.activate(stableId, index, true)

                background: Rectangle {
                    radius: Theme.radiusSmall
                    color: itemDelegate.highlighted ? Theme.selected
                                                    : itemDelegate.hovered
                                                      ? Theme.raisedHover : "transparent"
                    border.width: itemDelegate.highlighted ? 1 : 0
                    border.color: Theme.borderStrong
                }

                contentItem: RowLayout {
                    spacing: 7

                    Rectangle {
                        Layout.preferredWidth: 26
                        Layout.preferredHeight: 26
                        radius: 7
                        color: itemDelegate.highlighted ? Theme.accentSoft : Theme.raised
                        border.color: itemDelegate.pendingCount > 0 ? Theme.accent : Theme.border

                        Text {
                            anchors.centerIn: parent
                            text: rail.itemName(itemDelegate.modelData).length > 0
                                  ? rail.itemName(itemDelegate.modelData).charAt(0).toUpperCase()
                                  : "·"
                            color: itemDelegate.highlighted ? Theme.accent : Theme.muted
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            Layout.fillWidth: true
                            text: rail.itemName(itemDelegate.modelData)
                            color: Theme.text
                            font.pixelSize: 11
                            font.weight: itemDelegate.highlighted ? Font.DemiBold : Font.Normal
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: itemDelegate.pendingCount > 0
                                  ? qsTr("%1 new version(s)").arg(itemDelegate.pendingCount)
                                  : rail.itemKind(itemDelegate.modelData)
                            color: itemDelegate.pendingCount > 0 ? Theme.accent : Theme.muted
                            font.pixelSize: 8
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                width: Math.max(0, parent.width - 24)
                visible: rail.currentItems.length === 0
                text: rail.emptyLabel(rail.currentSection)
                color: Theme.muted
                font.pixelSize: 10
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }

        ShapeButton {
            objectName: "railCreateItemButton"
            Layout.fillWidth: true
            implicitHeight: 30
            iconSource: rail.currentSection === "assets"
                        ? "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                        : "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
            text: rail.createLabel(rail.currentSection)
            enabled: rail.projectOpen
            onClicked: rail.requestCreation()
        }
    }
}

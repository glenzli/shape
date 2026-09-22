pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: panel
    property string profile: "plain"
    signal profileSelected(string profile)
    signal guideRequested()
    color: Theme.surface
    border.color: Theme.border
    radius: Theme.radiusLarge
    implicitHeight: body.implicitHeight + 32
    ColumnLayout {
        id: body
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            Label { text: qsTr("Output format"); color: Theme.text; font.weight: Font.DemiBold; Layout.fillWidth: true }
            ShapeButton { text: qsTr("Rules and examples"); quiet: true; onClicked: panel.guideRequested() }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            ShapeButton { text: qsTr("Plain text"); selected: panel.profile === "plain"; onClicked: panel.profileSelected("plain") }
            ShapeButton { text: qsTr("Production script"); selected: panel.profile !== "plain"; onClicked: panel.profileSelected("narration") }
            Item { Layout.fillWidth: true }
        }
        RowLayout {
            visible: panel.profile !== "plain"
            Layout.fillWidth: true
            Label { text: qsTr("Writing example"); color: Theme.muted; Layout.rightMargin: 6 }
            Repeater {
                model: [{key: "narration", label: qsTr("General script")}, {key: "listening", label: qsTr("Listening exercise")}, {key: "dialogue", label: qsTr("Dialogue")}]
                delegate: ShapeButton {
                    required property var modelData
                    text: modelData.label
                    selected: panel.profile === modelData.key
                    quiet: !selected
                    onClicked: panel.profileSelected(modelData.key)
                }
            }
            Item { Layout.fillWidth: true }
        }
        Label {
            Layout.fillWidth: true
            text: panel.profile === "plain" ? qsTr("A regular document for reading, translation, or other text work.") : qsTr("Roles, pauses, cues, delivery and repeats are written in the script. Examples only guide the first draft.")
            color: Theme.muted
            font.pixelSize: 12
            wrapMode: Text.WordWrap
        }
    }
}

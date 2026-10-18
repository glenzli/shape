pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: root
    objectName: "deckMaterialTaskBar"
    required property DeckMaterialTaskController controller
    visible: controller.active
    implicitHeight: visible ? content.implicitHeight + 24 : 0
    color: Theme.raised
    border.color: Theme.border
    RowLayout {
        id: content
        anchors.fill: parent; anchors.margins: 12; spacing: 14
        ColumnLayout {
            Layout.fillWidth: true; spacing: 3
            Label { Layout.fillWidth: true; text: qsTr("Image for Deck · %1").arg(root.controller.cardTitle); color: Theme.text; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Label { Layout.fillWidth: true; text: root.controller.error.length ? root.controller.error : qsTr("Generate and choose a version above, then return the accepted image. Deck will ask before using it on the card."); color: root.controller.error.length ? Theme.warning : Theme.muted; wrapMode: Text.WordWrap }
        }
        ShapeButton { objectName: "deckMaterialCancel"; text: qsTr("Cancel task"); enabled: !root.controller.busy; onClicked: root.controller.cancel() }
        ShapeButton { objectName: "deckMaterialReturn"; text: root.controller.busy ? qsTr("Returning…") : qsTr("Return to Deck"); primary: true; enabled: root.controller.canReturn; onClicked: root.controller.returnImage() }
    }
}

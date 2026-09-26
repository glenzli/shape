pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

// Application notices share the same surface, spacing and controls as other dialogs.
ShapeDialog {
    id: dialog
    property string text: ""
    property string informativeText: ""

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(480, parent.width - 48)
    modal: true
    closePolicy: Popup.CloseOnEscape
    onOpened: closeButton.forceActiveFocus()

    contentItem: ScrollView {
        id: body
        implicitHeight: Math.min(messageColumn.implicitHeight, Math.max(80, dialog.parent.height - 180))
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

        ColumnLayout {
            id: messageColumn
            width: body.availableWidth
            spacing: 12
            Label {
                Layout.fillWidth: true
                visible: dialog.text.length > 0
                text: dialog.text
                textFormat: Text.PlainText
                wrapMode: Text.WordWrap
                color: Theme.text
                font.pixelSize: Theme.fontBody
            }
            Label {
                Layout.fillWidth: true
                visible: dialog.informativeText.length > 0
                text: dialog.informativeText
                textFormat: Text.PlainText
                wrapMode: Text.WrapAnywhere
                color: Theme.muted
                font.pixelSize: Theme.fontBody
            }
        }
    }

    footer: Pane {
        topPadding: 0
        leftPadding: 20
        rightPadding: 20
        bottomPadding: 20
        background: Item {}
        contentItem: RowLayout {
            Item { Layout.fillWidth: true }
            ShapeButton {
                id: closeButton
                objectName: "messageDialogCloseButton"
                text: qsTr("Close")
                primary: true
                onClicked: dialog.accept()
                Keys.onReturnPressed: dialog.accept()
                Keys.onEnterPressed: dialog.accept()
            }
        }
    }
}

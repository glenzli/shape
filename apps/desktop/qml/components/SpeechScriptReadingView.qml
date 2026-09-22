pragma ComponentBehavior: Bound
//! Read-only semantic projection of the authoritative Rust parser.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop
import "ScriptPresentation.js" as ScriptPresentation

ColumnLayout {
    id: view
    property var preview: ({plan: {events: [], issues: []}})
    spacing: 12
    Label {
        visible: (view.preview.plan?.events || []).length > 0
        Layout.fillWidth: true
        text: qsTr("Delivery · %1").arg(ScriptPresentation.delivery(view.preview.plan?.delivery))
        color: Theme.accent; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: Object.keys(view.preview.plan?.roles || {})
        delegate: Label {
            required property string modelData
            Layout.fillWidth: true
            text: qsTr("Role · %1 · %2").arg(modelData).arg(view.preview.plan.roles[modelData].description || view.preview.plan.roles[modelData].language)
            color: Theme.muted; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
    Repeater {
        model: view.preview.plan?.issues || []
        delegate: Label {
            required property var modelData
            Layout.fillWidth: true
            text: qsTr("Line %1: %2").arg(modelData.line).arg(ScriptPresentation.issue(modelData.code))
            color: Theme.danger
            wrapMode: Text.WordWrap
        }
    }
    Repeater {
        model: (view.preview.plan?.events || []).filter(e => e.kind !== "note")
        delegate: ColumnLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: 7
            Label {
                visible: modelData.kind === "speech" && modelData.role.length > 0
                text: modelData.role || ""
                color: Theme.accent
                font.pixelSize: 11
            }
            Label {
                Layout.fillWidth: true
                text: ScriptPresentation.eventText(modelData)
                textFormat: Text.PlainText
                color: modelData.kind === "speech" || (modelData.kind === "heading" || modelData.kind === "scene") ? Theme.text : Theme.muted
                font.pixelSize: (modelData.kind === "heading" || modelData.kind === "scene") ? 17 : modelData.kind === "speech" ? 14 : 12
                font.weight: (modelData.kind === "heading" || modelData.kind === "scene") ? Font.DemiBold : Font.Normal
                wrapMode: Text.WordWrap
                topPadding: modelData.kind === "pause" ? 6 : 0
                bottomPadding: modelData.kind === "speech" ? 10 : 4
            }
        }
    }
}

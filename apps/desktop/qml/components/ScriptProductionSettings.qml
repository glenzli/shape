pragma ComponentBehavior: Bound
//! Writing-time production requirements; actual declarations travel in the adopted script.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: settings
    property string profile: "narration"
    property string delivery: "neutral"
    property string cast: "Narrator"
    property int repeatCount: 2
    property int gapSeconds: 2
    property int pauseSeconds: 5
    property bool answerBeep: false
    signal settingsEdited(string delivery, string cast, int count, int gap, int pause, bool beep)
    function edited() : void { settingsEdited(delivery, cast, repeatCount, gapSeconds, pauseSeconds, answerBeep) }
    color: Theme.surface
    radius: Theme.radiusLarge
    border.color: Theme.border
    implicitHeight: body.implicitHeight + 32
    ColumnLayout {
        id: body
        anchors.fill: parent; anchors.margins: 16; spacing: 14
        Label { text: qsTr("Production settings"); font.weight: Font.DemiBold; color: Theme.text }
        Label {
            Layout.fillWidth: true
            text: qsTr("AI writes these requirements into the script. Shape checks the controls before adoption.")
            color: Theme.muted; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        RowLayout {
            Layout.fillWidth: true; spacing: 16
            ColumnLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Overall delivery"); color: Theme.muted; font.pixelSize: 11 }
                ShapeComboBox {
                    objectName: "writingDelivery"
                    Layout.fillWidth: true
                    property var keys: ["neutral", "clear", "warm", "lively"]
                    model: [qsTr("Natural and steady"), qsTr("Clear and even"), qsTr("Warm and friendly"), qsTr("Light and lively")]
                    currentIndex: Math.max(0, keys.indexOf(settings.delivery))
                    enabled: settings.profile !== "listening"
                    onActivated: index => { settings.delivery = keys[index]; settings.edited() }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Stable role names"); color: Theme.muted; font.pixelSize: 11 }
                ShapeTextField {
                    objectName: "writingCast"
                    Layout.fillWidth: true
                    text: settings.cast
                    placeholderText: qsTr("Narrator, Reader — separate names with commas")
                    onTextEdited: { settings.cast = text; settings.edited() }
                }
            }
        }
        Label {
            Layout.fillWidth: true
            text: settings.profile === "listening"
                ? qsTr("Listening delivery stays clear and even. Language changes keep the same character; choose each character's voice after adopting the script.")
                : qsTr("Use one role for a monologue. Add roles for actual dialogue or interviews; each name keeps one voice throughout.")
            color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap
        }
        RowLayout {
            visible: settings.profile === "listening"
            Layout.fillWidth: true; spacing: 12
            ColumnLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Play each question"); color: Theme.muted; font.pixelSize: 11 }
                ShapeComboBox {
                    objectName: "writingRepeatCount"
                    Layout.fillWidth: true
                    model: [qsTr("Once"), qsTr("Twice · identical audio"), qsTr("Three times · identical audio")]
                    currentIndex: settings.repeatCount - 1
                    onActivated: index => { settings.repeatCount = index + 1; settings.edited() }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Between plays"); color: Theme.muted; font.pixelSize: 11 }
                ShapeComboBox {
                    objectName: "writingGapSeconds"
                    Layout.fillWidth: true
                    property var seconds: [0, 1, 2, 3, 5, 10]
                    model: seconds.map(s => qsTr("%1 seconds").arg(s))
                    currentIndex: Math.max(0, seconds.indexOf(settings.gapSeconds))
                    onActivated: index => { settings.gapSeconds = seconds[index]; settings.edited() }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Time to answer"); color: Theme.muted; font.pixelSize: 11 }
                ShapeComboBox {
                    objectName: "writingPauseSeconds"
                    Layout.fillWidth: true
                    property var seconds: [3, 5, 10, 15, 30]
                    model: seconds.map(s => qsTr("%1 seconds").arg(s))
                    currentIndex: Math.max(0, seconds.indexOf(settings.pauseSeconds))
                    onActivated: index => { settings.pauseSeconds = seconds[index]; settings.edited() }
                }
            }
        }
        ShapeButton {
            objectName: "writingAnswerBeep"
            visible: settings.profile === "listening"
            text: settings.answerBeep ? qsTr("Answer cue · soft beep enabled") : qsTr("Answer cue · no sound")
            selected: settings.answerBeep; quiet: !selected
            onClicked: { settings.answerBeep = !settings.answerBeep; settings.edited() }
        }
    }
}

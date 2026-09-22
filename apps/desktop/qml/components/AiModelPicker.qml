pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

RowLayout {
    id: picker
    property string family: "text"
    property string defaultModelKey: ""
    property string overrideKey: ""
    readonly property var keys: family === "image"
                                ? ["gpt_5_6_luna", "gpt_6_luna", "gpt_6_sol"]
                                : ["local_qwen", "gpt_6_luna", "gpt_6_sol"]
    readonly property string effectiveModelKey: keys.indexOf(overrideKey) >= 0
                                                ? overrideKey : defaultModelKey
    signal choiceSelected(string key)

    function labelFor(key) : string {
        switch (key) {
        case "local_qwen": return qsTr("Local Qwen 3.5")
        case "gpt_5_6_luna": return "GPT-5.6 Luna"
        case "gpt_6_luna": return "GPT-6 Luna"
        case "gpt_6_sol": return "GPT-6 Sol"
        default: return key
        }
    }

    spacing: 8
    Layout.fillWidth: true

    Text {
        text: qsTr("Model for this run")
        color: Theme.muted
        font.pixelSize: 12
    }
    ShapeComboBox {
        objectName: "aiModelChoice"
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        model: [qsTr("Default · %1").arg(picker.labelFor(picker.defaultModelKey))]
               .concat(picker.keys.map(key => picker.labelFor(key)))
        currentIndex: Math.max(0, picker.keys.indexOf(picker.overrideKey) + 1)
        onActivated: index => picker.choiceSelected(index === 0 ? "" : picker.keys[index - 1])
    }
}

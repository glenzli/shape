pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

ColumnLayout {
    id: picker
    property string family: "text"
    property string defaultModelKey: ""
    property string defaultEffortKey: ""
    property string overrideKey: ""
    property string effortOverrideKey: ""
    readonly property var keys: family === "image"
                                ? ["gpt_5_6_luna", "gpt_6_luna", "gpt_6_sol"]
                                : ["local_qwen", "gpt_6_luna", "gpt_6_sol"]
    readonly property string effectiveModelKey: keys.indexOf(overrideKey) >= 0
                                                ? overrideKey : defaultModelKey
    readonly property var effortKeys: effectiveModelKey === "local_qwen" ? []
                                      : effectiveModelKey === "gpt_6_sol"
                                        ? ["low", "medium", "high", "xhigh", "max", "ultra"]
                                        : ["low", "medium", "high", "xhigh", "max"]
    readonly property bool automaticEffortAvailable: family === "image"
    readonly property string resolvedDefaultEffortKey: effortKeys.indexOf(defaultEffortKey) >= 0
                                                        ? defaultEffortKey
                                                        : family === "image" ? "" : "low"
    readonly property string effectiveEffortKey: effortKeys.length === 0 ? ""
                                                 : effortOverrideKey === "auto" && automaticEffortAvailable ? ""
                                                 : effortKeys.indexOf(effortOverrideKey) >= 0 ? effortOverrideKey
                                                 : resolvedDefaultEffortKey
    signal choiceSelected(string key)
    signal effortSelected(string key)

    function labelFor(key) : string {
        switch (key) {
        case "local_qwen": return qsTr("Local Qwen 3.5")
        case "gpt_5_6_luna": return "GPT-5.6 Luna"
        case "gpt_6_luna": return "GPT-6 Luna"
        case "gpt_6_sol": return "GPT-6 Sol"
        default: return key
        }
    }

    function effortLabel(key) : string {
        switch (key) {
        case "low": return qsTr("Low")
        case "medium": return qsTr("Medium")
        case "high": return qsTr("High")
        case "xhigh": return qsTr("Extra high")
        case "max": return qsTr("Max")
        case "ultra": return qsTr("Ultra")
        default: return qsTr("Runtime default")
        }
    }

    spacing: 6
    Layout.fillWidth: true

    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            spacing: 4
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
        ColumnLayout {
            visible: picker.effortKeys.length > 0
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            spacing: 4
            Text {
                text: qsTr("Effort for this run")
                color: Theme.muted
                font.pixelSize: 12
            }
            ShapeComboBox {
                objectName: "aiEffortChoice"
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                model: [qsTr("Default · %1").arg(picker.effortLabel(picker.resolvedDefaultEffortKey))]
                       .concat(picker.automaticEffortAvailable ? [picker.effortLabel("")] : [])
                       .concat(picker.effortKeys.map(key => picker.effortLabel(key)))
                currentIndex: picker.effortOverrideKey === "" ? 0
                              : picker.effortOverrideKey === "auto" && picker.automaticEffortAvailable ? 1
                              : Math.max(0, picker.effortKeys.indexOf(picker.effortOverrideKey)
                                            + (picker.automaticEffortAvailable ? 2 : 1))
                onActivated: index => picker.effortSelected(index === 0 ? ""
                    : picker.automaticEffortAvailable && index === 1 ? "auto"
                    : picker.effortKeys[index - (picker.automaticEffortAvailable ? 2 : 1)])
            }
        }
    }
}

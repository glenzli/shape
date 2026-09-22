pragma ComponentBehavior: Bound

//! Window-owned settings overlay. Keeping this surface in the main scene
//! makes geometry and focus deterministic with an expanded native title bar.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop
import "components"

Item {
    id: overlay

    required property UiPreferences uiPreferences
    required property InferTextController inferText

    anchors.fill: parent
    z: 1000
    visible: false
    focus: visible

    function open() : void {
        inferText.refreshCredentialStatus()
        visible = true
        forceActiveFocus()
    }

    function close() : void {
        visible = false
    }

    Keys.onEscapePressed: event => {
        overlay.close()
        event.accepted = true
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.effectiveDark ? "#99000000" : "#550d1720"

        MouseArea {
            anchors.fill: parent
            onClicked: overlay.close()
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(620, overlay.width - 48)
        height: Math.min(590, overlay.height - 48)
        radius: Theme.radiusLarge
        color: Theme.raised
        border.color: Theme.borderStrong

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.AllButtons
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 72
                color: Theme.chrome
                radius: Theme.radiusLarge

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 16
                    color: parent.color
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: Theme.border
                }

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 24
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3

                    Text {
                        text: qsTr("Shape settings")
                        color: Theme.text
                        font.pixelSize: 17
                        font.weight: Font.DemiBold
                    }

                    Text {
                        text: qsTr("Appearance, language, and AI models")
                        color: Theme.muted
                        font.pixelSize: 11
                    }
                }
            }

            Flickable {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: width
                contentHeight: settingsContent.implicitHeight + 52
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}

                ColumnLayout {
                    id: settingsContent
                    x: 26
                    y: 26
                    width: parent.width - 52
                    spacing: 18

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 5

                        Text {
                            text: qsTr("Appearance")
                            color: Theme.text
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                        }

                        Text {
                            text: qsTr("Follow the system by default, or keep Shape light or dark.")
                            color: Theme.muted
                            font.pixelSize: 11
                        }
                    }

                    RowLayout {
                        spacing: 8

                        Repeater {
                            model: [
                                { key: 0, label: qsTr("System") },
                                { key: 1, label: qsTr("Light") },
                                { key: 2, label: qsTr("Dark") }
                            ]

                            delegate: ShapeButton {
                                required property var modelData
                                Layout.preferredWidth: 92
                                text: modelData.label
                                selected: overlay.uiPreferences.appearanceMode === modelData.key
                                onClicked: overlay.uiPreferences.appearanceMode = modelData.key
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.border
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 5

                        Text {
                            text: qsTr("Language")
                            color: Theme.text
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                        }

                        Text {
                            text: qsTr("The default follows your system language.")
                            color: Theme.muted
                            font.pixelSize: 11
                        }
                    }

                    RowLayout {
                        spacing: 8

                        Repeater {
                            model: [
                                { key: "system", label: qsTr("System") },
                                { key: "zh_CN", label: "简体中文" },
                                { key: "en", label: "English" }
                            ]

                            delegate: ShapeButton {
                                required property var modelData
                                Layout.preferredWidth: 92
                                text: modelData.label
                                selected: overlay.uiPreferences.languageMode === modelData.key
                                onClicked: overlay.uiPreferences.languageMode = modelData.key
                            }
                        }
                    }


                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.border
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: qsTr("AI models")
                            color: Theme.text
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                        }

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Cloud choices send the prompt to the subscription provider. Availability depends on Infer Runtime access.")
                            color: Theme.muted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Text {
                                Layout.preferredWidth: 92
                                text: qsTr("Text editing")
                                color: Theme.text
                            }

                            ShapeComboBox {
                                objectName: "textModelSelector"
                                Layout.preferredWidth: 210
                                model: [qsTr("Local Qwen 3.5"), "GPT-6 Luna", "GPT-6 Sol"]
                                currentIndex: Math.max(0, ["local_qwen", "gpt_6_luna", "gpt_6_sol"]
                                                            .indexOf(overlay.uiPreferences.textModel))
                                onActivated: index => overlay.uiPreferences.textModel =
                                                 ["local_qwen", "gpt_6_luna", "gpt_6_sol"][index]
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Text {
                                Layout.preferredWidth: 92
                                text: qsTr("Image creation")
                                color: Theme.text
                            }

                            ShapeComboBox {
                                objectName: "imageModelSelector"
                                Layout.preferredWidth: 210
                                model: ["GPT-5.6 Luna", "GPT-6 Luna", "GPT-6 Sol"]
                                currentIndex: Math.max(0, ["gpt_5_6_luna", "gpt_6_luna", "gpt_6_sol"]
                                                            .indexOf(overlay.uiPreferences.imageModel))
                                onActivated: index => overlay.uiPreferences.imageModel =
                                                 ["gpt_5_6_luna", "gpt_6_luna", "gpt_6_sol"][index]
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.border
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 5

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: qsTr("Infer Runtime access")
                                color: Theme.text
                                font.pixelSize: 13
                                font.weight: Font.DemiBold
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: overlay.inferText.credentialConfigured
                                      ? qsTr("Credential saved") : qsTr("Not configured")
                                color: overlay.inferText.credentialConfigured
                                       ? Theme.success : Theme.muted
                                font.pixelSize: 10
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Paste the one-time managed token for app “shape” from Infer Console. It stays outside projects and settings exports.")
                            color: Theme.muted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        ShapeTextField {
                            id: inferCredential
                            objectName: "inferCredentialField"

                            Layout.fillWidth: true
                            implicitHeight: 32
                            enabled: !overlay.inferText.running
                            echoMode: TextInput.Password
                            placeholderText: qsTr("64-character managed token")
                            color: Theme.text
                            placeholderTextColor: Theme.muted
                            selectionColor: Theme.accentSoft
                            selectedTextColor: Theme.text
                            Accessible.name: qsTr("Shape Infer managed credential")

                            background: Rectangle {
                                color: Theme.surface
                                radius: Theme.radiusSmall
                                border.color: parent.activeFocus ? Theme.accent : Theme.border
                            }
                        }

                        ShapeButton {
                            implicitHeight: 32
                            text: overlay.inferText.credentialConfigured
                                  ? qsTr("Replace") : qsTr("Save credential")
                            enabled: !overlay.inferText.running
                                     && inferCredential.text.length > 0
                            onClicked: {
                                if (overlay.inferText.installCredential(inferCredential.text)) {
                                    inferCredential.text = ""
                                }
                            }
                        }
                    }

                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 60
                color: Theme.chrome
                radius: Theme.radiusLarge

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 16
                    color: parent.color
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 1
                    color: Theme.border
                }

                ShapeButton {
                    anchors.right: parent.right
                    anchors.rightMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Done")
                    primary: true
                    onClicked: overlay.close()
                }
            }
        }
    }
}

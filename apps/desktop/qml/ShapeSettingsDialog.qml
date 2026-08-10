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
                        text: qsTr("Appearance, language, and local AI access")
                        color: Theme.muted
                        font.pixelSize: 11
                    }
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 26
                    spacing: 22

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
                        Layout.fillWidth: true
                        spacing: 8

                        Repeater {
                            model: [
                                { key: 0, label: qsTr("System") },
                                { key: 1, label: qsTr("Light") },
                                { key: 2, label: qsTr("Dark") }
                            ]

                            delegate: ShapeButton {
                                required property var modelData
                                Layout.fillWidth: true
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
                        Layout.fillWidth: true
                        spacing: 8

                        Repeater {
                            model: [
                                { key: "system", label: qsTr("System") },
                                { key: "zh_CN", label: "简体中文" },
                                { key: "en", label: "English" }
                            ]

                            delegate: ShapeButton {
                                required property var modelData
                                Layout.fillWidth: true
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

                        TextField {
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

                    Item { Layout.fillHeight: true }
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

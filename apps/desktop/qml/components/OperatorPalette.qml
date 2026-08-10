pragma ComponentBehavior: Bound

//! Searchable, type-compatible Operator chooser. It owns presentation-only
//! filtering; the Rust session remains authoritative when a draft is requested.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Popup {
    id: palette
    objectName: "operatorPalette"

    property string sceneKindKey: ""
    readonly property var filteredOperators: filterOperators(searchField.text)
    readonly property int compatibleOperatorCount: filterOperators("").length
    readonly property int visibleOperatorCount: filteredOperators.length

    signal operatorRequested(string operatorTypeKey)

    function catalog() : var {
        return [
            {
                "typeKey": "text.edit",
                "objectName": "addTextEditOperatorAction",
                "sceneKindKey": "text_document",
                "label": qsTr("Text edit"),
                "description": qsTr("Revise exact text while preserving accepted history."),
                "category": qsTr("TEXT"),
                "keywords": "text edit revise copy",
                "icon": "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
            },
            {
                "typeKey": "text.transform",
                "objectName": "addTextTransformOperatorAction",
                "sceneKindKey": "text_document",
                "label": qsTr("AI text transform"),
                "description": qsTr("Rewrite, expand, or polish through Infer Runtime."),
                "category": qsTr("AI · TEXT"),
                "keywords": "ai text transform rewrite expand polish llm",
                "icon": "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
            },
            {
                "typeKey": "audio.speech_synthesize",
                "objectName": "addSpeechOperatorAction",
                "sceneKindKey": "text_document",
                "label": qsTr("Speech synthesis"),
                "description": qsTr("Create a local voice Candidate from accepted text."),
                "category": qsTr("AI · AUDIO"),
                "keywords": "ai audio speech synthesize voice tts",
                "icon": "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
            },
            {
                "typeKey": "image.crop",
                "objectName": "addCropOperatorAction",
                "sceneKindKey": "image_raster",
                "label": qsTr("Crop image"),
                "description": qsTr("Frame a non-destructive raster crop Candidate."),
                "category": qsTr("IMAGE"),
                "keywords": "image raster crop frame",
                "icon": "qrc:/qt/qml/Shape/Desktop/icons/crop.svg"
            }
        ]
    }

    function filterOperators(query) : var {
        const normalized = query.trim().toLowerCase()
        return catalog().filter(operator => {
            if (operator.sceneKindKey !== palette.sceneKindKey) return false
            if (normalized.length === 0) return true
            const haystack = (operator.label + " " + operator.description + " "
                              + operator.typeKey + " " + operator.keywords).toLowerCase()
            return haystack.indexOf(normalized) >= 0
        })
    }

    function openFor(anchorItem) : void {
        searchField.text = ""
        const mapped = anchorItem.mapToItem(palette.parent,
                                            anchorItem.width - palette.width,
                                            anchorItem.height + 6)
        palette.x = Math.max(12, Math.min(mapped.x, palette.parent.width - palette.width - 12))
        palette.y = Math.max(12, Math.min(mapped.y, palette.parent.height - palette.height - 12))
        palette.open()
    }

    function chooseOperator(operatorTypeKey) : void {
        const compatible = filterOperators("")
        for (let index = 0; index < compatible.length; ++index) {
            if (compatible[index].typeKey === operatorTypeKey) {
                palette.close()
                palette.operatorRequested(operatorTypeKey)
                return
            }
        }
    }

    parent: Overlay.overlay
    width: 382
    height: 420
    padding: 0
    modal: false
    dim: false
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    focus: true

    onOpened: {
        searchField.forceActiveFocus()
        searchField.selectAll()
    }

    background: Rectangle {
        radius: Theme.radiusLarge
        color: Theme.raised
        border.color: Theme.borderStrong
        layer.enabled: true
    }

    contentItem: ColumnLayout {
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            Layout.topMargin: 14
            Layout.bottomMargin: 12
            spacing: 10

            RowLayout {
                Layout.fillWidth: true

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1

                    Text {
                        text: qsTr("ADD OPERATOR")
                        color: Theme.text
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.5
                    }

                    Text {
                        text: qsTr("Only compatible Operators are shown")
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                    }
                }

                ShapeIconButton {
                    source: "qrc:/qt/qml/Shape/Desktop/icons/close.svg"
                    toolTipText: qsTr("Close")
                    onClicked: palette.close()
                }
            }

            TextField {
                id: searchField
                objectName: "operatorSearchField"
                Layout.fillWidth: true
                implicitHeight: 36
                leftPadding: 36
                rightPadding: 12
                placeholderText: qsTr("Search name, media, or capability…")
                color: Theme.text
                selectionColor: Theme.accent
                selectedTextColor: Theme.accentText

                background: Rectangle {
                    radius: Theme.radiusSmall
                    color: Theme.surface
                    border.color: searchField.activeFocus ? Theme.accent : Theme.border
                }

                ShapeIcon {
                    anchors.left: parent.left
                    anchors.leftMargin: 11
                    anchors.verticalCenter: parent.verticalCenter
                    source: "qrc:/qt/qml/Shape/Desktop/icons/search.svg"
                    size: 15
                    color: searchField.activeFocus ? Theme.accent : Theme.muted
                }

                Keys.onEscapePressed: palette.close()
                Keys.onReturnPressed: {
                    if (palette.filteredOperators.length === 1) {
                        palette.chooseOperator(palette.filteredOperators[0].typeKey)
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ListView {
            id: operatorList
            objectName: "operatorPaletteList"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 8
            clip: true
            spacing: 4
            model: palette.filteredOperators

            delegate: ItemDelegate {
                id: operatorDelegate
                objectName: modelData.objectName
                required property var modelData
                width: ListView.view.width
                height: 72
                leftPadding: 10
                rightPadding: 10
                topPadding: 8
                bottomPadding: 8
                hoverEnabled: true
                Accessible.name: modelData.label
                Accessible.description: modelData.description
                onClicked: palette.chooseOperator(modelData.typeKey)

                background: Rectangle {
                    radius: Theme.radiusMedium
                    color: operatorDelegate.down ? Theme.selected
                                                 : operatorDelegate.hovered
                                                   ? Theme.raisedHover : "transparent"
                    border.color: operatorDelegate.visualFocus ? Theme.accent : "transparent"
                }

                contentItem: RowLayout {
                    spacing: 11

                    Rectangle {
                        Layout.preferredWidth: 38
                        Layout.preferredHeight: 38
                        radius: 9
                        color: Theme.accentSoft

                        ShapeIcon {
                            anchors.centerIn: parent
                            source: operatorDelegate.modelData.icon
                            size: 19
                            color: Theme.accent
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                Layout.fillWidth: true
                                text: operatorDelegate.modelData.label
                                color: Theme.text
                                font.pixelSize: Theme.fontBody
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Text {
                                text: operatorDelegate.modelData.category
                                color: Theme.accent
                                font.pixelSize: 9
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.4
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: operatorDelegate.modelData.description
                            color: Theme.muted
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: operatorDelegate.modelData.typeKey
                            color: Theme.disabled
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Text {
                visible: palette.filteredOperators.length === 0
                anchors.centerIn: parent
                width: parent.width - 32
                text: qsTr("No compatible Operator matches this search.")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }
    }
}

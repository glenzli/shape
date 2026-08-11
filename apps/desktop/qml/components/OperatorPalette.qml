pragma ComponentBehavior: Bound

//! Searchable, type-compatible Operator chooser. Rust supplies compatible
//! machine descriptors; this owner adds localized presentation and search.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Popup {
    id: palette
    objectName: "operatorPalette"

    property var operators: []
    readonly property var filteredOperators: filterOperators(searchField.text)
    readonly property int compatibleOperatorCount: filterOperators("").length
    readonly property int visibleOperatorCount: filteredOperators.length

    signal operatorRequested(string operatorTypeKey)

    function labelFor(typeKey) : string {
        switch (typeKey) {
        case "text.edit":
        case "text.transform": return qsTr("AI text editor")
        case "audio.speech_synthesize": return qsTr("Turn text into speech")
        case "image.edit":
        case "image.crop":
        case "image.resize": return qsTr("Edit the image")
        default: return typeKey
        }
    }

    function descriptionFor(typeKey) : string {
        switch (typeKey) {
        case "text.edit":
        case "text.transform":
            return qsTr("Combine graph materials with a reusable prompt, tone, style, and candidate workflow.")
        case "audio.speech_synthesize":
            return qsTr("Create a spoken version from the current text.")
        case "image.edit":
        case "image.crop":
        case "image.resize":
            return qsTr("Frame, resize, and refine the image in one editing step.")
        default:
            return qsTr("Available creative action")
        }
    }

    function categoryFor(categoryKey) : string {
        switch (categoryKey) {
        case "text": return qsTr("WRITE")
        case "ai_text": return qsTr("WRITE")
        case "ai_audio": return qsTr("CREATE AUDIO")
        case "image": return qsTr("IMAGE")
        default: return qsTr("NEXT STEP")
        }
    }

    function keywordsFor(typeKey) : string {
        switch (typeKey) {
        case "text.edit":
        case "text.transform": return "text write edit revise ai transform rewrite expand polish llm"
        case "audio.speech_synthesize": return "ai audio speech synthesize voice tts"
        case "image.edit":
        case "image.crop":
        case "image.resize": return "image raster edit crop frame resize scale dimensions assist"
        default: return ""
        }
    }

    function objectNameFor(typeKey) : string {
        switch (typeKey) {
        case "text.edit":
        case "text.transform": return "addWritingOperatorAction"
        case "audio.speech_synthesize": return "addSpeechOperatorAction"
        case "image.edit":
        case "image.crop":
        case "image.resize": return "addImageEditingAction"
        default: return "compatibleOperatorAction"
        }
    }

    function iconFor(iconKey) : string {
        switch (iconKey) {
        case "edit": return "qrc:/qt/qml/Shape/Desktop/icons/edit.svg"
        case "sparkle": return "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
        case "waveform": return "qrc:/qt/qml/Shape/Desktop/icons/waveform.svg"
        case "crop": return "qrc:/qt/qml/Shape/Desktop/icons/crop.svg"
        case "fit": return "qrc:/qt/qml/Shape/Desktop/icons/fit.svg"
        default: return "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
        }
    }

    function catalog() : var {
        // The editor itself is graph-authorable before any material is
        // selected. Compatibility descriptors add source-bound actions, but
        // may never make the node library empty on a blank canvas.
        const result = [{
            "typeKey": "text.edit",
            "objectName": palette.objectNameFor("text.edit"),
            "label": palette.labelFor("text.edit"),
            "description": palette.descriptionFor("text.edit"),
            "category": palette.categoryFor("ai_text"),
            "keywords": palette.keywordsFor("text.edit"),
            "icon": palette.iconFor("sparkle"),
            "inputDataTypeKey": "text.document",
            "outputDataTypeKey": "text.document"
        }]
        let imageEditingAdded = false
        for (let index = 0; index < palette.operators.length; ++index) {
            const descriptor = palette.operators[index]
            if (descriptor.typeKey === "text.edit"
                    || descriptor.typeKey === "text.transform") continue
            if (descriptor.typeKey === "image.crop"
                    || descriptor.typeKey === "image.resize") {
                if (imageEditingAdded) continue
                imageEditingAdded = true
                result.push({
                    "typeKey": "image.edit",
                    "objectName": palette.objectNameFor("image.edit"),
                    "label": palette.labelFor("image.edit"),
                    "description": palette.descriptionFor("image.edit"),
                    "category": palette.categoryFor(descriptor.categoryKey),
                    "keywords": palette.keywordsFor("image.edit"),
                    "icon": palette.iconFor("crop"),
                    "inputDataTypeKey": descriptor.inputDataTypeKey,
                    "outputDataTypeKey": descriptor.outputDataTypeKey
                })
                continue
            }
            result.push({
                "typeKey": descriptor.typeKey,
                "objectName": palette.objectNameFor(descriptor.typeKey),
                "label": palette.labelFor(descriptor.typeKey),
                "description": palette.descriptionFor(descriptor.typeKey),
                "category": palette.categoryFor(descriptor.categoryKey),
                "keywords": palette.keywordsFor(descriptor.typeKey),
                "icon": palette.iconFor(descriptor.iconKey),
                "inputDataTypeKey": descriptor.inputDataTypeKey,
                "outputDataTypeKey": descriptor.outputDataTypeKey
            })
        }
        return result
    }

    function filterOperators(query) : var {
        const normalized = query.trim().toLowerCase()
        return catalog().filter(operator => {
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
                        text: qsTr("ADD A NODE")
                        color: Theme.text
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.5
                    }

                    Text {
                        text: qsTr("Nodes can be added without selecting another node")
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
                placeholderText: qsTr("Search the node library…")
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
                text: qsTr("No matching creative step was found.")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }
    }
}

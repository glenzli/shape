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
        case "text.create": return qsTr("Text creation")
        case "text.translate": return qsTr("Translate")
        case "text.summarize": return qsTr("Summarize")
        case "text.polish": return qsTr("Polish")
        case "text.expand": return qsTr("Expand")
        case "text.outline": return qsTr("Outline")
        case "text.prepare_script": return qsTr("Prepare a script")
        case "audio.generate": return qsTr("Create sound effects or music")
        case "image.generate": return qsTr("Generate an image")
        case "text.edit":
        case "text.transform": return qsTr("Text editing")
        case "audio.speech_synthesize": return qsTr("Turn text into speech")
        case "image.edit":
        case "image.crop":
        case "image.resize": return qsTr("Edit the image")
        default: return typeKey
        }
    }

    function dataTypeLabel(typeKey) : string {
        switch (typeKey) {
        case "text.document": return qsTr("Text")
        case "image.raster": return qsTr("Image")
        case "audio.clip": return qsTr("Audio")
        default: return qsTr("No source needed")
        }
    }

    function moveSelection(delta) : void {
        if (filteredOperators.length === 0) return
        operatorList.currentIndex = Math.max(0, Math.min(filteredOperators.length - 1,
                                                        operatorList.currentIndex + delta))
        operatorList.positionViewAtIndex(operatorList.currentIndex, ListView.Contain)
    }

    function chooseHighlighted() : void {
        const item = filteredOperators[operatorList.currentIndex]
        if (item) chooseOperator(item.typeKey)
    }

    onFilteredOperatorsChanged: operatorList.currentIndex = filteredOperators.length > 0 ? 0 : -1

    function descriptionFor(typeKey) : string {
        switch (typeKey) {
        case "text.translate": return qsTr("Translate an original into another language; keep a separate output.")
        case "text.summarize": return qsTr("Extract the main points into a concise summary.")
        case "text.polish": return qsTr("Improve grammar, clarity and flow while keeping the meaning.")
        case "text.expand": return qsTr("Develop a short text into a fuller draft.")
        case "text.outline": return qsTr("Organize an original into headings and key points.")
        case "text.prepare_script": return qsTr("Turn an original into a narration script with roles and timing.")
        case "audio.generate": return qsTr("Generate local sound effects or short music from a description.")
        case "image.generate": return qsTr("Create an image from a description with Codex Luna through Infer.")
        case "text.create": return qsTr("Start with an idea or write manually; choose plain text or a narration script.")
        case "text.edit":
        case "text.transform":
            return qsTr("Derive a separate text from an original: rewrite, translate, summarize, or prepare a script.")
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
        case "image.generate": return "image generate illustration poster cover codex luna 图片 生图 插画 海报 封面"
        case "text.translate": return "translate translation language 翻译 中英"
        case "text.summarize": return "summary summarize 摘要 总结"
        case "text.polish": return "polish grammar 润色 校对"
        case "text.expand": return "expand 扩写"
        case "text.outline": return "outline 大纲 提纲"
        case "text.prepare_script": return "script narration 脚本 口播 改编"
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
        default: return "add_" + typeKey.replace(".", "_")
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
        const result = []
        let imageEditingAdded = false
        for (let index = 0; index < palette.operators.length; ++index) {
            const descriptor = palette.operators[index]
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
        }).sort((a, b) => {
            const rank = item => item.label.toLowerCase() === normalized ? 0
                                  : item.label.toLowerCase().indexOf(normalized) >= 0 ? 1 : 2
            return rank(a) - rank(b)
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
    width: Math.min(420, parent.width - 24)
    height: Math.min(520, parent.height - 24)
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

            ShapeTextField {
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
                Keys.onDownPressed: palette.moveSelection(1)
                Keys.onUpPressed: palette.moveSelection(-1)
                Keys.onReturnPressed: palette.chooseHighlighted()
                Keys.onEnterPressed: palette.chooseHighlighted()
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
            ScrollBar.vertical: ScrollBar { policy: operatorList.contentHeight > operatorList.height ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff }
            spacing: 4
            model: palette.filteredOperators

            delegate: ItemDelegate {
                id: operatorDelegate
                objectName: modelData.objectName
                required property var modelData
                width: ListView.view.width - 12
                height: 88
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
                    color: operatorDelegate.down || operatorDelegate.ListView.isCurrentItem ? Theme.selected
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
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: palette.dataTypeLabel(operatorDelegate.modelData.inputDataTypeKey)
                                  + " → " + palette.dataTypeLabel(operatorDelegate.modelData.outputDataTypeKey)
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

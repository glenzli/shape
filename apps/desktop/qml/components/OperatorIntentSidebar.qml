pragma ComponentBehavior: Bound

//! Focused intent and constraint presentation for one comprehensive Operator.
//! The caller owns draft identity and persistence; this component owns only
//! transient editing, section interaction, and action affordances.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: sidebar
    objectName: "operatorIntentSidebar"

    property string operatorTitle: ""
    property string operatorKindLabel: ""
    property string intentText: ""
    property string intentPlaceholder: qsTr("Describe the result you want…")
    property var changeItems: []
    property var preserveItems: []
    property var references: []
    property bool allowReferences: true
    property bool allowCandidateCount: false
    property string referencesEmptyText: qsTr("No reference materials")
    property bool editable: true
    property bool running: false
    property bool stopRequested: false
    property string statusText: ""
    property string primaryActionText: qsTr("Create a version")
    property bool primaryActionEnabled: true
    property string defaultModelKey: "gpt_5_6_luna"
    property string defaultEffortKey: ""
    property string modelContextId: ""
    property int defaultCandidateCount: 1
    property int candidateCountOverride: 0
    readonly property int selectedCandidateCount: Math.max(1, Math.min(4,
        candidateCountOverride > 0 ? candidateCountOverride : defaultCandidateCount))
    property string modelOverride: ""
    readonly property string selectedModelKey: modelPicker.effectiveModelKey
    property string effortOverride: ""
    readonly property string selectedEffortKey: modelPicker.effectiveEffortKey
    readonly property string editedIntentText: intentEditor.text

    onModelContextIdChanged: {
        modelOverride = ""
        effortOverride = ""
        candidateCountOverride = 0
    }

    signal intentEdited(string text)
    signal intentCommitRequested(string text)
    signal changeItemActivated(int index)
    signal preserveItemActivated(int index)
    signal referenceActivated(int index)
    signal addReferenceRequested()
    signal primaryActionRequested()
    signal candidateCountSelected(int count)
    signal stopActionRequested()

    function itemLabel(item) : string {
        if (typeof item === "string") return item
        if (!item) return ""
        if (item.label !== undefined) return String(item.label)
        if (item.name !== undefined) return String(item.name)
        return ""
    }

    function itemDetail(item) : string {
        if (!item || typeof item === "string") return ""
        if (item.detail !== undefined) return String(item.detail)
        if (item.kindLabel !== undefined) return String(item.kindLabel)
        return ""
    }

    implicitWidth: 276
    color: Theme.panel
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 12

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Text {
                Layout.fillWidth: true
                text: sidebar.operatorTitle.length > 0
                      ? sidebar.operatorTitle : qsTr("Creative direction")
                color: Theme.text
                font.pixelSize: Theme.fontHeading
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                text: sidebar.operatorKindLabel
                visible: text.length > 0
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        Text {
            text: qsTr("WHAT YOU WANT")
            color: Theme.muted
            font.pixelSize: Theme.fontMicro
            font.weight: Font.DemiBold
            font.letterSpacing: 0.7
        }

        ShapeTextEditor {
            id: intentEditor
            objectName: "operatorIntentEditor"

            Layout.fillWidth: true
            Layout.preferredHeight: 86
            text: sidebar.intentText
            enabled: sidebar.editable && !sidebar.running
            placeholderText: sidebar.intentPlaceholder
            color: Theme.text
            placeholderTextColor: Theme.muted
            selectionColor: Theme.accentSoft
            selectedTextColor: Theme.text
            font.pixelSize: Theme.fontBody
            wrapMode: TextEdit.Wrap
            Accessible.name: qsTr("Creative direction")
            onTextChanged: {
                if (editorActiveFocus && text !== sidebar.intentText) sidebar.intentEdited(text)
            }
            onEditorActiveFocusChanged: {
                if (!editorActiveFocus && text !== sidebar.intentText) {
                    sidebar.intentCommitRequested(text)
                }
            }

            background: Rectangle {
                radius: Theme.controlRadius
                color: Theme.control
                border.color: intentEditor.editorActiveFocus ? Theme.focusRing : Theme.border
            }
        }

        ScrollView {
            id: detailsScroll
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: contentHeight > availableHeight ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth

            ColumnLayout {
                width: detailsScroll.availableWidth - 12
                spacing: 12

                IntentListSection {
                    Layout.fillWidth: true
                    title: qsTr("CHANGE")
                    emptyText: qsTr("No explicit changes")
                    items: sidebar.changeItems
                    accentColor: Theme.accent
                    onItemActivated: index => sidebar.changeItemActivated(index)
                }

                IntentListSection {
                    Layout.fillWidth: true
                    title: qsTr("PRESERVE")
                    emptyText: qsTr("No preserve constraints")
                    items: sidebar.preserveItems
                    accentColor: Theme.success
                    onItemActivated: index => sidebar.preserveItemActivated(index)
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    RowLayout {
                        Layout.fillWidth: true

                        Text {
                            text: qsTr("REFERENCES")
                            color: Theme.muted
                            font.pixelSize: Theme.fontMicro
                            font.weight: Font.DemiBold
                            font.letterSpacing: 0.7
                        }

                        Item { Layout.fillWidth: true }

                        ShapeIconButton {
                            visible: sidebar.allowReferences
                            source: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
                            toolTipText: qsTr("Add reference")
                            accessibleName: toolTipText
                            enabled: sidebar.editable && !sidebar.running
                            buttonSize: 24
                            iconSize: 14
                            onClicked: sidebar.addReferenceRequested()
                        }
                    }

                    Repeater {
                        model: sidebar.references

                        delegate: ItemDelegate {
                            id: referenceDelegate

                            required property int index
                            required property var modelData

                            Layout.fillWidth: true
                            implicitHeight: 42
                            leftPadding: 8
                            rightPadding: 8
                            onClicked: sidebar.referenceActivated(index)

                            background: Rectangle {
                                radius: Theme.controlRadius
                                color: referenceDelegate.hovered
                                       ? Theme.raisedHover : Theme.raised
                                border.color: Theme.border
                            }

                            contentItem: RowLayout {
                                spacing: 7

                                ShapeIcon {
                                    source: "qrc:/qt/qml/Shape/Desktop/icons/open.svg"
                                    size: 14
                                    color: Theme.muted
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 1

                                    Text {
                                        Layout.fillWidth: true
                                        text: sidebar.itemLabel(referenceDelegate.modelData)
                                        color: Theme.textSoft
                                        font.pixelSize: Theme.fontMeta
                                        elide: Text.ElideRight
                                    }

                                    Text {
                                        Layout.fillWidth: true
                                        text: sidebar.itemDetail(referenceDelegate.modelData)
                                        visible: text.length > 0
                                        color: Theme.muted
                                        font.pixelSize: Theme.fontMicro
                                        elide: Text.ElideRight
                                    }
                                }
                            }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: sidebar.references.length === 0
                        text: sidebar.referencesEmptyText
                        color: Theme.muted
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: sidebar.statusText.length > 0
            text: sidebar.statusText
            color: Theme.muted
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }

        AiModelPicker {
            id: modelPicker
            objectName: "imageModelPicker"
            family: "image"
            defaultModelKey: sidebar.defaultModelKey
            defaultEffortKey: sidebar.defaultEffortKey
            overrideKey: sidebar.modelOverride
            effortOverrideKey: sidebar.effortOverride
            enabled: sidebar.editable && !sidebar.running
            onChoiceSelected: key => sidebar.modelOverride = key
            onEffortSelected: key => sidebar.effortOverride = key
        }

        RowLayout {
            Layout.fillWidth: true
            visible: sidebar.allowCandidateCount
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: qsTr("Versions this run")
                color: Theme.textSoft
                font.pixelSize: Theme.fontMeta
            }

            ShapeComboBox {
                objectName: "imageCandidateCountPicker"
                Layout.preferredWidth: 76
                model: ["1", "2", "3", "4"]
                currentIndex: Math.max(0, Math.min(3, sidebar.selectedCandidateCount - 1))
                enabled: sidebar.editable && !sidebar.running
                Accessible.name: qsTr("Number of image versions")
                onActivated: index => {
                    sidebar.candidateCountOverride = index + 1
                    sidebar.candidateCountSelected(index + 1)
                }
            }
        }

        ShapeButton {
            objectName: "operatorIntentPrimaryAction"
            Layout.fillWidth: true
            text: sidebar.running ? qsTr("Working…") : sidebar.primaryActionText
            iconSource: "qrc:/qt/qml/Shape/Desktop/icons/sparkle.svg"
            primary: true
            enabled: sidebar.editable && !sidebar.running
                     && sidebar.primaryActionEnabled
            onClicked: sidebar.primaryActionRequested()
        }

        ShapeButton {
            objectName: "imageBatchStopButton"
            Layout.fillWidth: true
            visible: sidebar.allowCandidateCount && sidebar.running
            text: sidebar.stopRequested ? qsTr("Stopping after this image…")
                                        : qsTr("Stop after this image")
            enabled: !sidebar.stopRequested
            onClicked: sidebar.stopActionRequested()
        }
    }

    component IntentListSection: ColumnLayout {
        id: section

        required property string title
        required property string emptyText
        required property var items
        required property color accentColor

        signal itemActivated(int index)

        spacing: 6

        Text {
            text: section.title
            color: Theme.muted
            font.pixelSize: Theme.fontMicro
            font.weight: Font.DemiBold
            font.letterSpacing: 0.7
        }

        Repeater {
            model: section.items

            delegate: ItemDelegate {
                id: sectionItem

                required property int index
                required property var modelData

                Layout.fillWidth: true
                implicitHeight: 34
                leftPadding: 8
                rightPadding: 8
                onClicked: section.itemActivated(index)

                background: Rectangle {
                    radius: Theme.controlRadius
                    color: sectionItem.hovered ? Theme.raisedHover : Theme.raised
                    border.color: Theme.border
                }

                contentItem: RowLayout {
                    spacing: 7

                    Rectangle {
                        Layout.preferredWidth: 5
                        Layout.preferredHeight: 5
                        radius: 3
                        color: section.accentColor
                    }

                    Text {
                        Layout.fillWidth: true
                        text: sidebar.itemLabel(sectionItem.modelData)
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMeta
                        elide: Text.ElideRight
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: section.items.length === 0
            text: section.emptyText
            color: Theme.muted
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }
    }
}

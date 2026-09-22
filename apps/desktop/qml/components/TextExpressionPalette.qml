pragma ComponentBehavior: Bound

//! Visual authored-expression owner for AI text editing. Built-in and personal
//! tones compile to one bounded project-draft snapshot; the personal library is
//! only a reusable source and never remains a live dependency of the draft.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: palette
    objectName: "textExpressionPalette"
    implicitWidth: paletteLayout.implicitWidth
    implicitHeight: paletteLayout.implicitHeight

    property string persistedExpressionJson: ""
    property var selectedTones: [
        {
            "kind": "preset",
            "preset": "neutral"
        }
    ]
    property string intensityKey: "balanced"
    property string audienceKey: "general"
    property string customAudienceName: ""
    property string customAudienceInstruction: ""
    property string styleKey: "natural"
    property bool restoring: false

    readonly property string expressionJson: JSON.stringify(currentExpression())
    readonly property bool expressionComplete: audienceKey !== "custom" || (customAudienceName.trim().length > 0 && customAudienceInstruction.trim().length > 0)
    readonly property var builtInTones: [
        {
            "key": "neutral",
            "label": qsTr("Calm"),
            "visual": "ripple",
            "description": qsTr("Even and unforced"),
            "example": qsTr("Let’s take this one step at a time.")
        },
        {
            "key": "warm",
            "label": qsTr("Warm"),
            "visual": "glow",
            "description": qsTr("Open and welcoming"),
            "example": qsTr("I’m glad you told me.")
        },
        {
            "key": "empathetic",
            "label": qsTr("Empathetic"),
            "visual": "embrace",
            "description": qsTr("Attentive to feelings"),
            "example": qsTr("That sounds difficult; I understand why.")
        },
        {
            "key": "confident",
            "label": qsTr("Confident"),
            "visual": "ascent",
            "description": qsTr("Clear and assured"),
            "example": qsTr("This is the right direction; let’s proceed.")
        },
        {
            "key": "playful",
            "label": qsTr("Playful"),
            "visual": "spark",
            "description": qsTr("Light and energetic"),
            "example": qsTr("Let’s give it a little spark.")
        },
        {
            "key": "restrained",
            "label": qsTr("Restrained"),
            "visual": "frame",
            "description": qsTr("Measured and subtle"),
            "example": qsTr("This still needs some adjustment.")
        },
        {
            "key": "serious",
            "label": qsTr("Serious"),
            "visual": "pillar",
            "description": qsTr("Firm and considered"),
            "example": qsTr("This issue requires careful attention.")
        },
        {
            "key": "urgent",
            "label": qsTr("Urgent"),
            "visual": "pulse",
            "description": qsTr("Direct and immediate"),
            "example": qsTr("Please act on this now.")
        }
    ]
    readonly property var audiences: [
        {
            "key": "general",
            "label": qsTr("General")
        },
        {
            "key": "close_friend",
            "label": qsTr("Close friend")
        },
        {
            "key": "colleague",
            "label": qsTr("Colleague")
        },
        {
            "key": "customer",
            "label": qsTr("Customer")
        },
        {
            "key": "public",
            "label": qsTr("Public")
        },
        {
            "key": "expert",
            "label": qsTr("Expert")
        },
        {
            "key": "beginner",
            "label": qsTr("Beginner")
        },
        {
            "key": "custom",
            "label": qsTr("Custom…")
        }
    ]

    signal expressionEdited(string expressionJson)

    function defaultExpression(): var {
        return {
            "tones": [
                {
                    "kind": "preset",
                    "preset": "neutral"
                }
            ],
            "intensity": "balanced",
            "audience": {
                "kind": "preset",
                "preset": "general"
            }
        };
    }

    function currentExpression(): var {
        const audience = audienceKey === "custom" ? {
            "kind": "custom",
            "name": customAudienceName.trim(),
            "instruction": customAudienceInstruction.trim()
        } : {
            "kind": "preset",
            "preset": audienceKey
        };
        return {
            "tones": selectedTones,
            "intensity": intensityKey,
            "audience": audience
        };
    }

    function isPresetSelected(key): bool {
        for (let index = 0; index < selectedTones.length; ++index) {
            if (selectedTones[index].kind === "preset" && selectedTones[index].preset === key)
                return true;
        }
        return false;
    }

    function isCustomSelected(name, instruction, example, visual): bool {
        for (let index = 0; index < selectedTones.length; ++index) {
            const tone = selectedTones[index];
            if (tone.kind === "custom" && tone.name === name && tone.instruction === instruction && (tone.example || "") === (example || "") && tone.visual === visual)
                return true;
        }
        return false;
    }

    function choosePreset(key): void {
        let next = selectedTones.filter(tone => tone.kind !== "preset" || tone.preset !== key);
        if (next.length === selectedTones.length) {
            if (key === "neutral") {
                next = [
                    {
                        "kind": "preset",
                        "preset": "neutral"
                    }
                ];
            } else {
                next = next.filter(tone => tone.kind !== "preset" || tone.preset !== "neutral");
                if (next.length >= 2)
                    next.shift();
                next.push({
                    "kind": "preset",
                    "preset": key
                });
            }
        }
        if (next.length === 0)
            next = [
                {
                    "kind": "preset",
                    "preset": "neutral"
                }
            ];
        selectedTones = next;
        notifyEdited();
    }

    function chooseCustom(preset): void {
        let next = selectedTones.filter(tone => tone.kind !== "preset" || tone.preset !== "neutral");
        next = next.filter(tone => tone.kind !== "custom");
        if (!isCustomSelected(preset.name, preset.instruction, preset.example, preset.visualKey)) {
            if (next.length >= 2)
                next.shift();
            const customTone = {
                "kind": "custom",
                "name": preset.name,
                "instruction": preset.instruction,
                "visual": preset.visualKey
            };
            if (preset.example && preset.example.trim().length > 0)
                customTone.example = preset.example.trim();
            next.push(customTone);
        }
        if (next.length === 0)
            next = [
                {
                    "kind": "preset",
                    "preset": "neutral"
                }
            ];
        selectedTones = next;
        notifyEdited();
    }

    function chooseIntensity(key): void {
        intensityKey = key;
        notifyEdited();
    }

    function chooseAudience(key): void {
        audienceKey = key;
        notifyEdited();
    }

    function notifyEdited(): void {
        if (!restoring && expressionComplete)
            expressionEdited(expressionJson);
    }

    function restoreExpression(json): void {
        restoring = true;
        let decoded = defaultExpression();
        try {
            if (json.length > 0)
                decoded = JSON.parse(json);
        } catch (error) {
            decoded = defaultExpression();
        }
        selectedTones = decoded.tones && decoded.tones.length > 0 ? decoded.tones.slice(0, 2) : defaultExpression().tones;
        intensityKey = ["subtle", "balanced", "strong"].indexOf(decoded.intensity) >= 0 ? decoded.intensity : "balanced";
        if (decoded.audience && decoded.audience.kind === "custom") {
            audienceKey = "custom";
            customAudienceName = decoded.audience.name || "";
            customAudienceInstruction = decoded.audience.instruction || "";
        } else {
            const key = decoded.audience ? decoded.audience.preset : "general";
            audienceKey = audienceIndex(key) >= 0 ? key : "general";
            customAudienceName = "";
            customAudienceInstruction = "";
        }
        restoring = false;
    }

    function audienceIndex(key): int {
        for (let index = 0; index < audiences.length; ++index) {
            if (audiences[index].key === key)
                return index;
        }
        return -1;
    }

    function presetSelectionIndex(key): int {
        for (let index = 0; index < selectedTones.length; ++index) {
            const tone = selectedTones[index];
            if (tone.kind === "preset" && tone.preset === key)
                return index;
        }
        return -1;
    }

    function customSelectionIndex(preset): int {
        for (let index = 0; index < selectedTones.length; ++index) {
            const tone = selectedTones[index];
            if (tone.kind === "custom" && tone.name === preset.name && tone.instruction === preset.instruction && (tone.example || "") === (preset.example || "") && tone.visual === preset.visualKey)
                return index;
        }
        return -1;
    }

    function selectionRoleLabel(index): string {
        if (index === 0)
            return qsTr("Primary");
        if (index === 1)
            return qsTr("Accent");
        return "";
    }

    function toneLabel(tone): string {
        if (tone.kind === "custom")
            return tone.name;
        for (let index = 0; index < builtInTones.length; ++index) {
            if (builtInTones[index].key === tone.preset)
                return builtInTones[index].label;
        }
        return qsTr("Calm");
    }

    function toneExample(tone): string {
        if (tone.kind === "custom")
            return tone.example || "";
        for (let index = 0; index < builtInTones.length; ++index) {
            if (builtInTones[index].key === tone.preset)
                return builtInTones[index].example;
        }
        return "";
    }

    function primaryToneKey(): string {
        const tone = selectedTones.length > 0 ? selectedTones[0] : defaultExpression().tones[0];
        return tone.kind === "custom" ? "personal" : tone.preset;
    }

    function toneColor(visualKey): color {
        if (visualKey === "glow")
            return "#c98255";
        if (visualKey === "embrace")
            return "#aa7c9b";
        if (visualKey === "ascent")
            return "#668e9b";
        if (visualKey === "spark")
            return "#bf9655";
        if (visualKey === "frame")
            return "#77808d";
        if (visualKey === "pillar")
            return "#70768a";
        if (visualKey === "pulse")
            return "#bd6d70";
        return "#6d97b5";
    }

    function intensityLabel(): string {
        if (intensityKey === "subtle")
            return qsTr("Subtle intensity");
        if (intensityKey === "strong")
            return qsTr("Strong intensity");
        return qsTr("Balanced intensity");
    }

    function audienceLabel(): string {
        if (audienceKey === "custom")
            return qsTr("For %1").arg(customAudienceName);
        const index = audienceIndex(audienceKey);
        return index >= 0 ? qsTr("For %1").arg(audiences[index].label) : qsTr("For General");
    }

    function styleLabel(): string {
        if (styleKey === "concise")
            return qsTr("Concise style");
        if (styleKey === "professional")
            return qsTr("Professional style");
        if (styleKey === "literary")
            return qsTr("Literary style");
        if (styleKey === "casual")
            return qsTr("Casual style");
        return qsTr("Natural style");
    }

    function expressionSummary(): string {
        const primary = toneLabel(selectedTones[0]);
        const toneSummary = selectedTones.length > 1 ? qsTr("Mainly %1 · %2 as an accent").arg(primary).arg(toneLabel(selectedTones[1])) : qsTr("Mainly %1").arg(primary);
        return audienceLabel() + " · " + toneSummary + " · " + intensityLabel() + " · " + styleLabel();
    }

    onPersistedExpressionJsonChanged: restoreExpression(persistedExpressionJson)
    Component.onCompleted: restoreExpression(persistedExpressionJson)

    TextExpressionLibrary {
        id: personalLibrary
    }

    component ToneGlyph: Canvas {
        id: glyph
        required property string visualKey
        property color strokeColor: Theme.textSoft

        implicitWidth: 30
        implicitHeight: 30
        antialiasing: true
        onVisualKeyChanged: requestPaint()
        onStrokeColorChanged: requestPaint()
        onPaint: {
            const context = getContext("2d");
            context.reset();
            context.strokeStyle = strokeColor.toString();
            context.fillStyle = strokeColor.toString();
            context.lineWidth = 1.8;
            context.lineCap = "round";
            context.lineJoin = "round";
            const center = width / 2;
            if (visualKey === "glow") {
                context.beginPath();
                context.arc(center, center, 5.5, 0, Math.PI * 2);
                context.stroke();
                for (let index = 0; index < 8; ++index) {
                    const angle = Math.PI * 2 * index / 8;
                    context.beginPath();
                    context.moveTo(center + Math.cos(angle) * 9, center + Math.sin(angle) * 9);
                    context.lineTo(center + Math.cos(angle) * 12, center + Math.sin(angle) * 12);
                    context.stroke();
                }
            } else if (visualKey === "embrace") {
                context.beginPath();
                context.arc(11, 15, 7, -1.2, 1.2);
                context.arc(19, 15, 7, 1.94, 4.34);
                context.stroke();
            } else if (visualKey === "ascent") {
                context.beginPath();
                context.moveTo(6, 22);
                context.lineTo(14, 14);
                context.lineTo(19, 17);
                context.lineTo(25, 8);
                context.stroke();
                context.beginPath();
                context.moveTo(20, 8);
                context.lineTo(25, 8);
                context.lineTo(25, 13);
                context.stroke();
            } else if (visualKey === "spark") {
                context.beginPath();
                context.moveTo(center, 4);
                context.lineTo(17.5, 12.5);
                context.lineTo(26, center);
                context.lineTo(17.5, 17.5);
                context.lineTo(center, 26);
                context.lineTo(12.5, 17.5);
                context.lineTo(4, center);
                context.lineTo(12.5, 12.5);
                context.closePath();
                context.stroke();
            } else if (visualKey === "frame") {
                context.strokeRect(7, 8, 16, 14);
                context.beginPath();
                context.moveTo(11, 12);
                context.lineTo(19, 12);
                context.moveTo(11, 17);
                context.lineTo(17, 17);
                context.stroke();
            } else if (visualKey === "pillar") {
                context.beginPath();
                context.moveTo(8, 23);
                context.lineTo(22, 23);
                context.moveTo(10, 8);
                context.lineTo(20, 8);
                context.moveTo(12, 8);
                context.lineTo(12, 23);
                context.moveTo(18, 8);
                context.lineTo(18, 23);
                context.stroke();
            } else if (visualKey === "pulse") {
                context.beginPath();
                context.moveTo(3, 16);
                context.lineTo(9, 16);
                context.lineTo(12, 8);
                context.lineTo(17, 23);
                context.lineTo(20, 13);
                context.lineTo(27, 13);
                context.stroke();
            } else {
                for (let index = 0; index < 3; ++index) {
                    context.beginPath();
                    context.arc(center, 17, 5 + index * 4, Math.PI * 1.1, Math.PI * 1.9);
                    context.stroke();
                }
            }
        }
    }

    component ToneOption: ShapeButton {
        id: option
        required property string title
        required property string visualKey
        property string detail: ""
        property string roleLabel: ""
        property color semanticColor: Theme.muted
        property bool current: false

        implicitHeight: 50
        focusPolicy: Qt.StrongFocus
        hoverEnabled: true
        Accessible.name: [title, detail, roleLabel].filter(value => value.length > 0).join(". ")

        background: Rectangle {
            radius: Theme.compactControlRadius
            color: option.current ? Theme.accentSoft : option.down ? Theme.buttonGhostPressed : option.hovered ? Theme.buttonGhostHover : Theme.control
            border.width: option.current || option.visualFocus ? 1 : 0
            border.color: option.visualFocus ? Theme.focusRing : option.current ? Theme.accentBorder : Theme.border
        }

        contentItem: RowLayout {
            spacing: 7

            Rectangle {
                Layout.preferredWidth: 4
                Layout.fillHeight: true
                radius: 2
                color: option.semanticColor
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        Layout.fillWidth: true
                        text: option.title
                        color: option.current ? Theme.accent : Theme.text
                        font.pixelSize: Theme.fontMeta
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        visible: option.roleLabel.length > 0
                        text: option.roleLabel
                        color: Theme.accent
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                    }
                }
                Text {
                    objectName: option.objectName + "-description"
                    Layout.fillWidth: true
                    text: option.detail
                    color: Theme.textSoft
                    font.pixelSize: Theme.fontMicro
                    elide: Text.ElideRight
                }
            }
        }
    }

    component SegmentChip: ShapeButton {
        id: segment
        property bool current: false
        implicitHeight: Theme.compactControlHeight
        leftPadding: 8
        rightPadding: 8
        background: Rectangle {
            radius: Theme.compactControlRadius
            color: segment.current ? Theme.accentSoft : segment.hovered ? Theme.buttonGhostHover : Theme.control
            border.width: segment.current || segment.visualFocus ? 1 : 0
            border.color: segment.visualFocus ? Theme.focusRing : Theme.accentBorder
        }
        contentItem: Text {
            text: segment.text
            color: segment.current ? Theme.accent : Theme.textSoft
            font.pixelSize: Theme.fontMeta
            font.weight: segment.current ? Font.DemiBold : Font.Normal
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    ColumnLayout {
        id: paletteLayout
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text {
                    text: qsTr("TONE")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMicro
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.6
                }
                Text {
                    Layout.fillWidth: true
                    text: qsTr("Choose up to two; these describe intent, not model limits.")
                    color: Theme.muted
                    font.pixelSize: Theme.fontMicro
                    wrapMode: Text.WordWrap
                }
            }

            ShapeIconButton {
                objectName: "addCustomToneButton"
                source: "qrc:/qt/qml/Shape/Desktop/icons/add.svg"
                toolTipText: qsTr("Create a personal tone")
                accessibleName: toolTipText
                buttonSize: 28
                onClicked: customTonePopup.open()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 52
            radius: Theme.compactControlRadius
            color: Theme.surfaceSubtle
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 1

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 5

                    Text {
                        id: expressionSummaryText
                        objectName: "textExpressionSummaryText"
                        Layout.fillWidth: true
                        text: palette.expressionSummary()
                        color: Theme.textSoft
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        objectName: "textTone-" + palette.primaryToneKey() + "-role"
                        text: qsTr("Primary")
                        color: Theme.accent
                        font.pixelSize: Theme.fontMicro
                        font.weight: Font.DemiBold
                    }
                }

                Text {
                    objectName: "textTone-" + palette.primaryToneKey() + "-example"
                    Layout.fillWidth: true
                    text: qsTr("“%1”").arg(palette.toneExample(palette.selectedTones[0]))
                    color: Theme.muted
                    font.pixelSize: Theme.fontMicro
                    font.italic: true
                    elide: Text.ElideRight
                }
            }
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: 6
            rowSpacing: 6

            Repeater {
                model: palette.builtInTones

                delegate: ToneOption {
                    required property var modelData
                    objectName: "textTone-" + modelData.key
                    Layout.fillWidth: true
                    title: modelData.label
                    detail: modelData.description
                    visualKey: modelData.visual
                    semanticColor: palette.toneColor(modelData.visual)
                    current: palette.isPresetSelected(modelData.key)
                    roleLabel: palette.selectionRoleLabel(palette.presetSelectionIndex(modelData.key))
                    onClicked: palette.choosePreset(modelData.key)
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: personalLibrary.presets.length > 0
            spacing: 6

            Text {
                text: qsTr("PERSONAL TONES")
                color: Theme.muted
                font.pixelSize: Theme.fontMicro
                font.weight: Font.DemiBold
                font.letterSpacing: 0.6
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 6
                rowSpacing: 6

                Repeater {
                    model: personalLibrary.presets

                    delegate: ToneOption {
                        required property var modelData
                        objectName: "customTone-" + modelData.id
                        Layout.fillWidth: true
                        title: modelData.name
                        detail: modelData.instruction
                        visualKey: modelData.visualKey
                        semanticColor: palette.toneColor(modelData.visualKey)
                        current: palette.isCustomSelected(modelData.name, modelData.instruction, modelData.example, modelData.visualKey)
                        roleLabel: palette.selectionRoleLabel(palette.customSelectionIndex(modelData))
                        onClicked: palette.chooseCustom(modelData)
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                Layout.fillWidth: true
                text: qsTr("Intensity")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
            }
            Repeater {
                model: [
                    {
                        "key": "subtle",
                        "label": qsTr("Subtle")
                    },
                    {
                        "key": "balanced",
                        "label": qsTr("Balanced")
                    },
                    {
                        "key": "strong",
                        "label": qsTr("Strong")
                    }
                ]
                delegate: SegmentChip {
                    required property var modelData
                    text: modelData.label
                    current: palette.intensityKey === modelData.key
                    onClicked: palette.chooseIntensity(modelData.key)
                }
            }
        }

        Text {
            text: qsTr("AUDIENCE")
            color: Theme.muted
            font.pixelSize: Theme.fontMicro
            font.weight: Font.DemiBold
            font.letterSpacing: 0.6
        }

        ShapeComboBox {
            id: audienceCombo
            objectName: "textAudienceSelector"
            Layout.fillWidth: true
            model: palette.audiences
            textRole: "label"
            valueRole: "key"
            currentIndex: Math.max(0, palette.audienceIndex(palette.audienceKey))
            onActivated: index => palette.chooseAudience(palette.audiences[index].key)
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: palette.audienceKey === "custom"
            spacing: 6

            ShapeTextField {
                id: customAudienceNameField
                Layout.fillWidth: true
                text: palette.customAudienceName
                placeholderText: qsTr("Audience name")
                maximumLength: 64
                onEditingFinished: {
                    palette.customAudienceName = text;
                    palette.notifyEdited();
                }
            }
            ShapeTextArea {
                id: customAudienceInstructionField
                Layout.fillWidth: true
                Layout.preferredHeight: 62
                text: palette.customAudienceInstruction
                placeholderText: qsTr("What should this audience understand or feel?")
                wrapMode: TextEdit.Wrap
                onActiveFocusChanged: if (!activeFocus) {
                    palette.customAudienceInstruction = text;
                    palette.notifyEdited();
                }
            }
        }
    }

    property Popup customToneDialog: Popup {
        id: customTonePopup
        parent: Overlay.overlay
        implicitWidth: 420
        x: Math.round((parent.width - implicitWidth) / 2)
        y: Math.round((parent.height - implicitHeight) / 2)
        modal: true
        focus: true
        padding: 18
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: Theme.panelRaised
            radius: Theme.panelRadius
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 12

            Text {
                text: qsTr("Create a personal tone")
                color: Theme.text
                font.pixelSize: Theme.fontHeading
                font.weight: Font.DemiBold
            }
            Text {
                Layout.fillWidth: true
                text: qsTr("Name the feeling in your own words. Shape saves it for reuse and copies its meaning into each node.")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }
            ShapeTextField {
                id: customToneNameField
                objectName: "customToneNameField"
                Layout.fillWidth: true
                placeholderText: qsTr("For example: quiet conviction")
                maximumLength: 64
            }
            ShapeTextArea {
                id: customToneInstructionField
                objectName: "customToneInstructionField"
                Layout.fillWidth: true
                Layout.preferredHeight: 86
                placeholderText: qsTr("Describe how it should sound, and what it should avoid.")
                wrapMode: TextEdit.Wrap
            }
            ShapeTextArea {
                id: customToneExampleField
                objectName: "customToneExampleField"
                Layout.fillWidth: true
                Layout.preferredHeight: 72
                placeholderText: qsTr("Write one short sentence that sounds like this tone.")
                wrapMode: TextEdit.Wrap
                onTextChanged: if (text.length > 64)
                    text = text.slice(0, 64)
            }
            Text {
                text: qsTr("Visual mark")
                color: Theme.muted
                font.pixelSize: Theme.fontMeta
            }
            RowLayout {
                Layout.fillWidth: true
                Repeater {
                    model: ["ripple", "glow", "embrace", "ascent", "spark", "frame", "pillar", "pulse"]
                    delegate: ShapeButton {
                        id: visualChoice
                        required property string modelData
                        expanded: true
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        implicitWidth: 36
                        implicitHeight: 36
                        background: Rectangle {
                            radius: Theme.compactControlRadius
                            color: customTonePopup.customToneVisualKey === visualChoice.modelData ? Theme.accentSoft : Theme.control
                            border.width: customTonePopup.customToneVisualKey === visualChoice.modelData ? 1 : 0
                            border.color: Theme.accentBorder
                        }
                        contentItem: ToneGlyph {
                            anchors.centerIn: parent
                            visualKey: visualChoice.modelData
                            strokeColor: customTonePopup.customToneVisualKey === visualChoice.modelData ? Theme.accent : Theme.textSoft
                        }
                        onClicked: customTonePopup.customToneVisualKey = visualChoice.modelData
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Item {
                    Layout.fillWidth: true
                }
                ShapeButton {
                    text: qsTr("Cancel")
                    onClicked: customTonePopup.close()
                }
                ShapeButton {
                    objectName: "saveCustomToneButton"
                    primary: true
                    text: qsTr("Save tone")
                    enabled: customToneNameField.text.trim().length > 0 && customToneInstructionField.text.trim().length > 0 && customToneExampleField.text.trim().length > 0
                    onClicked: {
                        const id = personalLibrary.savePreset("", customToneNameField.text, customToneInstructionField.text, customToneExampleField.text, customTonePopup.customToneVisualKey);
                        if (id.length === 0)
                            return;
                        const preset = personalLibrary.presets[0];
                        palette.chooseCustom(preset);
                        customToneNameField.clear();
                        customToneInstructionField.clear();
                        customToneExampleField.clear();
                        customTonePopup.customToneVisualKey = "ripple";
                        customTonePopup.close();
                    }
                }
            }
        }

        property string customToneVisualKey: "ripple"
    }
}

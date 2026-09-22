pragma ComponentBehavior: Bound

//! Preset-only speech Operator workspace. Generation and playback lifecycles
//! remain in host-created controllers; this owner keeps authored UI state.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: workspace
    objectName: "audioSpeechOperatorWorkspace"

    required property DesktopBackend backend
    required property InferSpeechController inferSpeech
    required property AudioPreviewController audioPreview
    property string projectPath: ""
    property string operatorDraftId: ""
    property string outputName: ""
    property string outputArtifactId: ""
    property string sourceArtifactId: ""
    property string sourceName: ""
    property string sourceText: ""
    property bool canGenerate: false
    property bool credentialConfigured: false
    property string acceptedAudioArtifactId: ""
    property bool sourceChanged: false
    property int acceptedDurationMillis: 0
    property int acceptedSampleRateHz: 0
    property int acceptedChannels: 0
    property string acceptedOriginKey: ""
    property var candidate: null
    property string presetAlias: ""
    property string presetCatalogRevision: ""
    property string scriptJson: ""
    property var scriptPreview: ({})
    readonly property bool scriptMode: scriptJson.length > 0
    function refreshScriptPreview() : void {
        const json = backend.speechScriptPreview(operatorDraftId)
        scriptPreview = json.length > 0 ? JSON.parse(json) : ({})
    }
    onScriptJsonChanged: Qt.callLater(workspace.refreshScriptPreview)
    onSourceTextChanged: Qt.callLater(workspace.refreshScriptPreview)
    property string language: ""
    property int speedMilli: 0
    property bool syntheticDisclosureRequired: false
    property int pendingSpeedMilli: speedMilli > 0 ? speedMilli : 1000

    readonly property bool hasCandidateAudio: candidate !== null
                                                && candidate.hasAudioPreview
    readonly property string previewArtifactId: hasCandidateAudio
                                                ? outputArtifactId
                                                : acceptedAudioArtifactId
    readonly property string previewCandidateId: hasCandidateAudio ? candidate.id : ""
    readonly property bool hasAcceptedAudio: acceptedAudioArtifactId.length > 0
    readonly property int displayDurationMillis: hasCandidateAudio
                                                  ? candidate.audioDurationMillis
                                                  : acceptedDurationMillis
    readonly property int displaySampleRateHz: hasCandidateAudio
                                               ? candidate.audioSampleRateHz
                                               : acceptedSampleRateHz
    readonly property int displayChannels: hasCandidateAudio
                                           ? candidate.audioChannels
                                           : acceptedChannels
    readonly property string displayOriginKey: hasCandidateAudio
                                               ? candidate.audioOriginKey
                                               : acceptedOriginKey
    property alias artifactName: artifactNameField.text

    signal draftSaveRequested(string draftId, string presetAlias,
                                    string presetCatalogRevision, string language,
                                    int speedMilli, bool syntheticDisclosureRequired)
    signal writingRequested()
    signal exportRequested()
    signal synthesizeRequested(string sourceArtifactId, string draftId,
                               string artifactName, string presetAlias,
                               string presetCatalogRevision, string language,
                               int speedMilli, bool syntheticDisclosureRequired)

    readonly property var presetChoices: inferSpeech.presets.map(function(preset) {
        return { key: preset.key, alias: preset.alias, language: preset.language,
                 catalogRevision: preset.catalogRevision, label: workspace.voiceLabel(preset.key) }
    })

    readonly property var languageChoices: [
        { key: "auto", label: qsTr("Automatic · mixed languages") },
        { key: "Chinese", label: qsTr("Chinese") }, { key: "English", label: qsTr("English") },
        { key: "Japanese", label: qsTr("Japanese") }, { key: "Korean", label: qsTr("Korean") },
        { key: "German", label: qsTr("German") }, { key: "French", label: qsTr("French") },
        { key: "Russian", label: qsTr("Russian") }, { key: "Portuguese", label: qsTr("Portuguese") },
        { key: "Spanish", label: qsTr("Spanish") }, { key: "Italian", label: qsTr("Italian") }
    ]

    function chooseLanguage(index) : void {
        if (index < 0 || index >= languageChoices.length || operatorDraftId.length === 0) return
        speedSaveTimer.stop()
        const selected = presetChoices.find(preset => preset.alias === presetAlias)
        if (!selected) return
        draftSaveRequested(operatorDraftId, presetAlias, selected.catalogRevision,
                           languageChoices[index].key, pendingSpeedMilli, true)
    }

    function voiceLabel(key) : string {
        switch (key) {
        case "vivian": return qsTr("Vivian · Bright Mandarin female")
        case "serena": return qsTr("Serena · Warm Mandarin female")
        case "uncle_fu": return qsTr("Uncle Fu · Mature Mandarin male")
        case "dylan": return qsTr("Dylan · Beijing male")
        case "eric": return qsTr("Eric · Sichuan male")
        case "ryan": return qsTr("Ryan · Dynamic English male")
        case "aiden": return qsTr("Aiden · Warm American male")
        case "ono_anna": return qsTr("Ono Anna · Bright Japanese female")
        case "sohee": return qsTr("Sohee · Warm Korean female")
        default: return key
        }
    }

    function chooseVoice(index) : void {
        if (index < 0 || index >= presetChoices.length || operatorDraftId.length === 0) return
        speedSaveTimer.stop()
        const preset = presetChoices[index]
        draftSaveRequested(operatorDraftId, preset.alias, preset.catalogRevision,
                           language.length > 0 ? language : "auto", pendingSpeedMilli, true)
    }

    function synchronizeDraftConfiguration() : void {
        speedSaveTimer.stop()
        pendingSpeedMilli = speedMilli > 0 ? speedMilli : 1000
        speedSlider.value = pendingSpeedMilli
    }

    function persistDraftConfiguration() : void {
        if (operatorDraftId.length === 0 || pendingSpeedMilli === speedMilli) return
        draftSaveRequested(operatorDraftId, presetAlias, presetCatalogRevision,
                           language, pendingSpeedMilli,
                           syntheticDisclosureRequired)
    }

    function requestSynthesis() : void {
        speedSaveTimer.stop()
        synthesizeRequested(sourceArtifactId, operatorDraftId,
                            artifactNameField.text.trim(), presetAlias,
                            presetCatalogRevision, language,
                            pendingSpeedMilli, syntheticDisclosureRequired)
    }

    function formatDuration(milliseconds) : string {
        const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000))
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds
    }

    function originLabel(key) : string {
        if (key === "synthetic_speech") return qsTr("Synthetic speech · disclosed")
        if (key === "recorded_source") return qsTr("Recorded source")
        if (key === "synthetic_sound") return qsTr("Synthetic sound · disclosed")
        if (key === "transformed_audio") return qsTr("Transformed audio")
        return qsTr("Audio origin unavailable")
    }

    function generationError(code) : string {
        switch (code) {
        case "": return ""
        case "invalid_speech_script": return qsTr("Check the script preview and resolve every cue before generating.")
        case "speech_cancelled": return qsTr("Stopped. Generate again to continue the completed segments with the same text and voice.")
        case "speech_format_changed": return qsTr("The speech service changed audio format between segments. Try again.")
        case "speech_audio_too_large": return qsTr("This narration exceeds the 128 MiB audio limit. Split the text into shorter parts.")
        case "speech_text_too_long":
        case "invalid_speech_source": return qsTr("Use nonempty text up to 64 KiB. Split a longer manuscript into separate works.")
        case "override_not_allowed":
        case "policy_violation": return qsTr("Infer has not granted Shape access to this voice. Choose another voice or update Shape's voice access in Infer.")
        case "credential_missing": return qsTr("Configure Shape's Infer access before synthesizing speech.")
        case "generation_busy": return qsTr("Speech synthesis is already running.")
        case "stale_candidate": return qsTr("The accepted source text changed. Try again from the latest revision.")
        case "unsupported_speech_preset": return qsTr("This Infer Runtime does not publish the required voice preset.")
        case "no_candidate": return qsTr("No local speech model currently satisfies this request.")
        case "provider_unavailable":
        case "infer_unavailable": return qsTr("The local speech provider is unavailable.")
        case "infer_policy_violation": return qsTr("Infer returned speech outside Shape's local execution policy.")
        case "invalid_audio_output": return qsTr("Infer returned audio that did not match Shape's WAV contract.")
        default: return qsTr("Speech synthesis could not create a candidate.")
        }
    }

    function playbackError(code) : string {
        if (code === "") return ""
        if (code === "preview_unavailable") return qsTr("The verified audio preview is unavailable.")
        if (code === "preview_invalid") return qsTr("The selected WAV preview is invalid.")
        if (code === "no_audio_preview") return qsTr("Select an audio candidate before playing.")
        return qsTr("Audio playback is unavailable.")
    }

    function refreshPreview() : void {
        if (previewArtifactId.length === 0
                || (!hasCandidateAudio && !hasAcceptedAudio)) {
            audioPreview.clear()
            return
        }
        audioPreview.loadPreview(previewArtifactId, previewCandidateId)
    }

    onPreviewArtifactIdChanged: Qt.callLater(workspace.refreshPreview)
    onPreviewCandidateIdChanged: Qt.callLater(workspace.refreshPreview)
    onOperatorDraftIdChanged: {
        Qt.callLater(workspace.synchronizeDraftConfiguration)
        Qt.callLater(workspace.refreshScriptPreview)
    }
    onSpeedMilliChanged: {
        if (!speedSlider.pressed) Qt.callLater(workspace.synchronizeDraftConfiguration)
    }
    onVisibleChanged: {
        if (!visible && speedSaveTimer.running) {
            speedSaveTimer.stop()
            persistDraftConfiguration()
        }
    }
    Component.onCompleted: {
        if (artifactNameField.text.length === 0) {
            artifactNameField.text = outputName.length > 0 ? outputName : sourceName.length > 0
                                     ? qsTr("%1 narration").arg(sourceName)
                                     : qsTr("Narration")
        }
        synchronizeDraftConfiguration()
        Qt.callLater(workspace.refreshPreview)
    }
    Component.onDestruction: {
        if (speedSaveTimer.running) persistDraftConfiguration()
        workspace.audioPreview.clear()
    }

    SpeechScriptHelpDialog { id: scriptHelp }

    Timer {
        id: speedSaveTimer
        interval: 350
        repeat: false
        onTriggered: workspace.persistDraftConfiguration()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            ShapeButton {
                objectName: "speechReturnToWritingButton"
                text: qsTr("Write")
                quiet: true
                enabled: !workspace.inferSpeech.running
                onClicked: workspace.writingRequested()
            }
            ShapeButton {
                objectName: "speechWorkspaceTitle"
                text: qsTr("Voices and audition")
                selected: true
            }
            ShapeButton {
                objectName: "speechExportButton"
                text: qsTr("Export audio")
                quiet: true
                enabled: workspace.hasCandidateAudio || workspace.hasAcceptedAudio
                onClicked: workspace.exportRequested()
            }
            Item { Layout.fillWidth: true }
            Label {
                text: workspace.hasCandidateAudio ? qsTr("Candidate ready")
                    : workspace.hasAcceptedAudio ? qsTr("Accepted audio") : qsTr("Choose voices for your text")
                color: Theme.muted
                font.pixelSize: 12
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                radius: Theme.radiusLarge
                color: Theme.surface
                border.color: Theme.border

                ScrollView {
                    id: speechColumn0
                    objectName: "speechColumn0"
                    anchors.fill: parent
                    anchors.margins: 18
                    clip: true
                    contentWidth: availableWidth
                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                    ScrollBar.vertical.policy: contentHeight > availableHeight ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
                    ScrollBar.vertical.interactive: true
                    ColumnLayout {
                    width: speechColumn0.availableWidth - 12
                    spacing: 10

                    Text {
                        text: qsTr("ACCEPTED TEXT SOURCE")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: workspace.sourceChanged
                        text: qsTr("The text has changed since this audio was made. Return to writing to create a new recording. This recording is still available.")
                        color: Theme.accent
                        wrapMode: Text.Wrap
                        font.pixelSize: 12
                    }
                    Text {
                        Layout.fillWidth: true
                        text: workspace.sourceName
                        color: Theme.text
                        font.pixelSize: 18
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }
                    ShapeTextEditor {
                        readOnly: true
                        visible: !workspace.scriptMode
                        Layout.fillWidth: true
                        Layout.preferredHeight: 280
                        text: workspace.sourceText.length > 0
                              ? workspace.sourceText
                              : qsTr("The accepted source remains immutable and is read only here.")
                        color: Theme.textSoft
                        font.pixelSize: 15
                        wrapMode: TextEdit.Wrap
                    }
                    SpeechScriptPanel {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.max(280, Math.min(480, speechColumn0.height - 110))
                        visible: workspace.scriptMode
                        enabled: !workspace.inferSpeech.running && !workspace.backend.speechCueImporting
                        backend: workspace.backend
                        draftId: workspace.operatorDraftId
                        optionsJson: workspace.scriptJson
                        preview: workspace.scriptPreview
                        voices: workspace.presetChoices
                    }
                    Text {
                        Layout.fillWidth: true
                        text: workspace.scriptMode ? qsTr("Only spoken lines enter the speech model. Shape inserts pauses and cues.") : qsTr("Your adopted text is used for speech. You can return to writing at any time.")
                        color: Theme.disabled
                        font.pixelSize: 9
                        wrapMode: Text.WordWrap
                    }
                }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                radius: Theme.radiusLarge
                color: Theme.surface
                border.color: workspace.hasCandidateAudio ? Theme.accent : Theme.border

                ScrollView {
                    id: speechColumn1
                    objectName: "speechColumn1"
                    anchors.fill: parent
                    anchors.margins: 18
                    clip: true
                    contentWidth: availableWidth
                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                    ScrollBar.vertical.policy: contentHeight > availableHeight ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
                    ScrollBar.vertical.interactive: true
                    ColumnLayout {
                    width: speechColumn1.availableWidth - 12
                    spacing: 12

                    Text {
                        text: workspace.hasAcceptedAudio ? qsTr("AUDIO REVISION")
                                                        : qsTr("VOICE PRESET")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    ShapeButton {
                        objectName: "speechScriptRulesButton"
                        text: qsTr("Script rules and AI writing prompt")
                        onClicked: scriptHelp.open()
                    }

                    Label {
                        Layout.fillWidth: true
                        text: workspace.scriptMode ? qsTr("Input · narration script") : qsTr("Input · plain text")
                        color: Theme.muted
                        font.pixelSize: 12
                    }
                    ShapeButton {
                        visible: workspace.sourceChanged && !workspace.canGenerate && workspace.operatorDraftId.length > 0
                        text: qsTr("Use latest original")
                        onClicked: workspace.backend.refreshTextInput(workspace.operatorDraftId)
                    }
                    ShapeTextField {
                        id: artifactNameField
                        objectName: "speechArtifactNameField"
                        onEditingFinished: workspace.backend.renameArtifact(workspace.outputArtifactId, text)
                        Layout.fillWidth: true
                        visible: workspace.canGenerate
                        placeholderText: qsTr("Audio artifact name")
                        selectByMouse: true
                    }

                    ShapeComboBox {
                        id: voiceSelector
                        objectName: "speechVoiceSelector"
                        Layout.fillWidth: true
                        visible: workspace.canGenerate && (!workspace.scriptMode || (workspace.scriptPreview.roles || []).length === 0)
                        enabled: !workspace.inferSpeech.running
                        model: workspace.presetChoices
                        textRole: "label"
                        currentIndex: {
                            for (let i = 0; i < workspace.presetChoices.length; ++i)
                                if (workspace.presetChoices[i].alias === workspace.presetAlias) return i
                            return -1
                        }
                        Accessible.name: qsTr("Voice")
                        onActivated: (index) => workspace.chooseVoice(index)
                    }

                    ShapeComboBox {
                        id: languageSelector
                        objectName: "speechLanguageSelector"
                        Layout.fillWidth: true
                        visible: workspace.canGenerate && (!workspace.scriptMode || (workspace.scriptPreview.roles || []).length === 0)
                        enabled: !workspace.inferSpeech.running
                        model: workspace.languageChoices
                        textRole: "label"
                        currentIndex: {
                            for (let i = 0; i < workspace.languageChoices.length; ++i)
                                if (workspace.languageChoices[i].key === workspace.language) return i
                            return 0
                        }
                        Accessible.name: qsTr("Language")
                        onActivated: (index) => workspace.chooseLanguage(index)
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        visible: workspace.canGenerate
                        Text {
                            text: qsTr("Pace")
                            color: Theme.textSoft
                            font.pixelSize: 10
                        }
                        Slider {
                            id: speedSlider
                            objectName: "speechSpeedSlider"
                            Layout.fillWidth: true
                            enabled: !workspace.inferSpeech.running
                            from: 750
                            to: 1250
                            stepSize: 50
                            value: workspace.pendingSpeedMilli
                            snapMode: Slider.SnapAlways
                            onMoved: {
                                workspace.pendingSpeedMilli = Math.round(value)
                                speedSaveTimer.restart()
                            }
                            onPressedChanged: {
                                if (!pressed) {
                                    workspace.pendingSpeedMilli = Math.round(value)
                                    speedSaveTimer.stop()
                                    workspace.persistDraftConfiguration()
                                }
                            }
                        }
                        Text {
                            text: (workspace.pendingSpeedMilli / 1000).toFixed(2) + "×"
                            color: Theme.accent
                            font.pixelSize: 10
                        }
                    }

                    ShapeButton {
                        objectName: "speechGenerateButton"
                        visible: workspace.canGenerate
                        primary: true
                        busy: workspace.inferSpeech.running
                        enabled: !workspace.inferSpeech.running
                                 && (!workspace.scriptMode || workspace.scriptPreview.ready === true)
                                 && !workspace.backend.speechCueImporting
                                 && workspace.credentialConfigured
                                 && workspace.operatorDraftId.length > 0
                                 && workspace.presetAlias.length > 0
                                 && workspace.presetCatalogRevision.length > 0
                                 && workspace.language.length > 0
                                 && workspace.syntheticDisclosureRequired
                                 && artifactNameField.text.trim().length > 0
                        text: qsTr("Create speech candidate")
                        onClicked: workspace.requestSynthesis()
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: !workspace.credentialConfigured && workspace.canGenerate
                        text: qsTr("Configure Infer access in Settings.")
                        color: Theme.muted
                        font.pixelSize: 9
                        wrapMode: Text.WordWrap
                    }

                    Item { Layout.fillHeight: true }

                    ColumnLayout {
                        Layout.fillWidth: true
                        visible: workspace.inferSpeech.running || (workspace.inferSpeech.errorCode.length > 0 && workspace.inferSpeech.completedSegments > 0)
                        spacing: 6
                        ProgressBar {
                            Layout.fillWidth: true
                            from: 0
                            to: Math.max(1, workspace.inferSpeech.totalSegments)
                            value: workspace.inferSpeech.completedSegments
                            indeterminate: workspace.inferSpeech.running && workspace.inferSpeech.totalSegments === 0
                        }
                        Text {
                            Layout.fillWidth: true
                            text: workspace.inferSpeech.cancelling
                                  ? qsTr("Stopping after the current segment…")
                                  : qsTr("Completed %1 of %2 segments").arg(workspace.inferSpeech.completedSegments).arg(workspace.inferSpeech.totalSegments)
                            color: Theme.textSoft
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }
                        ShapeButton {
                            text: qsTr("Stop after this segment")
                            visible: workspace.inferSpeech.running
                            enabled: !workspace.inferSpeech.cancelling
                            onClicked: workspace.inferSpeech.cancel()
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 118
                        visible: workspace.hasCandidateAudio || workspace.hasAcceptedAudio
                        radius: Theme.radiusMedium
                        color: Theme.raised
                        border.color: workspace.hasCandidateAudio ? Theme.accent : Theme.border

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 6
                            RowLayout {
                                Layout.fillWidth: true
                                ShapeButton {
                                    objectName: "audioPreviewToggleButton"
                                    text: workspace.audioPreview.playing ? qsTr("Pause") : qsTr("Play")
                                    enabled: workspace.audioPreview.hasAudio
                                    onClicked: workspace.audioPreview.togglePlayback()
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: workspace.formatDuration(workspace.audioPreview.positionMillis)
                                          + " / " + workspace.formatDuration(
                                              workspace.displayDurationMillis)
                                    color: Theme.textSoft
                                    font.pixelSize: 10
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                from: 0
                                to: Math.max(1, workspace.displayDurationMillis)
                                value: workspace.audioPreview.positionMillis
                                onMoved: workspace.audioPreview.seekTo(value)
                            }
                            Text {
                                Layout.fillWidth: true
                                text: qsTr("%1 Hz · %2 channel(s) · %3")
                                      .arg(workspace.displaySampleRateHz)
                                      .arg(workspace.displayChannels)
                                      .arg(workspace.originLabel(workspace.displayOriginKey))
                                color: Theme.muted
                                font.pixelSize: 9
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: workspace.inferSpeech.errorCode.length > 0
                                 || workspace.audioPreview.errorCode.length > 0
                        text: workspace.inferSpeech.errorCode.length > 0
                              ? workspace.generationError(workspace.inferSpeech.errorCode)
                              : workspace.playbackError(workspace.audioPreview.errorCode)
                        color: Theme.danger
                        font.pixelSize: 9
                        wrapMode: Text.WordWrap
                    }
                }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Long text is split at sentence boundaries and assembled into one WAV. Review the audio before accepting it. Use Export to save the selected audio.")
            color: Theme.muted
            font.pixelSize: 9
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
        }
    }
}

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

    required property InferSpeechController inferSpeech
    required property AudioPreviewController audioPreview
    property string projectPath: ""
    property string operatorDraftId: ""
    property string sourceArtifactId: ""
    property string sourceName: ""
    property string sourceText: ""
    property bool canGenerate: false
    property bool credentialConfigured: false
    property string acceptedAudioArtifactId: ""
    property int acceptedDurationMillis: 0
    property int acceptedSampleRateHz: 0
    property int acceptedChannels: 0
    property string acceptedOriginKey: ""
    property var candidate: null
    property string presetAlias: ""
    property string presetCatalogRevision: ""
    property string language: ""
    property int speedMilli: 0
    property bool syntheticDisclosureRequired: false
    property int pendingSpeedMilli: speedMilli > 0 ? speedMilli : 1000

    readonly property bool hasCandidateAudio: candidate !== null
                                                && candidate.hasAudioPreview
    readonly property string previewArtifactId: hasCandidateAudio
                                                ? sourceArtifactId
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
    signal synthesizeRequested(string sourceArtifactId, string draftId,
                               string artifactName, string presetAlias,
                               string presetCatalogRevision, string language,
                               int speedMilli, bool syntheticDisclosureRequired)

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
    onOperatorDraftIdChanged: Qt.callLater(workspace.synchronizeDraftConfiguration)
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
            artifactNameField.text = sourceName.length > 0
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

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: workspace.hasCandidateAudio ? Theme.accent : Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 9

                Text {
                    objectName: "speechWorkspaceTitle"
                    text: qsTr("SPEECH SYNTHESIS")
                    color: Theme.text
                    font.pixelSize: Theme.fontMeta
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }

                Rectangle {
                    Layout.preferredWidth: inputType.implicitWidth + 16
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.surface
                    border.color: Theme.border
                    Text {
                        id: inputType
                        anchors.centerIn: parent
                        text: "text.document"
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }
                Text { text: "→"; color: Theme.muted }
                Rectangle {
                    Layout.preferredWidth: operatorType.implicitWidth + 16
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.accentSoft
                    border.color: Theme.accent
                    Text {
                        id: operatorType
                        anchors.centerIn: parent
                        text: "audio.speech_synthesize"
                        color: Theme.accent
                        font.pixelSize: 10
                    }
                }
                Text { text: "→"; color: Theme.muted }
                Rectangle {
                    Layout.preferredWidth: outputType.implicitWidth + 16
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.surface
                    border.color: Theme.border
                    Text {
                        id: outputType
                        anchors.centerIn: parent
                        text: "audio.clip"
                        color: Theme.textSoft
                        font.pixelSize: 10
                    }
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: workspace.hasCandidateAudio ? qsTr("Candidate ready")
                                                     : workspace.hasAcceptedAudio
                                                       ? qsTr("Accepted audio")
                                                       : qsTr("Local-only preset")
                    color: workspace.hasCandidateAudio ? Theme.accent : Theme.muted
                    font.pixelSize: 10
                }
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

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 10

                    Text {
                        text: qsTr("ACCEPTED TEXT SOURCE")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }
                    Text {
                        Layout.fillWidth: true
                        text: workspace.sourceName
                        color: Theme.text
                        font.pixelSize: 18
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        text: workspace.sourceText.length > 0
                              ? workspace.sourceText
                              : qsTr("The accepted source remains immutable and is read only here.")
                        color: Theme.textSoft
                        font.pixelSize: 15
                        lineHeight: 1.3
                        wrapMode: Text.WordWrap
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Only the accepted text Revision enters Infer Runtime.")
                        color: Theme.disabled
                        font.pixelSize: 9
                        wrapMode: Text.WordWrap
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

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 12

                    Text {
                        text: workspace.hasAcceptedAudio ? qsTr("AUDIO REVISION")
                                                        : qsTr("VOICE PRESET")
                        color: Theme.muted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    TextField {
                        id: artifactNameField
                        objectName: "speechArtifactNameField"
                        Layout.fillWidth: true
                        visible: workspace.canGenerate
                        placeholderText: qsTr("Audio artifact name")
                        selectByMouse: true
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 64
                        radius: Theme.radiusMedium
                        color: Theme.raised
                        border.color: Theme.border
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 2
                            Text {
                                text: qsTr("Bright female · Mandarin")
                                color: Theme.text
                                font.pixelSize: 13
                                font.weight: Font.Medium
                            }
                            Text {
                                Layout.fillWidth: true
                                text: workspace.presetAlias + " · "
                                      + workspace.presetCatalogRevision
                                color: Theme.muted
                                font.pixelSize: 9
                                elide: Text.ElideMiddle
                            }
                        }
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
                        Layout.fillWidth: true
                        visible: workspace.canGenerate
                        primary: true
                        enabled: !workspace.inferSpeech.running
                                 && workspace.credentialConfigured
                                 && workspace.operatorDraftId.length > 0
                                 && workspace.presetAlias.length > 0
                                 && workspace.presetCatalogRevision.length > 0
                                 && workspace.language.length > 0
                                 && workspace.syntheticDisclosureRequired
                                 && artifactNameField.text.trim().length > 0
                        text: workspace.inferSpeech.running
                              ? qsTr("Synthesizing locally…")
                              : qsTr("Create speech candidate")
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

        Text {
            Layout.fillWidth: true
            text: qsTr("Synthetic speech is always disclosed. Candidate audio remains transient until explicit acceptance.")
            color: Theme.muted
            font.pixelSize: 9
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
        }
    }
}

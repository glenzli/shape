pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: sound
    objectName: "soundGenerationWorkspace"
    required property DesktopBackend backend
    required property InferSoundController inferSound
    required property AudioPreviewController audioPreview
    property var draft: null
    property string artifactId: ""
    property string candidateId: ""
    property string acceptedRevisionId: ""
    property bool credentialConfigured: false
    property var history: null
    property bool showDetails: false
    readonly property bool editable: draft !== null && !inferSound.running
    readonly property bool validFields: prompt.text.trim().length > 0 && /^[0-9]+$/.test(duration.text)
        && Number(duration.text) >= 1 && Number(duration.text) <= 30
        && /^[0-9]+$/.test(seed.text) && Number(seed.text) <= 4294967295
    signal exportRequested()
    color: Theme.surface
    radius: Theme.radiusMedium
    border.color: Theme.border

    function restoreDraft(): void {
        let p = null
        try { if (draft !== null) p = JSON.parse(draft.soundGenerationJson) } catch (e) {}
        if (p === null && history !== null) p = history.operation
        if (p === null) return
        prompt.text = p.prompt
        duration.text = String(p.duration_seconds)
        seed.text = String(p.seed)
        model.currentIndex = p.kind === "short_music" ? 1 : 0
    }
    function saveDraft(): bool {
        if (draft === null || inferSound.running || !validFields) return false
        // Read displayed text at the click boundary, including uncommitted numeric edits.
        return backend.updateSoundDraft(draft.id, prompt.text,
            model.currentIndex === 1 ? "short_music" : "sound_effect",
            Number(duration.text), seed.text)
    }
    function refresh(): void {
        history = null
        if (artifactId.length > 0 && (candidateId.length > 0 || acceptedRevisionId.length > 0)) {
            try { history = JSON.parse(backend.soundDetails(artifactId, candidateId)) } catch (e) {}
            audioPreview.loadPreview(artifactId, candidateId)
        } else audioPreview.clear()
        if (draft === null) restoreDraft()
    }
    function statusText(): string {
        if (draft === null && acceptedRevisionId.length > 0)
            return qsTr("This version is accepted. You can listen, export WAV, or inspect its generation details.")
        if (inferSound.running) return inferSound.stage === 1
            ? qsTr("Preparing your description locally…") : qsTr("Generating audio locally…")
        if (inferSound.errorCode === "generation_cancelled") return qsTr("Stopped. Your description is ready to try again.")
        if (inferSound.errorCode.length > 0) return qsTr("Generation could not finish. Check Infer access and try again. (%1)").arg(inferSound.errorCode)
        return qsTr("Chinese descriptions are translated locally. Listen to a version, then choose Use this version to keep it.")
    }
    onDraftChanged: restoreDraft()
    onCandidateIdChanged: Qt.callLater(refresh)
    onAcceptedRevisionIdChanged: Qt.callLater(refresh)
    Component.onCompleted: { refresh(); restoreDraft() }
    Component.onDestruction: { saveDraft(); audioPreview.clear() }

    ScrollView {
        anchors.fill: parent
        anchors.margins: 20
        contentWidth: availableWidth
        ColumnLayout {
            width: parent.width
            spacing: 12
            Label { text: qsTr("Sound effects and short music"); color: Theme.text; font.pixelSize: 20; font.weight: Font.DemiBold }
            Label { Layout.fillWidth: true; text: qsTr("Describe the sounds, instruments, rhythm and atmosphere you want."); wrapMode: Text.WordWrap; color: Theme.muted }
            ShapeTextEditor {
                id: prompt
                objectName: "soundPromptField"
                Layout.fillWidth: true
                Layout.preferredHeight: 112
                enabled: sound.editable
                placeholderText: qsTr("Rain on a window, distant thunder, no speech or music…")
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                onActiveFocusChanged: if (!activeFocus) sound.saveDraft()
            }
            RowLayout {
                Layout.fillWidth: true
                ColumnLayout {
                    Layout.fillWidth: true
                    Label { text: qsTr("Model"); color: Theme.muted }
                    ShapeComboBox {
                        id: model
                        objectName: "soundModelPicker"
                        Layout.fillWidth: true
                        enabled: sound.editable
                        model: [qsTr("Stable Audio 3 Small · Sound effects"), qsTr("Stable Audio 3 Small · Music")]
                        onActivated: sound.saveDraft()
                    }
                }
                ColumnLayout {
                    Label { text: qsTr("Seconds · 1–30"); color: Theme.muted }
                    ShapeTextField { id: duration; objectName: "soundDurationField"; Layout.preferredWidth: 110; text: "5"; enabled: sound.editable; selectByMouse: true; onEditingFinished: sound.saveDraft() }
                }
                ColumnLayout {
                    Label { text: qsTr("Seed"); color: Theme.muted }
                    ShapeTextField { id: seed; objectName: "soundSeedField"; Layout.preferredWidth: 145; text: "42"; enabled: sound.editable; selectByMouse: true; onEditingFinished: sound.saveDraft() }
                }
            }
            RowLayout {
                ShapeButton {
                    objectName: "soundGenerateButton"
                    text: sound.inferSound.errorCode.length > 0 ? qsTr("Try again") : qsTr("Generate a version")
                    primary: true
                    visible: sound.draft !== null
                    enabled: sound.editable && sound.validFields && sound.credentialConfigured
                    onClicked: {
                        const artifact = sound.artifactId
                        const draftId = sound.draft.id
                        if (sound.saveDraft()) sound.inferSound.generate(sound.backend.bundlePath, artifact, draftId)
                    }
                }
                ShapeButton { objectName: "soundStopButton"; text: qsTr("Stop"); visible: sound.inferSound.running; onClicked: sound.inferSound.stop() }
                Item { Layout.fillWidth: true }
                Label { text: qsTr("44.1 kHz · Stereo · WAV"); color: Theme.muted; font.pixelSize: 11 }
            }
            Label { Layout.fillWidth: true; text: sound.statusText(); color: sound.inferSound.errorCode.length > 0 ? Theme.danger : Theme.muted; wrapMode: Text.WordWrap }
            Label { Layout.fillWidth: true; visible: sound.backend.lastError.length > 0; text: sound.backend.lastError; color: Theme.danger; wrapMode: Text.WordWrap }
            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }
            RowLayout {
                ShapeButton { objectName: "soundPlayButton"; text: sound.audioPreview.playing ? qsTr("Pause") : qsTr("Play"); enabled: sound.audioPreview.hasAudio; onClicked: sound.audioPreview.togglePlayback() }
                Slider { objectName: "soundPlaybackSlider"; Layout.fillWidth: true; from: 0; to: Math.max(1, sound.audioPreview.durationMillis); value: sound.audioPreview.positionMillis; enabled: sound.audioPreview.hasAudio; onMoved: sound.audioPreview.seekTo(value) }
                Label { text: (sound.audioPreview.positionMillis / 1000).toFixed(1) + " / " + (sound.audioPreview.durationMillis / 1000).toFixed(1) + " s"; color: Theme.muted }
                ShapeButton { objectName: "soundExportButton"; text: qsTr("Export WAV"); enabled: sound.audioPreview.hasAudio; onClicked: sound.exportRequested() }
            }
            Label { visible: sound.audioPreview.errorCode.length > 0; text: qsTr("Audio playback is unavailable."); color: Theme.danger }
            ShapeButton { objectName: "soundDetailsButton"; text: sound.showDetails ? qsTr("Hide generation details") : qsTr("Generation details"); visible: sound.history !== null; quiet: true; onClicked: sound.showDetails = !sound.showDetails }
            ColumnLayout {
                Layout.fillWidth: true
                visible: sound.showDetails && sound.history !== null
                property var provenance: sound.history !== null ? sound.history.provenance : null
                property var prepared: provenance !== null ? provenance.sound_prompt : null
                Label { text: qsTr("Original description"); color: Theme.muted }
                Label { Layout.fillWidth: true; text: parent.prepared !== null ? parent.prepared.original_prompt : ""; wrapMode: Text.WordWrap; color: Theme.text }
                Label { text: qsTr("Description sent to the audio model"); color: Theme.muted }
                Label { Layout.fillWidth: true; text: parent.prepared !== null ? parent.prepared.effective_prompt : ""; wrapMode: Text.WordWrap; color: Theme.text }
                Label {
                    Layout.fillWidth: true
                    text: parent.prepared !== null ? qsTr("Preparation: %1 · %2 ms").arg(parent.prepared.rules_revision).arg(parent.prepared.preparation_elapsed_ms) : ""
                    wrapMode: Text.WrapAnywhere; color: Theme.muted; font.pixelSize: 10
                }
                Label { Layout.fillWidth: true; text: parent.prepared !== null && parent.prepared.text_job !== null ? qsTr("Text task: %1 · %2").arg(parent.prepared.text_job.id).arg(parent.prepared.text_job.physical_model) : qsTr("English description used unchanged"); wrapMode: Text.WrapAnywhere; color: Theme.muted; font.pixelSize: 10 }
                Label { Layout.fillWidth: true; text: sound.history !== null ? qsTr("Sound task: %1 · Seed %2 · %3 s").arg(sound.history.job_id).arg(sound.history.operation.seed).arg(sound.history.operation.duration_seconds) : ""; wrapMode: Text.WrapAnywhere; color: Theme.muted; font.pixelSize: 10 }
            }
        }
    }
}

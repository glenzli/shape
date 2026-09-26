pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs
import Shape.Desktop

Item {
    id: exportDialog
    required property AudioExportController controller
    property string artifactId: ""
    property string candidateId: ""

    function openForAudio(sourceId, versionId) : void {
        artifactId = sourceId
        candidateId = versionId
        saveDialog.open()
    }

    FileDialog {
        id: saveDialog
        objectName: "audioExportFileDialog"
        title: qsTr("Export audio")
        fileMode: FileDialog.SaveFile
        nameFilters: [qsTr("WAV audio (*.wav)")]
        defaultSuffix: "wav"
        onAccepted: exportDialog.controller.exportAudio(exportDialog.artifactId, exportDialog.candidateId, selectedFile)
    }

    ShapeMessageDialog {
        id: resultDialog
        objectName: "audioExportResultDialog"
        title: qsTr("Export audio")
    }

    Connections {
        target: exportDialog.controller
        function onFinished(success, destination, errorCode) {
            resultDialog.text = success ? qsTr("Audio exported") : qsTr("Audio export failed")
            if (success) resultDialog.informativeText = destination
            else if (errorCode === "export_inside_project") resultDialog.informativeText = qsTr("Choose a location outside the Shape project.")
            else if (errorCode === "export_audio_unavailable") resultDialog.informativeText = qsTr("Select the audio again and retry.")
            else resultDialog.informativeText = qsTr("Choose a writable location for the WAV file and retry. No incomplete file was saved.")
            resultDialog.open()
        }
    }
}

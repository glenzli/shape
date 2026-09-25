pragma ComponentBehavior: Bound

//! One controlled Agent file task: exact accepted input, authored instruction,
//! transient output review. Candidate acceptance remains in the shared footer.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Shape.Desktop

Item {
    id: task
    objectName: "agentFileTaskWorkspace"

    required property string projectPath
    required property string artifactId
    required property string revisionId
    required property string sourceName
    required property string sourceText
    required property var selectedCandidate
    required property DesktopBackend backend
    required property InferAgentController inferAgent
    required property bool credentialConfigured

    readonly property bool hasAgentResult: selectedCandidate !== null
                                           && selectedCandidate.contextArtifactId === artifactId
                                           && selectedCandidate.artifactId !== artifactId
                                           && selectedCandidate.kindKey === "text_document"
    readonly property string resultText: hasAgentResult
                                         ? backend.textAuthoringContent(artifactId,
                                                                        String(selectedCandidate.id)) : ""

    function errorMessage(code) : string {
        if (code === "") return ""
        if (code === "credential_missing" || code === "credential_unavailable")
            return qsTr("Configure Infer access in Settings before running this task.")
        if (code === "agent_task_forbidden")
            return qsTr("Agent file tasks are not enabled for Shape in Infer.")
        if (code === "agent_task_unavailable" || code === "provider_unavailable")
            return qsTr("The Agent provider is unavailable. Try again later.")
        if (code === "stale_candidate")
            return qsTr("The source changed while the task was running. Review the current source and run again.")
        if (code === "infer_invalid_response" || code === "invalid_text_output")
            return qsTr("The Agent result failed validation and was not added as a candidate.")
        if (code === "infer_unavailable")
            return qsTr("Infer is unavailable. Check its status and try again.")
        if (code === "infer_outcome_unknown")
            return qsTr("Infer did not confirm the task outcome. Check its Jobs before starting another task.")
        if (code === "invalid_prompt")
            return qsTr("Describe the change you want the Agent to make.")
        return qsTr("The Agent task failed. No project content was changed.")
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        Text {
            Layout.fillWidth: true
            text: qsTr("Infer sends a copy of this accepted source to the authorized cloud Agent. Review its result before adding it to the project.")
            color: Theme.muted
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            ShapeTextEditor {
                id: instructionEditor
                objectName: "agentFileInstruction"
                Layout.fillWidth: true
                Layout.preferredHeight: 108
                placeholderText: qsTr("Describe the file change, for example: make this animation loop smoothly without changing its colors.")
                wrapMode: TextEdit.Wrap
                enabled: !task.inferAgent.running
            }

            ShapeButton {
                objectName: "runAgentFileTaskButton"
                Layout.alignment: Qt.AlignBottom
                text: task.inferAgent.running ? qsTr("Running…") : qsTr("Run Agent")
                primary: true
                enabled: !task.inferAgent.running && task.credentialConfigured
                         && instructionEditor.text.trim().length > 0
                onClicked: task.inferAgent.generate(task.projectPath, task.artifactId,
                                                    task.revisionId, instructionEditor.text)
            }
        }

        Text {
            visible: !task.credentialConfigured
            Layout.fillWidth: true
            text: qsTr("Configure Infer access in Settings before running this task.")
            color: Theme.muted
            font.pixelSize: 11
        }
        Text {
            visible: task.inferAgent.running
            Layout.fillWidth: true
            text: qsTr("The Agent is working on a private copy. The accepted source stays available.")
            color: Theme.muted
            font.pixelSize: 11
        }
        Text {
            visible: task.inferAgent.errorCode.length > 0
            Layout.fillWidth: true
            text: task.errorMessage(task.inferAgent.errorCode)
            color: Theme.danger
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 6
                Text {
                    text: qsTr("Accepted source · %1").arg(task.sourceName)
                    color: Theme.textSoft
                    font.pixelSize: 11
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                ShapeTextEditor {
                    objectName: "agentFileSourceText"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    readOnly: true
                    text: task.sourceText
                    font.family: "monospace"
                    wrapMode: TextEdit.Wrap
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 6
                Text {
                    text: task.hasAgentResult
                          ? qsTr("Candidate · %1").arg(task.selectedCandidate.artifactName)
                          : qsTr("Agent result")
                    color: Theme.textSoft
                    font.pixelSize: 11
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                ShapeTextEditor {
                    objectName: "agentFileResultText"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    readOnly: true
                    text: task.hasAgentResult ? task.resultText : ""
                    placeholderText: qsTr("Run the task to create a reviewable candidate.")
                    font.family: "monospace"
                    wrapMode: TextEdit.Wrap
                }
            }
        }

        Text {
            visible: task.hasAgentResult
            Layout.fillWidth: true
            text: qsTr("Use the candidate controls below to accept or discard this result. Accept creates a separate work and keeps the source unchanged.")
            color: Theme.muted
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }
    }
}

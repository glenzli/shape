//! Read-only presentation of the selected artifact identity and material contract.

import QtQuick
import QtQuick.Layouts
import Shape.Desktop

Rectangle {
    id: details

    property string artifactName: ""
    property string artifactKind: ""
    property string artifactId: ""
    property bool hasAcceptedRevision: false
    property bool hasCandidate: false
    property string mediaType: ""
    property double byteLength: 0
    property string contentDigest: ""

    function formattedBytes() : string {
        if (byteLength <= 0) {
            return qsTr("No materialized content")
        }
        if (byteLength < 1024) {
            return qsTr("%1 bytes").arg(byteLength)
        }
        return qsTr("%1 KB").arg((byteLength / 1024).toFixed(1))
    }

    radius: Theme.radiusLarge
    color: Theme.surface
    border.color: Theme.border

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        Text {
            text: qsTr("ARTIFACT DETAILS")
            color: Theme.muted
            font.pixelSize: Theme.fontMeta
            font.weight: Font.DemiBold
            font.letterSpacing: 0.7
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 94
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 5

                Text {
                    Layout.fillWidth: true
                    text: details.artifactName.length > 0 ? details.artifactName
                                                          : qsTr("No artifact selected")
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: details.artifactKind
                    color: Theme.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }

                Item { Layout.fillHeight: true }

                Text {
                    text: details.hasCandidate ? qsTr("Candidate awaiting review")
                                               : details.hasAcceptedRevision
                                                 ? qsTr("Accepted head · verified")
                                                 : qsTr("No accepted head")
                    color: details.hasCandidate ? Theme.accent
                                                : details.hasAcceptedRevision
                                                  ? Theme.success : Theme.muted
                    font.pixelSize: 10
                }
            }
        }

        Text {
            text: qsTr("IDENTITY")
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.5
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 58
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            Text {
                anchors.fill: parent
                anchors.margins: 12
                text: details.artifactId
                color: Theme.textSoft
                font.pixelSize: 10
                elide: Text.ElideMiddle
                verticalAlignment: Text.AlignVCenter
            }
        }

        Text {
            text: qsTr("MATERIAL")
            color: Theme.muted
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.5
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 112
            radius: Theme.radiusMedium
            color: Theme.raised
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 6

                Text {
                    Layout.fillWidth: true
                    text: details.mediaType.length > 0 ? details.mediaType : qsTr("No media type")
                    color: Theme.text
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }

                Text {
                    text: details.formattedBytes()
                    color: Theme.muted
                    font.pixelSize: 10
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                Text {
                    Layout.fillWidth: true
                    text: details.contentDigest.length > 0 ? details.contentDigest
                                                          : qsTr("No content digest")
                    color: Theme.textSoft
                    font.pixelSize: 9
                    elide: Text.ElideMiddle
                }
            }
        }

        Item { Layout.fillHeight: true }

        Text {
            Layout.fillWidth: true
            text: qsTr("Artifact identity stays stable while accepted revisions advance explicitly.")
            color: Theme.muted
            font.pixelSize: 9
            wrapMode: Text.WordWrap
            lineHeight: 1.3
        }
    }
}

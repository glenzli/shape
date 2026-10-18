pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import Shape.Desktop
import "CardListImport.js" as CardListImport

ShapeDialog {
    id: root
    objectName: "cardKitWorkspace"
    required property DesktopBackend backend
    required property CardKitController controller
    required property CardPrintController cardPrint
    required property InferImageController inferImage
    required property InferTextController inferText
    required property MaterialEditController materialEdit
    required property AudioPreviewController audioPreview
    required property UiPreferences uiPreferences
    property var composition: ({cards: []})
    property string selectedCardId: ""
    property bool dirty: false
    property string sessionProject: ""
    property string sessionPath: ""
    property string localError: ""
    property string artworkCardId: ""
    property string artworkProject: ""
    property string artworkPath: ""
    property string artworkKitId: ""
    property string artworkArtifactId: ""
    property string artworkDraftId: ""
    property string artworkCandidateId: ""
    property var artworkOriginal: null
    property var editingStage: ({})
    property bool editingArtwork: false
    property bool generationStarted: false
    readonly property var cards: composition.cards || []
    readonly property var currentCard: cards.find(c => c.id === selectedCardId) || null
    readonly property var images: [{name: qsTr("Choose accepted artwork…"), id: ""}].concat(
        backend.artifacts.filter(a => a.kindKey === "image_raster" && a.hasAcceptedRevision))
    readonly property var savedKits: {
        const result = [{name: qsTr("Continue saved cards…"), id: ""}]
        const seen = {}
        for (const item of controller.drafts) {
            result.push({name: item.composition.name, id: item.artifact_id})
            seen[item.artifact_id] = true
        }
        for (const item of backend.artifacts)
            if (item.kindKey === "card_kit" && item.hasAcceptedRevision && !seen[item.id])
                result.push({name: item.name, id: item.id})
        return result
    }
    readonly property var artworkCandidate: backend.candidates.find(c =>
        c.id === artworkCandidateId && c.artifactId === artworkArtifactId) || null
    readonly property bool artworkTargetExists: artworkProject === backend.projectId
        && artworkPath === backend.bundlePath
        && artworkKitId === (controller.draft.artifact_id || "")
        && cards.some(c => c.id === artworkCardId)
    readonly property bool artworkRunning: inferImage.running
        && generationStarted && artworkTargetExists && !editingArtwork
    readonly property bool artworkStageCurrent: !editingArtwork || (artworkOriginal !== null
        && backend.artifacts.some(a => a.id === artworkOriginal.artifact_id && a.acceptedRevisionId === artworkOriginal.revision_id)
        && backend.operatorDrafts.some(d => d.id === artworkDraftId && d.contextArtifactId === artworkArtifactId
            && d.operatorTypeKey === "image.external_edit" && d.inputArtifactId === artworkOriginal.artifact_id
            && d.inputRevisionId === artworkOriginal.revision_id))
    property string artworkCandidateSource: ""
    property bool listApplied: false
    property int cardIdSequence: 1
    readonly property var listPreview: CardListImport.parse(listText.text, composition)
    signal inferAccessSetupRequested()

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: parent ? parent.width - 28 : 1260
    height: parent ? parent.height - 40 : 820
    modal: true
    title: qsTr("Card materials")
    closePolicy: Popup.NoAutoClose

    function copy(value) { return JSON.parse(JSON.stringify(value)) }
    function pin(item) { return {artifact_id: item.id, revision_id: item.acceptedRevisionId} }
    function synchronize() {
        composition = copy(controller.draft.composition || {cards: []})
        if (!cards.some(c => c.id === selectedCardId)) selectedCardId = cards.length ? cards[0].id : ""
        dirty = false
    }
    function changed(next) {
        composition = next; dirty = true; localError = ""
        controller.invalidateReview(); saveTimer.restart()
    }
    function setField(key, value) {
        if (composition[key] === value) return
        const next = copy(composition); next[key] = value; changed(next)
    }
    function setCardField(key, value) {
        if (!currentCard || currentCard[key] === value) return
        const next = copy(composition), index = next.cards.findIndex(c => c.id === selectedCardId)
        next.cards[index][key] = value
        if (key === "rules") next.cards[index].text_material = null
        changed(next)
    }
    function flush() {
        saveTimer.stop()
        if (!dirty) return true
        if (!controller.save(composition)) return false
        dirty = false; return true
    }
    function openWorkspace(artifactId) {
        if (sessionProject !== backend.projectId || sessionPath !== backend.bundlePath) {
            sessionProject = backend.projectId; sessionPath = backend.bundlePath; dirty = false; composition = ({cards: []})
            selectedCardId = ""; clearArtwork()
        }
        if (!flush()) return
        controller.refresh()
        if (artifactId && artifactId.length) {
            if (!controller.load(artifactId)) return
        } else if (!controller.draft.artifact_id) {
            if (savedKits.length > 1) { if (!controller.load(savedKits[1].id)) return }
            else if (!controller.beginNew()) return
        }
        synchronize(); resumeArtworkEdit(); controller.showCard(selectedCardId); open()
    }
    function newKit() {
        if (flush() && controller.beginNew()) { clearArtwork(); synchronize() }
    }
    function reloadSavedDraft() {
        saveTimer.stop()
        if (controller.load(controller.draft.artifact_id)) { clearArtwork(); synchronize(); resumeArtworkEdit(); controller.showCard(selectedCardId) }
    }
    function selectKit(artifactId) {
        if (artifactId && flush() && controller.load(artifactId)) { clearArtwork(); synchronize(); resumeArtworkEdit(); controller.showCard(selectedCardId) }
    }
    function addCard() {
        const next = copy(composition)
        const id = nextCardId(new Set(next.cards.map(c => c.id)))
        next.cards.push({id: id, title: "", card_type: "", cost: "", rules: "", stats: "", flavor: "", art_prompt: "", quantity: 1, artwork: null, text_material: null})
        changed(next); selectedCardId = id
    }
    function nextCardId(ids) {
        let id
        do { id = "card-" + Date.now().toString(36) + "-" + (cardIdSequence++).toString(36) } while (ids.has(id))
        return id
    }
    function openCardList() {
        if (!flush()) return
        listApplied = false; listText.text = ""; listDialog.open(); listText.focusEditor()
    }
    function cardListError() {
        const result = listPreview
        switch (result.error) {
        case "empty": return qsTr("Paste at least one card title to begin.")
        case "too_large": case "composition_size": return qsTr("This list is too large for these cards. Add a smaller group.")
        case "too_many_cards": return qsTr("A set can contain up to 64 cards. Add fewer rows or start a new set.")
        case "columns": return qsTr("Row %1 has more than three columns. Use title, rules and artwork description.").arg(result.line)
        case "title": return qsTr("Row %1 needs a card title.").arg(result.line)
        case "field_length": return qsTr("Row %1 has too much text. Shorten its title, rules or artwork description.").arg(result.line)
        case "quote": return qsTr("Check the quotation marks around row %1. Copying the cells again may help.").arg(result.line)
        case "invalid_text": return qsTr("Row %1 contains unsupported text. Remove hidden control characters and try again.").arg(result.line)
        default: return ""
        }
    }
    function appendCardList() {
        const result = CardListImport.parse(listText.text, composition)
        if (listApplied || result.error.length || !result.rows.length) return false
        const next = copy(composition)
        next.cards = copy(CardListImport.effectiveCards(composition))
        const ids = new Set(next.cards.map(c => c.id))
        for (const row of result.rows) {
            const id = nextCardId(ids)
            ids.add(id)
            next.cards.push({id: id, title: row.title, rules: row.rules, art_prompt: row.art_prompt,
                card_type: "", cost: "", stats: "", flavor: "", quantity: 1, artwork: null, text_material: null})
        }
        listApplied = true; changed(next); selectedCardId = next.cards[next.cards.length - result.rows.length].id
        flush(); listDialog.close(); return true
    }
    function moveCard(delta) {
        const next = copy(composition), index = next.cards.findIndex(c => c.id === selectedCardId), to = index + delta
        if (index < 0 || to < 0 || to >= next.cards.length) return
        next.cards.splice(to, 0, next.cards.splice(index, 1)[0]); changed(next)
    }
    function removeCard() {
        if (cards.length < 2) return
        const next = copy(composition), index = next.cards.findIndex(c => c.id === selectedCardId)
        next.cards.splice(index, 1); changed(next)
        selectedCardId = next.cards[Math.min(index, next.cards.length - 1)].id
    }
    function cardState(card) {
        if (!card) return ""
        if (artworkTargetExists && editingArtwork && card.id === artworkCardId)
            return artworkCandidate ? qsTr("Review returned artwork") : qsTr("Artwork edit in progress")
        if (!(card.title || "").trim().length || (!(card.rules || "").trim().length && !card.text_material)) return qsTr("Needs text")
        if (!card.artwork) return qsTr("Needs artwork")
        if ((controller.preview.overflow_card_ids || []).indexOf(card.id) >= 0) return qsTr("Shorten text")
        if (controller.canExport) return qsTr("Ready")
        if (controller.hasReview) return qsTr("Checked")
        return qsTr("Ready to check")
    }
    function artworkIndex() {
        if (!currentCard || !currentCard.artwork) return 0
        return Math.max(0, images.findIndex(a => a.id === currentCard.artwork.artifact_id))
    }
    function bindArtwork(cardId, source) {
        if (sessionProject !== backend.projectId || sessionPath !== backend.bundlePath) return false
        const next = copy(composition), index = next.cards.findIndex(c => c.id === cardId)
        if (index < 0) return false
        next.cards[index].artwork = source; changed(next); return flush()
    }
    function chooseArtwork(index) {
        if (!currentCard || index < 1 || index >= images.length) return
        bindArtwork(selectedCardId, pin(images[index]))
    }
    function clearArtwork() {
        artworkCardId = ""; artworkProject = ""; artworkPath = ""; artworkKitId = ""; artworkArtifactId = ""
        artworkDraftId = ""; artworkCandidateId = ""; artworkCandidateSource = ""
        artworkOriginal = null; editingStage = ({}); editingArtwork = false; generationStarted = false
    }
    function openLargePreview() {
        if (!controller.previewSource.length) return false
        largePreview.svgSource = controller.previewSource
        largePreview.canvasWidth = composition.canvas_width || 750
        largePreview.canvasHeight = composition.canvas_height || 1050
        largePreview.cardTitle = currentCard ? currentCard.title : ""
        largePreview.actualSize = true
        largePreview.open(); return true
    }
    function beginImageImport() {
        if (!currentCard || !flush()) return
        imageImport.cardId = selectedCardId; imageImport.projectId = backend.projectId
        imageImport.kitId = controller.draft.artifact_id; imageImport.projectPath = backend.bundlePath; imageImport.open()
    }
    function importArtwork(file, cardId, projectId, kitId, projectPath) {
        if (projectId !== backend.projectId || projectPath !== backend.bundlePath
            || kitId !== controller.draft.artifact_id || !cards.some(c => c.id === cardId)) return false
        const previous = new Set(backend.artifacts.map(a => a.id))
        if (!backend.importRaster(file)) return false
        const added = backend.artifacts.filter(a => !previous.has(a.id)
            && a.kindKey === "image_raster" && a.hasAcceptedRevision)
        if (added.length === 1) return bindArtwork(cardId, pin(added[0]))
        localError = qsTr("The image was imported. Choose it from accepted artwork to add it to this card.")
        return false
    }
    function openArtworkGeneration() {
        if (!currentCard || !flush()) return
        if (artworkTargetExists && artworkCardId === selectedCardId && !editingArtwork && artworkArtifactId.length) {
            generationDialog.open(); return
        }
        if (inferImage.running) { localError = qsTr("Finish or stop the current image run before starting another."); return }
        clearArtwork(); artworkCardId = selectedCardId; artworkProject = backend.projectId; artworkPath = backend.bundlePath; artworkKitId = controller.draft.artifact_id
        generationPrompt.text = [composition.brief, composition.style, currentCard.title, currentCard.art_prompt,
            qsTr("Landscape card illustration. Keep lettering and rules outside the artwork.")].filter(t => t && t.trim().length).join("\n\n")
        generationDialog.open()
    }
    function generateArtwork() {
        if (!artworkTargetExists || inferImage.running || !generationPrompt.text.trim().length) return false
        if (!artworkArtifactId.length) {
            const card = cards.find(c => c.id === artworkCardId)
            const draft = backend.beginAiImageDraft(card.title || qsTr("Card artwork"), generationPrompt.text, 1536, 1024)
            if (!draft.id || !draft.contextArtifactId) { localError = backend.lastError; return false }
            artworkArtifactId = draft.contextArtifactId; artworkDraftId = draft.id
        }
        if (!backend.updateAiImageDraft(artworkDraftId, generationPrompt.text, 1536, 1024, 1)) return false
        generationStarted = true
        inferImage.generate(backend.bundlePath, artworkArtifactId, artworkDraftId, modelPicker.effectiveModelKey, modelPicker.effectiveEffortKey, 1)
        return true
    }
    function beginArtworkEdit() {
        if (!currentCard || !currentCard.artwork || !flush()) return false
        const image = backend.artifacts.find(a => a.id === currentCard.artwork.artifact_id)
        if (!image || image.acceptedRevisionId !== currentCard.artwork.revision_id) {
            localError = qsTr("Choose the current accepted artwork explicitly before editing it."); return false
        }
        if (!editingArtwork || artworkCardId !== selectedCardId || !artworkTargetExists
            || JSON.stringify(artworkOriginal) !== JSON.stringify(currentCard.artwork)) {
            if (!restoreArtworkEdit(selectedCardId)) {
                const id = backend.beginOperatorDraft(image.id, "image.external_edit")
                const draft = backend.operatorDrafts.find(d => d.id === id)
                if (!draft || draft.inputRevisionId !== image.acceptedRevisionId) return false
                clearArtwork(); artworkCardId = selectedCardId; artworkProject = backend.projectId; artworkPath = backend.bundlePath; artworkKitId = controller.draft.artifact_id
                artworkOriginal = copy(currentCard.artwork); artworkArtifactId = draft.contextArtifactId
                artworkDraftId = id; editingStage = draft; editingArtwork = true
                if (!controller.rememberArtworkEdit(artworkCardId, artworkOriginal, draft))
                    localError = qsTr("The working copy is open. Its return location could not be saved for reopening Shape.")
            }
        }
        materialEdit.restoreShadowSession(backend.bundlePath, artworkArtifactId)
        refinementDialog.open(); return true
    }
    function restoreArtworkEdit(cardId) {
        const record = controller.recoverArtworkEdit(cardId)
        if (!record.stage || !record.stage.id) return false
        clearArtwork(); artworkCardId = cardId; artworkProject = backend.projectId; artworkPath = backend.bundlePath; artworkKitId = controller.draft.artifact_id
        artworkOriginal = copy(record.source); artworkArtifactId = record.stage.contextArtifactId
        artworkDraftId = record.stage.id; editingStage = record.stage; editingArtwork = true
        const candidate = backend.candidates.find(c => c.artifactId === artworkArtifactId)
        artworkCandidateId = candidate ? candidate.id : ""
        materialEdit.restoreShadowSession(backend.bundlePath, artworkArtifactId)
        return true
    }
    function resumeArtworkEdit() {
        for (const card of cards) {
            if (restoreArtworkEdit(card.id)) { selectedCardId = card.id; return true }
        }
        return false
    }
    function useArtwork() {
        if (!artworkTargetExists || !artworkStageCurrent || !artworkCandidate || inferImage.running || materialEdit.running || materialEdit.shadowActive) return false
        const target = cards.find(c => c.id === artworkCardId)
        if (editingArtwork && JSON.stringify(target.artwork) !== JSON.stringify(artworkOriginal)) return false
        if (!backend.acceptCandidate(artworkCandidate.id)) return false
        const accepted = backend.artifacts.find(a => a.id === artworkArtifactId && a.hasAcceptedRevision)
        if (!accepted || !bindArtwork(artworkCardId, pin(accepted))) return false
        if (editingArtwork) controller.forgetArtworkEdit(artworkCardId)
        selectedCardId = artworkCardId; generationDialog.close(); refinementDialog.close(); clearArtwork(); return true
    }
    function updateArtworkPreview() {
        if (artworkTargetExists && artworkCandidate && backend.prepareImagePreviews(artworkArtifactId, artworkCandidate.id))
            artworkCandidateSource = backend.candidateImageSource
        else artworkCandidateSource = ""
    }
    onArtworkCandidateChanged: updateArtworkPreview()
    onSelectedCardIdChanged: if (!dirty && selectedCardId.length) controller.showCard(selectedCardId)
    Connections {
        target: root.controller
        function onDocumentChanged() { root.synchronize() }
    }
    Connections {
        target: root.backend
        function onProjectChanged() {
            if (root.sessionProject !== root.backend.projectId || root.sessionPath !== root.backend.bundlePath) {
                saveTimer.stop(); root.dirty = false; root.close()
                generationDialog.close(); refinementDialog.close(); root.clearArtwork()
                listDialog.close(); largePreview.close(); imageImport.close(); zipExport.close(); printDialog.close()
            }
        }
    }
    Connections {
        target: root.inferImage
        function onCandidateCreated(candidateId, artifactId) {
            if (root.artworkTargetExists && !root.editingArtwork && artifactId === root.artworkArtifactId)
                root.artworkCandidateId = candidateId
        }
    }
    Connections {
        target: root.materialEdit
        function onCandidateCreated(candidateId, artifactId) {
            if (root.artworkTargetExists && root.editingArtwork && artifactId === root.artworkArtifactId)
                root.artworkCandidateId = candidateId
        }
    }
    Timer { id: saveTimer; interval: 500; onTriggered: root.flush() }
    FileDialog {
        id: imageImport
        property string cardId: ""
        property string projectId: ""
        property string projectPath: ""
        property string kitId: ""
        title: qsTr("Choose card artwork")
        nameFilters: [qsTr("Images (*.png *.jpg *.jpeg)")]
        onAccepted: {
            root.importArtwork(selectedFile, cardId, projectId, kitId, projectPath)
        }
    }
    FileDialog {
        id: zipExport
        property string projectId: ""
        property string projectPath: ""
        property var sourcePin: ({})
        title: qsTr("Export accepted card files")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "zip"
        nameFilters: [qsTr("Card material archive (*.zip)")]
        onAccepted: root.exportKitFiles(selectedFile, projectId, projectPath, sourcePin)
    }
    contentItem: ColumnLayout {
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            ShapeComboBox {
                objectName: "cardKitContinuePicker"
                Layout.preferredWidth: 240
                model: root.savedKits; textRole: "name"
                onActivated: index => root.selectKit(root.savedKits[index].id)
            }
            ShapeButton { objectName: "cardKitNew"; text: qsTr("New cards"); onClicked: root.newKit() }
            ShapeTextField {
                objectName: "cardKitName"
                Layout.fillWidth: true
                text: root.composition.name || ""; placeholderText: qsTr("Name these card materials")
                onTextEdited: root.setField("name", text)
            }
            Label { text: root.dirty ? (root.controller.error.length ? qsTr("Draft not saved") : qsTr("Saving draft…")) : qsTr("Draft saved"); color: Theme.muted; font.pixelSize: 11 }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 16
            ColumnLayout {
                Layout.preferredWidth: 230; Layout.maximumWidth: 260; Layout.fillHeight: true; spacing: 8
                Label { text: qsTr("Shared brief"); color: Theme.text; font.weight: Font.DemiBold }
                ShapeTextEditor {
                    objectName: "cardKitBrief"
                    Layout.fillWidth: true; Layout.preferredHeight: 95
                    text: root.composition.brief || ""; placeholderText: qsTr("What are these cards for?")
                    wrapMode: TextEdit.Wrap
                    onTextChanged: if (editorActiveFocus) root.setField("brief", text)
                }
                Label { text: qsTr("Visual style"); color: Theme.text; font.weight: Font.DemiBold }
                ShapeTextEditor {
                    objectName: "cardKitStyle"
                    Layout.fillWidth: true; Layout.preferredHeight: 80
                    text: root.composition.style || ""; placeholderText: qsTr("Palette, mood and illustration style…")
                    wrapMode: TextEdit.Wrap
                    onTextChanged: if (editorActiveFocus) root.setField("style", text)
                }
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: qsTr("Cards · %1").arg(root.cards.length); color: Theme.text; font.weight: Font.DemiBold; Layout.fillWidth: true }
                    ShapeButton { objectName: "cardKitAddCard"; text: qsTr("Add"); enabled: root.cards.length < 64; onClicked: root.addCard() }
                }
                ShapeButton { objectName: "cardKitAddFromList"; Layout.fillWidth: true; text: qsTr("Add from a list…"); enabled: root.cards.length < 64; onClicked: root.openCardList() }
                ListView {
                    id: cardList
                    objectName: "cardKitCardList"
                    Layout.fillWidth: true; Layout.fillHeight: true
                    model: root.cards; spacing: 6; clip: true
                    currentIndex: root.cards.findIndex(c => c.id === root.selectedCardId)
                    function keepCurrentVisible() {
                        if (currentIndex >= 0 && height > 0) positionViewAtIndex(currentIndex, ListView.Contain)
                    }
                    onCurrentIndexChanged: Qt.callLater(keepCurrentVisible)
                    onCountChanged: Qt.callLater(keepCurrentVisible)
                    onHeightChanged: Qt.callLater(keepCurrentVisible)
                    onVisibleChanged: if (visible) Qt.callLater(keepCurrentVisible)
                    delegate: ItemDelegate {
                        id: cardRow
                        objectName: "cardKitRow-" + (cardRow.card.id || "")
                        required property var modelData
                        required property int index
                        readonly property var card: modelData || ({})
                        width: ListView.view.width
                        height: 66
                        onClicked: if (cardRow.card.id) root.selectedCardId = cardRow.card.id
                        background: Rectangle { radius: 8; color: root.selectedCardId === cardRow.card.id ? Theme.accentSoft : Theme.panel; border.color: root.selectedCardId === cardRow.card.id ? Theme.accentBorder : Theme.border }
                        contentItem: ColumnLayout {
                            spacing: 4
                            Label { Layout.fillWidth: true; text: (cardRow.index + 1) + ". " + (cardRow.card.title || qsTr("Untitled card")); elide: Text.ElideRight; color: Theme.text; font.pixelSize: 13 }
                            Label { text: root.cardState(cardRow.card); color: cardRow.card.artwork ? Theme.muted : Theme.warning; font.pixelSize: 11 }
                        }
                    }
                }
                RowLayout {
                    ShapeButton { objectName: "cardKitMoveUp"; text: qsTr("Move up"); enabled: root.cards.findIndex(c => c.id === root.selectedCardId) > 0; onClicked: root.moveCard(-1) }
                    ShapeButton { objectName: "cardKitMoveDown"; text: qsTr("Move down"); enabled: root.cards.findIndex(c => c.id === root.selectedCardId) < root.cards.length - 1; onClicked: root.moveCard(1) }
                }
            }
            ScrollView {
                Layout.fillWidth: true; Layout.fillHeight: true
                contentWidth: availableWidth; clip: true; ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ColumnLayout {
                    width: parent.width; spacing: 10
                    Label { text: qsTr("Card text"); color: Theme.text; font.pixelSize: 16; font.weight: Font.DemiBold }
                    ShapeTextField { objectName: "cardKitTitle"; Layout.fillWidth: true; text: root.currentCard ? root.currentCard.title : ""; placeholderText: qsTr("Card title"); onTextEdited: root.setCardField("title", text) }
                    RowLayout {
                        Layout.fillWidth: true
                        ShapeTextField { Layout.fillWidth: true; text: root.currentCard ? root.currentCard.card_type : ""; placeholderText: qsTr("Type (optional)"); onTextEdited: root.setCardField("card_type", text) }
                        ShapeTextField { Layout.preferredWidth: 85; text: root.currentCard ? root.currentCard.cost : ""; placeholderText: qsTr("Cost"); onTextEdited: root.setCardField("cost", text) }
                    }
                    Label { text: qsTr("Rules or description"); color: Theme.muted }
                    ShapeTextEditor {
                        objectName: "cardKitRules"
                        Layout.fillWidth: true; Layout.preferredHeight: 175
                        text: root.currentCard ? root.currentCard.rules : ""
                        placeholderText: qsTr("Write the card effect or description…")
                        wrapMode: TextEdit.Wrap
                        onTextChanged: if (editorActiveFocus) root.setCardField("rules", text)
                    }
                    ShapeTextField { Layout.fillWidth: true; text: root.currentCard ? root.currentCard.stats : ""; placeholderText: qsTr("Stats (optional)"); onTextEdited: root.setCardField("stats", text) }
                    ShapeTextField { Layout.fillWidth: true; text: root.currentCard ? root.currentCard.flavor : ""; placeholderText: qsTr("Flavor text (optional)"); onTextEdited: root.setCardField("flavor", text) }
                    Label { text: qsTr("Artwork description"); color: Theme.muted }
                    ShapeTextEditor {
                        objectName: "cardKitArtPrompt"
                        Layout.fillWidth: true; Layout.preferredHeight: 110
                        text: root.currentCard ? root.currentCard.art_prompt : ""
                        placeholderText: qsTr("Describe this card’s subject and composition…")
                        wrapMode: TextEdit.Wrap
                        onTextChanged: if (editorActiveFocus) root.setCardField("art_prompt", text)
                    }
                    RowLayout {
                        Label { text: qsTr("Copies"); color: Theme.muted }
                        SpinBox { objectName: "cardKitQuantity"; from: 1; to: 999; value: root.currentCard ? root.currentCard.quantity : 1; onValueModified: root.setCardField("quantity", value) }
                        Item { Layout.fillWidth: true }
                        ShapeButton { text: qsTr("Remove card"); enabled: root.cards.length > 1; onClicked: root.removeCard() }
                    }
                }
            }
            ColumnLayout {
                Layout.preferredWidth: 300; Layout.maximumWidth: 330; Layout.fillHeight: true; spacing: 10
                Label { text: qsTr("Artwork and check"); color: Theme.text; font.pixelSize: 16; font.weight: Font.DemiBold }
                Rectangle {
                    Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: root.height < 740 ? 120 : 200
                    radius: Theme.panelRadius; color: Theme.panelInset; border.color: Theme.border
                    Image { objectName: "cardKitPreviewImage"; anchors.fill: parent; anchors.margins: 8; source: root.controller.previewSource; fillMode: Image.PreserveAspectFit; asynchronous: false }
                    Label { anchors.centerIn: parent; width: parent.width - 36; visible: !root.controller.previewSource.length; text: qsTr("Check the cards to see the layout here."); wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter; color: Theme.muted }
                    MouseArea { anchors.fill: parent; enabled: root.controller.previewSource.length > 0; cursorShape: Qt.PointingHandCursor; onClicked: root.openLargePreview() }
                }
                ShapeButton { objectName: "cardKitEnlargePreview"; Layout.fillWidth: true; text: qsTr("Enlarge preview"); enabled: root.controller.previewSource.length > 0; onClicked: root.openLargePreview() }
                ShapeComboBox { objectName: "cardKitArtworkPicker"; Layout.fillWidth: true; model: root.images; textRole: "name"; currentIndex: root.artworkIndex(); onActivated: index => root.chooseArtwork(index) }
                RowLayout {
                    Layout.fillWidth: true
                    ShapeButton { objectName: "cardKitImportArtwork"; Layout.fillWidth: true; text: qsTr("Import image…"); onClicked: root.beginImageImport() }
                    ShapeButton { objectName: "cardKitGenerateArtwork"; Layout.fillWidth: true; text: qsTr("Generate image…"); onClicked: root.openArtworkGeneration() }
                }
                ShapeButton { objectName: "cardKitRefineArtwork"; Layout.fillWidth: true; text: root.artworkTargetExists && root.editingArtwork && root.artworkCardId === root.selectedCardId ? qsTr("Continue artwork edit…") : qsTr("Refine in Shadow…"); enabled: root.currentCard !== null && root.currentCard.artwork !== null; onClicked: root.beginArtworkEdit() }
                Label { Layout.fillWidth: true; text: root.currentCard ? root.cardState(root.currentCard) : ""; color: Theme.accent; wrapMode: Text.WordWrap }
                Label { Layout.fillWidth: true; text: qsTr("Next: add text and accepted artwork, check each card, then use the version before exporting."); color: Theme.muted; wrapMode: Text.WordWrap; font.pixelSize: 12 }
            }
        }
        Label {
            objectName: "cardKitFeedback"
            Layout.fillWidth: true; visible: text.length > 0
            text: root.localError || root.controller.error || root.controller.status
            color: root.localError.length || root.controller.error.length ? Theme.danger : Theme.accent
            wrapMode: Text.WordWrap; font.pixelSize: 12
        }
        RowLayout {
            visible: root.controller.error.length > 0 && root.dirty
            Layout.fillWidth: true
            ShapeButton { objectName: "cardKitCopyDraft"; text: qsTr("Copy complete draft"); onClicked: root.controller.copyDraft(root.composition) }
            ShapeButton { objectName: "cardKitReloadDraft"; text: qsTr("Reload saved draft"); onClicked: root.reloadSavedDraft() }
            Label { Layout.fillWidth: true; text: qsTr("Reloading replaces unsaved fields. Copy the draft first if you want to keep them."); color: Theme.muted; wrapMode: Text.WordWrap }
        }
        Label {
            Layout.fillWidth: true; visible: (root.controller.preview.overflow_card_ids || []).length > 0
            text: qsTr("Shorten the text on %1 cards before exporting.").arg((root.controller.preview.overflow_card_ids || []).length)
            color: Theme.warning; wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true; visible: (root.controller.preview.stale_inputs || []).length > 0
            text: qsTr("Some chosen materials have newer versions. Choose them explicitly if you want to update these cards.")
            color: Theme.warning; wrapMode: Text.WordWrap
        }
        RowLayout {
            Layout.fillWidth: true
            ShapeButton { objectName: "cardKitClose"; text: qsTr("Back to project"); onClicked: if (root.flush()) root.close() }
            Item { Layout.fillWidth: true }
            ShapeButton { objectName: "cardKitReview"; text: qsTr("Check cards"); onClicked: if (root.flush()) root.controller.review(root.selectedCardId) }
            ShapeButton { objectName: "cardKitAccept"; text: qsTr("Use this version"); primary: true; enabled: !root.dirty && root.controller.hasReview; onClicked: root.controller.accept() }
            ShapeButton { objectName: "cardKitPrint"; text: qsTr("Print layout…"); enabled: !root.dirty && root.controller.canExport && !root.cardPrint.running; onClicked: printDialog.openForSource(root.controller.source) }
            ShapeButton { objectName: "cardKitExport"; text: qsTr("Export accepted files…"); enabled: !root.dirty && root.controller.canExport; onClicked: { zipExport.projectId = root.backend.projectId; zipExport.projectPath = root.backend.bundlePath; zipExport.sourcePin = root.copy(root.controller.source); zipExport.open() } }
        }
    }
    function exportKitFiles(file, projectId, projectPath, sourcePin) {
        if (projectId !== backend.projectId || projectPath !== backend.bundlePath
            || JSON.stringify(sourcePin) !== JSON.stringify(controller.source)) return false
        return controller.exportFiles(file)
    }
    CardPrintDialog { id: printDialog; backend: root.backend; controller: root.cardPrint }
    ShapeDialog {
        id: largePreview
        objectName: "cardKitLargePreview"
        property string svgSource: ""
        property string cardTitle: ""
        property int canvasWidth: 750
        property int canvasHeight: 1050
        property bool actualSize: true
        parent: Overlay.overlay; anchors.centerIn: parent
        width: parent ? Math.min(1200, parent.width - 60) : 1100
        height: parent ? parent.height - 60 : 800
        modal: true; title: cardTitle || qsTr("Card preview"); closePolicy: Popup.CloseOnEscape
        contentItem: ColumnLayout {
            RowLayout {
                Layout.fillWidth: true
                Label { Layout.fillWidth: true; text: largePreview.actualSize ? qsTr("Actual size · Scroll to inspect the whole card") : qsTr("Fit to window"); color: Theme.muted; wrapMode: Text.WordWrap }
                ShapeButton { objectName: "cardKitPreviewFit"; text: qsTr("Fit"); onClicked: largePreview.actualSize = false }
                ShapeButton { objectName: "cardKitPreviewActual"; text: qsTr("Actual size"); onClicked: largePreview.actualSize = true }
            }
            ScrollView {
                id: previewScroll
                Layout.fillWidth: true; Layout.fillHeight: true
                contentWidth: largeImage.width; contentHeight: largeImage.height
                clip: true
                background: Rectangle { color: Theme.panelInset; radius: Theme.controlRadius }
                Image {
                    id: largeImage
                    objectName: "cardKitLargeImage"
                    readonly property real fitScale: Math.min(1, previewScroll.availableWidth / largePreview.canvasWidth, previewScroll.availableHeight / largePreview.canvasHeight)
                    width: largePreview.canvasWidth * (largePreview.actualSize ? 1 : fitScale)
                    height: largePreview.canvasHeight * (largePreview.actualSize ? 1 : fitScale)
                    source: largePreview.svgSource
                    sourceSize: Qt.size(largePreview.canvasWidth, largePreview.canvasHeight)
                    fillMode: Image.PreserveAspectFit
                }
            }
            ShapeButton { objectName: "cardKitPreviewClose"; Layout.alignment: Qt.AlignRight; text: qsTr("Back to cards"); onClicked: largePreview.close() }
        }
    }
    ShapeDialog {
        id: listDialog
        objectName: "cardKitListDialog"
        parent: Overlay.overlay; anchors.centerIn: parent
        width: parent ? Math.min(760, parent.width - 60) : 760
        height: parent ? Math.min(660, parent.height - 60) : 660
        modal: true; title: qsTr("Add cards from a list"); closePolicy: Popup.CloseOnEscape
        contentItem: ColumnLayout {
            spacing: 10
            Label { Layout.fillWidth: true; text: qsTr("Copy rows from a table: title, rules, artwork description. Only the title is required; the other columns can be filled in later."); wrapMode: Text.WordWrap; color: Theme.muted }
            ShapeTextEditor { id: listText; objectName: "cardKitListText"; Layout.fillWidth: true; Layout.preferredHeight: 200; wrapMode: TextEdit.NoWrap; placeholderText: qsTr("Paste your rows here…") }
            Label { objectName: "cardKitListSummary"; Layout.fillWidth: true; text: root.listPreview.error.length ? root.cardListError() : root.listPreview.replacesPlaceholder ? qsTr("New cards: %1. The empty starter card will be replaced.").arg(root.listPreview.rows.length) : qsTr("New cards: %1. Existing cards stay as they are.").arg(root.listPreview.rows.length); color: root.listPreview.error.length && root.listPreview.error !== "empty" ? Theme.danger : Theme.text; wrapMode: Text.WordWrap }
            Label { Layout.fillWidth: true; visible: root.listPreview.headerSkipped; text: qsTr("The column heading row is skipped."); color: Theme.muted }
            Label { Layout.fillWidth: true; visible: root.listPreview.duplicateCount > 0; text: qsTr("Repeated titles are kept as separate cards: %1.").arg(root.listPreview.duplicateCount); color: Theme.warning; wrapMode: Text.WordWrap }
            ListView {
                objectName: "cardKitListPreview"
                Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 5
                model: root.listPreview.rows
                delegate: Rectangle {
                    id: rowPreview
                    required property var modelData
                    required property int index
                    readonly property var row: modelData || ({})
                    width: ListView.view.width; height: 58
                    radius: 6; color: Theme.panel; border.color: Theme.border
                    ColumnLayout {
                        anchors.fill: parent; anchors.margins: 8; spacing: 3
                        Label { Layout.fillWidth: true; text: (rowPreview.index + 1) + ". " + (rowPreview.row.title || ""); color: Theme.text; elide: Text.ElideRight }
                        Label { Layout.fillWidth: true; text: rowPreview.row.rules || rowPreview.row.art_prompt || qsTr("Details can be added later."); color: Theme.muted; elide: Text.ElideRight }
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                ShapeButton { objectName: "cardKitListCancel"; text: qsTr("Cancel"); onClicked: listDialog.close() }
                Item { Layout.fillWidth: true }
                ShapeButton { objectName: "cardKitListApply"; text: root.listPreview.rows.length === 1 ? qsTr("Add card") : qsTr("Add %1 cards").arg(root.listPreview.rows.length); primary: true; enabled: !root.listApplied && !root.listPreview.error.length && root.listPreview.rows.length > 0; onClicked: root.appendCardList() }
            }
        }
    }
    ShapeDialog {
        id: generationDialog
        objectName: "cardArtworkGenerationDialog"
        parent: Overlay.overlay; anchors.centerIn: parent
        width: parent ? Math.min(940, parent.width - 60) : 940; height: parent ? Math.min(740, parent.height - 60) : 740
        modal: true; title: qsTr("Create artwork for this card"); closePolicy: Popup.NoAutoClose
        contentItem: RowLayout {
            ColumnLayout {
                Layout.preferredWidth: 340; Layout.fillHeight: true
                Label { Layout.fillWidth: true; text: qsTr("The shared brief and style are included. Review the result before adding it to the card."); color: Theme.muted; wrapMode: Text.WordWrap }
                ShapeTextEditor { id: generationPrompt; objectName: "cardArtworkGenerationPrompt"; Layout.fillWidth: true; Layout.fillHeight: true; wrapMode: TextEdit.Wrap; readOnly: root.artworkRunning }
                AiModelPicker { id: modelPicker; Layout.fillWidth: true; family: "image"; onChoiceSelected: key => overrideKey = key; onEffortSelected: key => effortOverrideKey = key; defaultModelKey: root.uiPreferences.imageModel; defaultEffortKey: root.uiPreferences.imageEffort; enabled: !root.inferImage.running }
                Label { Layout.fillWidth: true; text: root.artworkRunning ? qsTr("Creating artwork…") : root.inferImage.errorCode; color: Theme.muted; wrapMode: Text.WordWrap }
                ShapeButton { objectName: "cardArtworkGenerate"; Layout.fillWidth: true; text: qsTr("Generate a version"); primary: true; enabled: root.artworkTargetExists && root.inferText.credentialConfigured && !root.inferImage.running && generationPrompt.text.trim().length > 0; onClicked: root.generateArtwork() }
                ShapeButton { text: qsTr("Stop after current image"); visible: root.artworkRunning; onClicked: root.inferImage.stopAfterCurrent() }
                Label { Layout.fillWidth: true; visible: !root.inferText.credentialConfigured; text: qsTr("Add the Shape Infer credential in Settings before generating."); color: Theme.warning; wrapMode: Text.WordWrap }
                ShapeButton { text: qsTr("Open Settings"); visible: !root.inferText.credentialConfigured; onClicked: root.inferAccessSetupRequested() }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true
                Image { Layout.fillWidth: true; Layout.fillHeight: true; source: root.artworkCandidateSource; fillMode: Image.PreserveAspectFit }
                ShapeButton { objectName: "cardArtworkUseGenerated"; text: qsTr("Use this artwork for the card"); primary: true; enabled: root.artworkTargetExists && root.artworkCandidate !== null && !root.inferImage.running; onClicked: root.useArtwork() }
                ShapeButton { text: qsTr("Back to cards"); onClicked: generationDialog.close() }
            }
        }
    }
    ShapeDialog {
        id: refinementDialog
        objectName: "cardArtworkRefinementDialog"
        parent: Overlay.overlay; anchors.centerIn: parent
        width: parent ? parent.width - 60 : 1180; height: parent ? parent.height - 60 : 760
        modal: true; title: qsTr("Refine card artwork"); closePolicy: Popup.NoAutoClose
        contentItem: ColumnLayout {
            MaterialEditWorkspace {
                Layout.fillWidth: true; Layout.fillHeight: true
                backend: root.backend; controller: root.materialEdit; audioPreview: root.audioPreview
                projectPath: root.backend.bundlePath
                outputArtifactId: root.editingStage.contextArtifactId || ""
                operatorDraftId: root.editingStage.id || ""
                inputArtifactId: root.editingStage.inputArtifactId || ""
                inputRevisionId: root.editingStage.inputRevisionId || ""
                imageMode: true
                inputCurrent: root.artworkStageCurrent
                sourceName: (root.cards.find(c => c.id === root.artworkCardId) || {}).title || ""
                candidate: root.artworkCandidate
                acceptedImageSource: root.editingStage.contextArtifactId ? root.backend.materialSourceImage(root.editingStage.contextArtifactId) : ""
                candidateImageSource: root.artworkCandidateSource
            }
            RowLayout {
                ShapeButton { text: qsTr("Back to cards"); onClicked: refinementDialog.close() }
                Item { Layout.fillWidth: true }
                ShapeButton { objectName: "cardArtworkUseRefined"; text: qsTr("Use this artwork for the card"); primary: true; enabled: root.artworkTargetExists && root.artworkStageCurrent && root.artworkCandidate !== null && !root.materialEdit.running && !root.materialEdit.shadowActive; onClicked: root.useArtwork() }
            }
        }
    }
}

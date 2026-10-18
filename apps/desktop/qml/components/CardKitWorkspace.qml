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
    property bool listFirstHeader: false
    property int cardIdSequence: 1
    property bool moreText: false
    property bool showArtPrompt: false
    readonly property var currentPreview: (controller.preview.cards || []).find(c => c.id === selectedCardId) || ({})
    readonly property var currentLayout: currentPreview.resolved_layout || ({})
    readonly property var hiddenContent: currentPreview.hidden_content || ({artwork: false, text_fields: []})
    readonly property bool imageCropLimited: (controller.preview.cards || []).some(card => (card.issues || []).indexOf("image_crop_limit") >= 0)
    readonly property string imageCropMessage: qsTr("Some images are too narrow or wide for edge cropping. Choose Show the complete image, or use artwork with more suitable proportions.")
    readonly property var listPreview: CardListImport.parse(listText.text, composition, listFirstHeader)
    signal inferAccessSetupRequested()

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: parent ? parent.width - 28 : 1260
    height: parent ? parent.height - 40 : 820
    modal: true
    title: qsTr("Make cards")
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
        if (key === "layout" && value && !composition.layout) {
            localError = qsTr("Choose a flexible default layout for this set before changing a card’s layout."); return
        }
        const next = copy(composition), index = next.cards.findIndex(c => c.id === selectedCardId)
        next.cards[index][key] = value
        if (key === "rules") next.cards[index].text_material = null
        changed(next)
    }
    function flush() {
        saveTimer.stop()
        if (!dirty) return true
        if (!controller.save(composition)) return false
        dirty = false; if (visible) previewTimer.restart(); return true
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
        if (flush() && controller.beginNew()) { clearArtwork(); synchronize(); controller.showCard(selectedCardId) }
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
        listApplied = false; listFirstHeader = false; listText.text = ""; listDialog.open(); listText.focusEditor()
    }
    function cardListError() {
        const result = listPreview
        switch (result.error) {
        case "empty": return qsTr("Paste at least one card title or text to begin.")
        case "too_large": case "composition_size": return qsTr("This list is too large for these cards. Add a smaller group.")
        case "too_many_cards": return qsTr("A set can contain up to 64 cards. Add fewer rows or start a new set.")
        case "columns": return qsTr("Row %1 has more than three columns. Use title, text and image description.").arg(result.line)
        case "content": return qsTr("Row %1 needs a title or text. An image description alone is not printable content.").arg(result.line)
        case "heading": return qsTr("Use title, text or image description as column headings, or turn off the heading option.")
        case "field_length": return qsTr("Row %1 has too much text. Shorten its title, rules or artwork description.").arg(result.line)
        case "quote": return qsTr("Check the quotation marks around row %1. Copying the cells again may help.").arg(result.line)
        case "invalid_text": return qsTr("Row %1 contains unsupported text. Remove hidden control characters and try again.").arg(result.line)
        default: return ""
        }
    }
    function appendCardList() {
        const result = CardListImport.parse(listText.text, composition, listFirstHeader)
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
        const state = (controller.preview.cards || []).find(item => item.id === card.id)
        const issues = state ? state.issues || [] : []
        if (issues.indexOf("image_crop_limit") >= 0) return qsTr("Use the full image or artwork with different proportions")
        if (issues.indexOf("layout_overflow") >= 0) return qsTr("Shorten text or change the layout")
        if (issues.indexOf("empty_content") >= 0) return qsTr("Add an image or text")
        if (issues.indexOf("missing_artwork") >= 0) return qsTr("Add an image")
        if (issues.indexOf("missing_text") >= 0 || issues.indexOf("missing_title") >= 0 || issues.indexOf("missing_body") >= 0) return qsTr("Add text")
        if (state && state.content_ready) return qsTr("Ready")
        return qsTr("Ready to check")
    }
    function cardName(card) {
        if ((card.title || "").trim().length) return card.title
        if ((card.rules || "").trim().length) return card.rules.trim().split("\n")[0]
        const image = card.artwork ? backend.artifacts.find(item => item.id === card.artwork.artifact_id) : null
        return image ? image.name : qsTr("Untitled card")
    }
    function cardNeedsAttention(card) {
        const state = (controller.preview.cards || []).find(item => item.id === card.id)
        return !!state && !state.content_ready && (state.issues || []).length > 0
    }
    function hiddenDescription() {
        const names = []
        if (hiddenContent.artwork) names.push(qsTr("image"))
        const labels = {title: qsTr("title"), rules: qsTr("body text"), text_material: qsTr("accepted text"),
            card_type: qsTr("type"), cost: qsTr("cost"), stats: qsTr("stats"), flavor: qsTr("additional text")}
        for (const key of hiddenContent.text_fields || []) if (labels[key]) names.push(labels[key])
        return names.length ? qsTr("This layout hides %1. The content is kept.").arg(names.join(" · ")) : ""
    }
    function removeArtwork() {
        if (!currentCard || !currentCard.artwork) return false
        const cardId = selectedCardId
        setCardField("artwork", null)
        if (artworkCardId === cardId) { clearArtwork(); controller.forgetArtworkEdit(cardId) }
        return flush()
    }
    function setCanvas(width, height) {
        const next = copy(composition)
        if (next.canvas_width === width && next.canvas_height === height) return
        next.canvas_width = width; next.canvas_height = height
        if (!next.layout) next.layout = {mode: "auto"}
        changed(next)
    }
    function beginCardImages() {
        if (!flush()) return false
        if (!composition.layout) {
            localError = qsTr("Choose a flexible layout or start a new set before adding complete card images. The saved cards keep their original layout until you choose.")
            setLayoutDialog.open(); return false
        }
        return imageIntake.chooseFiles()
    }
    function canAppendImage(cardId) {
        return !!composition.layout && (cards.some(card => card.id === cardId) || CardListImport.effectiveCards(composition).length < 64)
    }
    function appendImportedCard(source, cardId, width, height, useShape, projectId, projectPath, kitId, batchOrder) {
        if (projectId !== backend.projectId || projectPath !== backend.bundlePath || kitId !== controller.draft.artifact_id)
            return {ok: false, message: qsTr("The project or card set changed. Reopen the image list.")}
        const existing = cards.find(card => card.id === cardId)
        if (existing) {
            if (JSON.stringify(existing.artwork) !== JSON.stringify(source))
                return {ok: false, message: qsTr("This card changed. Keep its current content and reopen the image list.")}
            return {ok: flush(), message: controller.error}
        }
        if (!canAppendImage(cardId)) return {ok: false, message: qsTr("This set already contains 64 cards.")}
        const next = copy(composition)
        next.cards = copy(CardListImport.effectiveCards(composition))
        let notice = ""
        if (useShape && next.cards.length === 0 && width > 0 && height > 0) {
            if (width / height < 0.1 || width / height > 10) {
                notice = qsTr("This image is unusually narrow or wide. The set’s proportions were kept, and the complete image will fit inside the card.")
            } else {
                const scale = Math.min(2400 / Math.max(width, height), Math.max(1, 240 / Math.min(width, height)))
                next.canvas_width = Math.round(width * scale)
                next.canvas_height = Math.round(height * scale)
            }
        }
        const card = {id: cardId, title: "", card_type: "", cost: "", rules: "", stats: "", flavor: "", art_prompt: "",
            quantity: 1, artwork: source, text_material: null, layout: {mode: "image", image_fit: "contain", padding_percent: 0, border: false}}
        const later = (batchOrder || []).slice((batchOrder || []).indexOf(cardId) + 1)
        const insertion = next.cards.findIndex(item => later.indexOf(item.id) >= 0)
        next.cards.splice(insertion < 0 ? next.cards.length : insertion, 0, card)
        changed(next); selectedCardId = cardId
        const saved = flush()
        if (saved && notice.length) localError = notice
        return {ok: saved, message: saved ? notice : controller.error}
    }
    function acceptAndPrint() {
        if (!flush()) return false
        previewTimer.stop()
        let acceptedNow = false
        if (!controller.currentAccepted) {
            if (!controller.review(selectedCardId)) return false
            if (imageCropLimited) { localError = imageCropMessage; return false }
            if ((controller.preview.cards || []).some(card => (card.issues || []).some(issue => issue !== "layout_overflow"))) {
                localError = qsTr("Complete the content needed by each card’s layout before printing.")
                return false
            }
            if (!controller.accept()) return false
            acceptedNow = true
        }
        const opened = printDialog.openForSource(controller.source, controller.preview.print_defaults)
        if (!opened && acceptedNow) localError = qsTr("The card version was accepted. Adjust the print layout in the opened window to continue.")
        return opened
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
        largePreview.canvasWidth = currentLayout.canvas_width || composition.canvas_width || 750
        largePreview.canvasHeight = currentLayout.canvas_height || composition.canvas_height || 1050
        largePreview.cardTitle = currentCard ? cardName(currentCard) : ""
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
        controller.showCard(selectedCardId)
        if (artworkTargetExists && artworkCardId === selectedCardId && !editingArtwork && artworkArtifactId.length) {
            generationDialog.open(); return
        }
        if (inferImage.running) { localError = qsTr("Finish or stop the current image run before starting another."); return }
        clearArtwork(); artworkCardId = selectedCardId; artworkProject = backend.projectId; artworkPath = backend.bundlePath; artworkKitId = controller.draft.artifact_id
        generationPrompt.text = [composition.brief, composition.style, currentCard.title, currentCard.art_prompt,
            currentLayout.mode === "image" || currentLayout.mode === "auto" ? qsTr("Create a complete card image. Follow the chosen card proportions.") : qsTr("Card illustration. Keep the card text outside the artwork.")].filter(t => t && t.trim().length).join("\n\n")
        generationDialog.open()
    }
    function generateArtwork() {
        if (!artworkTargetExists || inferImage.running || !generationPrompt.text.trim().length) return false
        const canvas = generationCanvas()
        if (!artworkArtifactId.length) {
            const card = cards.find(c => c.id === artworkCardId)
            const draft = backend.beginAiImageDraft(card.title || qsTr("Card artwork"), generationPrompt.text, canvas.width, canvas.height)
            if (!draft.id || !draft.contextArtifactId) { localError = backend.lastError; return false }
            artworkArtifactId = draft.contextArtifactId; artworkDraftId = draft.id
        }
        if (!backend.updateAiImageDraft(artworkDraftId, generationPrompt.text, canvas.width, canvas.height, 1)) return false
        generationStarted = true
        inferImage.generate(backend.bundlePath, artworkArtifactId, artworkDraftId, modelPicker.effectiveModelKey, modelPicker.effectiveEffortKey, 1)
        return true
    }
    function generationCanvas() {
        if (currentLayout.mode !== "image" && currentLayout.mode !== "auto") return {width: 1536, height: 1024}
        const ratio = (currentLayout.canvas_width || composition.canvas_width || 750) / (currentLayout.canvas_height || composition.canvas_height || 1050)
        return ratio > 1.1 ? {width: 1536, height: 1024} : ratio < 0.9 ? {width: 1024, height: 1536} : {width: 1024, height: 1024}
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
                saveTimer.stop(); previewTimer.stop(); root.dirty = false; root.close()
                generationDialog.close(); refinementDialog.close(); root.clearArtwork()
                listDialog.close(); largePreview.close(); imageImport.close(); zipExport.close(); printDialog.close(); setLayoutDialog.close()
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
    Timer { id: previewTimer; interval: 200; onTriggered: if (root.visible && !root.dirty && root.selectedCardId.length) root.controller.showCard(root.selectedCardId) }
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
                Layout.preferredWidth: 190
                model: root.savedKits; textRole: "name"
                onActivated: index => root.selectKit(root.savedKits[index].id)
            }
            ShapeButton { objectName: "cardKitNew"; text: qsTr("New cards"); onClicked: root.newKit() }
            ShapeTextField {
                objectName: "cardKitName"
                Layout.fillWidth: true
                text: root.composition.name || ""; placeholderText: qsTr("Name this card set")
                onTextEdited: root.setField("name", text)
            }
            ShapeButton { objectName: "cardKitSetLayout"; text: qsTr("Set layout…"); onClicked: setLayoutDialog.open() }
            ShapeButton { objectName: "cardKitSave"; text: qsTr("Save draft"); enabled: root.dirty; onClicked: root.flush() }
            Label { text: root.dirty ? (root.controller.error.length ? qsTr("Draft not saved") : qsTr("Saving draft…")) : qsTr("Draft saved"); color: Theme.muted; font.pixelSize: 11 }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 16
            ColumnLayout {
                Layout.preferredWidth: 230; Layout.maximumWidth: 260; Layout.fillHeight: true; spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: qsTr("Cards · %1").arg(root.cards.length); color: Theme.text; font.weight: Font.DemiBold; Layout.fillWidth: true }
                    ShapeButton { objectName: "cardKitAddCard"; text: qsTr("Blank card"); enabled: root.cards.length < 64; onClicked: root.addCard() }
                }
                ShapeButton { objectName: "cardKitAddImages"; Layout.fillWidth: true; text: qsTr("Add existing card images…"); enabled: root.cards.length < 64; onClicked: root.beginCardImages() }
                ShapeButton { objectName: "cardKitAddFromList"; Layout.fillWidth: true; text: qsTr("Add from a list…"); enabled: root.cards.length < 64; onClicked: root.openCardList() }
                Label { Layout.fillWidth: true; text: qsTr("Start with images, text, or both. Each card can use its own layout."); wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 12 }
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
                            Label { Layout.fillWidth: true; text: (cardRow.index + 1) + ". " + root.cardName(cardRow.card); elide: Text.ElideRight; color: Theme.text; font.pixelSize: 13 }
                            Label { text: root.cardState(cardRow.card); color: root.cardNeedsAttention(cardRow.card) ? Theme.warning : Theme.muted; font.pixelSize: 11 }
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
                    CardLayoutOptions {
                        objectName: "cardKitCardLayoutOptions"
                        Layout.fillWidth: true; perCard: true; enabled: !!root.composition.layout
                        configuration: root.currentCard ? root.currentCard.layout || null : null
                        resolved: root.currentLayout
                        editIdentity: root.backend.projectId + "|" + root.backend.bundlePath + "|" + (root.controller.draft.artifact_id || "") + "|" + root.selectedCardId
                        editingEnabled: root.visible && !!root.composition.layout
                        onEdited: value => root.setCardField("layout", value)
                    }
                    Label { objectName: "cardKitHiddenContent"; Layout.fillWidth: true; visible: text.length > 0; text: root.hiddenDescription(); color: Theme.warning; wrapMode: Text.WordWrap }
                    Label { text: qsTr("Card text"); color: Theme.text; font.pixelSize: 16; font.weight: Font.DemiBold }
                    ShapeTextField { objectName: "cardKitTitle"; Layout.fillWidth: true; text: root.currentCard ? root.currentCard.title : ""; placeholderText: qsTr("Title (optional)"); onTextEdited: root.setCardField("title", text) }
                    Label { text: qsTr("Body text"); color: Theme.muted }
                    ShapeTextEditor {
                        objectName: "cardKitRules"
                        Layout.fillWidth: true; Layout.preferredHeight: 175
                        text: root.currentCard ? root.currentCard.rules : ""
                        placeholderText: qsTr("A word, question, explanation, reminder…")
                        wrapMode: TextEdit.Wrap
                        onTextChanged: if (editorActiveFocus) root.setCardField("rules", text)
                    }
                    Label { Layout.fillWidth: true; visible: root.currentCard !== null && !!root.currentCard.text_material; text: qsTr("This card uses accepted text from the project. Writing here replaces that text binding."); color: Theme.muted; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                    ShapeButton { objectName: "cardKitMoreText"; text: root.moreText ? qsTr("Hide more text") : qsTr("More text…"); onClicked: root.moreText = !root.moreText }
                    ColumnLayout {
                        visible: root.moreText; Layout.fillWidth: true
                        RowLayout {
                            Layout.fillWidth: true
                            ShapeTextField { Layout.fillWidth: true; text: root.currentCard ? root.currentCard.card_type : ""; placeholderText: qsTr("Type (optional)"); onTextEdited: root.setCardField("card_type", text) }
                            ShapeTextField { Layout.preferredWidth: 85; text: root.currentCard ? root.currentCard.cost : ""; placeholderText: qsTr("Cost"); onTextEdited: root.setCardField("cost", text) }
                        }
                        ShapeTextField { Layout.fillWidth: true; text: root.currentCard ? root.currentCard.stats : ""; placeholderText: qsTr("Stats (optional)"); onTextEdited: root.setCardField("stats", text) }
                        ShapeTextField { Layout.fillWidth: true; text: root.currentCard ? root.currentCard.flavor : ""; placeholderText: qsTr("Additional text (optional)"); onTextEdited: root.setCardField("flavor", text) }
                    }
                    ShapeButton { text: root.showArtPrompt ? qsTr("Hide image description") : qsTr("Image description…"); onClicked: root.showArtPrompt = !root.showArtPrompt }
                    ShapeTextEditor {
                        objectName: "cardKitArtPrompt"
                        visible: root.showArtPrompt
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
                Label { text: qsTr("Card preview and image"); color: Theme.text; font.pixelSize: 16; font.weight: Font.DemiBold }
                Rectangle {
                    Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: root.height < 740 ? 120 : 200
                    radius: Theme.panelRadius; color: Theme.panelInset; border.color: Theme.border
                    Image { objectName: "cardKitPreviewImage"; anchors.fill: parent; anchors.margins: 8; source: root.controller.previewSource; fillMode: Image.PreserveAspectFit; asynchronous: false }
                    Label { anchors.centerIn: parent; width: parent.width - 36; visible: !root.controller.previewSource.length; text: qsTr("Check the cards to see the layout here."); wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter; color: Theme.muted }
                    MouseArea { anchors.fill: parent; enabled: root.controller.previewSource.length > 0; cursorShape: Qt.PointingHandCursor; onClicked: root.openLargePreview() }
                }
                RowLayout {
                    Layout.fillWidth: true
                    ShapeButton { objectName: "cardKitEnlargePreview"; Layout.fillWidth: true; text: qsTr("Enlarge preview"); enabled: root.controller.previewSource.length > 0; onClicked: root.openLargePreview() }
                    ShapeButton { objectName: "cardKitRemoveImage"; text: qsTr("Remove image"); enabled: root.currentCard !== null && root.currentCard.artwork !== null; onClicked: root.removeArtwork() }
                }
                ShapeComboBox { objectName: "cardKitArtworkPicker"; Layout.fillWidth: true; model: root.images; textRole: "name"; currentIndex: root.artworkIndex(); onActivated: index => root.chooseArtwork(index) }
                RowLayout {
                    Layout.fillWidth: true
                    ShapeButton { objectName: "cardKitImportArtwork"; Layout.fillWidth: true; text: qsTr("Import image…"); onClicked: root.beginImageImport() }
                    ShapeButton { objectName: "cardKitGenerateArtwork"; Layout.fillWidth: true; text: qsTr("Generate image…"); onClicked: root.openArtworkGeneration() }
                }
                ShapeButton { objectName: "cardKitRefineArtwork"; Layout.fillWidth: true; text: root.artworkTargetExists && root.editingArtwork && root.artworkCardId === root.selectedCardId ? qsTr("Continue artwork edit…") : qsTr("Refine in Shadow…"); enabled: root.currentCard !== null && root.currentCard.artwork !== null; onClicked: root.beginArtworkEdit() }
                Label { Layout.fillWidth: true; text: root.currentCard ? root.cardState(root.currentCard) : ""; color: Theme.accent; wrapMode: Text.WordWrap }
                Label { Layout.fillWidth: true; text: qsTr("Choose the content you need, then accept the cards and arrange them for printing."); color: Theme.muted; wrapMode: Text.WordWrap; font.pixelSize: 12 }
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
            text: qsTr("Text does not fit on %1 cards. Adjust the text or layout, or use a larger card size for printing.").arg((root.controller.preview.overflow_card_ids || []).length)
            color: Theme.warning; wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true; visible: root.imageCropLimited
            text: root.imageCropMessage; color: Theme.warning; wrapMode: Text.WordWrap
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
            ShapeButton { objectName: "cardKitAccept"; text: qsTr("Use this version"); enabled: !root.dirty && root.controller.hasReview; onClicked: root.controller.accept() }
            ShapeButton { objectName: "cardKitPrint"; text: !root.dirty && root.controller.currentAccepted ? qsTr("Print layout…") : qsTr("Accept and arrange for print…"); primary: true; enabled: !root.cardPrint.running; onClicked: root.acceptAndPrint() }
            ShapeButton { objectName: "cardKitExport"; text: qsTr("Export accepted files…"); enabled: !root.dirty && root.controller.canExport; onClicked: { zipExport.projectId = root.backend.projectId; zipExport.projectPath = root.backend.bundlePath; zipExport.sourcePin = root.copy(root.controller.source); zipExport.open() } }
        }
    }
    function exportKitFiles(file, projectId, projectPath, sourcePin) {
        if (projectId !== backend.projectId || projectPath !== backend.bundlePath
            || JSON.stringify(sourcePin) !== JSON.stringify(controller.source)) return false
        return controller.exportFiles(file)
    }
    CardPrintDialog { id: printDialog; backend: root.backend; controller: root.cardPrint }
    CardImageIntakeDialog {
        id: imageIntake
        backend: root.backend; controller: root.controller
        capacity: 64 - CardListImport.effectiveCards(root.composition).length
        canUseImageShape: !!root.composition.layout && CardListImport.effectiveCards(root.composition).length === 0
        appendImage: (source, cardId, width, height, useShape, projectId, projectPath, kitId, order) => root.appendImportedCard(source, cardId, width, height, useShape, projectId, projectPath, kitId, order)
        canAppend: cardId => root.canAppendImage(cardId)
    }
    ShapeDialog {
        id: setLayoutDialog
        objectName: "cardKitSetLayoutDialog"
        property bool promptsExpanded: false
        property bool customRatioExpanded: false
        property string ratioError: ""
        function applyCustomRatio() {
            let width = NaN, height = NaN
            try { width = Number.fromLocaleString(Qt.locale(), customWidth.text); height = Number.fromLocaleString(Qt.locale(), customHeight.text) } catch (_) {}
            const ratio = width / height
            if (!customWidth.acceptableInput || !customHeight.acceptableInput || !isFinite(width) || !isFinite(height) || width <= 0 || height <= 0 || ratio < 0.1 || ratio > 10) {
                ratioError = qsTr("Enter a positive width and height. Neither side can be more than ten times the other."); return false
            }
            ratioError = ""
            root.setCanvas(ratio >= 1 ? 2400 : Math.round(2400 * ratio), ratio >= 1 ? Math.round(2400 / ratio) : 2400)
            return true
        }
        parent: Overlay.overlay; anchors.centerIn: parent
        width: parent ? Math.min(700, parent.width - 60) : 700
        height: parent ? Math.min(760, parent.height - 60) : 760
        modal: true; title: qsTr("Set layout and optional creative direction")
        closePolicy: Popup.NoAutoClose
        contentItem: ColumnLayout {
            ScrollView {
                Layout.fillWidth: true; Layout.fillHeight: true; clip: true
                contentWidth: availableWidth; ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ColumnLayout {
                    width: parent.width; spacing: 12
                    Label { text: qsTr("Card proportions"); color: Theme.text; font.weight: Font.DemiBold }
                    ShapeComboBox {
                        objectName: "cardKitShape"
                        Layout.fillWidth: true
                        model: [qsTr("Keep current proportions"), qsTr("Portrait · 5:7"), qsTr("Landscape · 7:5"), qsTr("Square · 1:1"), qsTr("Landscape · 3:2"), qsTr("Portrait · 2:3")]
                        currentIndex: {
                            const w = root.composition.canvas_width || 750, h = root.composition.canvas_height || 1050
                            if (w * 7 === h * 5) return 1
                            if (w * 5 === h * 7) return 2
                            if (w === h) return 3
                            if (w * 2 === h * 3) return 4
                            if (w * 3 === h * 2) return 5
                            return 0
                        }
                        onActivated: index => {
                            const sizes = [[0, 0], [750, 1050], [1050, 750], [900, 900], [1200, 800], [800, 1200]]
                            if (index > 0) root.setCanvas(sizes[index][0], sizes[index][1])
                        }
                    }
                    ShapeButton { objectName: "cardKitCustomRatio"; text: qsTr("Custom proportions…"); onClicked: setLayoutDialog.customRatioExpanded = !setLayoutDialog.customRatioExpanded }
                    RowLayout {
                        visible: setLayoutDialog.customRatioExpanded; Layout.fillWidth: true
                        ShapeTextField { id: customWidth; objectName: "cardKitRatioWidth"; Layout.fillWidth: true; placeholderText: qsTr("Width"); text: "3"; validator: DoubleValidator { bottom: 0.001; top: 10000; decimals: 3; notation: DoubleValidator.StandardNotation } }
                        Label { text: ":"; color: Theme.text }
                        ShapeTextField { id: customHeight; objectName: "cardKitRatioHeight"; Layout.fillWidth: true; placeholderText: qsTr("Height"); text: "2"; validator: DoubleValidator { bottom: 0.001; top: 10000; decimals: 3; notation: DoubleValidator.StandardNotation } }
                        ShapeButton { objectName: "cardKitRatioApply"; text: qsTr("Apply proportions"); onClicked: setLayoutDialog.applyCustomRatio() }
                    }
                    Label { Layout.fillWidth: true; visible: setLayoutDialog.ratioError.length > 0; text: setLayoutDialog.ratioError; color: Theme.danger; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; visible: !root.composition.layout; text: qsTr("Choosing new proportions or a flexible layout changes the appearance of these original cards. Their text and images are kept."); color: Theme.warning; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: qsTr("The whole set uses one card size. Print starts with these proportions; choose the finished size in the print layout."); color: Theme.muted; wrapMode: Text.WordWrap }
                    CardLayoutOptions {
                        objectName: "cardKitDefaultLayoutOptions"
                        Layout.fillWidth: true
                        configuration: root.composition.layout || null
                        resolved: root.controller.preview.default_layout || ({})
                        editIdentity: root.backend.projectId + "|" + root.backend.bundlePath + "|" + (root.controller.draft.artifact_id || "") + "|default"
                        editingEnabled: root.visible && setLayoutDialog.visible
                        onEdited: value => root.setField("layout", value)
                    }
                    ShapeButton { text: setLayoutDialog.promptsExpanded ? qsTr("Hide creative direction") : qsTr("Creative direction (optional)…"); onClicked: setLayoutDialog.promptsExpanded = !setLayoutDialog.promptsExpanded }
                    ColumnLayout {
                        visible: setLayoutDialog.promptsExpanded; Layout.fillWidth: true
                        Label { text: qsTr("Shared brief"); color: Theme.muted }
                        ShapeTextEditor { objectName: "cardKitBrief"; Layout.fillWidth: true; Layout.preferredHeight: 95; text: root.composition.brief || ""; placeholderText: qsTr("What are these cards for?"); wrapMode: TextEdit.Wrap; onTextChanged: if (editorActiveFocus) root.setField("brief", text) }
                        Label { text: qsTr("Visual style"); color: Theme.muted }
                        ShapeTextEditor { objectName: "cardKitStyle"; Layout.fillWidth: true; Layout.preferredHeight: 80; text: root.composition.style || ""; placeholderText: qsTr("Palette, mood and illustration style…"); wrapMode: TextEdit.Wrap; onTextChanged: if (editorActiveFocus) root.setField("style", text) }
                    }
                }
            }
            ShapeButton { objectName: "cardKitSetLayoutClose"; Layout.alignment: Qt.AlignRight; text: qsTr("Done"); onClicked: if (root.flush()) setLayoutDialog.close() }
        }
    }
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
            Label { Layout.fillWidth: true; text: qsTr("Paste one card per row, or copy table columns for title, text and image description. A title or body text is enough. A single column keeps every row as a card."); wrapMode: Text.WordWrap; color: Theme.muted }
            ShapeTextEditor { id: listText; objectName: "cardKitListText"; Layout.fillWidth: true; Layout.preferredHeight: 200; wrapMode: TextEdit.NoWrap; placeholderText: qsTr("Paste your rows here…") }
            CheckBox { objectName: "cardKitListHasHeading"; text: qsTr("First row contains column headings"); checked: root.listFirstHeader; onToggled: root.listFirstHeader = checked }
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

pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import Shape.Desktop

ShapeDialog {
    id: root
    objectName: "pictureBookWorkspace"
    required property DesktopBackend backend
    required property PictureBookController controller
    required property MaterialEditController materialEdit
    required property InferImageController inferImage
    required property InferTextController inferText
    required property UiPreferences uiPreferences
    property var draft: ({})
    property string activeLanguage: "en"
    property int pageIndex: 0
    property bool dirty: false
    property bool scriptDirty: false
    property string script: ""
    property string sessionProject: ""
    property string sessionBundle: ""
    property bool checkpointDirty: false
    property bool synchronizing: false
    property bool editingArtwork: false
    property var artworkEdit: ({})
    property int artworkPage: -1
    property var artworkSource: ({})
    property string artworkProject: ""
    property string artworkBundle: ""
    property string artworkPages: ""
    property bool applyingArtwork: false
    property string artworkCandidateImage: ""
    property string generationProject: ""
    property string generationBundle: ""
    property string generationBook: ""
    property int generationPage: -1
    property string generationPages: ""
    property var generationPageSource: ({})
    property string generationArtifact: ""
    property string generationDraft: ""
    property string generationCandidateId: ""
    property string generationImage: ""
    property string generationOriginalImage: ""
    property bool generationStarted: false
    property int generationWidth: 1024
    property int generationHeight: 1024
    property string localError: ""
    readonly property bool sessionCurrent: sessionProject === backend.projectId && sessionBundle === backend.bundlePath
    readonly property bool generationTargetCurrent: generationProject === backend.projectId && generationBundle === backend.bundlePath
        && generationBook === controller.draft.artifact_id && generationPage >= 0
        && JSON.stringify(draft.pages || []) === generationPages
        && generationPage < (draft.pages || []).length
    readonly property var generationCandidate: backend.candidates.find(c => c.id === generationCandidateId && c.artifactId === generationArtifact) || null
    readonly property bool ownGeneration: generationStarted && inferImage.running && generationTargetCurrent
    readonly property var artworkCandidate: backend.candidates.find(c => c.artifactId === (artworkEdit.contextArtifactId || "")) || null
    readonly property var artworkStage: backend.operatorDrafts.find(d => d.id === artworkEdit.id
        && d.contextArtifactId === artworkEdit.contextArtifactId && d.operatorTypeKey === "image.external_edit") || null
    readonly property bool artworkStageCurrent: artworkProject === backend.projectId && artworkBundle === backend.bundlePath && artworkStage !== null
        && artworkStage.inputArtifactId === artworkSource.artifact_id && artworkStage.inputRevisionId === artworkSource.revision_id
    readonly property var books: {
        const result = [{name: qsTr("Continue saved books…"), id: ""}], seen = {}
        for (const item of controller.drafts) {
            result.push({name: item.state.composition.name, id: item.artifact_id}); seen[item.artifact_id] = true
        }
        for (const item of backend.artifacts)
            if (item.kindKey === "picture_book" && item.hasAcceptedRevision && !seen[item.id]) result.push(item)
        return result
    }
    readonly property var texts: [{name: qsTr("Choose a text material…"), id: ""}].concat(backend.artifacts.filter(a => a.kindKey === "text_document" && a.hasAcceptedRevision))
    readonly property var images: [{name: qsTr("Blank page"), id: ""}].concat(backend.artifacts.filter(a => a.kindKey === "image_raster" && a.hasAcceptedRevision))
    readonly property var currentPage: draft.pages && draft.pages.length > 0 ? draft.pages[Math.min(pageIndex, draft.pages.length - 1)] : ({kind:"blank", print_only:false})
    signal materialRequested()
    signal editMaterialRequested(string artifactId)
    signal inferAccessSetupRequested()
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: parent ? parent.width - 28 : 1200
    height: parent ? parent.height - 40 : 820
    modal: true
    title: qsTr("Picture book")
    closePolicy: Popup.NoAutoClose

    function copy(value) { return JSON.parse(JSON.stringify(value)) }
    function samePin(left, right) { return !!left && !!right && left.artifact_id === right.artifact_id && left.revision_id === right.revision_id }
    function pin(item) { return {artifact_id:item.id, revision_id:item.acceptedRevisionId} }
    function changed(next) {
        if (artworkEdit.id && JSON.stringify(next.pages || []) !== artworkPages) {
            artworkEdit = ({}); editingArtwork = false; artworkPages = ""
            if (!applyingArtwork) localError = qsTr("The page sequence changed. Reopen artwork editing for the page you want to adjust.")
        }
        draft = next
        dirty = true
        controller.invalidate()
        checkpoint()
    }
    function workspaceState() {
        return {composition:copy(draft), active_language:activeLanguage, page_index:pageIndex,
            script:script, script_dirty:scriptDirty,
            artwork_edit:artworkEdit.contextArtifactId && artworkSource.artifact_id
                ? {output_artifact_id:artworkEdit.contextArtifactId, page_index:artworkPage, source:copy(artworkSource)} : null}
    }
    function checkpoint() {
        if (synchronizing || !controller.draft.artifact_id || !sessionCurrent) return
        checkpointDirty = true; saveTimer.restart()
    }
    function flush() {
        saveTimer.stop()
        if (!checkpointDirty) return true
        if (!sessionCurrent || !controller.saveState(workspaceState())) return false
        checkpointDirty = false; return true
    }
    function newBook() { if (flush()) controller.beginNew() }
    function selectBook(artifactId) { if (artifactId && flush()) controller.load(artifactId) }
    function discardDraft() { saveTimer.stop(); if (controller.discardDraft()) checkpointDirty = false }
    function setField(key, value) {
        const next = copy(draft); next[key] = value; changed(next)
    }
    function loadScript() {
        const source = draft.languages ? draft.languages[activeLanguage] : null
        script = source ? controller.scriptText(source) : "[Cover]\n\n[Author]\n\n[1]\n"
        scriptDirty = false
    }
    function chooseLanguage(language) {
        if (scriptDirty || !language.trim().length) return false
        activeLanguage = language.trim()
        loadScript()
        if (controller.previewSource.length && (draft.languages || {})[activeLanguage])
            controller.showPage(activeLanguage, pageIndex)
        return true
    }
    function selectText(index) {
        if (scriptDirty || index < 1 || index >= texts.length) return false
        const next = copy(draft)
        next.languages[activeLanguage] = pin(texts[index])
        if (!next.languages[next.default_language]) next.default_language = activeLanguage
        changed(next)
        loadScript()
        return true
    }
    function refreshMaterials() {
        if (scriptDirty) return false
        const previous = (draft.languages || {})[activeLanguage] || {}
        const result = controller.refreshMaterials(draft)
        if (!result.changed) return false
        changed(result.composition)
        const current = (draft.languages || {})[activeLanguage] || {}
        if (previous.artifact_id !== current.artifact_id || previous.revision_id !== current.revision_id)
            loadScript()
        return true
    }
    function synchronize() {
        synchronizing = true
        if (generationBook && generationBook !== controller.draft.artifact_id) clearGeneration()
        editingArtwork = false
        artworkEdit = ({})
        const saved = controller.draft.state || {}
        draft = copy(saved.composition || controller.composition)
        activeLanguage = saved.active_language || draft.default_language || "en"
        pageIndex = saved.page_index || 0
        dirty = controller.hasDraftChanges
        script = saved.script !== undefined ? saved.script : "[Cover]\n\n[Author]\n\n[1]\n"
        scriptDirty = saved.script_dirty || false
        const edit = saved.artwork_edit
        if (edit && edit.page_index < (draft.pages || []).length
            && samePin(draft.pages[edit.page_index].source, edit.source)) {
            const stage = backend.operatorDrafts.find(d => d.contextArtifactId === edit.output_artifact_id && d.operatorTypeKey === "image.external_edit")
            if (stage && stage.inputArtifactId === edit.source.artifact_id
                && stage.inputRevisionId === edit.source.revision_id) {
                artworkEdit = stage; artworkPage = edit.page_index; artworkSource = copy(edit.source); artworkProject = backend.projectId; artworkBundle = backend.bundlePath
                artworkPages = JSON.stringify(draft.pages || [])
                materialEdit.restoreShadowSession(backend.bundlePath, stage.contextArtifactId)
            }
        }
        checkpointDirty = false
        synchronizing = false
    }
    function openWorkspace(bookId) {
        if (!sessionCurrent) {
            sessionProject = backend.projectId
            sessionBundle = backend.bundlePath
            checkpointDirty = false
        }
        if (!flush() || !controller.refresh()) return
        if (bookId && bookId.length > 0) { if (!controller.load(bookId)) return }
        else if (controller.draft.artifact_id) { if (!controller.load(controller.draft.artifact_id)) return }
        else if (books.length > 1) { if (!controller.load(books[1].id)) return }
        else controller.beginNew()
        open()
    }
    function choosePage(index) {
        pageIndex = Math.max(0, Math.min(index, (draft.pages || []).length - 1))
        if (controller.previewSource.length > 0) controller.showPage(activeLanguage, pageIndex)
    }
    function selectImage(index) {
        const next = copy(draft)
        const item = images[index]
        next.pages[pageIndex] = item.id.length ? {kind:"image", source:pin(item), print_only:currentPage.print_only || false} : {kind:"blank", print_only:currentPage.print_only || false}
        changed(next)
    }
    function imageIndex() {
        if (!currentPage.source) return 0
        for (let i=1; i<images.length; ++i) if (images[i].id === currentPage.source.artifact_id) return i
        return 0
    }
    function movePage(delta) {
        const next = copy(draft), to = pageIndex + delta
        if (to < 0 || to >= next.pages.length) return
        const item = next.pages.splice(pageIndex, 1)[0]; next.pages.splice(to, 0, item)
        next.pages[0].print_only = false
        changed(next); pageIndex = to
    }
    function beginArtworkEdit() {
        if (!sessionCurrent || !currentPage.source) return false
        if (artworkProject !== backend.projectId || artworkBundle !== backend.bundlePath || artworkPage !== pageIndex
            || !samePin(artworkSource, currentPage.source) || !artworkStageCurrent
            || !artworkEdit.id) {
            const stage = controller.beginArtworkEdit(currentPage.source)
            if (!stage.id) return false
            artworkEdit = stage
            artworkPage = pageIndex
            artworkSource = copy(currentPage.source)
            artworkProject = backend.projectId
            artworkBundle = backend.bundlePath
            artworkPages = JSON.stringify(draft.pages || [])
        }
        editingArtwork = true
        materialEdit.restoreShadowSession(backend.bundlePath, artworkEdit.contextArtifactId)
        checkpoint()
        updateArtworkPreview()
        return true
    }
    function updateArtworkPreview() {
        artworkCandidateImage = artworkCandidate && artworkProject === backend.projectId && artworkBundle === backend.bundlePath
            ? controller.artworkCandidatePreview(artworkEdit.contextArtifactId, artworkCandidate.id) : ""
    }
    function adoptArtwork() {
        const page = (draft.pages || [])[artworkPage]
        if (!editingArtwork || !artworkCandidate || !artworkStageCurrent
            || JSON.stringify(draft.pages || []) !== artworkPages
            || !page || !samePin(page.source, artworkSource)) return false
        const result = controller.adoptArtwork(artworkEdit.contextArtifactId, artworkCandidate.id)
        if (!result.artifact_id) return false
        const next = copy(draft)
        next.pages[artworkPage].source = result
        applyingArtwork = true; changed(next); applyingArtwork = false
        pageIndex = artworkPage
        editingArtwork = false
        artworkEdit = ({})
        return true
    }
    function clearGeneration() {
        generationProject = ""; generationBundle = ""; generationBook = ""; generationPage = -1; generationPages = ""
        generationArtifact = ""; generationDraft = ""; generationCandidateId = ""
        generationImage = ""; generationOriginalImage = ""; generationStarted = false
        generationDialog.close()
    }
    function currentPageScript() {
        const tags = /\[(cover|封面|author|作者|title|扉页|[0-9]+)\]/gi
        const blocks = []; let match
        while ((match = tags.exec(script)) !== null) {
            const tag = /^[0-9]+$/.test(match[1]) ? String(Number(match[1])) : match[1].toLowerCase()
            blocks.push({tag:tag, start:tags.lastIndex, end:match.index})
        }
        const hasTitle = blocks.some(b => b.tag === "title" || b.tag === "扉页")
        const expected = pageIndex === 0 ? ["cover", "封面"] : pageIndex === 1 && hasTitle
            ? ["title", "扉页"] : [String(pageIndex - (hasTitle ? 1 : 0))]
        const matches = blocks.map((b, index) => ({tag:b.tag, text:script.slice(b.start, index + 1 < blocks.length ? blocks[index + 1].end : script.length).trim()})).filter(b => expected.indexOf(b.tag) >= 0)
        // This is editable prompt context only. The core remains the script validation owner.
        return matches.length === 1 ? matches[0].text : script
    }
    function openArtworkGeneration() {
        if (!sessionCurrent || !flush()) return false
        if (generationTargetCurrent && generationPage === pageIndex) { generationDialog.open(); return true }
        if (inferImage.running) { localError = qsTr("Finish or stop the current image run before starting another."); return false }
        clearGeneration(); localError = ""
        generationProject = backend.projectId; generationBundle = backend.bundlePath; generationBook = controller.draft.artifact_id
        generationPage = pageIndex; generationPages = JSON.stringify(draft.pages || [])
        generationPageSource = copy(currentPage.source || {})
        generationOriginalImage = controller.artworkPreview(generationPageSource)
        const ratio = (draft.canvas_width || 1024) / (draft.canvas_height || 768)
        generationWidth = ratio > 1.15 ? 1536 : 1024; generationHeight = ratio < 0.87 ? 1536 : 1024
        generationPrompt.text = [qsTr("Picture-book illustration for page %1. Keep lettering and captions outside the artwork.").arg(pageIndex + 1),
            draft.name || "", qsTr("Page script:\n%1").arg(currentPageScript())].join("\n\n")
        generationDialog.open(); return true
    }
    function generateArtwork() {
        if (!generationTargetCurrent || inferImage.running || !generationPrompt.text.trim().length) return false
        if (!generationArtifact.length) {
            const stage = backend.beginAiImageDraft((draft.name || qsTr("Book")) + " · " + (generationPage + 1), generationPrompt.text, generationWidth, generationHeight)
            if (!stage.id || !stage.contextArtifactId) return false
            generationArtifact = stage.contextArtifactId; generationDraft = stage.id
        }
        if (!backend.updateAiImageDraft(generationDraft, generationPrompt.text, generationWidth, generationHeight, 1)) return false
        generationStarted = true
        inferImage.generate(backend.bundlePath, generationArtifact, generationDraft, generationModel.effectiveModelKey, generationModel.effectiveEffortKey, 1)
        return true
    }
    function updateGenerationPreview() {
        generationImage = generationTargetCurrent && generationCandidate
            ? controller.artworkCandidatePreview(generationArtifact, generationCandidate.id) : ""
    }
    function adoptGeneratedArtwork() {
        if (!generationTargetCurrent || !generationCandidate || inferImage.running || !flush()) return false
        const accepted = controller.adoptArtwork(generationArtifact, generationCandidate.id)
        if (!accepted.artifact_id) return false
        const next = copy(draft), index = generationPage
        next.pages[index] = {kind:"image", source:accepted, print_only:next.pages[index].print_only || false}
        changed(next); pageIndex = index; flush(); clearGeneration(); return true
    }
    onArtworkCandidateChanged: if (editingArtwork) updateArtworkPreview()
    onGenerationCandidateChanged: updateGenerationPreview()
    onPageIndexChanged: checkpoint()
    onActiveLanguageChanged: checkpoint()
    onScriptChanged: checkpoint()
    onScriptDirtyChanged: checkpoint()
    Connections {
        target: root.controller
        function onDocumentChanged() { root.synchronize() }
    }
    Connections {
        target: root.inferImage
        function onCandidateCreated(candidateId, artifactId) {
            if (root.generationTargetCurrent && root.generationStarted && artifactId === root.generationArtifact)
                root.generationCandidateId = candidateId
        }
    }
    onClosed: { flush(); controller.invalidate() }
    Component.onDestruction: if (sessionCurrent) flush()
    Connections {
        target: root.backend
        function onProjectChanged() {
            if (!root.sessionCurrent) {
                saveTimer.stop(); root.checkpointDirty = false
                nativeExport.close(); pdfExport.close(); root.clearGeneration(); root.close()
                root.editingArtwork = false; root.artworkEdit = ({})
            }
        }
    }
    Timer { id: saveTimer; interval: 500; onTriggered: root.flush() }
    function exportContext() {
        return {project:backend.projectId, bundle:backend.bundlePath, source:copy(controller.source)}
    }
    function exportContextCurrent(context) {
        return sessionCurrent && context && context.project === backend.projectId && context.bundle === backend.bundlePath
            && !!context.source && !!context.source.artifact_id && samePin(context.source, controller.source)
            && context.source.artifact_id === controller.draft.artifact_id
    }
    function beginNativeExport() { nativeExport.requestContext = exportContext(); nativeExport.open() }
    function completeNativeExport(file, context) { return exportContextCurrent(context) && controller.exportNative(file) }
    function beginPdfExport() {
        pdfExport.requestContext = exportContext(); pdfExport.language = activeLanguage
        pdfExport.includePrintOnly = includePrint.checked; pdfExport.open()
    }
    function completePdfExport(file, context, language, printOnly) {
        return exportContextCurrent(context) && controller.exportPdf(file, language, printOnly)
    }
    FileDialog {
        id: nativeExport
        objectName: "pictureBookNativeExportDialog"
        property var requestContext: ({})
        title: qsTr("Export editable Storybook project")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "scproj"
        nameFilters: [qsTr("Storybook project (*.scproj)")]
        onAccepted: root.completeNativeExport(selectedFile, requestContext)
    }
    ShapeDialog {
        id: generationDialog
        objectName: "pictureBookArtworkGenerationDialog"
        parent: Overlay.overlay; anchors.centerIn: parent
        width: parent ? parent.width - 60 : 1180; height: parent ? parent.height - 60 : 760
        modal: true; title: qsTr("Create artwork for this page"); closePolicy: Popup.NoAutoClose
        contentItem: RowLayout {
            ColumnLayout {
                Layout.preferredWidth: 340; Layout.maximumWidth: 380; Layout.fillHeight: true
                Label { Layout.fillWidth: true; text: qsTr("The current page script is included. Adjust the description, then review a generated version before adopting it."); color: Theme.muted; wrapMode: Text.WordWrap }
                ShapeTextEditor { id: generationPrompt; objectName: "pictureBookArtworkGenerationPrompt"; Layout.fillWidth: true; Layout.fillHeight: true; wrapMode: TextEdit.Wrap; readOnly: root.ownGeneration }
                AiModelPicker { id: generationModel; Layout.fillWidth: true; family: "image"; onChoiceSelected: key => overrideKey = key; onEffortSelected: key => effortOverrideKey = key; defaultModelKey: root.uiPreferences.imageModel; defaultEffortKey: root.uiPreferences.imageEffort; enabled: !root.inferImage.running }
                Label { Layout.fillWidth: true; visible: root.ownGeneration || (root.generationStarted && root.inferImage.errorCode.length > 0); text: root.ownGeneration ? qsTr("Creating artwork…") : qsTr("Could not generate this page. Check image access and try again."); color: Theme.muted; wrapMode: Text.WordWrap }
                Label { Layout.fillWidth: true; visible: !root.generationTargetCurrent; text: qsTr("This page changed while the image was being prepared. Reopen generation from the page to choose a new target."); color: Theme.warning; wrapMode: Text.WordWrap }
                ShapeButton { objectName: "pictureBookArtworkGenerate"; Layout.fillWidth: true; text: qsTr("Generate a version"); primary: true; enabled: root.generationTargetCurrent && root.inferText.credentialConfigured && !root.inferImage.running && generationPrompt.text.trim().length > 0; onClicked: root.generateArtwork() }
                ShapeButton { text: qsTr("Stop after current image"); visible: root.ownGeneration; onClicked: root.inferImage.stopAfterCurrent() }
                Label { Layout.fillWidth: true; visible: !root.inferText.credentialConfigured; text: qsTr("Add the Shape Infer credential in Settings before generating."); color: Theme.warning; wrapMode: Text.WordWrap }
                ShapeButton { text: qsTr("Open Settings"); visible: !root.inferText.credentialConfigured; onClicked: root.inferAccessSetupRequested() }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true
                ImageCompareWorkspace { Layout.fillWidth: true; Layout.fillHeight: true; acceptedSource: root.generationOriginalImage; candidateSource: root.generationImage }
                ShapeButton { objectName: "adoptPictureBookGeneratedArtwork"; text: qsTr("Adopt image for this page"); primary: true; enabled: root.generationTargetCurrent && root.generationCandidate !== null && !root.inferImage.running; onClicked: root.adoptGeneratedArtwork() }
                ShapeButton { text: qsTr("Back to current page"); onClicked: generationDialog.close() }
            }
        }
    }
    FileDialog {
        id: pdfExport
        objectName: "pictureBookPdfExportDialog"
        property var requestContext: ({})
        property string language: ""
        property bool includePrintOnly: false
        title: qsTr("Export adopted reading layout as PDF")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "pdf"
        nameFilters: [qsTr("PDF document (*.pdf)")]
        onAccepted: root.completePdfExport(selectedFile, requestContext, language, includePrintOnly)
    }
    contentItem: ColumnLayout {
        spacing: 10
        ColumnLayout {
            visible: root.editingArtwork
            Layout.fillWidth: true
            Layout.fillHeight: true
            RowLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Artwork for page %1").arg(root.artworkPage + 1); color: Theme.text; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
                ShapeButton { objectName: "returnToPictureBookPage"; text: qsTr("Back to current page"); onClicked: root.editingArtwork = false }
            }
            MaterialEditWorkspace {
                objectName: "pictureBookMaterialEdit"
                Layout.fillWidth: true
                Layout.fillHeight: true
                backend: root.backend
                controller: root.materialEdit
                audioPreview: null
                projectPath: root.backend.bundlePath
                outputArtifactId: root.artworkEdit.contextArtifactId || ""
                operatorDraftId: root.artworkEdit.id || ""
                readonly property var stage: root.backend.operatorDrafts.find(d => d.id === operatorDraftId) || root.artworkEdit
                inputArtifactId: stage.inputArtifactId || ""
                inputRevisionId: stage.inputRevisionId || ""
                readonly property var input: root.images.find(a => a.id === inputArtifactId)
                inputCurrent: !!input && input.acceptedRevisionId === inputRevisionId
                sourceName: input ? input.name : ""
                candidate: root.artworkCandidate
                acceptedImageSource: outputArtifactId.length && inputRevisionId.length ? root.backend.materialSourceImage(outputArtifactId) : ""
                candidateImageSource: root.artworkCandidateImage
            }
            Label { Layout.fillWidth: true; text: qsTr("Adopting saves the edited image and changes only this page’s artwork draft. Review and adopt the book separately to save its new version."); wrapMode: Text.WordWrap; color: Theme.muted }
            ShapeButton { objectName: "adoptPictureBookArtwork"; text: qsTr("Adopt image for this page"); primary: true; enabled: root.artworkStageCurrent && !!root.artworkCandidate && !root.materialEdit.running && !root.materialEdit.shadowActive; onClicked: root.adoptArtwork() }
        }
        ColumnLayout {
        visible: !root.editingArtwork
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            ShapeComboBox {
                id: bookPicker
                objectName: "pictureBookPicker"
                Layout.preferredWidth: 270
                model: root.books
                textRole: "name"
                displayText: currentIndex >= 0 ? currentText : qsTr("Saved picture books")
                onActivated: index => root.selectBook(root.books[index].id)
            }
            ShapeButton { objectName: "newPictureBook"; text: qsTr("New book"); onClicked: root.newBook() }
            ShapeTextField {
                objectName: "pictureBookName"
                Layout.fillWidth: true
                text: root.draft.name || ""
                placeholderText: qsTr("Book name")
                onTextEdited: root.setField("name", text)
            }
            ShapeButton { text: qsTr("Import artwork…"); onClicked: root.materialRequested() }
            Label { text: root.checkpointDirty ? (root.controller.error.length ? qsTr("Draft not saved") : qsTr("Saving draft…")) : qsTr("Draft saved"); color: Theme.muted; font.pixelSize: 11 }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: qsTr("Canvas"); color: Theme.muted }
            SpinBox { from: 256; to: 8192; value: root.draft.canvas_width || 1024; editable: true; onValueModified: root.setField("canvas_width",value) }
            Label { text: "×"; color: Theme.muted }
            SpinBox { from: 256; to: 8192; value: root.draft.canvas_height || 768; editable: true; onValueModified: root.setField("canvas_height",value) }
            Label { text: qsTr("Text size"); color: Theme.muted }
            SpinBox { from: 12; to: 96; value: root.draft.font_size || 26; onValueModified: root.setField("font_size",value) }
            Item { Layout.fillWidth: true }
            ShapeButton { objectName: "refreshPictureBookMaterials"; text: qsTr("Use current material versions"); enabled: !root.scriptDirty; onClicked: root.refreshMaterials() }
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 14
            ColumnLayout {
                Layout.preferredWidth: 210
                Layout.minimumWidth: 190
                Layout.maximumWidth: 225
                Layout.fillHeight: true
                Label { text: qsTr("Shared page sequence"); font.weight: Font.DemiBold; color: Theme.text }
                ListView {
                    id: pages
                    objectName: "pictureBookPages"
                    Layout.fillWidth: true; Layout.minimumWidth: 0
                    Layout.fillHeight: true
                    clip: true
                    spacing: 5
                    model: root.draft.pages || []
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        width: ListView.view.width
                        height: 52
                        radius: 7
                        color: index === root.pageIndex ? Theme.accentSoft : Theme.panel
                        border.color: index === root.pageIndex ? Theme.accent : Theme.border
                        Column {
                            anchors.left: parent.left; anchors.leftMargin: 12; anchors.verticalCenter: parent.verticalCenter
                            Label { text: index === 0 ? qsTr("Cover") : index === 1 && root.controller.hasTitlePage ? qsTr("Title page") : qsTr("Page %1").arg(index + 1); color: Theme.text }
                            Label { text: modelData.print_only ? qsTr("Print only") : modelData.kind === "image" ? qsTr("Artwork") : qsTr("Blank / text page"); color: Theme.muted; font.pixelSize: 11 }
                        }
                        TapHandler { onTapped: root.choosePage(parent.index) }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    ShapeButton { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 40; text: "+"; enabled: (root.draft.pages || []).length < 64; onClicked: { const next=root.copy(root.draft); next.pages.push({kind:"blank",print_only:false}); root.changed(next); root.choosePage(next.pages.length-1) } }
                    ShapeButton { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 40; text: "↑"; enabled: root.pageIndex > 0; onClicked: root.movePage(-1) }
                    ShapeButton { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 40; text: "↓"; enabled: root.pageIndex < (root.draft.pages || []).length-1; onClicked: root.movePage(1) }
                    ShapeButton { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 40; text: "−"; enabled: (root.draft.pages || []).length > 1; onClicked: { const next=root.copy(root.draft); next.pages.splice(root.pageIndex,1); next.pages[0].print_only=false; root.changed(next); root.choosePage(root.pageIndex) } }
                }
                ShapeComboBox {
                    objectName: "pictureBookArtwork"
                    Layout.fillWidth: true; Layout.minimumWidth: 0
                    model: root.images
                    textRole: "name"
                    currentIndex: root.imageIndex()
                    onActivated: index => root.selectImage(index)
                }
                CheckBox {
                    text: qsTr("Print-only page")
                    enabled: root.pageIndex > 0
                    checked: root.currentPage.print_only || false
                    onToggled: { const next=root.copy(root.draft); next.pages[root.pageIndex].print_only=checked; root.changed(next) }
                }
                ShapeButton {
                    objectName: "generatePictureBookArtwork"
                    Layout.fillWidth: true; Layout.minimumWidth: 0
                    text: qsTr("Generate artwork for this page…")
                    onClicked: root.openArtworkGeneration()
                }
                ShapeButton {
                    objectName: "editPictureBookArtwork"
                    Layout.fillWidth: true; Layout.minimumWidth: 0
                    text: qsTr("Adjust this page in Shadow")
                    enabled: !!root.currentPage.source
                    onClicked: root.beginArtworkEdit()
                }
                Label { Layout.fillWidth: true; Layout.minimumWidth: 0; text: qsTr("Page tags keep their positions when artwork is reordered. An edited image returns to this page after you adopt it."); wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 11 }
            }
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Rectangle {
                    objectName: "pictureBookPreview"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Theme.panel
                    border.color: Theme.border
                    radius: 10
                    Image { anchors.fill: parent; anchors.margins: 12; source: root.controller.previewSource; fillMode: Image.PreserveAspectFit; cache: false }
                    Label {
                        anchors.centerIn: parent
                        width: parent.width-60
                        visible: root.controller.previewSource.length === 0
                        text: qsTr("Add a language script and page artwork, then review the book.")
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        color: Theme.muted
                    }
                }
                Label { Layout.fillWidth: true; text: root.controller.status; wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 11 }
            }
            ColumnLayout {
                Layout.preferredWidth: 320
                Layout.minimumWidth: 280
                Layout.maximumWidth: 340
                Layout.fillHeight: true
                Label { text: qsTr("Language script"); font.weight: Font.DemiBold; color: Theme.text }
                RowLayout {
                    ShapeComboBox {
                        objectName: "pictureBookLanguagePicker"
                        Layout.fillWidth: true
                        model: Object.keys(root.draft.languages || {})
                        currentIndex: model.indexOf(root.activeLanguage)
                        enabled: !root.scriptDirty
                        onActivated: index => root.chooseLanguage(model[index])
                    }
                    ShapeTextField { id: languageCode; objectName: "pictureBookLanguageCode"; Layout.preferredWidth: 88; text: root.activeLanguage; placeholderText: "zh-CN"; enabled: !root.scriptDirty }
                    ShapeButton { objectName: "addPictureBookLanguage"; text: "+"; enabled: !root.scriptDirty; onClicked: root.chooseLanguage(languageCode.text) }
                }
                CheckBox {
                    text: qsTr("Default reading language")
                    checked: root.activeLanguage === root.draft.default_language
                    enabled: !!(root.draft.languages || {})[root.activeLanguage]
                    onClicked: root.setField("default_language",root.activeLanguage)
                }
                ShapeComboBox {
                    objectName: "pictureBookTextMaterial"
                    Layout.fillWidth: true
                    model: root.texts
                    textRole: "name"
                    currentIndex: 0
                    enabled: !root.scriptDirty
                    onActivated: index => root.selectText(index)
                }
                Label {
                    objectName: "pictureBookPendingScriptHint"
                    Layout.fillWidth: true
                    visible: root.scriptDirty
                    text: qsTr("Adopt this script or discard the draft before switching languages, choosing text, or refreshing materials.")
                    wrapMode: Text.WordWrap
                    color: Theme.muted
                    font.pixelSize: 11
                }
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    TextArea {
                        id: scriptEditor
                        objectName: "pictureBookScript"
                        text: root.script
                        color: Theme.text
                        wrapMode: TextEdit.Wrap
                        selectByMouse: true
                        font.pixelSize: 13
                        onTextChanged: if (text !== root.script) { root.script = text; root.scriptDirty = true; root.dirty = true; root.controller.invalidate() }
                    }
                }
                ShapeButton {
                    objectName: "adoptPictureBookScript"
                    Layout.fillWidth: true
                    text: qsTr("Adopt script as new material")
                    enabled: root.activeLanguage.length > 0 && (root.scriptDirty || !(root.draft.languages || {})[root.activeLanguage])
                    onClicked: {
                        const source=(root.draft.languages || {})[root.activeLanguage] || {}
                        const result=root.controller.adoptScript((root.draft.name || qsTr("Book")) + " · " + root.activeLanguage,source,root.script)
                        if (!result.artifact_id) return
                        const next=root.copy(root.draft); next.languages[root.activeLanguage]=result; if (!next.languages[next.default_language]) next.default_language=root.activeLanguage; root.changed(next); root.loadScript()
                    }
                }
                Label { Layout.fillWidth: true; text: qsTr("[Cover] / [封面], [Author] / [作者], optional [Title] / [扉页], then [1], [2]… All languages share title-page presence. Missing blocks stay textless."); wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 11 }
            }
        }
        Label { Layout.fillWidth: true; visible: text.length > 0; text: root.localError || root.controller.error; color: Theme.danger; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            ShapeButton { objectName: "reviewPictureBook"; text: qsTr("Review book"); primary: !root.controller.hasReview; enabled: !root.scriptDirty && Object.keys(root.draft.languages || {}).length > 0; onClicked: { if(root.flush() && root.controller.review(root.draft)) root.controller.showPage(root.activeLanguage,root.pageIndex) } }
            ShapeButton { objectName: "acceptPictureBook"; text: qsTr("Adopt book version"); primary: true; enabled: root.controller.hasReview; onClicked: root.controller.accept() }
            ShapeButton { objectName: "discardPictureBookDraft"; text: qsTr("Discard draft"); onClicked: root.discardDraft() }
            Item { Layout.fillWidth: true }
            CheckBox { id: includePrint; text: qsTr("Include print-only pages"); checked: false }
            ShapeButton { text: qsTr("PDF…"); enabled: root.controller.hasAccepted && !root.dirty; onClicked: root.beginPdfExport() }
            ShapeButton { text: ".scproj…"; enabled: root.controller.hasAccepted && !root.dirty; onClicked: root.beginNativeExport() }
            ShapeButton { objectName: "continueInStorybook"; text: qsTr("Continue in Storybook"); visible: root.controller.nativeExport.toString().length > 0; onClicked: root.controller.revealNativeExport() }
            ShapeButton { text: qsTr("Return to project"); onClicked: if (root.flush()) root.close() }
        }
        Label { Layout.fillWidth: true; text: qsTr("Drafts and unfinished scripts are saved in this project. Adopt a book version to export it. Storybook keeps its own editable typography and print tools."); color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap }
        Label { Layout.fillWidth: true; visible: root.controller.nativeExport.toString().length > 0; text: qsTr("Open the exported .scproj in Storybook to refine multilingual text, soft backdrops and print layout."); color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap }
        }
    }
}

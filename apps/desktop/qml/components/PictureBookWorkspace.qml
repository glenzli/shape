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
    property var draft: ({})
    property string activeLanguage: "en"
    property int pageIndex: 0
    property bool dirty: false
    property bool scriptDirty: false
    property string script: ""
    property string sessionProject: ""
    readonly property var books: backend.artifacts.filter(a => a.kindKey === "picture_book" && a.hasAcceptedRevision)
    readonly property var texts: [{name: qsTr("Choose a text material…"), id: ""}].concat(backend.artifacts.filter(a => a.kindKey === "text_document" && a.hasAcceptedRevision))
    readonly property var images: [{name: qsTr("Blank page"), id: ""}].concat(backend.artifacts.filter(a => a.kindKey === "image_raster" && a.hasAcceptedRevision))
    readonly property var currentPage: draft.pages && draft.pages.length > 0 ? draft.pages[Math.min(pageIndex, draft.pages.length - 1)] : ({kind:"blank", print_only:false})
    signal materialRequested()
    signal editMaterialRequested(string artifactId)
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: parent ? parent.width - 28 : 1200
    height: parent ? parent.height - 40 : 820
    modal: true
    title: qsTr("Picture book")
    closePolicy: Popup.NoAutoClose

    function copy(value) { return JSON.parse(JSON.stringify(value)) }
    function pin(item) { return {artifact_id:item.id, revision_id:item.acceptedRevisionId} }
    function changed(next) {
        draft = next
        dirty = true
        controller.invalidate()
    }
    function setField(key, value) {
        const next = copy(draft); next[key] = value; changed(next)
    }
    function loadScript() {
        const source = draft.languages ? draft.languages[activeLanguage] : null
        script = source ? controller.scriptText(source) : "[Cover]\n\n[Author]\n\n[1]\n"
        scriptDirty = false
    }
    function synchronize() {
        draft = copy(controller.composition)
        activeLanguage = draft.default_language || "en"
        pageIndex = 0
        dirty = !controller.hasAccepted
        loadScript()
    }
    function openWorkspace(bookId) {
        if (sessionProject !== backend.projectId) {
            sessionProject = backend.projectId
            controller.beginNew()
            synchronize()
        }
        if (bookId && bookId.length > 0) controller.load(bookId)
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
    Connections {
        target: root.controller
        function onDocumentChanged() { root.synchronize() }
    }
    onClosed: controller.invalidate()
    FileDialog {
        id: nativeExport
        title: qsTr("Export editable Storybook project")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "scproj"
        nameFilters: [qsTr("Storybook project (*.scproj)")]
        onAccepted: root.controller.exportNative(selectedFile)
    }
    FileDialog {
        id: pdfExport
        title: qsTr("Export adopted reading layout as PDF")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "pdf"
        nameFilters: [qsTr("PDF document (*.pdf)")]
        onAccepted: root.controller.exportPdf(selectedFile, root.activeLanguage, includePrint.checked)
    }
    contentItem: ColumnLayout {
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
                onActivated: index => root.controller.load(root.books[index].id)
            }
            ShapeButton { objectName: "newPictureBook"; text: qsTr("New book"); onClicked: root.controller.beginNew() }
            ShapeTextField {
                objectName: "pictureBookName"
                Layout.fillWidth: true
                text: root.draft.name || ""
                placeholderText: qsTr("Book name")
                onTextEdited: root.setField("name", text)
            }
            ShapeButton { text: qsTr("Import artwork…"); onClicked: root.materialRequested() }
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
            ShapeButton { objectName: "refreshPictureBookMaterials"; text: qsTr("Use current material versions"); onClicked: { root.changed(root.controller.refreshMaterials(root.draft)); root.loadScript() } }
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
                    Layout.fillWidth: true; Layout.minimumWidth: 0
                    text: qsTr("Edit artwork material")
                    enabled: !!root.currentPage.source
                    onClicked: root.editMaterialRequested(root.currentPage.source.artifact_id)
                }
                Label { Layout.fillWidth: true; Layout.minimumWidth: 0; text: qsTr("Page tags keep their positions when artwork is reordered. Bind an adopted edited material to continue."); wrapMode: Text.WordWrap; color: Theme.muted; font.pixelSize: 11 }
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
                        Layout.fillWidth: true
                        model: Object.keys(root.draft.languages || {})
                        currentIndex: model.indexOf(root.activeLanguage)
                        onActivated: index => { root.activeLanguage = model[index]; root.loadScript(); if (root.controller.previewSource.length) root.controller.showPage(root.activeLanguage,root.pageIndex) }
                    }
                    ShapeTextField { id: languageCode; Layout.preferredWidth: 88; text: root.activeLanguage; placeholderText: "zh-CN" }
                    ShapeButton { text: "+"; onClicked: { root.activeLanguage=languageCode.text.trim(); root.loadScript() } }
                }
                CheckBox {
                    text: qsTr("Default reading language")
                    checked: root.activeLanguage === root.draft.default_language
                    enabled: !!(root.draft.languages || {})[root.activeLanguage]
                    onClicked: root.setField("default_language",root.activeLanguage)
                }
                ShapeComboBox {
                    Layout.fillWidth: true
                    model: root.texts
                    textRole: "name"
                    currentIndex: 0
                    onActivated: index => {
                        if (index < 1) return
                        const next=root.copy(root.draft); next.languages[root.activeLanguage]=root.pin(root.texts[index]); if (!next.languages[next.default_language]) next.default_language=root.activeLanguage; root.changed(next); root.loadScript()
                    }
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
        Label { Layout.fillWidth: true; visible: text.length > 0; text: root.controller.error; color: Theme.danger; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            ShapeButton { objectName: "reviewPictureBook"; text: qsTr("Review book"); primary: !root.controller.hasReview; enabled: !root.scriptDirty && Object.keys(root.draft.languages || {}).length > 0; onClicked: { if(root.controller.review(root.draft)) root.controller.showPage(root.activeLanguage,root.pageIndex) } }
            ShapeButton { objectName: "acceptPictureBook"; text: qsTr("Adopt book version"); primary: true; enabled: root.controller.hasReview; onClicked: root.controller.accept() }
            ShapeButton { objectName: "discardPictureBookDraft"; text: qsTr("Discard draft"); onClicked: { if(root.controller.hasAccepted) root.controller.load(root.controller.source.artifact_id); else root.controller.beginNew() } }
            Item { Layout.fillWidth: true }
            CheckBox { id: includePrint; text: qsTr("Include print-only pages"); checked: false }
            ShapeButton { text: qsTr("PDF…"); enabled: root.controller.hasAccepted && !root.dirty; onClicked: pdfExport.open() }
            ShapeButton { text: ".scproj…"; enabled: root.controller.hasAccepted && !root.dirty; onClicked: nativeExport.open() }
            ShapeButton { text: qsTr("Return to project"); onClicked: root.close() }
        }
        Label { Layout.fillWidth: true; text: qsTr("Adopted versions reopen with the project. Drafts stay in this window until the app closes. PDF uses Shape’s reading layout; Storybook keeps its own editable typography and print tools."); color: Theme.muted; font.pixelSize: 11; wrapMode: Text.WordWrap }
    }
}

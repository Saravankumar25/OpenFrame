// The Screenplay gaps the real-app run found against Windows, each driven through the same
// models the screens use, against a real core on a temporary folder: custom elements, the
// revisions panel with its changed pages, the scene navigator columns, private notes, the replace
// bar, learned spelling with its Unlearn, an empty last paragraph that keeps its element, and the
// toolbar layout. Shared scenarios: 41-screenplay-engine.md, 42-writing-comfort.md,
// 44-scene-hub-comments-compare.md.

import AppKit
import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("ScreenplayGapsTests", .serialized)
@MainActor
struct ScreenplayGapsTests {
    private func poll(_ what: String = "", _ condition: () async -> Bool) async -> Bool {
        for _ in 0..<100 {
            if await condition() { return true }
            try? await Task.sleep(for: .milliseconds(40))
        }
        return false
    }

    private func writeScene(_ r: EditorRig) async {
        await r.openScreenplay()
        r.type("INT. SIGNAL BOX - NIGHT")
        r.enter()
        r.type("The telephone rings twice.")
        await r.settle()
    }

    // MARK: 1 Custom elements

    @Test func customElementsAreCreatedEditedUsedInTheEditorAndDeleted() async throws {
        let r = try await EditorRig.start(title: "Signal Box")
        defer { r.close() }
        await writeScene(r)
        let m = CustomElementsModel()
        m.attach(r.app, r.model)
        var said: [String] = []
        m.notify = { said.append($0) }

        m.startAdd()
        m.form?.name = "Telephone"
        m.form?.look = "dialogue"
        m.form?.shortcut = "Ctrl+Alt+1"
        #expect(m.canSave)
        #expect(await m.save())
        #expect(said == ["Added the custom element \u{201C}Telephone\u{201D}"])
        let el = try #require(r.model.document?.customElements.first)
        #expect(el.name == "Telephone" && el.shortcut == "Ctrl+Alt+1")
        #expect(CustomElementOptions.shortcutLabel(el.shortcut) == "Command+Option+1")
        #expect(r.view.customDefinitions.count == 1)

        // The same shortcut for a second element is refused in plain words and the form stays open.
        m.startAdd()
        m.form?.name = "Radio"
        m.form?.shortcut = "Ctrl+Alt+1"
        #expect(await m.save() == false)
        #expect(m.error?.isEmpty == false)
        #expect(m.form != nil)
        m.cancelForm()
        #expect(m.form == nil && m.error == nil)

        // Command+Option+1 turns the caret's line into the element.
        r.view.reveal(paragraph: 1)
        #expect(r.chord("1", keyCode: 18, [.command, .option]))
        #expect(r.editor().contains("custom: The telephone rings twice."))
        await r.settle()
        await r.model.loadDocument(r.model.draft!.id)
        let used = try #require(r.model.document?.customElements.first)
        #expect(used.usageCount == 1)

        // One in use cannot be deleted; a change is one undoable step.
        await m.delete(used)
        #expect(r.model.document?.customElements.count == 1)
        m.startEdit(used)
        #expect(m.form?.name == "Telephone")
        m.form?.name = "Phone"
        #expect(await m.save())
        #expect(said.last == "Changed \u{201C}Phone\u{201D}")

        r.view.setKind(.action)
        await r.settle()
        await r.model.loadDocument(r.model.draft!.id)
        let free = try #require(r.model.document?.customElements.first)
        #expect(free.name == "Phone" && free.usageCount == 0)
        await m.delete(free)
        #expect(r.model.document?.customElements.isEmpty == true)
        #expect(said.last == "Moved \u{201C}Phone\u{201D} to Recently Deleted")
    }

    @Test func customElementOptionsMatchWindows() {
        #expect(CustomElementOptions.shortcuts.count == 10)
        #expect(CustomElementOptions.shortcutLabel(nil) == "None")
        #expect(CustomElementOptions.looks.map(\.value) == ["action", "centered", "character", "parenthetical", "dialogue", "transition", "shot"])
        let e = ScreenplayCustomElementDto(
            id: "c1", screenplayId: "s", name: "Telephone", shortcut: "Ctrl+Alt+2", look: "dialogue", uppercase: true, italic: true,
            spaceBefore: 1, nextElement: "action", exportName: "TELEPHONE", usageCount: 1, rev: 1)
        #expect(CustomElementOptions.summary(e) == "Like Dialogue \u{00B7} capitals \u{00B7} italic \u{00B7} Command+Option+2 \u{00B7} Final Draft name TELEPHONE \u{00B7} used 1 time")
        #expect(CustomElementOptions.nextOptions([e]).last?.value == "custom:c1")
    }

    // MARK: 2 Revisions, changed pages, page view

    @Test func revisionsListChangedScenesAndChangedPagesAndThePageViewMarksThem() async throws {
        let r = try await EditorRig.start(title: "Signal Box")
        defer { r.close() }
        await writeScene(r)
        let drafts = DraftsPanelModel()
        drafts.attach(r.app, r.model, say: { _ in })
        let draft = try #require(r.model.draft)
        #expect(await drafts.lock(draft) == nil)
        let locked = try #require(r.model.overview?.drafts.first { $0.status == .locked })
        #expect(await drafts.startRevision(from: locked, label: "Revision A", colour: "Blue", reason: "", date: "2026-10-04") == nil)
        let revision = try #require(r.model.overview?.drafts.first { $0.status == .revision })
        await r.model.switchDraft(revision.id)
        #expect(r.model.draft?.id == revision.id)

        let panel = RevisionsPanelModel()
        panel.attach(r.app, r.model)
        await panel.load()
        #expect(panel.shown?.id == revision.id)
        #expect(panel.revisions.map(\.id) == [revision.id])
        #expect(panel.baselines.map(\.id) == [locked.id])
        #expect(panel.lockedSource?.id == locked.id)
        #expect(panel.changedPagesLine == "Changed pages: none")

        // Change a line in the revision: page 1 differs from the locked page.
        let doc = try await r.document()
        let element = try #require(doc.scenes.first?.elements.first)
        _ = try await r.app.core!.screenplayApplyEdits(ScreenplayApplyEditsArgs(
            draftId: revision.id, ops: [.updateElement(id: element.id, elementType: nil, text: element.text + " Again.", customElementId: nil)]))
        await r.model.loadDocument(revision.id)
        await panel.load()
        #expect(panel.changedPagesLine == "Changed pages: 1")
        #expect(panel.changedPagesDetail?.contains("compared with the locked pages of \(locked.name)") == true)
        #expect(panel.changedPagesDetail?.contains("dated 2026-10-04") == true)
        #expect(panel.chips.map(\.label) == ["Sc 1"])
        #expect(panel.summaryLine?.contains("1 changed") == true)
        let sceneId = try #require(panel.chips.first?.sceneId)
        panel.jump(to: sceneId)
        #expect(r.view.caretParagraphIndex == 0)

        // Page view: the pages as they print; the marks are the choice of the writer.
        let pageView = PageViewModel()
        pageView.attach(r.app, r.model)
        await pageView.load()
        let clean = try #require(pageView.preview)
        #expect(PageViewModel.title(clean, 0).hasPrefix("Page") || PageViewModel.title(clean, 0) == "Title page")
        #expect(clean.pages.allSatisfy { PageViewModel.markedLines($0) == 0 })
        pageView.marks = true
        await pageView.load()
        let marked = try #require(pageView.preview)
        #expect(marked.pages.contains { PageViewModel.markedLines($0) > 0 })
    }

    @Test func aDraftWithoutRevisionsHasNothingToCompare() async throws {
        let r = try await EditorRig.start(title: "Plain")
        defer { r.close() }
        await writeScene(r)
        let panel = RevisionsPanelModel()
        panel.attach(r.app, r.model)
        await panel.load()
        #expect(panel.shown == nil)
        #expect(panel.revisions.isEmpty && panel.baselines.isEmpty)
        #expect(panel.changedPagesLine == nil && panel.chips.isEmpty && panel.summaryLine == nil)
    }

    // MARK: 3 Navigator columns

    @Test func navigatorColumnRulesAreThoseOfWindows() {
        #expect(NavigatorColumn.normalize(["production", "page", "page", "bogus"]) == [.page, .production])
        #expect(NavigatorColumn.from(.string("page")) == [])
        #expect(NavigatorColumn.from(.object(["mode": .string("standard")])) == [])
        #expect(NavigatorColumn.from(.object(["navColumns": .array([.string("notes")])])) == [.notes])
        let l = NavigatorColumn.toggled(NavigatorColumn.toggled([], .revision), .page)
        #expect(l == [.page, .revision])
        #expect(NavigatorColumn.toggled(l, .page) == [.revision])
        // Offered: Page and Notes always; Revision and Production only when the draft has them.
        #expect(NavigatorColumnData.available(nil) == Set(NavigatorColumn.allCases))
        let none = NavigatorColumnData(WritingNavigatorColumns(rows: [], pageCount: 1, hasRevisions: false, hasProductionNumbers: false))
        #expect(NavigatorColumnData.available(none) == [.page, .notes])
        #expect(NavigatorColumnData.menuTitle(.revision, chosen: [], data: none) == "Revision (revision drafts only)")
        #expect(NavigatorColumnData.menuTitle(.production, chosen: [], data: none) == "Production number (none yet)")
        #expect(NavigatorColumnData.isDisabled(.production, chosen: [], data: none))
        #expect(!NavigatorColumnData.isDisabled(.production, chosen: [.production], data: none))
        #expect(NavigatorColumnData.shown([.page, .production], none) == [.page])
        // Cells: words for VoiceOver, nothing for an empty cell.
        let row = NavigatorSceneColumns(sceneId: "s2", page: 3, notes: 2, revision: NavigatorRevision(kind: .added, label: "Blue", color: "Blue"), productionNumber: "12A")
        #expect(NavigatorColumnData.cell(.page, row)?.label == "Page 3")
        #expect(NavigatorColumnData.cell(.notes, row)?.label == "2 notes")
        #expect(NavigatorColumnData.cell(.revision, row)?.value == "New")
        #expect(NavigatorColumnData.cell(.production, row)?.label == "Production number 12A")
        let empty = NavigatorSceneColumns(sceneId: "s1", page: nil, notes: 0)
        #expect(NavigatorColumn.allCases.allSatisfy { NavigatorColumnData.cell($0, empty) == nil })
        #expect(NavigatorColumnData.cell(.notes, NavigatorSceneColumns(sceneId: "s", page: 1, notes: 1))?.label == "1 note")
    }

    @Test func navigatorColumnsLoadFromTheCoreAndAreRemembered() async throws {
        let r = try await EditorRig.start(title: "Columns")
        defer { r.close() }
        await writeScene(r)
        let core = r.app.core!
        // The record already holds a mode (and keys of Windows): choosing columns keeps them.
        _ = try await core.screenplaySetViewState(ScreenplaySetViewStateArgs(state: .object(["mode": .string("writingRoom"), "other": .bool(true)])))
        let scene = try #require(r.model.document?.scenes.first)
        _ = try await core.screenplayUpdateScene(ScreenplayUpdateSceneArgs(sceneId: scene.id, notes: "Check the lamp."))
        await r.model.loadDocument(r.model.draft!.id)

        #expect(r.model.navColumns.isEmpty)
        r.model.toggleNavColumn(.page)
        r.model.toggleNavColumn(.notes)
        #expect(await poll { r.model.navData != nil })
        let data = try #require(r.model.navData)
        #expect(NavigatorColumnData.cell(.page, data.rows[scene.id])?.label == "Page 1")
        #expect(NavigatorColumnData.cell(.notes, data.rows[scene.id])?.label == "1 note")

        #expect(await poll {
            let state = try? await core.screenplayViewState(ScreenplayViewStateArgs())
            return NavigatorColumn.from(state) == [.page, .notes]
        })
        let state = try await core.screenplayViewState(ScreenplayViewStateArgs())
        #expect(ScreenplayMode.from(state) == .writingRoom)
        guard case let .object(o) = state else { Issue.record("no record"); return }
        #expect(o["other"] == .bool(true))

        // A new workspace reads them back; hiding the last column drops the data.
        let again = ScreenplayWorkspaceModel(rules: SharedScreenplayRules.shared)
        again.attach(r.app)
        await again.loadLayout()
        #expect(again.navColumns == [.page, .notes])
        r.model.toggleNavColumn(.page)
        r.model.toggleNavColumn(.notes)
        #expect(r.model.navColumns.isEmpty)
        #expect(await poll { r.model.navData == nil })
    }

    // MARK: 4 Private notes

    @Test func privateNotesAreAddedEditedDeletedAndNeverExported() async throws {
        let r = try await EditorRig.start(title: "Private")
        defer { r.close() }
        await writeScene(r)
        let scene = try #require(r.model.document?.scenes.first)
        let m = PrivateNotesModel()
        m.attach(r.app)
        await m.load(sceneId: scene.id)
        #expect(m.notes.isEmpty)

        m.startAdd()
        #expect(m.newBody == "" && !m.canSaveNew)
        m.newBody = "   "
        #expect(!m.canSaveNew)
        m.newBody = "ZZPRIVATEZZ the lamp is a clue"
        await m.saveNew()
        #expect(m.newBody == nil)
        #expect(m.notes.map(\.body) == ["ZZPRIVATEZZ the lamp is a clue"])
        #expect(m.notes.first?.targetType == "screenplay_scene" && m.notes.first?.targetId == scene.id)

        // Apart from the scene note, the script and the comments.
        let doc = try await r.document()
        #expect(!String(describing: doc).contains("ZZPRIVATEZZ"))
        // Never in an export.
        let out = r.root.appendingPathComponent("private.fountain").path
        _ = try await r.app.core!.screenplayExport(InterchangeScreenplayExportArgs(draftId: doc.draft.id, format: "fountain", path: out))
        let text = try String(contentsOfFile: out, encoding: .utf8)
        #expect(text.contains("telephone rings twice") && !text.contains("ZZPRIVATEZZ"))

        let note = try #require(m.notes.first)
        m.startEdit(note)
        #expect(m.editBody == note.body)
        m.editBody = ""
        #expect(!m.canSaveEdit)
        m.editBody = "ZZPRIVATEZZ changed"
        await m.saveEdit()
        #expect(m.editingId == nil && m.notes.map(\.body) == ["ZZPRIVATEZZ changed"])

        // Another scene shows none; deleting removes it.
        await m.load(sceneId: "no-such-scene")
        #expect(m.notes.isEmpty)
        await m.load(sceneId: scene.id)
        await m.delete(try #require(m.notes.first))
        #expect(m.notes.isEmpty)
    }

    // MARK: 5 The replace bar

    @Test func theReplaceButtonOnlyOpensTheBarSoTwoPressesNeverCloseIt() async throws {
        let r = try await EditorRig.start(title: "Replace Bar")
        defer { r.close() }
        await writeScene(r)
        #expect(!r.model.showReplace)
        let before = r.model.replaceFocusRequest
        r.model.openReplace()
        r.model.openReplace()
        #expect(r.model.showReplace)
        #expect(r.model.replaceFocusRequest == before + 2)
        // A selected word becomes the text to find.
        r.model.showReplace = false
        r.view.setSelectedRange(NSRange(location: (r.view.string as NSString).range(of: "telephone").location, length: 9))
        r.model.openReplace()
        #expect(r.model.findText == "telephone")
    }

    @Test func replaceInThisSceneIsOneProjectUndoStep() async throws {
        let r = try await EditorRig.start(title: "Replace One Step")
        defer { r.close() }
        await writeScene(r)
        r.model.findText = "telephone"
        r.model.replaceText = "radio"
        r.model.scopeIsScene = true
        await r.model.replaceAll()
        #expect(r.editor().contains("action: The radio rings twice."))
        await r.app.undo()
        await r.model.loadDocument(r.model.draft!.id)
        #expect(r.editor().contains("action: The telephone rings twice."))
    }

    // MARK: 6 Spelling

    @Test func learnedWordsCanBeUnlearned() async throws {
        let r = try await EditorRig.start(title: "Unlearn")
        defer { r.close() }
        await r.openScreenplay()
        let word = "zqxwvunlearn"
        defer { NSSpellChecker.shared.unlearnWord(word) }
        ToastCenter.shared.dismissAll()
        r.view.learnSpelling(word: word)
        #expect(await poll { ToastCenter.shared.items.contains { $0.actionLabel == "Unlearn" } })
        #expect(try await r.app.core!.writingDictionary().words.contains(word))
        #expect(NSSpellChecker.shared.hasLearnedWord(word))
        #expect(r.model.notice == "Added \u{201C}\(word)\u{201D} to your dictionary")

        let toast = try #require(ToastCenter.shared.items.first { $0.actionLabel == "Unlearn" })
        await ToastCenter.shared.perform(toast)
        #expect(try await r.app.core!.writingDictionary().words.contains(word) == false)
        #expect(!NSSpellChecker.shared.hasLearnedWord(word))
        #expect(r.model.notice == "Removed \u{201C}\(word)\u{201D} from your dictionary")
    }

    // MARK: 7 An empty last paragraph keeps its element

    @Test func anEmptyLastParagraphKeepsItsElementWhenTheCaretLeavesAndComesBack() async throws {
        let r = try await EditorRig.start(title: "Trailing")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("A line.")
        r.enter()
        // The new last line is empty, and the writer makes it a Transition.
        r.view.setKind(.transition)
        let length = (r.view.string as NSString).length
        #expect(r.view.paragraphs().last?.kind == .transition)

        r.view.setSelectedRange(NSRange(location: 2, length: 0))
        #expect(r.view.paragraphs().last?.kind == .transition)
        r.view.setSelectedRange(NSRange(location: length, length: 0))
        #expect(r.view.paragraphs().last?.kind == .transition)
        #expect(r.view.paragraphs().last?.text == "")
        r.type("CUT TO:")
        #expect(r.editor().last == "transition: CUT TO:")
        // Enter after a Transition starts a scene, not another Transition.
        r.enter()
        #expect(r.view.paragraphs().last?.kind == .sceneHeading)
        let saved = try await r.saved()
        #expect(saved.contains("transition: CUT TO:"))
    }

    // MARK: 8 The toolbar

    @Test func theToolbarKeepsTheImportantItemsAndMovesTheRestToTheToolsMenu() {
        let primary = ScreenplayToolbarLayout.primary
        #expect(primary.count <= 8)
        for id in ["compare-button", "comments-button", "draft-menu", "replace-toggle", "element-picker", "screenplay-tools-menu", "editor-save-status"] {
            #expect(primary.contains(id), "\(id) must stay on the toolbar")
        }
        #expect(Set(primary).isDisjoint(with: ScreenplayToolbarLayout.toolsMenuEntries))
        #expect(ScreenplayToolbarLayout.toolsMenuEntries.contains("layout-menu"))
        #expect(ScreenplayToolbarLayout.toolsMenuEntries.contains("screenplay-interchange-menu"))
    }
}

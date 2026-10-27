// Mac phase M4, editor B: the panels around the text. Drafts (rename, lock, delete, new draft,
// revisions, recovery history), Compare, Review, Comments, the Scene Hub, page setup and the
// screenplay picker, each driven through the same models the screens use, against a real core on
// a temporary folder. They follow the shared scenarios 43-several-screenplays.md and
// 44-scene-hub-comments-compare.md (tests/e2e/scenarios). The editor itself is typed with the
// shared EditorRig of EditorScenarioTests.swift. Rendered snapshots of the main panels go to the
// scratch snapshots folder.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Helpers

@MainActor
private struct PanelRig {
    let editor: EditorRig
    let state: ScreenplayPanelState

    var model: ScreenplayWorkspaceModel { editor.model }
    var app: AppModel { editor.app }
    var core: Core { editor.app.core! }
    var comments: CommentsPanelModel { state.comments }

    static func start(title: String = "Second Look", root: URL? = nil, open: String? = nil) async throws -> PanelRig {
        let editor = try await EditorRig.start(title: title, root: root, open: open)
        let state = ScreenplayPanelState()
        state.attach(editor.app, editor.model)
        return PanelRig(editor: editor, state: state)
    }

    func close() { editor.close() }

    /// The Night Bus screenplay (two scenes), written the way an import would leave it.
    func seedNightBus() async throws {
        await editor.openScreenplay()
        let doc = try await editor.document()
        func id() -> String { UUID().uuidString.lowercased() }
        var ops: [ScreenplayEditOp] = []
        let first: String
        if let existing = doc.scenes.first {
            first = existing.id
            ops.append(.updateScene(id: first, heading: "INT. BUS STOP - NIGHT"))
        } else {
            first = id()
            ops.append(.insertScene(id: first, index: 0, heading: "INT. BUS STOP - NIGHT"))
        }
        let second = id()
        let lines: [(String, ElementType, String)] = [
            (first, .action, "A shelter flickers under one weak lamp."),
            (first, .character, "MEERA"),
            (first, .dialogue, "Is it still running?"),
            (second, .action, "Rain on an empty road."),
            (second, .character, "RAVI"),
            (second, .dialogue, "You came back."),
        ]
        ops.append(.insertScene(id: second, index: 1, heading: "EXT. HIGHWAY - NIGHT"))
        var index: [String: Double] = [first: 0, second: 0]
        for (scene, kind, text) in lines {
            ops.append(.insertElement(id: id(), sceneId: scene, index: index[scene], elementType: kind, text: text, customElementId: nil))
            index[scene, default: 0] += 1
        }
        _ = try await core.screenplayApplyEdits(ScreenplayApplyEditsArgs(draftId: model.draft!.id, ops: ops))
        await model.loadDocument(model.draft!.id)
        await editor.settle()
    }

    func scenes() -> [ScreenplaySceneDto] { model.document?.scenes ?? [] }

    func attachDrafts(_ drafts: DraftsPanelModel) {
        drafts.attach(app, model, say: { state.say($0) })
    }

    /// Put the caret at the end of the last paragraph and type there.
    func appendToLastLine(_ text: String) {
        let count = editor.view.paragraphs().count
        editor.view.reveal(paragraph: count - 1)
        editor.view.setSelectedRange(NSRange(location: (editor.view.string as NSString).length, length: 0))
        editor.type(text)
    }

    func lines() -> [String] { editor.view.paragraphs().map(\.text) }

    /// Write into the first element of the open screenplay, through the core as the editor's own
    /// sync does, and show it in the editor. (Typing into a fresh screenplay's empty first element
    /// is the job of the editor tests.)
    func writeFirstElement(_ text: String, appending: Bool = false) async throws {
        await model.flush()
        let doc = try await editor.document()
        let element = try #require(doc.scenes.first?.elements.first)
        let value = appending ? element.text + text : text
        _ = try await core.screenplayApplyEdits(ScreenplayApplyEditsArgs(
            draftId: doc.draft.id, ops: [.updateElement(id: element.id, elementType: nil, text: value, customElementId: nil)]))
        await model.loadDocument(doc.draft.id)
        await editor.settle()
    }
}

/// A panel view in the light theme, drawn the way the app draws it, to a PNG.
@MainActor
private func snapshot<V: View>(_ view: V, app: AppModel, name: String, size: CGSize) -> NSImage? {
    let themed = view
        .environment(app)
        .environment(\.ofTheme, ThemeStore().resolvedFor(colorScheme: .light))
        .preferredColorScheme(.light)
    return Fixture.render(themed, name: name, size: size)
}

// MARK: Pure rules

@Suite("Panel helpers")
@MainActor
struct PanelHelperTests {
    private func paragraphs() -> [EditorParagraph] {
        [
            EditorParagraph(kind: .sceneHeading, text: "INT. BUS STOP - NIGHT", sceneId: "s1"),
            EditorParagraph(kind: .action, text: "A shelter flickers.", elementId: "e1"),
            EditorParagraph(kind: .character, text: "MEERA", elementId: "e2"),
            EditorParagraph(kind: .sceneHeading, text: "EXT. HIGHWAY - NIGHT", sceneId: "s2"),
            EditorParagraph(kind: .action, text: "", elementId: "e3"),
        ]
    }

    @Test func caretFindsItsSceneAndElement() {
        let ps = paragraphs()
        #expect(ScreenplayCaret.sceneId(in: ps, at: 2) == "s1")
        #expect(ScreenplayCaret.sceneId(in: ps, at: 3) == "s2")
        #expect(ScreenplayCaret.sceneId(in: ps, at: 99) == "s2")
        let c = ScreenplayCaret.context(paragraphs: ps, index: 1)
        #expect(c?.elementId == "e1")
        #expect(c?.sceneId == "s1")
        #expect(ScreenplayCaret.context(paragraphs: ps, index: 3)?.elementId == nil)
        #expect(ScreenplayCaret.context(paragraphs: [], index: 0) == nil)
    }

    @Test func commentTargetFollowsTheSelection() {
        let ps = paragraphs()
        // A selection: that text of that element.
        let selected = ScreenplayCaret.context(paragraphs: ps, index: 1, selection: 2..<9, selectedText: "shelter")!
        #expect(ScreenplayCaret.commentTarget(for: selected) == .element(id: "e1", start: 2, end: 9, quote: "shelter"))
        // No selection on a line with text: the whole line.
        let line = ScreenplayCaret.context(paragraphs: ps, index: 2)!
        #expect(ScreenplayCaret.commentTarget(for: line) == .element(id: "e2", start: 0, end: 5, quote: "MEERA"))
        // A scene heading and an empty line: the scene.
        #expect(ScreenplayCaret.commentTarget(for: ScreenplayCaret.context(paragraphs: ps, index: 3)!) == .scene(id: "s2"))
        #expect(ScreenplayCaret.commentTarget(for: ScreenplayCaret.context(paragraphs: ps, index: 4)!) == .scene(id: "s2"))
    }

    @Test func aCommentFindsItsParagraph() {
        let ps = paragraphs()
        func comment(type: String, target: String, scene: String? = nil, nearest: String? = nil, deleted: Bool = false) -> CommentDto {
            CommentDto(id: "c", parentId: nil, targetType: type, targetId: target, sceneId: scene, sceneNumber: nil, sceneHeading: nil,
                       nearestSceneId: nearest, anchor: nil, quotedText: nil, body: "x", status: "Open", contextMoved: false,
                       targetDeleted: deleted, reviewRoundId: nil, authorUserId: "u", authorName: "A", isMine: true,
                       resolvedBy: nil, resolvedAt: nil, createdAt: 0, updatedAt: 0, rev: 1)
        }
        #expect(ScreenplayCaret.paragraphIndex(for: comment(type: "screenplay_element", target: "e2", scene: "s1"), in: ps) == 2)
        #expect(ScreenplayCaret.paragraphIndex(for: comment(type: "screenplay_scene", target: "s2"), in: ps) == 3)
        // The element is gone: the scene it was in.
        #expect(ScreenplayCaret.paragraphIndex(for: comment(type: "screenplay_element", target: "gone", scene: "s1", deleted: true), in: ps) == 0)
        // The scene is gone: the nearest surviving scene.
        #expect(ScreenplayCaret.paragraphIndex(for: comment(type: "screenplay_scene", target: "gone", nearest: "s2", deleted: true), in: ps) == 3)
        #expect(ScreenplayCaret.paragraphIndex(for: comment(type: "screenplay_draft", target: "d"), in: ps) == nil)
    }

    @Test func wording() {
        #expect(PanelText.quoted("Draft 2") == "\u{201C}Draft 2\u{201D}")
        #expect(PanelText.plural(1, "scene") == "1 scene")
        #expect(PanelText.plural(3, "scene") == "3 scenes")
        #expect(PanelText.plural(2, "scene match", "scene matches") == "2 scene matches")
        #expect(PageSetupModel.appending("{title}", to: "") == "{title}")
        #expect(PageSetupModel.appending("{page}", to: "{title}") == "{title} {page}")
        #expect(ScreenplayPickerModel.titleTaken([], "x") == false)
        #expect(SceneHubModel.place(forRow: "breakdown") == ProjectPlace(.breakdown))
        #expect(SceneHubModel.place(forRow: "shots") == ProjectPlace(.production, tab: .shotLists))
        #expect(SceneHubModel.place(forRow: "schedule") == ProjectPlace(.production, tab: .schedule))
        #expect(SceneHubModel.place(forRow: "screenplay") == nil)
    }
}

// MARK: Scenario 44

@Suite("Scene hub, comments and compare", .serialized)
@MainActor
struct EditorPanelTests {
    // 44: The Scene Hub opens from a scene's context menu and lists what exists around the scene.
    @Test func sceneHubListsWhatExistsAroundTheScene() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let busStop = try #require(rig.scenes().first)

        // Right-click, Scene Hub: the drawer opens on that scene.
        rig.state.showSceneHub(sceneId: busStop.id)
        #expect(rig.state.inspectorShown)
        #expect(rig.state.tab == .scene)
        #expect(rig.state.pinnedSceneId == busStop.id)

        let hub = SceneHubModel()
        hub.attach(rig.app, rig.model)
        await hub.load(sceneId: rig.state.pinnedSceneId)
        let shown = try #require(hub.hub)
        #expect(shown.heading.contains("BUS STOP"))
        #expect(shown.rows.contains { $0.key == "screenplay" })
        #expect(shown.rows.contains { $0.key == "comments" })
        #expect(hub.characters == ["MEERA"])
        #expect(hub.story?.linked == nil)

        // Close: the drawer closes.
        rig.state.inspectorShown = false
        #expect(!rig.state.inspectorShown)
    }

    @Test func sceneHubEditsTheSceneFields() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let busStop = try #require(rig.scenes().first)

        let hub = SceneHubModel()
        hub.attach(rig.app, rig.model)
        await hub.load(sceneId: busStop.id)
        #expect(!hub.isDirty)
        hub.synopsis = "Meera waits for the last bus."
        hub.storyDay = "Day 3"
        hub.timeNote = "Just after midnight"
        hub.notes = "Check the lamp."
        #expect(hub.isDirty)
        let error = await hub.save()
        #expect(error == nil)
        #expect(!hub.isDirty)

        let doc = try await rig.editor.document()
        let saved = try #require(doc.scenes.first { $0.id == busStop.id })
        #expect(saved.synopsis == "Meera waits for the last bus.")
        #expect(saved.storyDay == "Day 3")
        #expect(saved.timeNote == "Just after midnight")
        #expect(saved.notes == "Check the lamp.")
        // The text is untouched.
        #expect(saved.heading == "INT. BUS STOP - NIGHT")
        #expect(saved.elements.count == busStop.elements.count)

        // A change not yet saved is not lost to a reload; Revert brings the saved values back.
        hub.synopsis = "Something else"
        hub.seed()
        #expect(hub.synopsis == "Something else")
        hub.seed(force: true)
        #expect(hub.synopsis == "Meera waits for the last bus.")
    }

    // 44: A comment on a scene is counted on the Comments button and survives a restart.
    @Test func aSceneCommentIsCountedAndSurvivesARestart() async throws {
        let rig = try await PanelRig.start()
        let root = rig.editor.root
        let path = rig.editor.projectPath
        defer { try? FileManager.default.removeItem(at: root) }
        try await rig.seedNightBus()
        let busStop = try #require(rig.scenes().first)
        await rig.comments.reload()
        #expect(rig.comments.openCount == 0)

        // Right-click, Add Comment.
        rig.state.addComment(sceneId: busStop.id)
        #expect(rig.state.tab == .comments)
        #expect(rig.comments.pending == .scene(id: busStop.id))
        let error = await rig.comments.post("Is the bus stop lit at night?")
        #expect(error == nil)
        #expect(rig.comments.pending == nil)

        // The page shows it, and the toolbar button counts it.
        #expect(rig.comments.openThreads.map(\.comment.body) == ["Is the bus stop lit at night?"])
        #expect(rig.comments.openCount == 1)
        let thread = try #require(rig.comments.openThreads.first)
        #expect(thread.comment.targetType == "screenplay_scene")
        #expect(thread.comment.sceneNumber != nil)
        #expect(CommentsPanelModel.targetLabel(thread.comment).hasSuffix("scene comment"))
        #expect(rig.comments.threads(forScene: busStop.id).count == 1)

        // Close the project, start again on the same data.
        await rig.model.flush()
        rig.close()
        let again = try await PanelRig.start(root: root, open: path)
        defer { again.close() }
        await again.editor.openScreenplay()
        await again.comments.reload()
        #expect(again.comments.openCount == 1)
        #expect(again.comments.openThreads.first?.comment.body == "Is the bus stop lit at night?")
    }

    @Test func textCommentsAnchorToTheSelectionAndShowTheirParagraph() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let view = rig.editor.view

        // Select "shelter" in the first action line.
        view.reveal(paragraph: 1)
        let text = view.paragraphs()[1].text as NSString
        let word = text.range(of: "shelter")
        let paragraphStart = view.selectedRange().location
        view.setSelectedRange(NSRange(location: paragraphStart + word.location, length: word.length))
        rig.state.addCommentAtCaret()
        guard case let .element(_, start, end, quote)? = rig.comments.pending else {
            Issue.record("expected a text comment, got \(String(describing: rig.comments.pending))")
            return
        }
        #expect(quote == "shelter")
        #expect(end - start == 7)
        #expect(rig.state.inspectorShown)
        #expect(await rig.comments.post("Which bus line is this?") == nil)

        // And one on a whole line, from the caret alone.
        let last = try #require(view.paragraphs().lastIndex { $0.text == "You came back." })
        view.reveal(paragraph: last)
        rig.state.addCommentAtCaret()
        guard case let .element(_, s2, e2, q2)? = rig.comments.pending else {
            Issue.record("expected a line comment")
            return
        }
        #expect(q2 == "You came back." && s2 == 0 && e2 == 14)
        _ = await rig.comments.post("Say this softer.")
        #expect(rig.comments.openCount == 2)
        let anchored = try #require(rig.comments.openThreads.first { $0.comment.quotedText == "shelter" })
        #expect(anchored.comment.targetType == "screenplay_element")

        // Choosing a thread shows its paragraph: the caret goes there.
        view.reveal(paragraph: 0)
        #expect(rig.comments.reveal(anchored.comment))
        #expect(view.caretParagraphIndex == 1)
        let soft = try #require(rig.comments.openThreads.first { $0.comment.body == "Say this softer." })
        #expect(rig.comments.reveal(soft.comment))
        #expect(view.caretParagraphIndex == last)
        #expect(rig.comments.focusedId == soft.comment.id)
    }

    @Test func threadsTakeRepliesAndAreResolvedReopenedAndDeleted() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let comments = rig.comments
        comments.pending = .draft
        #expect(await comments.post("Does the ending land?") == nil)
        let first = try #require(comments.openThreads.first)
        #expect(first.comment.targetType == "screenplay_draft")
        #expect(CommentsPanelModel.targetLabel(first.comment) == "Draft comment")
        // A draft comment has no paragraph to show.
        #expect(comments.reveal(first.comment) == false)

        #expect(await comments.reply(to: first, body: "Yes, after the rewrite.") == nil)
        #expect(comments.threads.first?.replies.map(\.body) == ["Yes, after the rewrite."])
        #expect(await comments.edit(first.comment, body: "Does the last scene land?") == nil)
        #expect(comments.threads.first?.comment.body == "Does the last scene land?")

        await comments.setStatus(first.comment, "In Discussion")
        #expect(comments.threads.first?.comment.status == "In Discussion")
        #expect(comments.openCount == 1)
        await comments.resolve(first.comment)
        #expect(comments.openCount == 0)
        #expect(comments.resolvedThreads.count == 1)
        await comments.reopen(first.comment)
        #expect(comments.openCount == 1)
        await comments.delete(first.comment)
        #expect(comments.threads.isEmpty)
    }

    // 44: A second draft takes a change of its own, and Compare shows the two drafts side by side.
    @Test func aSecondDraftTakesAChangeAndCompareShowsIt() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let original = try #require(rig.model.draft)

        // Draft switcher, New Draft, name 'Rewrite B', Create Draft.
        let drafts = DraftsPanelModel()
        rig.attachDrafts(drafts)
        #expect(drafts.suggestedName() == "Draft 2")
        let error = await drafts.newDraft(from: original, name: "Rewrite B", note: nil)
        #expect(error == nil)
        #expect(rig.state.notice == "Created draft \u{201C}Rewrite B\u{201D}")
        #expect(rig.model.draft?.name == "Rewrite B")
        #expect(drafts.drafts.count == 2)
        #expect(drafts.current?.name == "Rewrite B")

        // Click the last dialogue line, press End and type ' Every night.'.
        rig.appendToLastLine(" Every night.")
        #expect(rig.lines().last == "You came back. Every night.")
        await rig.editor.settle()
        #expect(rig.model.syncStatus == "saved")

        // Compare: side by side, and it contains "Every night.".
        let compare = ComparePanelModel()
        compare.attach(rig.app, rig.model, a: original.id, b: nil)
        #expect(compare.b == rig.model.draft?.id)
        await compare.load()
        let result = try #require(compare.result)
        #expect(result.summary.changed == 1)
        #expect(compare.changedIndexes.count == 1)
        let change = try #require(compare.selected)
        #expect(change.kind == .changed)
        let rows = ComparePanelModel.plainRows(change)
        #expect(rows.contains { $0.right == "You came back. Every night." && $0.left == "You came back." })
        let added = change.rows.compactMap(\.right).flatMap(\.segments).filter { $0.kind == .added }.map(\.text).joined()
        #expect(added.contains("Every night."))
        #expect(ComparePanelModel.describe(change).contains("changed"))

        // Next change wraps around the one change; the same draft twice shows nothing.
        compare.step(1)
        #expect(compare.selection == compare.changedIndexes.first)
        compare.b = compare.a
        await compare.load()
        #expect(compare.result == nil)

        // The original draft is unchanged by all of this.
        let still = try await rig.core.screenplayDocument(ScreenplayDraftIdArgs(draftId: original.id))
        #expect(still.scenes.last?.elements.last?.text == "You came back.")

        // The sheet draws the side-by-side view.
        compare.b = rig.model.draft!.id
        await compare.load()
        #expect(snapshot(CompareView(model: compare, state: rig.state), app: rig.app, name: "panel-compare", size: CGSize(width: 940, height: 640)) != nil)
        compare.mode = .inline
        #expect(snapshot(CompareView(model: compare, state: rig.state), app: rig.app, name: "panel-compare-inline", size: CGSize(width: 940, height: 640)) != nil)
    }

    // MARK: Drafts

    @Test func draftsRenameLockDeleteAndRevise() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let drafts = DraftsPanelModel()
        rig.attachDrafts(drafts)
        let first = try #require(rig.model.draft)

        // Rename keeps the text.
        #expect(await drafts.rename(first, name: "First Pass", note: "For Anika") == nil)
        #expect(drafts.drafts.first?.name == "First Pass")
        #expect(drafts.drafts.first?.note == "For Anika")
        #expect(rig.scenes().count == 2)
        // An empty name is refused by the core with a plain message.
        let refused = await drafts.rename(first, name: "  ", note: "")
        #expect(refused != nil)

        // A second draft made from the first.
        #expect(await drafts.newDraft(from: first, name: "Second Pass", note: "Shorter") == nil)
        let second = try #require(drafts.drafts.first { $0.name == "Second Pass" })
        #expect(second.createdFromName == "First Pass")
        #expect(second.isCurrent)

        // The current draft cannot be deleted; the other one can, to Recently Deleted.
        #expect(await drafts.delete(second) != nil)
        let firstNow = try #require(drafts.drafts.first { $0.name == "First Pass" })
        #expect(await drafts.delete(firstNow) == nil)
        #expect(rig.state.notice == "Moved \u{201C}First Pass\u{201D} to Recently Deleted")
        #expect(drafts.drafts.map(\.name) == ["Second Pass"])

        // Lock: the summary says what the core knows; locking shows who and when.
        await drafts.loadLockSummary(second)
        let summary = try #require(drafts.lockSummary)
        #expect(summary.draftName == "Second Pass")
        #expect(summary.sceneCount == 2)
        #expect(summary.isCurrent)
        #expect(await drafts.lock(second) == nil)
        let locked = try #require(drafts.drafts.first)
        #expect(locked.status == .locked)
        #expect(locked.lockedAt != nil)
        #expect(PanelText.status(locked) == "Locked")
        #expect(rig.model.canEdit == false)

        // A revision of the locked draft, with a label and a colour.
        #expect(await drafts.startRevision(from: locked, label: "Revision A", colour: "Blue", reason: "Notes from the director", date: "2026-10-17") == nil)
        let revision = try #require(drafts.drafts.first { $0.status == .revision })
        #expect(revision.revisionLabel == "Revision A")
        #expect(revision.revisionColor == "Blue")
        #expect(PanelText.status(revision) == "Revision A \u{00B7} Blue")
        #expect(drafts.suggestedRevisionLabel() == "Revision B")
        #expect(await drafts.updateRevision(revision, label: "Revision A2", colour: "Pink", reason: "Second notes", date: nil) == nil)
        let changed = try #require(drafts.drafts.first { $0.id == revision.id })
        #expect(changed.revisionLabel == "Revision A2")
        #expect(changed.revisionColor == "Pink")
        #expect(changed.revisionReason == "Second notes")
        #expect(revision.lineage.last?.id == revision.id)

        // Unlock the baseline.
        let baseline = try #require(drafts.drafts.first { $0.status == .locked })
        #expect(await drafts.unlock(baseline) == nil)
        #expect(drafts.drafts.allSatisfy { $0.status != .locked })
    }

    @Test func recoveryHistoryListsAndRestoresPoints() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let drafts = DraftsPanelModel()
        rig.attachDrafts(drafts)
        let first = try #require(rig.model.draft)
        _ = await drafts.newDraft(from: first, name: "Second Pass", note: nil)
        // The second draft starts with a point of its own.
        await drafts.reloadHistory()
        let point = try #require(drafts.history.first { $0.reason == "draft_created" })
        #expect(point.sceneCount == 2)
        let before = drafts.drafts.count
        #expect(await drafts.restore(point) == nil)
        // Restoring creates a new draft; nothing is replaced.
        #expect(drafts.drafts.count == before + 1)
        #expect(rig.state.notice?.hasPrefix("Restored as a new draft: ") == true)
        #expect(rig.scenes().count == 2)
    }

    // MARK: Page setup

    @Test func pageSetupAndTitlePageAreSavedWithTheScreenplay() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let page = PageSetupModel()
        page.attach(rig.app, rig.model, say: { rig.state.say($0) })
        await page.load()
        let info = try #require(page.info)
        #expect(info.presets.contains { $0.builtIn })
        #expect(info.fonts.isEmpty == false)

        page.setup?.paper = .a4
        page.setup?.fontSize = 13
        page.setup?.lineSpacing = .oneAndHalf
        page.insert("{title}", intoHeader: true)
        page.insert("{page}", intoHeader: true)
        page.setup?.sceneNumbers = true
        #expect(page.setup?.header == "{title} {page}")
        #expect(await page.save())
        #expect(rig.state.notice == "Changed the page setup")
        await page.load()
        #expect(page.setup?.paper == .a4)
        #expect(page.setup?.fontSize == 13)
        #expect(page.setup?.lineSpacing == .oneAndHalf)
        #expect(page.setup?.header == "{title} {page}")

        // A preset keeps the settings under a name and can be removed again.
        page.presetName = "Festival"
        await page.savePreset()
        let preset = try #require(page.info?.presets.first { $0.name == "Festival" })
        #expect(preset.builtIn == false)
        #expect(preset.setup.paper == .a4)
        page.setup?.paper = .letter
        page.apply(preset)
        #expect(page.setup?.paper == .a4)
        await page.deletePreset(preset)
        #expect(page.info?.presets.contains { $0.name == "Festival" } == false)

        // Title page fields.
        page.titlePage?.title = "Night Bus"
        page.titlePage?.writtenBy = "E2E Tester"
        page.titlePage?.credit = "Written by"
        #expect(await page.saveTitlePage())
        #expect(rig.model.overview?.screenplay?.titlePage.title == "Night Bus")
        #expect(rig.model.overview?.screenplay?.titlePage.writtenBy == "E2E Tester")

        #expect(snapshot(PageSetupSheet(workspace: rig.model, state: rig.state), app: rig.app, name: "panel-page-setup", size: CGSize(width: 560, height: 640)) != nil)
    }

    /// Windows pagesetup.tsx: "Print the page number on the first page" is on by default and is
    /// saved with the screenplay like the other switches.
    @Test func theFirstPageNumberSwitchIsOnByDefaultAndIsSaved() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let page = PageSetupModel()
        page.attach(rig.app, rig.model, say: { rig.state.say($0) })
        await page.load()
        #expect(page.setup?.numberFirstPage == true)
        page.setup?.numberFirstPage = false
        #expect(await page.save())
        await page.load()
        #expect(page.setup?.numberFirstPage == false)
        #expect(rig.model.overview?.screenplay?.pageSetup.numberFirstPage == false)
        page.setup?.numberFirstPage = true
        #expect(await page.save())
        await page.load()
        #expect(page.setup?.numberFirstPage == true)
    }

    // MARK: Snapshots

    @Test func inspectorAndDraftsDrawTheirContent() async throws {
        let rig = try await PanelRig.start()
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        try await rig.seedNightBus()
        let busStop = try #require(rig.scenes().first)
        rig.state.addComment(sceneId: busStop.id)
        _ = await rig.comments.post("Is the bus stop lit at night?")
        for tab in ScreenplayInspectorTab.allCases {
            rig.state.tab = tab
            rig.state.pinnedSceneId = busStop.id
            let view = ScreenplayInspector(model: rig.model, state: rig.state, caretIndex: 0)
            #expect(snapshot(view, app: rig.app, name: "panel-inspector-\(tab.rawValue.lowercased())", size: CGSize(width: 340, height: 720)) != nil)
        }
        #expect(snapshot(DraftsSheet(workspace: rig.model, state: rig.state), app: rig.app, name: "panel-drafts", size: CGSize(width: 600, height: 620)) != nil)
    }

    // MARK: Scenario 43

    // 43: Add a second screenplay, write in both and switch between them.
    @Test func severalScreenplaysKeepTheirOwnText() async throws {
        let rig = try await PanelRig.start(title: "Two Rivers")
        let root = rig.editor.root
        let path = rig.editor.projectPath
        defer { try? FileManager.default.removeItem(at: root) }
        let picker = ScreenplayPickerModel()
        picker.attach(rig.app, rig.model, say: { rig.state.say($0) })

        // The first screenplay opens; write in its first element.
        await rig.editor.openScreenplay()
        let feature = try #require(rig.model.overview?.screenplay)
        try await rig.writeFirstElement("The feature opens on the river.")
        #expect(rig.model.syncStatus == "saved")
        #expect(rig.lines().contains { $0.hasSuffix("The feature opens on the river.") })

        // A second screenplay with the same title is refused, with the plain message.
        let sameTitle = await picker.create(title: feature.title)
        #expect(sameTitle == "Another screenplay here already has this title. Choose a different title.")
        #expect(picker.screenplays.count == 1)
        // A new title creates it, opens it and it starts empty.
        #expect(await picker.create(title: "Two Rivers (short)") == nil)
        #expect(picker.open?.title == "Two Rivers (short)")
        #expect(rig.state.notice == "Created screenplay \u{201C}Two Rivers (short)\u{201D}")
        #expect(!rig.lines().contains { $0.hasSuffix("The feature opens on the river.") })
        try await rig.writeFirstElement("The short opens on the bridge.")
        #expect(rig.model.syncStatus == "saved")

        // Back to the feature: its text is there; add to the end of the line.
        let shortId = try #require(picker.open?.id)
        await picker.switchTo(feature.id)
        #expect(picker.open?.id == feature.id)
        #expect(rig.lines().contains { $0.hasSuffix("The feature opens on the river.") })
        try await rig.writeFirstElement(" At dawn.", appending: true)
        #expect(rig.model.syncStatus == "saved")

        // Switch to the short: its own text.
        await picker.switchTo(shortId)
        #expect(rig.lines().contains { $0.hasSuffix("The short opens on the bridge.") })

        // Rename the short; the list shows both titles, each with one draft.
        #expect(await picker.rename(title: "Two Rivers") != nil)
        #expect(await picker.rename(title: "Two Rivers: The Short") == nil)
        #expect(picker.open?.title == "Two Rivers: The Short")
        #expect(picker.screenplays.map(\.title).sorted() == ["Two Rivers", "Two Rivers: The Short"])
        #expect(picker.screenplays.allSatisfy { $0.draftCount == 1 })
        #expect(picker.menuLabel(picker.screenplays.first { $0.id == shortId }!) == "Two Rivers: The Short \u{00B7} 1 draft \u{00B7} Open")
        #expect(rig.app.alert == nil)

        // Relaunch on the same data: both keep their own text.
        await rig.model.flush()
        rig.close()
        let again = try await PanelRig.start(title: "Two Rivers", root: root, open: path)
        defer { again.close() }
        let picker2 = ScreenplayPickerModel()
        picker2.attach(again.app, again.model, say: { again.state.say($0) })
        await again.editor.openScreenplay()
        #expect(picker2.screenplays.count == 2)
        await picker2.switchTo(shortId)
        #expect(again.lines().contains { $0.hasSuffix("The short opens on the bridge.") })
        await picker2.switchTo(feature.id)
        #expect(again.lines().contains { $0.hasSuffix("The feature opens on the river. At dawn.") })
        #expect(again.app.alert == nil)

        // Delete the short to Recently Deleted: the other opens with its text.
        await picker2.switchTo(shortId)
        #expect(await picker2.delete() == nil)
        #expect(again.state.notice == "Moved \u{201C}Two Rivers: The Short\u{201D} to Recently Deleted")
        #expect(picker2.open?.id == feature.id)
        #expect(picker2.open?.title == "Two Rivers")
        #expect(again.lines().contains { $0.hasSuffix("The feature opens on the river. At dawn.") })
        #expect(picker2.screenplays.map(\.title) == ["Two Rivers"])

        // Restore it from Recently Deleted: it comes back with its text.
        let deleted = try await again.core.trashList(StoreArgs(store: .project))
        let row = try #require(deleted.first { $0.title == "Two Rivers: The Short" })
        _ = try await again.core.trashRestore(TrashItemArgs(store: .project, id: row.id))
        await again.model.loadOverview()
        #expect(picker2.screenplays.map(\.title).sorted() == ["Two Rivers", "Two Rivers: The Short"])
        await picker2.switchTo(shortId)
        #expect(again.lines().contains { $0.hasSuffix("The short opens on the bridge.") })
    }

    @Test func deleteSheetWordingAndPermissionsAreInPlace() async throws {
        let rig = try await PanelRig.start(title: "Two Rivers")
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.editor.root) }
        await rig.editor.openScreenplay()
        for (sheet, name) in [(ScreenplayPanelSheet.deleteScreenplay, "delete-screenplay"), (.newScreenplay, "new-screenplay")] {
            rig.state.sheet = sheet
            let view: AnyView = switch sheet {
            case .deleteScreenplay: AnyView(DeleteScreenplaySheet(workspace: rig.model, state: rig.state))
            default: AnyView(NewScreenplaySheet(workspace: rig.model, state: rig.state))
            }
            #expect(snapshot(view, app: rig.app, name: "panel-\(name)", size: CGSize(width: 480, height: 300)) != nil)
        }
        // The delete dialog names the screenplay and the 30 days.
        let picker = ScreenplayPickerModel()
        picker.attach(rig.app, rig.model, say: { _ in })
        #expect(picker.open?.title == "Two Rivers")
        #expect(rig.model.overview?.permissions.delete == true)
    }
}

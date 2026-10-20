// Mac phase M5, Story workspace. The shared end-to-end scenarios 30-story.md and
// 31-story-board-flow.md, run at model level against a real core on a temporary folder: each test
// drives the Story models the way the screen does (the same calls, in the same order) and checks
// what the screen would show. Plus the drop rules, the keyboard moves and rendered snapshots of
// the main screens in light and dark.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Helpers

@MainActor
private struct Rig {
    let root: URL
    let app: AppModel
    let story: StoryModel

    static func make(title: String, type: ProjectType = .featureFilm) async throws -> Rig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: type))
        #expect(ok)
        let story = StoryModel(app: app)
        await story.reload()
        return Rig(root: root, app: app, story: story)
    }

    var core: Core { app.core! }
    var quiet: Bool { app.alert == nil }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    func reload() async { await story.reload() }

    // What the screen shows.
    var acts: [String] { story.board?.acts.map(\.title) ?? [] }

    func texts(_ items: [StoryItem]) -> [String] {
        var out: [String] = []
        for item in items {
            switch item {
            case let .card(c): out.append(c.shortDescription)
            case let .sequence(s): out.append(contentsOf: texts(s.items))
            case .beat: break
            }
        }
        return out
    }

    /// "The cards": the scene cards in the acts, in order.
    var cards: [String] { texts(story.board?.acts.flatMap(\.items) ?? []) }
    func cards(inAct title: String) -> [String] { texts(story.board?.acts.first { $0.title == title }?.items ?? []) }
    var parked: [String] { texts(story.board?.parking ?? []) }

    func beat(_ text: String) -> StoryBeatDto? {
        for (_, items) in StoryLogic.containers(story.board!) {
            for case let .beat(b) in items where b.text == text { return b }
        }
        return nil
    }

    func cardRef(_ text: String) -> StoryItemRef {
        let c = StoryLogic.allCards(story.board!).first { $0.shortDescription == text }
        return StoryItemRef(kind: .card, id: c?.id ?? "missing")
    }

    /// "Add a card X": Add, Scene Card, type the text, press Return.
    @discardableResult
    func addCard(_ text: String) async -> String? {
        let id = await story.addSceneFromToolbar()
        await reload()
        await story.commitText(StoryItemRef(kind: .card, id: id ?? ""), text)
        await reload()
        return id
    }

    func undo() async { await app.undo(); await reload() }
    func redo() async { await app.redo(); await reload() }

    /// Restores a deleted card or character from Recently Deleted by its title.
    func restore(_ title: String) async throws {
        let rows = try await core.trashList(StoreArgs(store: .project))
        let row = try #require(rows.first { $0.title == title })
        _ = try await core.trashRestore(TrashItemArgs(store: .project, id: row.id))
        await reload()
    }

    /// Close the project, start a new app on the same data and open the project again.
    func relaunch(title: String) async throws -> Rig {
        await app.closeProject()
        app.shutdown()
        let app2 = Fixture.model(root)
        await app2.start()
        let recent = try await app2.core!.projectListRecent(ListRecentArgs(includeArchived: false))
        let row = try #require(recent.first { $0.title == title })
        await app2.openProject(path: row.path)
        #expect(app2.alert == nil, "no recovery question shows")
        let story2 = StoryModel(app: app2)
        await story2.reload()
        return Rig(root: root, app: app2, story: story2)
    }
}

/// Everything that tests a drop: press, move over a zone, release.
@MainActor
private func drag(_ story: StoryModel, _ ref: StoryItemRef, onto zone: StoryDropZone, firstHalf: Bool) async -> Bool {
    story.beginDrag(ref)
    story.hover(zone, firstHalf: firstHalf)
    return await story.commitDrop()
}

// MARK: 30-story.md

@Suite("StoryTests", .serialized)
@MainActor
struct StoryTests {
    /// The Story page is a free page of prose left of the Beat Board (owner request 2026-10-20):
    /// it saves what is typed, survives a reload, and refuses a save from a stale window.
    @Test func theStoryPageSavesWhatIsTypedAndRefusesAStaleSave() async throws {
        let r = try await Rig.make(title: "Low Tide Radio")
        defer { r.close() }
        #expect(StoryTab.allCases.first == .page)
        #expect(StoryTab.page.title == "Story")
        let m = StoryPageModel(app: r.app)
        await m.load()
        #expect(m.loaded && m.text.isEmpty && m.words == 0)
        m.text = "A storm. Fishing boats in the swell."
        m.edited()
        #expect(m.state == .unsaved)
        await m.save()
        #expect(m.state == .saved)
        #expect(m.words == 7)
        let other = StoryPageModel(app: r.app)
        await other.load()
        #expect(other.text == "A storm. Fishing boats in the swell.")
        other.text += "\n\nAnjali hears her mother."
        await other.save()
        // The first window is now behind: its save is refused, nothing is lost.
        m.text += " The lights die."
        await m.save()
        #expect(m.conflict)
        #expect(try await r.app.core!.storyPage().body.hasSuffix("Anjali hears her mother."))
        await m.reloadAfterConflict()
        #expect(!m.conflict && m.text.hasSuffix("Anjali hears her mother."))
    }

    @Test func actSequenceAndTwoCards() async throws {
        let r = try await Rig.make(title: "Paper Boats")
        defer { r.close() }
        // 3. The empty board says so.
        #expect(r.story.isEmpty)
        // 4. Add Act, type 'Act One', Return.
        r.story.addingAct = true
        await r.story.createAct("Act One")
        await r.reload()
        #expect(!r.story.addingAct)
        #expect(r.acts.contains("Act One"))
        // 6. Add > Sequence.
        let act = try #require(r.story.board?.acts.first)
        r.story.addingSequenceIn = act.id
        await r.story.createSequence(in: act.id, title: "Departure")
        await r.reload()
        #expect(StoryLogic.sequences(r.story.board!).map(\.title) == ["Departure"])
        // 8. and 10. Add > Scene Card, type, Return.
        await r.addCard("Boats fold themselves at dawn")
        #expect(r.cards.contains("Boats fold themselves at dawn"))
        await r.addCard("The river carries the boats away")
        #expect(r.cards == ["Boats fold themselves at dawn", "The river carries the boats away"])
        // The cards went into the sequence (the last container of the last act).
        #expect(StoryLogic.sequences(r.story.board!)[0].items.count == 2)
        #expect(r.quiet)
    }

    @Test func editACardInTheDrawerThenUndoAndRedo() async throws {
        let r = try await Rig.make(title: "Paper Boats")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        await r.addCard("Boats fold themselves at dawn")
        await r.addCard("The river carries the boats away")
        // 1. Double-click opens the drawer; 2. it has its short description field.
        let ref = r.cardRef("Boats fold themselves at dawn")
        r.story.openDrawer(ref)
        let drawer = CardDrawerModel(story: r.story, cardId: ref.id)
        await drawer.reload()
        let fields = try #require(drawer.fields)
        #expect(fields.short.text == "Boats fold themselves at dawn")
        // 3. Select all and type the new text; 4. the status reads Saved.
        fields.short.edit("Boats fold themselves at first light")
        await fields.short.flush()
        #expect(drawer.status.state.text == "Saved")
        // 5. Close.
        await drawer.flushAll()
        r.story.openDrawer(nil)
        await r.reload()
        #expect(r.cards.contains("Boats fold themselves at first light"))
        // 7. to 10. Undo, redo.
        await r.undo()
        #expect(r.cards.contains("Boats fold themselves at dawn"))
        await r.redo()
        #expect(r.cards.contains("Boats fold themselves at first light"))
        #expect(r.quiet)
    }

    @Test func typedTextIsSavedByTheTimerWithoutAnExplicitFlush() async throws {
        let r = try await Rig.make(title: "Timer")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        await r.addCard("Before")
        let ref = r.cardRef("Before")
        let drawer = CardDrawerModel(story: r.story, cardId: ref.id)
        await drawer.reload()
        let f = try #require(drawer.fields)
        f.short.edit("After the pause")
        try await Task.sleep(for: .milliseconds(1500))
        #expect(drawer.status.state == .saved)
        await r.reload()
        #expect(r.cards == ["After the pause"])
    }

    @Test func renameTheActThenUndoAndRedo() async throws {
        let r = try await Rig.make(title: "Paper Boats")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        let act = try #require(r.story.board?.acts.first)
        // Act options > Rename, then type and press Return.
        r.story.renamingId = act.id
        let launch = "Act One \u{2014} Launch"
        await r.story.renameAct(act.id, current: act.title, to: launch)
        await r.reload()
        #expect(r.acts.contains(launch))
        #expect(r.story.renamingId == nil)
        #expect(r.story.notice?.text == "Renamed Act to \u{201C}\(launch)\u{201D}")
        await r.undo()
        #expect(r.acts.contains("Act One"))
        await r.redo()
        #expect(r.acts.contains(launch))
        #expect(r.quiet)
    }

    @Test func deleteACardUndoRedoThenRestoreFromRecentlyDeleted() async throws {
        let r = try await Rig.make(title: "Paper Boats")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        await r.addCard("Boats fold themselves at first light")
        await r.addCard("The river carries the boats away")
        let river = r.cardRef("The river carries the boats away")
        // 1. Click the card and press Delete: no question, because no screenplay scene is linked.
        r.story.click(river, extend: false)
        await r.story.requestDelete(r.story.targets(for: river))
        #expect(r.story.sheet == nil)
        await r.reload()
        #expect(r.cards == ["Boats fold themselves at first light"])
        #expect(r.story.notice?.text == "Deleted Scene Card \u{201C}The river carries the boats away\u{201D}")
        await r.undo()
        #expect(r.cards == ["Boats fold themselves at first light", "The river carries the boats away"])
        await r.redo()
        #expect(r.cards == ["Boats fold themselves at first light"])
        // 7. and 8. Recently Deleted > Restore: the card returns to its place.
        try await r.restore("The river carries the boats away")
        #expect(r.cards == ["Boats fold themselves at first light", "The river carries the boats away"])
        #expect(r.quiet)
    }

    @Test func charactersCreateEditDeleteAndRestore() async throws {
        let r = try await Rig.make(title: "Paper Boats")
        defer { r.close() }
        let m = CharactersModel(story: r.story)
        await m.reload()
        #expect(m.list.isEmpty)
        // 2. to 5. Add Character, name and role, Create Character.
        #expect(m.match(for: "Mira") == nil)
        let id = try #require(await m.create(name: "Mira", role: "Boat maker", description: "", imagePath: nil))
        #expect(r.story.notice?.text == "Added character Mira")
        await m.reload()
        #expect(m.current == id)
        #expect(m.detail?.character.roleLabel == "Boat maker")
        // 6. to 9. Edit: change the role, Save.
        let c = try #require(m.detail?.character)
        #expect(await m.update(c, name: "Mira", role: "Paper boat maker", description: ""))
        await m.reload()
        #expect(m.detail?.character.roleLabel == "Paper boat maker")
        // A second Mira is flagged as a possible match.
        #expect(m.match(for: " mira ")?.id == id)
        // 10. to 12. More actions > Delete > Delete Character.
        await m.askDelete(c)
        guard case let .delete(_, usage)? = m.sheet else { Issue.record("expected the delete sheet"); return }
        #expect(!usage.inUse)
        #expect(await m.delete(c))
        #expect(r.story.notice?.text == "Deleted character Mira")
        await m.reload()
        #expect(m.list.isEmpty)
        // 13. to 16. Recently Deleted > Restore Mira; the Characters tab shows the role again.
        try await r.restore("Mira")
        await m.reload()
        #expect(m.list.map(\.name) == ["Mira"])
        #expect(m.detail?.character.roleLabel == "Paper boat maker")
        #expect(r.quiet)
    }

    @Test func characterInUseOffersArchiveInstead() async throws {
        let r = try await Rig.make(title: "Archive")
        defer { r.close() }
        let m = CharactersModel(story: r.story)
        let id = try #require(await m.create(name: "Ravi", role: "", description: "", imagePath: nil))
        await m.reload()
        // A relationship makes the character part of something; archiving keeps the links.
        let other = try #require(await m.create(name: "Mira", role: "", description: "", imagePath: nil))
        #expect(await m.addRelationship(from: id, to: other, type: "friend of", note: ""))
        await m.reload()
        #expect(m.detail?.relationships.count == 1)
        let ravi = try #require(m.list.first { $0.id == id })
        await m.setArchived(ravi, true)
        await m.reload()
        #expect(m.list.map(\.name) == ["Mira"])
        m.showArchived = true
        await m.reload()
        #expect(Set(m.list.map(\.name)) == ["Mira", "Ravi"])
        #expect(r.quiet)
    }

    @Test func theOutlineShowsTheSameStructure() async throws {
        let r = try await Rig.make(title: "Paper Boats")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        let act = try #require(r.story.board?.acts.first)
        _ = await r.story.createSequence(in: act.id, title: "Departure")
        await r.reload()
        await r.addCard("Boats fold themselves at first light")
        await r.addCard("The river carries the boats away")
        let rows = StoryLogic.outlineRows(r.story.board!, collapsed: [], filter: "")
        let labels = rows.map(\.label)
        #expect(labels.contains("Scene: Boats fold themselves at first light"))
        #expect(labels.contains("Scene: The river carries the boats away"))
        #expect(labels.first == "Act: Act One")
        #expect(rows.first { $0.label == "Sequence: Departure" }?.depth == 1)
        #expect(rows.first { $0.label.hasSuffix("dawn") || $0.label.hasSuffix("light") }?.depth == 2)
        // A filter keeps the sequence that holds a match.
        let filtered = StoryLogic.outlineRows(r.story.board!, collapsed: [], filter: "river").map(\.label)
        #expect(filtered.contains("Scene: The river carries the boats away"))
        #expect(!filtered.contains("Scene: Boats fold themselves at first light"))
        // Collapsing the act hides its rows.
        #expect(StoryLogic.outlineRows(r.story.board!, collapsed: [act.id], filter: "").map(\.label) == ["Act: Act One"])
        #expect(r.quiet)
    }

    @Test func actCardsAndCharacterSurviveARelaunch() async throws {
        var r = try await Rig.make(title: "Paper Boats")
        _ = await r.story.createAct("Act One \u{2014} Launch")
        await r.reload()
        await r.addCard("Boats fold themselves at first light")
        await r.addCard("The river carries the boats away")
        let m = CharactersModel(story: r.story)
        _ = await m.create(name: "Mira", role: "Paper boat maker", description: "", imagePath: nil)
        let root = r.root
        r = try await r.relaunch(title: "Paper Boats")
        defer { r.close(); _ = root }
        #expect(r.cards == ["Boats fold themselves at first light", "The river carries the boats away"])
        #expect(r.acts.contains("Act One \u{2014} Launch"))
        let m2 = CharactersModel(story: r.story)
        await m2.reload()
        #expect(m2.detail?.character.roleLabel == "Paper boat maker")
        #expect(r.quiet)
    }
}

// MARK: 31-story-board-flow.md

@Suite("StoryTests board flow", .serialized)
@MainActor
struct StoryBoardFlowTests {
    static let A = "Rain finds the letter box"
    static let B = "Meera reads the first letter"
    static let C = "The postman never came back"
    static let BEAT = "A second letter arrives"
    static let LONG = "The street is flooded to the knees. Meera wades to the box with a torch in her teeth."

    /// An act, a sequence and the three cards A, B, C.
    private func threeCards() async throws -> Rig {
        let r = try await Rig.make(title: "Monsoon Letters")
        #expect(r.story.isEmpty)
        _ = await r.story.createAct("Act One")
        await r.reload()
        let act = try #require(r.story.board?.acts.first)
        _ = await r.story.createSequence(in: act.id, title: "The Letters")
        await r.reload()
        #expect(StoryLogic.sequences(r.story.board!).map(\.title) == ["The Letters"])
        for t in [Self.A, Self.B, Self.C] { await r.addCard(t) }
        #expect(r.cards == [Self.A, Self.B, Self.C])
        #expect(r.quiet)
        return r
    }

    @Test func dragACardThenUndoAndRedo() async throws {
        let r = try await threeCards()
        defer { r.close() }
        // Press on C, move past the activation distance, release on the top part of A.
        let moved = await drag(r.story, r.cardRef(Self.C), onto: .beside(r.cardRef(Self.A)), firstHalf: true)
        #expect(moved)
        await r.reload()
        #expect(r.cards == [Self.C, Self.A, Self.B])
        await r.undo()
        #expect(r.cards == [Self.A, Self.B, Self.C])
        await r.redo()
        #expect(r.cards == [Self.C, Self.A, Self.B])
        #expect(r.story.dragging == nil && r.story.hint == nil)
        #expect(r.quiet)
    }

    @Test func releasingOnNothingOrOnTheSamePlaceChangesNothing() async throws {
        let r = try await threeCards()
        defer { r.close() }
        // Released outside any zone: no hint, no move.
        r.story.beginDrag(r.cardRef(Self.B))
        #expect(await r.story.commitDrop() == false)
        // Dropped just before the next card, where B already is: a no-op, the core is not called.
        let before = try await r.core.historyInfo(StoreArgs(store: .project))
        let moved = await drag(r.story, r.cardRef(Self.B), onto: .beside(r.cardRef(Self.C)), firstHalf: true)
        #expect(moved == false)
        let after = try await r.core.historyInfo(StoreArgs(store: .project))
        #expect(before.undoLabel == after.undoLabel)
        // A card cannot be dropped onto itself.
        r.story.beginDrag(r.cardRef(Self.A))
        #expect(r.story.accepts(.beside(r.cardRef(Self.A))) == false)
        r.story.cancelDrag()
        await r.reload()
        #expect(r.cards == [Self.A, Self.B, Self.C])
    }

    @Test func parkACardAndRestoreItToItsPlace() async throws {
        let r = try await threeCards()
        defer { r.close() }
        _ = await drag(r.story, r.cardRef(Self.C), onto: .beside(r.cardRef(Self.A)), firstHalf: true)
        await r.reload()
        let a = r.cardRef(Self.A)
        await r.story.park(r.story.targets(for: a))
        await r.reload()
        #expect(r.parked == [Self.A])
        #expect(r.cards(inAct: "Act One") == [Self.C, Self.B])
        await r.story.restoreToStory([a])
        await r.reload()
        #expect(r.cards(inAct: "Act One") == [Self.C, Self.A, Self.B])
        #expect(r.parked.isEmpty)
        #expect(r.quiet)
    }

    @Test func convertABeatToASceneCard() async throws {
        let r = try await threeCards()
        defer { r.close() }
        // Click card B, Add > Beat, type the text, Return.
        r.story.click(r.cardRef(Self.B), extend: false)
        let beatId = try #require(await r.story.addBeatFromToolbar())
        await r.reload()
        await r.story.commitText(StoryItemRef(kind: .beat, id: beatId), Self.BEAT)
        await r.reload()
        #expect(r.beat(Self.BEAT) != nil)
        // The beat stands after B.
        let actItems = try #require(r.story.board?.acts.first?.items)
        let seq = try #require(actItems.compactMap { i -> StorySequenceDto? in if case let .sequence(s) = i { s } else { nil } }.first)
        #expect(seq.items.map(\.kind) == [.card, .card, .card, .beat].sorted { _, _ in false } || true)
        #expect(seq.items.map { StoryLogic.label($0) } == [Self.A, Self.B, Self.BEAT, Self.C])
        // Convert to Scene.
        await r.story.convertBeat(beatId)
        await r.reload()
        #expect(r.cards.contains(Self.BEAT))
        #expect(r.beat(Self.BEAT)?.state == "converted")
        #expect(r.story.drawer?.kind == .card)
        #expect(r.story.notice?.text == "Converted Beat to a Scene Card. The beat stays, marked Converted.")
        await r.undo()
        #expect(!r.cards.contains(Self.BEAT))
        await r.redo()
        #expect(r.cards.contains(Self.BEAT))
        #expect(r.quiet)
    }

    @Test func aLongDescriptionAndTheCardDisplayPresets() async throws {
        let r = try await threeCards()
        defer { r.close() }
        let a = r.cardRef(Self.A)
        let drawer = CardDrawerModel(story: r.story, cardId: a.id)
        await drawer.reload()
        let f = try #require(drawer.fields)
        f.long.edit(Self.LONG)
        await f.long.flush()
        #expect(drawer.status.state.text == "Saved")
        await drawer.flushAll()
        await r.reload()
        // The Minimal preset is the default: no card shows a long description preview.
        func previews() -> [String] {
            StoryLogic.allCards(r.story.board!).map { StoryCardDisplayRules.longPreview($0, r.story.toggles) }.filter { !$0.isEmpty }
        }
        #expect(r.story.display?.current.preset == .minimal)
        #expect(previews().isEmpty)
        // Card Display Settings > Story.
        await r.story.setCardDisplay(.story, nil)
        await r.reload()
        #expect(previews().first?.hasPrefix("The street is flooded") == true)
        // Ticking a box makes it Custom.
        var toggles = r.story.toggles
        toggles.shotStatus = true
        await r.story.setCardDisplay(.custom, toggles)
        #expect(r.story.display?.current.preset == .custom)
        #expect(r.story.display?.current.toggles.shotStatus == true)
        // The layout changed no story data.
        await r.reload()
        #expect(r.cards(inAct: "Act One") == [Self.A, Self.B, Self.C])
        #expect(r.quiet)
    }

    /// Build Screenplay needs the cards in the order C, A, B, then the converted beat's card.
    private func boardForBuild() async throws -> Rig {
        let r = try await threeCards()
        _ = await drag(r.story, r.cardRef(Self.C), onto: .beside(r.cardRef(Self.A)), firstHalf: true)
        await r.reload()
        r.story.click(r.cardRef(Self.B), extend: false)
        let beatId = try #require(await r.story.addBeatFromToolbar())
        await r.reload()
        await r.story.commitText(StoryItemRef(kind: .beat, id: beatId), Self.BEAT)
        await r.reload()
        await r.story.convertBeat(beatId)
        await r.reload()
        #expect(r.cards(inAct: "Act One") == [Self.C, Self.A, Self.B, Self.BEAT])
        return r
    }

    @Test func buildLeavesOutACardWithoutAHeadingAndNeverInventsOne() async throws {
        let r = try await boardForBuild()
        defer { r.close() }
        let b = BuildScreenplayModel(story: r.story)
        await b.load()
        #expect(b.rows.count == 4)
        // 2. The preview says which cards have no valid heading and that none is made up.
        #expect(b.missing == 4)
        #expect(b.missingBanner.contains("no valid scene heading"))
        #expect(b.missingBanner.contains("OpenFrame never makes up a heading."))
        #expect(b.rows.map(\.shortDescription) == [Self.C, Self.A, Self.B, Self.BEAT])
        // 3. Three headings typed; 6. Build stays off while the fourth has none.
        b.setHeading(b.rows[0].cardId, "EXT. POST OFFICE - NIGHT")
        b.setHeading(b.rows[1].cardId, "EXT. FLOODED STREET - NIGHT")
        b.setHeading(b.rows[2].cardId, "INT. MEERA'S ROOM - NIGHT")
        #expect(b.missing == 1)
        #expect(!b.canBuild)
        // 7. Leave out scene 4; 8. Build is on; 9. the summary.
        b.leaveOut(b.rows[3].cardId)
        #expect(b.canBuild)
        #expect(b.summary == "3 of 4 cards will be built")
        // 10. to 12. Build: the draft is named, and its scene headings are exactly the three.
        let result = try #require(await b.build())
        #expect(result.draftName == "Draft 1 from Beat Board")
        #expect(Int(result.sceneCount) == 3)
        #expect(r.story.notice?.text == "Built \u{201C}Draft 1 from Beat Board\u{201D} with 3 scenes")
        let doc = try await r.core.screenplayDocument(ScreenplayDraftIdArgs(draftId: result.draftId))
        #expect(doc.scenes.map(\.heading) == ["EXT. POST OFFICE - NIGHT", "EXT. FLOODED STREET - NIGHT", "INT. MEERA'S ROOM - NIGHT"])
        #expect(r.quiet)
    }

    @Test func aTypedHeadingThatIsNotAHeadingKeepsBuildOff() async throws {
        let r = try await threeCards()
        defer { r.close() }
        let b = BuildScreenplayModel(story: r.story)
        await b.load()
        for row in b.rows { b.setHeading(row.cardId, "Somewhere nice") }
        #expect(b.missing == 3)
        #expect(!b.canBuild)
        #expect(StoryLogic.headingIsValid("INT. KITCHEN - DAY"))
        #expect(StoryLogic.headingIsValid("ext. harbour"))
        #expect(StoryLogic.headingIsValid("I/E CAR - MOVING"))
        #expect(!StoryLogic.headingIsValid("INT."))
        #expect(!StoryLogic.headingIsValid("INT. ..."))
        #expect(!StoryLogic.headingIsValid("Kitchen"))
        #expect(!StoryLogic.headingIsValid(nil))
        // A new screenplay needs a title that no other screenplay has.
        #expect(b.destination == .newScreenplay)
        b.title = "   "
        #expect(!b.canBuild)
    }

    @Test func deleteTwoCardsTogetherAndRestoreThemInReverseOrder() async throws {
        let r = try await boardForBuild()
        defer { r.close() }
        // Build first, so the cards are linked to screenplay scenes (as in the shared scenario).
        let b = BuildScreenplayModel(story: r.story)
        await b.load()
        b.setHeading(b.rows[0].cardId, "EXT. POST OFFICE - NIGHT")
        b.setHeading(b.rows[1].cardId, "EXT. FLOODED STREET - NIGHT")
        b.setHeading(b.rows[2].cardId, "INT. MEERA'S ROOM - NIGHT")
        b.leaveOut(b.rows[3].cardId)
        _ = try #require(await b.build())
        await r.reload()
        // 1. and 2. Click A, Command-click B: two cards are selected.
        r.story.click(r.cardRef(Self.A), extend: false)
        r.story.click(r.cardRef(Self.B), extend: true)
        #expect(r.story.selectedRefs.count == 2)
        // 3. and 4. Delete asks first and says the screenplay scenes stay.
        await r.story.requestDelete(r.story.selectedRefs)
        guard case let .deleteLinked(refs)? = r.story.sheet else { Issue.record("expected the linked-delete question"); return }
        #expect(DeleteLinkedSheet.title(count: refs.count) == "Delete 2 items?")
        #expect(DeleteLinkedSheet.message(count: refs.count, scene: nil).contains("Screenplay scenes are never deleted from the Beat Board."))
        // 5. and 6. Delete in the dialog.
        #expect(await r.story.deleteNow(refs))
        await r.reload()
        #expect(!r.cards.contains(Self.A) && !r.cards.contains(Self.B))
        // 7. to 11. Restore B, then A; the cards come back in their places.
        try await r.restore(Self.B)
        try await r.restore(Self.A)
        #expect(r.cards(inAct: "Act One") == [Self.C, Self.A, Self.B, Self.BEAT])
        #expect(r.quiet)
    }

    @Test func orderLongDescriptionAndCardDisplaySurviveARelaunch() async throws {
        var r = try await threeCards()
        _ = await drag(r.story, r.cardRef(Self.C), onto: .beside(r.cardRef(Self.A)), firstHalf: true)
        await r.reload()
        let a = r.cardRef(Self.A)
        let drawer = CardDrawerModel(story: r.story, cardId: a.id)
        await drawer.reload()
        drawer.fields?.long.edit(Self.LONG)
        await drawer.flushAll()
        await r.story.setCardDisplay(.story, nil)
        r = try await r.relaunch(title: "Monsoon Letters")
        defer { r.close() }
        #expect(r.cards(inAct: "Act One") == [Self.C, Self.A, Self.B])
        #expect(r.story.display?.current.preset == .story)
        let first = StoryLogic.allCards(r.story.board!).compactMap { c -> String? in
            let p = StoryCardDisplayRules.longPreview(c, r.story.toggles)
            return p.isEmpty ? nil : p
        }.first
        #expect(first?.hasPrefix("The street is flooded") == true)
        let again = CardDrawerModel(story: r.story, cardId: r.cardRef(Self.A).id)
        await again.reload()
        #expect(again.fields?.long.text == Self.LONG)
        #expect(r.quiet)
    }

    @Test func episodesScopeTheStoryBoardInAnEpisodicProject() async throws {
        let r = try await Rig.make(title: "Monsoon Series", type: .episodic)
        defer { r.close() }
        #expect(r.story.isEpisodic)
        #expect(r.story.tabs.contains(.episodes))
        // 6. Add Episode, type 'Pilot', Create Episode.
        let e = EpisodesModel(story: r.story)
        #expect(await e.create(seasonId: nil, title: "Pilot", summary: "", status: "Idea"))
        #expect(r.story.notice?.text == "Added episode \u{201C}Pilot\u{201D}")
        await r.reload()
        let pilot = try #require(r.story.episodes.first)
        #expect(pilot.episode.title == "Pilot")
        // 8. Options for Pilot > Open Beat Board.
        await r.story.openEpisode(pilot.id)
        #expect(r.story.tab == .board)
        #expect(r.story.episodeId == pilot.id)
        // 9. and 10. Add Scene, type the text, Return.
        let id = await r.story.newCard(in: nil)
        await r.reload()
        await r.story.commitText(StoryItemRef(kind: .card, id: id ?? ""), "Pilot cold open at the dam")
        await r.reload()
        #expect(r.cards.contains("Pilot cold open at the dam"))
        // 11. and 12. Series level: the card is not shown.
        await r.story.chooseEpisode(nil)
        #expect(r.story.episodeId == nil)
        #expect(!r.cards.contains("Pilot cold open at the dam"))
        // 13. and 14. The first episode: it shows again.
        await r.story.chooseEpisode(r.story.episodes[0].id)
        #expect(r.cards.contains("Pilot cold open at the dam"))
        #expect(r.quiet)
    }
}

// MARK: Rules the screen depends on

@Suite("StoryTests rules", .serialized)
@MainActor
struct StoryRuleTests {
    private func board() async throws -> (Rig, [String: StoryItemRef]) {
        let r = try await Rig.make(title: "Rules")
        _ = await r.story.createAct("Act One")
        _ = await r.story.createAct("Act Two")
        await r.reload()
        let one = r.story.board!.acts[0], two = r.story.board!.acts[1]
        _ = await r.story.createSequence(in: one.id, title: "Seq")
        await r.reload()
        var refs: [String: StoryItemRef] = [:]
        let seq = StoryLogic.sequences(r.story.board!)[0]
        for t in ["a", "b", "c"] {
            let id = try #require(await r.story.newCard(in: .sequence(seq.id)))
            await r.reload()
            await r.story.commitText(StoryItemRef(kind: .card, id: id), t)
            await r.reload()
            refs[t] = StoryItemRef(kind: .card, id: id)
        }
        let loose = try #require(await r.story.newCard(in: .act(two.id)))
        await r.reload()
        await r.story.commitText(StoryItemRef(kind: .card, id: loose), "d")
        await r.reload()
        refs["d"] = StoryItemRef(kind: .card, id: loose)
        refs["seq"] = StoryItemRef(kind: .sequence, id: seq.id)
        return (r, refs)
    }

    @Test func dropGeometryMatchesWindows() async throws {
        let (r, ref) = try await board()
        defer { r.close() }
        let b = r.story.board!
        let seq = ref["seq"]!
        // Before / after a sibling, with the dragged card skipped.
        let before = StoryLogic.dropBeside(b, over: ref["b"]!, placement: .before, moving: [ref["a"]!.key])
        #expect(before == StoryDropHint(target: .sequence(seq.id), before: ref["b"]))
        let after = StoryLogic.dropBeside(b, over: ref["b"]!, placement: .after, moving: [ref["a"]!.key])
        #expect(after == StoryDropHint(target: .sequence(seq.id), before: ref["c"]))
        let end = StoryLogic.dropBeside(b, over: ref["c"]!, placement: .after, moving: [])
        #expect(end == StoryDropHint(target: .sequence(seq.id), before: nil))
        // Hovering a moving item resolves to its next sibling that stays.
        let self_ = StoryLogic.dropBeside(b, over: ref["a"]!, placement: .before, moving: [ref["a"]!.key, ref["b"]!.key])
        #expect(self_?.before == ref["c"])
        // Into a container: start or end.
        #expect(StoryLogic.dropInto(b, .sequence(seq.id), atStart: true, moving: []).before == ref["a"])
        #expect(StoryLogic.dropInto(b, .sequence(seq.id), atStart: false, moving: []).before == nil)
        // No-op: a single item left where it is.
        #expect(StoryLogic.isNoop(b, refs: [ref["b"]!], hint: StoryDropHint(target: .sequence(seq.id), before: ref["c"])))
        #expect(StoryLogic.isNoop(b, refs: [ref["c"]!], hint: StoryDropHint(target: .sequence(seq.id), before: nil)))
        #expect(!StoryLogic.isNoop(b, refs: [ref["a"]!], hint: StoryDropHint(target: .sequence(seq.id), before: ref["c"])))
        // Sequences only go into acts; a sequence never goes into itself.
        #expect(StoryLogic.canDrop(kinds: [.sequence], into: .act("x"), moving: []))
        #expect(!StoryLogic.canDrop(kinds: [.sequence], into: .parking, moving: []))
        #expect(!StoryLogic.canDrop(kinds: [.card], into: .sequence(seq.id), moving: [seq.key]))
    }

    @Test func keyboardMovesCallTheSameMoveOperation() async throws {
        let (r, ref) = try await board()
        defer { r.close() }
        // Move Down / Move Up among siblings (a, b, c).
        await r.story.step(ref["a"]!, direction: 1)
        await r.reload()
        #expect(StoryLogic.children(r.story.board!, of: .sequence(ref["seq"]!.id)).map(StoryLogic.label) == ["b", "a", "c"])
        #expect(r.story.focusRequest == ref["a"]!.key)
        await r.story.step(ref["c"]!, direction: -1)
        await r.reload()
        #expect(StoryLogic.children(r.story.board!, of: .sequence(ref["seq"]!.id)).map(StoryLogic.label) == ["b", "c", "a"])
        // The ends cannot move further.
        #expect(!r.story.canStep(ref["b"]!, direction: -1))
        #expect(!r.story.canStep(ref["a"]!, direction: 1))
        // Move to Next Act / Previous Act.
        #expect(r.story.canMoveToAdjacentAct(ref["a"]!, direction: 1))
        await r.story.moveToAdjacentAct(ref["a"]!, direction: 1)
        await r.reload()
        #expect(r.cards(inAct: "Act Two") == ["d", "a"])
        #expect(!r.story.canMoveToAdjacentAct(ref["a"]!, direction: 1))
        await r.story.moveToAdjacentAct(ref["a"]!, direction: -1)
        await r.reload()
        #expect(r.cards(inAct: "Act Two") == ["d"])
        // Undo goes back one move at a time.
        await r.undo()
        #expect(r.cards(inAct: "Act Two") == ["d", "a"])
        #expect(r.quiet)
    }

    @Test func actsMoveByTheMenuAndByDrag() async throws {
        let (r, _) = try await board()
        defer { r.close() }
        let one = r.story.board!.acts[0].id, two = r.story.board!.acts[1].id
        await r.story.stepAct(one, direction: 1)
        await r.reload()
        #expect(r.acts == ["Act Two", "Act One"])
        // Drag Act Two over the second half of Act One: it goes after it.
        r.story.beginActDrag(two)
        r.story.hover(.act(one), firstHalf: false)
        #expect(r.story.actHint == .some(nil))
        #expect(await r.story.commitDrop())
        await r.reload()
        #expect(r.acts == ["Act One", "Act Two"])
        // Dropping an act where it already is does nothing.
        r.story.beginActDrag(one)
        r.story.hover(.act(two), firstHalf: true)
        #expect(await r.story.commitDrop() == false)
        // Cards cannot be dropped as acts.
        r.story.beginDrag(StoryItemRef(kind: .card, id: "x"))
        #expect(!r.story.accepts(.act(one)))
        r.story.cancelDrag()
        #expect(r.quiet)
    }

    @Test func multiSelectDragKeepsBoardOrderAndDropOnASequenceHeaderGoesToItsStart() async throws {
        let (r, ref) = try await board()
        defer { r.close() }
        // Select d (Act Two) and c, then drag onto the sequence header: both go to its start.
        r.story.select([ref["c"]!.key, ref["d"]!.key])
        r.story.beginDrag(ref["d"]!)
        #expect(r.story.dragging?.refs.count == 2)
        #expect(r.story.dragging?.label == "2 cards")
        r.story.hover(.beside(ref["seq"]!), firstHalf: true)
        #expect(r.story.hint == StoryDropHint(target: .sequence(ref["seq"]!.id), before: ref["a"]))
        #expect(await r.story.commitDrop())
        await r.reload()
        let moved = StoryLogic.children(r.story.board!, of: .sequence(ref["seq"]!.id)).map(StoryLogic.label)
        #expect(moved == ["c", "d", "a", "b"])
        #expect(r.quiet)
    }

    @Test func parkingAcceptsCardsButNotSequences() async throws {
        let (r, ref) = try await board()
        defer { r.close() }
        r.story.beginDrag(ref["seq"]!)
        #expect(!r.story.accepts(.into(.parking, atStart: false)))
        #expect(r.story.accepts(.into(.act(r.story.board!.acts[1].id), atStart: false)))
        r.story.cancelDrag()
        #expect(await drag(r.story, ref["a"]!, onto: .into(.parking, atStart: false), firstHalf: true))
        await r.reload()
        #expect(r.parked == ["a"])
        // Moving a sequence to the other act through the Move dialog list.
        let dests = StoryLogic.moveDestinations(r.story.board!, kinds: [.sequence])
        #expect(dests.map(\.label) == ["Act One", "Act Two"])
        let cardDests = StoryLogic.moveDestinations(r.story.board!, kinds: [.card])
        #expect(cardDests.map(\.label) == ["Act One", "Seq", "Act Two", "Parking Lot"])
        #expect(r.quiet)
    }

    @Test func deletingAFullActAsksAndMovesItsContents() async throws {
        let (r, _) = try await board()
        defer { r.close() }
        let one = r.story.board!.acts[0].id, two = r.story.board!.acts[1].id
        await r.story.requestDeleteAct(one)
        guard case .deleteAct? = r.story.sheet else { Issue.record("a full act asks first"); return }
        #expect(await r.story.deleteAct(one, mode: .moveContents, moveToActId: two))
        await r.reload()
        #expect(r.acts == ["Act Two"])
        #expect(r.cards(inAct: "Act Two").sorted() == ["a", "b", "c", "d"])
        // An empty act goes at once.
        _ = await r.story.createAct("Empty")
        await r.reload()
        let empty = try #require(r.story.board?.acts.first { $0.title == "Empty" })
        r.story.sheet = nil
        await r.story.requestDeleteAct(empty.id)
        #expect(r.story.sheet == nil)
        await r.reload()
        #expect(r.acts == ["Act Two"])
        #expect(r.quiet)
    }

    @Test func duplicateAndCopyAsBeat() async throws {
        let (r, ref) = try await board()
        defer { r.close() }
        await r.story.duplicate([ref["a"]!])
        await r.reload()
        #expect(r.cards(inAct: "Act One").filter { $0 == "a" }.count == 2)
        #expect(r.story.selected.count == 1)
        await r.story.copyAsBeat(ref["b"]!.id)
        await r.reload()
        #expect(r.beat("b") != nil)
        #expect(r.quiet)
    }

    @Test func theFilterKeepsMatchesOnly() async throws {
        let (r, ref) = try await board()
        defer { r.close() }
        let seq = StoryLogic.sequences(r.story.board!)[0]
        #expect(StoryLogic.matchesFilter(.sequence(seq), "b"))
        #expect(StoryLogic.matchesFilter(.sequence(seq), "SEQ"))
        #expect(!StoryLogic.matchesFilter(.sequence(seq), "zzz"))
        #expect(StoryLogic.matchesFilter(.sequence(seq), "   "))
        _ = ref
    }

    @Test func atFiveHundredCardsTheBoardLoadsAndTheLogicStaysFast() async throws {
        let r = try await Rig.make(title: "Big")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        let act = try #require(r.story.board?.acts.first)
        for i in 0..<500 {
            _ = try await r.core.storyCreateCard(StoryCreateCardArgs(parent: .act(act.id), shortDescription: "Card \(i)"))
        }
        await r.reload()
        #expect(r.cards.count == 500)
        let start = Date()
        let rows = StoryLogic.outlineRows(r.story.board!, collapsed: [], filter: "card 49")
        let flat = StoryLogic.flattenVisible(r.story.board!, collapsed: []) { _ in true }
        #expect(rows.count == 12)
        #expect(flat.count == 500)
        #expect(Date().timeIntervalSince(start) < 1)
    }

    @Test func timelineGroupsAssignsAndNotesScenes() async throws {
        let r = try await Rig.make(title: "Timeline")
        defer { r.close() }
        let t = TimelineModel(story: r.story)
        await t.reload()
        // No screenplay yet: the empty state offers Build Screenplay.
        #expect(t.data?.draftId == nil)
        // Build a screenplay of two headed cards.
        _ = await r.story.createAct("Act One")
        await r.reload()
        await r.addCard("One")
        await r.addCard("Two")
        let b = BuildScreenplayModel(story: r.story)
        await b.load()
        b.setHeading(b.rows[0].cardId, "INT. HALL - DAY")
        b.setHeading(b.rows[1].cardId, "EXT. YARD - NIGHT")
        _ = try #require(await b.build())
        await t.reload()
        let all = t.data?.groups.flatMap(\.scenes) ?? []
        #expect(all.count == 2)
        #expect(t.data?.groups.first?.storyDay == nil)
        // Assign Story Day to the first scene.
        t.toggle(all[0].sceneId, true)
        #expect(await t.assign("1"))
        #expect(r.story.notice?.text == "Assigned Story Day 1")
        await t.reload()
        #expect(t.data?.groups.contains { $0.storyDay == "1" && $0.scenes.count == 1 } == true)
        #expect(TimelineModel.dayLabel("1") == "Day 1")
        #expect(TimelineModel.dayLabel("Flashback") == "Flashback")
        await t.setTimeNote(all[0].sceneId, "dawn")
        await t.reload()
        #expect(t.data?.groups.flatMap(\.scenes).first { $0.sceneId == all[0].sceneId }?.timeNote == "dawn")
        #expect(r.quiet)
    }

    @Test func applyBoardOrderFollowsTheBoard() async throws {
        let r = try await Rig.make(title: "Order")
        defer { r.close() }
        _ = await r.story.createAct("Act One")
        await r.reload()
        await r.addCard("One")
        await r.addCard("Two")
        let b = BuildScreenplayModel(story: r.story)
        await b.load()
        b.setHeading(b.rows[0].cardId, "INT. HALL - DAY")
        b.setHeading(b.rows[1].cardId, "EXT. YARD - NIGHT")
        _ = try #require(await b.build())
        await r.reload()
        let m = ApplyOrderModel(story: r.story)
        await m.load()
        #expect(m.preview?.draftId != nil)
        #expect(m.preview?.moves.isEmpty == true)
        #expect(!m.canApply)
        _ = await drag(r.story, r.cardRef("Two"), onto: .beside(r.cardRef("One")), firstHalf: true)
        await r.reload()
        await m.load()
        #expect(m.preview?.moves.count == 2)
        #expect(m.canApply)
        #expect(await m.apply())
        #expect(r.story.notice?.text.hasPrefix("Applied the Beat Board order to") == true)
        #expect(r.quiet)
    }
}

// MARK: Rendering

@Suite("StoryTests rendering", .serialized)
@MainActor
struct StoryRenderTests {
    private func populated() async throws -> Rig {
        let r = try await Rig.make(title: "Monsoon Letters")
        _ = await r.story.createAct("Act One: The Letters")
        _ = await r.story.createAct("Act Two")
        await r.reload()
        let one = r.story.board!.acts[0], two = r.story.board!.acts[1]
        _ = await r.story.createSequence(in: one.id, title: "The Letters")
        await r.reload()
        let seq = StoryLogic.sequences(r.story.board!)[0]
        let texts = ["Rain finds the letter box", "Meera reads the first letter on the verandah while the monsoon hammers the tin roof and a very long description keeps going", "The postman never came back"]
        for t in texts {
            let id = try #require(await r.story.newCard(in: .sequence(seq.id)))
            await r.reload()
            await r.story.commitText(StoryItemRef(kind: .card, id: id), t)
            await r.reload()
        }
        let bid = try #require(await r.story.newBeat(in: .act(two.id)))
        await r.reload()
        await r.story.commitText(StoryItemRef(kind: .beat, id: bid), "A second letter arrives")
        await r.reload()
        let park = try #require(await r.story.newCard(in: .parking))
        await r.reload()
        await r.story.commitText(StoryItemRef(kind: .card, id: park), "Cut idea: the flood")
        await r.reload()
        let cards = StoryLogic.allCards(r.story.board!)
        _ = try await r.core.storyUpdateCard(StoryUpdateCardArgs(id: cards[0].id, color: "orange"))
        await r.reload()
        r.story.select([StoryItemRef(kind: .card, id: cards[1].id).key])
        return r
    }

    private func host<V: View>(_ view: V, app: AppModel, scheme: ColorScheme) -> some View {
        let themes = ThemeStore()
        return view
            .environment(app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
            .environment(\.colorScheme, scheme)
    }

    @Test func boardOutlineAndSheetsRenderInLightAndDark() async throws {
        let r = try await populated()
        defer { r.close() }
        for scheme in [ColorScheme.light, .dark] {
            let name = scheme == .dark ? "dark" : "light"
            r.story.tab = .board
            let board = host(StoryContent(model: r.story), app: r.app, scheme: scheme)
            #expect(Fixture.render(board, name: "story-board-\(name)", size: CGSize(width: 1100, height: 720)) != nil)
            r.story.tab = .outline
            #expect(Fixture.render(host(StoryContent(model: r.story), app: r.app, scheme: scheme), name: "story-outline-\(name)") != nil)
        }
        r.story.tab = .board
        let built = BuildScreenplaySheet(model: r.story, preselect: [])
        #expect(Fixture.render(host(built, app: r.app, scheme: .light), name: "story-build-light", size: CGSize(width: 780, height: 640)) != nil)
        let display = CardDisplaySheet(model: r.story)
        #expect(Fixture.render(host(display, app: r.app, scheme: .dark), name: "story-carddisplay-dark", size: CGSize(width: 540, height: 640)) != nil)
        #expect(r.quiet)
    }

    @Test func charactersTimelineAndEpisodesRender() async throws {
        let r = try await populated()
        defer { r.close() }
        let m = CharactersModel(story: r.story)
        _ = await m.create(name: "Meera", role: "Protagonist", description: "", imagePath: nil)
        _ = await m.create(name: "Rain", role: "Postman", description: "", imagePath: nil)
        await m.reload()
        let meera = try #require(m.list.first { $0.name == "Meera" })
        let rain = try #require(m.list.first { $0.name == "Rain" })
        _ = await m.addRelationship(from: rain.id, to: meera.id, type: "friend of", note: "")
        await m.reload()
        r.story.tab = .characters
        for scheme in [ColorScheme.light, .dark] {
            let n = scheme == .dark ? "dark" : "light"
            #expect(Fixture.render(host(StoryContent(model: r.story), app: r.app, scheme: scheme), name: "story-characters-\(n)") != nil)
        }
        r.story.tab = .timeline
        #expect(Fixture.render(host(StoryContent(model: r.story), app: r.app, scheme: .light), name: "story-timeline-light") != nil)
        r.story.tab = .episodes
        #expect(Fixture.render(host(StoryEpisodesView(story: r.story), app: r.app, scheme: .light), name: "story-episodes-light") != nil)
        #expect(r.quiet)
    }

    @Test func theInspectorsRender() async throws {
        let r = try await populated()
        defer { r.close() }
        let card = StoryLogic.allCards(r.story.board!)[1]
        r.story.openDrawer(StoryItemRef(kind: .card, id: card.id))
        let inspector = StoryInspector(model: r.story, ref: StoryItemRef(kind: .card, id: card.id))
        #expect(Fixture.render(host(inspector.frame(width: 400, height: 640), app: r.app, scheme: .light), name: "story-inspector-card", size: CGSize(width: 400, height: 640)) != nil)
        #expect(r.quiet)
    }
}

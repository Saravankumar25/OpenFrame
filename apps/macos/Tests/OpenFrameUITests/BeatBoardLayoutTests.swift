// Beat Board layout: Vertical columns or Horizontal rows (Windows commit 0f30921e,
// BoardView.test.tsx "Beat Board layout"). The choice is a view preference kept per person, an
// unknown stored value is Vertical, the arrow keys and the drop positions follow the layout, and
// the act menu says Move Up and Move Down in rows. Run against a real core on a temporary folder.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("BeatBoardLayoutTests", .serialized)
@MainActor
struct BeatBoardLayoutTests {
    @MainActor private struct Board {
        let root: URL
        let app: AppModel
        let story: StoryModel
        var refs: [String: StoryItemRef]
        func close() {
            story.setBoardLayout(.vertical)
            app.shutdown()
            try? FileManager.default.removeItem(at: root)
        }
    }

    /// Act One holds a sequence with cards c1 and c2; the Parking Lot holds beat b1.
    private func board() async throws -> Board {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        #expect(await app.createProject(CreateProjectArgs(title: "Rows", projectType: .featureFilm)))
        let story = StoryModel(app: app)
        await story.reload()
        _ = await story.createAct("Act One")
        await story.reload()
        let act = try #require(story.board?.acts.first)
        _ = await story.createSequence(in: act.id, title: "Seq")
        await story.reload()
        let seq = StoryLogic.sequences(story.board!)[0]
        var refs: [String: StoryItemRef] = ["s1": StoryItemRef(kind: .sequence, id: seq.id)]
        for t in ["c1", "c2"] {
            let id = try #require(await story.newCard(in: .sequence(seq.id)))
            await story.reload()
            await story.commitText(StoryItemRef(kind: .card, id: id), t)
            await story.reload()
            refs[t] = StoryItemRef(kind: .card, id: id)
        }
        let beat = try #require(await story.newBeat(in: .parking))
        await story.reload()
        await story.commitText(StoryItemRef(kind: .beat, id: beat), "b1")
        await story.reload()
        refs["b1"] = StoryItemRef(kind: .beat, id: beat)
        story.tab = .board
        return Board(root: root, app: app, story: story, refs: refs)
    }

    @Test func aFreshStoreStartsVerticalAndAnUnknownValueFallsBackToVertical() throws {
        let suite = "openframe.tests.boardLayout"
        let d = try #require(UserDefaults(suiteName: suite))
        defer { d.removePersistentDomain(forName: suite) }
        #expect(StoryBoardLayout.load(d) == .vertical)
        d.set("diagonal", forKey: StoryBoardLayout.key)
        #expect(StoryBoardLayout.load(d) == .vertical)
        StoryBoardLayout.horizontal.save(d)
        #expect(d.string(forKey: "of.story.boardLayout") == "horizontal")
        #expect(StoryBoardLayout.load(d) == .horizontal)
    }

    @Test func theSwitchIsRememberedForTheNextVisit() async throws {
        let b = try await board()
        defer { b.close() }
        b.story.setBoardLayout(.horizontal)
        #expect(b.story.horizontalBoard)
        #expect(UserDefaults.standard.string(forKey: StoryBoardLayout.key) == "horizontal")
        // A new visit (the app restarted) reads the stored choice back.
        #expect(StoryModel(app: b.app).boardLayout == .horizontal)
        b.story.setBoardLayout(.vertical)
        #expect(StoryModel(app: b.app).boardLayout == .vertical)
        // The Outline never changes with the layout.
        b.story.setBoardLayout(.horizontal)
        b.story.tab = .outline
        #expect(!b.story.horizontalBoard)
    }

    @Test func arrowKeysFollowTheLayout() async throws {
        let b = try await board()
        defer { b.close() }
        let s = b.story
        func key(_ k: String) -> String { b.refs[k]!.key }
        // Columns: Down is the next card in the act, Right jumps to the next column (the Parking Lot).
        s.arrow(.down, option: false, shift: false, from: b.refs["c1"]!)
        #expect(s.focusRequest == key("c2"))
        s.arrow(.right, option: false, shift: false, from: b.refs["c1"]!)
        #expect(s.focusRequest == key("b1"))

        s.setBoardLayout(.horizontal)
        // Rows: Right is the next card in the same act, Down jumps to the next row (the Parking Lot).
        s.arrow(.right, option: false, shift: false, from: b.refs["c1"]!)
        #expect(s.focusRequest == key("c2"))
        s.arrow(.left, option: false, shift: false, from: b.refs["c2"]!)
        #expect(s.focusRequest == key("c1"))
        s.arrow(.down, option: false, shift: false, from: b.refs["c1"]!)
        #expect(s.focusRequest == key("b1"))
        // Up from the Parking Lot row lands on the first item of the act row above.
        s.arrow(.up, option: false, shift: false, from: b.refs["b1"]!)
        #expect(s.focusRequest == key("s1"))
    }

    @Test func optionWithTheArrowMovesTheCardAlongTheRow() async throws {
        let b = try await board()
        defer { b.close() }
        let s = b.story
        s.setBoardLayout(.horizontal)
        // Option+Right in rows is Option+Down in columns: c1 moves after c2.
        await s.arrow(.right, option: true, shift: false, from: b.refs["c1"]!)?.value
        await s.reload()
        let seq = try #require(StoryLogic.sequences(s.board!).first)
        #expect(seq.items.map(\.ref) == [b.refs["c2"]!, b.refs["c1"]!])
        // Option+Up in rows changes focus only (it is Left in columns).
        #expect(s.arrow(.up, option: true, shift: false, from: b.refs["c1"]!) == nil)
    }

    @Test func dropPositionsFollowTheLayout() {
        // A card zone of 200 by 80; the pointer is left of its middle but low in it.
        let card = CGSize(width: 200, height: 80)
        let leftAndLow = CGPoint(x: 80, y: 60)
        // Columns: cards stack top to bottom, so a lower point means after.
        #expect(StoryBoardLayout.inFirstHalf(alongY: StoryBoardLayout.vertical.itemsAlongY, location: leftAndLow, size: card) == false)
        // Rows: cards flow left to right, so a point to the left means before.
        #expect(StoryBoardLayout.inFirstHalf(alongY: StoryBoardLayout.horizontal.itemsAlongY, location: leftAndLow, size: card) == true)
        let rightAndHigh = CGPoint(x: 130, y: 10)
        #expect(StoryBoardLayout.inFirstHalf(alongY: false, location: rightAndHigh, size: card) == false)
        #expect(StoryBoardLayout.inFirstHalf(alongY: true, location: rightAndHigh, size: card) == true)
        // Acts: columns compare along x, rows along y.
        let actRow = CGSize(width: 900, height: 160)
        let point = CGPoint(x: 710, y: 30)
        #expect(StoryBoardLayout.inFirstHalf(alongY: StoryBoardLayout.horizontal.actsAlongY, location: point, size: actRow) == true)
        #expect(StoryBoardLayout.inFirstHalf(alongY: StoryBoardLayout.vertical.actsAlongY, location: point, size: actRow) == false)
    }

    @Test func theActMenuNamesTheMoveInTheDirectionOfTheLayout() {
        #expect(StoryBoardLayout.vertical.moveEarlier == "Move Left")
        #expect(StoryBoardLayout.vertical.moveLater == "Move Right")
        #expect(StoryBoardLayout.horizontal.moveEarlier == "Move Up")
        #expect(StoryBoardLayout.horizontal.moveLater == "Move Down")
        #expect(StoryBoardLayout.horizontal.arrow(.left) == .up)
        #expect(StoryBoardLayout.horizontal.arrow(.right) == .down)
        #expect(StoryBoardLayout.horizontal.arrow(.up) == .left)
        #expect(StoryBoardLayout.horizontal.arrow(.down) == .right)
        #expect(StoryBoardLayout.vertical.arrow(.left) == .left)
    }

    @Test func dragAndDropWorksInRows() async throws {
        let b = try await board()
        defer { b.close() }
        let s = b.story
        s.setBoardLayout(.horizontal)
        // Drag c2 onto the left half of c1: it goes before c1.
        s.beginDrag(b.refs["c2"]!)
        s.hover(.beside(b.refs["c1"]!), firstHalf: StoryBoardLayout.inFirstHalf(
            alongY: s.boardLayout.itemsAlongY, location: CGPoint(x: 20, y: 70), size: CGSize(width: 220, height: 80)))
        #expect(await s.commitDrop())
        await s.reload()
        let seq = try #require(StoryLogic.sequences(s.board!).first)
        #expect(seq.items.map(\.ref) == [b.refs["c2"]!, b.refs["c1"]!])
    }

    @Test func theRowsRenderInLightAndDark() async throws {
        let b = try await board()
        defer { b.close() }
        b.story.setBoardLayout(.horizontal)
        let themes = ThemeStore()
        for scheme in [ColorScheme.light, .dark] {
            let view = StoryContent(model: b.story).environment(b.app).environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme)).environment(\.colorScheme, scheme)
            #expect(Fixture.render(view, name: "story-board-rows-\(scheme == .dark ? "dark" : "light")", size: CGSize(width: 1000, height: 700)) != nil)
        }
    }
}

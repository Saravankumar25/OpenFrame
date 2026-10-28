import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

/// Story opens on the Story page. A search hit on a board item or a character asks Story for the
/// right tab through the Story show intents.
@Suite("Story entry intents", .serialized)
@MainActor
struct StoryEntryIntentTests {
    private func hit(_ nav: [String: JSONValue]) -> SearchHit {
        SearchHit(store: .project, entityType: "story_scene_card", entityId: "c1", title: "Card", snippet: "",
                  context: "", nav: .object(nav), rank: 1)
    }

    @Test func aStoryHitPicksTheTabFromItsIds() {
        #expect(SearchRouting.target(for: hit(["workspace": .string("story"), "cardId": .string("c1")])).storyTab == .board)
        #expect(SearchRouting.target(for: hit(["workspace": .string("story"), "beatId": .string("b1")])).storyTab == .board)
        #expect(SearchRouting.target(for: hit(["workspace": .string("story"), "sub": .string("characters"), "characterId": .string("k1")])).storyTab == .characters)
        #expect(SearchRouting.target(for: hit(["workspace": .string("story")])).storyTab == .page)
        #expect(SearchRouting.target(for: hit(["workspace": .string("vault")])).storyTab == nil)
    }

    @Test func jumpingToAStoryHitFiresTheTabIntent() {
        defer { _ = ProjectIntents.shared.consume(StoryTab.board.showIntent) }
        SearchRouting.go(SearchTarget(workspace: .story, storyTab: .board))
        #expect(ProjectIntents.shared.pending == "story.show_board")
        #expect(ProjectIntents.shared.consume("story.show_board"))
    }
}

@Suite @MainActor struct StoryContinueTabTests {
    @Test func aContinuePlaceOpensTheStoryTabItNames() {
        #expect(ProjectNav.storyTab(["workspace": .string("story"), "sub": .string("board")]) == .board)
        #expect(ProjectNav.storyTab(["workspace": .string("story"), "sub": .string("outline")]) == .outline)
        #expect(ProjectNav.storyTab(["workspace": .string("story"), "sub": .string("nothing")]) == .page)
        #expect(ProjectNav.storyTab(["workspace": .string("story")]) == .page)
        #expect(ProjectNav.storyTab(["workspace": .string("story"), "params": .object(["characterId": .string("c1")])]) == .characters)
    }
}

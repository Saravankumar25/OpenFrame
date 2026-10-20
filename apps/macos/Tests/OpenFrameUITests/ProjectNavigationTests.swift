// The project's navigation follows the Windows structure: a flat sidebar of eight workspaces,
// Production with thirteen tabs, and Back and Forward through the pages visited.

import Testing
@testable import OpenFrameUI

@MainActor
struct ProjectNavigationTests {
    @Test func theSidebarIsTheWindowsListInTheWindowsOrder() {
        #expect(Workspace.sidebar.map(\.title) == ["Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"])
        // Notes and Tasks, Activity and Recently Deleted are reached from the project menu.
        for ws in [Workspace.notes, .activity, .recentlyDeleted] { #expect(!Workspace.sidebar.contains(ws)) }
        // Control+Command+1 to 8 follow the sidebar.
        #expect(Workspace.home.shortcut == "1")
        #expect(Workspace.production.shortcut == "6")
        #expect(Workspace.files.shortcut == "8")
        #expect(Workspace.notes.shortcut == nil)
    }

    @Test func productionHasTheThirteenWindowsTabsInOrder() {
        #expect(ProductionTab.allCases.map(\.title) == [
            "Overview", "Catalog", "Locations", "Cast & Crew", "Moodboards", "Storyboards", "Shot Lists",
            "Schedule", "Day Out of Days", "Daily View", "Sides", "Reports", "Budget",
        ])
        #expect(ProductionTab(windowsId: "cast-crew") == .castCrew)
        #expect(ProductionTab(windowsId: "shots") == .shotLists)
        #expect(ProductionTab(windowsId: "dood") == .dayOutOfDays)
        #expect(ProductionTab(windowsId: "daily") == .daily)
        #expect(ProductionTab(windowsId: "nowhere") == nil)
    }

    @Test func placesRoundTripAndOlderNamesStillResolve() {
        for ws in Workspace.allCases where ws != .production {
            #expect(ProjectPlace(key: ProjectPlace(ws).key) == ProjectPlace(ws))
        }
        for tab in ProductionTab.allCases {
            let p = ProjectPlace(.production, tab: tab)
            #expect(ProjectPlace(key: p.key) == p)
        }
        // A workspace that is not Production carries no tab; Production defaults to Overview.
        #expect(ProjectPlace(.screenplay, tab: .budget).tab == nil)
        #expect(ProjectPlace(.production).tab == .overview)
        // Saved state and links from before the Windows structure.
        #expect(ProjectPlace(key: "schedule") == ProjectPlace(.production, tab: .schedule))
        #expect(ProjectPlace(key: "budget") == ProjectPlace(.production, tab: .budget))
        #expect(ProjectPlace(key: "visual") == ProjectPlace(.production, tab: .moodboards))
        #expect(ProjectPlace(key: "callSheets") == ProjectPlace(.callSheets))
        #expect(ProjectPlace(key: "nowhere") == nil)
    }

    @Test func hubTabsLiveWhereWindowsHasThem() {
        #expect(BDTab.breakdown.place == ProjectPlace(.breakdown))
        #expect(BDTab.catalog.place == ProjectPlace(.production, tab: .catalog))
        #expect(BDTab.castCrew.place == ProjectPlace(.production, tab: .castCrew))
        #expect(CSTab.callSheets.place == ProjectPlace(.callSheets))
        #expect(CSTab.daily.place == ProjectPlace(.production, tab: .daily))
        #expect(SchedSection.dood.place == ProjectPlace(.production, tab: .dayOutOfDays))
        #expect(VisualTab.shots.place == ProjectPlace(.production, tab: .shotLists))
    }

    @Test func backAndForwardRetraceThePagesVisited() {
        var h = PlaceHistory()
        let home = ProjectPlace(.home), script = ProjectPlace(.screenplay), cast = ProjectPlace(.production, tab: .castCrew)
        #expect(!h.canGoBack && !h.canGoForward)
        h.moved(from: home, to: script)
        h.moved(from: script, to: cast)
        #expect(h.goBack(from: cast) == script)
        #expect(h.goBack(from: script) == home)
        #expect(h.goBack(from: home) == nil)
        #expect(h.goForward(from: home) == script)
        // A new move after going back drops the forward pages, as in a browser.
        h.moved(from: script, to: ProjectPlace(.files))
        #expect(!h.canGoForward)
        // Staying on the same page is not a move.
        let before = h
        h.moved(from: script, to: script)
        #expect(h == before)
    }

    @Test func historyKeepsAtMostFiftyPages() {
        var h = PlaceHistory()
        let places = Workspace.sidebar.map { ProjectPlace($0) }
        for i in 0..<120 { h.moved(from: places[i % places.count], to: places[(i + 1) % places.count]) }
        var steps = 0
        var at = places[0]
        while let p = h.goBack(from: at) { at = p; steps += 1 }
        #expect(steps == PlaceHistory.limit)
    }
}

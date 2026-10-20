// Where the person is inside a project, and how they move: the workspaces in the sidebar, the
// tabs of the Production workspace, the pages visited (Back and Forward) and the per-project
// models that two places share. The structure is the Windows one (apps/desktop/src/app/routes.tsx
// and workspaces/production): one flat list of eight workspaces, Production holding the thirteen
// planning tabs. Notes and Tasks, Activity and Recently Deleted are reached from the project menu.

import Foundation
import Observation
import SwiftUI

/// The workspaces of a project. The sidebar shows `Workspace.sidebar`, in that order.
enum Workspace: String, CaseIterable, Identifiable, Hashable {
    case home, ideaVault, story, screenplay, breakdown, production, callSheets, files
    case notes, activity, recentlyDeleted

    var id: String { rawValue }

    /// The sidebar, in the Windows order (routes.tsx).
    static let sidebar: [Workspace] = [.home, .ideaVault, .story, .screenplay, .breakdown, .production, .callSheets, .files]

    var title: String {
        switch self {
        case .home: "Home"
        case .ideaVault: "Idea Vault"
        case .story: "Story"
        case .screenplay: "Screenplay"
        case .breakdown: "Breakdown"
        case .production: "Production"
        case .callSheets: "Call Sheets"
        case .files: "Files"
        case .notes: "Notes and Tasks"
        case .activity: "Activity"
        case .recentlyDeleted: "Recently Deleted"
        }
    }

    var symbol: String {
        switch self {
        case .home: "house"
        case .ideaVault: "lightbulb"
        case .story: "rectangle.split.3x1"
        case .screenplay: "doc.text"
        case .breakdown: "checklist"
        case .production: "film"
        case .callSheets: "list.clipboard"
        case .files: "folder"
        case .notes: "checklist.checked"
        case .activity: "clock.arrow.circlepath"
        case .recentlyDeleted: "trash"
        }
    }

    /// Keyboard shortcut in the Go menu: Control+Command+1 to 8 in sidebar order. Command+1 to
    /// Command+8 belong to the screenplay elements, as in Mac screenwriting apps.
    var shortcut: KeyEquivalent? {
        guard let i = Workspace.sidebar.firstIndex(of: self) else { return nil }
        return KeyEquivalent(Character(String(i + 1)))
    }
}

/// The tabs of the Production workspace, in the Windows order (workspaces/production/tabs).
enum ProductionTab: String, CaseIterable, Identifiable, Hashable {
    case overview, catalog, locations, castCrew, moodboards, storyboards, shotLists
    case schedule, dayOutOfDays, daily, sides, reports, budget

    var id: String { rawValue }

    var title: String {
        switch self {
        case .overview: "Overview"
        case .catalog: "Catalog"
        case .locations: "Locations"
        case .castCrew: "Cast & Crew"
        case .moodboards: "Moodboards"
        case .storyboards: "Storyboards"
        case .shotLists: "Shot Lists"
        case .schedule: "Schedule"
        case .dayOutOfDays: "Day Out of Days"
        case .daily: "Daily View"
        case .sides: "Sides"
        case .reports: "Reports"
        case .budget: "Budget"
        }
    }
}

extension ProductionTab {
    /// The Windows id of a Production tab ("cast-crew", "shots", "dood"), as the core's search
    /// hits and AI links send it.
    init?(windowsId: String) {
        switch windowsId {
        case "cast-crew": self = .castCrew
        case "shots": self = .shotLists
        case "dood": self = .dayOutOfDays
        default: self.init(rawValue: windowsId)
        }
    }
}

/// One place in a project: a workspace and, for Production, its tab.
struct ProjectPlace: Hashable {
    var workspace: Workspace
    var tab: ProductionTab?

    init(_ workspace: Workspace, tab: ProductionTab? = nil) {
        self.workspace = workspace
        self.tab = workspace == .production ? (tab ?? .overview) : nil
    }

    var title: String {
        if let tab { return "\(workspace.title) \u{00B7} \(tab.title)" }
        return workspace.title
    }

    /// The text form used by `openFrameShowWorkspace` and the window's saved state:
    /// "screenplay", or "production/schedule". Names of the places the Mac used before the
    /// Windows structure ("schedule", "budget", "visual") still resolve, so older saved state,
    /// search results and AI links keep working.
    var key: String { tab.map { "\(workspace.rawValue)/\($0.rawValue)" } ?? workspace.rawValue }

    init?(key: String) {
        let parts = key.split(separator: "/", maxSplits: 1).map(String.init)
        if parts.count == 2, let ws = Workspace(rawValue: parts[0]), let tab = ProductionTab(rawValue: parts[1]) {
            self.init(ws, tab: tab)
            return
        }
        switch parts.first ?? "" {
        case "schedule": self.init(.production, tab: .schedule)
        case "dood": self.init(.production, tab: .dayOutOfDays)
        case "budget": self.init(.production, tab: .budget)
        case "visual", "moodboards": self.init(.production, tab: .moodboards)
        case "storyboards": self.init(.production, tab: .storyboards)
        case "shots": self.init(.production, tab: .shotLists)
        case "daily": self.init(.production, tab: .daily)
        case "sides": self.init(.production, tab: .sides)
        case "reports": self.init(.production, tab: .reports)
        case "catalog": self.init(.production, tab: .catalog)
        case "locations": self.init(.production, tab: .locations)
        case "castCrew": self.init(.production, tab: .castCrew)
        default:
            guard let ws = Workspace(rawValue: parts.first ?? "") else { return nil }
            self.init(ws)
        }
    }
}

/// The pages visited in this window, for Back and Forward (Windows: stores.ts history, at most
/// 50 entries). A move made by Back or Forward is not itself recorded.
struct PlaceHistory: Equatable {
    static let limit = 50
    private(set) var back: [ProjectPlace] = []
    private(set) var forward: [ProjectPlace] = []

    var canGoBack: Bool { !back.isEmpty }
    var canGoForward: Bool { !forward.isEmpty }

    /// Records a move from `from` to `to` made by the person (a sidebar click, a link).
    mutating func moved(from: ProjectPlace, to: ProjectPlace) {
        guard from != to else { return }
        back.append(from)
        if back.count > Self.limit { back.removeFirst(back.count - Self.limit) }
        forward.removeAll()
    }

    mutating func goBack(from current: ProjectPlace) -> ProjectPlace? {
        guard let target = back.popLast() else { return nil }
        forward.append(current)
        return target
    }

    mutating func goForward(from current: ProjectPlace) -> ProjectPlace? {
        guard let target = forward.popLast() else { return nil }
        back.append(current)
        return target
    }
}

/// Moves the open project's window to a place (the AI, search, links between workspaces).
@MainActor
enum ProjectNav {
    static func show(_ place: ProjectPlace) {
        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: place.key)
    }

    static func show(_ ws: Workspace, tab: ProductionTab? = nil) { show(ProjectPlace(ws, tab: tab)) }
}

/// The models of one open project that two places share, so a catalog item opened from the
/// Breakdown shows in Production > Catalog with what was chosen, and the Call Sheets list and
/// the Daily View read the same day. Made when first needed, dropped with the project.
@MainActor
@Observable
final class ProjectHubs {
    @ObservationIgnored let app: AppModel
    @ObservationIgnored private var breakdownHub: BreakdownHub?
    @ObservationIgnored private var callSheetsHub: CallSheetsHub?
    @ObservationIgnored private var scheduleModel: ScheduleModel?
    @ObservationIgnored private var visualHub: VisualHub?

    init(app: AppModel) { self.app = app }

    var breakdown: BreakdownHub {
        if let breakdownHub { return breakdownHub }
        let h = BreakdownHub(app: app)
        breakdownHub = h
        return h
    }

    var callSheets: CallSheetsHub {
        if let callSheetsHub { return callSheetsHub }
        let h = CallSheetsHub(app: app)
        callSheetsHub = h
        return h
    }

    var schedule: ScheduleModel {
        if let scheduleModel { return scheduleModel }
        let m = ScheduleModel(app: app)
        scheduleModel = m
        return m
    }

    var visual: VisualHub {
        if let visualHub { return visualHub }
        let h = VisualHub(app: app)
        visualHub = h
        return h
    }
}

// MARK: Where each hub tab lives

extension BDTab {
    /// Breakdown stays in the Breakdown workspace; Catalog, Locations and Cast & Crew are
    /// Production tabs (Windows); the category list is in Project Settings > Advanced.
    var place: ProjectPlace? {
        switch self {
        case .breakdown: ProjectPlace(.breakdown)
        case .catalog: ProjectPlace(.production, tab: .catalog)
        case .locations: ProjectPlace(.production, tab: .locations)
        case .castCrew: ProjectPlace(.production, tab: .castCrew)
        case .categories: nil
        }
    }
}

extension CSTab {
    var place: ProjectPlace {
        switch self {
        case .callSheets: ProjectPlace(.callSheets)
        case .daily: ProjectPlace(.production, tab: .daily)
        case .sides: ProjectPlace(.production, tab: .sides)
        case .reports: ProjectPlace(.production, tab: .reports)
        }
    }
}

extension SchedSection {
    var place: ProjectPlace { ProjectPlace(.production, tab: self == .schedule ? .schedule : .dayOutOfDays) }
}

extension VisualTab {
    var place: ProjectPlace {
        switch self {
        case .moodboards: ProjectPlace(.production, tab: .moodboards)
        case .storyboards: ProjectPlace(.production, tab: .storyboards)
        case .shots: ProjectPlace(.production, tab: .shotLists)
        }
    }
}

extension View {
    /// A hub shown with its own tab bar is one accessibility container. Shown as one place
    /// (Production > Schedule), it is not: with a single page inside, SwiftUI would fold the page's
    /// own container (and its name, "day-detail" for one) into the hub's.
    @ViewBuilder func hubContainer(_ id: String, standalone: Bool) -> some View {
        if standalone {
            accessibilityElement(children: .contain).accessibilityIdentifier(id)
        } else {
            self
        }
    }

    /// Keeps a shared hub on the tab this place shows, and follows the hub to the place of
    /// another tab when the page moves the person there (opening a catalog item from a scene).
    func followsHubTab<Tab: Equatable>(_ tab: Tab, fixed: Tab?, set: @escaping (Tab) -> Void, place: @escaping (Tab) -> ProjectPlace?) -> some View {
        modifier(HubTabFollower(tab: tab, fixed: fixed, set: set, place: place))
    }
}

private struct HubTabFollower<Tab: Equatable>: ViewModifier {
    let tab: Tab
    let fixed: Tab?
    let set: (Tab) -> Void
    let place: (Tab) -> ProjectPlace?

    func body(content: Content) -> some View {
        content
            .onAppear { if let fixed, tab != fixed { set(fixed) } }
            .onChange(of: tab) { _, new in
                guard let fixed, new != fixed else { return }
                if let target = place(new) {
                    ProjectNav.show(target)
                } else {
                    set(fixed)
                }
            }
    }
}

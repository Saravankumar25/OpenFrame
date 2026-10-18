// The open project: a sidebar of workspaces and the selected workspace. Each workspace reads
// from the core and reloads on the core's change events.

import SwiftUI
import OpenFrameCore

/// The workspaces of a project, in sidebar order. Workspaces of later Mac phases are listed
/// so the structure matches Windows, and say plainly that they are not on the Mac yet.
enum Workspace: String, CaseIterable, Identifiable, Hashable {
    case home, screenplay, story, ideaVault, breakdown
    case schedule, callSheets, budget, visual, files, notes, activity, recentlyDeleted

    var id: String { rawValue }

    var title: String {
        switch self {
        case .home: "Project Home"
        case .screenplay: "Screenplay"
        case .story: "Story"
        case .ideaVault: "Idea Vault"
        case .breakdown: "Breakdown"
        case .schedule: "Schedule"
        case .callSheets: "Call Sheets"
        case .budget: "Budget"
        case .visual: "Visual Planning"
        case .files: "Files"
        case .notes: "Notes and Tasks"
        case .activity: "Activity"
        case .recentlyDeleted: "Recently Deleted"
        }
    }

    var symbol: String {
        switch self {
        case .home: "house"
        case .screenplay: "doc.text"
        case .story: "rectangle.3.group"
        case .ideaVault: "lightbulb"
        case .breakdown: "list.bullet.rectangle"
        case .schedule: "calendar"
        case .callSheets: "doc.richtext"
        case .budget: "dollarsign.circle"
        case .visual: "photo.on.rectangle"
        case .files: "folder"
        case .notes: "checklist"
        case .activity: "clock.arrow.circlepath"
        case .recentlyDeleted: "trash"
        }
    }

    var section: String {
        switch self {
        case .home: "Project"
        case .screenplay, .story, .ideaVault: "Writing"
        case .breakdown, .schedule, .callSheets, .budget, .visual: "Production"
        case .files, .notes, .activity, .recentlyDeleted: "Project"
        }
    }

    /// The Mac phase that brings the workspace to the Mac (chapter-1b-mac-phases.md).
    var macPhase: String? {
        switch self {
        case .home, .screenplay, .story, .ideaVault, .breakdown: nil
        default: "M6"
        }
    }

    /// Keyboard shortcut in the Go menu: Control+Command+1 to 5 for the main workspaces. Command+1
    /// to Command+8 belong to the screenplay elements, as in Mac screenwriting apps.
    var shortcut: KeyEquivalent? {
        switch self {
        case .home: "1"
        case .screenplay: "2"
        case .story: "3"
        case .ideaVault: "4"
        case .breakdown: "5"
        default: nil
        }
    }
}

struct ProjectView: View {
    let project: ProjectSummary
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @SceneStorage("workspace") private var stored: String = Workspace.home.rawValue
    @State private var selection: Workspace? = .home

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                ForEach(["Writing", "Production", "Project"], id: \.self) { section in
                    Section(section) {
                        ForEach(Workspace.allCases.filter { $0.section == section }) { ws in
                            Label(ws.title, systemImage: ws.symbol)
                                .foregroundStyle(ws.macPhase == nil ? .primary : .secondary)
                                .tag(ws)
                                .accessibilityIdentifier("sidebar-\(ws.rawValue)")
                        }
                    }
                }
            }
            .navigationSplitViewColumnWidth(min: 190, ideal: 220, max: 300)
            .safeAreaInset(edge: .bottom) { SaveStatusView().padding(10) }
        } detail: {
            detail(selection ?? .home)
                .navigationTitle(project.title)
                .navigationSubtitle(selection?.title ?? "")
        }
        .toolbar {
            ToolbarItem(placement: .navigation) {
                Button {
                    Task { await app.closeProject() }
                } label: {
                    Label("All Projects", systemImage: "chevron.left")
                }
                .help("Close this project and show all projects")
                .accessibilityIdentifier("close-project")
            }
        }
        .onAppear { selection = Workspace(rawValue: stored) ?? .home }
        .onChange(of: selection) { _, new in stored = (new ?? .home).rawValue }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowScreenplayTarget)) { note in
            // Kept for the Screenplay workspace in case it is not on screen yet.
            let info = note.userInfo ?? [:]
            ScreenplayWorkspaceModel.pendingTarget = (info["screenplayId"] as? String, info["draftId"] as? String)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowWorkspace)) { note in
            if let raw = note.object as? String, let ws = Workspace(rawValue: raw) { selection = ws }
        }
    }

    @ViewBuilder private func detail(_ ws: Workspace) -> some View {
        switch ws {
        case .home: ProjectHomeView(project: project) { selection = $0 }
        case .screenplay: ScreenplayWorkspace()
        case .story: StoryWorkspace()
        case .ideaVault: IdeaVaultWorkspace(store: .project)
        case .breakdown: BreakdownWorkspace()
        default: NotOnMacYet(workspace: ws)
        }
    }
}

struct NotOnMacYet: View {
    let workspace: Workspace
    var body: some View {
        ContentUnavailableView {
            Label(workspace.title, systemImage: workspace.symbol)
        } description: {
            Text("\(workspace.title) is not on the Mac yet. It arrives in Mac phase \(workspace.macPhase ?? "M6"). Your data for it is safe in the project and opens on Windows.")
        }
        .accessibilityIdentifier("not-on-mac-\(workspace.rawValue)")
    }
}

/// Saved, Saving, or a save problem, from the core's save-state events.
struct SaveStatusView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: 6) {
            switch app.saveState?.status {
            case .saving?, .savedPendingExternal?:
                ProgressView().controlSize(.mini)
                Text("Saving")
            case .error?:
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(theme["red"])
                Text(app.saveState?.error?.message ?? "Not saved").lineLimit(2)
            default:
                Image(systemName: "checkmark.circle").foregroundStyle(theme["green"])
                Text("Saved")
            }
            Spacer(minLength: 0)
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("save-status")
    }
}

extension Notification.Name {
    static let openFrameShowWorkspace = Notification.Name("studio.openframe.showWorkspace")
}

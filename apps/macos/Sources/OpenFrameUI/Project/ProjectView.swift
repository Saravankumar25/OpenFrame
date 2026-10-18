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

    /// Keyboard shortcut in the Go menu: Control+Command+1 to 9 for the main workspaces. Command+1
    /// to Command+8 belong to the screenplay elements, as in Mac screenwriting apps.
    var shortcut: KeyEquivalent? {
        switch self {
        case .home: "1"
        case .screenplay: "2"
        case .story: "3"
        case .ideaVault: "4"
        case .breakdown: "5"
        case .schedule: "6"
        case .callSheets: "7"
        case .budget: "8"
        case .visual: "9"
        default: nil
        }
    }
}

struct ProjectView: View {
    let project: ProjectSummary
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    /// "<project id>|<workspace>": the window comes back to the workspace it showed for this
    /// project; another project (a new one, or one opened from Home) starts on Project Home.
    @SceneStorage("workspace") private var stored: String = ""
    /// The AI Assistant panel docks beside the page; it stays as the person left it.
    @SceneStorage("assistantOpen") private var assistantOpen = false
    @State private var selection: Workspace? = .home
    @State private var columns: NavigationSplitViewVisibility = .automatic
    /// The project as the core has it now (title and status change from rename, undo and redo).
    @State private var live: ProjectSummary?
    @State private var renaming: RecentProject?

    private var current: ProjectSummary { live ?? project }
    private var asRecent: RecentProject {
        RecentProject(
            projectId: current.id, path: current.path, title: current.title, projectType: current.projectType.rawValue,
            status: current.status, archived: current.archived, pinned: false, lastOpenedAt: 0, available: true)
    }

    var body: some View {
        NavigationSplitView(columnVisibility: $columns) {
            List(selection: $selection) {
                ForEach(["Writing", "Production", "Project"], id: \.self) { section in
                    Section(section) {
                        ForEach(Workspace.allCases.filter { $0.section == section }) { ws in
                            Label(ws.title, systemImage: ws.symbol)
                                .tag(ws)
                                .accessibilityIdentifier("sidebar-\(ws.rawValue)")
                        }
                    }
                }
            }
            .navigationSplitViewColumnWidth(min: 190, ideal: 220, max: 300)
            .safeAreaInset(edge: .bottom) { SaveStatusView().padding(10) }
        } detail: {
            // The panel docks after the page in a row (as on Windows), so opening it narrows the page
            // and never covers a control. A row, not an inspector: workspaces use inspectors of their own.
            HStack(spacing: 0) {
                detail(selection ?? .home)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                if assistantOpen {
                    Divider()
                    AIPanelView(workspace: selection ?? .home) { assistantOpen = false }
                        .frame(width: 360)
                        .transition(.move(edge: .trailing))
                }
            }
            .navigationTitle(current.title)
            .interchangeAnchor()
            .navigationSubtitle(selection?.title ?? "")
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) { assistantButton }
            ToolbarItem(placement: .primaryAction) { projectMenu }
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
        .task(id: app.changes.any) {
            if let now = try? await app.core?.projectCurrent(), now.id == project.id { live = now }
        }
        .sheet(isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } })) {
            if let recent = renaming { RenameProjectSheet(project: recent, actions: ProjectActions(app: app)) {} }
        }
        .onAppear {
            let parts = stored.split(separator: "|", maxSplits: 1).map(String.init)
            selection = parts.count == 2 && parts[0] == project.id ? Workspace(rawValue: parts[1]) ?? .home : .home
        }
        .onChange(of: selection) { _, new in stored = "\(project.id)|\((new ?? .home).rawValue)" }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameScreenplayFocus)) { note in
            columns = (note.object as? Bool) == true ? .detailOnly : .automatic
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowScreenplayTarget)) { note in
            // Kept for the Screenplay workspace in case it is not on screen yet.
            let info = note.userInfo ?? [:]
            ScreenplayWorkspaceModel.pendingTarget = (info["screenplayId"] as? String, info["draftId"] as? String, info["sceneId"] as? String)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameToggleAssistant)) { _ in assistantOpen.toggle() }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowWorkspace)) { note in
            if let raw = note.object as? String, let ws = Workspace(rawValue: raw) { selection = ws }
        }
    }

    /// The project menu: status, rename, duplicate, archive, show in Finder (Windows: the project menu in the top bar).
    /// Opens and closes the AI Assistant panel (Windows: the Assistant button and Ctrl+J).
    private var assistantButton: some View {
        Button {
            assistantOpen.toggle()
        } label: {
            Label("Assistant", systemImage: "sparkles")
        }
        .keyboardShortcut("j", modifiers: .command)
        .help(assistantOpen ? "Close AI Assistant (Command+J)" : "Open AI Assistant (Command+J)")
        .accessibilityLabel(assistantOpen ? "Close AI Assistant" : "Open AI Assistant")
        .accessibilityIdentifier("ai-toggle")
    }

    /// The project menu: status, rename, duplicate, archive, show in Finder (Windows: the project menu in the top bar).
    private var projectMenu: some View {
        let actions = ProjectActions(app: app)
        return Menu {
            Menu("Status: \(current.status)") {
                ForEach(ProjectStatus.allCases, id: \.self) { status in
                    Button {
                        Task { await actions.setStatus(status) }
                    } label: {
                        if status.rawValue == current.status { Label(status.rawValue, systemImage: "checkmark") } else { Text(status.rawValue) }
                    }
                }
            }
            Divider()
            Button("Rename\u{2026}") { renaming = asRecent }
            Button("Duplicate Project") { Task { await actions.duplicate(asRecent) } }
            Button("Show in Finder") { Task { await actions.reveal(projectId: current.id) } }
            Divider()
            Button("Recently Deleted") { selection = .recentlyDeleted }
            Button("Archive Project") {
                Task {
                    await actions.setArchived(asRecent, true)
                    await app.closeProject()
                }
            }
            Divider()
            Button("Search and Commands\u{2026}") { NotificationCenter.default.post(name: .openFrameShowSearch, object: nil) }
            Button("All Projects") { Task { await app.closeProject() } }
        } label: {
            Label(current.title, systemImage: "ellipsis.circle")
        }
        .help("Project actions")
        .accessibilityLabel("Project: \(current.title). Switch or manage projects")
        .accessibilityIdentifier("project-menu")
    }

    @ViewBuilder private func detail(_ ws: Workspace) -> some View {
        switch ws {
        case .home: ProjectHomeView(project: project) { selection = $0 }
        case .screenplay: ScreenplayWorkspace()
        case .story: StoryWorkspace()
        case .ideaVault: IdeaVaultWorkspace(store: .project)
        case .breakdown: BreakdownWorkspace()
        case .schedule: ScheduleWorkspace()
        case .callSheets: CallSheetsWorkspace()
        case .budget: BudgetWorkspace()
        case .visual: VisualPlanningWorkspace()
        case .files: FilesWorkspace()
        case .notes: NotesWorkspace()
        case .activity: ActivityWorkspace()
        case .recentlyDeleted: RecentlyDeletedWorkspace()
        }
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

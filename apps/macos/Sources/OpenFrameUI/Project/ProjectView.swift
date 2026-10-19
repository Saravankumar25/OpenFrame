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
    @State private var columns: NavigationSplitViewVisibility = .all
    /// The sidebar was hidden by Screenplay Focus (not by the person), so leaving Focus shows it.
    @State private var hiddenForFocus = false
    /// The project as the core has it now (title and status change from rename, undo and redo).
    @State private var live: ProjectSummary?
    @State private var renaming: RecentProject?
    /// Project Settings, as a sheet; the value is the page it opens on.
    @State private var settingsTab: ProjectSettingsTab?

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
            // The project's own sidebar button (below) has a name screen readers and tests can use.
            .toolbar(removing: .sidebarToggle)
        } detail: {
            // The panel docks after the page in a row (as on Windows), so opening it narrows the page
            // and never covers a control. A row, not an inspector: workspaces use inspectors of their own.
            // The reader gives the page exactly the room left: a page never reports a minimum
            // larger than the window, which would grow the window past the screen.
            GeometryReader { geo in
                HStack(spacing: 0) {
                    detail(selection ?? .home)
                        .frame(minWidth: 0, maxWidth: .infinity, minHeight: 0, maxHeight: .infinity)
                    if assistantOpen {
                        Divider()
                        AIPanelView(workspace: selection ?? .home) { assistantOpen = false }
                            .frame(width: Self.assistantWidth(for: geo.size.width))
                            .transition(.move(edge: .trailing))
                    }
                }
                .frame(width: geo.size.width, height: geo.size.height)
            }
            .navigationTitle(current.title)
            .interchangeAnchor()
            .navigationSubtitle(selection?.title ?? "")
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) { assistantButton }
            ToolbarItem(placement: .primaryAction) { projectSettingsButton }
            ToolbarItem(placement: .primaryAction) { projectMenu }
            ToolbarItem(placement: .navigation) { sidebarButton }
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
        .sheet(item: Binding(get: { settingsTab.map(SettingsSheetTarget.init) }, set: { settingsTab = $0?.tab })) { target in
            ProjectSettingsView(initialTab: target.tab)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowProjectSettings)) { note in
            settingsTab = (note.object as? String).flatMap(ProjectSettingsTab.init(rawValue:)) ?? .general
        }
        .onAppear {
            let parts = stored.split(separator: "|", maxSplits: 1).map(String.init)
            selection = parts.count == 2 && parts[0] == project.id ? Workspace(rawValue: parts[1]) ?? .home : .home
        }
        .onChange(of: selection) { _, new in stored = "\(project.id)|\((new ?? .home).rawValue)" }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameScreenplayFocus)) { note in
            // Focus hides the sidebar and leaving Focus brings it back; a sidebar the person hid
            // stays hidden (the Screenplay says "not in Focus" each time it closes).
            if (note.object as? Bool) == true {
                if columns != .detailOnly { hiddenForFocus = true; columns = .detailOnly }
            } else if hiddenForFocus {
                hiddenForFocus = false
                columns = .all
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowScreenplayTarget)) { note in
            // Kept for the Screenplay workspace in case it is not on screen yet.
            let info = note.userInfo ?? [:]
            ScreenplayWorkspaceModel.pendingTarget = (info["screenplayId"] as? String, info["draftId"] as? String, info["sceneId"] as? String)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameToggleAssistant)) { _ in toggleAssistant() }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameToggleSidebar)) { _ in setSidebar(shown: columns == .detailOnly) }
        .onChange(of: columns, initial: true) { _, now in ProjectSidebar.shared.shown = now != .detailOnly }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowWorkspace)) { note in
            if let raw = note.object as? String, let ws = Workspace(rawValue: raw) { selection = ws }
        }
    }

    /// The AI Assistant panel is 360 points wide, narrower in a narrow window so the page keeps
    /// room (at least 260 points for the panel).
    /// Opens or closes the AI panel. In a window too narrow for the sidebar, the page and the
    /// panel side by side, opening the panel hides the sidebar (as macOS split views do); the
    /// sidebar button brings it back.
    private func toggleAssistant() {
        assistantOpen.toggle()
        if assistantOpen, columns != .detailOnly,
           let width = NSApp.keyWindow?.contentLayoutRect.width ?? NSApp.mainWindow?.contentLayoutRect.width,
           width < 1040 {
            columns = .detailOnly
        }
    }

    static func assistantWidth(for detail: CGFloat) -> CGFloat {
        min(360, max(220, (detail * 0.4).rounded()))
    }

    /// Shows and hides the workspace sidebar (also View > Hide Sidebar, Command+Control+S).
    private var sidebarButton: some View {
        let shown = columns != .detailOnly
        // The target state is fixed when the button is drawn: an accessibility press on a toolbar
        // button runs its action twice, which a plain toggle would undo.
        return Button {
            setSidebar(shown: !shown)
        } label: {
            Label(shown ? "Hide Sidebar" : "Show Sidebar", systemImage: "sidebar.left")
        }
        .help(shown ? "Hide Sidebar (Command+Control+S)" : "Show Sidebar (Command+Control+S)")
        .accessibilityLabel(shown ? "Hide Sidebar" : "Show Sidebar")
        .accessibilityIdentifier("sidebar-toggle")
    }

    private func setSidebar(shown: Bool) {
        hiddenForFocus = false
        withAnimation { columns = shown ? .all : .detailOnly }
    }

    /// Project Settings, always visible in the top bar with its name (Windows: Project settings in the project menu).
    private var projectSettingsButton: some View {
        Button {
            settingsTab = .general
        } label: {
            Label("Project Settings", systemImage: "gearshape")
                .labelStyle(.titleAndIcon)
        }
        .help("Project Settings (\(ShortcutStore.shared.label("app.project_settings")))")
        .accessibilityLabel("Project Settings")
        .accessibilityIdentifier("project-settings-button")
    }

    /// Opens and closes the AI Assistant panel (Windows: the Assistant button and Ctrl+J).
    private var assistantButton: some View {
        Button {
            toggleAssistant()
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
            Button("Project Settings\u{2026}") { settingsTab = .general }
                .accessibilityIdentifier("project-menu-project-settings")
            Button("Settings\u{2026}") { SettingsRouter.open() }
                .accessibilityIdentifier("project-menu-app-settings")
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

/// Identifies the Project Settings page the sheet opens on.
private struct SettingsSheetTarget: Identifiable {
    let tab: ProjectSettingsTab
    var id: String { "project-settings" }
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
    /// View > Hide Sidebar or Show Sidebar (Command+Control+S).
    static let openFrameToggleSidebar = Notification.Name("studio.openframe.toggleSidebar")
}

/// Whether the open project shows its workspace sidebar, so the View menu can name its command.
@MainActor @Observable
final class ProjectSidebar {
    static let shared = ProjectSidebar()
    var shown = true
}

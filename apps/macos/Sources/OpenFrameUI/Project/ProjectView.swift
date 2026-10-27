// The open project: a sidebar of workspaces and the selected workspace (structure as on Windows,
// see ProjectNavigation.swift). Each workspace reads from the core and reloads on the core's
// change events. The toolbar carries All Projects, Back and Forward, the project menu, the AI
// Assistant and Settings, each with a visible name.

import SwiftUI
import OpenFrameCore

struct ProjectView: View {
    let project: ProjectSummary
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    /// "<project id>|<workspace>": the window comes back to the workspace it showed for this
    /// project; another project (a new one, or one opened from Home) starts on Project Home.
    @SceneStorage("workspace") private var stored: String = ""
    /// The AI Assistant panel docks beside the page; it stays as the person left it.
    @SceneStorage("assistantOpen") private var assistantOpen = false
    /// The width the person dragged the AI panel to (kept for this Mac).
    @AppStorage("openframe.ai.width") private var assistantPreferred: Double = Double(ProjectView.assistantDefaultWidth)
    /// The width at the start of a drag.
    @State private var dragStartWidth: CGFloat?
    @State private var selection: Workspace? = .home
    /// The Production tab shown (Windows: the Production subnav).
    @State private var productionTab: ProductionTab = .overview
    /// Pages visited, for Back and Forward.
    @State private var history = PlaceHistory()
    /// The place last shown, so a sidebar click is recorded as a move from it.
    @State private var shown = ProjectPlace(.home)
    /// Set while Back or Forward moves the window, so that move is not recorded again.
    @State private var replaying = false
    /// Screenplay Focus hides everything but the page (owner change list item 13).
    @State private var focus = false
    @State private var hubs: ProjectHubs?
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
        withNavigation(withSheets(splitView))
    }

    private var splitView: some View {
        NavigationSplitView(columnVisibility: $columns) {
            List(selection: $selection) {
                ForEach(Workspace.sidebar) { ws in
                    Label(ws.title, systemImage: ws.symbol)
                        .tag(ws)
                        .accessibilityIdentifier("sidebar-\(ws.rawValue)")
                }
            }
            .safeAreaInset(edge: .top) {
                // The project name sits over its pages, so the toolbar can keep every button.
                Text(current.title)
                    .font(.headline)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 16).padding(.top, 6).padding(.bottom, 2)
                    .accessibilityIdentifier("sidebar-project-title")
            }
            .safeAreaInset(edge: .bottom) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Project stored locally on this Mac")
                        .font(.caption)
                        .foregroundStyle(theme["muted"])
                        .fixedSize(horizontal: false, vertical: true)
                    SaveStatusView()
                }
                .padding(10)
            }
            // The project's own sidebar button (below) has a name screen readers and tests can use.
            .toolbar(removing: .sidebarToggle)
            // On the column's outermost view: on the list inside the insets, a first launch (no
            // saved width) ignored it and opened the sidebar 142 points wide.
            .navigationSplitViewColumnWidth(min: 190, ideal: 220, max: 300)
        } detail: {
            // The panel docks after the page in a row (as on Windows), so opening it narrows the page
            // and never covers a control. A row, not an inspector: workspaces use inspectors of their own.
            // The reader gives the page exactly the room left: a page never reports a minimum
            // larger than the window, which would grow the window past the screen.
            GeometryReader { geo in
                let panelShown = assistantOpen && !focus
                let pageWidth = geo.size.width
                    - (panelShown ? Self.assistantWidth(for: geo.size.width, preferred: assistantPreferred) + 1 : 0)
                HStack(spacing: 0) {
                    detail(selection ?? .home)
                        .frame(minWidth: 0, maxWidth: .infinity, minHeight: 0, maxHeight: .infinity)
                        .environment(\.pageWidth, pageWidth.rounded(.down))
                    if panelShown {
                        Divider()
                        let width = Self.assistantWidth(for: geo.size.width, preferred: assistantPreferred)
                        AIPanelView(workspace: selection ?? .home) { assistantOpen = false }
                            .frame(width: width)
                            .overlay(alignment: .leading) { assistantResizeHandle(current: width, detail: geo.size.width) }
                            .transition(.move(edge: .trailing))
                    }
                }
                .frame(width: geo.size.width, height: geo.size.height)
            }
            .navigationTitle(current.title)
            .interchangeAnchor()
            .navigationSubtitle(currentPlace.title)
            .environment(hubs)
        }
        .toolbar {
            ToolbarItem(placement: .navigation) { sidebarButton }
            ToolbarItem(placement: .navigation) {
                Button {
                    Task { await app.closeProject() }
                } label: {
                    Label("All Projects", systemImage: "square.grid.2x2").labelStyle(.titleAndIcon)
                        .labelStyle(.titleAndIcon)
                }
                .help("Close this project and show all projects")
                .accessibilityIdentifier("close-project")
            }
            ToolbarItem(placement: .navigation) { backForward }
            ToolbarItem(placement: .primaryAction) { projectMenu }
            ToolbarItem(placement: .primaryAction) { searchButton }
            ToolbarItem(placement: .primaryAction) { newMenu }
            ToolbarItem(placement: .primaryAction) { assistantButton }
            ToolbarItem(placement: .primaryAction) { projectSettingsButton }
        }
        .toolbar(focus ? .hidden : .automatic, for: .windowToolbar)
        .modifier(ToolbarWithoutTitle())
    }

    private func withSheets(_ content: some View) -> some View {
        content
        .task(id: app.changes.any) {
            if let now = try? await app.core?.projectCurrent(), now.id == project.id { live = now }
        }
        .themedSheet(isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } })) {
            if let recent = renaming { RenameProjectSheet(project: recent, actions: ProjectActions(app: app)) {} }
        }
        .themedSheet(item: Binding(get: { settingsTab.map(SettingsSheetTarget.init) }, set: { settingsTab = $0?.tab })) { target in
            ProjectSettingsView(initialTab: target.tab)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowProjectSettings)) { note in
            settingsTab = (note.object as? String).flatMap(ProjectSettingsTab.init(rawValue:)) ?? .general
        }
    }

    private func withNavigation(_ content: some View) -> some View {
        content
        .onAppear {
            if hubs == nil { hubs = ProjectHubs(app: app) }
            if let core = app.core { Task { await SpellingPreference.shared.load(from: core) } }
            let parts = stored.split(separator: "|", maxSplits: 1).map(String.init)
            let place = parts.count == 2 && parts[0] == project.id ? ProjectPlace(key: parts[1]) ?? ProjectPlace(.home) : ProjectPlace(.home)
            // Restoring the saved page is not a move the person made.
            if place != currentPlace { replaying = true }
            apply(place)
            shown = place
        }
        .onChange(of: currentPlace) { _, new in
            stored = "\(project.id)|\(new.key)"
            if replaying {
                replaying = false
            } else {
                history.moved(from: shown, to: new)
            }
            shown = new
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameScreenplayFocus)) { note in
            // Focus hides the sidebar, the toolbar and the AI panel, and leaving Focus brings them
            // back; a sidebar the person hid stays hidden (the Screenplay says "not in Focus"
            // each time it closes).
            focus = (note.object as? Bool) == true
            if focus {
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
            if let raw = note.object as? String, let place = ProjectPlace(key: raw) { apply(place) }
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameNavigateBack)) { _ in goBack() }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameNavigateForward)) { _ in goForward() }
        .onChange(of: history, initial: true) { _, h in
            ProjectSidebar.shared.canGoBack = h.canGoBack
            ProjectSidebar.shared.canGoForward = h.canGoForward
        }
    }

    /// The place on screen now.
    private var currentPlace: ProjectPlace { ProjectPlace(selection ?? .home, tab: productionTab) }

    private func apply(_ place: ProjectPlace) {
        if let tab = place.tab { productionTab = tab }
        selection = place.workspace
    }

    private func goBack() {
        guard let target = history.goBack(from: currentPlace) else { return }
        replaying = true
        apply(target)
        if target == currentPlace { replaying = false }
    }

    private func goForward() {
        guard let target = history.goForward(from: currentPlace) else { return }
        replaying = true
        apply(target)
        if target == currentPlace { replaying = false }
    }

    /// Back and Forward through the pages visited (Windows: the top-bar arrows, Alt+Left and
    /// Alt+Right; here Command+[ and Command+], as in Finder and Safari).
    private var backForward: some View {
        ControlGroup {
            Button {
                goBack()
            } label: {
                Label("Back", systemImage: "chevron.left").labelStyle(.titleAndIcon)
            }
            .disabled(!history.canGoBack)
            .help("Back (Command+[)")
            .accessibilityIdentifier("nav-back")
            Button {
                goForward()
            } label: {
                Label("Forward", systemImage: "chevron.right").labelStyle(.titleAndIcon)
            }
            .disabled(!history.canGoForward)
            .help("Forward (Command+])")
            .accessibilityIdentifier("nav-forward")
        }
        .controlGroupStyle(.navigation)
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

    /// The AI panel's width (Windows: AiPanel.tsx): what the person dragged it to (360 points at
    /// first), between 300 points and the smaller of 640 points and 45 percent of the window, so the
    /// page always keeps most of the room. A very narrow window still gets a 220-point panel.
    static let assistantDefaultWidth: CGFloat = 360

    static func assistantWidth(for detail: CGFloat, preferred: CGFloat = assistantDefaultWidth) -> CGFloat {
        let most = max(220, min(640, (detail * 0.45).rounded()))
        let least = min(300, most)
        return min(most, max(least, preferred.rounded()))
    }

    /// The left edge of the AI panel: drag to make it wider or narrower, double-click to go back to
    /// 360 points, or use the arrow keys (24 points a press) when it has the keyboard.
    private func assistantResizeHandle(current: CGFloat, detail: CGFloat) -> some View {
        Color.clear
            .frame(width: 6)
            .contentShape(Rectangle())
            .onHover { inside in if inside { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() } }
            .gesture(
                DragGesture(minimumDistance: 1)
                    .onChanged { g in
                        let start = dragStartWidth ?? current
                        dragStartWidth = start
                        assistantPreferred = Double(Self.assistantWidth(for: detail, preferred: start - g.translation.width))
                    }
                    .onEnded { _ in dragStartWidth = nil }
            )
            .onTapGesture(count: 2) { assistantPreferred = Double(Self.assistantDefaultWidth) }
            .focusable()
            .onKeyPress(.leftArrow) { assistantPreferred = Double(Self.assistantWidth(for: detail, preferred: current + 24)); return .handled }
            .onKeyPress(.rightArrow) { assistantPreferred = Double(Self.assistantWidth(for: detail, preferred: current - 24)); return .handled }
            .help("Drag to resize the AI Assistant. Double-click for the usual width.")
            .accessibilityElement()
            .accessibilityLabel("Resize the AI Assistant")
            .accessibilityValue("\(Int(current)) points")
            .accessibilityAdjustableAction { direction in
                let step: CGFloat = direction == .increment ? 24 : -24
                assistantPreferred = Double(Self.assistantWidth(for: detail, preferred: current + step))
            }
            .accessibilityIdentifier("ai-resize")
    }

    /// Shows and hides the workspace sidebar (also View > Hide Sidebar, Command+Control+S).
    private var sidebarButton: some View {
        let shown = columns != .detailOnly
        // The target state is fixed when the button is drawn: an accessibility press on a toolbar
        // button runs its action twice, which a plain toggle would undo.
        return Button {
            setSidebar(shown: !shown)
        } label: {
            Label(shown ? "Hide Sidebar" : "Show Sidebar", systemImage: "sidebar.left").labelStyle(.titleAndIcon)
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
            Label("Settings", systemImage: "gearshape").labelStyle(.titleAndIcon)
                .labelStyle(.titleAndIcon)
        }
        .help("Project Settings (\(ShortcutStore.shared.label("app.project_settings")))")
        .accessibilityLabel("Project Settings")
        .accessibilityIdentifier("project-settings-button")
    }

    /// Search this project and run commands (Windows: the top bar's search box; Command+K).
    private var searchButton: some View {
        Button {
            NotificationCenter.default.post(name: .openFrameShowSearch, object: nil)
        } label: {
            Label("Search", systemImage: "magnifyingglass").labelStyle(.titleAndIcon)
        }
        .help("Search this project (Command+K)")
        .accessibilityLabel("Search this project")
        .accessibilityIdentifier("project-search-button")
    }

    /// Create something new from any page (Windows: the top bar's + menu, in the same order).
    private var newMenu: some View {
        Menu {
            Section("Create") {
                newItem("New Idea", "vault.new_note", at: ProjectPlace(.ideaVault))
                newItem("New Act", "story.new_act", at: ProjectPlace(.story))
                newItem("New Sequence", "story.new_sequence", at: ProjectPlace(.story))
                newItem("New Beat", "story.new_beat", at: ProjectPlace(.story))
                newItem("New Scene Card", "story.new_scene", at: ProjectPlace(.story))
                newItem("New Character", "story.new_character", at: ProjectPlace(.story))
                newItem("New Location", "production.new_location", at: ProjectPlace(.production, tab: .locations))
                newItem("New Shot List", "production.new_shot", at: ProjectPlace(.production, tab: .shotLists))
                newItem("New Moodboard", "production.new_moodboard", at: ProjectPlace(.production, tab: .moodboards))
            }
            Divider()
            Button("Import Screenplay\u{2026}") {
                InterchangePresenter.shared.present(.importScreenplay(openScreenplayId: nil))
            }
            Divider()
            newItem("New Note", "notes.new_note", at: ProjectPlace(.notes))
            newItem("New Task", "notes.new_task", at: ProjectPlace(.notes))
            Divider()
            Button("New Project\u{2026}") {
                Task {
                    await app.closeProject()
                    try? await Task.sleep(for: .milliseconds(300))
                    NotificationCenter.default.post(name: .openFrameNewProject, object: nil)
                }
            }
        } label: {
            Label("New", systemImage: "plus").labelStyle(.titleAndIcon)
        }
        .help("Create an idea, act, beat, scene card, character, location, shot, moodboard, note or task")
        .accessibilityLabel("New")
        .accessibilityIdentifier("project-new-menu")
    }

    private func newItem(_ title: String, _ intent: String, at place: ProjectPlace) -> some View {
        Button(title) {
            ProjectIntents.shared.fire(intent)
            apply(place)
        }
    }

    /// Opens and closes the AI Assistant panel (Windows: the Assistant button and Ctrl+J).
    private var assistantButton: some View {
        Button {
            toggleAssistant()
        } label: {
            Label("Assistant", systemImage: "sparkles").labelStyle(.titleAndIcon)
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
            Button("Notes and Tasks") { selection = .notes }
                .accessibilityIdentifier("project-menu-notes")
            Button("Activity") { selection = .activity }
                .accessibilityIdentifier("project-menu-activity")
            Button("Recently Deleted") { selection = .recentlyDeleted }
                .accessibilityIdentifier("project-menu-recently-deleted")
            Divider()
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
            Label("Project", systemImage: "folder").labelStyle(.titleAndIcon)
        }
        .help("Project actions")
        .accessibilityLabel("Project: \(current.title). Switch or manage projects")
        .accessibilityIdentifier("project-menu")
    }

    @ViewBuilder private func detail(_ ws: Workspace) -> some View {
        switch ws {
        case .home: ProjectHomeView(project: project) { selection = $0 }
        case .ideaVault: IdeaVaultWorkspace(store: .project)
        case .story: StoryWorkspace()
        case .screenplay: ScreenplayWorkspace()
        case .breakdown: BreakdownWorkspace(fixed: .breakdown)
        case .production: ProductionWorkspace(tab: $productionTab)
        case .callSheets: CallSheetsWorkspace(fixed: .callSheets)
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
    /// Go > Back and Go > Forward (Command+[ and Command+]).
    static let openFrameNavigateBack = Notification.Name("studio.openframe.navigateBack")
    static let openFrameNavigateForward = Notification.Name("studio.openframe.navigateForward")
}

/// Whether the open project shows its workspace sidebar, so the View menu can name its command.
@MainActor @Observable
final class ProjectSidebar {
    static let shared = ProjectSidebar()
    var shown = true
    /// Whether Back and Forward have somewhere to go, so the Go menu can enable them.
    var canGoBack = false
    var canGoForward = false
}

/// The window keeps its title (Window menu, Mission Control, screen readers) but the toolbar does
/// not draw it: on a 1280-point screen the title pushed the Project, Search, New, Assistant and
/// Settings buttons into the overflow menu. The project name is at the top of the sidebar and the
/// page is marked in the sidebar.
private struct ToolbarWithoutTitle: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 15, *) {
            content.toolbar(removing: .title)
        } else {
            content
        }
    }
}

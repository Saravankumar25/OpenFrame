// Application Home: recent projects, New Project, Open, samples, and the data-location note.
// The project list always comes from the core (project.list_recent).

import AppKit
import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class HomeModel {
    var recents: [RecentProject] = []
    var showArchived = false
    /// Search projects: title, type and status.
    var query = ""
    var loading = true
    var failure: CoreFailure?

    func load(_ core: Core?) async {
        guard let core else { return }
        // The list is asked again a moment later when the core was busy (right after a project closes),
        // so Home never shows an empty list that is only a failed read.
        for attempt in 0..<4 {
            do {
                recents = try await core.projectListRecent(ListRecentArgs(includeArchived: showArchived))
                failure = nil
                break
            } catch {
                failure = CoreFailure(asCore: error)
                if attempt < 3 { try? await Task.sleep(for: .milliseconds(400)) }
            }
        }
        loading = false
    }

    private var shown: [RecentProject] {
        let q = query.trimmingCharacters(in: .whitespaces).lowercased()
        guard !q.isEmpty else { return recents }
        return recents.filter {
            $0.title.lowercased().contains(q) || $0.projectType.lowercased().contains(q) || $0.status.lowercased().contains(q)
        }
    }
    var pinned: [RecentProject] { shown.filter { $0.pinned && !$0.archived } }
    var others: [RecentProject] { shown.filter { !$0.pinned && !$0.archived } }
    var archived: [RecentProject] { shown.filter(\.archived) }
    var hasMatches: Bool { !pinned.isEmpty || !others.isEmpty || (showArchived && !archived.isEmpty) }
}

struct HomeView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var home: HomeModel
    @State private var showNewProject = false
    @State private var showVault = false
    @State private var sheet: HomeSheet?

    private enum HomeSheet: Identifiable {
        case rename(RecentProject), delete(RecentProject), unavailable(RecentProject)
        var id: String {
            switch self {
            case let .rename(p): "rename-\(p.projectId)"
            case let .delete(p): "delete-\(p.projectId)"
            case let .unavailable(p): "unavailable-\(p.projectId)"
            }
        }
    }

    /// A model that already holds its list is used by the screenshot tests, where tasks do not run off screen.
    init(model: HomeModel = HomeModel()) { _home = State(initialValue: model) }

    private var actions: ProjectActions { ProjectActions(app: app) }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: PageSpacing.large) {
                    header
                    if home.loading {
                        ProgressView().frame(maxWidth: .infinity)
                    } else if let failure = home.failure, home.recents.isEmpty {
                        VStack(spacing: PageSpacing.small) {
                            Text("Your projects could not be read.").font(.headline)
                            Text(failure.message).foregroundStyle(.secondary).multilineTextAlignment(.center)
                            Button("Try Again") { reload() }
                        }
                        .frame(maxWidth: .infinity)
                        .accessibilityIdentifier("home-load-error")
                    } else if home.recents.isEmpty {
                        emptyState
                    } else if !home.hasMatches {
                        noMatches
                    } else {
                        projectSections
                    }
                    DataLocationNote()
                }
                .padding(.vertical, PageSpacing.page)
                .pageColumn()
            }
            .background(theme["bg"])
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    Button { showVault = true } label: { Label("Global Idea Vault", systemImage: "lightbulb") }
                        .help("Your own Idea Vault, outside any project")
                        .accessibilityIdentifier("home-global-vault")
                    Button { openExisting() } label: { Label("Open", systemImage: "folder") }
                        .help("Open a project folder")
                    Button { InterchangePresenter.shared.present(.openProject) } label: { Label("Restore Backup or Package\u{2026}", systemImage: "archivebox") }
                        .help("Open a Full Project Package or restore a backup")
                        .accessibilityIdentifier("home-restore-package")
                    Button { showNewProject = true } label: { Label("New Project", systemImage: "plus") }
                        .help(ShortcutStore.shared.tooltipWords("New Project", "home.new_project"))
                        .accessibilityIdentifier("home-new-project")
                }
                // Settings with its name, so it is found at a glance (Command+Comma does the same).
                ToolbarItem(placement: .primaryAction) {
                    Button { SettingsRouter.open() } label: {
                        Label("Settings", systemImage: "gearshape").labelStyle(.titleAndIcon)
                    }
                    .help("Settings (Command+Comma)")
                    .accessibilityLabel("Settings")
                    .accessibilityIdentifier("home-settings")
                }
                ToolbarItem(placement: .automatic) {
                    Toggle("Show archived", isOn: $home.showArchived)
                        .toggleStyle(.checkbox)
                }
            }
            .navigationTitle("Projects")
            .searchable(text: $home.query, placement: .toolbar, prompt: "Search projects")
        }
        .interchangeAnchor()
        .sheet(isPresented: $showNewProject) { NewProjectSheet() }
        .sheet(item: $sheet) { sheet in
            switch sheet {
            case let .rename(p): RenameProjectSheet(project: p, actions: actions) { reload() }
            case let .delete(p): DeleteProjectSheet(project: p, actions: actions) { reload() }
            case let .unavailable(p): UnavailableProjectSheet(project: p, actions: actions) { reload() }
            }
        }
        .sheet(isPresented: $showVault) {
            NavigationStack {
                GlobalIdeaVaultView()
                    .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Done") { showVault = false } } }
            }
            .sheetFrame(minWidth: 900, minHeight: 620)
        }
        .task(id: TaskKey(running: app.isRunning, archived: home.showArchived, changes: app.changes.any)) {
            await home.load(app.core)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameNewProject)) { _ in showNewProject = true }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameOpenProject)) { _ in openExisting() }
    }

    private struct TaskKey: Hashable { let running: Bool; let archived: Bool; let changes: Int }

    private var header: some View {
        VStack(spacing: PageSpacing.small) {
            BrandMark(size: 48)
            Text("OpenFrame Studio")
                .font(.system(size: 26, weight: .semibold))
                .tracking(-0.3)
                .multilineTextAlignment(.center)
            if let name = app.info?.profile.displayName {
                Text("Signed in on this Mac as \(name)")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity)
    }

    private var emptyState: some View {
        VStack(spacing: PageSpacing.medium) {
            Text("Start your first project")
                .font(.title3.weight(.semibold))
            Text("A project holds one film or series: its screenplay, story, breakdown and production plans, in one folder on this Mac.")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 12) {
                Button("New Project") { showNewProject = true }
                    .keyboardShortcut("n", modifiers: .command)
                    .controlSize(.large)
                    .buttonStyle(.borderedProminent)
                Button("Open a Project Folder") { openExisting() }
                    .controlSize(.large)
            }
        }
        .padding(PageSpacing.large)
        .frame(maxWidth: .infinity)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .accessibilityIdentifier("home-empty")
    }

    @ViewBuilder private var projectSections: some View {
        if !home.pinned.isEmpty { section("Pinned", home.pinned) }
        if !home.others.isEmpty { section(home.pinned.isEmpty ? "Recent" : "Other projects", home.others) }
        if home.showArchived && !home.archived.isEmpty { section("Archived", home.archived) }
    }

    private var noMatches: some View {
        VStack(spacing: 6) {
            Text("No projects match \u{201C}\(home.query)\u{201D}.").font(.headline)
            Text("Search looks at the title, the type and the status.").foregroundStyle(.secondary)
        }
        .multilineTextAlignment(.center)
        .frame(maxWidth: .infinity)
        .accessibilityIdentifier("home-no-matches")
    }

    private func reload() { Task { await home.load(app.core) } }

    private func section(_ title: String, _ items: [RecentProject]) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            // The cards are centred as a group, so their caption is centred over them.
            Text(title)
                .font(.headline)
                .accessibilityAddTraits(.isHeader)
                .frame(maxWidth: .infinity)
                .padding(.top, PageSpacing.large)
                .padding(.bottom, PageSpacing.small)
            CenteredCards(cardWidth: 240) {
                ForEach(items, id: \.projectId) { project in
                    ProjectCard(project: project, handlers: handlers(for: project))
                }
            }
        }
    }

    private func handlers(for project: RecentProject) -> ProjectCard.Handlers {
        ProjectCard.Handlers(
            open: {
                if project.available {
                    Task { await app.openProject(path: project.path) }
                } else {
                    sheet = .unavailable(project)
                }
            },
            rename: { sheet = .rename(project) },
            duplicate: { Task { await actions.duplicate(project); reload() } },
            pin: { pinned in
                Task {
                    _ = try? await app.core?.projectSetPinned(SetPinnedArgs(projectId: project.projectId, pinned: pinned))
                    reload()
                }
            },
            reveal: { Task { await actions.reveal(projectId: project.projectId) } },
            setArchived: { value in Task { await actions.setArchived(project, value); reload() } },
            delete: { sheet = .delete(project) },
            locate: { sheet = .unavailable(project) },
            remove: { Task { await actions.forget(project); reload() } }
        )
    }

    private func openExisting() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Open"
        panel.message = "Choose an OpenFrame project folder"
        if panel.runModal() == .OK, let url = panel.url {
            Task { await app.openProject(path: url.path) }
        }
    }
}

struct ProjectCard: View {
    struct Handlers {
        var open: () -> Void
        var rename: () -> Void
        var duplicate: () -> Void
        var pin: (Bool) -> Void
        var reveal: () -> Void
        var setArchived: (Bool) -> Void
        var delete: () -> Void
        var locate: () -> Void
        var remove: () -> Void
    }

    let project: RecentProject
    let handlers: Handlers
    @Environment(\.ofTheme) private var theme
    @State private var hovering = false

    var body: some View {
        Button(action: handlers.open) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(project.title)
                        .font(.headline)
                        .lineLimit(2)
                    Spacer(minLength: 4)
                    if project.pinned {
                        Image(systemName: "pin.fill").foregroundStyle(.secondary).accessibilityHidden(true)
                    }
                }
                Text("\(project.projectType) \u{00B7} \(project.status)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 0)
                if project.available {
                    Text("Opened \(Formatting.relative(ms: project.lastOpenedAt))")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                } else {
                    Label("Folder not found. Reconnect the drive or locate the project.", systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundStyle(theme["yellow-t"])
                }
            }
            .padding(16)
            .frame(maxWidth: .infinity, minHeight: 112, alignment: .topLeading)
            .background(theme[hovering ? "hover" : "panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
        .onHover { hovering = $0 }
        .contextMenu {
            Button("Open") { handlers.open() }
            Button("Rename\u{2026}") { handlers.rename() }.disabled(!project.available)
            Button("Duplicate Project") { handlers.duplicate() }.disabled(!project.available)
            Button(project.pinned ? "Unpin" : "Pin") { handlers.pin(!project.pinned) }
            Button("Show in Finder") { handlers.reveal() }.disabled(!project.available)
            if !project.available { Button("Locate Project\u{2026}") { handlers.locate() } }
            Divider()
            if project.archived {
                Button("Restore from Archive") { handlers.setArchived(false) }
            } else {
                Button("Archive") { handlers.setArchived(true) }
            }
            Button("Remove from Recent") { handlers.remove() }
            Divider()
            Button("Delete Project\u{2026}", role: .destructive) { handlers.delete() }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Open \(project.title), \(project.projectType), \(project.status)\(project.pinned ? ", pinned" : "")\(project.archived ? ", archived" : "")\(project.available ? "" : ", unavailable")")
        .accessibilityHint("Opens the project")
        .accessibilityIdentifier("project-card-\(project.title)")
    }
}

/// Press feedback on the press itself, with a critically damped spring (no bounce).
struct PressableCardStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.98 : 1)
            .opacity(configuration.isPressed ? 0.92 : 1)
            .animation(.spring(response: 0.2, dampingFraction: 1), value: configuration.isPressed)
    }
}

/// Where the work lives and what to back up (shown on Home, as on Windows).
struct DataLocationNote: View {
    @Environment(AppModel.self) private var app

    var body: some View {
        if let info = app.info {
            VStack(spacing: 6) {
                Label("Your work is stored on this Mac", systemImage: "internaldrive")
                    .font(.subheadline.weight(.semibold))
                Text("Projects are kept in \(info.projectsDir). Back up the OpenFrame folder in Documents to keep a copy of everything.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                Button("Show in Finder") {
                    Task { try? await app.core?.revealLocation("dataRoot") }
                }
                .buttonStyle(.link)
            }
            .frame(maxWidth: .infinity)
            .accessibilityIdentifier("data-location-note")
        }
    }
}

extension Notification.Name {
    static let openFrameNewProject = Notification.Name("studio.openframe.newProject")
    static let openFrameOpenProject = Notification.Name("studio.openframe.openProject")
}

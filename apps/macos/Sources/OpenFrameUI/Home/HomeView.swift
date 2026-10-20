// Application Home (Windows: app/home/AppHome.tsx), in Mac styling: "Your projects" with Open
// Project, Restore Backup or Package and New Project; Pinned and Recent projects as cards with
// their type and status; Open Archived for a page of archived projects with Restore and Open;
// the Global Idea Vault, Settings and your name in the toolbar; a status line at the bottom.
// The project list always comes from the core (project.list_recent).

import AppKit
import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class HomeModel {
    var recents: [RecentProject] = []
    /// The Archived projects page is shown (Windows: Open Archived).
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
                // Archived projects are always read, so Open Archived can say how many there are.
                recents = try await core.projectListRecent(ListRecentArgs(includeArchived: true))
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
    /// Archived projects matching the search, on the Archived page only.
    var archived: [RecentProject] { showArchived ? shown.filter(\.archived) : [] }
    /// How many projects are archived (the Open Archived button says it).
    var archivedCount: Int { recents.filter(\.archived).count }
    var activeCount: Int { recents.filter { !$0.archived }.count }
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
                    if home.loading {
                        ProgressView().frame(maxWidth: .infinity).padding(.top, PageSpacing.page)
                    } else if let failure = home.failure, home.recents.isEmpty {
                        VStack(spacing: PageSpacing.small) {
                            Text("Your project list couldn't be loaded.").font(.headline)
                            Text("Your projects are safe on this computer. \(failure.message)").foregroundStyle(.secondary).multilineTextAlignment(.center)
                            Button("Try Again") { reload() }
                        }
                        .frame(maxWidth: .infinity)
                        .accessibilityIdentifier("home-load-error")
                    } else if home.recents.isEmpty {
                        emptyState
                    } else if home.showArchived {
                        archivedPage
                    } else {
                        projectsPage
                    }
                    DataLocationNote()
                }
                .padding(.vertical, PageSpacing.page)
                .pageColumn(maxWidth: PageSpacing.wideColumn)
            }
            .background(theme["bg"])
            .safeAreaInset(edge: .bottom, spacing: 0) { statusLine }
            .toolbar {
                ToolbarItem(placement: .navigation) {
                    HStack(spacing: 6) {
                        BrandMark(size: 20)
                        Text("OpenFrame").font(.headline)
                    }
                    .accessibilityElement(children: .combine)
                    .accessibilityLabel("OpenFrame Studio")
                }
                ToolbarItemGroup(placement: .primaryAction) {
                    Button { showVault = true } label: { Label("Global Idea Vault", systemImage: "lightbulb").labelStyle(.titleAndIcon) }
                        .help("Your ideas outside any project")
                        .accessibilityIdentifier("home-global-vault")
                    // Settings with its name (owner change list item 6). On Home it holds the settings
                    // of this Mac; Command+Comma does the same.
                    Button { SettingsRouter.open() } label: { Label("Settings", systemImage: "gearshape").labelStyle(.titleAndIcon) }
                        .help("Settings: appearance, keyboard shortcuts, Offline AI, about (Command+Comma)")
                        .accessibilityIdentifier("home-settings")
                    Button { SettingsRouter.open(tab: .general) } label: {
                        Label(profileName.isEmpty ? "Your Name" : profileName, systemImage: "person.crop.circle").labelStyle(.titleAndIcon)
                    }
                    .help("Your name, shown on comments and in the history. Click to change it.")
                    .accessibilityLabel("Your name: \(profileName). Change")
                    .accessibilityIdentifier("home-your-name")
                }
            }
            .navigationTitle("")
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

    private var profileName: String { app.info?.profile.displayName ?? "" }

    /// The line at the bottom of the window (Windows: the status bar on Home).
    private var statusLine: some View {
        HStack(spacing: 6) {
            Circle().fill(theme["muted"]).frame(width: 7, height: 7).accessibilityHidden(true)
            Text("No project open")
            Spacer()
            Label("Offline \u{00B7} Your projects stay on this computer", systemImage: "internaldrive")
        }
        .font(.caption)
        .foregroundStyle(theme["muted"])
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, 6)
        .background(theme["panel"])
        .overlay(alignment: .top) { Divider() }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("home-status")
    }

    /// The three ways in, with their names (Windows: Open Project, Restore Backup or Package, New
    /// Project), after the project search (Windows keeps it in the page header too).
    @ViewBuilder private var pageActions: some View {
        ProjectSearchField(text: $home.query)
            .frame(width: 220)
        Button { openExisting() } label: { Label("Open Project", systemImage: "folder").labelStyle(.titleAndIcon) }
            .buttonStyle(.bordered)
            .help(ShortcutStore.shared.tooltipWords("Open Project", "home.open_project"))
            .accessibilityIdentifier("home-open-project")
        Button { InterchangePresenter.shared.present(.openProject) } label: { Label("Restore Backup or Package\u{2026}", systemImage: "archivebox").labelStyle(.titleAndIcon) }
            .buttonStyle(.borderless)
            .foregroundStyle(.primary)
            .help("Restore a backup or open a project package")
            .accessibilityIdentifier("home-restore-package")
        Button { showNewProject = true } label: { Label("New Project", systemImage: "plus").labelStyle(.titleAndIcon) }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut("n", modifiers: .command)
            .help(ShortcutStore.shared.tooltipWords("New Project", "home.new_project"))
            .accessibilityIdentifier("home-new-project")
    }

    /// As on Windows: the title and its line on the left, search and the three buttons in one row
    /// on the right; in a narrow window the buttons go under the title instead of off the edge.
    private var homeHeader: some View {
        let titleBlock = VStack(alignment: .leading, spacing: 4) {
            Text("Your projects")
                .font(.system(size: 24, weight: .semibold))
                .tracking(-0.3)
                .accessibilityAddTraits(.isHeader)
            Text("Pick up where you left off, or start something new.")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        return ViewThatFits(in: .horizontal) {
            HStack(alignment: .center, spacing: PageSpacing.medium) {
                titleBlock.fixedSize()
                Spacer(minLength: PageSpacing.medium)
                HStack(spacing: 10) { pageActions }.fixedSize()
            }
            VStack(alignment: .leading, spacing: PageSpacing.medium) {
                titleBlock
                BDFlowLayout(spacing: 10) { pageActions }
            }
        }
        .padding(.top, PageSpacing.page)
        .padding(.bottom, PageSpacing.small)
    }

    private var projectsPage: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            homeHeader
            if !home.hasMatches {
                noMatches
            } else {
                if !home.pinned.isEmpty {
                    section("Pinned", home.pinned, trailing: nil)
                }
                if home.query.trimmingCharacters(in: .whitespaces).isEmpty || !home.others.isEmpty {
                    section(home.query.isEmpty ? "Recent projects" : "Search results", home.others, trailing: home.query.isEmpty ? openArchivedButton : nil)
                }
            }
        }
    }

    private var openArchivedButton: AnyView {
        AnyView(
            Button { home.showArchived = true } label: {
                Label("Open Archived\(home.archivedCount > 0 ? " (\(home.archivedCount))" : "")", systemImage: "archivebox").labelStyle(.titleAndIcon)
            }
            .buttonStyle(.borderless)
            .help("Archived projects are not deleted. Open them here to restore one.")
            .accessibilityIdentifier("home-open-archived")
        )
    }

    /// Archived projects (Windows: the Archived page): each with Restore and Open.
    private var archivedPage: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            PageHeader("Archived projects", subtitle: "Archived projects are not deleted. They stay on your computer and can be restored at any time.") {
                ProjectSearchField(text: $home.query)
                    .frame(width: 220)
                Button { home.showArchived = false } label: { Label("Back to projects", systemImage: "chevron.left").labelStyle(.titleAndIcon) }
                    .accessibilityIdentifier("home-back-to-projects")
            }
            VStack(spacing: 0) {
                if home.archived.isEmpty {
                    Text(home.archivedCount == 0
                         ? "No archived projects. Archive a project from its menu to tidy your list without deleting anything."
                         : "No archived projects match \u{201C}\(home.query)\u{201D}.")
                        .foregroundStyle(theme["muted"])
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: .infinity)
                        .padding(PageSpacing.large)
                }
                ForEach(home.archived, id: \.projectId) { p in
                    archivedRow(p)
                    if p.projectId != home.archived.last?.projectId { Divider() }
                }
            }
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
        }
    }

    private func archivedRow(_ p: RecentProject) -> some View {
        let h = handlers(for: p)
        return HStack(spacing: PageSpacing.small) {
            Button(action: h.open) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(p.title).fontWeight(.semibold).foregroundStyle(theme["ink"])
                    Text("\(p.projectType) \u{00B7} Last modified \(Formatting.relative(ms: p.modifiedAt ?? p.lastOpenedAt))")
                        .font(.caption).foregroundStyle(theme["muted"])
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Open \(p.title), \(p.projectType), \(p.status), archived\(p.available ? "" : ", unavailable")")
            .accessibilityIdentifier("project-card-\(p.title)")
            .contextMenu { ProjectCardMenu(project: p, handlers: h) }
            Spacer()
            BDChip(text: "Archived")
            if !p.available { BDChip(text: "Unavailable", tone: .red) }
            Button("Restore") { h.setArchived(false) }
                .accessibilityLabel("Restore \(p.title)")
            Button("Open") { h.open() }
                .accessibilityLabel("Open \(p.title)")
        }
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, 10)
    }

    private struct TaskKey: Hashable { let running: Bool; let archived: Bool; let changes: Int }

    private var emptyState: some View {
        VStack(spacing: PageSpacing.medium) {
            Text("Create your first project.")
                .font(.title3.weight(.semibold))
            Text("A project holds your ideas, beat board, screenplay and production plan for one film. Everything is stored on this computer.")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 12) {
                Button { showNewProject = true } label: { Label("Create Project", systemImage: "plus").labelStyle(.titleAndIcon) }
                    .keyboardShortcut("n", modifiers: .command)
                    .controlSize(.large)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("home-new-project")
                Button { openExisting() } label: { Label("Open Existing Project", systemImage: "folder").labelStyle(.titleAndIcon) }
                    .controlSize(.large)
                Button { InterchangePresenter.shared.present(.openProject) } label: { Label("Restore Backup or Package\u{2026}", systemImage: "archivebox").labelStyle(.titleAndIcon) }
                    .controlSize(.large)
            }
        }
        .padding(PageSpacing.large)
        .frame(maxWidth: .infinity)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("home-empty")
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

    private func section(_ title: String, _ items: [RecentProject], trailing: AnyView?) -> some View {
        VStack(alignment: .leading, spacing: PageSpacing.small) {
            // Small capitals as on Windows ("RECENT PROJECTS"), with Open Archived at the right.
            HStack(alignment: .firstTextBaseline, spacing: PageSpacing.small) {
                Text(title)
                    .textCase(.uppercase)
                    .font(.caption.weight(.bold))
                    .tracking(0.7)
                    .foregroundStyle(theme["muted"])
                    .accessibilityAddTraits(.isHeader)
                Spacer(minLength: PageSpacing.small)
                if let trailing { trailing }
            }
            .padding(.top, PageSpacing.large)
            if items.isEmpty {
                Text(home.activeCount == 0
                     ? "All your projects are archived. Open Archived to restore one, or create a new project."
                     : "Every project here is pinned above.")
                    .font(.callout).foregroundStyle(theme["muted"])
            }
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 240), spacing: PageSpacing.medium, alignment: .top)],
                      alignment: .leading, spacing: PageSpacing.medium) {
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
        ZStack(alignment: .topTrailing) {
            Button(action: handlers.open) {
                VStack(alignment: .leading, spacing: 0) {
                    ProjectPreview(projectId: project.projectId)
                        .frame(height: 78)
                        .frame(maxWidth: .infinity)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 8) {
                        Text(project.title)
                            .font(.headline)
                            .lineLimit(1)
                            .truncationMode(.tail)
                            .help(project.title)
                        BDFlowLayout(spacing: 4) {
                            BDChip(text: project.projectType)
                            BDChip(text: project.status, tone: .blue)
                            if project.pinned { BDChip(text: "Pinned", tone: .yellow, symbol: "pin.fill") }
                            if !project.available { BDChip(text: "Unavailable", tone: .red) }
                        }
                        if project.available {
                            Text("Last modified \(Formatting.relative(ms: project.modifiedAt ?? project.lastOpenedAt))")
                                .font(.caption)
                                .foregroundStyle(.tertiary)
                        } else {
                            Text("Folder not found. Reconnect the drive or locate the project.")
                                .font(.caption)
                                .foregroundStyle(theme["yellow-t"])
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    .padding(12)
                }
                .frame(maxWidth: .infinity, alignment: .topLeading)
                .background(theme[hovering ? "hover" : "panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
                .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            }
            .buttonStyle(PressableCardStyle())
            .onHover { hovering = $0 }
            .contextMenu { ProjectCardMenu(project: project, handlers: handlers) }
            .accessibilityElement(children: .combine)
            .accessibilityLabel("Open \(project.title), \(project.projectType), \(project.status)\(project.pinned ? ", pinned" : "")\(project.archived ? ", archived" : "")\(project.available ? "" : ", unavailable")")
            .accessibilityHint("Opens the project")
            .accessibilityIdentifier("project-card-\(project.title)")
            // The card's actions, named (Windows: the card's More actions menu).
            Menu {
                ProjectCardMenu(project: project, handlers: handlers)
            } label: {
                Label("Actions", systemImage: "ellipsis").labelStyle(.titleAndIcon).font(.caption.weight(.medium))
            }
            .menuStyle(.button)
            .buttonStyle(.borderless)
            .menuIndicator(.hidden)
            .controlSize(.small)
            .fixedSize()
            .padding(.horizontal, 8).padding(.vertical, 4)
            .background(.regularMaterial, in: Capsule())
            .padding(6)
            .help("Rename, pin, duplicate, archive or delete \(project.title)")
            .accessibilityLabel("Actions for \(project.title)")
            .accessibilityIdentifier("project-card-actions-\(project.title)")
        }
    }
}

/// The picture at the top of a project card, the same for a project every time: the Windows
/// app's five painted previews (night with a moon, bus, green, warm, cool), in the same order
/// and chosen by the same hash of the project id. Fixed colours: placeholder imagery, not theme.
struct ProjectPreview: View {
    let projectId: String

    private static func hex(_ v: UInt32) -> Color {
        Color(red: Double((v >> 16) & 0xff) / 255, green: Double((v >> 8) & 0xff) / 255, blue: Double(v & 0xff) / 255)
    }

    private var kind: Int {
        Int(projectId.unicodeScalars.reduce(UInt32(0)) { ($0 &* 31) &+ $1.value } % 5)
    }

    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width, h = geo.size.height
            switch kind {
            case 0: // night
                LinearGradient(colors: [Self.hex(0x0e1424), Self.hex(0x243250)], startPoint: .top, endPoint: .bottom)
                    .overlay(alignment: .topLeading) {
                        Circle().fill(Self.hex(0xf6e7a8))
                            .frame(width: h * 0.34, height: h * 0.34)
                            .offset(x: w * 0.78 - h * 0.17, y: h * 0.22 - h * 0.17)
                    }
            case 1: // bus
                VStack(spacing: 0) {
                    Self.hex(0x586a80).frame(height: h * 0.5)
                    Self.hex(0xc98b2f).frame(height: h * 0.28)
                    Self.hex(0x2a2d34)
                }
            case 2:
                LinearGradient(colors: [Self.hex(0x5a8a62), Self.hex(0x1f3d2a)], startPoint: .topLeading, endPoint: .bottomTrailing)
            case 3:
                LinearGradient(colors: [Self.hex(0xe8a15a), Self.hex(0x8b3f22)], startPoint: .topLeading, endPoint: .bottomTrailing)
            default:
                LinearGradient(colors: [Self.hex(0x4d6d98), Self.hex(0x1c2b46)], startPoint: .topLeading, endPoint: .bottomTrailing)
            }
        }
    }
}

/// The actions of a project, in the card's menu and its context menu.
struct ProjectCardMenu: View {
    let project: RecentProject
    let handlers: ProjectCard.Handlers

    var body: some View {
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
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("data-location-note")
        }
    }
}

/// "Search projects": a real search field in the page (Escape clears it).
struct ProjectSearchField: NSViewRepresentable {
    @Binding var text: String

    func makeNSView(context: Context) -> NSSearchField {
        let f = NSSearchField()
        f.placeholderString = "Search projects"
        f.setAccessibilityLabel("Search projects")
        f.setAccessibilityIdentifier("home-search")
        f.delegate = context.coordinator
        f.sendsSearchStringImmediately = true
        return f
    }

    func updateNSView(_ f: NSSearchField, context: Context) {
        if f.stringValue != text { f.stringValue = text }
    }

    func makeCoordinator() -> Coordinator { Coordinator(text: $text) }

    final class Coordinator: NSObject, NSSearchFieldDelegate {
        var text: Binding<String>
        init(text: Binding<String>) { self.text = text }
        func controlTextDidChange(_ note: Notification) {
            if let f = note.object as? NSSearchField { text.wrappedValue = f.stringValue }
        }
    }
}

extension Notification.Name {
    static let openFrameNewProject = Notification.Name("studio.openframe.newProject")
    static let openFrameOpenProject = Notification.Name("studio.openframe.openProject")
}

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
    var loading = true
    var failure: CoreFailure?

    func load(_ core: Core?) async {
        guard let core else { return }
        do {
            recents = try await core.projectListRecent(ListRecentArgs(includeArchived: showArchived))
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error)
        }
        loading = false
    }

    var pinned: [RecentProject] { recents.filter { $0.pinned && !$0.archived } }
    var others: [RecentProject] { recents.filter { !$0.pinned && !$0.archived } }
    var archived: [RecentProject] { recents.filter(\.archived) }
}

struct HomeView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var home = HomeModel()
    @State private var showNewProject = false
    @State private var showVault = false

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    header
                    if home.loading {
                        ProgressView().frame(maxWidth: .infinity)
                    } else if home.recents.isEmpty {
                        emptyState
                    } else {
                        projectSections
                    }
                    DataLocationNote()
                }
                .padding(32)
                .frame(maxWidth: 880, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
            .background(theme["bg"])
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    Button { showVault = true } label: { Label("Global Idea Vault", systemImage: "lightbulb") }
                        .help("Your own Idea Vault, outside any project")
                        .accessibilityIdentifier("home-global-vault")
                    Button { openExisting() } label: { Label("Open", systemImage: "folder") }
                        .help("Open a project folder")
                    Button { showNewProject = true } label: { Label("New Project", systemImage: "plus") }
                        .help("Create a project")
                        .accessibilityIdentifier("home-new-project")
                }
                ToolbarItem(placement: .automatic) {
                    Toggle("Show archived", isOn: $home.showArchived)
                        .toggleStyle(.checkbox)
                }
            }
            .navigationTitle("Projects")
        }
        .sheet(isPresented: $showNewProject) { NewProjectSheet() }
        .sheet(isPresented: $showVault) {
            NavigationStack {
                GlobalIdeaVaultView()
                    .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Done") { showVault = false } } }
            }
            .frame(minWidth: 900, minHeight: 620)
        }
        .task(id: TaskKey(running: app.isRunning, archived: home.showArchived, changes: app.changes.any)) {
            await home.load(app.core)
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameNewProject)) { _ in showNewProject = true }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameOpenProject)) { _ in openExisting() }
    }

    private struct TaskKey: Hashable { let running: Bool; let archived: Bool; let changes: Int }

    private var header: some View {
        HStack(spacing: 14) {
            BrandMark(size: 40)
            VStack(alignment: .leading, spacing: 2) {
                Text("OpenFrame Studio")
                    .font(.system(size: 24, weight: .semibold))
                    .tracking(-0.3)
                if let name = app.info?.profile.displayName {
                    Text("Signed in on this Mac as \(name)")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Start your first project")
                .font(.title3.weight(.semibold))
            Text("A project holds one film or series: its screenplay, story, breakdown and production plans, in one folder on this Mac.")
                .foregroundStyle(.secondary)
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
        .padding(24)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .accessibilityIdentifier("home-empty")
    }

    @ViewBuilder private var projectSections: some View {
        if !home.pinned.isEmpty { section("Pinned", home.pinned) }
        if !home.others.isEmpty { section(home.pinned.isEmpty ? "Recent" : "Other projects", home.others) }
        if home.showArchived && !home.archived.isEmpty { section("Archived", home.archived) }
    }

    private func section(_ title: String, _ items: [RecentProject]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.headline).foregroundStyle(.secondary)
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 240), spacing: 14)], spacing: 14) {
                ForEach(items, id: \.projectId) { project in
                    ProjectCard(project: project) {
                        Task { await app.openProject(path: project.path) }
                    } onPin: { pinned in
                        Task {
                            _ = try? await app.core?.projectSetPinned(SetPinnedArgs(projectId: project.projectId, pinned: pinned))
                            await home.load(app.core)
                        }
                    } onRemove: {
                        Task {
                            _ = try? await app.core?.projectRemoveRecent(ProjectIdArgs(projectId: project.projectId))
                            await home.load(app.core)
                        }
                    }
                }
            }
        }
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
    let project: RecentProject
    let onOpen: () -> Void
    let onPin: (Bool) -> Void
    let onRemove: () -> Void
    @Environment(\.ofTheme) private var theme
    @State private var hovering = false

    var body: some View {
        Button(action: onOpen) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(project.title)
                        .font(.headline)
                        .lineLimit(2)
                    Spacer(minLength: 4)
                    if project.pinned {
                        Image(systemName: "pin.fill").foregroundStyle(.secondary).accessibilityLabel("Pinned")
                    }
                }
                Text(project.projectType)
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
        .disabled(!project.available)
        .contextMenu {
            Button("Open") { onOpen() }.disabled(!project.available)
            Button(project.pinned ? "Unpin" : "Pin") { onPin(!project.pinned) }
            Divider()
            Button("Remove from Recent") { onRemove() }
        }
        .accessibilityElement(children: .combine)
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
            VStack(alignment: .leading, spacing: 6) {
                Label("Your work is stored on this Mac", systemImage: "internaldrive")
                    .font(.subheadline.weight(.semibold))
                Text("Projects are kept in \(info.projectsDir). Back up the OpenFrame folder in Documents to keep a copy of everything.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                Button("Show in Finder") {
                    Task { try? await app.core?.revealLocation("dataRoot") }
                }
                .buttonStyle(.link)
            }
            .accessibilityIdentifier("data-location-note")
        }
    }
}

extension Notification.Name {
    static let openFrameNewProject = Notification.Name("studio.openframe.newProject")
    static let openFrameOpenProject = Notification.Name("studio.openframe.openProject")
}

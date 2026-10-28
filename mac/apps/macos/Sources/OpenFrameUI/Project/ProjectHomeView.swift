// Project Home (Windows: workspaces/project-home), in Mac styling. A new project shows three ways
// to begin; a project with work in it shows Continue, Quick Access and Recent on the left, and
// Production status, Project Files and open Tasks on the right. Every figure comes from the core
// (project.home, files.list, tasks.list); nothing is invented.

import SwiftUI
import OpenFrameCore

struct ProjectHomeView: View {
    let project: ProjectSummary
    let go: (Workspace) -> Void
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var home: ProjectHome?
    @State private var failure: CoreFailure?
    @State private var files: [ProjectFileDto] = []
    @State private var tasks: [TaskDto] = []

    private static let productionKeys: Set<String> = ["breakdownItems", "locations", "shots", "shootingDays", "callSheets"]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: PageSpacing.large) {
                if let failure, home == nil {
                    VStack(spacing: PageSpacing.small) {
                        Text("Project Home couldn't be displayed.").font(.headline)
                        Text("Your project is safe. \(failure.message)").foregroundStyle(theme["muted"])
                        Button("Try Again") { Task { await load() } }
                    }
                    .frame(maxWidth: .infinity)
                } else if let h = home {
                    header(h)
                    if h.isEmpty { waysToBegin } else { active(h) }
                    if h.project.archived {
                        BDBanner(tone: .info, content: {
                            Text("This project is archived. Nothing is deleted. It is only hidden from your recent projects.")
                        }, actions: {
                            Button("Restore to Active") {
                                Task { await ProjectActions(app: app).setArchived(asRecent(h.project), false) }
                            }
                        })
                    }
                } else {
                    ProgressView().frame(maxWidth: .infinity)
                }
            }
            .padding(.bottom, PageSpacing.page)
            .pageColumn(maxWidth: PageSpacing.wideColumn)
        }
        .background(theme["bg"])
        .task(id: app.changes.any) { await load() }
        .accessibilityIdentifier("project-home")
    }

    private func load() async {
        guard let core = app.core else { return }
        do {
            home = try await core.projectHome()
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error)
        }
        files = (try? await core.filesList().files) ?? []
        tasks = (try? await core.tasksList(TaskListArgs(status: "Open"))) ?? []
    }

    private func asRecent(_ p: ProjectSummary) -> RecentProject {
        RecentProject(projectId: p.id, path: p.path, title: p.title, projectType: p.projectType.rawValue,
                      status: p.status, archived: p.archived, pinned: false, lastOpenedAt: 0, available: true)
    }

    // MARK: Header

    private func header(_ h: ProjectHome) -> some View {
        let p = h.project
        let sub = h.isEmpty
            ? "\(p.projectType.rawValue) \u{00B7} Created \(Formatting.relative(ms: p.createdAt))"
            : [p.projectType.rawValue, p.language, p.genre, "Last opened \(Formatting.relative(ms: h.openedAt))"]
                .compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{00B7} ")
        return PageHeader(p.title, subtitle: sub) {
            Menu {
                ForEach(ProjectStatus.allCases, id: \.self) { s in
                    Button {
                        Task { await ProjectActions(app: app).setStatus(s) }
                    } label: {
                        if s.rawValue == p.status { Label(s.rawValue, systemImage: "checkmark") } else { Text(s.rawValue) }
                    }
                }
            } label: {
                Text("Status: \(p.status)")
            }
            .fixedSize()
            .help("Change the project status")
            .accessibilityIdentifier("project-home-status")
            Button { ProjectSettingsRouter.open() } label: { Label("Project settings", systemImage: "gearshape") }
                .accessibilityIdentifier("project-home-settings")
        }
    }

    // MARK: A new project

    private var waysToBegin: some View {
        VStack(alignment: .leading, spacing: PageSpacing.small) {
            Text("Start with your ideas, build your story, or open a screenplay.").font(.headline)
            Text("Pick whichever fits where you are. You can move between them at any time.").foregroundStyle(theme["muted"])
            CenteredCards(cardWidth: 240, spacing: PageSpacing.medium) {
                begin("Open Idea Vault", "Collect thoughts, images, links and notes about the film. Nothing needs to be organised yet.", "lightbulb", .ideaVault, primary: true)
                begin("Build Story", "Arrange acts, beats and scene cards on the Beat Board until the story works.", "rectangle.split.3x1", .story)
                begin("Write/Import Screenplay", "Start writing, or import a screenplay you already have.", "doc.text", .screenplay)
            }
            .padding(.top, PageSpacing.small)
        }
        .padding(PageSpacing.medium)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("project-home-start")
    }

    private func begin(_ title: String, _ text: String, _ symbol: String, _ ws: Workspace, primary: Bool = false) -> some View {
        Button { open(ws) } label: {
            VStack(alignment: .leading, spacing: 8) {
                Image(systemName: symbol).font(.title3).foregroundStyle(theme["accent"]).accessibilityHidden(true)
                Text(title).font(.headline)
                Text(text).font(.callout).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
            }
            .padding(PageSpacing.medium)
            .frame(maxWidth: .infinity, minHeight: 120, alignment: .topLeading)
            .background(theme["bg"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(primary ? theme["accent"] : theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
    }

    // MARK: A project with work in it

    private func active(_ h: ProjectHome) -> some View {
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .top, spacing: PageSpacing.medium) {
                leftColumn(h).frame(minWidth: 420, maxWidth: .infinity)
                rightColumn(h).frame(minWidth: 300, maxWidth: 420)
            }
            VStack(alignment: .leading, spacing: PageSpacing.medium) {
                leftColumn(h)
                rightColumn(h)
            }
        }
    }

    private func leftColumn(_ h: ProjectHome) -> some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            let continues = Array(h.continueLocations.prefix(4))
            if !continues.isEmpty {
                SectionHeader("Continue")
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 200), spacing: PageSpacing.small)], spacing: PageSpacing.small) {
                    ForEach(Array(continues.enumerated()), id: \.offset) { _, loc in continueCard(loc) }
                }
            }
            SectionHeader("Quick Access")
            BDFlowLayout(spacing: PageSpacing.small) {
                quick("Idea Vault", "lightbulb", .ideaVault)
                quick("Beat Board", "rectangle.split.3x1", .story)
                quick("Screenplay", "doc.text", .screenplay)
                quick("Breakdown", "checklist", .breakdown)
                quick("Production", "film", .production)
                quick("Call Sheets", "list.clipboard", .callSheets)
            }
            BDFlowLayout(spacing: PageSpacing.small) {
                small("Notes & Tasks", "checklist.checked") { go(.notes) }
                small("Activity", "clock.arrow.circlepath") { go(.activity) }
                small("Recently Deleted", "trash") { go(.recentlyDeleted) }
                small("Templates", "doc.on.doc") { ProjectSettingsRouter.open(tab: .templates) }
            }
            SectionHeader("Recent")
            let recent = h.recentActivity.filter { !$0.action.hasPrefix("project.") && !$0.action.hasPrefix("trash.") }.prefix(6)
            VStack(alignment: .leading, spacing: 0) {
                if recent.isEmpty {
                    Text("Recent changes to scenes, cards, drafts and documents will appear here.")
                        .font(.callout).foregroundStyle(theme["muted"]).padding(PageSpacing.medium)
                }
                ForEach(Array(recent), id: \.id) { entry in
                    recentRow(entry)
                    if entry.id != recent.last?.id { Divider() }
                }
            }
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        }
    }

    private func rightColumn(_ h: ProjectHome) -> some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            let production = h.counts.filter { Self.productionKeys.contains($0.key) }
            if !production.isEmpty {
                card {
                    Text("Production status").font(.headline)
                    ForEach(h.counts.filter { $0.key == "scenes" || $0.key == "drafts" } + production, id: \.key) { c in
                        HStack { Text(c.label); Spacer(); Text("\(Int(c.count))").bold().monospacedDigit() }
                            .font(.callout)
                    }
                    Button { go(.production) } label: { Label("Open Production", systemImage: "film") }
                        .controlSize(.small)
                }
            }
            card {
                HStack {
                    Text("Project Files").font(.headline)
                    Spacer()
                    Button {
                        ProjectIntents.shared.fire("files.add")
                        go(.files)
                    } label: { Label("Add File", systemImage: "plus") }
                    .controlSize(.small)
                }
                if files.isEmpty {
                    Text("Keep project documents and attachments here.").font(.callout).foregroundStyle(theme["muted"])
                } else {
                    ForEach(files.prefix(5), id: \.id) { f in
                        HStack {
                            Image(systemName: "doc").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                            Text(f.displayName).lineLimit(1).truncationMode(.middle).help(f.displayName)
                            Spacer()
                            if !f.asset.available { BDChip(text: "Unavailable", tone: .red) }
                        }
                        .font(.callout)
                    }
                    Button(files.count > 5 ? "All \(files.count) files" : "Open Files") { go(.files) }
                        .buttonStyle(.borderless)
                }
            }
            if !tasks.isEmpty {
                card {
                    HStack {
                        Text("Tasks").font(.headline)
                        Spacer()
                        BDChip(text: "\(tasks.count) open")
                    }
                    ForEach(tasks.prefix(4), id: \.id) { t in
                        HStack {
                            Text(t.title).lineLimit(2)
                            Spacer()
                            if let due = t.dueAt {
                                Text("due \(Date(timeIntervalSince1970: due / 1000).formatted(.dateTime.day().month(.abbreviated)))")
                                    .font(.caption).foregroundStyle(theme["muted"])
                            }
                        }
                        .font(.callout)
                    }
                    Button("Open Notes & Tasks") { go(.notes) }.buttonStyle(.borderless)
                }
            }
        }
    }

    // MARK: Parts

    private func card<C: View>(@ViewBuilder _ content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: PageSpacing.small) { content() }
            .padding(PageSpacing.medium)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
    }

    private func quick(_ title: String, _ symbol: String, _ ws: Workspace) -> some View {
        Button { open(ws) } label: { Label(title, systemImage: symbol) }
            .accessibilityIdentifier("project-home-quick-\(ws.rawValue)")
    }

    /// "Build Story" and "Beat Board" mean the board, not the Story page Story opens on
    /// (Windows: go({ workspace: "story", sub: "board" })). The request waits until Story takes it.
    private func open(_ ws: Workspace) {
        if ws == .story { ProjectIntents.shared.fire(StoryTab.board.showIntent) }
        go(ws)
    }

    private func small(_ title: String, _ symbol: String, _ action: @escaping () -> Void) -> some View {
        Button(action: action) { Label(title, systemImage: symbol) }
            .buttonStyle(.borderless)
            .font(.callout)
    }

    private static func continueTitle(_ loc: [String: JSONValue]) -> (String, String) {
        let ws = loc["workspace"]?.stringValue ?? ""
        if ws == "production", loc["sub"]?.stringValue == "schedule" { return ("Continue schedule", "film") }
        switch ws {
        case "screenplay": return ("Continue writing", "doc.text")
        case "story": return ("Continue the Beat Board", "rectangle.split.3x1")
        case "vault": return ("Continue in the Idea Vault", "lightbulb")
        case "breakdown": return ("Continue the breakdown", "checklist")
        case "production": return ("Continue production", "film")
        case "callsheets": return ("Continue call sheets", "list.clipboard")
        default: return ("Continue", "arrow.right")
        }
    }

    private func continueCard(_ loc: [String: JSONValue]) -> some View {
        let (title, symbol) = Self.continueTitle(loc)
        let label = loc["label"]?.stringValue ?? ""
        return Button { ProjectNav.open(windowsLocation: loc) } label: {
            HStack(spacing: PageSpacing.small) {
                Image(systemName: symbol).foregroundStyle(theme["accent"]).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).fontWeight(.semibold)
                    Text(label).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1).help(label)
                }
                Spacer()
                Image(systemName: "arrow.right").foregroundStyle(theme["muted"]).accessibilityHidden(true)
            }
            .padding(PageSpacing.small + 4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
        .accessibilityLabel("\(title): \(label)")
    }

    private func recentRow(_ entry: ActivityEntry) -> some View {
        let meta = (entry.actorName.map { "\($0) \u{00B7} " } ?? "") + Formatting.relative(ms: entry.at)
        let body = HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(entry.summary).fontWeight(.semibold).lineLimit(1).help(entry.summary)
                Text(meta).font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer()
        }
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, 8)
        return Group {
            if case .object(let nav) = entry.nav {
                Button { ProjectNav.open(windowsLocation: nav) } label: {
                    HStack { body; Image(systemName: "arrow.right").foregroundStyle(theme["muted"]).padding(.trailing, PageSpacing.medium) }
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            } else {
                body
            }
        }
    }
}

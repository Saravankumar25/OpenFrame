// Episodes and seasons for an episodic project. Each episode has its own Story Board; opening an
// episode scopes the board. Mirrors workspaces/story/Episodes.tsx.

import SwiftUI
import UniformTypeIdentifiers
import OpenFrameCore

enum EpisodeSheet: Identifiable {
    case new(seasonId: String?)
    case edit(StoryEpisodeDto)
    case duplicate(StoryEpisodeDto)
    case delete(StoryEpisodeDto)
    case renameSeason(String)

    var id: String {
        switch self {
        case let .new(s): "new-\(s ?? "")"
        case let .edit(e): "edit-\(e.id)"
        case let .duplicate(e): "duplicate-\(e.id)"
        case let .delete(e): "delete-\(e.id)"
        case let .renameSeason(id): "rename-season-\(id)"
        }
    }
}

@MainActor
@Observable
final class EpisodesModel {
    static let noSeason = "__none__"

    let story: StoryModel
    var seasonTab: String?
    var showBoard = false
    var sheet: EpisodeSheet?
    private(set) var recurringCharacters = 0

    init(story: StoryModel) { self.story = story }

    var series: StorySeriesDto? { story.series }

    var tabs: [(value: String, label: String)] {
        guard let s = series else { return [] }
        return s.seasons.map { ($0.id, $0.title) } + (s.unseasoned.isEmpty ? [] : [(Self.noSeason, "No season")])
    }

    var active: String {
        if let t = seasonTab, tabs.contains(where: { $0.value == t }) { return t }
        return tabs.first?.value ?? Self.noSeason
    }

    var season: StorySeasonDto? { series?.seasons.first { $0.id == active } }
    var episodes: [StoryEpisodeDto] { season?.episodes ?? series?.unseasoned ?? [] }
    var seasonId: String? { season?.id }
    var total: Int { (series?.seasons.reduce(0) { $0 + $1.episodes.count } ?? 0) + (series?.unseasoned.count ?? 0) }
    var projectTitle: String { if case let .project(p) = story.app.phase { p.title } else { "" } }

    func loadReferences() async {
        guard let core = story.app.core else { return }
        if let list = try? await core.storyCharacters(StoryCharacterListArgs(includeArchived: false)) {
            recurringCharacters = list.filter { $0.episodeId == nil }.count
        }
    }

    func addSeason() async {
        guard let core = story.app.core else { return }
        do {
            let r = try await core.storyCreateSeason(StoryCreateSeasonArgs())
            seasonTab = r.id
            story.say("Added a season", undoable: true)
            await story.reload()
        } catch { story.fail("Could not add the season", error) }
    }

    func renameSeason(_ id: String, _ title: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storyUpdateSeason(StoryUpdateSeasonArgs(id: id, title: title)) }
        catch { story.fail("Could not rename the season", error) }
    }

    func moveSeasonLeft(_ s: StorySeasonDto) async {
        guard let core = story.app.core, let seasons = series?.seasons, let i = seasons.firstIndex(where: { $0.id == s.id }), i > 0 else { return }
        do { _ = try await core.storyMoveSeason(StoryMoveSeasonArgs(id: s.id, beforeId: seasons[i - 1].id)) }
        catch { story.fail("Could not move the season", error) }
    }

    func deleteSeason(_ s: StorySeasonDto) async {
        guard let core = story.app.core else { return }
        do {
            _ = try await core.storyDeleteSeason(StoryIdArgs(id: s.id))
            story.say("Deleted \(s.title). Its episodes were kept.", undoable: true)
        } catch { story.fail("Could not delete the season", error) }
    }

    @discardableResult
    func create(seasonId: String?, title: String, summary: String, status: String) async -> Bool {
        guard let core = story.app.core else { return false }
        let t = title.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            _ = try await core.storyCreateEpisode(StoryCreateEpisodeArgs(seasonId: seasonId, title: t, summary: summary.isEmpty ? nil : summary, status: status))
            story.say("Added episode \u{201C}\(t)\u{201D}", undoable: true)
            return true
        } catch { story.fail("Could not add the episode", error); return false }
    }

    @discardableResult
    func update(_ e: StoryEpisodeDto, title: String, summary: String, status: String, seasonId: String?) async -> Bool {
        guard let core = story.app.core else { return false }
        do {
            _ = try await core.storyUpdateEpisode(StoryUpdateEpisodeArgs(id: e.id, title: title, summary: summary, status: status))
            if seasonId != e.seasonId { _ = try await core.storyMoveEpisode(StoryMoveEpisodeArgs(id: e.id, seasonId: seasonId, beforeId: nil)) }
            return true
        } catch { story.fail("Could not save the episode", error); return false }
    }

    func duplicate(_ e: StoryEpisodeDto, copyStory: Bool) async {
        guard let core = story.app.core else { return }
        do {
            _ = try await core.storyDuplicateEpisode(StoryDuplicateEpisodeArgs(id: e.id, copyStory: copyStory))
            story.say("Duplicated \u{201C}\(e.title)\u{201D}", undoable: true)
        } catch { story.fail("Could not duplicate the episode", error) }
    }

    func delete(_ e: StoryEpisodeDto) async {
        guard let core = story.app.core else { return }
        do {
            _ = try await core.storyDeleteEpisode(StoryIdArgs(id: e.id))
            story.say("Deleted episode \u{201C}\(e.title)\u{201D}", undoable: true)
        } catch { story.fail("Could not delete the episode", error) }
    }

    /// Reorder: put `id` where `onto` is. The core takes "before which episode" (nil = last).
    func reorder(_ id: String, onto target: String) async {
        guard let core = story.app.core else { return }
        var ids = episodes.map(\.id)
        guard let from = ids.firstIndex(of: id), let to = ids.firstIndex(of: target), from != to else { return }
        ids.move(fromOffsets: IndexSet(integer: from), toOffset: to > from ? to + 1 : to)
        let newIndex = ids.firstIndex(of: id) ?? to
        let before = newIndex + 1 < ids.count ? ids[newIndex + 1] : nil
        do {
            _ = try await core.storyMoveEpisode(StoryMoveEpisodeArgs(id: id, seasonId: seasonId, beforeId: before))
            story.say("Reordered episodes", undoable: true)
        } catch { story.fail("Could not reorder the episodes", error) }
    }

    func step(_ e: StoryEpisodeDto, direction: Int) async {
        let ids = episodes.map(\.id)
        guard let i = ids.firstIndex(of: e.id) else { return }
        let j = i + direction
        guard ids.indices.contains(j) else { return }
        await reorder(e.id, onto: ids[j])
    }

    static func statusTone(_ s: String?) -> String {
        switch s {
        case "Locked": "ink"
        case "Writing", "Rewrite": "blue"
        case "Shooting", "Complete": "green"
        case "Development": "yellow"
        default: "muted"
        }
    }
}

// MARK: View

struct StoryEpisodesView: View {
    @Bindable var story: StoryModel
    @State private var m: EpisodesModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    init(story: StoryModel) {
        self.story = story
        _m = State(initialValue: EpisodesModel(story: story))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(spacing: PageSpacing.small) {
                VStack(spacing: PageSpacing.tight) {
                    Text("Series: \(m.projectTitle)").font(.title2.weight(.semibold)).accessibilityAddTraits(.isHeader)
                    Text("Which episode am I working on?").foregroundStyle(theme["muted"])
                }
                HStack(spacing: PageSpacing.small) {
                Button { Task { await m.addSeason() } } label: { Label("Add Season", systemImage: "plus") }
                Button { m.sheet = .new(seasonId: m.seasonId) } label: { Label("Add Episode", systemImage: "plus") }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("story-add-episode")
                }
            }
            .frame(maxWidth: .infinity)
            if let s = m.series {
                if !s.isEpisodic {
                    Text("This project is a \(s.projectType). Episodes are optional; the Story Board works without them.")
                        .font(.callout).foregroundStyle(theme["muted"])
                }
                if m.total == 0 && s.seasons.isEmpty {
                    ContentUnavailableView {
                        Label("Create your first episode.", systemImage: "tv")
                    } description: {
                        Text("Each episode has its own Story Board, screenplay drafts and production planning.")
                    } actions: {
                        Button("Add Episode") { m.sheet = .new(seasonId: nil) }.buttonStyle(.borderedProminent)
                    }
                } else {
                    HStack(alignment: .top, spacing: 16) {
                        main.frame(maxWidth: .infinity, alignment: .topLeading)
                        references
                    }
                }
            } else {
                ProgressView()
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .task(id: app.changes.key(StoryTables.series + ["story_character"])) { await m.loadReferences() }
        .sheet(item: $m.sheet) { sheet in
            switch sheet {
            case let .new(seasonId): EpisodeFormSheet(m: m, editing: nil, seasonId: seasonId)
            case let .edit(e): EpisodeFormSheet(m: m, editing: e, seasonId: e.seasonId)
            case let .duplicate(e): DuplicateEpisodeSheet(m: m, e: e)
            case let .delete(e): DeleteEpisodeSheet(m: m, e: e)
            case let .renameSeason(id): RenameSeasonSheet(m: m, id: id)
            }
        }
        .accessibilityIdentifier("story-episodes")
    }

    private var main: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                if !m.tabs.isEmpty {
                    Picker("Season", selection: Binding(get: { m.active }, set: { m.seasonTab = $0 })) {
                        ForEach(m.tabs, id: \.value) { Text($0.label).tag($0.value) }
                    }
                    .pickerStyle(.segmented).labelsHidden().fixedSize()
                    .accessibilityLabel("Season")
                }
                if let season = m.season {
                    Menu {
                        Button("Rename season\u{2026}") { m.sheet = .renameSeason(season.id) }
                        Button("Move left") { Task { await m.moveSeasonLeft(season) } }.disabled(m.series?.seasons.first?.id == season.id)
                        Divider()
                        Button("Delete season", role: .destructive) { Task { await m.deleteSeason(season) } }
                    } label: { Image(systemName: "ellipsis.circle") }
                        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                        .accessibilityLabel("Options for \(season.title)")
                }
                Spacer()
                Picker("View", selection: $m.showBoard) {
                    Text("List").tag(false)
                    Text("Season Board").tag(true)
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize()
                .accessibilityLabel("View")
            }
            if m.showBoard { seasonBoard } else { episodeList }
        }
    }

    private var episodeList: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                if m.episodes.isEmpty {
                    Text("No episodes in this season yet.").foregroundStyle(theme["muted"]).padding(16).frame(maxWidth: .infinity, alignment: .leading)
                }
                ForEach(m.episodes, id: \.id) { e in
                    HStack(spacing: 10) {
                        Button { Task { await story.openEpisode(e.id) } } label: {
                            VStack(alignment: .leading, spacing: 2) {
                                Text("E\(String(format: "%02d", Int(e.number))) \(e.title)").fontWeight(.semibold)
                                if let s = e.summary, !s.isEmpty { Text(s).font(.callout).foregroundStyle(theme["muted"]) }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        Text(StoryLogic.plural(Int(e.cardCount), "card")).font(.caption).foregroundStyle(theme["muted"])
                        if let s = e.status { StatusChip(status: s) }
                        EpisodeMenu(m: m, story: story, e: e)
                    }
                    .padding(.horizontal, 12).padding(.vertical, 10)
                    .accessibilityElement(children: .contain)
                    .accessibilityLabel("Episode \(Int(e.number)): \(e.title)")
                    Divider()
                }
            }
        }
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
    }

    private var seasonBoard: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("\(m.season?.title ?? "No season"): Board").fontWeight(.semibold)
            Text("Plan the season at a glance. Drag episode cards to reorder, or use Move Earlier and Move Later in the menu.")
                .font(.callout).foregroundStyle(theme["muted"])
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 190), spacing: 12)], spacing: 12) {
                    ForEach(m.episodes, id: \.id) { e in
                        SeasonBoardCard(m: m, story: story, e: e)
                    }
                }
            }
        }
    }

    private var references: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Series references").font(.headline)
            HStack { Text("Recurring characters"); Spacer(); Text("\(m.recurringCharacters)").fontWeight(.semibold) }
            Text("Episodes can reference these lightweight lists. OpenFrame does not require you to manage cross-episode continuity.")
                .font(.caption).foregroundStyle(theme["muted"])
        }
        .padding(14)
        .frame(width: 260, alignment: .topLeading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Series references")
    }
}

private struct StatusChip: View {
    let status: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let tone = EpisodesModel.statusTone(status)
        let background: Color = tone == "muted" ? theme["surface2"] : (tone == "ink" ? theme["ink"] : theme["\(tone)-soft"])
        let foreground: Color = tone == "ink" ? theme["panel"] : (tone == "muted" ? theme["ink2"] : theme["\(tone)-t"])
        Text(status).font(.caption2.weight(.semibold))
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(background, in: Capsule())
            .foregroundStyle(foreground)
    }
}

private struct EpisodeMenu: View {
    @Bindable var m: EpisodesModel
    @Bindable var story: StoryModel
    let e: StoryEpisodeDto

    var body: some View {
        Menu {
            Button("Open Story Board") { Task { await story.openEpisode(e.id) } }
            Button("Edit\u{2026}") { m.sheet = .edit(e) }
            Button("Duplicate\u{2026}") { m.sheet = .duplicate(e) }
            Button("Move Earlier") { Task { await m.step(e, direction: -1) } }.disabled(m.episodes.first?.id == e.id)
            Button("Move Later") { Task { await m.step(e, direction: 1) } }.disabled(m.episodes.last?.id == e.id)
            Divider()
            Button("Delete", role: .destructive) { m.sheet = .delete(e) }
        } label: { Image(systemName: "ellipsis") }
            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
            .accessibilityLabel("Options for \(e.title)")
            .accessibilityIdentifier("story-episode-options-\(e.id)")
    }
}

// MARK: Sheets

private struct EpisodeFormSheet: View {
    @Bindable var m: EpisodesModel
    let editing: StoryEpisodeDto?
    let seasonId: String?
    @State private var title = ""
    @State private var summary = ""
    @State private var status = "Idea"
    @State private var season = EpisodesModel.noSeason
    @FocusState private var focused: Bool

    var body: some View {
        StorySheetFrame(title: editing.map { "Edit \u{201C}\($0.title)\u{201D}" } ?? "New Episode") {
            VStack(alignment: .leading, spacing: 10) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Title").font(.callout.weight(.medium))
                    TextField("Pilot", text: $title).textFieldStyle(.roundedBorder).focused($focused)
                        .accessibilityLabel("Episode title").accessibilityIdentifier("story-episode-title")
                }
                VStack(alignment: .leading, spacing: 3) {
                    Text("One-line summary").font(.callout.weight(.medium))
                    TextEditor(text: $summary).frame(height: 54).padding(3)
                        .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(.quaternary))
                        .accessibilityLabel("One-line summary")
                }
                HStack {
                    Picker("Status", selection: $status) { ForEach(m.series?.statuses ?? [], id: \.self) { Text($0).tag($0) } }
                    Picker("Season", selection: $season) {
                        ForEach(m.series?.seasons ?? [], id: \.id) { Text($0.title).tag($0.id) }
                        Text("No season").tag(EpisodesModel.noSeason)
                    }
                }
                Text("The episode number comes from its order in the season.").font(.caption).foregroundStyle(.secondary)
            }
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            Button(editing == nil ? "Create Episode" : "Save") {
                let sid = season == EpisodesModel.noSeason ? nil : season
                Task {
                    let ok: Bool
                    if let e = editing { ok = await m.update(e, title: title, summary: summary, status: status, seasonId: sid) }
                    else { ok = await m.create(seasonId: sid, title: title, summary: summary, status: status) }
                    if ok { m.sheet = nil }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(title.trimmingCharacters(in: .whitespaces).isEmpty)
            .accessibilityIdentifier("story-save-episode")
        }
        .onAppear {
            title = editing?.title ?? ""
            summary = editing?.summary ?? ""
            status = editing?.status ?? "Idea"
            season = seasonId ?? EpisodesModel.noSeason
            focused = true
        }
    }
}

private struct DuplicateEpisodeSheet: View {
    @Bindable var m: EpisodesModel
    let e: StoryEpisodeDto
    @State private var copyStory = true

    var body: some View {
        StorySheetFrame(title: "Duplicate \u{201C}\(e.title)\u{201D}", width: 420) {
            VStack(alignment: .leading, spacing: 6) {
                Toggle("Also copy its Story Board (new, independent cards)", isOn: $copyStory).toggleStyle(.checkbox)
                Text("The copy is a new episode. Screenplay links are not copied.").font(.caption).foregroundStyle(.secondary)
            }
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Duplicate") {
                m.sheet = nil
                Task { await m.duplicate(e, copyStory: copyStory) }
            }
            .keyboardShortcut(.defaultAction)
        }
    }
}

private struct DeleteEpisodeSheet: View {
    @Bindable var m: EpisodesModel
    let e: StoryEpisodeDto

    var body: some View {
        StorySheetFrame(title: "Delete \u{201C}\(e.title)\u{201D}?", width: 420) {
            Text("The episode and its Story Board move to Recently Deleted together. You can restore them or use Undo.")
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Delete Episode", role: .destructive) {
                m.sheet = nil
                Task { await m.delete(e) }
            }
            .keyboardShortcut(.defaultAction)
        }
    }
}

private struct RenameSeasonSheet: View {
    @Bindable var m: EpisodesModel
    let id: String
    @State private var title = ""
    @FocusState private var focused: Bool

    var body: some View {
        StorySheetFrame(title: "Rename season", width: 380) {
            VStack(alignment: .leading, spacing: 3) {
                Text("Season title").font(.callout.weight(.medium))
                TextField("Season title", text: $title).textFieldStyle(.roundedBorder).focused($focused)
                    .accessibilityLabel("Season title")
            }
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Save") {
                let t = title
                m.sheet = nil
                Task { await m.renameSeason(id, t) }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(title.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .onAppear {
            title = m.series?.seasons.first { $0.id == id }?.title ?? ""
            focused = true
        }
    }
}

private struct SeasonBoardCard: View {
    @Bindable var m: EpisodesModel
    @Bindable var story: StoryModel
    let e: StoryEpisodeDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 10, style: .continuous)
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Episode \(Int(e.number))").font(.caption2.weight(.bold)).textCase(.uppercase).foregroundStyle(theme["muted"])
                Spacer()
                EpisodeMenu(m: m, story: story, e: e)
            }
            Text(e.title).fontWeight(.semibold)
            if let s = e.summary, !s.isEmpty { Text(s).font(.callout).foregroundStyle(theme["muted"]).lineLimit(2) }
            if let s = e.status { StatusChip(status: s) }
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, minHeight: 110, alignment: .topLeading)
        .background(theme["panel"], in: shape)
        .overlay(shape.strokeBorder(theme["line"]))
        .contentShape(shape)
        .onTapGesture(count: 2) { Task { await story.openEpisode(e.id) } }
        .draggable(e.id)
        .dropDestination(for: String.self) { items, _ in
            guard let id = items.first, m.episodes.contains(where: { $0.id == id }) else { return false }
            Task { await m.reorder(id, onto: e.id) }
            return true
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Episode \(Int(e.number)): \(e.title)")
        .accessibilityAction(named: "Open Story Board") { Task { await story.openEpisode(e.id) } }
        .accessibilityAction(named: "Move Earlier") { Task { await m.step(e, direction: -1) } }
        .accessibilityAction(named: "Move Later") { Task { await m.step(e, direction: 1) } }
    }
}

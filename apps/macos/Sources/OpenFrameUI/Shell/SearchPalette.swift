// Command Palette and global search (Command+K). Windows: app/shell/SearchOverlay.tsx. The text goes
// to search.query (the core decides what a person may see; private notes stay with their owner) and
// a result jumps to its workspace. Commands (New Project, Close Project, workspaces, Settings,
// About) are listed above the results.

import AppKit
import Observation
import SwiftUI
import OpenFrameCore

/// Where a search result leads on the Mac.
struct SearchTarget: Equatable {
    var workspace: Workspace?
    /// Set for screenplay results: screenplayId, draftId, sceneId, episodeId where the core knows them.
    var screenplay: [String: String] = [:]
    /// The result lives where the Mac has no screen yet (Settings, or a Windows-only area).
    var opensSettings = false
}

enum SearchRouting {
    /// The nav target the core sends with a hit ({"workspace": ..., "sub": ..., "<x>Id": ...}) as a Mac destination.
    static func target(for hit: SearchHit) -> SearchTarget {
        let ws = hit.nav["workspace"]?.stringValue ?? ""
        let sub = hit.nav["sub"]?.stringValue
        switch ws {
        case "home": return SearchTarget(workspace: .home)
        case "vault": return SearchTarget(workspace: .ideaVault)
        case "story": return SearchTarget(workspace: .story)
        case "breakdown": return SearchTarget(workspace: .breakdown)
        case "screenplay":
            var info: [String: String] = [:]
            for key in ["screenplayId", "draftId", "sceneId", "episodeId"] {
                if let v = hit.nav[key]?.stringValue { info[key] = v }
            }
            return SearchTarget(workspace: .screenplay, screenplay: info)
        case "production":
            switch sub {
            case "schedule": return SearchTarget(workspace: .schedule)
            case "sides": return SearchTarget(workspace: .callSheets)
            case "budget": return SearchTarget(workspace: .budget)
            case "shots", "moodboards", "storyboards": return SearchTarget(workspace: .visual)
            default: return SearchTarget(workspace: .breakdown)
            }
        case "callsheets": return SearchTarget(workspace: .callSheets)
        case "files": return SearchTarget(workspace: .files)
        case "notes": return SearchTarget(workspace: .notes)
        case "activity": return SearchTarget(workspace: .activity)
        case "trash": return SearchTarget(workspace: .recentlyDeleted)
        case "settings": return SearchTarget(workspace: nil, opensSettings: true)
        default: return SearchTarget(workspace: nil)
        }
    }

    @MainActor
    static func go(_ target: SearchTarget) {
        if target.opensSettings {
            NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
            return
        }
        guard let ws = target.workspace else { return }
        if ws == .screenplay {
            NotificationCenter.default.post(name: .openFrameShowScreenplayTarget, object: nil, userInfo: target.screenplay)
        }
        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: ws.rawValue)
    }

    static let typeLabels: [String: String] = [
        "vault_item": "Idea Vault", "story_act": "Act", "story_sequence": "Sequence", "story_beat": "Beat",
        "story_scene_card": "Scene Card", "story_character": "Character", "screenplay_scene": "Scene",
        "screenplay_element": "Screenplay", "catalog_item": "Catalog", "location": "Location",
        "cast_member": "Cast", "crew_member": "Crew", "shot": "Shot", "storyboard": "Storyboard",
        "call_sheet": "Call Sheet", "project_file": "File", "comment": "Comment", "project_note": "Note",
        "private_note": "Private Note",
    ]

    static func label(for entityType: String) -> String {
        typeLabels[entityType] ?? entityType.replacingOccurrences(of: "_", with: " ").capitalized
    }

    /// A snippet with the core's match markers (U+0001 ... U+0002) shown in bold.
    static func highlighted(_ snippet: String) -> AttributedString {
        var out = AttributedString()
        var bold = false
        var buffer = ""
        func flush() {
            guard !buffer.isEmpty else { return }
            var piece = AttributedString(buffer)
            if bold { piece.font = .callout.weight(.bold) }
            out += piece
            buffer = ""
        }
        for ch in snippet {
            if ch == "\u{1}" { flush(); bold = true }
            else if ch == "\u{2}" { flush(); bold = false }
            else { buffer.append(ch) }
        }
        flush()
        return out
    }
}

struct PaletteCommand: Identifiable {
    let id: String
    let title: String
    let symbol: String
    var hint: String?
    let run: @MainActor () async -> Void
}

enum PaletteRow: Identifiable {
    case command(PaletteCommand)
    case hit(SearchHit)

    var id: String {
        switch self {
        case let .command(c): "command:\(c.id)"
        case let .hit(h): "hit:\(h.store.rawValue):\(h.entityType):\(h.entityId)"
        }
    }
}

@MainActor
@Observable
final class SearchPaletteModel {
    var query = ""
    private(set) var hits: [SearchHit] = []
    private(set) var searching = false
    private(set) var failure: String?
    var active = 0

    let app: AppModel
    init(app: AppModel) { self.app = app }

    private var inProject: Bool { if case .project = app.phase { true } else { false } }

    var trimmed: String { query.trimmingCharacters(in: .whitespacesAndNewlines) }

    var commands: [PaletteCommand] {
        var list: [PaletteCommand] = []
        if inProject {
            list.append(PaletteCommand(id: "new-project", title: "New Project", symbol: "plus") { [app] in
                await app.closeProject()
                try? await Task.sleep(for: .milliseconds(300))
                NotificationCenter.default.post(name: .openFrameNewProject, object: nil)
            })
            list.append(PaletteCommand(id: "close-project", title: "Close Project", symbol: "xmark.circle", hint: "Shift\u{2318}W") { [app] in
                await app.closeProject()
            })
            for ws in Workspace.allCases where ws.macPhase == nil {
                list.append(PaletteCommand(id: "go-\(ws.rawValue)", title: "Go to \(ws.title)", symbol: ws.symbol,
                                           hint: ws.shortcut.map { "\u{2303}\u{2318}\(String($0.character))" }) {
                    NotificationCenter.default.post(name: .openFrameShowWorkspace, object: ws.rawValue)
                })
            }
        } else {
            list.append(PaletteCommand(id: "new-project", title: "New Project", symbol: "plus", hint: "\u{2318}N") {
                NotificationCenter.default.post(name: .openFrameNewProject, object: nil)
            })
            list.append(PaletteCommand(id: "open-project", title: "Open Project\u{2026}", symbol: "folder", hint: "\u{2318}O") {
                NotificationCenter.default.post(name: .openFrameOpenProject, object: nil)
            })
        }
        list.append(PaletteCommand(id: "settings", title: "Settings", symbol: "gearshape", hint: "\u{2318},") {
            NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
        })
        list.append(PaletteCommand(id: "about", title: "About OpenFrame Studio", symbol: "info.circle") {
            NotificationCenter.default.post(name: .openFrameShowAbout, object: nil)
        })
        return list
    }

    /// Commands whose words contain the text, then the results of the search.
    var rows: [PaletteRow] {
        let q = trimmed.lowercased()
        let matching = q.isEmpty ? commands : commands.filter { $0.title.lowercased().contains(q) }
        return matching.map(PaletteRow.command) + hits.map(PaletteRow.hit)
    }

    /// Runs the search for the current text. A newer text cancels the older one (the view does this
    /// with `.task(id:)`); a result that arrives for old text is dropped.
    func search() async {
        let text = trimmed
        guard inProject, !text.isEmpty, let core = app.core else {
            hits = []
            searching = false
            failure = nil
            return
        }
        searching = true
        do {
            let found = try await core.searchQuery(SearchArgs(text: text, includeGlobal: false, limit: 60, globalOnly: false))
            guard text == trimmed else { return }
            hits = found
            failure = nil
        } catch {
            guard text == trimmed else { return }
            hits = []
            failure = CoreFailure(asCore: error).message
        }
        searching = false
        active = 0
    }

    func move(_ delta: Int) {
        let count = rows.count
        guard count > 0 else { return }
        active = min(max(active + delta, 0), count - 1)
    }

    /// Run the chosen row. Returns true when the palette should close.
    func choose(_ row: PaletteRow) async {
        switch row {
        case let .command(c): await c.run()
        case let .hit(h): SearchRouting.go(SearchRouting.target(for: h))
        }
    }
}

// MARK: View

struct SearchPaletteView: View {
    let model: SearchPaletteModel
    let close: () -> Void
    @Environment(\.ofTheme) private var theme
    @FocusState private var focused: Bool

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
                TextField("Search scenes, ideas, characters, locations, files, or type a command", text: $model.query)
                    .textFieldStyle(.plain)
                    .font(.system(size: 16))
                    .focused($focused)
                    .onSubmit { chooseActive() }
                    .accessibilityIdentifier("palette-field")
                if model.searching { ProgressView().controlSize(.small) }
                Text("esc").font(.caption).foregroundStyle(.tertiary)
            }
            .padding(14)
            Divider()
            results
        }
        .frame(width: 680)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).strokeBorder(theme["line"]))
        .shadow(color: .black.opacity(0.25), radius: 24, y: 8)
        .onAppear { focused = true }
        .onKeyPress(.downArrow) { model.move(1); return .handled }
        .onKeyPress(.upArrow) { model.move(-1); return .handled }
        .onExitCommand { close() }
        .task(id: model.trimmed) {
            if !model.trimmed.isEmpty { try? await Task.sleep(for: .milliseconds(150)) }
            guard !Task.isCancelled else { return }
            await model.search()
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Search and commands")
        .accessibilityIdentifier("search-palette")
    }

    private func chooseActive() {
        let rows = model.rows
        guard rows.indices.contains(model.active) else { return }
        let row = rows[model.active]
        close()
        Task { await model.choose(row) }
    }

    @ViewBuilder private var results: some View {
        let rows = model.rows
        if let failure = model.failure {
            Text(failure).foregroundStyle(theme["red-t"]).padding(16)
        } else if rows.isEmpty {
            Text(model.searching ? "Searching\u{2026}" : "No matches. Search looks at titles, text, notes, file names and tags.")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity)
                .padding(20)
                .accessibilityIdentifier("palette-empty")
        } else {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 2) {
                        ForEach(Array(rows.enumerated()), id: \.element.id) { index, row in
                            rowView(row, selected: index == model.active)
                                .id(row.id)
                                .onTapGesture {
                                    model.active = index
                                    chooseActive()
                                }
                        }
                    }
                    .padding(8)
                }
                .frame(maxHeight: 420)
                .onChange(of: model.active) { _, new in
                    if rows.indices.contains(new) { proxy.scrollTo(rows[new].id) }
                }
            }
        }
    }

    @ViewBuilder private func rowView(_ row: PaletteRow, selected: Bool) -> some View {
        Group {
            switch row {
            case let .command(c):
                HStack(spacing: 12) {
                    Image(systemName: c.symbol).frame(width: 22).foregroundStyle(theme["accent"])
                    Text(c.title)
                    Spacer()
                    if let hint = c.hint { Text(hint).font(.caption).foregroundStyle(.tertiary) }
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Command: \(c.title)")
            case let .hit(h):
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Text(SearchRouting.label(for: h.entityType))
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(theme["accent-ink"])
                        .padding(.horizontal, 7).padding(.vertical, 2)
                        .background(theme["accent-soft"], in: Capsule())
                        .frame(width: 110, alignment: .leading)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(h.title.isEmpty ? "Untitled" : h.title).fontWeight(.medium).lineLimit(1)
                        Text(SearchRouting.highlighted(h.snippet))
                            .font(.callout).foregroundStyle(.secondary).lineLimit(1)
                    }
                    Spacer(minLength: 8)
                    Text(h.context).font(.caption).foregroundStyle(.tertiary).lineLimit(1)
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel("\(SearchRouting.label(for: h.entityType)): \(h.title.isEmpty ? "Untitled" : h.title)")
            }
        }
        .padding(.horizontal, 10).padding(.vertical, 7)
        .background(selected ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .contentShape(Rectangle())
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

extension Notification.Name {
    static let openFrameShowSearch = Notification.Name("studio.openframe.showSearch")
    static let openFrameShowAbout = Notification.Name("studio.openframe.showAbout")
}

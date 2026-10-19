// The Story workspace: Board, Outline, Characters, Timeline and (for series) Episodes. Board and
// Outline show the same structure from the core; switching views never changes data. Mirrors
// apps/desktop/src/workspaces/story/index.tsx with Mac conventions (a tab strip, an inspector
// on the right, context menus, sheets, Command shortcuts).

import AppKit
import SwiftUI
import OpenFrameCore

struct StoryWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var model: StoryModel?

    var body: some View {
        Group {
            if let model {
                StoryContent(model: model)
            } else {
                ProgressView()
            }
        }
        .task { if model == nil { model = StoryModel(app: app) } }
    }
}

struct StoryContent: View {
    @Bindable var model: StoryModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    private var reloadKey: Int {
        app.changes.key(StoryTables.boardQuery) &+ app.changes.key(StoryTables.series) &+ model.viewRevision
    }

    var body: some View {
        VStack(spacing: 0) {
            tabBar
            Divider()
            content.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        }
        .background(theme["bg"])
        .overlay(alignment: .bottom) { StoryNoticeBar(model: model) }
        .task(id: reloadKey) { await model.reload() }
        .sheet(item: $model.sheet) { StorySheetHost(model: model, sheet: $0) }
        .inspector(isPresented: Binding(get: { model.drawer != nil }, set: { if !$0 { model.openDrawer(nil) } })) {
            if let ref = model.drawer {
                StoryInspector(model: model, ref: ref).inspectorColumnWidth(min: 320, ideal: 400, max: 560)
            }
        }
        .onChange(of: model.showEpisodes) { _, shown in if !shown, model.tab == .episodes { model.tab = .board } }
        .onExitCommand {
            if model.drawer != nil { model.openDrawer(nil) } else { model.clearSelection() }
        }
        .accessibilityIdentifier("story-workspace")
    }

    // MARK: Tabs

    private var tabBar: some View {
        ZStack {
            HStack(spacing: 12) {
            Picker("Story views", selection: $model.tab) {
                ForEach(model.tabs) { Text($0.title).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .accessibilityLabel("Story views")
            .accessibilityIdentifier("story-tabs")
            }
            HStack(spacing: 8) {
            Spacer()
            if model.showEpisodes && model.tab != .episodes {
                Text("Episode").font(.caption).foregroundStyle(theme["muted"])
                Picker("Episode", selection: Binding(
                    get: { model.episodeId ?? "" },
                    set: { id in Task { await model.chooseEpisode(id.isEmpty ? nil : id) } })) {
                    ForEach(model.episodes) { Text($0.label).tag($0.id) }
                    Text("Series level (no episode)").tag("")
                }
                .labelsHidden()
                .frame(maxWidth: 280)
                .accessibilityLabel("Episode")
                .accessibilityIdentifier("story-episode-picker")
            }
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 8)
    }

    @ViewBuilder private var content: some View {
        switch model.tab {
        case .board, .outline: BoardPage(model: model)
        case .characters: CharactersView(story: model)
        case .timeline: StoryTimelineView(story: model)
        case .episodes: StoryEpisodesView(story: model)
        }
    }
}

// MARK: Board page

struct BoardPage: View {
    @Bindable var model: StoryModel
    @AppStorage("of.story.zoom") private var zoomPercent = 100
    @AppStorage("of.story.parkingOpen") private var parkingOpen = true
    @Environment(\.ofTheme) private var theme
    static let zoomLevels = [50, 70, 85, 100, 125, 150]

    var body: some View {
        if let board = model.board {
            VStack(spacing: 0) {
                toolbar(board)
                if model.selectedRefs.count > 1 { selectionBar(board) }
                if model.isEmpty { emptyState } else if model.tab == .board {
                    StoryBoardView(model: model, board: board)
                } else {
                    StoryOutlineView(model: model, board: board)
                }
            }
        } else if let f = model.loadFailure {
            ContentUnavailableView("The Story Board could not be loaded.", systemImage: "exclamationmark.triangle", description: Text(f.message))
        } else {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    // MARK: Toolbar

    private func toolbar(_ board: StoryBoardState) -> some View {
        HStack(spacing: 10) {
            Menu {
                Button("Act") { model.addingAct = true }
                Button("Sequence") {
                    if let last = board.acts.last { model.addingSequenceIn = last.id }
                }
                .disabled(board.acts.isEmpty)
                Button("Beat") { Task { await model.addBeatFromToolbar() } }
                Button("Scene Card") { Task { await model.addSceneFromToolbar() } }
                    .keyboardShortcut("n", modifiers: .command)
                Divider()
                Button("From Idea Vault\u{2026}") {
                    NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.ideaVault.rawValue)
                }
            } label: {
                Label("Add", systemImage: "plus")
            }
            .menuStyle(.borderedButton)
            .fixedSize()
            .accessibilityIdentifier("story-add-menu")

            HStack(spacing: 4) {
                Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                TextField("Search / filter cards", text: $model.filter)
                    .textFieldStyle(.plain)
                    .accessibilityLabel("Search / filter cards")
                if !model.filter.isEmpty {
                    Button { model.filter = "" } label: { Image(systemName: "xmark.circle.fill") }
                        .buttonStyle(.borderless).accessibilityLabel("Clear filter")
                }
            }
            .padding(.horizontal, 8).padding(.vertical, 5)
            .frame(width: 200)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 7, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 7, style: .continuous).strokeBorder(theme["line"]))

            if model.tab == .board {
                HStack(spacing: 2) {
                    let idx = Self.zoomLevels.firstIndex(of: zoomPercent) ?? 3
                    Button { zoomPercent = Self.zoomLevels[max(0, idx - 1)] } label: { Image(systemName: "minus") }
                        .disabled(idx <= 0).accessibilityLabel("Zoom out")
                    Text("\(zoomPercent)%").font(.callout.weight(.semibold)).monospacedDigit().frame(width: 44)
                        .accessibilityLabel("Zoom \(zoomPercent) percent")
                    Button { zoomPercent = Self.zoomLevels[min(Self.zoomLevels.count - 1, idx + 1)] } label: { Image(systemName: "plus") }
                        .disabled(idx >= Self.zoomLevels.count - 1).accessibilityLabel("Zoom in")
                }
                .buttonStyle(.borderless)
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Zoom")
            }
            Spacer()
            if model.tab == .board {
                Toggle(isOn: $parkingOpen) { Label("Parking Lot", systemImage: "parkingsign") }
                    .toggleStyle(.button)
                    .accessibilityIdentifier("story-parking-toggle")
            }
            Button { model.sheet = .build(preselect: []) } label: { Label("Build Screenplay", systemImage: "doc.text") }
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("story-build")
            Menu {
                Button("Card Display Settings\u{2026}") { model.sheet = .cardDisplay }
                Button("Apply Board Order to Screenplay\u{2026}") { model.sheet = .applyOrder }
                Divider()
                Menu("Export Outline") {
                    Button("PDF\u{2026}") { export("pdf") }
                    Button("Plain Text\u{2026}") { export("txt") }
                    Button("CSV\u{2026}") { export("csv") }
                }
            } label: {
                Image(systemName: "ellipsis.circle")
            }
            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
            .accessibilityLabel("More Story Board actions")
            .accessibilityIdentifier("story-more")
        }
        .padding(.horizontal, 16).padding(.vertical, 8)
    }

    private func export(_ format: String) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "Story Board.\(format)"
        panel.message = "Choose where to save the Story Board outline"
        if panel.runModal() == .OK, let url = panel.url {
            Task { await model.export(format: format, to: url.path) }
        }
    }

    private func selectionBar(_ board: StoryBoardState) -> some View {
        let refs = model.selectedRefs
        return HStack(spacing: 8) {
            Text("\(refs.count) \(refs.allSatisfy { $0.kind == .card } ? "cards" : "items") selected").fontWeight(.semibold)
            Spacer()
            Button("Move\u{2026}") { model.sheet = .move(refs) }
            Button("Duplicate") { Task { await model.duplicate(refs) } }
            Button("Park") { Task { await model.park(refs) } }
            Button("Delete", role: .destructive) { Task { await model.requestDelete(refs) } }
            Button { model.clearSelection() } label: { Image(systemName: "xmark") }
                .buttonStyle(.borderless).accessibilityLabel("Clear selection")
        }
        .controlSize(.small)
        .padding(.horizontal, 16).padding(.vertical, 7)
        .background(theme["accent-soft"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Selection actions")
    }

    // MARK: Empty

    @ViewBuilder private var emptyState: some View {
        if model.addingAct {
            ScrollView(.horizontal) {
                HStack(alignment: .top) { NewActColumn(model: model, zoom: Double(zoomPercent) / 100) }.padding(16)
            }
        } else {
            ContentUnavailableView {
                Label("Your Story Board is empty.", systemImage: "rectangle.3.group")
            } description: {
                Text("Scene Cards are short reminders of what happens in each scene. Drag them until the story works. Start shaping the story. Add an Act, Sequence, Beat, or Scene Card, or drag an idea from your Idea Vault.")
            } actions: {
                HStack {
                    Button { Task { await model.newCard(in: nil) } } label: { Label("Add Scene", systemImage: "plus") }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("story-empty-add-scene")
                    Button { model.addingAct = true } label: { Label("Add Act", systemImage: "plus") }
                        .accessibilityIdentifier("story-add-act")
                }
            }
            .accessibilityIdentifier("story-empty")
        }
    }
}

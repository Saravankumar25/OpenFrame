// The Screenplay workspace (Mac phase M4): screenplay and draft choice, the scene navigator, the
// native editor and its sync with the core. The core decides everything; this file only moves
// text and selections between the editor and the core.

import AppKit
import SwiftUI
import OpenFrameCore

/// The paragraphs the editor shows for a document: each scene's heading, then its elements.
/// Line breaks inside an element are line separators (U+2028) in the editor and "\n" in the core.
enum ScreenplayParagraphs {
    static let lineSeparator = "\u{2028}"

    /// Editor paragraphs of a snapshot, through the shared block mapping of the model port.
    @MainActor
    static func from(_ snapshot: ScreenplaySnapshot, rules: ElementRules) -> [EditorParagraph] {
        ScreenplayModel.blocks(from: snapshot, rules: rules).map { b in
            EditorParagraph(
                kind: b.kind,
                text: b.text.replacingOccurrences(of: "\n", with: lineSeparator),
                elementId: b.kind == .sceneHeading ? nil : b.id,
                customElementId: b.customElementId,
                sceneId: b.kind == .sceneHeading ? b.id : nil)
        }
    }

    static func blocks(_ paragraphs: [EditorParagraph]) -> [EditorBlock] {
        paragraphs.map { p in
            EditorBlock(
                id: p.kind == .sceneHeading ? (p.sceneId ?? p.elementId) : p.elementId,
                kind: p.kind,
                text: p.text.replacingOccurrences(of: lineSeparator, with: "\n"),
                customElementId: p.customElementId)
        }
    }

    /// Read the editor as a snapshot. New or repeated ids are made unique and written back, so the
    /// next read sees the same ids.
    @MainActor
    static func read(_ view: ScreenplayTextView) -> ScreenplaySnapshot {
        var blocks = blocks(view.paragraphs())
        let before = blocks.map(\.id)
        ScreenplayModel.assignIds(&blocks)
        let after = blocks.map(\.id)
        if before != after { view.assign(ids: after) }
        return ScreenplayModel.snapshot(fromBlocks: blocks)
    }
}

@MainActor
@Observable
final class ScreenplayWorkspaceModel {
    /// A screenplay and draft asked for while the workspace was not on screen (Build Screenplay
    /// posts its target before the workspace appears). Read once when the workspace loads.
    static var pendingTarget: (screenplayId: String?, draftId: String?, sceneId: String?)?

    var overview: ScreenplayOverview?
    var document: ScreenplayDocument?
    var failure: CoreFailure?
    var syncStatus: String = "saved"
    var currentScene: String?
    var showNewDraft = false
    /// Standard, Writing Room or Focus. Kept per project and user in the layout record.
    var mode: ScreenplayMode = .standard
    /// Characters and locations for the suggestion list.
    var sources: WritingSuggestionSources?
    /// Pages of the saved draft (the core counts them).
    var pagination: ScreenplayPagination?
    /// Where the caret is in its scene: the page shown is worked out from it.
    var sceneProgress: (sceneId: String, fraction: Double)?
    /// A short line about what just happened ("Added MEERA to Characters"); clears itself.
    var notice: String?
    var currentCustomId: String?
    /// The optional navigator columns the writer chose (kept in the layout record), and what the
    /// core reports for them. The data loads once a column is shown.
    var navColumns: [NavigatorColumn] = []
    var navData: NavigatorColumnData?
    // Replace All (Command+Option+F).
    var showReplace = false
    /// Counts requests to put the caret in the Find field (the bar is already open when it rises).
    var replaceFocusRequest = 0
    var findText = ""
    var replaceText = ""
    var matchCase = false
    var wholeWord = false
    var scopeIsScene = true

    @ObservationIgnored private var noticeTask: Task<Void, Never>?
    @ObservationIgnored weak var textView: ScreenplayTextView?
    @ObservationIgnored var sync: ScreenplaySync?
    /// Escape in the editor with nothing to dismiss (leaves Focus Mode).
    @ObservationIgnored var onEscape: (() -> Void)?
    @ObservationIgnored let rules: SharedScreenplayRules
    @ObservationIgnored private weak var app: AppModel?
    @ObservationIgnored private var screenplayId: String?

    init(rules: SharedScreenplayRules) {
        self.rules = rules
    }

    var draft: ScreenplayDraftDto? { document?.draft }
    var canEdit: Bool { document?.canEdit ?? false }

    func attach(_ app: AppModel) {
        self.app = app
        Self.register(self)
    }

    // MARK: Leaving: everything typed reaches the core first

    private final class Weak { weak var model: ScreenplayWorkspaceModel?; init(_ m: ScreenplayWorkspaceModel) { model = m } }
    private static var live: [Weak] = []

    private static func register(_ model: ScreenplayWorkspaceModel) {
        live.removeAll { $0.model == nil }
        if !live.contains(where: { $0.model === model }) { live.append(Weak(model)) }
    }

    /// Send what every open Screenplay workspace holds to the core. Call it before the project
    /// closes (`AppModel.closeProject`) and before the app quits (`applicationShouldTerminate`
    /// answers `.terminateLater`, awaits this, then replies).
    static func flushAll() async {
        for entry in live { await entry.model?.flush() }
    }

    /// Connect the text view: what it shows besides the text, and what it asks of the app.
    func bind(_ view: ScreenplayTextView) {
        textView = view
        view.rules = rules
        view.onCreateSuggestion = { [weak self] kind, name, movesOn in self?.createValue(kind, name, movesOn: movesOn) }
        // A known name filled by Enter is saved on its own before the Dialogue line is added.
        view.beforeSuggestionMoveOn = { [weak self] in self?.sync?.saveThen {} }
        view.onLearnSpelling = { [weak self] word in self?.learn(word) }
        view.onEscape = { [weak self] in self?.onEscape?() }
        syncViewAttributes()
    }

    /// Scene numbers, custom elements and suggestion values go to the text view. Custom elements
    /// first: the paragraphs are styled from them when the text loads.
    func syncViewAttributes() {
        guard let view = textView else { return }
        let definitions = document?.customElements ?? []
        if view.customDefinitions != definitions { view.customDefinitions = definitions }
        let labels = Dictionary((document?.scenes ?? []).map { ($0.id, $0.label) }, uniquingKeysWith: { a, _ in a })
        if view.sceneLabels != labels { view.sceneLabels = labels }
        if view.suggestionSources != sources {
            view.suggestionSources = sources
            view.setKnownNames((sources?.characters ?? []).map(\.name) + (sources?.locations ?? []).map(\.name))
        }
    }

    // MARK: Layout mode

    /// Read the saved mode (the layout record is shared with the Windows app).
    func loadLayout() async {
        guard let core = app?.core else { return }
        if let state = try? await core.screenplayViewState(ScreenplayViewStateArgs()) {
            mode = ScreenplayMode.from(state)
            navColumns = NavigatorColumn.from(state)
        }
        await loadNavigatorColumns()
    }

    // MARK: Navigator columns

    /// Show or hide one navigator column and keep the choice (other keys of the record stay).
    func toggleNavColumn(_ column: NavigatorColumn) {
        navColumns = NavigatorColumn.toggled(navColumns, column)
        Task { await loadNavigatorColumns() }
        saveLayout(columns: true, mode: false)
    }

    /// The last save of the layout record. Saves run one after the other: two quick changes
    /// each read the record, change their own key and write it back, and run in parallel the
    /// later read could miss the earlier write and undo it.
    private var layoutSave: Task<Void, Never>?

    /// Save the chosen keys as they are when the save runs (the newest values win); the other
    /// keys of the record stay.
    private func saveLayout(columns: Bool, mode saveMode: Bool) {
        guard let core = app?.core else { return }
        let before = layoutSave
        layoutSave = Task { [weak self] in
            await before?.value
            guard let self else { return }
            var state = try? await core.screenplayViewState(ScreenplayViewStateArgs())
            if saveMode { state = ScreenplayMode.merged(state, mode: self.mode) }
            if columns { state = NavigatorColumn.merged(state, columns: self.navColumns) }
            _ = try? await core.screenplaySetViewState(ScreenplaySetViewStateArgs(state: state ?? .object([:])))
        }
    }

    /// Ask the core for the pages, notes, revision and production numbers of the draft's scenes.
    func loadNavigatorColumns() async {
        guard !navColumns.isEmpty, let core = app?.core, let id = draft?.id else {
            if navColumns.isEmpty { navData = nil }
            return
        }
        if let r = try? await core.writingNavigatorColumns(WritingNavigatorArgs(draftId: id)), id == draft?.id {
            navData = NavigatorColumnData(r)
        }
    }

    /// Change the mode and save it, keeping the other keys of the record as they are.
    func setMode(_ new: ScreenplayMode) {
        guard new != mode else { return }
        mode = new
        saveLayout(columns: false, mode: true)
    }

    /// Say what just happened. The line also shows as a toast, which stays reachable when the
    /// toolbar is crowded (the toolbar has no room for a message of any length).
    func announce(_ message: String) {
        notice = message
        ToastCenter.shared.show(message)
        noticeTask?.cancel()
        noticeTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(5))
            if !Task.isCancelled { self?.notice = nil }
        }
    }

    func loadSources() async {
        guard let core = app?.core else { return }
        if let loaded = try? await core.writingSuggestionSources(WritingSuggestionSourcesArgs()) {
            sources = loaded
            syncViewAttributes()
        }
    }

    func refreshPagination() async {
        guard let core = app?.core, let id = draft?.id else { return }
        if let p = try? await core.screenplayPagination(ScreenplayDraftIdArgs(draftId: id)), p.draftId == draft?.id {
            pagination = p
        }
        await loadNavigatorColumns()
    }

    func caretMoved() {
        sceneProgress = textView?.caretSceneProgress()
    }

    /// "Page 3 of 12", or "12 pages" while the caret is in a scene the core does not know yet.
    var pageLabel: String? {
        guard let p = pagination else { return nil }
        let total = Int(p.pageCount)
        if let progress = sceneProgress, let scene = p.scenes.first(where: { $0.sceneId == progress.sceneId }) {
            let first = Int(scene.firstPage), last = max(Int(scene.lastPage), first)
            let page = min(last, first + Int(progress.fraction * Double(last - first + 1)))
            return "Page \(page) of \(total)"
        }
        return total == 1 ? "1 page" : "\(total) pages"
    }

    // MARK: Suggestions and spelling (through the core)

    /// "Create New" in the suggestion list. After Enter moved on to Dialogue the new value is recorded
    /// between the typed name and the Dialogue line (`saveThen`), so project undo takes them back
    /// in the order they were made.
    func createValue(_ kind: SuggestCreate, _ name: String, movesOn: Bool) {
        let work: @MainActor () async throws -> Void = { [weak self] in
            guard let self, let core = self.app?.core else { return }
            do {
                switch kind {
                case .character: _ = try await core.storyCreateCharacter(StoryCreateCharacterArgs(name: name))
                case .location: _ = try await core.locationsCreate(ProductionLocationCreateArgs(name: name))
                }
                self.announce(kind == .character ? "Added \(name) to Characters" : "Added \(name) to Locations")
                await self.loadSources()
            } catch {
                self.app?.show("Could not add \(name)", error)
            }
        }
        if movesOn, let sync {
            sync.saveThen(work)
        } else {
            Task { try? await work() }
        }
    }

    /// "Learn Spelling": the word joins the core's dictionary, which the Windows app reads as well.
    func learn(_ word: String) {
        Task { [weak self] in
            guard let self, let core = self.app?.core else { return }
            do {
                _ = try await core.writingDictionaryAdd(WritingWordArgs(word: word))
                let message = "Added “\(word)” to your dictionary"
                self.notice = message
                // The toast offers Unlearn, which takes the word out of both dictionaries again.
                ToastCenter.shared.show(message, actionLabel: "Unlearn") { [weak self] in await self?.unlearn(word) }
            } catch {
                self.app?.show("Could not add the word", error)
            }
        }
    }

    /// Take a learned word back: out of the core's dictionary and the system's, so it is
    /// underlined again.
    func unlearn(_ word: String) async {
        guard let core = app?.core else { return }
        do {
            _ = try await core.writingDictionaryRemove(WritingWordArgs(word: word))
            textView?.unlearnSpelling(word: word)
            let message = "Removed “\(word)” from your dictionary"
            notice = message
            ToastCenter.shared.show(message)
        } catch {
            app?.show("Could not remove the word", error)
        }
    }

    // MARK: Replace All

    func openReplace() {
        if let view = textView, view.selectedRange().length > 0, view.selectedRange().length < 80 {
            let text = ((view.string as NSString).substring(with: view.selectedRange()))
            if !text.contains("\n"), !text.contains(ScreenplayParagraphs.lineSeparator) { findText = text }
        }
        showReplace = true
        replaceFocusRequest += 1
    }

    /// Replace in the draft or the current scene through the core, after the typed text is saved.
    /// The editor then shows the core's text. Project undo takes it back in one step.
    func replaceAll() async {
        guard canEdit, let core = app?.core, let draftId = draft?.id, !findText.isEmpty else { return }
        await flush()
        await loadDocument(draftId)
        var sceneId: String?
        var sceneLabel: String?
        if scopeIsScene {
            guard let id = textView?.caretSceneProgress()?.sceneId,
                  let scene = document?.scenes.first(where: { $0.id == id }) else {
                announce("Put the cursor in a scene first.")
                return
            }
            sceneId = id
            sceneLabel = scene.label
        }
        do {
            let result = try await core.screenplayReplaceAll(ScreenplayReplaceAllArgs(
                draftId: draftId, query: findText, replacement: replaceText, matchCase: matchCase,
                wholeWord: wholeWord, includeNotes: true, sceneId: sceneId))
            await loadDocument(draftId)
            let n = Int(result.count)
            if n == 0 {
                announce("No matches")
            } else {
                announce("Replaced \(n) match\(n == 1 ? "" : "es")" + (sceneLabel.map { " in Scene \($0)" } ?? ""))
            }
        } catch {
            app?.show("Could not replace", error)
        }
    }

    /// The screenplay the person had open last in this project (kept in the layout record, beside the mode).
    @ObservationIgnored private var rememberedScreenplayId: String?

    private func savedScreenplayId() async -> String? {
        guard let core = app?.core, case let .object(o)? = try? await core.screenplayViewState(ScreenplayViewStateArgs()),
              case let .string(id)? = o["lastScreenplayId"] else { return nil }
        return id
    }

    private func rememberScreenplay(_ id: String) {
        guard id != rememberedScreenplayId, let core = app?.core else { return }
        rememberedScreenplayId = id
        Task {
            let current = try? await core.screenplayViewState(ScreenplayViewStateArgs())
            var o: [String: JSONValue] = [:]
            if case let .object(existing)? = current { o = existing }
            o["lastScreenplayId"] = .string(id)
            _ = try? await core.screenplaySetViewState(ScreenplaySetViewStateArgs(state: .object(o)))
        }
    }

    func loadOverview() async {
        guard let core = app?.core else { return }
        do {
            if screenplayId == nil, let saved = await savedScreenplayId() {
                // Reopen the screenplay that was open last; a deleted one falls back to the default.
                if let back = try? await core.screenplayOverview(ScreenplayOverviewArgs(screenplayId: saved)), back.screenplay?.id == saved { screenplayId = saved; rememberedScreenplayId = saved }
            }
            var o = try await core.screenplayOverview(ScreenplayOverviewArgs(screenplayId: screenplayId))
            if o.screenplays.isEmpty, o.permissions.edit {
                // A project starts without a screenplay; the first visit makes one, as on Windows.
                _ = try await core.screenplayCreate(CreateScreenplayArgs())
                o = try await core.screenplayOverview(ScreenplayOverviewArgs(screenplayId: screenplayId))
            }
            overview = o
            screenplayId = o.screenplay?.id
            if let id = screenplayId { rememberScreenplay(id) }
            failure = nil
            if let draftId = o.screenplay?.currentDraftId ?? o.drafts.first?.id {
                await loadDocument(draftId)
            }
        } catch {
            failure = CoreFailure(asCore: error)
        }
    }

    func loadDocument(_ draftId: String) async {
        guard let core = app?.core else { return }
        let openAtStart = document?.draft.id
        do {
            let doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
            // A refresh of the draft that was open when it began must not bring that draft back
            // after the person switched to another one while the answer was on its way.
            if let openAtStart, openAtStart == draftId, let now = document?.draft.id, now != openAtStart { return }
            if let sync, sync.isDirty, doc.draft.id == document?.draft.id {
                // Never replace text the person is typing; the sync sends it first.
                return
            }
            let switching = doc.draft.id != document?.draft.id
            document = doc
            syncViewAttributes()
            if switching || sync == nil {
                makeSync(for: doc)
                textView?.load(ScreenplayParagraphs.from(ScreenplayModel.snapshot(fromScenes: doc.scenes), rules: rules.table))
            } else {
                sync?.receiveServer(seq: doc.seq, snapshot: ScreenplayModel.snapshot(fromScenes: doc.scenes))
            }
            textView?.isEditable = doc.canEdit
            await refreshPagination()
        } catch {
            failure = CoreFailure(asCore: error)
        }
    }

    private func makeSync(for doc: ScreenplayDocument) {
        sync?.dispose()
        let draftId = doc.draft.id
        let table = rules.table
        sync = ScreenplaySync(
            initial: ScreenplayModel.snapshot(fromScenes: doc.scenes),
            seq: doc.seq,
            deps: ScreenplaySync.Deps(
                send: { [weak self] ops in
                    guard let core = self?.app?.core else {
                        throw CoreFailure(code: "internal.no_core", message: "OpenFrame is not ready.")
                    }
                    return try await core.screenplayApplyEdits(ScreenplayApplyEditsArgs(draftId: draftId, ops: ops)).seq
                },
                read: { [weak self] in
                    guard let view = self?.textView else { return [] }
                    return ScreenplayParagraphs.read(view)
                },
                apply: { [weak self] snap in
                    self?.textView?.load(ScreenplayParagraphs.from(snap, rules: table))
                },
                onError: { [weak self] error in
                    self?.app?.show("Your last change could not be saved", error)
                    return .retry
                },
                onStatus: { [weak self] status in
                    self?.syncStatus = "\(status)"
                    if status == .saved { Task { await self?.refreshPagination() } }
                }))
    }

    func textChanged() { sync?.markDirty() }

    /// Show a screenplay and draft chosen elsewhere (Build Screenplay in Story).
    func show(screenplayId: String?, draftId: String?, sceneId: String? = nil) async {
        await flush()
        if let screenplayId { self.screenplayId = screenplayId }
        await loadOverview()
        if let draftId, draftId != draft?.id { await loadDocument(draftId) }
        // A search result or a Story link names a scene: put the caret on its heading.
        if let sceneId, let scenes = document?.scenes, let i = scenes.firstIndex(where: { $0.id == sceneId }) {
            let paragraph = scenes.prefix(i).reduce(0) { $0 + 1 + $1.elements.count }
            textView?.reveal(paragraph: paragraph)
        }
    }

    /// Store everything typed before leaving (workspace switch, close, quit).
    func flush() async { await sync?.flush() }

    // MARK: Drafts

    func switchDraft(_ id: String) async {
        await flush()
        guard let core = app?.core else { return }
        do {
            _ = try await core.screenplaySetCurrentDraft(ScreenplayDraftArgs(draftId: id))
            await loadOverview()
        } catch { app?.show("Could not switch the draft", error) }
    }

    func newDraft(name: String, note: String?) async {
        await flush()
        guard let core = app?.core, let source = draft?.id else { return }
        do {
            _ = try await core.screenplayNewDraft(ScreenplayNewDraftArgs(sourceDraftId: source, name: name, note: note, makeCurrent: true))
            await loadOverview()
        } catch { app?.show("Could not create the draft", error) }
    }

    func renumber() async {
        await flush()
        guard let core = app?.core, let id = draft?.id else { return }
        do { _ = try await core.screenplayRenumberScenes(ScreenplayDraftIdArgs(draftId: id)) }
        catch { app?.show("Could not renumber the scenes", error) }
    }
}

// MARK: - Views

struct ScreenplayWorkspace: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = ScreenplayWorkspaceModel(rules: SharedScreenplayRules.shared)
    @State private var currentKind: ElementType = .action
    @State private var currentIndex = 0
    @State private var panels = ScreenplayPanelState()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private static let contentTables = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element", "screenplay_custom_element", "comment"]
    private static let sourceTables = ["story_character", "cast_member", "location", "story_scene_card"]

    var body: some View {
        let focus = model.mode == .focus
        GeometryReader { geo in
        HSplitView {
            if !focus {
                // In a narrow window the scene list gives way first, so the page keeps most of the room.
                SceneNavigator(model: model, panels: panels, currentIndex: currentIndex)
                    .frame(minWidth: 160, idealWidth: 240, maxWidth: max(160, min(340, (geo.size.width * 0.3).rounded())))
            }
            VStack(spacing: 0) {
                if model.showReplace && !focus { ScreenplayReplaceBar(model: model) }
                ZStack {
                    theme["well"]
                    ScreenplayEditorHost(model: model, onMove: { p, i in
                        currentKind = p.kind
                        currentIndex = i
                        model.currentCustomId = p.kind == .custom ? p.customElementId : nil
                    })
                }
            }
            // The page scrolls sideways in a narrow pane, so the pane can be narrower than the page
            // and the workspace still fits a small window with the AI Assistant open.
            .frame(minWidth: 220)
            .overlay(alignment: .topTrailing) { if focus { focusBar } }
        }
        }
        .toolbar(focus ? .hidden : .automatic, for: .windowToolbar)
        .toolbar { toolbar }
        .background { modeShortcuts }
        .onChange(of: model.mode) { _, mode in
            // Focus hides the side pane; Writing Room opens it.
            if mode == .focus { panels.inspectorShown = false }
            if mode == .writingRoom { panels.inspectorShown = true }
            // The project window hides its workspace sidebar too while the writer is in Focus.
            NotificationCenter.default.post(name: .openFrameScreenplayFocus, object: mode == .focus)
        }
        .onDisappear { NotificationCenter.default.post(name: .openFrameScreenplayFocus, object: false) }
        .onChange(of: panels.inspectorShown) { _, shown in
            // Asking for a panel leaves Focus, as on Windows.
            if shown && model.mode == .focus { setMode(.standard) }
        }
        .overlay { if let f = model.failure, model.document == nil { ContentUnavailableView("Screenplay unavailable", systemImage: "doc.text", description: Text(f.message)) } }
        .task {
            model.attach(app)
            model.onEscape = { if model.mode == .focus { setMode(.standard) } }
            await model.loadLayout()
            if let target = ScreenplayWorkspaceModel.pendingTarget {
                ScreenplayWorkspaceModel.pendingTarget = nil
                await model.show(screenplayId: target.screenplayId, draftId: target.draftId, sceneId: target.sceneId)
            } else {
                await model.loadOverview()
            }
        }
        .task(id: app.changes.key(Self.contentTables)) {
            if let id = model.draft?.id { await model.loadDocument(id) }
        }
        .task(id: app.changes.key(Self.sourceTables)) { await model.loadSources() }
        .onDisappear { Task { await model.flush() } }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowScreenplayTarget)) { note in
            let info = note.userInfo ?? [:]
            ScreenplayWorkspaceModel.pendingTarget = nil
            Task { await model.show(screenplayId: info["screenplayId"] as? String, draftId: info["draftId"] as? String, sceneId: info["sceneId"] as? String) }
        }
        .screenplayPanels(model: model, state: panels, caretIndex: currentIndex)
        .interchangeAnchor()
        .accessibilityIdentifier("screenplay-workspace")
    }

    /// Change the mode; with Reduce Motion on, the layout changes without animation.
    private func setMode(_ mode: ScreenplayMode) {
        if reduceMotion { model.setMode(mode) } else { withAnimation(.easeInOut(duration: 0.18)) { model.setMode(mode) } }
    }

    /// The small bar Focus Mode keeps: draft name, save state and the way out.
    private var focusBar: some View {
        HStack(spacing: 10) {
            Text(model.draft?.name ?? "").font(.callout.weight(.semibold)).lineLimit(1)
            Button("Exit Focus") { setMode(.standard) }
                .controlSize(.small)
                .accessibilityIdentifier("focus-exit")
            Text(model.syncStatus == "saved" ? "Saved" : model.syncStatus == "error" ? "Not saved" : "Saving")
                .font(.caption).foregroundStyle(.secondary)
                .accessibilityIdentifier("focus-save-status")
        }
        .padding(.horizontal, PageSpacing.small + 4).padding(.vertical, PageSpacing.small - 2)
        .background(theme["panel"].opacity(0.95), in: RoundedRectangle(cornerRadius: 10))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(theme["line"]))
        .padding(PageSpacing.medium)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Focus Mode")
        .accessibilityIdentifier("focus-bar")
    }

    /// Shortcuts that stay alive when the toolbar is hidden.
    private var modeShortcuts: some View {
        ZStack {
            Button("Focus Mode") { setMode(model.mode == .focus ? .standard : .focus) }
                .keyboardShortcut("f", modifiers: [.command, .shift])
            Button("Writing Room Mode") { setMode(model.mode == .writingRoom ? .standard : .writingRoom) }
                .keyboardShortcut("r", modifiers: [.command, .shift])
        }
        .opacity(0).allowsHitTesting(false).accessibilityHidden(true)
    }

    @ToolbarContentBuilder private var toolbar: some ToolbarContent {
        ScreenplayPanelToolbar(state: panels, model: model, setMode: setMode)
        ToolbarItem(placement: .principal) {
            Menu {
                ForEach(SharedScreenplayRules.shared.menuKinds, id: \.self) { k in
                    Button { model.textView?.setKind(k) } label: {
                        if currentKind == k { Label(SharedScreenplayRules.shared.label(for: k), systemImage: "checkmark") }
                        else { Text(SharedScreenplayRules.shared.label(for: k)) }
                    }
                }
                let customs = model.document?.customElements ?? []
                if !customs.isEmpty {
                    Divider()
                    ForEach(customs, id: \.id) { c in
                        Button { model.textView?.setCustomElement(c.id) } label: {
                            if currentKind == .custom && model.currentCustomId == c.id { Label(c.name, systemImage: "checkmark") }
                            else { Text(c.name) }
                        }
                    }
                }
                Divider()
                Button("Insert Page Break") { model.textView?.insertPageBreak() }
            } label: {
                Text(elementLabel).frame(minWidth: 130, alignment: .leading)
            }
            .disabled(!model.canEdit)
            .help("Element of the current line (Command+1 to Command+7, Tab to change). Insert Page Break: Command+Return")
            .accessibilityLabel("Element")
            .accessibilityValue(elementLabel)
            .accessibilityIdentifier("element-picker")
        }
        ToolbarItem(placement: .automatic) {
            // Opens the bar and never closes it: a second press of the same button (a double
            // click, or an accessibility press followed by a click) must not shut it again. The
            // bar's own Close button and Escape close it.
            Button { model.openReplace() } label: {
                Label("Replace", systemImage: "arrow.left.arrow.right")
            }
            .keyboardShortcut("f", modifiers: [.command, .option])
            .disabled(!model.canEdit)
            .help("Replace in the script (Command+Option+F). Command+F finds.")
            .accessibilityIdentifier("replace-toggle")
        }
        ToolbarItem(placement: .automatic) {
            Menu {
                if let drafts = model.overview?.drafts {
                    ForEach(drafts, id: \.id) { d in
                        Button { Task { await model.switchDraft(d.id) } } label: {
                            if d.isCurrent { Label(d.name, systemImage: "checkmark") } else { Text(d.name) }
                        }
                    }
                }
                Divider()
                Button("New Draft…") { panels.sheet = .newDraft }.disabled(!model.canEdit && model.draft?.lockedAt == nil)
                Button("Manage Drafts…") { panels.sheet = .drafts }
                Button("Renumber Scenes") { Task { await model.renumber() } }.disabled(!model.canEdit)
            } label: {
                Label(model.draft?.name ?? "Draft", systemImage: "square.stack")
            }
            .help("Drafts")
            .accessibilityLabel("Draft: \(model.draft?.name ?? "none"). Switch draft")
            .accessibilityIdentifier("draft-menu")
        }
        ToolbarItem(placement: .automatic) {
            ScreenplayToolsMenu(state: panels, model: model, setMode: setMode)
        }
        ToolbarItem(placement: .automatic) {
            // The page count and the save state share one item: the toolbar has room for little
            // more, and both are short.
            HStack(spacing: 8) {
                if let page = model.pageLabel {
                    Text(page)
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                        .help("\(model.pagination?.paper ?? ""), \(Int(model.pagination?.linesPerPage ?? 0)) lines per page")
                        .accessibilityLabel(page)
                        .accessibilityIdentifier("editor-page-count")
                }
                Text(model.syncStatus == "saved" ? "Saved" : model.syncStatus == "error" ? "Not saved" : "Saving")
                    .font(.caption)
                    .foregroundStyle(model.syncStatus == "error" ? theme["red-t"] : .secondary)
                    .accessibilityIdentifier("editor-save-status")
            }
            .fixedSize()
        }
    }

    private var elementLabel: String {
        if currentKind == .custom, let id = model.currentCustomId,
           let name = model.document?.customElements.first(where: { $0.id == id })?.name { return name }
        return SharedScreenplayRules.shared.label(for: currentKind)
    }
}

/// Scenes of the draft, numbered as the core numbers them. Clicking a scene moves the caret there.
/// Optional columns (Page, Notes, Revision, Production number) show what the core reports.
struct SceneNavigator: View {
    let model: ScreenplayWorkspaceModel
    let panels: ScreenplayPanelState
    let currentIndex: Int
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let scenes = model.document?.scenes ?? []
        let shown = NavigatorColumnData.shown(model.navColumns, model.navData)
        VStack(spacing: 0) {
            HStack {
                Text("Scenes (\(scenes.count))").font(.headline).accessibilityAddTraits(.isHeader)
                Spacer()
                Menu {
                    ForEach(NavigatorColumn.allCases, id: \.self) { c in
                        Toggle(NavigatorColumnData.menuTitle(c, chosen: model.navColumns, data: model.navData), isOn: Binding(
                            get: { model.navColumns.contains(c) },
                            set: { _ in model.toggleNavColumn(c) }))
                            .disabled(NavigatorColumnData.isDisabled(c, chosen: model.navColumns, data: model.navData))
                    }
                } label: {
                    Image(systemName: "tablecells")
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .help("Show or hide columns")
                .accessibilityLabel("Navigator columns")
                .accessibilityIdentifier("navigator-columns")
            }
            .padding(.horizontal, PageSpacing.medium).padding(.vertical, PageSpacing.small)
        List {
            Section {
                if !shown.isEmpty {
                    HStack(spacing: 8) {
                        Text("#").frame(minWidth: 22, alignment: .trailing)
                        Text("Heading").frame(maxWidth: .infinity, alignment: .leading)
                        ForEach(shown, id: \.self) { c in Text(c.heading).frame(width: columnWidth(c), alignment: .trailing) }
                    }
                    .font(.caption).foregroundStyle(theme["muted"])
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("navigator-column-heads")
                }
                ForEach(Array(scenes.enumerated()), id: \.element.id) { i, scene in
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Button {
                            model.textView?.reveal(paragraph: model.paragraphIndex(ofScene: scene.id) ?? 0)
                            model.textView?.window?.makeFirstResponder(model.textView)
                        } label: {
                            HStack(alignment: .firstTextBaseline, spacing: 8) {
                                Text(scene.label).font(.caption.monospacedDigit()).foregroundStyle(.secondary).frame(minWidth: 22, alignment: .trailing)
                                Text(scene.heading.isEmpty ? "Untitled scene" : scene.heading.uppercased())
                                    .font(.callout).lineLimit(2)
                                Spacer(minLength: 0)
                            }
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Scene \(scene.label), \(scene.heading)")
                        .accessibilityIdentifier("scene-row-\(scene.id)")
                        ForEach(shown, id: \.self) { c in
                            if let cell = NavigatorColumnData.cell(c, model.navData?.rows[scene.id]) {
                                Text(cell.value).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                                    .frame(width: columnWidth(c), alignment: .trailing)
                                    .accessibilityLabel(cell.label)
                                    .accessibilityIdentifier("navigator-cell-\(c.rawValue)")
                            } else {
                                Color.clear.frame(width: columnWidth(c), height: 1).accessibilityHidden(true)
                            }
                        }
                    }
                    .contextMenu {
                        Button("Scene Hub") { panels.showSceneHub(sceneId: scene.id) }
                        Button("Add Comment") { panels.addComment(sceneId: scene.id) }
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .accessibilityIdentifier("scene-navigator")
        }
    }

    private func columnWidth(_ c: NavigatorColumn) -> CGFloat {
        switch c {
        case .page, .notes: 34
        case .revision: 52
        case .production: 46
        }
    }
}

/// The text view inside a scroll view, centred on a page-coloured column.
struct ScreenplayEditorHost: NSViewRepresentable {
    let model: ScreenplayWorkspaceModel
    let onMove: (EditorParagraph, Int) -> Void
    @Environment(\.ofTheme) private var theme

    func makeCoordinator() -> Coordinator { Coordinator(model: model, onMove: onMove) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        // A pane narrower than the page scrolls sideways (the page keeps its width).
        scroll.hasHorizontalScroller = true
        scroll.autohidesScrollers = true
        scroll.drawsBackground = false
        let view = ScreenplayTextView.make()
        model.bind(view)
        view.screenplayDelegate = context.coordinator
        view.minSize = NSSize(width: 0, height: 0)
        view.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        view.autoresizingMask = [.width]
        scroll.documentView = view
        if let doc = model.document {
            view.load(ScreenplayParagraphs.from(ScreenplayModel.snapshot(fromScenes: doc.scenes), rules: model.rules.table))
            view.isEditable = doc.canEdit
        }
        return scroll
    }

    /// The scroll view takes exactly the room the pane offers; the page inside it may be wider
    /// and then scrolls sideways (without this the scroll view grew to the page's width and the
    /// pane cut it at both edges).
    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSScrollView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 600, height: proposal.height ?? 600)
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let view = scroll.documentView as? ScreenplayTextView else { return }
        view.backgroundColor = NSColor(theme["panel"])
        view.insertionPointColor = NSColor(theme["ink"])
        context.coordinator.onMove = onMove
    }

    @MainActor
    final class Coordinator: NSObject, ScreenplayTextViewDelegate {
        let model: ScreenplayWorkspaceModel
        var onMove: (EditorParagraph, Int) -> Void
        init(model: ScreenplayWorkspaceModel, onMove: @escaping (EditorParagraph, Int) -> Void) {
            self.model = model
            self.onMove = onMove
        }
        func screenplayTextDidChange(_ view: ScreenplayTextView) { model.textChanged() }
        func screenplayTextView(_ view: ScreenplayTextView, didMoveInto paragraph: EditorParagraph, index: Int) {
            onMove(paragraph, index)
            model.caretMoved()
        }
    }
}

extension Notification.Name {
    /// Posted with a Bool: the screenplay entered (true) or left (false) Focus Mode.
    static let openFrameScreenplayFocus = Notification.Name("studio.openframe.screenplayFocus")
}

// The Story workspace's model. It holds what the screen shows (the board, the series, the chosen
// episode, the selection, the open drawer, the notice line) and sends every change to a `story.*`
// operation. The core decides what is allowed and does the moving; nothing here stores story data.
// Behaviour mirrors apps/desktop/src/workspaces/story (index.tsx, actions.ts, dnd.tsx, items.tsx).

import Foundation
import Observation
import OpenFrameCore

enum StoryTab: String, CaseIterable, Identifiable {
    case board, outline, characters, timeline, episodes
    var id: String { rawValue }
    var title: String {
        switch self {
        case .board: "Board"
        case .outline: "Outline"
        case .characters: "Characters"
        case .timeline: "Timeline"
        case .episodes: "Episodes"
        }
    }
}

/// A sheet the workspace asks for.
enum StorySheet: Identifiable {
    case deleteAct(String)
    case deleteSequence(String)
    case deleteLinked([StoryItemRef])
    case move([StoryItemRef])
    case actNote(String)
    case build(preselect: [String])
    case applyOrder
    case cardDisplay

    var id: String {
        switch self {
        case let .deleteAct(id): "delete-act-\(id)"
        case let .deleteSequence(id): "delete-sequence-\(id)"
        case let .deleteLinked(refs): "delete-linked-\(refs.map(\.key).joined(separator: ","))"
        case let .move(refs): "move-\(refs.map(\.key).joined(separator: ","))"
        case let .actNote(id): "act-note-\(id)"
        case .build: "build"
        case .applyOrder: "apply-order"
        case .cardDisplay: "card-display"
        }
    }
}

/// A line at the bottom of the screen that says what just happened, with Undo when the change
/// went through the project history.
struct StoryNotice: Identifiable, Equatable {
    let id = UUID()
    var text: String
    var undoable: Bool
}

/// What is being dragged.
struct StoryDragState: Equatable {
    var refs: [StoryItemRef]
    var actId: String?
    var label: String
    var kinds: Set<StoryItemKind> { Set(refs.map(\.kind)) }
    var moving: Set<String> { Set(refs.map(\.key)) }
}

/// A place a dragged item can be dropped on.
enum StoryDropZone: Equatable {
    case beside(StoryItemRef)
    case into(StoryContainerRef, atStart: Bool)
    case act(String)
}

struct StoryEpisodeEntry: Identifiable, Hashable {
    var episode: StoryEpisodeDto
    var seasonTitle: String
    var id: String { episode.id }
    var code: String { "E" + String(format: "%02d", Int(episode.number)) }
    /// The label of the Episode list, as on Windows: "Season 1 · E01 Pilot".
    var label: String { "\(seasonTitle.isEmpty ? "" : seasonTitle + " \u{00B7} ")\(code) \(episode.title)" }
}

/// Tables a screen reads, as on Windows (api/story.ts).
enum StoryTables {
    static let board = [
        "story_act", "story_sequence", "story_beat", "story_scene_card", "story_attachment",
        "story_character", "story_character_card_link", "comment", "asset",
    ]
    static let screenplay = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element"]
    static let boardQuery = board + screenplay + ["breakdown_element", "production_scene_state", "shot", "storyboard"]
    static let characters = [
        "story_character", "story_character_relationship", "story_character_card_link", "story_scene_card",
        "asset", "cast_member", "catalog_item",
    ] + screenplay
    static let series = ["season", "episode", "story_scene_card", "project"]
    static let timeline = screenplay + ["episode"]
}

@MainActor
@Observable
final class StoryModel {
    let app: AppModel

    // What the core said.
    private(set) var board: StoryBoardState?
    private(set) var series: StorySeriesDto?
    private(set) var viewState: StoryViewState?
    private(set) var display: StoryCardDisplayState?
    private(set) var loadFailure: CoreFailure?
    private(set) var loaded = false
    private(set) var episodeId: String?

    // The person's view.
    var tab: StoryTab = .board
    var selected: [String] = []
    var editingKey: String?
    var renamingId: String?
    var addingSequenceIn: String?
    var addingAct = false
    var drawer: StoryItemRef?
    var sheet: StorySheet?
    var filter = ""
    var notice: StoryNotice?
    /// Asks the board to move keyboard focus to this item key.
    var focusRequest: String?
    private(set) var dragging: StoryDragState?
    private(set) var hint: StoryDropHint?
    private(set) var actHint: String??
    /// Bumped when a view setting changed (these writes emit no data event), so screens reload.
    private(set) var viewRevision = 0

    /// The episode the person chose or opened from Episodes; wins over the saved choice.
    @ObservationIgnored private var explicitEpisode: String?
    @ObservationIgnored private var hasExplicitEpisode = false
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var noticeTask: Task<Void, Never>?
    @ObservationIgnored private var hoverZone: StoryDropZone?

    init(app: AppModel) {
        self.app = app
    }

    // MARK: Derived

    var episodes: [StoryEpisodeEntry] {
        guard let series else { return [] }
        return series.seasons.flatMap { s in s.episodes.map { StoryEpisodeEntry(episode: $0, seasonTitle: s.title) } }
            + series.unseasoned.map { StoryEpisodeEntry(episode: $0, seasonTitle: "") }
    }
    var isEpisodic: Bool { series?.isEpisodic ?? false }
    var showEpisodes: Bool { isEpisodic || !episodes.isEmpty }
    var collapsed: Set<String> { Set(board?.collapsedIds ?? []) }

    /// The tabs to show: Episodes only for episodic projects or once an episode exists.
    var tabs: [StoryTab] { StoryTab.allCases.filter { $0 != .episodes || showEpisodes } }

    var isEmpty: Bool {
        guard let board else { return true }
        return board.acts.isEmpty && board.parking.isEmpty && board.unassigned.isEmpty
    }

    var selectedRefs: [StoryItemRef] {
        selected.compactMap(StoryItemRef.init(key:)).filter { $0.kind != .sequence }
    }

    var toggles: StoryCardToggles { display?.current.toggles ?? StoryCardDisplayRules.minimal }

    // MARK: Loading

    /// Reads the series, the saved view choices, the board and the card display. A slower answer
    /// that arrives after a newer request is dropped.
    func reload() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        do {
            async let s = core.storySeries()
            async let v = core.storyViewState(StoryScopeArgs())
            let (series, viewState) = try await (s, v)
            guard mine == generation else { return }
            self.series = series
            self.viewState = viewState
            let scope = resolveEpisode()
            async let b = core.storyBoard(StoryScopeArgs(episodeId: scope))
            async let d = core.storyCardDisplay()
            let (board, display) = try await (b, d)
            guard mine == generation else { return }
            if scope != episodeId || !loaded {
                if loaded { selected = []; drawer = nil }
                episodeId = scope
            }
            self.board = board
            self.display = display
            loadFailure = nil
            loaded = true
            let present = Set(StoryLogic.containers(board).flatMap { $0.items.map(\.ref.key) })
            selected = selected.filter { present.contains($0) }
        } catch {
            guard mine == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    /// The episode scope: the one just opened or chosen, else the saved choice, else the first
    /// episode of an episodic project, else the series level.
    private func resolveEpisode() -> String? {
        let known = Set(episodes.map(\.id))
        if hasExplicitEpisode {
            if let e = explicitEpisode, known.contains(e) { return e }
            if explicitEpisode == nil { return nil }
        }
        if let vs = viewState, vs.episodeChosen, vs.episodeId == nil || known.contains(vs.episodeId ?? "") { return vs.episodeId }
        if isEpisodic, let first = episodes.first { return first.id }
        return nil
    }

    // MARK: Notices

    func say(_ text: String, undoable: Bool = false) {
        let n = StoryNotice(text: text, undoable: undoable)
        notice = n
        noticeTask?.cancel()
        noticeTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(8))
            guard !Task.isCancelled else { return }
            if self?.notice?.id == n.id { self?.notice = nil }
        }
    }

    func dismissNotice() {
        noticeTask?.cancel()
        notice = nil
    }

    func undoLast() async {
        dismissNotice()
        await app.undo()
    }

    func fail(_ title: String, _ error: Error) {
        app.show(title, error)
    }

    private func quote(_ s: String) -> String {
        "\u{201C}\(s.count > 60 ? String(s.prefix(57)) + "\u{2026}" : s)\u{201D}"
    }

    func describe(_ refs: [StoryItemRef]) -> String {
        guard let board else { return "\(refs.count) items" }
        if refs.count == 1 {
            let kind = refs[0].kind == .card ? "Scene Card" : refs[0].kind == .beat ? "Beat" : "Sequence"
            if let loc = StoryLogic.locate(board, refs[0]) { return "\(kind) \(quote(StoryLogic.label(loc.item)))" }
            return kind
        }
        return refs.allSatisfy { $0.kind == .card } ? "\(refs.count) Scene Cards" : "\(refs.count) items"
    }

    // MARK: Selection

    func select(_ keys: [String]) { selected = keys }
    func clearSelection() { selected = [] }
    func toggle(_ key: String) {
        if let i = selected.firstIndex(of: key) { selected.remove(at: i) } else { selected.append(key) }
    }

    /// Plain click selects one; Command or Shift click toggles (multi-select).
    func click(_ ref: StoryItemRef, extend: Bool) {
        if extend { toggle(ref.key) } else { select([ref.key]) }
    }

    /// The refs an action applies to: the whole selection when the item is part of it.
    func targets(for ref: StoryItemRef) -> [StoryItemRef] {
        if selected.count > 1, selected.contains(ref.key) { return selectedRefs }
        return [ref]
    }

    func openDrawer(_ ref: StoryItemRef?) { drawer = ref }

    // MARK: View settings (no data events, no undo)

    func setCollapsed(_ id: String, _ collapsed: Bool) async {
        guard let core = app.core else { return }
        do {
            _ = try await core.storySetCollapsed(StoryCollapseArgs(id: id, collapsed: collapsed))
            await reload()
        } catch { fail("Could not change the view", error) }
    }

    func chooseEpisode(_ id: String?) async {
        guard let core = app.core else { return }
        explicitEpisode = id
        hasExplicitEpisode = true
        do {
            viewState = try await core.storySetCurrentEpisode(StoryEpisodeChoiceArgs(episodeId: id))
            viewRevision += 1
            await reload()
        } catch { fail("Could not change the episode", error) }
    }

    /// "Open Story Board" in Episodes: scope the board to the episode and show it.
    func openEpisode(_ id: String) async {
        tab = .board
        await chooseEpisode(id)
    }

    func setCardDisplay(_ preset: StoryCardPreset, _ toggles: StoryCardToggles?) async {
        guard let core = app.core else { return }
        // Show the choice at once so quick ticks build on each other; the core's answer replaces it.
        if var before = display, let toggles { before.current = StoryCardDisplay(preset: preset, toggles: toggles); display = before }
        do {
            display = try await core.storySetCardDisplay(StorySetCardDisplayArgs(preset: preset, toggles: toggles))
        } catch {
            display = try? await core.storyCardDisplay()
            fail("Could not change Card Display", error)
        }
    }

    // MARK: Acts and sequences

    @discardableResult
    func createAct(_ title: String) async -> String? {
        let t = title.trimmingCharacters(in: .whitespacesAndNewlines)
        addingAct = false
        guard !t.isEmpty, let core = app.core else { return nil }
        do { return try await core.storyCreateAct(StoryCreateActArgs(episodeId: episodeId, title: t)).id }
        catch { fail("Could not add the act", error); return nil }
    }

    func renameAct(_ id: String, current: String, to title: String) async {
        renamingId = nil
        let t = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, t != current, let core = app.core else { return }
        do {
            _ = try await core.storyUpdateAct(StoryUpdateActArgs(id: id, title: t))
            say("Renamed Act to \u{201C}\(t)\u{201D}", undoable: true)
        } catch { fail("Could not rename the act", error) }
    }

    func setActNote(_ id: String, _ note: String) async {
        guard let core = app.core else { return }
        do { _ = try await core.storyUpdateAct(StoryUpdateActArgs(id: id, note: note)) }
        catch { fail("Could not save the note", error) }
    }

    func moveAct(_ id: String, before: String?) async {
        guard let core = app.core, let board else { return }
        let title = board.acts.first { $0.id == id }?.title ?? "Act"
        do {
            _ = try await core.storyMoveAct(StoryMoveActArgs(id: id, beforeId: before))
            say("Moved Act \u{201C}\(title)\u{201D}", undoable: true)
        } catch { fail("Could not move the act", error) }
    }

    /// Keyboard and menu alternative to dragging an act: -1 left, +1 right.
    func stepAct(_ id: String, direction: Int) async {
        guard let board else { return }
        let ids = board.acts.map(\.id)
        guard let i = ids.firstIndex(of: id) else { return }
        if direction < 0, i > 0 { await moveAct(id, before: ids[i - 1]) }
        if direction > 0, i < ids.count - 1 { await moveAct(id, before: i + 2 < ids.count ? ids[i + 2] : nil) }
    }

    func requestDeleteAct(_ id: String) async {
        guard let board, let act = board.acts.first(where: { $0.id == id }), let core = app.core else { return }
        if act.items.isEmpty {
            do {
                _ = try await core.storyDeleteAct(StoryDeleteActArgs(id: id))
                say("Deleted Act \u{201C}\(act.title)\u{201D}", undoable: true)
            } catch { fail("Could not delete the act", error) }
            return
        }
        sheet = .deleteAct(id)
    }

    @discardableResult
    func deleteAct(_ id: String, mode: StoryDeleteMode, moveToActId: String?) async -> Bool {
        guard let core = app.core, let act = board?.acts.first(where: { $0.id == id }) else { return false }
        do {
            _ = try await core.storyDeleteAct(StoryDeleteActArgs(id: id, mode: mode, moveToActId: mode == .moveContents ? moveToActId : nil))
            say("Deleted Act \u{201C}\(act.title)\u{201D}", undoable: true)
            return true
        } catch { fail("Could not delete the act", error); return false }
    }

    @discardableResult
    func createSequence(in actId: String, title: String) async -> String? {
        addingSequenceIn = nil
        let t = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, let core = app.core else { return nil }
        do { return try await core.storyCreateSequence(StoryCreateSequenceArgs(actId: actId, title: t)).id }
        catch { fail("Could not add the sequence", error); return nil }
    }

    func requestDeleteSequence(_ id: String) async {
        guard let board, let seq = StoryLogic.sequences(board).first(where: { $0.id == id }), let core = app.core else { return }
        if seq.items.isEmpty {
            do {
                _ = try await core.storyDeleteSequence(StoryDeleteSequenceArgs(id: id))
                say("Deleted Sequence \u{201C}\(seq.title)\u{201D}", undoable: true)
            } catch { fail("Could not delete the sequence", error) }
            return
        }
        sheet = .deleteSequence(id)
    }

    @discardableResult
    func deleteSequence(_ id: String, mode: StoryDeleteMode, moveTo: StoryContainerRef?) async -> Bool {
        guard let core = app.core, let board, let seq = StoryLogic.sequences(board).first(where: { $0.id == id }) else { return false }
        do {
            _ = try await core.storyDeleteSequence(StoryDeleteSequenceArgs(id: id, mode: mode, moveTo: mode == .moveContents ? moveTo : nil))
            say("Deleted Sequence \u{201C}\(seq.title)\u{201D}", undoable: true)
            return true
        } catch { fail("Could not delete the sequence", error); return false }
    }

    // MARK: Cards and beats

    /// Adds a blank card and opens it for typing (the short description is typed on the card).
    @discardableResult
    func newCard(in container: StoryContainerRef?, index: Int? = nil) async -> String? {
        guard let core = app.core else { return nil }
        do {
            let r = try await core.storyCreateCard(StoryCreateCardArgs(parent: container, episodeId: episodeId, shortDescription: "", index: index.map(Double.init)))
            let key = StoryItemRef(kind: .card, id: r.id).key
            select([key])
            editingKey = key
            return r.id
        } catch { fail("Could not add the scene card", error); return nil }
    }

    @discardableResult
    func newBeat(in container: StoryContainerRef?, index: Int? = nil) async -> String? {
        guard let core = app.core else { return nil }
        do {
            let r = try await core.storyCreateBeat(StoryCreateBeatArgs(parent: container ?? .parking, episodeId: episodeId, text: "", index: index.map(Double.init)))
            let key = StoryItemRef(kind: .beat, id: r.id).key
            select([key])
            editingKey = key
            return r.id
        } catch { fail("Could not add the beat", error); return nil }
    }

    /// "Add > Scene Card" from the toolbar: after the selected item, or at the end of the story.
    @discardableResult
    func addSceneFromToolbar() async -> String? {
        guard let board else { return nil }
        let t = StoryLogic.defaultInsert(board, selected: selected)
        return await newCard(in: t.container, index: t.index)
    }

    @discardableResult
    func addBeatFromToolbar() async -> String? {
        guard let board else { return nil }
        let t = StoryLogic.defaultInsert(board, selected: selected)
        return await newBeat(in: t.container ?? .parking, index: t.index)
    }

    /// Saves a title or text typed on the board for a card, beat or sequence. It does not wait for
    /// the board to show a card that was only just created: the text is never dropped.
    func commitText(_ ref: StoryItemRef, _ text: String) async {
        editingKey = nil
        renamingId = nil
        guard let core = app.core else { return }
        let located = board.flatMap { StoryLogic.locate($0, ref) }?.item
        let trimmedEnd = text.replacingOccurrences(of: "\\s+$", with: "", options: .regularExpression)
        do {
            switch ref.kind {
            case .card:
                var current: String?
                if case let .card(c)? = located { current = c.shortDescription }
                if trimmedEnd != current { _ = try await core.storyUpdateCard(StoryUpdateCardArgs(id: ref.id, shortDescription: trimmedEnd)) }
            case .beat:
                var current: String?
                if case let .beat(b)? = located { current = b.text }
                if trimmedEnd != current { _ = try await core.storyUpdateBeat(StoryUpdateBeatArgs(id: ref.id, text: trimmedEnd)) }
            case .sequence:
                let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
                var current: String?
                if case let .sequence(s)? = located { current = s.title }
                if !t.isEmpty, t != current { _ = try await core.storyUpdateSequence(StoryUpdateSequenceArgs(id: ref.id, title: t)) }
            }
        } catch { fail("Could not save the change", error) }
    }

    func cancelEditing() {
        editingKey = nil
        renamingId = nil
    }

    func convertBeat(_ id: String) async {
        guard let core = app.core else { return }
        do {
            let card = try await core.storyConvertBeat(StoryIdArgs(id: id))
            drawer = StoryItemRef(kind: .card, id: card.id)
            say("Converted Beat to a Scene Card. The beat stays, marked Converted.", undoable: true)
        } catch { fail("Could not convert the beat", error) }
    }

    func copyAsBeat(_ id: String) async {
        guard let core = app.core else { return }
        do {
            let beat = try await core.storyCardToBeat(StoryIdArgs(id: id))
            select([StoryItemRef(kind: .beat, id: beat.id).key])
            say("Copied the Scene Card as a Beat. The card is unchanged.", undoable: true)
        } catch { fail("Could not copy the card as a beat", error) }
    }

    // MARK: Moving, parking, duplicating, deleting

    @discardableResult
    func runMove(_ refs: [StoryItemRef], to hint: StoryDropHint) async -> Bool {
        guard let core = app.core, let board else { return false }
        let ordered = StoryLogic.boardOrder(board, refs)
        do {
            _ = try await core.storyMoveItems(StoryMoveArgs(items: ordered, target: hint.target, before: hint.before, episodeId: episodeId))
            let label = StoryLogic.containerLabel(board, hint.target)
            let where_: String
            switch hint.target.parentType {
            case .parking: where_ = "the Parking Lot"
            case .act: where_ = "Act \(quote(label))"
            case .sequence: where_ = "Sequence \(quote(label))"
            case .unassigned: where_ = quote(label)
            }
            say("Moved \(describe(ordered)) to \(where_)", undoable: true)
            return true
        } catch { fail("Could not move the item", error); return false }
    }

    /// Keyboard move among siblings: -1 up, +1 down.
    func step(_ ref: StoryItemRef, direction: Int) async {
        guard let board, let hint = StoryLogic.stepHint(board, ref, direction: direction) else { return }
        if await runMove([ref], to: hint) { focusRequest = ref.key }
    }

    /// Menu move to the previous or next act.
    func moveToAdjacentAct(_ ref: StoryItemRef, direction: Int) async {
        guard let board, let hint = StoryLogic.adjacentActHint(board, ref, direction: direction) else { return }
        if await runMove([ref], to: hint) { focusRequest = ref.key }
    }

    func canStep(_ ref: StoryItemRef, direction: Int) -> Bool {
        guard let board else { return false }
        return StoryLogic.stepHint(board, ref, direction: direction) != nil
    }

    func canMoveToAdjacentAct(_ ref: StoryItemRef, direction: Int) -> Bool {
        guard let board else { return false }
        return StoryLogic.adjacentActHint(board, ref, direction: direction) != nil
    }

    func park(_ refs: [StoryItemRef]) async {
        let r = refs.filter { $0.kind != .sequence }
        guard !r.isEmpty, let core = app.core else { return }
        let label = describe(r)
        do {
            _ = try await core.storyParkItems(StoryItemsArgs(items: r))
            say("Moved \(label) to the Parking Lot", undoable: true)
        } catch { fail("Could not send to the Parking Lot", error) }
    }

    func restoreToStory(_ refs: [StoryItemRef]) async {
        guard let core = app.core else { return }
        let label = describe(refs)
        do {
            let placed = try await core.storyRestoreFromParking(StoryItemsArgs(items: refs))
            if placed.count == 1 { say("Restored \(label) to \(placed[0].label)", undoable: true) }
            else if placed.count > 1 { say("Restored \(placed.count) items to the story", undoable: true) }
        } catch { fail("Could not restore to the story", error) }
    }

    func duplicate(_ refs: [StoryItemRef]) async {
        let r = refs.filter { $0.kind != .sequence }
        guard !r.isEmpty, let core = app.core else { return }
        let label = describe(r)
        do {
            let out = try await core.storyDuplicateItems(StoryItemsArgs(items: r))
            select(out.ids.enumerated().map { StoryItemRef(kind: r.indices.contains($0.offset) ? r[$0.offset].kind : .card, id: $0.element).key })
            say("Duplicated \(label)", undoable: true)
        } catch { fail("Could not duplicate", error) }
    }

    /// Delete cards and beats. A card linked to a screenplay scene asks first and says the scene stays.
    func requestDelete(_ refs: [StoryItemRef]) async {
        let r = refs.filter { $0.kind != .sequence }
        guard !r.isEmpty, let board else { return }
        let linked = r.contains { ref in
            if let loc = StoryLogic.locate(board, ref), case let .card(c) = loc.item { return c.screenplaySceneId != nil }
            return false
        }
        if linked { sheet = .deleteLinked(r); return }
        await deleteNow(r)
    }

    @discardableResult
    func deleteNow(_ refs: [StoryItemRef]) async -> Bool {
        guard let core = app.core else { return false }
        let label = describe(refs)
        do {
            _ = try await core.storyDeleteItems(StoryItemsArgs(items: refs))
            let gone = Set(refs.map(\.key))
            selected = selected.filter { !gone.contains($0) }
            if let d = drawer, refs.contains(where: { $0.id == d.id }) { drawer = nil }
            say("Deleted \(label)", undoable: true)
            return true
        } catch { fail("Could not delete", error); return false }
    }

    // MARK: Attachments

    func addAttachment(ownerType: String, ownerId: String, path: String) async {
        guard let core = app.core else { return }
        do { _ = try await core.storyAddAttachment(StoryAttachArgs(ownerType: ownerType, ownerId: ownerId, path: path)) }
        catch { fail("Could not attach the file", error) }
    }

    func removeAttachment(_ id: String) async {
        guard let core = app.core else { return }
        do { _ = try await core.storyRemoveAttachment(StoryIdArgs(id: id)) }
        catch { fail("Could not remove the attachment", error) }
    }

    // MARK: Drag and drop

    func beginDrag(_ ref: StoryItemRef) {
        guard let board else { return }
        var refs = [ref]
        if ref.kind != .sequence, selected.contains(ref.key), selected.count > 1 {
            refs = selectedRefs.filter { StoryLogic.locate(board, $0) != nil }
        }
        let label: String
        if refs.count > 1 {
            label = "\(refs.count) \(refs.allSatisfy { $0.kind == .card } ? "cards" : "items")"
        } else if let loc = StoryLogic.locate(board, ref) {
            label = StoryLogic.label(loc.item)
        } else { label = "" }
        dragging = StoryDragState(refs: refs, actId: nil, label: label)
        hint = nil
        actHint = nil
    }

    func beginActDrag(_ id: String) {
        guard let board else { return }
        dragging = StoryDragState(refs: [], actId: id, label: board.acts.first { $0.id == id }?.title ?? "Act")
        hint = nil
        actHint = nil
    }

    /// Can what is being dragged be dropped on this zone?
    func accepts(_ zone: StoryDropZone) -> Bool {
        guard let board, let d = dragging else { return false }
        if d.actId != nil {
            if case .act = zone { return true }
            return false
        }
        switch zone {
        case .act: return false
        case let .into(c, _): return StoryLogic.canDrop(kinds: d.kinds, into: c, moving: d.moving)
        case let .beside(r):
            if d.moving.contains(r.key) { return false }
            guard let loc = StoryLogic.locate(board, r) else { return false }
            if d.kinds.contains(.sequence) { return loc.container.parentType == .act }
            if r.kind == .sequence { return StoryLogic.canDrop(kinds: d.kinds, into: .sequence(r.id), moving: d.moving) }
            return StoryLogic.canDrop(kinds: d.kinds, into: loc.container, moving: d.moving)
        }
    }

    /// The pointer is over a zone; `firstHalf` is the top half (or left half for acts).
    func hover(_ zone: StoryDropZone, firstHalf: Bool) {
        guard let board, let d = dragging, accepts(zone) else { return }
        hoverZone = zone
        switch zone {
        case let .act(actId):
            let ids = board.acts.map(\.id).filter { $0 != d.actId }
            guard let idx = ids.firstIndex(of: actId) else { return }
            let before: String? = firstHalf ? actId : (idx + 1 < ids.count ? ids[idx + 1] : nil)
            if actHint != .some(before) { actHint = .some(before) }
        case let .into(c, atStart):
            setHint(StoryLogic.dropInto(board, c, atStart: atStart, moving: d.moving))
        case let .beside(r):
            if r.kind == .sequence, !d.kinds.contains(.sequence) {
                setHint(StoryLogic.dropInto(board, .sequence(r.id), atStart: true, moving: d.moving))
            } else {
                setHint(StoryLogic.dropBeside(board, over: r, placement: firstHalf ? .before : .after, moving: d.moving))
            }
        }
    }

    private func setHint(_ h: StoryDropHint?) {
        if hint != h { hint = h }
    }

    func clearHover() {
        hint = nil
        actHint = nil
        hoverZone = nil
    }

    /// The pointer left a zone. A newer zone that was already entered keeps its hint.
    func leave(_ zone: StoryDropZone) {
        if hoverZone == zone { clearHover() }
    }

    func cancelDrag() {
        dragging = nil
        clearHover()
    }

    /// Release: commit only over a valid place. An invalid release changes nothing.
    @discardableResult
    func commitDrop() async -> Bool {
        let d = dragging
        let h = hint
        let ah = actHint
        cancelDrag()
        guard let board, let d else { return false }
        if let actId = d.actId {
            guard let ah else { return false }
            let ids = board.acts.map(\.id)
            guard let i = ids.firstIndex(of: actId) else { return false }
            let next: String? = i + 1 < ids.count ? ids[i + 1] : nil
            if ah == actId || ah == next { return false }
            await moveAct(actId, before: ah)
            return true
        }
        guard let h, !StoryLogic.isNoop(board, refs: d.refs, hint: h) else { return false }
        return await runMove(d.refs, to: h)
    }
}

/// What a minimized card shows by default (the Minimal preset), used until the setting loads.
enum StoryCardDisplayRules {
    static let minimal = StoryCardToggles(
        sceneHeading: false, shortDescription: true, longDescriptionPreview: false, notes: false,
        attachments: false, characterCount: false, location: false, dayNight: false, comments: true,
        revisionChange: false, breakdownStatus: false, shotStatus: false)

    /// The twelve checkboxes, in the order the specification lists them.
    static var toggles: [(key: WritableKeyPath<StoryCardToggles, Bool>, label: String)] { [
        (\.sceneHeading, "Scene Heading"), (\.shortDescription, "Short Description"),
        (\.longDescriptionPreview, "Long Description Preview"), (\.notes, "Notes"),
        (\.attachments, "Attachments"), (\.characterCount, "Character Count"), (\.location, "Location"),
        (\.dayNight, "Day/Night"), (\.comments, "Comments"), (\.revisionChange, "Revision/Change"),
        (\.breakdownStatus, "Breakdown Status"), (\.shotStatus, "Shot/Storyboard Status"),
    ] }

    /// True when the toggles add nothing beyond the compact card, so it keeps its compact layout.
    static func isCompact(_ t: StoryCardToggles) -> Bool {
        toggles.allSatisfy { t[keyPath: $0.key] == minimal[keyPath: $0.key] }
    }

    private static func firstLine(_ text: String?, max: Int) -> String {
        let line = (text ?? "").split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.first { !$0.isEmpty } ?? ""
        return line.count > max ? String(line.prefix(max)).trimmingCharacters(in: .whitespaces) + "\u{2026}" : line
    }

    static func longPreview(_ c: StorySceneCardDto, _ t: StoryCardToggles) -> String {
        t.longDescriptionPreview ? firstLine(c.longDescription, max: 160) : ""
    }

    static func notePreview(_ c: StorySceneCardDto, _ t: StoryCardToggles) -> String {
        t.notes ? firstLine(c.notes, max: 90) : ""
    }

    /// Facts a non-compact card shows under its text, as words (never colour alone).
    static func facts(_ c: StorySceneCardDto, _ t: StoryCardToggles) -> [String] {
        var out: [String] = []
        let place = [t.location ? c.setName : nil, t.dayNight ? c.timeOfDay : nil].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{00B7} ")
        if !place.isEmpty { out.append(place) }
        if let s = c.sceneStatus {
            if t.revisionChange, s.revised { out.append("Revised") }
            if t.breakdownStatus {
                let n = Int(s.breakdownItems)
                out.append(s.breakdownComplete ? "Breakdown done" : n > 0 ? "Breakdown: \(StoryLogic.plural(n, "item"))" : "No breakdown yet")
            }
            if t.shotStatus {
                out.append("\(StoryLogic.plural(Int(s.shots), "shot")) \u{00B7} \(StoryLogic.plural(Int(s.storyboards), "storyboard"))")
            }
        }
        return out
    }
}

// MARK: Keyboard navigation

extension StoryModel {
    /// Arrow-key focus movement over the visible items. Up and Down walk the reading order; Left
    /// and Right jump to the neighbouring act column. Shift extends the selection.
    func navigate(from key: String, direction: StoryNavigation, extend: Bool) {
        guard let board else { return }
        let flat = StoryLogic.flattenVisible(board, collapsed: collapsed) { StoryLogic.matchesFilter($0, self.filter) }
        guard let idx = flat.firstIndex(where: { $0.ref.key == key }) else { return }
        var next = idx
        switch direction {
        case .down: next = min(flat.count - 1, idx + 1)
        case .up: next = max(0, idx - 1)
        case .left, .right:
            let dir = direction == .right ? 1 : -1
            let act = flat[idx].actId
            var j = idx
            while j >= 0, j < flat.count, flat[j].actId == act { j += dir }
            if j >= 0, j < flat.count {
                let target = flat[j].actId
                while dir < 0, j > 0, flat[j - 1].actId == target { j -= 1 }
                next = j
            }
        }
        let nk = flat[next].ref.key
        if extend {
            var s = selected
            for k in [key, nk] where !s.contains(k) { s.append(k) }
            select(s)
        }
        focusRequest = nk
    }
}

enum StoryNavigation { case up, down, left, right }

// MARK: Export

extension StoryModel {
    /// Writes the board as an outline file (PDF, text or CSV) to a path chosen in the Save dialog.
    @discardableResult
    func export(format: String, to path: String, scope: String = "board") async -> Bool {
        guard let core = app.core else { return false }
        do {
            let r = try await core.storyExportBoard(ExportStoryOutlineArgs(
                format: format, path: path, layout: "outline", scope: scope, episodeId: episodeId,
                itemIds: scope == "selected" ? selected.compactMap { StoryItemRef(key: $0)?.id } : nil))
            say("Exported \(r.fileName)")
            return true
        } catch { fail("Could not export the Story Board", error); return false }
    }
}

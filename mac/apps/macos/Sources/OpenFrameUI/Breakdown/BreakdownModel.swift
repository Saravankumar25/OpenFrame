// The Breakdown tab's model: the scenes of the Production Source, the open scene's breakdown, the
// filters, the script selection, the sheets, and every change to the breakdown. It mirrors
// apps/desktop/src/workspaces/breakdown (index.tsx, ElementsPanel.tsx, dialogs.tsx) and sends
// each change to a `breakdown.*` or `production.*` operation. The core decides; nothing here
// stores production data.

import Foundation
import Observation
import OpenFrameCore

/// Words picked in the script, or an empty form, for the Add Element sheet.
struct AddInitial: Equatable {
    var category: BreakdownCategory?
    var name: String?
    var span: BreakdownSpan?
}

enum BDSheet: Identifiable {
    case selectSource
    case updateSource(String)
    case add(AddInitial)
    case edit(BreakdownElementDto, chooser: Bool)
    case review
    case changes(String)
    case notes(BreakdownElementDto)

    var id: String {
        switch self {
        case .selectSource: "select-source"
        case let .updateSource(d): "update-source-\(d)"
        case let .add(i): "add-\(i.span?.elementId ?? "manual")-\(i.span?.start ?? -1)-\(i.category?.rawValue ?? "")"
        case let .edit(e, _): "edit-\(e.id)"
        case .review: "review"
        case let .changes(s): "changes-\(s)"
        case let .notes(e): "notes-\(e.id)"
        }
    }
}

enum BDMode { case scene, review, history }

@MainActor
@Observable
final class BreakdownModel {
    unowned let hub: BreakdownHub
    @ObservationIgnored private let defaults: UserDefaults

    // What the core said.
    private(set) var view: BreakdownScenesView?
    private(set) var loadFailure: CoreFailure?
    private(set) var loaded = false
    private(set) var detail: BreakdownSceneDetail?
    private(set) var detailFailure: CoreFailure?
    private(set) var reviewRows: [BreakdownSceneRow] = []
    private(set) var reuseOffers: [BreakdownReuseItem] = []
    private(set) var sourceOptions: ProductionSourceOptions?
    private(set) var sourceOptionsLoading = false
    private(set) var updatePreview: ProductionUpdatePreview?
    private(set) var sceneChanges: BreakdownSceneChanges?

    // The person's view.
    private(set) var saved: SavedFilter
    private(set) var catalogItemId: String?
    var picked: String?
    var mode: BDMode = .scene
    var selection: TextSelection?
    private(set) var jump: BreakdownSourceAnchor?
    var sheet: BDSheet?
    var dismissedNewer: String?
    var selectedElementId: String?
    var collapsed: Set<BreakdownCategory> = []
    /// Suggestions left out of "Accept Selected" and "Reject Selected" in the review sheet.
    var reviewDeselected: Set<String> = []
    private(set) var suggesting = false
    private(set) var busy = false

    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var detailGen = 0

    init(hub: BreakdownHub, defaults: UserDefaults = .standard) {
        self.hub = hub
        self.defaults = defaults
        self.saved = SavedFilter.read(defaults)
    }

    // MARK: Derived

    var source: ProductionSourceInfo? { view?.source }
    var rows: [BreakdownSceneRow] { view?.scenes ?? [] }
    var total: Int { Int(view?.total ?? 0) }
    var counts: BreakdownFilterCounts? { view?.counts }
    /// The scene the panel shows: the one the person picked, otherwise the first in the list.
    var sceneId: String? { picked ?? rows.first?.sceneId }
    var number: String { detail?.number ?? "" }
    var canEdit: Bool { hub.canEdit }
    var needsReviewCount: Int { Int(counts?.needsReview ?? 0) }
    var historicalCount: Int {
        (view?.historical ?? []).reduce(0) { $0 + $1.elements.filter { !$0.archived }.count }
    }
    var newerDraft: ProductionDraftInfo? {
        guard let n = source?.newer, n.id != dismissedNewer else { return nil }
        return n
    }
    var pendingSuggestions: [BreakdownElementDto] { detail?.groups.flatMap(\.suggestions) ?? [] }
    var selectedSuggestions: [BreakdownElementDto] { pendingSuggestions.filter { !reviewDeselected.contains($0.id) } }
    var safeSuggestions: [BreakdownElementDto] { pendingSuggestions.filter { $0.matchKind != "ambiguous" } }

    // MARK: Loading

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        do {
            let v = try await core.breakdownScenes(BreakdownScenesArgs(filter: saved.filter, category: saved.category, catalogItemId: catalogItemId))
            guard gen == generation else { return }
            view = v
            loadFailure = nil
            loaded = true
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
            return
        }
        await reloadScene()
    }

    /// Reloads the open scene and what hangs on it (after the person picked another scene or mode).
    func reloadScene() async {
        guard let core = hub.core, view?.source != nil else { return }
        await reloadDetail(core: core)
        if mode == .review {
            let g = detailGen
            let r = try? await core.breakdownScenes(BreakdownScenesArgs(filter: .needsReview, category: nil, catalogItemId: nil))
            if g == detailGen { reviewRows = r?.scenes ?? [] }
        }
    }

    private func reloadDetail(core: Core) async {
        detailGen += 1
        let gen = detailGen
        guard view?.source != nil, let id = sceneId else {
            detail = nil
            detailFailure = nil
            reuseOffers = []
            return
        }
        do {
            var d = try await core.breakdownScene(BreakdownSceneArgs(sceneId: id))
            // A scene of an older draft after a source update: the same scene is found by lineage.
            if !d.inSource, let row = rows.first(where: { $0.lineageId == d.lineageId }), row.sceneId != id {
                picked = row.sceneId
                d = try await core.breakdownScene(BreakdownSceneArgs(sceneId: row.sceneId))
            }
            guard gen == detailGen else { return }
            if detail?.sceneId != d.sceneId { selection = nil }
            detail = d
            detailFailure = nil
            if d.inSource, hub.canEdit {
                let offers = try await core.breakdownQuickReuse(BreakdownSceneArgs(sceneId: d.sceneId))
                if gen == detailGen { reuseOffers = offers }
            } else {
                reuseOffers = []
            }
        } catch {
            guard gen == detailGen else { return }
            detail = nil
            detailFailure = CoreFailure(asCore: error)
        }
    }

    // MARK: Choosing what to see

    func selectScene(_ id: String) {
        picked = id
        mode = .scene
        jump = nil
        selection = nil
    }

    func focusScene(_ id: String) {
        selectScene(id)
    }

    func focusCatalogItem(_ id: String) {
        catalogItemId = id
        mode = .scene
        picked = nil
    }

    func clearCatalogFilter() {
        catalogItemId = nil
    }

    func setFilter(_ f: BreakdownSceneFilter) {
        saved.filter = f
        saved.write(defaults)
    }

    func setCategory(_ c: BreakdownCategory?) {
        saved.category = c
        saved.write(defaults)
    }

    func toggleCollapsed(_ c: BreakdownCategory) {
        if collapsed.contains(c) { collapsed.remove(c) } else { collapsed.insert(c) }
    }

    // MARK: Production Source

    func loadSourceOptions() async {
        guard let core = hub.core else { return }
        sourceOptionsLoading = true
        defer { sourceOptionsLoading = false }
        do {
            sourceOptions = try await core.productionDrafts()
        } catch {
            hub.fail("Could not list the screenplay drafts", error)
        }
    }

    @discardableResult
    func setSource(draftId: String, reason: String?) async -> Bool {
        guard let core = hub.core else { return false }
        busy = true
        defer { busy = false }
        do {
            let r = reason?.trimmingCharacters(in: .whitespacesAndNewlines)
            let s = try await core.productionSetSource(ProductionSetSourceArgs(draftId: draftId, reason: (r?.isEmpty ?? true) ? nil : r))
            hub.say("Production Source set to \(BDText.quoted(s.draft.name)). Your screenplay is not changed.", undoable: true)
            sheet = nil
            picked = nil
            return true
        } catch {
            hub.fail("Could not set the Production Source", error)
            return false
        }
    }

    func loadUpdatePreview(draftId: String) async {
        guard let core = hub.core else { return }
        updatePreview = nil
        do {
            updatePreview = try await core.productionPreviewUpdate(ProductionDraftArgs(draftId: draftId))
        } catch {
            hub.fail("Could not compare the drafts", error)
        }
    }

    @discardableResult
    func applyUpdate(draftId: String) async -> Bool {
        guard let core = hub.core else { return false }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.productionApplyUpdate(ProductionSetSourceArgs(draftId: draftId, reason: nil))
            hub.say("Production now uses \(BDText.quoted(r.source.draft.name)). \(BDText.plural(r.addedScenes.asInt, "new scene")) need\(r.addedScenes == 1 ? "s" : "") breakdown; nothing was deleted.", undoable: true)
            sheet = nil
            return true
        } catch {
            hub.fail("Could not update the Production Source", error)
            return false
        }
    }

    // MARK: Suggestions

    func suggest() async {
        guard let core = hub.core, let d = detail else { return }
        suggesting = true
        defer { suggesting = false }
        do {
            let r = try await core.breakdownSuggest(BreakdownSceneArgs(sceneId: d.sceneId))
            if r.added > 0 {
                hub.say("\(BDText.plural(r.added.asInt, "suggestion")) to review. Nothing is added to production until you accept.")
            } else if r.pending > 0 {
                hub.say("No new suggestions. Review the pending ones.")
            } else {
                hub.say("No likely elements found. You can still tag text or add elements manually.")
            }
        } catch {
            hub.fail("Could not suggest elements", error)
        }
    }

    /// Accepts one suggestion. Several similar catalog items open the chooser instead.
    func accept(_ s: BreakdownElementDto) async {
        if s.matchKind == "ambiguous" {
            sheet = .edit(s, chooser: true)
            return
        }
        guard let core = hub.core else { return }
        do {
            let el = try await core.breakdownAccept(BreakdownAcceptArgs(id: s.id, catalog: BreakdownCatalogChoice(mode: .auto, catalogItemId: nil)))
            hub.say("Accepted \(BDText.quoted(el.name))", undoable: true)
        } catch {
            if CoreFailure(asCore: error).code == "validation.ambiguous_match" {
                sheet = .edit(s, chooser: true)
            } else {
                hub.fail("Could not accept the suggestion", error)
            }
        }
    }

    @discardableResult
    func acceptEdited(_ s: BreakdownElementDto, category: BreakdownCategory, name: String, choice: BreakdownCatalogChoice) async -> Bool {
        guard let core = hub.core else { return false }
        busy = true
        defer { busy = false }
        do {
            let el = try await core.breakdownAccept(BreakdownAcceptArgs(id: s.id, category: category.rawValue, name: name.trimmingCharacters(in: .whitespacesAndNewlines), catalog: choice))
            hub.say("Accepted \(BDText.quoted(el.name))", undoable: true)
            sheet = nil
            return true
        } catch {
            hub.fail("Could not accept the suggestion", error)
            return false
        }
    }

    /// One undo step for the whole batch. Returns how many were accepted.
    @discardableResult
    func acceptMany(_ list: [BreakdownElementDto]) async -> BreakdownAcceptManyResult? {
        guard let core = hub.core, !list.isEmpty else { return nil }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.breakdownAcceptMany(BreakdownIdsArgs(ids: list.map(\.id)))
            let extra = r.needsChoice.isEmpty ? "" : " \(r.needsChoice.count) need\(r.needsChoice.count == 1 ? "s" : "") a choice of catalog item."
            hub.say("Accepted \(BDText.plural(r.accepted.asInt, "suggestion")).\(extra)", undoable: true)
            if r.needsChoice.isEmpty, list.count == pendingSuggestions.count, case .review? = sheet { sheet = nil }
            return r
        } catch {
            hub.fail("Could not accept the suggestions", error)
            return nil
        }
    }

    func reject(_ ids: [String]) async {
        guard let core = hub.core, !ids.isEmpty else { return }
        do {
            _ = try await core.breakdownReject(BreakdownIdsArgs(ids: ids))
            hub.say(ids.count == 1
                    ? "Suggestion rejected. It won't be suggested again for this scene."
                    : "Rejected \(ids.count) suggestions. They won't be suggested again for this scene.", undoable: true)
        } catch {
            hub.fail("Could not reject the suggestions", error)
        }
    }

    // MARK: Elements

    @discardableResult
    func addElement(category: BreakdownCategory, name: String, notes: String?, span: BreakdownSpan?, choice: BreakdownCatalogChoice) async -> Bool {
        guard let core = hub.core, let d = detail else { return false }
        busy = true
        defer { busy = false }
        do {
            let n = notes?.trimmingCharacters(in: .whitespacesAndNewlines)
            let el = try await core.breakdownAddElement(BreakdownAddArgs(
                sceneId: d.sceneId, category: category.rawValue, name: name.trimmingCharacters(in: .whitespacesAndNewlines),
                notes: (n?.isEmpty ?? true) ? nil : n, span: span, catalog: choice))
            hub.say("Added \(BDText.quoted(el.name)) to Scene \(d.number ?? "")".trimmingCharacters(in: .whitespaces), undoable: true)
            sheet = nil
            selection = nil
            return true
        } catch {
            hub.fail("Could not add the element", error)
            return false
        }
    }

    func remove(_ e: BreakdownElementDto) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.breakdownRemoveElement(BreakdownIdArgs(id: e.id))
            hub.say("Removed \(BDText.quoted(e.name)) from Scene \(number). The catalog item is kept.", undoable: true)
            if selectedElementId == e.id { selectedElementId = nil }
        } catch {
            hub.fail("Could not remove the element", error)
        }
    }

    @discardableResult
    func saveNotes(_ e: BreakdownElementDto, notes: String) async -> Bool {
        guard let core = hub.core else { return false }
        do {
            _ = try await core.breakdownUpdateElement(BreakdownUpdateArgs(id: e.id, notes: notes))
            sheet = nil
            return true
        } catch {
            hub.fail("Could not save the notes", error)
            return false
        }
    }

    func setComplete(_ complete: Bool) async {
        guard let core = hub.core, let d = detail else { return }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.breakdownSetComplete(BreakdownCompleteArgs(sceneId: d.sceneId, complete: complete))
            hub.say(complete ? "Scene \(number) marked complete" : "Scene \(number) reopened", undoable: true)
        } catch {
            hub.fail("Could not change the scene", error)
        }
    }

    func reuse(_ item: BreakdownReuseItem) async {
        guard let core = hub.core, let d = detail else { return }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.breakdownReuse(BreakdownReuseArgs(sceneIds: [d.sceneId], catalogItemIds: [item.catalogItemId]))
            hub.say(r.count > 0 ? "Reused \(BDText.quoted(item.name)) in Scene \(number)" : "Already in this scene.", undoable: r.count > 0)
        } catch {
            hub.fail("Could not reuse the item", error)
        }
    }

    /// "Show in script": finds the words that made the element and highlights them.
    func showSource(_ e: BreakdownElementDto) async {
        guard let core = hub.core else { return }
        do {
            let a = try await core.breakdownSourceAnchor(BreakdownIdArgs(id: e.id))
            switch a.status {
            case .missing:
                hub.say("\(BDText.quoted(a.phrase ?? e.name)) is no longer in the scene text. The element is kept; review it if the scene changed.")
            case .none_:
                hub.say("\(BDText.quoted(e.name)) was added without tagging script text.")
            case .moved:
                jump = a
                hub.say("The script changed. Showing where \(BDText.quoted(a.phrase ?? e.name)) is now.")
            case .exact:
                jump = a
            }
        } catch {
            hub.fail("Could not find the words in the script", error)
        }
    }

    func setArchived(_ e: BreakdownElementDto, _ archived: Bool) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.breakdownSetArchived(BreakdownArchiveArgs(id: e.id, archived: archived))
            hub.say(archived ? "Archived \(BDText.quoted(e.name))" : "Unarchived \(BDText.quoted(e.name))", undoable: true)
        } catch {
            hub.fail("Could not change the element", error)
        }
    }

    // MARK: Script changed

    func loadChanges(sceneId: String) async {
        guard let core = hub.core else { return }
        sceneChanges = nil
        do {
            sceneChanges = try await core.breakdownSceneChanges(BreakdownSceneArgs(sceneId: sceneId))
        } catch {
            hub.fail("Could not compare the scene", error)
        }
    }

    func keepProductionData(sceneId: String) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.breakdownMarkReviewed(BreakdownSceneArgs(sceneId: sceneId))
            hub.say("Kept the production data. The scene is marked as reviewed.", undoable: true)
            sheet = nil
        } catch {
            hub.fail("Could not mark the scene as reviewed", error)
        }
    }

    func applySuggestedUpdate(sceneId: String) async {
        guard let core = hub.core else { return }
        do {
            let r = try await core.breakdownApplySuggestedUpdate(BreakdownSceneArgs(sceneId: sceneId))
            hub.say(r.added > 0
                    ? "\(BDText.plural(r.added.asInt, "new suggestion")) added for review. Existing elements were kept."
                    : "Reviewed. Existing elements were kept.", undoable: true)
            sheet = nil
        } catch {
            hub.fail("Could not apply the update", error)
        }
    }

    // MARK: Review sheet

    func openReview() {
        reviewDeselected = []
        sheet = .review
    }

    func toggleReview(_ id: String) {
        if reviewDeselected.contains(id) { reviewDeselected.remove(id) } else { reviewDeselected.insert(id) }
    }

    func acceptSelected() async { await acceptMany(selectedSuggestions) }
    func acceptAllSafe() async { await acceptMany(safeSuggestions) }
    func rejectSelected() async { await reject(selectedSuggestions.map(\.id)) }

    // MARK: Tagging from the script

    /// "Tag Selection": the selected words open the Add Element sheet with the name filled in.
    func tagSelection(category: BreakdownCategory? = nil) {
        guard let s = selection, detail?.inSource == true else { return }
        sheet = .add(AddInitial(category: category, name: Highlight.suggestName(s.text),
                                span: BreakdownSpan(elementId: s.elementId, start: Double(s.start), end: Double(s.end))))
    }

    func addByHand() {
        sheet = .add(AddInitial())
    }
}

// MARK: Forms of the sheets

/// The Add Element sheet (also "Tag as…" on selected words).
@MainActor
@Observable
final class AddElementForm {
    let model: BreakdownModel
    let initial: AddInitial
    private(set) var category: BreakdownCategory
    private(set) var name: String
    var notes = ""
    let finder: CatalogMatchFinder

    init(model: BreakdownModel, initial: AddInitial) {
        self.model = model
        self.initial = initial
        category = initial.category ?? .props
        name = initial.name ?? ""
        finder = CatalogMatchFinder(core: { [weak model] in model?.hub.core })
        finder.update(category: category, name: name)
    }

    func setCategory(_ c: BreakdownCategory) {
        category = c
        finder.update(category: c, name: name)
    }

    func setName(_ n: String) {
        name = n
        finder.update(category: category, name: n)
    }

    var canSubmit: Bool { !name.trimmingCharacters(in: .whitespaces).isEmpty && finder.effective != nil && !model.busy }

    @discardableResult
    func submit() async -> Bool {
        guard canSubmit, let choice = finder.effective else { return false }
        return await model.addElement(category: category, name: name, notes: notes, span: initial.span, choice: choice)
    }
}

/// Edit a suggestion before accepting it, or choose among similar catalog items.
@MainActor
@Observable
final class EditSuggestionForm {
    let model: BreakdownModel
    let element: BreakdownElementDto
    let chooser: Bool
    private(set) var category: BreakdownCategory
    private(set) var name: String
    let finder: CatalogMatchFinder

    init(model: BreakdownModel, element: BreakdownElementDto, chooser: Bool) {
        self.model = model
        self.element = element
        self.chooser = chooser
        category = element.category
        name = element.displayName
        finder = CatalogMatchFinder(core: { [weak model] in model?.hub.core })
        finder.update(category: category, name: name)
    }

    func setCategory(_ c: BreakdownCategory) {
        category = c
        finder.update(category: c, name: name)
    }

    func setName(_ n: String) {
        name = n
        finder.update(category: category, name: n)
    }

    var canSubmit: Bool { !name.trimmingCharacters(in: .whitespaces).isEmpty && finder.effective != nil && !model.busy }

    @discardableResult
    func submit() async -> Bool {
        guard canSubmit, let choice = finder.effective else { return false }
        return await model.acceptEdited(element, category: category, name: name, choice: choice)
    }
}

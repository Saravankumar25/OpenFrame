// The Storyboards tab's model: the scene or storyboard being shown, its ordered panels and every
// change to them. Panel numbers come from order; a linked shot shows its generated label (12A)
// and moving a panel re-sequences the linked shots in the core. A scene's first panel creates its
// storyboard. Mirrors apps/desktop/src/workspaces/production/tabs/storyboards.tsx.

import AppKit
import Foundation
import Observation
import OpenFrameCore

enum PanelVisual: String {
    case image, sketch, placeholder
}

enum StoryboardSheet: Identifiable {
    case newBoard
    case rename(StoryboardSummary)
    case linkScene(StoryboardSummary)
    case deleteBoard(StoryboardSummary)
    case movePanel(PanelDto)
    /// A sketch for a new panel (`nil`) or to replace the picture of a panel.
    case sketch(panelId: String?)
    case export

    var id: String {
        switch self {
        case .newBoard: "new-board"
        case let .rename(b): "rename-\(b.id)"
        case let .linkScene(b): "link-scene-\(b.id)"
        case let .deleteBoard(b): "delete-\(b.id)"
        case let .movePanel(p): "move-\(p.id)"
        case let .sketch(id): "sketch-\(id ?? "new")"
        case .export: "export"
        }
    }
}

@MainActor
@Observable
final class StoryboardsModel {
    unowned let hub: VisualHub

    private(set) var boards: [StoryboardSummary] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    private(set) var detail: StoryboardDto?
    /// The shots of the open scene, for "Link Shot".
    private(set) var sceneShots: [ShotDto] = []
    private(set) var busy = false

    // The person's view.
    var boardKey: String?
    var sceneBoard: [String: String] = [:]
    var selectedPanelId: String?
    var sheet: StoryboardSheet?
    /// The order of the panels while a reorder is on its way to the core.
    private(set) var order: [String]?

    @ObservationIgnored private var generation = 0

    init(hub: VisualHub) { self.hub = hub }

    // MARK: Derived

    var selection: StoryboardSelection {
        StoryboardSelection.resolve(wanted: boardKey, sceneBoard: sceneBoard, scenes: hub.scenes, boards: boards)
    }
    var board: StoryboardDto? { detail?.board.id == selection.boardId ? detail : nil }
    var panels: [PanelDto] {
        guard let b = board else { return [] }
        guard let order else { return b.panels }
        return order.compactMap { id in b.panels.first { $0.id == id } }
    }
    var panel: PanelDto? { panels.first { $0.id == selectedPanelId } }
    var canAdd: Bool { selection.boardId != nil || selection.scene != nil }
    var sceneBoards: [StoryboardSummary] {
        guard let s = selection.scene else { return [] }
        return boards.filter { $0.sceneLineageId == s.lineageId }
    }
    /// Scene boards whose scene left the script.
    var removedBoards: [StoryboardSummary] { boards.filter(\.sceneRemoved) }
    var otherBoards: [StoryboardSummary] { boards.filter { $0.sceneLineageId == nil } }
    /// The scene a panel's shots belong to (nil for a standalone board or a removed scene).
    var activeLineage: String? {
        guard let b = board?.board, let l = b.sceneLineageId, !b.sceneRemoved else { return nil }
        return l
    }

    // MARK: Loading

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        do {
            let list = try await core.storyboardList()
            guard gen == generation else { return }
            boards = list
            loadFailure = nil
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
            return
        }
        loaded = true
        await reloadBoard(gen: gen)
    }

    private func reloadBoard(gen: Int) async {
        guard let core = hub.core, let id = selection.boardId else {
            detail = nil
            order = nil
            sceneShots = []
            return
        }
        do {
            let d = try await core.storyboardGet(StoryboardIdArgs(id: id))
            guard gen == generation else { return }
            detail = d
            order = nil
            if let p = selectedPanelId, !d.panels.contains(where: { $0.id == p }) { selectedPanelId = nil }
        } catch {
            guard gen == generation else { return }
            order = nil
            hub.app.show("Could not open the storyboard", error)
        }
        await reloadSceneShots()
    }

    func reloadSceneShots() async {
        guard let core = hub.core, let lineage = activeLineage else {
            sceneShots = []
            return
        }
        sceneShots = (try? await core.shotList(ShotListArgs(sceneLineageId: lineage))) ?? []
    }

    func choose(key: String) async {
        boardKey = key
        selectedPanelId = nil
        generation += 1
        await reloadBoard(gen: generation)
    }

    func chooseSceneBoard(_ id: String) async {
        guard let s = selection.scene else { return }
        sceneBoard[s.lineageId] = id
        selectedPanelId = nil
        generation += 1
        await reloadBoard(gen: generation)
    }

    /// A shot or search result opens one panel of one storyboard.
    func focus(storyboardId: String, panelId: String?) async {
        if let b = boards.first(where: { $0.id == storyboardId }) {
            if let l = b.sceneLineageId, !b.sceneRemoved {
                boardKey = "scene:\(l)"
                sceneBoard[l] = b.id
            } else {
                boardKey = "board:\(b.id)"
            }
        } else {
            boardKey = "board:\(storyboardId)"
        }
        selectedPanelId = panelId
        await reload()
    }

    // MARK: Boards

    @discardableResult
    func createBoard(name: String?, sceneId: String?) async -> StoryboardSummary? {
        let trimmed = name?.trimmingCharacters(in: .whitespaces)
        guard sceneId != nil || !(trimmed ?? "").isEmpty else { return nil }
        guard let b = await hub.perform("Could not create the storyboard", { core in
            try await core.storyboardCreate(CreateStoryboardArgs(name: trimmed.flatMap { $0.isEmpty ? nil : $0 }, sceneId: sceneId))
        }) else { return nil }
        ToastCenter.shared.undoable("Created storyboard \u{201C}\(b.name)\u{201D}", app: hub.app)
        if let l = b.sceneLineageId {
            boardKey = "scene:\(l)"
            sceneBoard[l] = b.id
        } else {
            boardKey = "board:\(b.id)"
        }
        await reload()
        return b
    }

    func rename(_ board: StoryboardSummary, to name: String) async -> Bool {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        let r = await hub.perform("Could not rename the storyboard", done: "Renamed storyboard") {
            try await $0.storyboardRename(RenameStoryboardArgs(id: board.id, name: name, expectedRev: board.rev))
        }
        return r != nil
    }

    func setScene(_ board: StoryboardSummary, sceneId: String?) async -> Bool {
        guard let b = await hub.perform("Could not change the scene of the storyboard", { core in
            try await core.storyboardSetScene(SetStoryboardSceneArgs(id: board.id, sceneId: sceneId))
        }) else { return false }
        ToastCenter.shared.undoable(b.sceneLineageId != nil ? "Linked storyboard to Scene \(b.sceneNumber ?? "")" : "Storyboard is now standalone", app: hub.app)
        if let l = b.sceneLineageId {
            boardKey = "scene:\(l)"
            sceneBoard[l] = b.id
        } else {
            boardKey = "board:\(b.id)"
        }
        return true
    }

    func deleteBoard(_ board: StoryboardSummary) async -> Bool {
        let r = await hub.perform("Could not delete the storyboard", done: "Deleted storyboard \u{201C}\(board.name)\u{201D}") {
            try await $0.storyboardDelete(StoryboardIdArgs(id: board.id))
        }
        if r != nil { boardKey = nil }
        return r != nil
    }

    // MARK: Panels

    /// The board new panels go into; a scene's first panel creates its storyboard.
    private func ensureBoard() async -> String? {
        if let id = selection.boardId { return id }
        if let scene = selection.scene {
            guard let b = await hub.perform("Could not create the storyboard", { core in
                try await core.storyboardCreate(CreateStoryboardArgs(sceneId: scene.sceneId))
            }) else { return nil }
            boardKey = "scene:\(scene.lineageId)"
            sceneBoard[scene.lineageId] = b.id
            await reload()
            return b.id
        }
        sheet = .newBoard
        return nil
    }

    /// Add a panel: an empty one, an imported picture (a file path) or a sketch (PNG bytes).
    @discardableResult
    func addPanel(_ kind: PanelVisual, path: String? = nil, png: Data? = nil, description: String? = nil, at index: Int? = nil) async -> PanelDto? {
        guard let boardId = await ensureBoard() else { return nil }
        busy = true
        defer { busy = false }
        let b64 = png?.base64EncodedString()
        guard let p = await hub.perform("Could not add the panel", { core in
            try await core.storyboardAddPanel(AddPanelArgs(
                storyboardId: boardId, visual: kind.rawValue, path: path, dataBase64: b64, description: description, index: index.map(Double.init)))
        }) else { return nil }
        ToastCenter.shared.undoable("Added panel \(Int(p.number))", app: hub.app)
        sheet = nil
        await reload()
        return p
    }

    /// "Import Image" on the Add Panel menu: choose a file, then add it.
    func importImagePanel() async {
        guard canAdd, let path = VisualPanels.chooseImages(multiple: false, message: "Choose an image for the panel").first else { return }
        await addPanel(.image, path: path)
    }

    func addPasted(image data: Data) async {
        guard canAdd else { return }
        await addPanel(.image, png: data)
    }

    /// Drag a panel to a new place (index is the target place, 0 based). The grid shows the new
    /// order at once and the core re-numbers the linked shots.
    func reorder(_ id: String, to index: Int) async {
        let ids = panels.map(\.id)
        guard let from = ids.firstIndex(of: id), from != index, index >= 0, index < ids.count else { return }
        order = VisualText.move(ids, from: from, to: index)
        let lineage = board?.board.sceneLineageId
        let ok = await hub.perform("Could not move the panel", done: lineage != nil ? "Moved panel \u{00B7} shot numbers follow the new order" : "Moved panel") {
            try await $0.storyboardReorderPanel(ReorderPanelArgs(id: id, index: Double(index)))
        }
        if ok == nil { order = nil }
    }

    /// The keyboard way to reorder: one place earlier or later.
    func nudge(_ id: String, by delta: Int) async {
        guard let i = panels.firstIndex(where: { $0.id == id }) else { return }
        await reorder(id, to: i + delta)
    }

    func deletePanels(_ ids: [String]) async {
        guard !ids.isEmpty else { return }
        let r = await hub.perform("Could not delete the panel", done: ids.count == 1 ? "Deleted panel" : "Deleted \(ids.count) panels") {
            try await $0.storyboardDeletePanels(DeletePanelsArgs(ids: ids))
        }
        if r != nil, let s = selectedPanelId, ids.contains(s) { selectedPanelId = nil }
    }

    func movePanel(_ panel: PanelDto, to storyboardId: String) async -> Bool {
        let r = await hub.perform("Could not move the panel", done: "Moved panel to another storyboard") {
            try await $0.storyboardMovePanel(MovePanelArgs(id: panel.id, storyboardId: storyboardId))
        }
        return r != nil
    }

    func update(_ panel: PanelDto, description: String? = nil, framing: String? = nil, movement: String? = nil, angle: String? = nil,
                soundNote: String? = nil, durationMs: Double? = nil, note: String? = nil) async {
        await hub.perform("Could not save the panel") {
            try await $0.storyboardUpdatePanel(UpdatePanelArgs(
                id: panel.id, description: description, framing: framing, movement: movement, angle: angle,
                soundNote: soundNote, durationMs: durationMs, note: note))
        }
    }

    /// Change the picture of a panel. The panel stays intact when it fails (another way can be tried).
    func setVisual(_ panel: PanelDto, _ kind: PanelVisual, path: String? = nil, png: Data? = nil) async -> Bool {
        busy = true
        defer { busy = false }
        let b64 = png?.base64EncodedString()
        let r = await hub.perform("Could not change the panel image", done: kind == .placeholder ? "Cleared panel image" : "Changed panel image") {
            try await $0.storyboardSetPanelVisual(SetPanelVisualArgs(id: panel.id, visual: kind.rawValue, path: path, dataBase64: b64))
        }
        if r != nil { sheet = nil }
        return r != nil
    }

    func chooseVisual(for panel: PanelDto) async {
        guard let path = VisualPanels.chooseImages(multiple: false, message: "Choose an image for the panel").first else { return }
        _ = await setVisual(panel, .image, path: path)
    }

    // MARK: Shots

    func linkShot(_ panel: PanelDto, shotId: String?) async {
        await hub.perform("Could not link the panel", done: shotId != nil ? "Linked panel to shot" : "Unlinked panel from shot") {
            try await $0.storyboardLinkShot(LinkShotArgs(panelId: panel.id, shotId: shotId))
        }
    }

    func createShot(from panel: PanelDto, sceneId: String?) async {
        let scene = activeLineage != nil ? nil : sceneId
        guard let s = await hub.perform("Could not create the shot", { core in
            try await core.storyboardCreateShotFromPanel(ShotFromPanelArgs(panelId: panel.id, sceneId: scene))
        }) else { return }
        ToastCenter.shared.undoable("Created shot \(s.label)", app: hub.app)
    }

    // MARK: Export

    func export(boardIds: [String]?, path: String, includeNotes: Bool, columns: Int = 3) async -> ExportResult? {
        let r = await hub.perform("Could not export the storyboard") {
            try await $0.storyboardExportSheet(ExportStoryboardSheetArgs(
                storyboardIds: boardIds, format: "pdf", path: path, includeNotes: includeNotes, columns: Double(columns)))
        }
        if let r { hub.info("Exported \(r.fileName)") }
        return r
    }
}

// The Shot Lists tab's model: the scene being planned (or all scenes), its ordered shots, the
// selection and every change to a shot. Labels (12A, 12B) are generated from order by the core;
// they are never typed or renumbered by hand. Only a description is required. Mirrors
// apps/desktop/src/workspaces/production/tabs/shots.tsx.

import Foundation
import Observation
import OpenFrameCore

enum ShotSheet: Identifiable {
    case move(ShotDto)
    case attach(ShotDto)
    case copyPlanning
    case export

    var id: String {
        switch self {
        case let .move(s): "move-\(s.id)"
        case let .attach(s): "attach-\(s.id)"
        case .copyPlanning: "copy-planning"
        case .export: "export"
        }
    }
}

@MainActor
@Observable
final class ShotsModel {
    unowned let hub: VisualHub

    private(set) var shots: [ShotDto] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    /// A shot opened from elsewhere (a panel's chip, a search result) that may not be in the list.
    private(set) var focused: ShotDto?
    private(set) var characters: [String] = []
    private(set) var attachBoards: [StoryboardSummary] = []
    private(set) var attachPanels: [PanelDto] = []

    // The person's view.
    var shotScene: String?
    var showRemoved = false
    var selectedId: String?
    var picked: Set<String> = []
    var sheet: ShotSheet?
    var composerError: String?
    /// The shot order while a reorder is on its way to the core.
    private(set) var order: [String]?

    @ObservationIgnored private var generation = 0

    init(hub: VisualHub) { self.hub = hub }

    // MARK: Derived

    var sceneKey: String? { resolveShotSceneKey(wanted: shotScene, scenes: hub.scenes, showRemoved: showRemoved) }
    var scene: VisualScene? { hub.scenes?.scenes.first { $0.lineageId == sceneKey } }
    var removed: RemovedVisualScene? { hub.scenes?.removed.first { $0.lineageId == sceneKey } }
    var allScenes: Bool { sceneKey == "all" }
    var canAdd: Bool { scene != nil }
    var shown: [ShotDto] {
        guard let order else { return shots }
        return order.compactMap { id in shots.first { $0.id == id } }
    }
    var groups: [ShotGroup] { ShotGroup.group(shots) }
    var selected: ShotDto? { shots.first { $0.id == selectedId } ?? (focused?.id == selectedId ? focused : nil) }

    // MARK: Loading

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        guard let key = sceneKey else {
            shots = []
            loaded = true
            return
        }
        do {
            let args = key == "all" ? ShotListArgs(includeRemoved: showRemoved) : ShotListArgs(sceneLineageId: key)
            let list = try await core.shotList(args)
            guard gen == generation else { return }
            shots = list
            order = nil
            loadFailure = nil
            picked = picked.filter { id in list.contains { $0.id == id } }
            if let s = selectedId, !list.contains(where: { $0.id == s }), focused?.id != s { selectedId = nil }
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
        }
        loaded = true
        if let f = focused, let fresh = try? await core.shotGet(ShotIdArgs(id: f.id)) { focused = fresh }
    }

    func chooseScene(_ key: String) async {
        shotScene = key
        selectedId = nil
        picked = []
        await reload()
    }

    func setShowRemoved(_ on: Bool) async {
        showRemoved = on
        if !on, removed != nil { shotScene = nil }
        await reload()
    }

    /// A panel's chip or a search result opens one shot.
    func focus(_ shot: ShotDto) async {
        focused = shot
        if shot.sceneRemoved { showRemoved = true }
        shotScene = shot.sceneLineageId
        selectedId = shot.id
        picked = [shot.id]
        await reload()
    }

    func loadCharacters(for shot: ShotDto) async {
        guard let core = hub.core, !shot.sceneRemoved else {
            characters = []
            return
        }
        characters = (try? await core.visualSceneCharacters(SceneCharactersArgs(sceneId: shot.sceneId))) ?? []
    }

    // MARK: Shots

    /// Add a shot to the open scene. Only a description is needed.
    @discardableResult
    func addShot(description: String) async -> ShotDto? {
        guard let scene else { return nil }
        guard !description.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            composerError = "Describe the shot to add it."
            return nil
        }
        composerError = nil
        return await hub.perform("Could not add the shot") {
            try await $0.shotCreate(CreateShotArgs(sceneId: scene.sceneId, description: description))
        }
    }

    func update(_ shot: ShotDto, description: String? = nil, size: String? = nil, movement: String? = nil, angle: String? = nil,
                lens: String? = nil, cameraNotes: String? = nil, characters: [String]? = nil, soundNote: String? = nil) async {
        await hub.perform("Could not save the shot") {
            try await $0.shotUpdate(UpdateShotArgs(
                id: shot.id, description: description, size: size, movement: movement, angle: angle, lens: lens,
                cameraNotes: cameraNotes, characters: characters, soundNote: soundNote))
        }
    }

    func delete(_ ids: [String]) async {
        guard !ids.isEmpty else { return }
        let r = await hub.perform("Could not delete the shot", done: ids.count == 1 ? "Deleted shot" : "Deleted \(ids.count) shots") {
            try await $0.shotDelete(DeleteShotsArgs(ids: ids))
        }
        if r != nil {
            picked = []
            if let s = selectedId, ids.contains(s) { selectedId = nil }
        }
    }

    @discardableResult
    func duplicate(_ shot: ShotDto) async -> ShotDto? {
        guard let s = await hub.perform("Could not duplicate the shot", { core in
            try await core.shotDuplicate(ShotIdArgs(id: shot.id))
        }) else { return nil }
        ToastCenter.shared.undoable("Duplicated as \(s.label)", app: hub.app)
        return s
    }

    func move(_ shot: ShotDto, toScene sceneId: String) async -> Bool {
        guard let s = await hub.perform("Could not move the shot", { core in
            try await core.shotMove(MoveShotArgs(id: shot.id, sceneId: sceneId))
        }) else { return false }
        ToastCenter.shared.undoable("Moved shot to Scene \(s.sceneNumber ?? "") as \(s.label)", app: hub.app)
        return true
    }

    /// Drag a shot to a new place in the scene's coverage (the target place, 0 based).
    func reorder(_ id: String, to index: Int) async {
        let ids = shown.map(\.id)
        guard let from = ids.firstIndex(of: id), from != index, index >= 0, index < ids.count else { return }
        order = VisualText.move(ids, from: from, to: index)
        let ok = await hub.perform("Could not reorder the shots", done: "Reordered shots") {
            try await $0.shotReorder(ReorderShotArgs(id: id, index: Double(index)))
        }
        if ok == nil { order = nil }
    }

    /// The keyboard way to reorder: one place earlier or later.
    func nudge(_ id: String, by delta: Int) async {
        guard let i = shown.firstIndex(where: { $0.id == id }) else { return }
        await reorder(id, to: i + delta)
    }

    /// Click rules of the list: plain opens the shot, Command toggles, Shift selects a run.
    func click(_ shot: ShotDto, command: Bool, shift: Bool) {
        if command {
            if picked.contains(shot.id) { picked.remove(shot.id) } else { picked.insert(shot.id) }
            return
        }
        if shift, !picked.isEmpty {
            let all = shown.map(\.id)
            guard let anchor = all.firstIndex(where: { picked.contains($0) }), let at = all.firstIndex(of: shot.id) else { return }
            picked = Set(all[min(anchor, at)...max(anchor, at)])
            return
        }
        picked = [shot.id]
        selectedId = shot.id
    }

    // MARK: Reference image and storyboard panels

    func setReference(_ shot: ShotDto, path: String?) async {
        await hub.perform("Could not change the reference image", done: path == nil ? "Removed reference image" : "Set reference image") {
            try await $0.shotSetReferenceImage(ShotReferenceArgs(id: shot.id, path: path))
        }
    }

    func chooseReference(for shot: ShotDto) async {
        guard let path = VisualPanels.chooseImages(multiple: false, message: "Choose a reference image").first else { return }
        await setReference(shot, path: path)
    }

    func createPanel(for shot: ShotDto) async {
        await hub.perform("Could not create the storyboard panel", done: "Created storyboard panel from shot") {
            try await $0.shotCreatePanel(ShotIdArgs(id: shot.id))
        }
    }

    func detach(_ shot: ShotDto, panelId: String) async {
        await hub.perform("Could not detach the storyboard panel", done: "Detached storyboard panel") {
            try await $0.shotDetachPanel(ShotPanelArgs(shotId: shot.id, panelId: panelId))
        }
    }

    func loadAttachChoices(for shot: ShotDto, boardId: String?) async {
        guard let core = hub.core else { return }
        let all = (try? await core.storyboardList()) ?? []
        attachBoards = all.filter { $0.sceneLineageId == shot.sceneLineageId } + all.filter { $0.sceneLineageId != shot.sceneLineageId }
        let id = boardId ?? attachBoards.first?.id
        if let id, let d = try? await core.storyboardGet(StoryboardIdArgs(id: id)) {
            attachPanels = d.panels
        } else {
            attachPanels = []
        }
    }

    func attach(_ shot: ShotDto, panelId: String) async -> Bool {
        let r = await hub.perform("Could not attach the storyboard panel", done: "Attached storyboard panel to \(shot.label)") {
            try await $0.shotAttachPanel(ShotPanelArgs(shotId: shot.id, panelId: panelId))
        }
        return r != nil
    }

    func copyPlanning(toScene sceneId: String) async -> Bool {
        guard let from = scene?.lineageId else { return false }
        guard let r = await hub.perform("Could not copy the planning", { core in
            try await core.shotCopyPlanning(CopyPlanningArgs(fromSceneLineageId: from, toSceneId: sceneId))
        }) else { return false }
        let s = Int(r.shots), b = Int(r.storyboards)
        ToastCenter.shared.undoable("Copied \(VisualText.plural(s, "shot")) and \(VisualText.plural(b, "storyboard"))", app: hub.app)
        return true
    }

    // MARK: Export

    func export(format: String, path: String, sceneIds: [String]?, includeRemoved: Bool) async -> ExportResult? {
        let r = await hub.perform("Could not export the shot list") {
            try await $0.shotExportList(ExportShotListArgs(format: format, path: path, sceneIds: sceneIds, includeRemoved: includeRemoved ? true : nil))
        }
        if let r { hub.info("Exported \(r.fileName)") }
        return r
    }
}

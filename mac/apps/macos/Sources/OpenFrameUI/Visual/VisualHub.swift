// The root model of the Visual Planning workspace. It owns the models of the three tabs
// (Moodboards, Storyboards, Shot Lists), the scenes every tab plans against (visual.scenes),
// and the moves between tabs (a panel opens its shot, a shot opens its panel). It holds no
// project data of its own: every read and change is a core operation (ADR-0027). Mirrors
// apps/desktop/src/workspaces/production/visual/shared.tsx and the three tabs.

import AppKit
import Foundation
import Observation
import OpenFrameCore
import UniformTypeIdentifiers

@MainActor
@Observable
final class VisualHub {
    let app: AppModel
    var tab: VisualTab = .moodboards
    private(set) var scenes: VisualScenes?
    private(set) var scenesLoaded = false
    private(set) var scenesFailure: CoreFailure?
    /// A hint for the screens. The core still refuses what a role may not do.
    private(set) var canEdit = true

    @ObservationIgnored private(set) var moodboards: MoodboardsModel!
    @ObservationIgnored private(set) var storyboards: StoryboardsModel!
    @ObservationIgnored private(set) var shots: ShotsModel!
    @ObservationIgnored private var sceneGen = 0

    init(app: AppModel) {
        self.app = app
        moodboards = MoodboardsModel(hub: self)
        storyboards = StoryboardsModel(hub: self)
        shots = ShotsModel(hub: self)
    }

    var core: Core? { app.core }
    var hasScenes: Bool { !(scenes?.scenes.isEmpty ?? true) }

    // MARK: Loading

    func reloadScenes() async {
        guard let core else { return }
        sceneGen += 1
        let gen = sceneGen
        do {
            let s = try await core.visualScenes()
            guard gen == sceneGen else { return }
            scenes = s
            scenesFailure = nil
        } catch {
            guard gen == sceneGen else { return }
            scenesFailure = CoreFailure(asCore: error)
        }
        scenesLoaded = true
    }

    func reloadMembers() async {
        guard let core else { return }
        if let members = try? await core.projectMembers() {
            let me = members.first { $0.isYou }
            canEdit = me == nil || me?.role == "Owner" || me?.role == "Editor"
        }
    }

    // MARK: Running changes

    /// Runs one core call for a change. A failure is shown in plain words and the screen keeps
    /// what it had; `done` is the undoable toast of a change that went through.
    @discardableResult
    func perform<T>(_ title: String, done: String? = nil, _ body: (Core) async throws -> T) async -> T? {
        guard let core else { return nil }
        do {
            let result = try await body(core)
            if let done { ToastCenter.shared.undoable(done, app: app) }
            return result
        } catch {
            app.show(title, error)
            return nil
        }
    }

    func info(_ text: String) { ToastCenter.shared.show(text) }

    /// "Scene 12 changed since planning": the person has looked, so the note goes away.
    func markReviewed(_ scene: VisualScene) async {
        let ok = await perform("Could not mark the scene as reviewed", done: "Marked Scene \(scene.number) as reviewed") {
            try await $0.visualMarkSceneReviewed(MarkSceneReviewedArgs(sceneLineageId: scene.lineageId))
        }
        if ok != nil { await reloadScenes() }
    }

    // MARK: Moving between tabs

    /// A panel's shot chip opens that shot in the Shot Lists tab.
    func openShot(_ shotId: String) async {
        guard let core else { return }
        if let shot = try? await core.shotGet(ShotIdArgs(id: shotId)) {
            await shots.focus(shot)
            tab = .shots
        }
    }

    /// A shot's panel chip opens that panel in the Storyboards tab.
    func openPanel(storyboardId: String, panelId: String) async {
        tab = .storyboards
        await storyboards.focus(storyboardId: storyboardId, panelId: panelId)
    }
}

// MARK: Pickers (AppKit)

@MainActor
enum VisualPanels {
    static var imageTypes: [UTType] {
        VisualText.imageExtensions.compactMap { UTType(filenameExtension: $0) }
    }

    /// Ask for image files. The chosen paths are checked by the core.
    static func chooseImages(multiple: Bool, message: String = "Choose images") -> [String] {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = multiple
        panel.allowedContentTypes = imageTypes
        panel.message = message
        guard panel.runModal() == .OK else { return [] }
        return panel.urls.map(\.path)
    }

    /// Ask where to save an export.
    static func chooseSavePath(name: String, message: String) -> String? {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = name
        panel.message = message
        return panel.runModal() == .OK ? panel.url?.path : nil
    }
}

/// What an export dialog offers: the formats of the operation and the format picked.
struct VisualExportFormat: Identifiable, Hashable {
    var id: String
    var label: String
    static let pdf = VisualExportFormat(id: "pdf", label: "PDF")
    static let csv = VisualExportFormat(id: "csv", label: "CSV")
    static let xlsx = VisualExportFormat(id: "xlsx", label: "Excel (XLSX)")
}

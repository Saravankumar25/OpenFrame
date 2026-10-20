// Project Settings, General tab (Windows: workspaces/settings/ProjectSettingsForm.tsx). The core
// holds the rules (title and type are required, only the owner may change settings); this model
// keeps the form's draft and forwards the save as project.update_settings and project.set_status.

import Observation
import OpenFrameCore
import SwiftUI

enum ProjectSettingsTab: String, CaseIterable, Identifiable, Hashable {
    case general, shortcuts, templates, advanced
    var id: String { rawValue }
    var title: String {
        switch self {
        case .general: "Project settings"
        case .shortcuts: "Keyboard shortcuts"
        case .templates: "Templates"
        case .advanced: "Advanced"
        }
    }
    var symbol: String {
        switch self {
        case .general: "gearshape"
        case .shortcuts: "keyboard"
        case .templates: "doc.on.doc"
        case .advanced: "slider.horizontal.3"
        }
    }
}

struct ProjectSettingsDraft: Equatable {
    var title = ""
    var projectType: ProjectType = .featureFilm
    var language = ""
    var genre = ""
    var creator = ""
    var logline = ""
    var status: ProjectStatus = .idea
    var projectNotes = ""

    init() {}

    init(_ s: ProjectSettings) {
        title = s.project.title
        projectType = s.project.projectType
        language = s.project.language ?? ""
        genre = s.project.genre ?? ""
        creator = s.project.creator ?? ""
        logline = s.project.logline ?? ""
        status = ProjectStatus(rawValue: s.project.status) ?? .idea
        projectNotes = s.projectNotes ?? ""
    }

    static func isEpisodic(_ t: ProjectType) -> Bool { t == .episodic || t == .series }

    /// New projects are films: Episodic and Series are no longer offered (owner change list item
    /// 9, 2026-10-19, as on Windows). A project made with one of them earlier keeps it.
    static let offeredTypes: [ProjectType] = [.featureFilm, .shortFilm]

    static func typeChoices(current: ProjectType) -> [ProjectType] {
        offeredTypes.contains(current) ? offeredTypes : offeredTypes + [current]
    }
}

@MainActor
@Observable
final class ProjectSettingsModel {
    var draft = ProjectSettingsDraft()
    private(set) var initial = ProjectSettingsDraft()
    private(set) var settings: ProjectSettings?
    private(set) var loading = true
    private(set) var saving = false
    private(set) var error: String?
    /// The form has been edited (the Save button is on).
    var showTitleError = false

    private weak var app: AppModel?

    init(app: AppModel?) { self.app = app }

    var dirty: Bool { draft != initial }
    var titleMissing: Bool { draft.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    var canSave: Bool { dirty && !titleMissing && !saving }
    var typeHint: String? {
        ProjectSettingsDraft.isEpisodic(draft.projectType) != ProjectSettingsDraft.isEpisodic(initial.projectType)
            ? "Changing the type never deletes anything you have written." : nil
    }

    func load(reset: Bool = false) async {
        guard let core = app?.core else { return }
        do {
            let s = try await core.projectGetSettings()
            settings = s
            initial = ProjectSettingsDraft(s)
            // A reload after a save must not drop what the person is typing now.
            if reset || !dirty || loading { draft = initial }
            error = nil
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        loading = false
    }

    func discard() {
        draft = initial
        showTitleError = false
    }

    /// Saves the form. Returns true when the core stored it.
    @discardableResult
    func save() async -> Bool {
        showTitleError = true
        guard let app, let core = app.core, canSave else { return false }
        saving = true
        defer { saving = false }
        let d = draft
        do {
            _ = try await core.projectUpdateSettings(UpdateSettingsArgs(
                title: d.title.trimmingCharacters(in: .whitespacesAndNewlines),
                language: d.language, genre: d.genre, creator: d.creator, logline: d.logline,
                projectType: d.projectType,
                settings: ["projectNotes": d.projectNotes.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? .null : .string(d.projectNotes)]))
            if d.status != initial.status {
                _ = try await core.projectSetStatus(SetStatusArgs(status: d.status))
            }
            ToastCenter.shared.undoable("Updated project settings", app: app)
            error = nil
            showTitleError = false
            await load(reset: true)
            return true
        } catch {
            self.error = CoreFailure(asCore: error).message
            return false
        }
    }

    func openLocation() async {
        do { try await app?.core?.revealLocation("project") } catch { app?.show("Could not show the folder", error) }
    }
}

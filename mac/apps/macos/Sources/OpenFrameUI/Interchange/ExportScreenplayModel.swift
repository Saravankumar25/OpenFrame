// Export Screenplay (Windows: ExportScreenplayDialog). What am I exporting, which version, in which
// format, and where will it be saved? An export is a snapshot written to the place the person
// chose: the project never changes and private notes are never included. A failed export leaves
// the project untouched and the core removes any half-written file.

import Foundation
import Observation
import OpenFrameCore

struct IXDraftChoice: Identifiable, Equatable {
    let id: String
    let label: String
    let screenplayId: String
    let screenplayTitle: String
    let draftName: String
    let status: String
    let revision: String?
    let sceneCount: Int
}

enum IXScope { case all, selected }

@MainActor
@Observable
final class ExportScreenplayModel {
    private(set) var loaded = false
    private(set) var loadFailure: String?
    private(set) var choices: [IXDraftChoice] = []
    private(set) var scenes: [InterchangeExportSceneDto] = []
    private(set) var scenesLoading = false
    private(set) var busy = false
    private(set) var error: String?
    private(set) var result: InterchangeScreenplayExportResult?

    var draft = ""
    var format: IXExportFormat = .pdf
    var showMore = false
    var scope: IXScope = .all
    var picked: Set<String> = []
    var revisionMarks = false
    var titlePage = true
    var sceneNumbers = true
    var revisionInfo = true
    var includeNotes = false

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let wantedDraft: String?

    init(app: AppModel, draftId: String? = nil) {
        self.app = app
        self.wantedDraft = draftId
    }

    var owner: AppModel { app }
    var choice: IXDraftChoice? { choices.first { $0.id == draft } ?? choices.first }
    var scopeOk: Bool { scope == .all || !picked.isEmpty }
    var empty: Bool { loaded && choices.isEmpty }

    /// Formats shown: the main list, plus plain text once asked for (or chosen).
    var formatsShown: [IXExportFormat] {
        showMore || format.additional ? IXExportFormat.allCases : IXExportFormat.main
    }

    var options: InterchangeScreenplayExportOptions {
        InterchangeScreenplayExportOptions(
            titlePage: titlePage, sceneNumbers: sceneNumbers, includeNotes: includeNotes,
            revisionMarks: revisionMarks, revisionInfo: revisionInfo,
            sceneIds: scope == .selected ? scenes.map(\.id).filter { picked.contains($0) } : nil)
    }

    var suggestedName: String {
        IXNames.defaultExportFileName(screenplayTitle: choice?.screenplayTitle ?? "", draftName: choice?.draftName ?? "", format: format)
    }

    func load() async {
        guard let core = app.core else { return }
        await ScreenplayWorkspaceModel.flushAll()
        do {
            let sources = try await core.interchangeSources()
            let many = sources.screenplays.count > 1
            choices = sources.screenplays.flatMap { s in
                s.drafts.map { d in
                    IXDraftChoice(
                        id: d.id,
                        label: IXLabels.draftLabel(screenplayTitle: s.title, draftName: d.name, multiple: many) + (d.isCurrent ? " (current)" : ""),
                        screenplayId: s.id, screenplayTitle: s.title, draftName: d.name, status: d.status,
                        revision: [d.revisionLabel, d.revisionColor.map { "\($0) Revision" }].compactMap { $0 }.joined(separator: " \u{00B7} ").ixNonEmpty,
                        sceneCount: Int(d.sceneCount))
                }
            }
            let all = choices.map(\.id)
            if let wantedDraft, all.contains(wantedDraft) {
                draft = wantedDraft
            } else if let first = sources.screenplays.first(where: { !$0.drafts.isEmpty }) {
                draft = first.currentDraftId.flatMap { all.contains($0) ? $0 : nil } ?? first.drafts[0].id
            }
            loaded = true
            await applyPageSetupDefaults()
            await loadScenes()
        } catch {
            loadFailure = CoreFailure(asCore: error).message
            loaded = true
        }
    }

    /// Starting choices from the screenplay's page setup: title page switch and revision marks.
    func applyPageSetupDefaults() async {
        guard let core = app.core, let c = choice else { return }
        if let overview = try? await core.screenplayOverview(ScreenplayOverviewArgs(screenplayId: c.screenplayId)),
           let setup = overview.screenplay?.pageSetup {
            titlePage = setup.includeTitlePage
            revisionMarks = setup.revisionMarks
            // Scene numbers start on in the export dialog, as on Windows.
            sceneNumbers = true
        }
    }

    func changeDraft(_ id: String) async {
        draft = id
        picked = []
        error = nil
        await applyPageSetupDefaults()
        await loadScenes()
    }

    func loadScenes() async {
        guard let core = app.core, !draft.isEmpty else { return }
        scenesLoading = true
        defer { scenesLoading = false }
        scenes = (try? await core.interchangeDraftScenes(InterchangeDraftScenesArgs(draftId: draft))) ?? []
    }

    func setPicked(_ id: String, _ on: Bool) {
        if on { picked.insert(id) } else { picked.remove(id) }
    }

    /// Write the file the person chose. The core checks the folder, the name and the format.
    @discardableResult
    func export(to url: URL) async -> Bool {
        guard let core = app.core, !draft.isEmpty, scopeOk else { return false }
        error = nil
        busy = true
        defer { busy = false }
        let scoped = ScopedAccess()
        scoped.hold(url.deletingLastPathComponent())
        scoped.hold(url)
        defer { scoped.release() }
        do {
            let r = try await core.screenplayExport(InterchangeScreenplayExportArgs(
                draftId: draft, format: format.rawValue, path: url.path, options: options))
            result = r
            ToastCenter.shared.show("Exported \(r.fileName)")
            return true
        } catch {
            self.error = CoreFailure(asCore: error).message
            return false
        }
    }

    func reveal() async {
        guard let path = result?.path else { return }
        try? await app.core?.revealLocation("exported", path: path)
    }
}

extension String {
    /// nil for an empty string.
    var ixNonEmpty: String? { isEmpty ? nil : self }
}

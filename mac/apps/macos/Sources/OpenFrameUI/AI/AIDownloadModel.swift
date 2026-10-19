// Download OpenFrame AI (ADR-0026). The models are not in the app: the person downloads them once,
// by clicking. Before the click this model only reads the size and free space; nothing touches the
// network until `start()`. The core does the work and reports phases; this model forwards the
// choices (start, pause, resume, remove, import) and shows what the core says.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class AIDownloadModel {
    let app: AppModel
    var download: AiDownloadDto?
    var loadFailed = false
    var busy = false
    /// Plain message from the last action, shown in the card (checksum failure, offline, no space).
    var error: String?
    /// Set by the screen: asks the person for a model file. Tests set a fixed answer.
    var pickFile: @MainActor () -> URL? = { nil }

    init(app: AppModel) {
        self.app = app
    }

    private var core: Core? { app.core }
    var phase: String { download?.phase ?? "" }
    var isRunning: Bool { AILogic.running.contains(phase) }

    func refresh() async {
        guard let core else { return }
        do {
            let before = download?.phase
            download = try await core.aiDownloadStatus()
            loadFailed = false
            if before != download?.phase, download?.phase == "Done" { announceReady() }
        } catch {
            loadFailed = true
        }
    }

    private func announceReady() {
        ToastCenter.shared.show("OpenFrame AI is ready.")
    }

    /// Start, resume or try again. A damaged set is removed first, but only before a download of it
    /// began: a paused or stopped download keeps what it already has and resumes.
    func start(damaged: Bool) async {
        guard let core, !busy, let d = download else { return }
        busy = true
        error = nil
        defer { busy = false }
        let removeFirst = damaged && (d.phase == "Idle" || d.problem?.code == "ai.remove_first")
        do {
            if removeFirst { _ = try await core.aiDownloadRemove() }
            _ = try await core.aiDownloadStart()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        await refresh()
    }

    func pause() async {
        guard let core, !busy else { return }
        busy = true
        error = nil
        defer { busy = false }
        do { _ = try await core.aiDownloadPause() } catch { self.error = CoreFailure(asCore: error).message }
        await refresh()
    }

    /// Remove the downloaded files (Settings). The screens show what is really on the disk after it.
    func remove() async -> Bool {
        guard let core, !busy else { return false }
        busy = true
        defer { busy = false }
        var ok = true
        do {
            _ = try await core.aiDownloadRemove()
            ToastCenter.shared.show("OpenFrame AI was removed. You can download it again here.")
        } catch {
            self.error = CoreFailure(asCore: error).message
            ok = false
        }
        await refresh()
        return ok
    }

    /// A model file the person already has: copied and checked against the pins, never moved.
    func importFile() async {
        guard let core, !busy else { return }
        // Busy from the first click: a second click while the panel is open does nothing.
        busy = true
        defer { busy = false }
        guard let url = pickFile() else { return }
        await importFile(at: url, core: core)
    }

    /// The import itself (also the path tests take).
    func importFile(at url: URL) async {
        guard let core else { return }
        busy = true
        defer { busy = false }
        await importFile(at: url, core: core)
    }

    private func importFile(at url: URL, core: Core) async {
        error = nil
        do {
            let r = try await core.aiDownloadImport(AiImportArgs(path: url.path))
            ToastCenter.shared.show(AILogic.importMessage(r))
        } catch {
            let message = CoreFailure(asCore: error).message
            self.error = message
            ToastCenter.shared.show(message, kind: .error)
        }
        await refresh()
    }
}

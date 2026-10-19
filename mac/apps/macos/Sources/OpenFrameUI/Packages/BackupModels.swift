// Create Backup (Windows: BackupDialog) and Export Full Project (ExportProjectDialog). Both run as
// background tasks of the core with progress and a real Cancel. Nothing is left behind when they
// are cancelled or fail: the core removes the half-written file.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class BackupModel {
    private(set) var preview: BackupPreview?
    private(set) var loadError: String?
    var label = ""
    private(set) var destination: URL?
    var portable = false
    let task = PackageTaskFollower()
    private(set) var startError: String?

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let access = ScopedAccess()

    init(app: AppModel) { self.app = app }

    var name: String { preview.map { PKText.backupFileName(baseName: $0.baseName, label: label) } ?? "" }
    var report: BackupReport? { task.result(BackupReport.self) }
    var canStart: Bool { destination != nil && !task.running && preview != nil && task.taskId == nil }
    var defaultDirectory: URL? { preview.map { URL(fileURLWithPath: $0.defaultDir, isDirectory: true) } }

    func load() async {
        await ScreenplayWorkspaceModel.flushAll()
        do { preview = try await app.core?.packagesBackupPreview() }
        catch { loadError = CoreFailure(asCore: error).message }
    }

    func choose(destination url: URL) {
        access.release()
        access.hold(url.deletingLastPathComponent())
        access.hold(url)
        destination = url
    }

    func start() async {
        guard let core = app.core, let destination, canStart else { return }
        startError = nil
        let args = BackupCreateArgs(path: destination.path, label: label.trimmingCharacters(in: .whitespaces).ixNonEmpty, includeExternal: portable)
        do {
            try await task.begin(core: core) { try await core.packagesCreateBackup(args).taskId }
        } catch {
            startError = CoreFailure(asCore: error).message
        }
    }

    func cancel() { task.cancel(core: app.core) }

    func finish() { task.stop(); access.release() }
}

@MainActor
@Observable
final class ExportProjectModel {
    private(set) var preview: BackupPreview?
    private(set) var loadError: String?
    var portable = false
    let task = PackageTaskFollower()
    private(set) var startError: String?

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let access = ScopedAccess()

    init(app: AppModel) { self.app = app }

    var report: BackupReport? { task.result(BackupReport.self) }

    /// "Harbour Lights.ofproject".
    var suggestedName: String { "\(preview?.projectTitle ?? "Project").ofproject" }
    var defaultDirectory: URL? {
        preview.map { URL(fileURLWithPath: $0.defaultDir, isDirectory: true).deletingLastPathComponent() }
    }

    func load() async {
        await ScreenplayWorkspaceModel.flushAll()
        do { preview = try await app.core?.packagesBackupPreview() }
        catch { loadError = CoreFailure(asCore: error).message }
    }

    func export(to url: URL) async {
        guard let core = app.core, !task.running else { return }
        startError = nil
        access.release()
        access.hold(url.deletingLastPathComponent())
        access.hold(url)
        let args = BackupCreateArgs(path: url.path, label: nil, includeExternal: portable)
        do {
            try await task.begin(core: core) { try await core.packagesExportProject(args).taskId }
        } catch {
            startError = CoreFailure(asCore: error).message
        }
    }

    func cancel() { task.cancel(core: app.core) }
    func finish() { task.stop(); access.release() }
}

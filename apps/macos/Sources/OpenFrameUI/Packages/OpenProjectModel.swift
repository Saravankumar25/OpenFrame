// Open Project Package and Restore Backup (Windows: OpenProjectDialog). Open Package, validate,
// project identity, collision choice, file and reference report, import. Importing never silently
// replaces the current project: a project that already exists opens as a copy by default, and
// Replace Existing After Backup saves a backup first and asks again. The core does the work as a
// background task; when it ends the window shows the project the core opened.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class OpenProjectModel {
    private(set) var info: PackageInspection?
    private(set) var path: String?
    var mode: PackageOpenMode = .copy
    var confirmReplace = false
    private(set) var checking = false
    private(set) var errorText: String?
    private(set) var startError: String?
    let task = PackageTaskFollower()

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let access = ScopedAccess()

    init(app: AppModel) { self.app = app }

    var isBackup: Bool { info?.manifest.packageType == .backup }
    var title: String { isBackup ? "Restore Backup" : "Open Project Package" }
    var collision: PackageCollision? { info?.collision }
    var missingExternal: [BackupFileEntry] { info?.external.filter { $0.status == "Missing" } ?? [] }
    var notesNotice: String? { info.flatMap { PKText.restoredNotesNotice($0.manifest) } }
    var report: PackageProjectImportReport? { task.result(PackageProjectImportReport.self) }
    var started: Bool { task.taskId != nil || task.starting }

    var primaryLabel: String {
        switch mode {
        case .replace: "Replace Existing After Backup"
        case .copy: "Open as Copy"
        case .open: isBackup ? "Restore" : "Open Project"
        }
    }

    /// The sheet's title: a collision is named before anything is created.
    var headline: String { collision != nil && !started ? "This project already exists on this computer" : title }

    /// Choose and inspect a package. Nothing is created yet. A damaged or wrong file is refused.
    func choose(file url: URL) async {
        guard let core = app.core else { return }
        access.release()
        access.hold(url)
        checking = true
        info = nil
        errorText = nil
        defer { checking = false }
        do {
            let r = try await core.packagesInspectPackage(PackagePathArgs(path: url.path))
            // Only a Full Project Package or a Backup opens here; the other kinds have their own sheets.
            guard r.manifest.packageType == .project || r.manifest.packageType == .backup else {
                errorText = "This is a \(r.typeLabel.lowercased()), not a project or backup package. Use Import Exchange Package\u{2026} for review and exchange packages."
                return
            }
            path = url.path
            info = r
            mode = r.defaultMode ?? .copy
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    /// Start the import. The editors store what they hold first, because opening another project
    /// closes this one.
    func start(_ chosen: PackageOpenMode? = nil) async {
        guard let core = app.core, let path, !task.running else { return }
        let m = chosen ?? mode
        startError = nil
        await ScreenplayWorkspaceModel.flushAll()
        let args = PackageImportProjectArgs(path: path, mode: m, parentDir: nil)
        do {
            try await task.begin(core: core) { try await core.packagesImportProject(args).taskId }
        } catch {
            startError = CoreFailure(asCore: error).message
            return
        }
        await task.waitUntilFinished()
        if let report { await adopt(report) }
    }

    /// The core opened the imported project; make the window show it, the way Locate does.
    func adopt(_ report: PackageProjectImportReport) async {
        await app.closeProject()
        await app.openProject(path: report.path)
    }

    func cancelTask() { task.cancel(core: app.core) }
    func finish() { task.stop(); access.release() }
}

// Import an exchange, review or response package (Windows: ImportExchangeDialog). Open Package,
// validation, mapping preview, changes by object, ambiguities and unmatched notes, selection, apply.
// Nothing changes before the person chooses; a failed apply is rolled back completely by the core.

import Foundation
import Observation
import OpenFrameCore

enum PKStaleChoice: String, Hashable { case view, comments, record, compare }

@MainActor
@Observable
final class ExchangeImportModel {
    private(set) var session: PackageImportSession?
    var selected: Set<String> = []
    var expanded: Set<String> = []
    var backup = false
    private(set) var busy = false
    private(set) var staleStep = false
    var staleChoice: PKStaleChoice = .comments
    private(set) var onlyText = false
    /// A problem that does not end the flow (a package that cannot be opened, a failed apply).
    private(set) var errorText: String?

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let access = ScopedAccess()
    @ObservationIgnored private let resumeId: String?

    init(app: AppModel, sessionId: String? = nil) {
        self.app = app
        self.resumeId = sessionId
    }

    var finished: Bool { session.map { $0.status != "Previewed" } ?? false }
    var result: PackageImportResult? { session?.result }
    var visibleChanges: [PackageChange] { (session?.changes ?? []).filter { !$0.copyOnly && (!onlyText || $0.group == "text") } }
    var visibleGroups: [PackageChangeGroup] { (session?.groups ?? []).filter { $0.key != "copy" && (!onlyText || $0.key == "text") } }
    var commentsReceived: Int { (session?.changes ?? []).filter { $0.kind == .comment && $0.applicable }.count }

    /// The choices of the stale step, in order.
    var staleOptions: [(choice: PKStaleChoice, title: String, hint: String, isDefault: Bool)] {
        guard let s = session else { return [] }
        var out: [(PKStaleChoice, String, String, Bool)] = []
        if s.canView { out.append((.view, "View the review without importing", "Open the package read-only. Nothing is imported.", false)) }
        if s.changes.contains(where: { $0.kind == .comment }) {
            out.append((.comments, "Import comments only", "Attach comments to matching text; your \(s.hostVersion) is not touched.", true))
        }
        if s.canCreateRecord { out.append((.record, "Create a separate review record", "Keep the package and its notes for reference without changing your work.", false)) }
        out.append((.compare, "Compare versions", "\(s.sourceVersion) (package) vs \(s.hostVersion) (yours)", false))
        return out.map { (choice: $0.0, title: $0.1, hint: $0.2, isDefault: $0.3) }
    }

    private func show(_ s: PackageImportSession) {
        session = s
        selected = PKText.defaultSelection(s.changes)
        staleStep = s.stale && s.status == "Previewed"
        staleChoice = PKText.staleDefault(s) == .commentsOnly ? .comments : .record
    }

    /// Resume an existing Import Session (for example after viewing the package).
    func resume() async {
        guard let id = resumeId, session == nil, let core = app.core else { return }
        do { show(try await core.packagesSession(PackageSessionArgs(sessionId: id))) }
        catch { errorText = CoreFailure(asCore: error).message }
    }

    /// The package the person chose. The core validates it before anything is shown.
    func open(file url: URL) async {
        guard let core = app.core else { return }
        access.release()
        access.hold(url)
        busy = true
        errorText = nil
        defer { busy = false }
        do { show(try await core.packagesOpenExchange(PackagePathArgs(path: url.path))) }
        catch { errorText = CoreFailure(asCore: error).message }
    }

    func apply(_ mode: PackageApplyMode) async {
        guard let core = app.core, let s = session else { return }
        busy = true
        errorText = nil
        defer { busy = false }
        do {
            session = try await core.packagesApplyImport(PackageApplyArgs(
                sessionId: s.id, mode: mode, selected: Array(selected), createBackup: backup))
            staleStep = false
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    /// Leave the sheet: a session that was only previewed is cancelled so nothing stays half open.
    func cancel() async {
        if let s = session, s.status == "Previewed", let core = app.core {
            do { _ = try await core.packagesCancelImport(PackageSessionArgs(sessionId: s.id)) }
            catch { errorText = CoreFailure(asCore: error).message }
        }
        access.release()
    }

    func undo() async {
        guard let core = app.core, let s = session else { return }
        busy = true
        defer { busy = false }
        do { session = try await core.packagesUndoImport(PackageSessionArgs(sessionId: s.id)) }
        catch { errorText = CoreFailure(asCore: error).message }
    }

    /// The stale step's Continue. `view` is handled by the sheet (it opens the viewer).
    func continueStale() async -> PKStaleChoice {
        switch staleChoice {
        case .view: break
        case .comments: await apply(.commentsOnly)
        case .record: await apply(.record)
        case .compare:
            onlyText = true
            staleStep = false
        }
        return staleChoice
    }

    func showAllChanges() { onlyText = false }

    func toggle(_ id: String, _ on: Bool) {
        if on { selected.insert(id) } else { selected.remove(id) }
    }

    func toggleGroup(_ key: String, _ on: Bool) {
        guard let s = session else { return }
        selected = PKText.toggleGroup(selected, s.changes, group: key, on: on)
    }

    func groupState(_ key: String) -> PKText.GroupSelection {
        PKText.groupSelection(selected, session?.changes ?? [], group: key)
    }

    func toggleExpanded(_ key: String) {
        if expanded.contains(key) { expanded.remove(key) } else { expanded.insert(key) }
    }
}

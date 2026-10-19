// The Review Queue (mockup 163) and the Import and Export History: import sessions with their
// results, kept review records, open reviews and the package log.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class ReviewQueueModel {
    private(set) var items: [PackageQueueItem] = []
    private(set) var scenes: [PackageSceneOption] = []
    private(set) var loaded = false
    private(set) var busyItem: String?
    private(set) var errorText: String?
    @ObservationIgnored let app: AppModel

    init(app: AppModel) { self.app = app }

    var pending: [PackageQueueItem] { items.filter { $0.status == "Pending" } }
    var ambiguous: [PackageQueueItem] { pending.filter { $0.kind == "ambiguous" } }
    var unmatched: [PackageQueueItem] { pending.filter { $0.kind == "unmapped" } }
    var attached: [PackageQueueItem] { items.filter { $0.status == "Attached" } }

    func load() async {
        guard let core = app.core else { return }
        do {
            items = try await core.packagesReviewQueue(PackageListArgs())
            scenes = (try? await core.packagesExchangeScenes(PackageScenesArgs())) ?? []
            errorText = nil
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
        loaded = true
    }

    /// Attach one waiting comment to a scene the person chose. Nothing is discarded.
    func attach(_ item: PackageQueueItem, to sceneId: String) async {
        guard let core = app.core else { return }
        busyItem = item.id
        defer { busyItem = nil }
        do {
            let r = try await core.packagesQueueAttach(PackageQueueAttachArgs(itemId: item.id, sceneId: sceneId))
            ToastCenter.shared.show("Attached to \(r.attachedLabel ?? "the scene").")
            await load()
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }
}

enum PKHistoryTab: String, CaseIterable, Hashable {
    case imports, records, reviews, log
    var title: String {
        switch self {
        case .imports: "Imports"
        case .records: "Review records"
        case .reviews: "My reviews"
        case .log: "Package log"
        }
    }
}

@MainActor
@Observable
final class PackageHistoryModel {
    var tab: PKHistoryTab = .imports
    private(set) var sessions: [PackageSessionSummary] = []
    private(set) var records: [PackageRecordSummary] = []
    private(set) var reviews: [PackageReviewSummary] = []
    private(set) var log: [PackageLogEntry] = []
    private(set) var loading = false
    private(set) var errorText: String?
    var deleting: PackageRecordSummary?

    @ObservationIgnored let app: AppModel
    init(app: AppModel) { self.app = app }

    func load() async {
        guard let core = app.core else { return }
        loading = true
        defer { loading = false }
        do {
            switch tab {
            case .imports: sessions = try await core.packagesSessions(PackageListArgs())
            case .records: records = try await core.packagesRecords(PackageListArgs())
            case .reviews: reviews = try await core.packagesReviewList(PackageListArgs())
            case .log: log = try await core.packagesLog(PackageListArgs())
            }
            errorText = nil
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    func undo(_ s: PackageSessionSummary) async {
        guard let core = app.core else { return }
        do {
            let r = try await core.packagesUndoImport(PackageSessionArgs(sessionId: s.id))
            ToastCenter.shared.show(r.result?.message ?? "Import undone.")
            await load()
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    /// Delete a kept review record: it moves to Recently Deleted and can be restored from there.
    func deleteRecord() async {
        guard let core = app.core, let r = deleting else { return }
        do {
            _ = try await core.packagesRecordDelete(PackageRecordArgs(recordId: r.id))
            ToastCenter.shared.undoable("Deleted review record", app: app)
            deleting = nil
            await load()
        } catch {
            errorText = CoreFailure(asCore: error).message
            deleting = nil
        }
    }

    static func resultTone(_ status: String) -> IXTone {
        status == "Failed" || status == "Rejected" ? .red : status == "Applied" ? .green : .yellow
    }
}

// Pure helpers for backup, project and exchange packages (Windows: features/packages/api.ts), and
// the follower of the core's background package tasks. All work happens in Rust: the screens only
// pass the files the person chose and the decisions they made.

import Foundation
import Observation
import OpenFrameCore

enum PKInfo {
    struct Entry { let label: String; let fileExtension: String; let menuLabel: String }

    /// The package intents. They are never interchangeable.
    static func entry(_ t: PackageType) -> Entry {
        switch t {
        case .backup: Entry(label: "Backup Package", fileExtension: "ofbackup", menuLabel: "Create Backup\u{2026}")
        case .project: Entry(label: "Full Project Package", fileExtension: "ofproject", menuLabel: "Export Full Project\u{2026}")
        case .scriptReview: Entry(label: "Screenplay review package", fileExtension: "ofscriptreview", menuLabel: "Export Review Package (script)\u{2026}")
        case .story: Entry(label: "Story Board package", fileExtension: "ofstory", menuLabel: "Story Board Package\u{2026}")
        case .breakdown: Entry(label: "Breakdown package", fileExtension: "ofbreakdown", menuLabel: "Breakdown Package\u{2026}")
        case .shots: Entry(label: "Shot List package", fileExtension: "ofshots", menuLabel: "Shot List Package\u{2026}")
        case .schedule: Entry(label: "Schedule package", fileExtension: "ofschedule", menuLabel: "Schedule Package\u{2026}")
        case .callReview: Entry(label: "Call Sheet review package", fileExtension: "ofcallreview", menuLabel: "Call Sheet Review Package\u{2026}")
        case .response: Entry(label: "Response package", fileExtension: "ofresponse", menuLabel: "Response Package\u{2026}")
        }
    }

    /// Exchange package types that can be exported from a project, in menu order. The Breakdown,
    /// Shot List, Schedule and Call Sheet Review packages were removed (owner change list item 9,
    /// 2026-10-19, as on Windows); only the script review and Story Board packages remain.
    static let exchangeExportTypes: [PackageType] = [.scriptReview, .story]
    static let exchangeOpenTypes: [PackageType] = [.scriptReview, .story, .response]
    static let projectOpenTypes: [PackageType] = [.project, .backup]
    static let reviewOpenTypes: [PackageType] = [.scriptReview, .response]

    static func extensions(_ types: [PackageType]) -> [String] { types.map { entry($0).fileExtension } }

    static func isExchange(_ t: PackageType) -> Bool { t != .backup && t != .project }
}

enum PKText {
    /// Backup file name: project name, date and time, and an optional label.
    static func backupFileName(baseName: String, label: String) -> String {
        let bad: Set<Character> = ["<", ">", ":", "\"", "/", "\\", "|", "?", "*"]
        let clean = String(label.trimmingCharacters(in: .whitespaces).map { bad.contains($0) ? "_" : $0 })
        return "\(baseName)\(clean.isEmpty ? "" : " \(clean)").ofbackup"
    }

    static func formatBytes(_ n: Double) -> String {
        if n < 1024 { return "\(Int(n)) B" }
        if n < 1024 * 1024 { return String(format: "%.0f KB", n / 1024) }
        if n < 1024 * 1024 * 1024 { return String(format: "%.1f MB", n / 1024 / 1024) }
        return String(format: "%.2f GB", n / 1024 / 1024 / 1024)
    }

    /// What the restore screen says about private notes before a backup or Full Project Package is
    /// restored: the person restoring becomes their owner. nil when the package holds none.
    static func restoredNotesNotice(_ m: PackageManifest) -> String? {
        guard m.privateNotesIncluded else { return nil }
        let n = m.tableCounts["private_note"]
        if n == 0 { return nil }
        if n == 1 { return "The private note in this package will become yours. You can see and edit it; other members of the project cannot." }
        let what = n.map { "The \(Int($0)) private notes in this package" } ?? "Private notes in this package"
        return "\(what) will become yours. You can see and edit them; other members of the project cannot."
    }

    static func stateChip(_ s: PackageChangeState) -> (label: String, tone: IXTone) {
        switch s {
        case .safe: ("Safe mapping", .green)
        case .review: ("Review required", .blue)
        case .conflict: ("Conflict", .red)
        case .ambiguous: ("Ambiguous", .yellow)
        case .unmapped: ("Unmapped", .red)
        case .none_: ("None", .plain)
        }
    }

    static func compatibilityTone(_ c: String) -> IXTone { c == "Valid" ? .green : c == "Stale" ? .yellow : .red }

    /// Human result state: never "Import complete" for a subset.
    static func resultLabel(_ status: String) -> String {
        switch status {
        case "PartiallyApplied": "Partially Applied"
        case "PendingReview": "Pending Review"
        case "Previewed": "Previewed, nothing applied"
        default: status
        }
    }

    /// The changes "Import Selected" starts with: what the preview proposes by default.
    static func defaultSelection(_ changes: [PackageChange]) -> Set<String> {
        Set(changes.filter { $0.applicable && !$0.copyOnly && $0.selectedByDefault }.map(\.id))
    }

    /// Toggle every applicable (non-copy) change of a group.
    static func toggleGroup(_ selected: Set<String>, _ changes: [PackageChange], group: String, on: Bool) -> Set<String> {
        var next = selected
        for c in changes where c.group == group && c.applicable && !c.copyOnly {
            if on { next.insert(c.id) } else { next.remove(c.id) }
        }
        return next
    }

    enum GroupSelection { case all, some, none }

    static func groupSelection(_ selected: Set<String>, _ changes: [PackageChange], group: String) -> GroupSelection {
        let items = changes.filter { $0.group == group && $0.applicable && !$0.copyOnly }
        let n = items.filter { selected.contains($0.id) }.count
        return n == 0 ? .none : n == items.count ? .all : .some
    }

    /// Default choice for a stale review package: comments only.
    static func staleDefault(_ s: PackageImportSession) -> PackageApplyMode {
        s.changes.contains { $0.kind == .comment && $0.applicable } ? .commentsOnly : .record
    }

    static func day(ms: Double) -> String { Formatting.dateTime(ms: ms) }
}

// MARK: Background tasks

/// Follows one background package task (backup, project export or import) through the core's
/// `task` events. The stream is opened before the task starts, so a task that finishes at once is
/// not missed. A real Cancel cancels the task in the core.
@MainActor
@Observable
final class PackageTaskFollower {
    private(set) var taskId: String?
    private(set) var progress: TaskProgress?
    private(set) var starting = false
    @ObservationIgnored private var listener: Task<Void, Never>?
    @ObservationIgnored private var stream: AsyncStream<AppEvent>?
    @ObservationIgnored private var inbox: [TaskProgress] = []

    var state: String? { progress?.state }
    var running: Bool { starting || (taskId != nil && !finished) }
    var finished: Bool { ["completed", "failed", "cancelled"].contains(progress?.state ?? "") }
    var percent: Int { Int(((progress?.progress) ?? 0) * 100) }
    var message: String { progress?.message ?? "Starting\u{2026}" }
    var errorMessage: String? { progress?.error?.message }

    /// The task's result decoded as `T`, once it completed.
    func result<T: Decodable>(_ type: T.Type) -> T? {
        guard progress?.state == "completed", let data = try? JSONEncoder().encode(progress?.result) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }

    /// Open the event stream, then start the task with `start`, which returns its id. Throws what
    /// `start` throws; the task id is followed from then on.
    func begin(core: Core, _ start: () async throws -> String) async throws {
        listener?.cancel()
        progress = nil
        taskId = nil
        inbox = []
        starting = true
        let events = core.events()
        stream = events
        listener = Task { [weak self] in
            for await event in events {
                guard let self else { return }
                if case let .task(t) = event { self.receive(t) }
            }
        }
        do {
            let id = try await start()
            taskId = id
            starting = false
            // A task event that arrived before the id was known.
            if let early = inbox.last(where: { $0.taskId == id }) { progress = early }
            inbox = []
        } catch {
            listener?.cancel()
            starting = false
            throw error
        }
    }

    private func receive(_ t: TaskProgress) {
        if let id = taskId {
            guard t.taskId == id else { return }
            progress = t
            if finished { listener?.cancel() }
        } else {
            inbox.append(t)
        }
    }

    /// Wait until the task finished (tests, and screens that must act on the end).
    func waitUntilFinished(seconds: Double = 120) async {
        let deadline = Date().addingTimeInterval(seconds)
        while !finished && Date() < deadline {
            try? await Task.sleep(for: .milliseconds(25))
        }
    }

    func cancel(core: Core?) {
        guard let id = taskId else { return }
        _ = core?.cancelTask(id)
    }

    func stop() { listener?.cancel() }
}

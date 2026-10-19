// Editor and core synchronisation, ported from the Windows editor's `sync.ts`. The core stays the
// source of truth:
//
// * local edits mark the editor dirty; after `delayMs` of quiet (or on `flush()`) the diff between
//   the baseline (what the core has) and the editor is sent as element-level ops; on success the
//   baseline advances;
// * documents read from the core carry an edit sequence number; a read older than the editor's own
//   last save is ignored, a newer or different one replaces the editor content, but never while
//   local edits are pending;
// * `hold` makes later saves wait for a change made elsewhere (a character created from the
//   suggestion list), so the undo history keeps the order the writer made them in.
//
// All closures are injected, so the controller runs without a core.

import Foundation
import OpenFrameCore

public enum ScreenplaySyncStatus: Sendable, Equatable {
    case saved, pending, saving, error
}

/// What to do after a save failed: keep the local edits and try again later, or go back to the
/// core's state.
public enum ScreenplaySyncFailure: Sendable, Equatable {
    case retry, revert
}

@MainActor
public final class ScreenplaySync {
    public struct Deps {
        /// Sends ops to the core; returns the new edit sequence.
        public var send: @MainActor ([ScreenplayEditOp]) async throws -> Double
        /// The current editor content.
        public var read: @MainActor () -> ScreenplaySnapshot
        /// Replaces the editor content with a core document.
        public var apply: @MainActor (ScreenplaySnapshot) -> Void
        /// A save failed.
        public var onError: @MainActor (Error) -> ScreenplaySyncFailure
        public var onStatus: (@MainActor (ScreenplaySyncStatus) -> Void)?

        public init(
            send: @escaping @MainActor ([ScreenplayEditOp]) async throws -> Double,
            read: @escaping @MainActor () -> ScreenplaySnapshot,
            apply: @escaping @MainActor (ScreenplaySnapshot) -> Void,
            onError: @escaping @MainActor (Error) -> ScreenplaySyncFailure,
            onStatus: (@MainActor (ScreenplaySyncStatus) -> Void)? = nil
        ) {
            self.send = send
            self.read = read
            self.apply = apply
            self.onError = onError
            self.onStatus = onStatus
        }
    }

    private struct Job {
        var snap: ScreenplaySnapshot
        var ops: [ScreenplayEditOp]
    }

    private var baseline: ScreenplaySnapshot
    private var ackSeq: Double
    private var dirty = false
    private var inflight: (id: Int, task: Task<Void, Never>)?
    private var nextTaskId = 0
    private var timer: Task<Void, Never>?
    private var latest: (seq: Double, snap: ScreenplaySnapshot)?
    private let deps: Deps
    private let delayMs: Int
    private let retryDelayMs: Int
    private var status: ScreenplaySyncStatus = .saved
    private var disposed = false

    public init(initial: ScreenplaySnapshot, seq: Double, deps: Deps, delayMs: Int = 400, retryDelayMs: Int = 3000) {
        self.baseline = initial
        self.ackSeq = seq
        self.deps = deps
        self.delayMs = delayMs
        self.retryDelayMs = retryDelayMs
    }

    public var isDirty: Bool { dirty || inflight != nil }
    public var currentStatus: ScreenplaySyncStatus { status }
    /// What the core is known to hold.
    public var baselineSnapshot: ScreenplaySnapshot { baseline }

    private func setStatus(_ s: ScreenplaySyncStatus) {
        if s != status {
            status = s
            deps.onStatus?(s)
        }
    }

    /// The editor content changed locally.
    public func markDirty() {
        dirty = true
        setStatus(.pending)
        schedule(delayMs)
    }

    private func schedule(_ ms: Int) {
        timer?.cancel()
        timer = Task { [weak self] in
            do { try await Task.sleep(for: .milliseconds(ms)) } catch { return }
            guard let self, !Task.isCancelled else { return }
            self.timer = nil
            await self.flush()
        }
    }

    private func cancelTimer() {
        timer?.cancel()
        timer = nil
    }

    private func takeId() -> Int {
        nextTaskId += 1
        return nextTaskId
    }

    private func finish(_ id: Int) {
        if inflight?.id == id { inflight = nil }
    }

    /// Reads the editor now and works out what to send. Clears the dirty flag.
    private func prepareSave() -> Job? {
        guard dirty, !disposed else { return nil }
        let snap = deps.read()
        let ops = ScreenplayModel.diff(baseline, snap)
        dirty = false
        if ops.isEmpty {
            setStatus(.saved)
            return nil
        }
        return Job(snap: snap, ops: ops)
    }

    private func perform(_ job: Job) async {
        setStatus(.saving)
        do {
            let seq = try await deps.send(job.ops)
            baseline = job.snap
            ackSeq = max(ackSeq, seq)
            setStatus(dirty ? .pending : .saved)
        } catch {
            switch deps.onError(error) {
            case .retry:
                // Keep the local text; the next diff against the unchanged baseline resends it.
                dirty = true
                setStatus(.error)
                schedule(retryDelayMs)
            case .revert:
                dirty = false
                setStatus(.saved)
                let back = (latest.map { $0.seq >= ackSeq } ?? false) ? latest!.snap : baseline
                baseline = back
                deps.apply(back)
            }
        }
    }

    /// Sends pending edits now and waits until the core has them.
    public func flush() async {
        cancelTimer()
        while let current = inflight { await current.task.value }
        guard dirty, !disposed else { return }
        guard let job = prepareSave() else {
            tryApplyServer()
            return
        }
        let id = takeId()
        let run = Task { [weak self] in
            guard let self else { return }
            await self.perform(job)
            self.finish(id)
        }
        inflight = (id, run)
        await run.value
        tryApplyServer()
    }

    /// Saves what the page holds now, then runs `work` (a change recorded outside the editor) and
    /// makes every later save wait for it. Call it before the next edit is applied to the page: the
    /// save reads the page at once unless an earlier save is still running. A failure of `work`
    /// does not stop later saves.
    public func saveThen(_ work: @escaping @MainActor () async throws -> Void) {
        cancelTimer()
        let before = inflight?.task
        let early: Job? = before == nil ? prepareSave() : nil
        let id = takeId()
        let gate = Task { [weak self] in
            guard let self else { return }
            if let before { await before.value }
            if let job = early ?? (before == nil ? nil : self.prepareSave()) {
                await self.perform(job)
            }
            try? await work()
            self.finish(id)
            self.tryApplyServer()
        }
        inflight = (id, gate)
    }

    /// Makes every later save wait until `work` settles (it is never cancelled, and a failure does
    /// not stop the saves). The new character from "Create New" is recorded between the typed name,
    /// saved just before, and the Dialogue line that follows, so Undo takes them back one at a time.
    public func hold(_ work: @escaping @MainActor () async throws -> Void) {
        let before = inflight?.task
        let id = takeId()
        let gate = Task { [weak self] in
            guard let self else { return }
            if let before { await before.value }
            try? await work()
            self.finish(id)
            self.tryApplyServer()
        }
        inflight = (id, gate)
    }

    /// A document read from the core.
    public func receiveServer(seq: Double, snapshot: ScreenplaySnapshot) {
        latest = (seq, snapshot)
        tryApplyServer()
    }

    private func tryApplyServer() {
        if dirty || inflight != nil || disposed { return }
        guard let s = latest, s.seq >= ackSeq else { return }
        if ScreenplayModel.equal(s.snap, baseline) { return }
        baseline = s.snap
        ackSeq = s.seq
        deps.apply(s.snap)
    }

    public func dispose() {
        disposed = true
        cancelTimer()
    }
}

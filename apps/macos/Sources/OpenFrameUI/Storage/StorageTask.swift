// One long storage action (vault backup, export, restore, move; project and library move) in a
// sheet: a confirmation first, then progress with a real Cancel, then the result or the plain
// error (Windows: features/storage/StorageTask.tsx). The task itself runs in Rust and reports
// through `task` events of kind storage.*; this only follows it.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class StorageTaskModel: Identifiable {
    enum Stage: Equatable {
        case confirm
        case running
        case completed(String)
        case failed(String)
        case cancelled
        /// The end was never heard: the core stopped, or nothing came for a long time.
        case lost
    }

    let id = UUID()
    let title: String
    /// Shown before anything starts.
    let confirm: [String]
    let confirmLabel: String
    /// A project or library move: the core closes and reopens the open project.
    let relocates: Bool
    let task = PackageTaskFollower()
    private(set) var startError: String?
    /// Set once the end was handled (places read again, the window shows the project again).
    private(set) var settled = false
    /// The end was not heard (see `Stage.lost`).
    private(set) var lost = false

    /// How long the task may stay silent before the window stops waiting for its end. Progress
    /// comes often during a move, so only a task whose end was missed stays silent this long.
    @ObservationIgnored let silence: Duration
    @ObservationIgnored private var lastHeard = ContinuousClock.now
    @ObservationIgnored private var streamEnded = false
    @ObservationIgnored private var released = false
    @ObservationIgnored private weak var app: AppModel?
    @ObservationIgnored private let startOp: (Core) async throws -> String
    @ObservationIgnored private let doneText: (PackageTaskFollower) -> String
    @ObservationIgnored private let onFinished: (@MainActor () async -> Void)?

    init(app: AppModel, title: String, confirm: [String], confirmLabel: String, relocates: Bool = false,
         start: @escaping (Core) async throws -> String, done: @escaping (PackageTaskFollower) -> String,
         onFinished: (@MainActor () async -> Void)? = nil, silence: Duration = .seconds(600)) {
        self.app = app
        self.silence = silence
        self.title = title
        self.confirm = confirm
        self.confirmLabel = confirmLabel
        self.relocates = relocates
        self.startOp = start
        self.doneText = done
        self.onFinished = onFinished
    }

    var stage: Stage {
        guard task.taskId != nil else { return task.starting ? .running : .confirm }
        switch task.state {
        case "completed": return .completed(doneText(task))
        case "failed": return .failed(task.errorMessage ?? "This did not finish. Nothing was changed.")
        case "cancelled": return .cancelled
        default: return lost ? .lost : .running
        }
    }

    var running: Bool { task.running && !lost }
    var finished: Bool { task.finished || lost }

    func start() async {
        guard let app, let core = app.core, task.taskId == nil, !task.starting else { return }
        startError = nil
        settled = false
        lost = false
        released = false
        streamEnded = false
        lastHeard = .now
        if relocates {
            // The move closes the project: what the writer typed is stored first.
            await ScreenplayWorkspaceModel.flushAll()
            app.beginRelocation()
        }
        // Its own view of the events: the stream ends when the core stops.
        let events = core.events()
        let watch = Task { [weak self] in
            for await event in events {
                if case let .task(t) = event, t.taskId == self?.task.taskId { self?.lastHeard = .now }
            }
            self?.streamEnded = true
        }
        do {
            try await task.begin(core: core) { try await self.startOp(core) }
        } catch {
            watch.cancel()
            startError = CoreFailure(asCore: error).message
            await release()
            settled = true
            return
        }
        lastHeard = .now
        Task { await self.settle(watch) }
    }

    /// Waits for the end, then reads the places again. A move can take a long time and sends
    /// progress while it runs. The core has no call that reports a task's state, so a missed end
    /// (the core stopped, or its last event was dropped) is noticed by the stream ending or by a
    /// long silence. The move counter is always given back, so a later close goes Home.
    private func settle(_ watch: Task<Void, Never>) async {
        while !task.finished {
            if streamEnded || ContinuousClock.now - lastHeard > silence {
                lost = true
                break
            }
            try? await Task.sleep(for: .milliseconds(100))
        }
        watch.cancel()
        await release()
        await onFinished?()
        settled = true
    }

    /// Gives back the move counter once.
    private func release() async {
        guard !released else { return }
        released = true
        if relocates, let app { await app.endRelocation() }
    }

    func cancel() { task.cancel(core: app?.core) }

    /// Waits until the end was handled (tests).
    func waitUntilSettled(seconds: Double = 120) async {
        let deadline = Date().addingTimeInterval(seconds)
        while !settled && Date() < deadline { try? await Task.sleep(for: .milliseconds(25)) }
    }
}

struct StorageTaskSheet: View {
    let model: StorageTaskModel
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        IXSheetFrame(title: model.title, width: 560, idPrefix: "storage-task") {
            VStack(alignment: .leading, spacing: 10) {
                switch model.stage {
                case .confirm:
                    ForEach(Array(model.confirm.enumerated()), id: \.offset) { _, line in
                        Text(line).font(.callout).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
                    }
                    if let e = model.startError { IXBanner(.err, e).accessibilityIdentifier("storage-task-error") }
                case .running:
                    progress
                case let .completed(text):
                    IXBanner(.ok, text).textSelection(.enabled).accessibilityIdentifier("storage-task-done")
                case let .failed(text):
                    IXBanner(.err, text).textSelection(.enabled).accessibilityIdentifier("storage-task-failed")
                case .cancelled:
                    IXBanner(.info, "Cancelled. Nothing was changed.").accessibilityIdentifier("storage-task-cancelled")
                case .lost:
                    IXBanner(.info, "OpenFrame did not hear how this ended. Check the places in Storage before you try again.")
                        .accessibilityIdentifier("storage-task-lost")
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } footer: {
            if model.finished {
                Button("Close", action: close)
                    .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                    .accessibilityIdentifier("storage-task-close")
            } else {
                Button("Cancel", action: close)
                    .keyboardShortcut(.cancelAction)
                    .disabled(model.running)
                    .accessibilityIdentifier("storage-task-dismiss")
                Button(model.confirmLabel) { Task { await model.start() } }
                    .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                    .disabled(model.running || model.task.taskId != nil)
                    .accessibilityIdentifier("storage-task-confirm")
            }
        }
        // While it runs the sheet stays: Cancel inside the progress stops the task first.
        .interactiveDismissDisabled(model.running)
    }

    private var progress: some View {
        let t = model.task
        return VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(t.message).font(.callout.weight(.semibold))
                Spacer()
                Text("\(t.percent)%").font(.caption).foregroundStyle(.secondary)
            }
            ProgressView(value: Double(t.percent), total: 100)
                .accessibilityLabel("Progress")
                .accessibilityValue("\(t.percent) percent")
            HStack {
                Text("Nothing changes until the copy is checked. Cancel leaves everything where it was.")
                    .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                Spacer()
                Button("Cancel") { model.cancel() }
                    .controlSize(.small)
                    .disabled(t.taskId == nil)
                    .accessibilityIdentifier("storage-task-cancel")
            }
        }
        .padding(12)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Storage task progress")
        .accessibilityIdentifier("storage-task-progress")
    }
}

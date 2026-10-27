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

    @ObservationIgnored private weak var app: AppModel?
    @ObservationIgnored private let startOp: (Core) async throws -> String
    @ObservationIgnored private let doneText: (PackageTaskFollower) -> String
    @ObservationIgnored private let onFinished: (@MainActor () async -> Void)?

    init(app: AppModel, title: String, confirm: [String], confirmLabel: String, relocates: Bool = false,
         start: @escaping (Core) async throws -> String, done: @escaping (PackageTaskFollower) -> String,
         onFinished: (@MainActor () async -> Void)? = nil) {
        self.app = app
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
        default: return .running
        }
    }

    var running: Bool { task.running }
    var finished: Bool { task.finished }

    func start() async {
        guard let app, let core = app.core, task.taskId == nil, !task.starting else { return }
        startError = nil
        settled = false
        if relocates { app.beginRelocation() }
        do {
            try await task.begin(core: core) { try await self.startOp(core) }
        } catch {
            startError = CoreFailure(asCore: error).message
            if relocates { await app.endRelocation() }
            settled = true
            return
        }
        Task { await self.settle() }
    }

    /// Waits for the end, then reads the places again. A move can take a long time: no deadline
    /// short enough to cut it off, and nothing here blocks the main thread.
    private func settle() async {
        while !task.finished { try? await Task.sleep(for: .milliseconds(100)) }
        if relocates, let app { await app.endRelocation() }
        await onFinished?()
        settled = true
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

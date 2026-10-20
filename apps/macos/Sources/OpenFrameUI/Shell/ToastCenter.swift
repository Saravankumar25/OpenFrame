// Toasts (Windows: app/toast.tsx): a dark pill at the bottom centre with plain words and, for a
// change that can be undone, an Undo button that calls the core's history.undo. They go away on
// their own and VoiceOver reads them out. There is no notification centre.

import AppKit
import Observation
import SwiftUI
import OpenFrameCore

struct ToastItem: Identifiable, Equatable {
    enum Kind { case info, error }
    let id: Int
    var kind: Kind
    var text: String
    var actionLabel: String?
    /// Not part of equality: closures cannot be compared.
    var run: (@MainActor () async -> Void)?

    static func == (a: ToastItem, b: ToastItem) -> Bool {
        a.id == b.id && a.text == b.text && a.kind == b.kind && a.actionLabel == b.actionLabel
    }
}

@MainActor
@Observable
final class ToastCenter {
    static let shared = ToastCenter()

    private(set) var items: [ToastItem] = []
    private var sequence = 0
    private var timers: [Int: Task<Void, Never>] = [:]

    /// Show a message. Like Windows, at most three are kept (the newest stay), and a message
    /// goes after 5 seconds (8 for an error).
    @discardableResult
    func show(
        _ text: String, kind: ToastItem.Kind = .info, seconds: Double? = nil,
        actionLabel: String? = nil, run: (@MainActor () async -> Void)? = nil
    ) -> Int {
        sequence += 1
        let item = ToastItem(id: sequence, kind: kind, text: text, actionLabel: actionLabel, run: run)
        for old in items.dropLast(2) { cancelTimer(old.id) }
        items = Array(items.suffix(2)) + [item]
        let id = item.id
        timers[id] = Task { [weak self] in
            try? await Task.sleep(for: .seconds(seconds ?? (kind == .error ? 8 : 5)))
            guard !Task.isCancelled else { return }
            self?.dismiss(id)
        }
        AccessibilityNotification.Announcement(text).post()
        return id
    }

    /// A finished change with an Undo button (Windows `toast.undoable`).
    @discardableResult
    func undoable(_ text: String, app: AppModel, store: StoreSel = .project) -> Int {
        show(text, actionLabel: "Undo") { [weak app] in
            guard let app else { return }
            await app.undoFromToast(store: store)
        }
    }

    func dismiss(_ id: Int) {
        cancelTimer(id)
        items.removeAll { $0.id == id }
    }

    func dismissAll() {
        for item in items { cancelTimer(item.id) }
        items = []
    }

    func perform(_ item: ToastItem) async {
        dismiss(item.id)
        await item.run?()
    }

    private func cancelTimer(_ id: Int) {
        timers[id]?.cancel()
        timers[id] = nil
    }
}

extension AppModel {
    /// Undo from a toast. Says what was undone, as the Windows toast does, and tells the person
    /// plainly when there was nothing to undo.
    func undoFromToast(store: StoreSel) async {
        guard let core else { return }
        do {
            let step = try await core.historyUndo(StoreArgs(store: store))
            ToastCenter.shared.show(step.label.map { "Undid: \($0)" } ?? "Nothing to undo.")
        } catch {
            ToastCenter.shared.show(CoreFailure(asCore: error).message, kind: .error)
        }
    }
}

/// The toasts of the window. Place with `.overlay(alignment: .bottom)` so only the pills take clicks.
struct ToastHost: View {
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private let center = ToastCenter.shared

    var body: some View {
        VStack(spacing: 8) {
            ForEach(center.items) { item in
                HStack(spacing: 12) {
                    Text(item.text)
                        .font(.callout)
                        .lineLimit(3)
                        .fixedSize(horizontal: false, vertical: true)
                    if let label = item.actionLabel {
                        Button(label) { Task { await center.perform(item) } }
                            .buttonStyle(.plain)
                            .font(.callout.weight(.semibold))
                            .foregroundStyle(theme["accent-soft"])
                            .accessibilityIdentifier("toast-action")
                    }
                    Button { center.dismiss(item.id) } label: {
                        Label("Dismiss", systemImage: "xmark").font(.caption.weight(.semibold))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Dismiss")
                }
                .foregroundStyle(theme["panel"])
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
                .background(item.kind == .error ? theme["red-t"] : theme["ink"], in: Capsule())
                .shadow(color: .black.opacity(0.2), radius: 8, y: 3)
                .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("toast")
            }
        }
        .padding(.bottom, 28)
        .animation(reduceMotion ? nil : .spring(response: 0.3, dampingFraction: 1), value: center.items)
    }
}

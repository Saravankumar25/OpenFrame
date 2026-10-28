// Settings > Offline AI > Assistant memory (Windows: workspaces/settings/AssistantMemory.tsx).
// What the assistant remembers about the person in the open project: facts and preferences they
// stated outright, or answers they confirmed. Kept on this Mac, never shared, each entry
// deletable. The list and the delete come from the core (`ai.memory_list`, `ai.memory_delete`),
// which needs an open project.

import Observation
import OpenFrameCore
import SwiftUI

enum AssistantMemoryText {
    static let title = "Assistant memory"
    static let empty = "Nothing is remembered yet. The assistant keeps a fact or preference only when you state it outright, or confirm one of its answers."
    static let hint = "Saved on this Mac, in this project, for you only, and only when you state a fact or confirm an answer. Delete an entry and the assistant forgets it."
    static let noProject = "Open a project to see what the assistant remembers in it."
    static let forgotten = "Forgotten."

    static func deleteLabel(_ e: AiMemoryEntry) -> String { "Delete: \(e.text)" }

    /// "Fact, saved Today 10:41" or "Preference, saved 12 Sep 09:02".
    static func meta(_ e: AiMemoryEntry, now: Date = Date()) -> String {
        "\(e.kind == .preference ? "Preference" : "Fact"), saved \(PageText.dayAndTime(ms: e.createdAt, now: now))"
    }
}

@MainActor
@Observable
final class AssistantMemoryModel {
    let app: AppModel
    /// `nil` while loading.
    private(set) var entries: [AiMemoryEntry]?
    private(set) var error: String?
    /// The entry being deleted now.
    private(set) var deleting: String?
    /// Tests answer the core calls themselves; the app uses the running core.
    @ObservationIgnored var invoker: (any CoreInvoking)?

    init(app: AppModel) {
        self.app = app
    }

    private var core: (any CoreInvoking)? { invoker ?? app.core }

    /// The memory belongs to a project: without one open there is nothing to list.
    var hasProject: Bool {
        if case .project = app.phase { return true }
        return false
    }

    func load() async {
        guard hasProject, let core else {
            entries = nil
            return
        }
        do {
            entries = try await core.aiMemoryList().entries
            error = nil
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    /// Forget one entry, then read the list again.
    func delete(_ e: AiMemoryEntry) async {
        guard let core, deleting == nil else { return }
        deleting = e.id
        defer { deleting = nil }
        do {
            _ = try await core.aiMemoryDelete(AiMemoryDeleteArgs(id: e.id))
            ToastCenter.shared.show(AssistantMemoryText.forgotten)
        } catch {
            ToastCenter.shared.show(CoreFailure(asCore: error).message, kind: .error)
        }
        await load()
    }
}

struct AssistantMemoryCard: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: AssistantMemoryModel?

    /// Reload when the project changes or the memory table does.
    private var reloadKey: String {
        var project = ""
        if case .project(let p) = app.phase { project = p.id }
        return "\(project):\(app.changes.key(["ai_memory"]))"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(AssistantMemoryText.title).font(.headline).accessibilityAddTraits(.isHeader)
            if let model {
                content(model)
            }
            Text(AssistantMemoryText.hint)
                .font(.caption).foregroundStyle(theme["muted"])
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("ai-memory-hint")
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(AssistantMemoryText.title)
        .accessibilityIdentifier("ai-memory-card")
        .onAppear { if model == nil { model = AssistantMemoryModel(app: app) } }
        .task(id: reloadKey) {
            if model == nil { model = AssistantMemoryModel(app: app) }
            await model?.load()
        }
    }

    @ViewBuilder private func content(_ m: AssistantMemoryModel) -> some View {
        if !m.hasProject {
            Text(AssistantMemoryText.noProject)
                .foregroundStyle(theme["muted"])
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("ai-memory-no-project")
        } else if let error = m.error, m.entries == nil {
            SettingsErrorText(message: error)
        } else if let entries = m.entries {
            if entries.isEmpty {
                Text(AssistantMemoryText.empty)
                    .foregroundStyle(theme["muted"])
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("ai-memory-empty")
            } else {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(entries, id: \.id) { e in
                        row(e, m)
                        if e.id != entries.last?.id { Divider() }
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Remembered entries")
                .accessibilityIdentifier("ai-memory-list")
            }
        } else {
            ProgressView().controlSize(.small)
        }
    }

    private func row(_ e: AiMemoryEntry, _ m: AssistantMemoryModel) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(verbatim: e.text).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
                Text(AssistantMemoryText.meta(e)).font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer(minLength: 8)
            Button("Delete") { Task { await m.delete(e) } }
                .buttonStyle(.ofStandard)
                .disabled(m.deleting != nil)
                .accessibilityLabel(AssistantMemoryText.deleteLabel(e))
                .accessibilityIdentifier("ai-memory-delete-\(e.id)")
        }
        .padding(.vertical, 6)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ai-memory-entry")
    }
}

// Drafts (Mac phase M4): the list of drafts of the screenplay with rename, lock and unlock,
// delete, a new draft from any draft, the revision label and colour, and the recovery history.
// Behaviour follows drafts.tsx and lock.tsx on Windows. The core owns every rule: this file asks
// for an operation and shows its answer.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class DraftsPanelModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?
    @ObservationIgnored var say: (String) -> Void = { _ in }

    var history: [ScreenplayHistoryPoint] = []
    var lockSummary: ScreenplayLockSummary?
    var busy = false
    /// Set once the model has its app and workspace; a sheet's own task waits for it.
    var attached = false

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel, say: @escaping (String) -> Void) {
        self.app = app
        self.workspace = workspace
        self.say = say
        attached = true
    }

    var drafts: [ScreenplayDraftDto] { workspace?.overview?.drafts ?? [] }
    var sortedDrafts: [ScreenplayDraftDto] { drafts.sorted { $0.createdAt > $1.createdAt } }
    var current: ScreenplayDraftDto? { drafts.first { $0.isCurrent } }
    var openDraftId: String? { workspace?.draft?.id }
    var permissions: ScreenplayPermissions? { workspace?.overview?.permissions }
    private var core: Core? { app?.core }

    /// A name for a new draft that no draft uses yet: "Draft 3".
    func suggestedName() -> String {
        let used = Set(drafts.map { $0.name.lowercased() })
        for n in (drafts.count + 1)..<(drafts.count + 100) where !used.contains("Draft \(n)".lowercased()) {
            return "Draft \(n)"
        }
        return ""
    }

    /// The next free "Revision A".
    func suggestedRevisionLabel() -> String {
        let used = Set(drafts.compactMap { $0.revisionLabel?.lowercased() })
        for i in 0..<26 {
            let label = "Revision \(Character(UnicodeScalar(65 + i)!))"
            if !used.contains(label.lowercased()) { return label }
        }
        return "Revision"
    }

    // MARK: Operations. Each returns the message to show when the core refuses, nil on success.

    func open(_ draft: ScreenplayDraftDto) async {
        await workspace?.switchDraft(draft.id)
        await reloadHistory()
    }

    func rename(_ draft: ScreenplayDraftDto, name: String, note: String) async -> String? {
        await run("The draft could not be renamed.") { core in
            _ = try await core.screenplayRenameDraft(ScreenplayRenameDraftArgs(draftId: draft.id, name: name, note: note))
        }
    }

    func delete(_ draft: ScreenplayDraftDto) async -> String? {
        await workspace?.flush()
        return await run("The draft could not be deleted.") { core in
            _ = try await core.screenplayDeleteDraft(ScreenplayDraftArgs(draftId: draft.id))
            await MainActor.run { self.say("Moved \(PanelText.quoted(draft.name)) to Recently Deleted") }
        }
    }

    func newDraft(from source: ScreenplayDraftDto, name: String, note: String?) async -> String? {
        await workspace?.flush()
        return await run("The draft could not be created.") { core in
            let d = try await core.screenplayNewDraft(ScreenplayNewDraftArgs(
                sourceDraftId: source.id, name: name, note: note?.trimmingCharacters(in: .whitespacesAndNewlines).panelOptional, makeCurrent: true))
            await MainActor.run { self.say("Created draft \(PanelText.quoted(d.name))") }
        }
    }

    func loadLockSummary(_ draft: ScreenplayDraftDto) async {
        lockSummary = nil
        guard let core else { return }
        lockSummary = try? await core.screenplayLockSummary(ScreenplayDraftArgs(draftId: draft.id))
    }

    func lock(_ draft: ScreenplayDraftDto) async -> String? {
        await workspace?.flush()
        return await run("The draft could not be locked.") { core in
            _ = try await core.screenplayLockDraft(ScreenplayDraftArgs(draftId: draft.id))
            await MainActor.run { self.say("Locked \(PanelText.quoted(draft.name)) as the Shooting Draft") }
        }
    }

    func unlock(_ draft: ScreenplayDraftDto) async -> String? {
        await run("The draft could not be unlocked.") { core in
            _ = try await core.screenplayUnlockDraft(ScreenplayDraftArgs(draftId: draft.id))
            await MainActor.run { self.say("Unlocked \(PanelText.quoted(draft.name))") }
        }
    }

    /// Start a revision of a locked draft: a new draft with a label and a colour.
    func startRevision(from source: ScreenplayDraftDto, label: String, colour: String, reason: String, date: String) async -> String? {
        await workspace?.flush()
        return await run("The revision could not be started.") { core in
            let d = try await core.screenplayStartRevision(ScreenplayStartRevisionArgs(
                draftId: source.id, label: label, color: colour,
                reason: reason.trimmingCharacters(in: .whitespacesAndNewlines).panelOptional, date: date.panelOptional))
            await MainActor.run { self.say("Started \(d.revisionLabel ?? d.name). \(source.name) stays locked.") }
        }
    }

    /// Change the label, colour, reason or date of a revision draft.
    func updateRevision(_ draft: ScreenplayDraftDto, label: String, colour: String, reason: String, date: String?) async -> String? {
        await run("The revision could not be changed.") { core in
            _ = try await core.screenplayUpdateRevision(ScreenplayUpdateRevisionArgs(
                draftId: draft.id, label: label, color: colour, reason: reason, date: date?.panelOptional))
        }
    }

    func reloadHistory() async {
        guard let core, let id = openDraftId else { history = []; return }
        history = (try? await core.screenplayHistoryPoints(ScreenplayDraftArgs(draftId: id))) ?? []
    }

    func restore(_ point: ScreenplayHistoryPoint) async -> String? {
        await workspace?.flush()
        return await run("The history point could not be restored.") { core in
            let d = try await core.screenplayRestoreHistoryPoint(ScreenplayHistoryPointArgs(historyPointId: point.id))
            await MainActor.run { self.say("Restored as a new draft: \(d.name)") }
        }
    }

    private func run(_ fallback: String, _ work: @escaping (Core) async throws -> Void) async -> String? {
        guard let core else { return fallback }
        busy = true
        defer { busy = false }
        do {
            try await work(core)
            await workspace?.loadOverview()
            await reloadHistory()
            return nil
        } catch {
            if !PanelText.isValidation(error) { app?.show("The draft was not changed", error) }
            return PanelText.message(error, fallback: fallback)
        }
    }
}

extension String {
    /// The string, or nil when it is empty after trimming.
    var panelOptional: String? {
        let t = trimmingCharacters(in: .whitespacesAndNewlines)
        return t.isEmpty ? nil : self
    }
}

// MARK: - The drafts sheet

private enum DraftDialog: Identifiable {
    case rename(ScreenplayDraftDto)
    case delete(ScreenplayDraftDto)
    case lock(ScreenplayDraftDto)
    case newFrom(ScreenplayDraftDto)
    case revision(ScreenplayDraftDto)
    case startRevision(ScreenplayDraftDto)

    var id: String {
        switch self {
        case let .rename(d): "rename-\(d.id)"
        case let .delete(d): "delete-\(d.id)"
        case let .lock(d): "lock-\(d.id)"
        case let .newFrom(d): "new-\(d.id)"
        case let .revision(d): "revision-\(d.id)"
        case let .startRevision(d): "start-\(d.id)"
        }
    }
}

struct DraftsSheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @Environment(\.dismiss) private var dismiss
    @State private var model = DraftsPanelModel()
    @State private var dialog: DraftDialog?

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            PanelTitle("Drafts")
            HStack {
                Spacer(minLength: 0)
                Button("Compare\u{2026}") { state.sheet = .compare(a: nil, b: nil) }
                    .disabled(model.drafts.count < 2)
                    .accessibilityIdentifier("drafts-compare")
                if model.permissions?.edit == true, let source = model.openDraftId.flatMap({ id in model.drafts.first { $0.id == id } }) ?? model.current {
                    Button("New Draft\u{2026}") { dialog = .newFrom(source) }
                        .accessibilityIdentifier("drafts-new")
                }
                Spacer(minLength: 0)
            }
            if let current = model.current, current.lineage.count > 1 {
                Text(current.lineage.map(\.name).joined(separator: " \u{2192} "))
                    .font(.callout).foregroundStyle(theme["muted"])
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)
                    .accessibilityLabel("Draft lineage: \(current.lineage.map(\.name).joined(separator: ", then "))")
            }
            List {
                ForEach(model.sortedDrafts, id: \.id) { d in draftRow(d) }
            }
            .frame(minHeight: 180)
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
            .accessibilityIdentifier("drafts-list")

            HistoryPointsView(model: model, workspace: workspace, state: state)

            HStack {
                Spacer()
                Button("Done") { state.sheet = nil }.keyboardShortcut(.defaultAction)
            }
        }
        .padding(PageSpacing.large)
        .sheetSize(width: 600, height: 620)
        .background(theme["panel"])
        .task {
            model.attach(app, workspace, say: { state.say($0) })
            await model.reloadHistory()
        }
        .sheet(item: $dialog) { d in
            switch d {
            case let .rename(draft): RenameDraftSheet(model: model, draft: draft)
            case let .delete(draft): DeleteDraftSheet(model: model, draft: draft)
            case let .lock(draft): LockDraftSheet(model: model, draft: draft)
            case let .newFrom(draft): NewDraftSheet(model: model, source: draft)
            case let .revision(draft): RevisionDetailsSheet(model: model, draft: draft)
            case let .startRevision(draft): StartRevisionSheet(model: model, source: draft)
            }
        }
        .accessibilityIdentifier("drafts-sheet")
    }

    private func draftRow(_ d: ScreenplayDraftDto) -> some View {
        let permissions = model.permissions
        let canEdit = permissions?.edit == true
        return HStack(alignment: .top, spacing: 10) {
            if let swatch = PanelText.swatch(d.revisionColor) {
                Circle().fill(swatch).frame(width: 10, height: 10).padding(.top, 5).accessibilityHidden(true)
            }
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(d.name).fontWeight(.semibold)
                    if d.isCurrent { PanelChip(text: "Current", tone: .accent) }
                    PanelChip(text: PanelText.status(d), tone: d.status == .locked ? .green : d.status == .review ? .blue : d.status == .revision ? .purple : .plain)
                    if d.id == model.openDraftId { PanelChip(text: "Open") }
                }
                let parts = [PanelText.ago(d.createdAt), d.note, d.createdFromName.map { "from \($0)" }].compactMap { $0 }.filter { !$0.isEmpty }
                Text(parts.joined(separator: " \u{00B7} ")).font(.caption).foregroundStyle(theme["muted"])
                if d.status == .locked, let at = d.lockedAt {
                    Text("Locked \(PanelText.when(at))\(d.lockedByName.map { " by \($0)" } ?? "")")
                        .font(.caption).foregroundStyle(theme["muted"])
                }
            }
            Spacer(minLength: 8)
            Button("Open") { Task { await model.open(d) } }
                .disabled(d.id == model.openDraftId)
                .accessibilityLabel("Open \(d.name)")
            Menu {
                if canEdit { Button("Rename\u{2026}") { dialog = .rename(d) } }
                if canEdit { Button("New Draft From This\u{2026}") { dialog = .newFrom(d) } }
                if !d.isCurrent, let current = model.current {
                    Button("Compare With Current") { state.sheet = .compare(a: d.id, b: current.id) }
                }
                if canEdit { Button("Start Review\u{2026}") { state.sheet = .review(startFor: d.id) } }
                if d.status == .revision, canEdit { Button("Revision Label and Colour\u{2026}") { dialog = .revision(d) } }
                if permissions?.lock == true, d.status != .locked { Button("Lock as Shooting Draft\u{2026}") { dialog = .lock(d) } }
                if d.status == .locked, canEdit { Button("Start Revision\u{2026}") { dialog = .startRevision(d) } }
                if d.status == .locked, permissions?.unlock == true {
                    Button("Unlock Draft") { Task { _ = await model.unlock(d) } }
                }
                if permissions?.delete == true {
                    Divider()
                    Button("Delete\u{2026}", role: .destructive) { dialog = .delete(d) }
                }
            } label: {
                Image(systemName: "ellipsis.circle")
            }
            .menuStyle(.borderlessButton)
            .fixedSize()
            .accessibilityLabel("Actions for \(d.name)")
        }
        .padding(.vertical, 3)
        .accessibilityElement(children: .contain)
    }
}

// MARK: Recovery history

/// The automatic history points of the open draft; restoring one makes a new draft.
struct HistoryPointsView: View {
    let model: DraftsPanelModel
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Recovery History\(workspace.draft.map { ": \($0.name)" } ?? "")")
                .font(.headline)
                .accessibilityAddTraits(.isHeader)
            if model.history.isEmpty {
                Text("No automatic points yet.").foregroundStyle(theme["muted"])
            } else {
                List {
                    ForEach(model.history, id: \.id) { h in
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text("\(PanelText.when(h.createdAt)): \(h.reason == "draft_created" ? "draft created" : "automatic point")")
                                Text(PanelText.plural(Int(h.sceneCount), "scene")).font(.caption).foregroundStyle(theme["muted"])
                            }
                            Spacer()
                            if model.permissions?.edit == true {
                                Button("Restore") {
                                    Task { _ = await model.restore(h) }
                                }
                                .accessibilityLabel("Restore the point from \(PanelText.when(h.createdAt))")
                            }
                        }
                    }
                }
                .frame(height: 120)
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
                .accessibilityIdentifier("history-list")
            }
            Text("Automatic history is for recovery. It is not a deliverable draft. Restoring creates a new draft; nothing is replaced.")
                .font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("history-points")
    }
}

// MARK: Dialogs

struct RenameDraftSheet: View {
    let model: DraftsPanelModel
    let draft: ScreenplayDraftDto
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var note = ""
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "Rename draft", sub: "Renaming changes only the label, never the text.") {
            VStack(alignment: .leading, spacing: 10) {
                PanelField(label: "Draft name") {
                    TextField("Draft name", text: $name).textFieldStyle(.roundedBorder).accessibilityIdentifier("draft-name")
                }
                PanelField(label: "Note (optional)") { PanelTextArea(label: "Note", text: $note) }
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Save") {
                Task {
                    error = await model.rename(draft, name: name, note: note)
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || model.busy)
        }
        .onAppear { name = draft.name; note = draft.note ?? "" }
    }
}

struct DeleteDraftSheet: View {
    let model: DraftsPanelModel
    let draft: ScreenplayDraftDto
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme

    var body: some View {
        PanelSheetFrame(title: "Delete \(PanelText.quoted(draft.name))?") {
            VStack(alignment: .leading, spacing: 10) {
                Text("The draft moves to Recently Deleted and can be restored. Your other drafts are not affected.")
                    .fixedSize(horizontal: false, vertical: true)
                if draft.isCurrent {
                    PanelBanner(tone: .warn, text: "This is the current draft. Make another draft current before deleting it.")
                } else {
                    Text("The current draft cannot be deleted until another draft is made current. A locked draft cannot be deleted while it is the only production baseline.")
                        .font(.callout).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Delete Draft", role: .destructive) {
                Task {
                    if await model.delete(draft) == nil { dismiss() }
                }
            }
            .disabled(draft.isCurrent || model.busy)
        }
    }
}

/// "Lock this draft as the Shooting Draft?" with the summary the core reports.
struct LockDraftSheet: View {
    let model: DraftsPanelModel
    let draft: ScreenplayDraftDto
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "Lock this draft as the Shooting Draft?", sub: "Confirm the details, then lock.", width: 520) {
            VStack(alignment: .leading, spacing: 10) {
                if let s = model.lockSummary {
                    Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 14, verticalSpacing: 6) {
                        row("Draft name", s.draftName, bold: true)
                        row("Current version", "\(s.isCurrent ? "Current draft" : "Not the current draft") \u{00B7} \(s.status.rawValue)")
                        GridRow {
                            Text("Unresolved review notes").foregroundStyle(theme["muted"])
                            VStack(alignment: .leading, spacing: 3) {
                                if s.openComments == 0 { Text("None") } else { PanelChip(text: "\(Int(s.openComments)) open", tone: .yellow) }
                                ForEach(Array(s.openCommentSamples.enumerated()), id: \.offset) { _, line in
                                    Text(line).font(.caption).foregroundStyle(theme["muted"])
                                }
                            }
                        }
                        row("Number of scenes", String(Int(s.sceneCount)))
                        row("Last modified", PanelText.when(s.lastModified))
                    }
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("lock-summary")
                } else {
                    ProgressView().controlSize(.small)
                }
                Text("Locking is a safety state, not a destructive action. The draft stays readable and exportable. Future edits will start a post-lock revision so the production baseline cannot be changed by accident.")
                    .fixedSize(horizontal: false, vertical: true)
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Lock as Shooting Draft") {
                Task {
                    error = await model.lock(draft)
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(model.lockSummary == nil || model.busy)
        }
        .task(id: model.attached) { await model.loadLockSummary(draft) }
    }

    private func row(_ k: String, _ v: String, bold: Bool = false) -> some View {
        GridRow {
            Text(k).foregroundStyle(theme["muted"])
            Text(v).fontWeight(bold ? .semibold : .regular)
        }
    }
}

/// A new draft from a chosen draft. The source draft stays exactly as it is.
struct NewDraftSheet: View {
    let model: DraftsPanelModel
    let source: ScreenplayDraftDto
    @Environment(\.dismiss) private var dismiss
    @State private var from = ""
    @State private var name = ""
    @State private var note = ""
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "New Draft") {
            VStack(alignment: .leading, spacing: 10) {
                if model.drafts.count > 1 {
                    PanelField(label: "Create from") {
                        Picker("Create from", selection: $from) {
                            ForEach(model.drafts, id: \.id) { d in
                                Text(d.isCurrent ? "\(d.name) (current)" : d.name).tag(d.id)
                            }
                        }
                        .labelsHidden()
                        .accessibilityLabel("Create from")
                    }
                }
                PanelField(label: "Draft name") {
                    TextField("Draft name", text: $name).textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Draft name").accessibilityIdentifier("new-draft-name")
                }
                PanelField(label: "Note (optional)") { PanelTextArea(label: "Note", text: $note) }
                Text("The source draft stays exactly as it is. The new draft becomes a separate version.")
                    .font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Create Draft") {
                Task {
                    guard let chosen = model.drafts.first(where: { $0.id == from }) else { return }
                    error = await model.newDraft(from: chosen, name: name, note: note)
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || model.busy)
        }
        .onAppear { from = source.id; name = model.suggestedName() }
    }
}

/// The label, colour, reason and date of a revision draft.
struct RevisionDetailsSheet: View {
    let model: DraftsPanelModel
    let draft: ScreenplayDraftDto
    @Environment(\.dismiss) private var dismiss
    @State private var label = ""
    @State private var colour = "Blue"
    @State private var reason = ""
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "Revision label and colour", sub: "Printed on revision pages. The colour is metadata and export styling only.") {
            VStack(alignment: .leading, spacing: 10) {
                PanelField(label: "Label") { TextField("Label", text: $label).textFieldStyle(.roundedBorder).accessibilityLabel("Label") }
                RevisionColourPicker(colour: $colour)
                PanelField(label: "Reason (optional)") { PanelTextArea(label: "Reason", text: $reason) }
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Save") {
                Task {
                    error = await model.updateRevision(draft, label: label, colour: colour, reason: reason, date: nil)
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(label.trimmingCharacters(in: .whitespaces).isEmpty || model.busy)
        }
        .onAppear {
            label = draft.revisionLabel ?? ""
            colour = draft.revisionColor ?? "Blue"
            reason = draft.revisionReason ?? ""
        }
    }
}

struct StartRevisionSheet: View {
    let model: DraftsPanelModel
    let source: ScreenplayDraftDto
    @Environment(\.dismiss) private var dismiss
    @State private var label = ""
    @State private var colour = "Blue"
    @State private var reason = ""
    @State private var date = Date()
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "Start Revision", sub: "Based on the locked \(source.name). The locked draft never changes.") {
            VStack(alignment: .leading, spacing: 10) {
                PanelField(label: "Label") { TextField("Label", text: $label).textFieldStyle(.roundedBorder).accessibilityLabel("Label") }
                RevisionColourPicker(colour: $colour)
                PanelField(label: "Revision date", hint: "Printed where the title page and header ask for the date.") {
                    DatePicker("Revision date", selection: $date, displayedComponents: .date).labelsHidden()
                }
                PanelField(label: "Reason (optional)") { PanelTextArea(label: "Reason", text: $reason) }
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Start Revision") {
                Task {
                    let f = DateFormatter()
                    f.calendar = Calendar(identifier: .gregorian)
                    f.locale = Locale(identifier: "en_US_POSIX")
                    f.dateFormat = "yyyy-MM-dd"
                    error = await model.startRevision(from: source, label: label, colour: colour, reason: reason, date: f.string(from: date))
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(label.trimmingCharacters(in: .whitespaces).isEmpty || model.busy)
        }
        .onAppear { label = model.suggestedRevisionLabel() }
    }
}

struct RevisionColourPicker: View {
    @Binding var colour: String

    var body: some View {
        PanelField(label: "Colour") {
            HStack(spacing: 8) {
                ForEach(PanelText.revisionColours, id: \.self) { c in
                    Button {
                        colour = c
                    } label: {
                        HStack(spacing: 5) {
                            Circle().fill(PanelText.swatch(c) ?? .gray).frame(width: 10, height: 10)
                            Text(c)
                        }
                    }
                    .buttonStyle(.bordered)
                    .tint(colour == c ? .accentColor : nil)
                    .accessibilityAddTraits(colour == c ? .isSelected : [])
                    .accessibilityLabel("\(c) revision colour")
                }
            }
        }
    }
}

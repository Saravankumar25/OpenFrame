// Private notes on a scene (Mac phase M4), as NotesPanel and PrivateNoteCard in panels.tsx on
// Windows: a note only its writer sees. The core keeps it apart from the script, the comments and
// the scene note, so it never reaches an export or a review package. The Scene tab lists the
// notes, adds one, edits one and deletes one.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class PrivateNotesModel {
    /// The kind of thing a note is attached to; the same value Windows writes.
    static let targetType = "screenplay_scene"

    @ObservationIgnored weak var app: AppModel?

    var sceneId: String?
    var notes: [PrivateNoteDto] = []
    /// The text of a note being added, or nil when no new note is open.
    var newBody: String?
    /// The note being edited, and its text.
    var editingId: String?
    var editBody = ""
    var error: String?

    init() {}

    func attach(_ app: AppModel) { self.app = app }

    private var core: Core? { app?.core }

    func load(sceneId: String?) async {
        if sceneId != self.sceneId {
            newBody = nil
            editingId = nil
            error = nil
        }
        self.sceneId = sceneId
        guard let core, let sceneId else { notes = []; return }
        let loaded = (try? await core.privateNoteList(ListPrivateNotesArgs(targetType: Self.targetType, targetId: sceneId))) ?? []
        if sceneId == self.sceneId { notes = loaded.sorted { $0.createdAt < $1.createdAt } }
    }

    func startAdd() { error = nil; newBody = "" }
    func cancelAdd() { newBody = nil; error = nil }

    var canSaveNew: Bool { !(newBody?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true) }

    func saveNew() async {
        guard let core, let sceneId, let body = newBody, canSaveNew else { return }
        do {
            _ = try await core.privateNoteCreate(CreatePrivateNoteArgs(targetType: Self.targetType, targetId: sceneId, body: body))
            newBody = nil
            error = nil
            await load(sceneId: sceneId)
        } catch {
            self.error = PanelText.message(error, fallback: "The private note could not be saved.")
        }
    }

    func startEdit(_ note: PrivateNoteDto) { error = nil; editingId = note.id; editBody = note.body }
    func cancelEdit() { editingId = nil; error = nil }

    var canSaveEdit: Bool { !editBody.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    func saveEdit() async {
        guard let core, let id = editingId, canSaveEdit else { return }
        do {
            _ = try await core.privateNoteUpdate(UpdatePrivateNoteArgs(id: id, body: editBody))
            editingId = nil
            error = nil
            await load(sceneId: sceneId)
        } catch {
            self.error = PanelText.message(error, fallback: "The private note could not be saved.")
        }
    }

    func delete(_ note: PrivateNoteDto) async {
        guard let core else { return }
        do {
            _ = try await core.privateNoteDelete(PrivateNoteIdArgs(id: note.id))
            if editingId == note.id { editingId = nil }
            await load(sceneId: sceneId)
        } catch {
            app?.show("Could not delete the private note", error)
        }
    }
}

/// The private notes of the scene shown in the Scene tab.
struct PrivateNotesSection: View {
    @Bindable var model: PrivateNotesModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Private notes").font(.subheadline.weight(.semibold)).foregroundStyle(theme["ink2"]).padding(.top, 4)
                .accessibilityAddTraits(.isHeader)
            if model.notes.isEmpty && model.newBody == nil {
                Text("No private notes on this scene.").foregroundStyle(theme["muted"])
            }
            ForEach(model.notes, id: \.id) { note in card(note) }
            if model.newBody != nil { newCard }
            if let error = model.error { PanelBanner(tone: .err, text: error) }
            if model.newBody == nil {
                Button("Add private note") { model.startAdd() }
                    .controlSize(.small)
                    .accessibilityIdentifier("private-note-add")
            }
            Text("Private notes are only for you. They are not printed and never leave the project in an export or a review package.")
                .font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("private-notes")
    }

    private func header<T: View>(@ViewBuilder _ trailing: () -> T) -> some View {
        HStack(spacing: 6) {
            Image(systemName: "lock").font(.caption).accessibilityHidden(true)
            Text("Private note").fontWeight(.semibold)
            PanelChip(text: "Only you", tone: .purple)
            Spacer(minLength: 0)
            trailing()
        }
    }

    @ViewBuilder private func card(_ note: PrivateNoteDto) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            header {
                Menu {
                    Button("Edit") { model.startEdit(note) }
                    Button("Delete", role: .destructive) { Task { await model.delete(note) } }
                } label: {
                    Label("Actions", systemImage: "ellipsis.circle")
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .accessibilityLabel("Private note actions")
                .accessibilityIdentifier("private-note-actions")
            }
            if model.editingId == note.id {
                PanelTextArea(label: "Private note", text: $model.editBody, minHeight: 64)
                HStack {
                    Spacer()
                    Button("Cancel") { model.cancelEdit() }
                    Button("Save") { Task { await model.saveEdit() } }
                        .disabled(!model.canSaveEdit)
                        .accessibilityIdentifier("private-note-save-edit")
                }
                .controlSize(.small)
            } else {
                Text(note.body).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("private-note-\(note.id)")
    }

    private var newCard: some View {
        VStack(alignment: .leading, spacing: 6) {
            header { EmptyView() }
            PanelTextArea(label: "Private note", text: Binding(get: { model.newBody ?? "" }, set: { model.newBody = $0 }), minHeight: 64)
            HStack {
                Spacer()
                Button("Cancel") { model.cancelAdd() }.accessibilityIdentifier("private-note-cancel")
                Button("Save") { Task { await model.saveNew() } }
                    .disabled(!model.canSaveNew)
                    .accessibilityIdentifier("private-note-save")
            }
            .controlSize(.small)
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("private-note-new")
    }
}

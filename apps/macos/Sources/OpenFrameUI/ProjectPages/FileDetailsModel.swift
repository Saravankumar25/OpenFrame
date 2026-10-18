// The open file in the details drawer: its display name and notes. The name is saved when the
// field is left or Return is pressed (Escape puts the old name back). Notes save by themselves
// after a short pause, and at once when the drawer closes, so typing is never lost. A value that
// changed for another reason (undo, redo) replaces only a field the person is not editing.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class FileDetailsModel {
    let files: FilesModel
    let id: String

    var name = ""
    var notes = ""
    private(set) var notesSaved = true
    var nameFocused = false
    var notesFocused = false

    @ObservationIgnored private var seeded = false
    @ObservationIgnored private var notesDirty = false
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private var inflight: Task<Void, Never>?
    @ObservationIgnored var delay: Duration = .milliseconds(600)

    init(files: FilesModel, id: String) {
        self.files = files
        self.id = id
    }

    var file: ProjectFileDto? { files.files.first { $0.id == id } }

    /// Takes the file's values into the fields the person is not editing.
    func sync() {
        guard let f = file else { return }
        if !seeded {
            name = f.displayName
            notes = f.notes ?? ""
            seeded = true
            return
        }
        if !nameFocused { name = f.displayName }
        if !notesFocused, !notesDirty { notes = f.notes ?? "" }
    }

    /// Renames when the field is left. An empty name puts the old one back.
    func commitName() async {
        guard let f = file else { return }
        let clean = name.trimmingCharacters(in: .whitespaces)
        if clean.isEmpty || clean == f.displayName {
            name = f.displayName
            return
        }
        if !(await files.rename(f, to: clean)) { name = file?.displayName ?? f.displayName }
    }

    func cancelName() {
        if let f = file { name = f.displayName }
    }

    func editNotes(_ value: String) {
        guard notes != value else { return }
        notes = value
        notesDirty = true
        notesSaved = false
        timer?.cancel()
        let wait = delay
        timer = Task { [weak self] in
            try? await Task.sleep(for: wait)
            guard !Task.isCancelled else { return }
            await self?.flushNotes()
        }
    }

    @discardableResult
    func flushNotes() async -> Bool {
        timer?.cancel()
        timer = nil
        if let inflight { await inflight.value }
        guard notesDirty, let f = file else { return !notesDirty }
        let text = notes
        notesDirty = false
        let run = Task { [weak self] in
            guard let self else { return }
            let ok = await files.saveNotes(f, notes: text)
            if ok {
                notesSaved = !notesDirty
            } else {
                notesDirty = true
            }
        }
        inflight = run
        await run.value
        inflight = nil
        return !notesDirty
    }

    func close() async {
        await commitName()
        await flushNotes()
        files.select(nil)
    }
}

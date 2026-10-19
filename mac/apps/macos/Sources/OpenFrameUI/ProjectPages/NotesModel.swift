// Notes and Tasks: the screen's model. It holds what the screen shows (the notes, the tasks, the
// filters, the open note or task) and sends every change to a `notes.*` or `tasks.*` operation.
// The core decides what is allowed; nothing here validates beyond what the form needs to word.

import Foundation
import Observation
import OpenFrameCore

enum TaskFilter: String, CaseIterable, Identifiable {
    case open, done, all
    var id: String { rawValue }
    var title: String {
        switch self {
        case .open: "Open"
        case .done: "Done"
        case .all: "All"
        }
    }
}

/// Which task the drawer shows: a blank one, or one that exists.
enum TaskTarget: Equatable {
    case new
    case existing(String)
}

@MainActor
@Observable
final class NotesModel {
    let app: AppModel

    /// The text being typed in the notes search field.
    var noteText = ""
    private(set) var appliedNoteText = ""
    var taskFilter: TaskFilter = .open
    /// A member's user id; empty means anyone.
    var owner = ""

    private(set) var notes: [NoteDto] = []
    private(set) var notesLoaded = false
    private(set) var tasks: [TaskDto] = []
    private(set) var tasksLoaded = false
    private(set) var members: [Member] = []
    private(set) var loadFailure: CoreFailure?

    var openNoteId: String?
    var openTask: TaskTarget?

    @ObservationIgnored private var generation = 0

    init(app: AppModel) { self.app = app }

    // MARK: Derived

    var openCount: Int { tasks.filter { $0.status == "Open" }.count }

    var shownTasks: [TaskDto] {
        tasks.filter {
            switch taskFilter {
            case .all: true
            case .open: $0.status == "Open"
            case .done: $0.status == "Done"
            }
        }
    }

    var searchingNotes: Bool { !appliedNoteText.isEmpty }

    var editingTask: TaskDto? {
        if case let .existing(id)? = openTask { tasks.first { $0.id == id } } else { nil }
    }

    /// What the tasks panel says when it has no rows to show.
    var tasksEmptyText: String? {
        guard tasksLoaded, shownTasks.isEmpty else { return nil }
        if tasks.isEmpty && owner.isEmpty { return nil }
        switch taskFilter {
        case .open: return "Nothing open. Well done."
        case .done: return "No completed tasks yet."
        case .all: return "No tasks for this owner."
        }
    }

    var ownerChoices: [(id: String, label: String)] {
        [("", "Anyone")] + members.map { ($0.userId, $0.isYou ? "Me" : $0.displayName) }
    }

    // MARK: Loading

    func applySearch() {
        let next = noteText.trimmingCharacters(in: .whitespaces)
        if next != appliedNoteText { appliedNoteText = next }
    }

    /// Reads the notes, the tasks and the members. A slower answer that arrives after a newer
    /// request is dropped.
    func reload() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        let text = appliedNoteText.isEmpty ? nil : appliedNoteText
        let ownerId = owner.isEmpty ? nil : owner
        do {
            async let n = core.notesList(NoteListArgs(text: text))
            async let t = core.tasksList(TaskListArgs(ownerUserId: ownerId))
            async let m = core.projectMembers()
            let (notes, tasks, members) = try await (n, t, m)
            guard mine == generation else { return }
            self.notes = notes
            self.tasks = tasks
            self.members = members
            notesLoaded = true
            tasksLoaded = true
            loadFailure = nil
        } catch {
            guard mine == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            notesLoaded = true
            tasksLoaded = true
        }
    }

    /// A jump from search, Activity or a task chip can open one note or task.
    func apply(_ intent: PageIntent) {
        if let id = intent.noteId { showNote(id) }
        if let id = intent.taskId { showTask(id) }
    }

    // MARK: Opening

    func showNote(_ id: String) {
        openTask = nil
        openNoteId = id
    }

    func showTask(_ id: String) {
        openNoteId = nil
        openTask = .existing(id)
    }

    func startNewTask() {
        openNoteId = nil
        openTask = .new
    }

    func closeDrawer() {
        openNoteId = nil
        openTask = nil
    }

    // MARK: Changing

    private func run<T>(_ title: String, _ body: (Core) async throws -> T) async -> T? {
        guard let core = app.core else { return nil }
        do {
            return try await body(core)
        } catch {
            app.show(title, error)
            return nil
        }
    }

    /// New Note: makes an empty note and opens it for typing.
    @discardableResult
    func newNote() async -> NoteDto? {
        guard let note = await run("Could not add the note", { core in
            try await core.notesCreate(CreateNoteArgs(title: nil, body: "", pinned: false))
        }) else { return nil }
        await reload()
        showNote(note.id)
        return note
    }

    func toggleDone(_ task: TaskDto) async {
        let done = task.status != "Done"
        if await run("Could not change the task", { core in try await core.tasksSetDone(SetTaskDoneArgs(id: task.id, done: done)) }) != nil {
            app.pageToast(done ? "Completed task \u{201C}\(task.title)\u{201D}" : "Reopened task \u{201C}\(task.title)\u{201D}")
            await reload()
        }
    }

    /// Chooses what the task panel shows for a task id.
    func task(_ id: String) -> TaskDto? { tasks.first { $0.id == id } }
}

// MARK: One note in the drawer

/// The open note: title and text with automatic saving. Typing waits a moment and saves in the
/// background; closing the drawer saves at once, so nothing typed is lost. A value changed for
/// another reason (undo, another window) replaces only a field the person is not editing.
@MainActor
@Observable
final class NoteEditorModel {
    enum Field: CaseIterable { case title, body }
    enum Status: Equatable { case saved, saving, failed }

    let notes: NotesModel
    let id: String

    private(set) var note: NoteDto?
    private(set) var missing = false
    var title = ""
    var body = ""
    private(set) var status: Status = .saved

    @ObservationIgnored private var baseline = (title: "", body: "")
    @ObservationIgnored private var dirty = Set<Field>()
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private var inflight: Task<Void, Never>?
    @ObservationIgnored var delay: Duration = .milliseconds(600)

    init(notes: NotesModel, id: String) {
        self.notes = notes
        self.id = id
    }

    private var core: Core? { notes.app.core }

    /// Reads the note and merges it into fields the person is not editing.
    func refresh() async {
        guard let core else { return }
        do {
            let fresh = try await core.notesGet(NoteIdArgs(id: id))
            missing = false
            let serverTitle = fresh.title ?? ""
            if note == nil {
                title = serverTitle
                body = fresh.body
                baseline = (serverTitle, fresh.body)
            } else {
                if serverTitle != baseline.title, !dirty.contains(.title) {
                    baseline.title = serverTitle
                    title = serverTitle
                }
                if fresh.body != baseline.body, !dirty.contains(.body) {
                    baseline.body = fresh.body
                    body = fresh.body
                }
            }
            note = fresh
        } catch {
            if note == nil || !(error is CancellationError) { missing = true }
        }
    }

    func edit(_ field: Field, to value: String) {
        switch field {
        case .title:
            guard title != value else { return }
            title = value
        case .body:
            guard body != value else { return }
            body = value
        }
        dirty.insert(field)
        status = .saving
        timer?.cancel()
        let wait = delay
        timer = Task { [weak self] in
            try? await Task.sleep(for: wait)
            guard !Task.isCancelled else { return }
            await self?.flush()
        }
    }

    /// Saves what is waiting. Returns true when nothing is left unsaved.
    @discardableResult
    func flush() async -> Bool {
        timer?.cancel()
        timer = nil
        if let inflight { await inflight.value }
        let keys = dirty
        guard !keys.isEmpty, let core else { return dirty.isEmpty }
        let t = title, b = body
        dirty = []
        status = .saving
        let noteId = id
        let run = Task { [weak self] in
            guard let self else { return }
            do {
                let saved = try await core.notesUpdate(UpdateNoteArgs(
                    id: noteId, title: keys.contains(.title) ? t : nil, body: keys.contains(.body) ? b : nil))
                if keys.contains(.title) { baseline.title = saved.title ?? "" }
                if keys.contains(.body) { baseline.body = saved.body }
                note = saved
                status = dirty.isEmpty ? .saved : .saving
            } catch {
                for k in keys { dirty.insert(k) }
                status = .failed
                notes.app.show("Could not save the note", error)
            }
        }
        inflight = run
        await run.value
        inflight = nil
        await notes.reload()
        return dirty.isEmpty
    }

    var heading: String { missing ? "Note not available" : (note == nil ? "Loading\u{2026}" : PageText.noteLabel(title: title, body: body)) }

    var statusText: String {
        switch status {
        case .saved: "Saved automatically"
        case .saving: "Saving\u{2026}"
        case .failed: "Not saved yet."
        }
    }

    func togglePin() async {
        guard let note, let core else { return }
        do {
            _ = try await core.notesSetPinned(SetNotePinnedArgs(id: id, pinned: !note.pinned))
            await refresh()
            await notes.reload()
        } catch {
            notes.app.show("Could not change the pin", error)
        }
    }

    /// Saves, deletes, tells the person where it went and closes the drawer.
    func delete() async -> Bool {
        await flush()
        guard let core else { return false }
        let label = PageText.noteLabel(title: title, body: body)
        do {
            _ = try await core.notesDelete(NoteIdArgs(id: id))
            notes.app.pageToast("Moved note \u{201C}\(label)\u{201D} to Recently Deleted")
            notes.closeDrawer()
            await notes.reload()
            return true
        } catch {
            notes.app.show("Could not delete the note", error)
            return false
        }
    }

    func close() async {
        await flush()
        notes.closeDrawer()
    }
}

// MARK: One task in the drawer

/// A thing a task can be related to, chosen from a search.
struct RelatedChoice: Equatable {
    var ref: RelatedRef
    var title: String
    var typeLabel: String
}

/// The task form: a blank task, or the fields of a task that exists. Nothing is saved until the
/// person presses Save or Add Task, then the core checks it.
@MainActor
@Observable
final class TaskEditorModel {
    let notes: NotesModel
    let task: TaskDto?

    var title: String
    var hasDue: Bool
    var due: Date
    var owner: String
    var taskNotes: String
    var related: RelatedChoice?
    private(set) var touched = false
    private(set) var busy = false

    // Search for the related item.
    var relatedText = ""
    private(set) var relatedHits: [SearchHit] = []
    private(set) var relatedSearching = false
    private(set) var relatableTypes: [RelatableType] = []

    @ObservationIgnored private var searchGeneration = 0

    init(notes: NotesModel, task: TaskDto?) {
        self.notes = notes
        self.task = task
        title = task?.title ?? ""
        hasDue = task?.dueAt != nil
        due = task?.dueAt.map { Date(timeIntervalSince1970: $0 / 1000) } ?? Date()
        owner = task?.ownerUserId ?? ""
        taskNotes = task?.notes ?? ""
        related = task?.related.map {
            RelatedChoice(ref: RelatedRef(targetType: $0.targetType, targetId: $0.targetId), title: $0.title ?? "Unavailable item", typeLabel: $0.typeLabel)
        }
    }

    var titleMissing: Bool { title.trimmingCharacters(in: .whitespaces).isEmpty }
    var titleError: String? { touched && titleMissing ? "Give the task a short title." : nil }

    private var initialRelated: String { task?.related.map { "\($0.targetType):\($0.targetId)" } ?? "" }
    private var currentRelated: String { related.map { "\($0.ref.targetType):\($0.ref.targetId)" } ?? "" }

    private func sameDue(_ ms: Double?) -> Bool {
        switch (hasDue, ms) {
        case (false, nil): return true
        case let (true, ms?): return Calendar.current.isDate(due, inSameDayAs: Date(timeIntervalSince1970: ms / 1000))
        default: return false
        }
    }

    /// A new task can always be added; an existing one is saved only after a change.
    var dirty: Bool {
        guard let task else { return true }
        return title != task.title || !sameDue(task.dueAt) || owner != (task.ownerUserId ?? "")
            || taskNotes != (task.notes ?? "") || currentRelated != initialRelated
    }

    var canSave: Bool { !busy && dirty }
    var saveLabel: String { task == nil ? "Add Task" : "Save" }
    var heading: String { task?.title ?? "New Task" }
    var relatedDeletedNote: String? {
        guard let r = task?.related, !r.available, currentRelated == initialRelated else { return nil }
        return "The related \(r.typeLabel.lowercased()) was deleted. The task keeps the reference."
    }
    var relatedNav: JSONValue? {
        guard let r = task?.related, r.available, PageNav.canOpen(r.nav), currentRelated == initialRelated else { return nil }
        return r.nav
    }

    func touch() { touched = true }

    /// Adds or saves the task. Returns true when the drawer can close.
    @discardableResult
    func save() async -> Bool {
        touched = true
        guard !titleMissing, !busy, let core = notes.app.core else { return false }
        busy = true
        defer { busy = false }
        let clean = title.trimmingCharacters(in: .whitespaces)
        let dueAt: Double? = hasDue ? PageText.dueMs(day: due) : nil
        do {
            if let task {
                _ = try await core.tasksUpdate(UpdateTaskArgs(
                    id: task.id, title: clean, dueAt: dueAt, clearDue: dueAt == nil, ownerUserId: owner, notes: taskNotes,
                    related: (related != nil && currentRelated != initialRelated) ? related?.ref : nil,
                    clearRelated: related == nil, expectedRev: task.rev))
                notes.app.pageToast("Edited task")
            } else {
                let trimmedNotes = taskNotes.trimmingCharacters(in: .whitespacesAndNewlines)
                _ = try await core.tasksCreate(CreateTaskArgs(
                    title: clean, dueAt: dueAt, ownerUserId: owner.isEmpty ? nil : owner,
                    notes: trimmedNotes.isEmpty ? nil : trimmedNotes, related: related?.ref))
                notes.app.pageToast("Added task \u{201C}\(clean)\u{201D}")
            }
            notes.closeDrawer()
            await notes.reload()
            return true
        } catch {
            notes.app.show("Could not save the task", error)
            return false
        }
    }

    func setDone(_ done: Bool) async {
        guard let task, let core = notes.app.core else { return }
        do {
            _ = try await core.tasksSetDone(SetTaskDoneArgs(id: task.id, done: done))
            notes.app.pageToast(done ? "Completed task \u{201C}\(task.title)\u{201D}" : "Reopened task \u{201C}\(task.title)\u{201D}")
            await notes.reload()
        } catch {
            notes.app.show("Could not change the task", error)
        }
    }

    func delete() async {
        guard let task, let core = notes.app.core else { return }
        do {
            _ = try await core.tasksDelete(TaskRefArgs(id: task.id))
            notes.app.pageToast("Moved task \u{201C}\(task.title)\u{201D} to Recently Deleted")
            notes.closeDrawer()
            await notes.reload()
        } catch {
            notes.app.show("Could not delete the task", error)
        }
    }

    // MARK: Related item

    func loadRelatableTypes() async {
        guard relatableTypes.isEmpty, let core = notes.app.core else { return }
        relatableTypes = (try? await core.tasksRelatableTypes()) ?? []
    }

    func label(for entityType: String) -> String {
        relatableTypes.first { $0.targetType == entityType }?.label ?? entityType
    }

    /// Searches the things a task can relate to. Called after a short pause in typing.
    func searchRelated() async {
        let text = relatedText.trimmingCharacters(in: .whitespaces)
        searchGeneration += 1
        let mine = searchGeneration
        guard !text.isEmpty, let core = notes.app.core else {
            relatedHits = []
            relatedSearching = false
            return
        }
        await loadRelatableTypes()
        relatedSearching = true
        let hits = try? await core.searchQuery(SearchArgs(
            text: text, includeGlobal: false, entityTypes: relatableTypes.map(\.targetType), limit: 8, globalOnly: false))
        guard mine == searchGeneration else { return }
        relatedHits = hits ?? []
        relatedSearching = false
    }

    func choose(_ hit: SearchHit) {
        related = RelatedChoice(
            ref: RelatedRef(targetType: hit.entityType, targetId: hit.entityId),
            title: hit.title.isEmpty ? "Untitled" : hit.title, typeLabel: label(for: hit.entityType))
        relatedText = ""
        relatedHits = []
    }

    func removeRelated() { related = nil }
}

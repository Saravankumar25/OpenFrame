// Notes and Tasks (Windows: workspaces/notes). "Small things that do not belong anywhere else."
// Project notes (pinned first) on one side, lightweight tasks on the other, and a drawer on the
// right for the open note or task.

import OpenFrameCore
import SwiftUI

struct NotesWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var model: NotesModel?

    var body: some View {
        Group {
            if let model {
                NotesScreen(model: model)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .fitsWindow()
        .onAppear {
            if model == nil { model = NotesModel(app: app) }
        }
    }
}

struct NotesScreen: View {
    @Bindable var model: NotesModel
    @Environment(\.ofTheme) private var theme

    private var drawerOpen: Bool { model.openNoteId != nil || model.openTask != nil }

    var body: some View {
        ScrollingPage {
        VStack(alignment: .leading, spacing: 0) {
            ProjectPageHeader(title: "Notes & Tasks", subtitle: "Small things that do not belong anywhere else.", identifier: "notes") {
                HStack {
                    Button { Task { await model.newNote() } } label: { Label("New Note", systemImage: "plus") }
                        .accessibilityIdentifier("new-note")
                    Button { model.startNewTask() } label: { Label("New Task", systemImage: "plus") }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("new-task")
                }
            }
            if let failure = model.loadFailure {
                Text(failure.message).foregroundStyle(theme["red-t"]).multilineTextAlignment(.center).frame(maxWidth: .infinity).padding(.horizontal, 24)
                    .accessibilityIdentifier("notes-load-error")
            }
            ViewThatFits(in: .horizontal) {
                HStack(alignment: .top, spacing: PageSpacing.medium) {
                    notesPanel
                    tasksPanel
                }
                VStack(spacing: PageSpacing.medium) {
                    notesPanel
                    tasksPanel
                }
            }
            .pageColumn(maxWidth: PageSpacing.wideColumn)
            .padding(.bottom, PageSpacing.large)
        }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["bg"])
        .inspector(isPresented: Binding(get: { drawerOpen }, set: { if !$0 { model.closeDrawer() } })) {
            drawer.inspectorColumnWidth(min: 340, ideal: 400, max: 540)
        }
        .task(id: model.app.changes.key(["project_note", "project_member", "task", "search_doc", "deleted_item"]) &+ model.appliedNoteText.hashValue &+ model.owner.hashValue) {
            await model.reload()
        }
        .task(id: model.noteText) {
            try? await Task.sleep(for: .milliseconds(200))
            if !Task.isCancelled { model.applySearch() }
        }
        .onAppear { model.apply(PageNav.take()) }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowWorkspace)) { _ in model.apply(PageNav.take()) }
        .onProjectIntents(["notes.new_note", "notes.new_task"]) { intent in
            if intent == "notes.new_task" { model.startNewTask() } else { Task { await model.newNote() } }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("notes-workspace")
    }

    // MARK: Drawer

    @ViewBuilder private var drawer: some View {
        if let id = model.openNoteId {
            NoteDrawer(notes: model, id: id).id(id)
        } else if case .new? = model.openTask {
            TaskDrawer(notes: model, task: nil).id("new-task")
        } else if let task = model.editingTask {
            TaskDrawer(notes: model, task: task).id(task.id)
        } else if case .existing? = model.openTask {
            VStack(spacing: 8) {
                Text("Task not available").font(.headline)
                Text("This task may have been deleted. Look in Recently Deleted to restore it.").foregroundStyle(theme["muted"])
                Button("Close") { model.closeDrawer() }.accessibilityIdentifier("task-close")
            }
            .padding(20)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    // MARK: Notes

    private var notesPanel: some View {
        PageCard {
            HStack {
                Text("Project Notes").fontWeight(.semibold).accessibilityAddTraits(.isHeader)
                Spacer()
                HStack(spacing: 4) {
                    Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                    TextField("Search notes", text: $model.noteText)
                        .textFieldStyle(.plain)
                        .frame(width: 170)
                        .accessibilityLabel("Search notes")
                        .accessibilityIdentifier("notes-search")
                }
                .padding(.horizontal, 8).padding(.vertical, 4)
                .background(theme["bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
            }
            .padding(12)
            Divider().overlay(theme["line"])
            if !model.notesLoaded {
                ProgressView().frame(maxWidth: .infinity).padding(20)
            } else if model.notes.isEmpty {
                if model.searchingNotes {
                    Text("No notes match \u{201C}\(model.appliedNoteText)\u{201D}.")
                        .font(.callout).foregroundStyle(theme["muted"]).padding(14)
                        .accessibilityIdentifier("notes-no-match")
                } else {
                    emptyNotes
                }
            } else {
                VStack(spacing: 0) {
                    ForEach(model.notes, id: \.id) { note in noteRow(note) }
                }
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("notes-list")
            }
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Project Notes")
        .accessibilityIdentifier("notes-panel")
    }

    private var emptyNotes: some View {
        ContentUnavailableView {
            Label("No notes yet.", systemImage: "note.text")
        } description: {
            Text("Add a note for something that doesn't naturally belong in Idea Vault or another workspace.")
        } actions: {
            Button { Task { await model.newNote() } } label: { Label("New Note", systemImage: "plus") }
                .accessibilityIdentifier("notes-empty-new-note")
        }
        .padding(.vertical, 12)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("notes-empty")
    }

    private func noteRow(_ note: NoteDto) -> some View {
        let selected = model.openNoteId == note.id
        let label = PageText.noteLabel(title: note.title, body: note.body)
        let preview = PageText.notePreview(title: note.title, body: note.body)
        return Button {
            model.showNote(note.id)
        } label: {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 5) {
                        if note.pinned { Image(systemName: "pin.fill").font(.caption).accessibilityLabel("Pinned") }
                        Text(label).fontWeight(.semibold).lineLimit(1)
                    }
                    if !preview.isEmpty {
                        Text(preview).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                    }
                }
                Spacer(minLength: 8)
                Text(PageText.dayWord(ms: note.updatedAt)).font(.caption).foregroundStyle(theme["muted"])
            }
            .padding(.horizontal, 12).padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(selected ? theme["accent-soft"] : Color.clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
        .accessibilityValue(note.pinned ? "Pinned" : "")
        .accessibilityAddTraits(selected ? .isSelected : [])
        .accessibilityIdentifier("note-row-\(note.id)")
        .contextMenu {
            Button("Open") { model.showNote(note.id) }
            Button(note.pinned ? "Unpin" : "Pin") { Task { await togglePin(note) } }
        }
        .overlay(alignment: .bottom) { Divider().overlay(theme["line"]) }
    }

    private func togglePin(_ note: NoteDto) async {
        guard let core = model.app.core else { return }
        do { _ = try await core.notesSetPinned(SetNotePinnedArgs(id: note.id, pinned: !note.pinned)) } catch { model.app.show("Could not change the pin", error) }
    }

    // MARK: Tasks

    private var tasksPanel: some View {
        PageCard {
            HStack(spacing: 8) {
                Text("Tasks").fontWeight(.semibold).lineLimit(1).fixedSize().accessibilityAddTraits(.isHeader)
                PageChip(text: "\(model.openCount) open").fixedSize()
                    .accessibilityIdentifier("tasks-open-count")
                Spacer()
                Picker("Owner", selection: $model.owner) {
                    ForEach(model.ownerChoices, id: \.id) { Text($0.label).tag($0.id) }
                }
                .labelsHidden()
                .frame(width: 120)
                .accessibilityLabel("Owner")
                .accessibilityIdentifier("tasks-owner")
                Picker("Show tasks", selection: $model.taskFilter) {
                    ForEach(TaskFilter.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 170)
                .accessibilityLabel("Show tasks")
                .accessibilityIdentifier("tasks-filter")
            }
            .padding(12)
            Divider().overlay(theme["line"])
            if !model.tasksLoaded {
                ProgressView().frame(maxWidth: .infinity).padding(20)
            } else if model.tasks.isEmpty && model.owner.isEmpty {
                ContentUnavailableView {
                    Label("No tasks yet.", systemImage: "checklist")
                } description: {
                    Text("Add a small task only when you need to remember an action.")
                } actions: {
                    Button { model.startNewTask() } label: { Label("New Task", systemImage: "plus") }
                        .accessibilityIdentifier("tasks-empty-new-task")
                }
                .padding(.vertical, 12)
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("tasks-empty")
            } else if let message = model.tasksEmptyText {
                Text(message).font(.callout).foregroundStyle(theme["muted"]).padding(14)
                    .accessibilityIdentifier("tasks-none-shown")
            } else {
                VStack(spacing: 0) {
                    ForEach(model.shownTasks, id: \.id) { task in taskRow(task) }
                }
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("tasks-list")
            }
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Tasks")
        .accessibilityIdentifier("tasks-panel")
    }

    private func taskRow(_ task: TaskDto) -> some View {
        let done = task.status == "Done"
        let due = task.dueAt.map { PageText.dueLabel(ms: $0) }
        let selected = model.editingTask?.id == task.id
        return HStack(spacing: 8) {
            Toggle(isOn: Binding(get: { done }, set: { _ in Task { await model.toggleDone(task) } })) { EmptyView() }
                .toggleStyle(.checkbox)
                .labelsHidden()
                .accessibilityLabel(done ? "Reopen \u{201C}\(task.title)\u{201D}" : "Mark \u{201C}\(task.title)\u{201D} done")
                .accessibilityIdentifier("task-check-\(task.id)")
            Button {
                model.showTask(task.id)
            } label: {
                HStack(spacing: 0) {
                    Text(task.title).strikethrough(done).foregroundStyle(done ? theme["muted"] : theme["ink"]).lineLimit(1)
                    if let name = task.ownerName { Text(" \u{00B7} \(name)").font(.caption).foregroundStyle(theme["muted"]) }
                    Spacer(minLength: 0)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Open task \u{201C}\(task.title)\u{201D}")
            .accessibilityIdentifier("task-row-\(task.id)")
            if let related = task.related { relatedChip(related) }
            if let due, !done {
                Text(due.overdue ? "\(due.text) \u{00B7} overdue" : due.text)
                    .font(.caption).foregroundStyle(due.overdue ? theme["red-t"] : theme["muted"])
            }
            if done { PageChip(text: "Done", tone: .green) }
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(selected ? theme["accent-soft"] : Color.clear)
        .overlay(alignment: .bottom) { Divider().overlay(theme["line"]) }
        .contextMenu {
            Button("Open") { model.showTask(task.id) }
            Button(done ? "Reopen" : "Mark Done") { Task { await model.toggleDone(task) } }
        }
    }

    @ViewBuilder private func relatedChip(_ related: RelatedObject) -> some View {
        let text = related.typeLabel + (related.title.map { ": \($0)" } ?? "")
        if PageNav.canOpen(related.nav) {
            Button { PageNav.open(related.nav) } label: { PageChip(text: text, tone: .blue) }
                .buttonStyle(.plain)
                .help("Open \(related.typeLabel)")
                .accessibilityLabel("Open \(related.typeLabel)")
                .accessibilityIdentifier("task-related-\(related.targetId)")
        } else {
            PageChip(text: related.typeLabel + (related.available ? "" : " (deleted)"))
                .help(related.available ? "" : "This item was deleted")
        }
    }
}

// MARK: Note drawer

private struct NoteDrawer: View {
    let notes: NotesModel
    @State private var editor: NoteEditorModel
    @FocusState private var focus: Focus?
    @Environment(\.ofTheme) private var theme

    private enum Focus { case title, body }

    init(notes: NotesModel, id: String) {
        self.notes = notes
        _editor = State(initialValue: NoteEditorModel(notes: notes, id: id))
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider().overlay(theme["line"])
            if editor.missing {
                Text("This note may have been deleted. Look in Recently Deleted to restore it.")
                    .foregroundStyle(theme["muted"]).padding(16)
                    .accessibilityIdentifier("note-missing")
                Spacer()
            } else if let note = editor.note {
                ScrollView {
                    VStack(alignment: .leading, spacing: 12) {
                        labelled("Title") {
                            TextField("Untitled note", text: Binding(get: { editor.title }, set: { editor.edit(.title, to: String($0.prefix(200))) }))
                                .textFieldStyle(.roundedBorder)
                                .focused($focus, equals: .title)
                                .onSubmit { Task { await editor.flush() } }
                                .accessibilityLabel("Note title")
                                .accessibilityIdentifier("note-title")
                        }
                        labelled("Note") {
                            TextEditor(text: Binding(get: { editor.body }, set: { editor.edit(.body, to: $0) }))
                                .font(.body)
                                .frame(minHeight: 260)
                                .scrollContentBackground(.hidden)
                                .padding(6)
                                .background(theme["bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                                .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(theme["line"]))
                                .focused($focus, equals: .body)
                                .overlay(alignment: .topLeading) {
                                    if editor.body.isEmpty {
                                        Text("Write anything that doesn't belong in the Idea Vault or another workspace.")
                                            .foregroundStyle(theme["muted"]).padding(.horizontal, 11).padding(.vertical, 14).allowsHitTesting(false)
                                    }
                                }
                                .accessibilityLabel("Note body")
                                .accessibilityIdentifier("note-body")
                        }
                        Text((note.createdByName.map { "Added by \($0) \u{00B7} " } ?? "")
                            + "Created \(PageText.dayAndTime(ms: note.createdAt)) \u{00B7} Changed \(PageText.dayAndTime(ms: note.updatedAt))")
                            .font(.caption).foregroundStyle(theme["muted"])
                            .accessibilityIdentifier("note-dates")
                        Text("Notes are kept out of formal documents such as screenplay and call-sheet exports.")
                            .font(.caption).foregroundStyle(theme["muted"])
                        Text(editor.statusText).font(.caption).foregroundStyle(theme["muted"])
                            .accessibilityIdentifier("note-save-status")
                    }
                    .padding(16)
                }
                Divider().overlay(theme["line"])
                HStack {
                    Button(role: .destructive) { Task { _ = await editor.delete() } } label: { Label("Delete", systemImage: "trash") }
                        .accessibilityIdentifier("note-delete")
                    Spacer()
                    Button { Task { await editor.togglePin() } } label: {
                        Label(note.pinned ? "Unpin" : "Pin", systemImage: note.pinned ? "pin.slash" : "pin")
                    }
                    .accessibilityIdentifier("note-pin")
                }
                .padding(12)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(theme["panel"])
        .task(id: notes.app.changes.key(["project_note", "project_member"])) { await editor.refresh() }
        .onChange(of: editor.note?.id) { _, _ in
            if let n = editor.note, n.body.isEmpty { focus = .body }
        }
        .onChange(of: focus) { old, new in
            if old != nil, new != old { Task { await editor.flush() } }
        }
        .onDisappear { Task { await editor.flush() } }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Project note")
        .accessibilityIdentifier("note-drawer")
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Project note").font(.caption).foregroundStyle(theme["muted"])
                Text(editor.heading).font(.headline).lineLimit(2)
                    .accessibilityIdentifier("note-heading")
            }
            Spacer()
            Button { Task { await editor.close() } } label: { Label("Close", systemImage: "xmark") }
                .buttonStyle(.borderless)
                .help("Close")
                .keyboardShortcut(.cancelAction)
                .accessibilityLabel("Close")
                .accessibilityIdentifier("note-close")
        }
        .padding(14)
    }

    private func labelled<V: View>(_ text: String, @ViewBuilder _ content: () -> V) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(text).font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
            content()
        }
    }
}

// MARK: Task drawer

private struct TaskDrawer: View {
    let notes: NotesModel
    @State private var editor: TaskEditorModel
    @FocusState private var titleFocused: Bool
    @Environment(\.ofTheme) private var theme

    init(notes: NotesModel, task: TaskDto?) {
        self.notes = notes
        _editor = State(initialValue: TaskEditorModel(notes: notes, task: task))
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Task").font(.caption).foregroundStyle(theme["muted"])
                    Text(editor.heading).font(.headline).lineLimit(2)
                        .accessibilityIdentifier("task-heading")
                }
                Spacer()
                Button { notes.closeDrawer() } label: { Label("Close", systemImage: "xmark") }
                    .buttonStyle(.borderless)
                    .help("Close")
                    .accessibilityLabel("Close")
                    .accessibilityIdentifier("task-close")
            }
            .padding(14)
            Divider().overlay(theme["line"])
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    if editor.task?.status == "Done" { PageChip(text: "Done", tone: .green).accessibilityIdentifier("task-done-chip") }
                    labelled("Task") {
                        VStack(alignment: .leading, spacing: 3) {
                            TextField("e.g. Confirm police station", text: Binding(get: { editor.title }, set: { editor.title = String($0.prefix(300)) }))
                                .textFieldStyle(.roundedBorder)
                                .focused($titleFocused)
                                .onSubmit { Task { await editor.save() } }
                                .accessibilityLabel("Task title")
                                .accessibilityIdentifier("task-title")
                            if let error = editor.titleError {
                                Text(error).font(.caption).foregroundStyle(theme["red-t"])
                                    .accessibilityIdentifier("task-title-error")
                            }
                        }
                    }
                    HStack(alignment: .top, spacing: 12) {
                        labelled("Due date") {
                            HStack {
                                Toggle("Set a due date", isOn: $editor.hasDue)
                                    .accessibilityIdentifier("task-has-due")
                                if editor.hasDue {
                                    DatePicker("Due date", selection: $editor.due, displayedComponents: .date)
                                        .labelsHidden()
                                        .accessibilityLabel("Due date")
                                        .accessibilityIdentifier("task-due")
                                }
                            }
                        }
                        labelled("Owner") {
                            Picker("Owner", selection: $editor.owner) {
                                Text("No owner").tag("")
                                ForEach(notes.members, id: \.userId) { m in
                                    Text(m.isYou ? "\(m.displayName) (you)" : m.displayName).tag(m.userId)
                                }
                            }
                            .labelsHidden()
                            .accessibilityLabel("Owner")
                            .accessibilityIdentifier("task-owner")
                        }
                    }
                    labelled("Related to") { relatedPicker }
                    labelled("Notes") {
                        TextEditor(text: Binding(get: { editor.taskNotes }, set: { editor.taskNotes = String($0.prefix(5000)) }))
                            .frame(minHeight: 90)
                            .scrollContentBackground(.hidden)
                            .padding(6)
                            .background(theme["bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                            .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(theme["line"]))
                            .accessibilityLabel("Task notes")
                            .accessibilityIdentifier("task-notes")
                    }
                }
                .padding(16)
            }
            Divider().overlay(theme["line"])
            footer
        }
        .background(theme["panel"])
        .onAppear { if editor.task == nil { titleFocused = true } }
        .task { await editor.loadRelatableTypes() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Task")
        .accessibilityIdentifier("task-drawer")
    }

    private var footer: some View {
        HStack {
            if editor.task != nil {
                Button(role: .destructive) { Task { await editor.delete() } } label: { Label("Delete", systemImage: "trash") }
                    .accessibilityIdentifier("task-delete")
            }
            Spacer()
            if let task = editor.task {
                if task.status == "Done" {
                    Button { Task { await editor.setDone(false) } } label: { Label("Reopen", systemImage: "arrow.uturn.backward") }
                        .accessibilityIdentifier("task-reopen")
                } else {
                    Button { Task { await editor.setDone(true) } } label: { Label("Mark Done", systemImage: "checkmark.circle") }
                        .accessibilityIdentifier("task-mark-done")
                }
            }
            Button("Cancel") { notes.closeDrawer() }
                .keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("task-cancel")
            Button(editor.saveLabel) { Task { await editor.save() } }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
                .disabled(!editor.canSave)
                .accessibilityIdentifier("task-save")
        }
        .padding(12)
    }

    @ViewBuilder private var relatedPicker: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Optional. Completing the task never changes the related item.").font(.caption).foregroundStyle(theme["muted"])
            if let r = editor.related {
                HStack {
                    PageChip(text: r.typeLabel)
                    Text(r.title).fontWeight(.medium).lineLimit(1)
                    Spacer()
                    Button("Remove") { editor.removeRelated() }
                        .accessibilityLabel("Remove related item")
                        .accessibilityIdentifier("task-related-remove")
                }
            } else {
                TextField("Find a scene, location, character\u{2026}", text: $editor.relatedText)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Find a scene, location, character, shooting day or call sheet")
                    .accessibilityIdentifier("task-related-search")
                    .task(id: editor.relatedText) {
                        try? await Task.sleep(for: .milliseconds(200))
                        if !Task.isCancelled { await editor.searchRelated() }
                    }
                if !editor.relatedText.trimmingCharacters(in: .whitespaces).isEmpty {
                    VStack(alignment: .leading, spacing: 0) {
                        if editor.relatedHits.isEmpty {
                            Text(editor.relatedSearching ? "Searching\u{2026}" : "Nothing matches. Tasks can relate to scenes, scene cards, characters, locations, breakdown items, cast, crew, shooting days and call sheets.")
                                .font(.caption).foregroundStyle(theme["muted"]).padding(8)
                        } else {
                            ForEach(editor.relatedHits, id: \.entityId) { hit in
                                Button { editor.choose(hit) } label: {
                                    HStack {
                                        PageChip(text: editor.label(for: hit.entityType))
                                        Text(hit.title.isEmpty ? "Untitled" : hit.title).lineLimit(1)
                                        Spacer()
                                        Text(hit.context).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                                    }
                                    .padding(.horizontal, 8).padding(.vertical, 5)
                                    .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                                .accessibilityIdentifier("task-related-hit-\(hit.entityId)")
                            }
                        }
                    }
                    .background(theme["bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(theme["line"]))
                    .accessibilityIdentifier("task-related-results")
                }
            }
            if let note = editor.relatedDeletedNote {
                Text(note).font(.caption).foregroundStyle(theme["muted"])
            }
            if let nav = editor.relatedNav, let r = editor.task?.related {
                Button { PageNav.open(nav) } label: { Label("Open \(r.typeLabel)", systemImage: "link") }
                    .buttonStyle(.link)
                    .accessibilityIdentifier("task-related-open")
            }
        }
    }

    private func labelled<V: View>(_ text: String, @ViewBuilder _ content: () -> V) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(text).font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
            content()
        }
    }
}

// Mac phase M6, Notes and Tasks: the shared scenario 80-notes-tasks, driven through the screen's
// models against the real core, plus edge and failure cases.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("ProjectPagesNotesTests", .serialized)
@MainActor
struct ProjectPagesNotesTests {
    // MARK: Scenario 80: a new note saves by itself; undo and redo

    @Test func aNewNoteSavesItsTitleAndTextByItselfThenUndoAndRedo() async throws {
        var s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        #expect(notes.notesLoaded)
        #expect(notes.notes.isEmpty)               // "No notes yet."
        #expect(!notes.searchingNotes)

        let created = try #require(await notes.newNote())
        #expect(notes.openNoteId == created.id)    // the drawer opens
        let editor = NoteEditorModel(notes: notes, id: created.id)
        await editor.refresh()
        editor.edit(.title, to: "Location scout")
        editor.edit(.body, to: "Check the old pier at low tide.")
        #expect(editor.status == .saving)
        await editor.close()
        #expect(notes.openNoteId == nil)
        #expect(s.labels(notes).contains { $0.hasPrefix("Location scout") })

        await s.app.undo()
        await notes.reload()
        #expect(!s.labels(notes).contains { $0.hasPrefix("Location scout") })
        await s.app.redo()
        await notes.reload()
        #expect(s.labels(notes).contains { $0.hasPrefix("Location scout") })
        #expect(s.quiet)
    }

    // MARK: Scenario 80: delete and restore

    @Test func deletingTheNoteMovesItToRecentlyDeletedAndRestoreBringsItBack() async throws {
        var s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        try await s.makeNote(notes, "Location scout", body: "Check the old pier at low tide.")

        try await s.deleteNote(notes, "Location scout")
        #expect(notes.notes.isEmpty)
        #expect(notes.openNoteId == nil)

        let trash = await s.trashModel()
        let row = try #require(trash.items.first { $0.title == "Location scout" })
        #expect(row.typeLabel == "Note")
        #expect(PageText.daysLeft(row.daysLeft) == "30 days left")
        #expect(await trash.restore(row))
        #expect(s.toast?.hasPrefix("Restored \u{201C}Location scout\u{201D}") == true)
        #expect(trash.items.isEmpty)

        await notes.reload()
        #expect(s.labels(notes).contains { $0.hasPrefix("Location scout") })
        #expect(s.quiet)
    }

    // MARK: Scenario 80: tasks

    @Test func aTaskIsAddedCompletedAndTheCompletionUndoneAndRedone() async throws {
        var s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        #expect(notes.tasks.isEmpty)

        notes.startNewTask()
        #expect(notes.openTask == .new)
        let form = TaskEditorModel(notes: notes, task: nil)
        form.title = "Confirm pier permit"
        #expect(await form.save())
        #expect(s.toast == "Added task \u{201C}Confirm pier permit\u{201D}")
        #expect(notes.openTask == nil)
        let task = try #require(notes.tasks.first)
        #expect(task.status == "Open")
        #expect(notes.shownTasks.map(\.title) == ["Confirm pier permit"])
        #expect(notes.openCount == 1)

        await notes.toggleDone(task)
        #expect(s.toast == "Completed task \u{201C}Confirm pier permit\u{201D}")
        #expect(notes.shownTasks.isEmpty)          // Open filter
        #expect(notes.tasksEmptyText == "Nothing open. Well done.")

        await s.app.undo()
        await notes.reload()
        #expect(notes.tasks.first?.status == "Open")   // the "Mark ... done" box shows again
        await s.app.redo()
        await notes.reload()
        notes.taskFilter = .all
        #expect(notes.shownTasks.first?.status == "Done")  // the "Reopen ..." box shows
        #expect(s.quiet)
    }

    // MARK: Scenario 80: after a relaunch

    @Test func afterARelaunchTheNoteAndTheCompletedTaskAreKept() async throws {
        var s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        var notes = await s.notesModel()
        try await s.makeNote(notes, "Location scout", body: "Check the old pier at low tide.")
        let form = TaskEditorModel(notes: notes, task: nil)
        form.title = "Confirm pier permit"
        #expect(await form.save())
        await notes.toggleDone(try #require(notes.tasks.first))

        try await s.relaunch(project: "Tide Tables")
        if case .recovery? = s.app.alert { Issue.record("a recovery question showed after a clean close") }
        notes = await s.notesModel()
        #expect(s.labels(notes).contains { $0.hasPrefix("Location scout") })
        notes.taskFilter = .all
        let task = try #require(notes.shownTasks.first)
        #expect(task.title == "Confirm pier permit")
        #expect(task.status == "Done")

        let note = try #require(notes.notes.first)
        notes.showNote(note.id)
        let editor = NoteEditorModel(notes: notes, id: note.id)
        await editor.refresh()
        #expect(editor.body == "Check the old pier at low tide.")
        await editor.close()
        #expect(notes.openNoteId == nil)
        #expect(s.quiet)
    }

    // MARK: Notes: edge and failure cases

    @Test func typingSavesAfterAShortPauseWithoutClosing() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let created = try #require(await notes.newNote())
        let editor = NoteEditorModel(notes: notes, id: created.id)
        editor.delay = .milliseconds(40)
        await editor.refresh()
        editor.edit(.body, to: "Half a thought")
        #expect(editor.status == .saving)
        try await Task.sleep(for: .milliseconds(400))
        #expect(editor.status == .saved)
        #expect(editor.statusText == "Saved automatically")
        let stored = try await s.core.notesGet(NoteIdArgs(id: created.id))
        #expect(stored.body == "Half a thought")
    }

    @Test func aNoteWithNoTitleIsNamedByItsFirstLine() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let created = try #require(await notes.newNote())
        let editor = NoteEditorModel(notes: notes, id: created.id)
        await editor.refresh()
        #expect(editor.heading == "Untitled note")
        editor.edit(.body, to: "Rain machine or wait?\nSecond camera?")
        await editor.close()
        #expect(s.labels(notes) == ["Rain machine or wait?"])
        #expect(PageText.notePreview(title: notes.notes[0].title, body: notes.notes[0].body) == "Second camera?")
    }

    @Test func pinnedNotesComeFirstAndPinAndUnpinWork() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        try await s.makeNote(notes, "Older")
        let newer = try await s.makeNote(notes, "Newer")
        #expect(s.labels(notes).first == "Newer")
        let older = try #require(notes.notes.first { $0.title == "Older" })
        let editor = NoteEditorModel(notes: notes, id: older.id)
        await editor.refresh()
        await editor.togglePin()
        #expect(editor.note?.pinned == true)
        #expect(s.labels(notes).first == "Older")
        await editor.togglePin()
        #expect(editor.note?.pinned == false)
        #expect(notes.notes.count == 2)
        #expect(notes.notes.contains { $0.id == newer.id })
        #expect(s.quiet)
    }

    @Test func searchingNotesFiltersThemAndSaysWhenNothingMatches() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        try await s.makeNote(notes, "Harbour master", body: "Needs the 5 am access time.")
        try await s.makeNote(notes, "Ferry route", body: "Rain machine or wait?")
        notes.noteText = "rain machine"
        notes.applySearch()
        await notes.reload()
        #expect(s.labels(notes) == ["Ferry route"])
        notes.noteText = "zebra"
        notes.applySearch()
        await notes.reload()
        #expect(notes.notes.isEmpty)
        #expect(notes.searchingNotes)              // "No notes match “zebra”."
        notes.noteText = ""
        notes.applySearch()
        await notes.reload()
        #expect(notes.notes.count == 2)
    }

    @Test func aNoteDeletedElsewhereShowsAsNotAvailableInTheDrawer() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let note = try await s.makeNote(notes, "Gone soon")
        let editor = NoteEditorModel(notes: notes, id: note.id)
        await editor.refresh()
        #expect(!editor.missing)
        _ = try await s.core.notesDelete(NoteIdArgs(id: note.id))
        await editor.refresh()
        #expect(editor.missing)
        #expect(editor.heading == "Note not available")
    }

    @Test func anUndoneEditReachesAnOpenNoteButNeverOverwritesTypedText() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let note = try await s.makeNote(notes, "Plan", body: "First draft")
        let editor = NoteEditorModel(notes: notes, id: note.id)
        await editor.refresh()
        editor.edit(.body, to: "Second draft")
        await editor.flush()
        await s.app.undo()
        await editor.refresh()
        // Typing coalesces into one step, so undo goes back to before this note's text.
        #expect(editor.body != "Second draft")
        // Text that is typed but not yet saved is left alone by a refresh.
        editor.edit(.body, to: "Typing right now")
        await editor.refresh()
        #expect(editor.body == "Typing right now")
        await editor.flush()
        #expect(s.quiet)
    }

    @Test func aFailedSaveKeepsTheTextAndSaysSoInPlainWords() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let note = try await s.makeNote(notes, "Doomed")
        let editor = NoteEditorModel(notes: notes, id: note.id)
        await editor.refresh()
        _ = try await s.core.notesDelete(NoteIdArgs(id: note.id))
        editor.edit(.body, to: "Words that cannot be saved")
        let saved = await editor.flush()
        #expect(!saved)
        #expect(editor.status == .failed)
        #expect(editor.statusText == "Not saved yet.")
        #expect(editor.body == "Words that cannot be saved")   // nothing typed is lost
        #expect(s.app.alert != nil)
    }

    // MARK: Tasks: edge and failure cases

    @Test func aTaskWithNoTitleIsRefusedInWordsAndNothingIsAdded() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let form = TaskEditorModel(notes: notes, task: nil)
        #expect(form.titleError == nil)            // not before the person has touched it
        form.title = "   "
        #expect(!(await form.save()))
        #expect(form.titleError == "Give the task a short title.")
        await notes.reload()
        #expect(notes.tasks.isEmpty)
        #expect(s.quiet)
    }

    @Test func aTaskKeepsItsDueDateOwnerAndNotesAndAnEditSavesOnlyAfterAChange() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let me = try #require(notes.members.first { $0.isYou })
        let form = TaskEditorModel(notes: notes, task: nil)
        form.title = "Book the ferry"
        form.hasDue = true
        form.due = Calendar.current.date(byAdding: .day, value: -2, to: Date())!
        form.owner = me.userId
        form.taskNotes = "Ask about the Sunday crossing."
        #expect(await form.save())
        let task = try #require(notes.tasks.first)
        #expect(task.ownerUserId == me.userId)
        #expect(task.ownerName != nil)
        #expect(task.notes == "Ask about the Sunday crossing.")
        let due = try #require(task.dueAt)
        #expect(PageText.dueLabel(ms: due).overdue)

        let edit = TaskEditorModel(notes: notes, task: task)
        #expect(!edit.dirty)
        #expect(!edit.canSave)
        #expect(edit.saveLabel == "Save")
        edit.title = "Book the late ferry"
        edit.hasDue = false
        #expect(edit.canSave)
        #expect(await edit.save())
        #expect(s.toast == "Edited task")
        let changed = try #require(notes.tasks.first)
        #expect(changed.title == "Book the late ferry")
        #expect(changed.dueAt == nil)
        #expect(changed.ownerUserId == me.userId)
        #expect(changed.notes == "Ask about the Sunday crossing.")
    }

    @Test func theOwnerFilterNarrowsTasksAndTheDoneFilterShowsCompletedOnes() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let me = try #require(notes.members.first { $0.isYou })
        #expect(notes.ownerChoices.first?.label == "Anyone")
        #expect(notes.ownerChoices.contains { $0.id == me.userId && $0.label == "Me" })
        for (title, owner) in [("Mine", me.userId), ("Nobody's", "")] {
            let f = TaskEditorModel(notes: notes, task: nil)
            f.title = title
            f.owner = owner
            #expect(await f.save())
        }
        #expect(notes.tasks.count == 2)
        notes.owner = me.userId
        await notes.reload()
        #expect(notes.tasks.map(\.title) == ["Mine"])
        notes.owner = ""
        await notes.reload()
        notes.taskFilter = .done
        #expect(notes.shownTasks.isEmpty)
        #expect(notes.tasksEmptyText == "No completed tasks yet.")
        await notes.toggleDone(try #require(notes.tasks.first { $0.title == "Mine" }))
        #expect(notes.shownTasks.map(\.title) == ["Mine"])
        await notes.toggleDone(notes.shownTasks[0])
        #expect(s.toast == "Reopened task \u{201C}Mine\u{201D}")
        #expect(notes.openCount == 2)
    }

    @Test func aTaskDrawerCanMarkDoneReopenAndDeleteToRecentlyDeleted() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let add = TaskEditorModel(notes: notes, task: nil)
        add.title = "Call the harbour office"
        #expect(await add.save())
        let task = try #require(notes.tasks.first)
        notes.showTask(task.id)
        #expect(notes.editingTask?.id == task.id)
        let form = TaskEditorModel(notes: notes, task: task)
        await form.setDone(true)
        #expect(s.toast == "Completed task \u{201C}Call the harbour office\u{201D}")
        #expect(notes.task(task.id)?.status == "Done")
        let doneForm = TaskEditorModel(notes: notes, task: notes.task(task.id))
        await doneForm.setDone(false)
        #expect(notes.task(task.id)?.status == "Open")
        await form.delete()
        #expect(s.toast == "Moved task \u{201C}Call the harbour office\u{201D} to Recently Deleted")
        #expect(notes.tasks.isEmpty)
        #expect(notes.openTask == nil)
        let trash = await s.trashModel()
        let row = try #require(trash.items.first)
        #expect(row.typeLabel == "Task")
        #expect(await trash.restore(row))
        await notes.reload()
        #expect(notes.tasks.map(\.title) == ["Call the harbour office"])
    }

    @Test func aStaleTaskEditIsRefusedInsteadOfOverwritingNewerText() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let add = TaskEditorModel(notes: notes, task: nil)
        add.title = "Original"
        #expect(await add.save())
        let old = try #require(notes.tasks.first)
        _ = try await s.core.tasksUpdate(UpdateTaskArgs(id: old.id, title: "Changed elsewhere", clearDue: false, clearRelated: false))
        let stale = TaskEditorModel(notes: notes, task: old)
        stale.title = "My change"
        #expect(!(await stale.save()))
        #expect(s.app.alert != nil)
        await notes.reload()
        #expect(notes.tasks.first?.title == "Changed elsewhere")
    }

    @Test func aTaskCanRelateToAnItemAndKeepsTheReferenceWhenItIsDeleted() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let place = try await s.core.locationsCreate(ProductionLocationCreateArgs(name: "Old pier"))
        let form = TaskEditorModel(notes: notes, task: nil)
        await form.loadRelatableTypes()
        #expect(!form.relatableTypes.isEmpty)
        form.relatedText = "Old pier"
        await form.searchRelated()
        let hit = try #require(form.relatedHits.first { $0.entityId == place.id })
        form.choose(hit)
        #expect(form.related?.title == "Old pier")
        #expect(form.relatedText.isEmpty)
        form.title = "Check the pier"
        #expect(await form.save())
        let task = try #require(notes.tasks.first)
        #expect(task.related?.targetId == place.id)
        #expect(task.related?.available == true)
        #expect(task.related.map { PageNav.canOpen($0.nav) } == true)

        // Marking the task done never changes the related item.
        await notes.toggleDone(task)
        let still = try await s.core.locationsGet(ProductionLocationIdArgs(id: place.id))
        #expect(still.name == "Old pier")

        // The related item is deleted: the task keeps the reference and says so.
        _ = try await s.core.locationsDelete(ProductionLocationIdArgs(id: place.id))
        await notes.reload()
        let after = try #require(notes.tasks.first)
        #expect(after.related?.available == false)
        let edit = TaskEditorModel(notes: notes, task: after)
        #expect(edit.relatedDeletedNote?.hasSuffix("The task keeps the reference.") == true)
        #expect(edit.relatedNav == nil)

        // Removing the reference is a change that can be saved.
        edit.removeRelated()
        #expect(edit.canSave)
        #expect(await edit.save())
        #expect(notes.tasks.first?.related == nil)
    }

    @Test func aJumpFromSearchOrActivityOpensTheNamedNoteOrTask() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let note = try await s.makeNote(notes, "Jump target")
        let add = TaskEditorModel(notes: notes, task: nil)
        add.title = "Jump task"
        #expect(await add.save())
        let task = try #require(notes.tasks.first)
        notes.apply(PageIntent(noteId: note.id))
        #expect(notes.openNoteId == note.id)
        notes.apply(PageIntent(taskId: task.id))
        #expect(notes.openNoteId == nil)
        #expect(notes.editingTask?.id == task.id)
        notes.closeDrawer()
        #expect(notes.openTask == nil)
    }
}

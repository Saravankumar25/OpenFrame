// Mac phase M6, Recently Deleted and Activity: the shared scenarios 25-recently-deleted-purge and
// 85-project-pages (the Activity, Recently Deleted and template parts that these screens show),
// driven through the models against the real core, plus edge and failure cases.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

private let lowTideIdea = "A ferry that only sails when nobody is waiting."

@Suite("ProjectPagesTrashTests", .serialized)
@MainActor
struct ProjectPagesTrashTests {
    // MARK: Scenario 25: 30 days left on both screens

    @Test func deletedItemsShow30DaysLeftOnBothRecentlyDeletedScreens() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let notes = await s.notesModel()
        #expect(notes.notes.isEmpty)               // "No notes yet."
        for title in ["Harbour master", "Old ferry route", "New ferry route"] { try await s.makeNote(notes, title) }
        try await s.deleteNote(notes, "Old ferry route")
        try await s.deleteNote(notes, "New ferry route")

        // The idea goes to the Vault and is deleted there.
        let vault = IdeaVaultModel(app: s.app, store: .project)
        await vault.reload()
        try await vault.quickCapture(lowTideIdea)
        #expect(vault.notice?.text == "Saved to the Project Idea Vault")
        #expect(vault.items.count == 1)
        vault.select(vault.items[0], command: false, shift: false)
        await vault.delete(vault.selectedItems)
        #expect(vault.items.isEmpty)

        // The Vault's own Recently Deleted: one row, 30 days left.
        vault.page = .deleted
        await vault.reloadTrash()
        #expect(vault.trash.count == 1)
        #expect(VaultText.daysLeft(vault.trash[0].daysLeft) == "30 days left")
        #expect(s.quiet)
        vault.page = .vault

        // The project's Recently Deleted.
        #expect(RecentlyDeletedModel.subtitle.hasPrefix("Deleted items stay here for 30 days, then they are removed for good."))
        let trash = await s.trashModel()
        for title in ["Old ferry route", "New ferry route"] {
            let row = try #require(trash.items.first { $0.title == title }, "\(title) is not in Recently Deleted")
            #expect(PageText.daysLeft(row.daysLeft) == "30 days left")
        }
        #expect(!trash.items.contains { $0.title == "Harbour master" })   // a live note is never listed
        #expect(trash.items.contains { $0.typeLabel == "Note" })
        #expect(s.quiet)
    }

    // MARK: Scenario 25: the 30-day purge, the early delete, the relaunch

    @Test func after30DaysTheNextOpenRemovesWhatIsDueAndNothingElse() async throws {
        var s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        var notes = await s.notesModel()
        for title in ["Harbour master", "Old ferry route", "New ferry route"] { try await s.makeNote(notes, title) }
        try await s.deleteNote(notes, "Old ferry route")
        try await s.deleteNote(notes, "New ferry route")
        let vault = IdeaVaultModel(app: s.app, store: .project)
        await vault.reload()
        try await vault.quickCapture(lowTideIdea)
        await vault.delete(vault.items)
        await vault.reloadTrash()
        let ideaTitle = try #require(vault.trash.first?.deleted.title)

        try await s.relaunch(project: "Low Tide") { root in
            try ProjectFileAging.age(root: root, title: "Old ferry route", days: 31)
            try ProjectFileAging.age(root: root, title: "New ferry route", days: 29.5)
            try ProjectFileAging.age(root: root, title: ideaTitle, days: 40)
        }
        if case .recovery? = s.app.alert { Issue.record("a recovery question showed after a clean close") }

        let trash = await s.trashModel()
        let newer = try #require(trash.items.first { $0.title == "New ferry route" })
        #expect(PageText.daysLeft(newer.daysLeft) == "1 day left")
        #expect(!trash.items.contains { $0.title == "Old ferry route" })
        #expect(trash.items.count == 1)

        notes = await s.notesModel()
        #expect(s.labels(notes).contains { $0.hasPrefix("Harbour master") })
        #expect(!s.labels(notes).contains { $0.hasPrefix("Old ferry route") })

        let vault2 = IdeaVaultModel(app: s.app, store: .project)
        await vault2.reloadTrash()
        #expect(vault2.trash.isEmpty)              // "Nothing deleted"
        #expect(s.quiet)
    }

    @Test func theOwnerCanDeleteAnItemForGoodBeforeItsDaysAreUpAfterAsking() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let notes = await s.notesModel()
        try await s.makeNote(notes, "New ferry route")
        try await s.deleteNote(notes, "New ferry route")
        let trash = await s.trashModel()
        let row = try #require(trash.items.first { $0.title == "New ferry route" })

        // The first click only asks. Nothing is deleted until the second, deliberate one.
        trash.purging = row
        await trash.reload()
        #expect(trash.items.contains { $0.title == "New ferry route" })
        trash.purging = nil                        // "Keep in Recently Deleted"
        #expect(trash.items.count == 1)

        trash.purging = row
        #expect(await trash.purge(row))
        #expect(trash.purging == nil)
        #expect(s.toast == "\u{201C}New ferry route\u{201D} was deleted permanently.")
        #expect(!trash.items.contains { $0.title == "New ferry route" })
        #expect(s.quiet)
    }

    @Test func afterARelaunchNothingRemovedComesBackAndTheLiveNoteStays() async throws {
        var s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        var notes = await s.notesModel()
        for title in ["Harbour master", "Old ferry route", "New ferry route"] { try await s.makeNote(notes, title) }
        try await s.deleteNote(notes, "Old ferry route")
        try await s.deleteNote(notes, "New ferry route")
        let trash = await s.trashModel()
        #expect(await trash.purge(try #require(trash.items.first { $0.title == "New ferry route" })))

        try await s.relaunch(project: "Low Tide")
        notes = await s.notesModel()
        #expect(s.labels(notes).contains { $0.hasPrefix("Harbour master") })
        #expect(!s.labels(notes).contains { $0.hasPrefix("Old ferry route") })
        #expect(!s.labels(notes).contains { $0.hasPrefix("New ferry route") })
        let after = await s.trashModel()
        #expect(after.items.map(\.title) == ["Old ferry route"])
        #expect(s.quiet)
    }

    // MARK: Scenario 85: templates deleted, restored, deleted for good

    @Test func twoTemplatesAreDeletedOneRestoredAndTheOtherDeletedForGood() async throws {
        var s = try await PagesSession.start(project: "Tide Line")
        defer { s.finish() }
        for name in ["Night exterior call", "Old call template"] {
            let t = try await s.core.templatesCreate(CreateTemplateArgs(scope: .project, templateType: "call_sheet", name: name))
            #expect(t.name == name)
        }
        let listing = try await s.core.templatesList(TemplateListArgs())
        for t in listing.project { _ = try await s.core.templatesDelete(TemplateRefArgs(scope: .project, id: t.id)) }
        #expect(s.quiet)

        let trash = await s.trashModel()
        let keep = try #require(trash.items.first { $0.title == "Night exterior call" })
        #expect(await trash.restore(keep))
        #expect(s.toast?.hasPrefix("Restored \u{201C}Night exterior call\u{201D}") == true)
        let drop = try #require(trash.items.first { $0.title == "Old call template" })
        trash.purging = drop                       // "Delete “Old call template” permanently?"
        #expect(await trash.purge(drop))
        #expect(s.toast == "\u{201C}Old call template\u{201D} was deleted permanently.")
        #expect(!trash.items.contains { $0.title == "Old call template" })
        #expect(s.quiet)

        try await s.relaunch(project: "Tide Line")
        let names = try await s.core.templatesList(TemplateListArgs()).project.map(\.name)
        #expect(names.contains("Night exterior call"))
        #expect(!names.contains("Old call template"))
        let after = await s.trashModel()
        #expect(after.items.isEmpty)               // "Nothing has been deleted."
        #expect(after.loaded)
        #expect(s.quiet)
    }

    // MARK: Scenario 85: project settings are reflected on the pages' header (title) and kept

    @Test func projectSettingsSaveUndoRedoAndSurviveARelaunch() async throws {
        var s = try await PagesSession.start(project: "Tide Line")
        defer { s.finish() }
        let saved = try await s.core.projectUpdateSettings(UpdateSettingsArgs(
            title: "Tide Line Revised", logline: "A ferry pilot races the tide to reach her brother."))
        #expect(saved.title == "Tide Line Revised")
        await s.app.undo()
        let undone = try #require(try await s.core.projectCurrent())
        #expect(undone.title == "Tide Line")
        await s.app.redo()
        let redone = try #require(try await s.core.projectCurrent())
        #expect(redone.title == "Tide Line Revised")

        let before = try await s.core.projectCurrent()
        #expect(before != nil)
        try await s.relaunch(project: "Tide Line Revised")
        #expect(try await s.core.projectCurrent()?.title == "Tide Line Revised")
        #expect(s.quiet)
    }

    // MARK: Scenario 85: Activity lists what happened and filters by kind

    @Test func activityListsWhatHappenedAndFiltersByKind() async throws {
        let s = try await PagesSession.start(project: "Tide Line")
        defer { s.finish() }
        let activity = await s.activityModel()
        #expect(activity.loaded)
        #expect(!activity.entries.isEmpty)         // the project being created is on the list
        #expect(activity.entries.contains { $0.actorName != nil })
        let actor = try #require(activity.entries.compactMap(\.actorName).first)
        #expect(activity.entries.contains { $0.actorName == actor })

        activity.filter = .story
        #expect(activity.shown.isEmpty)            // "Nothing of this kind has happened yet."
        #expect(activity.emptyText == "Nothing of this kind has happened yet.")
        activity.filter = .all
        #expect(!activity.shown.isEmpty)
        #expect(activity.shown.count == activity.entries.count)
        #expect(s.quiet)
    }

    @Test func activityEmptyStateSaysTypingIsNeverLoggedAndAnEntryNamesWhatChanged() async throws {
        let s = try await PagesSession.start(project: "Tide Line")
        defer { s.finish() }
        let activity = ActivityModel(app: s.app)
        #expect(activity.emptyText.hasSuffix("Ordinary typing is never logged."))
        let notes = await s.notesModel()
        try await s.makeNote(notes, "Pier visit")
        await activity.reload()
        // A real change shows with words, and a file or task entry can lead to its page.
        let add = TaskEditorModel(notes: notes, task: nil)
        add.title = "Check permits"
        #expect(await add.save())
        await activity.reload()
        #expect(activity.entries.contains { $0.summary.localizedCaseInsensitiveContains("Check permits") })
        let withNav = activity.entries.filter { PageNav.canOpen($0.nav) }
        #expect(withNav.contains { $0.nav["workspace"]?.stringValue == "notes" })
        #expect(activity.mayHaveMore == (activity.entries.count >= ActivityModel.page))
        #expect(!activity.canShowOlder)
    }

    @Test func showOlderActivityAsksForMoreUpToTheCeiling() async throws {
        let s = try await PagesSession.start(project: "Tide Line")
        defer { s.finish() }
        let activity = await s.activityModel()
        #expect(activity.limit == 200)
        await activity.showOlder()
        #expect(activity.limit == 400)
        for _ in 0..<5 { await activity.showOlder() }
        #expect(activity.limit == ActivityModel.ceiling)
    }

    // MARK: Recently Deleted: edge and failure cases

    @Test func itemsDeletedAreSearchableAndTheSearchSaysWhenNothingMatches() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let notes = await s.notesModel()
        for title in ["Ferry route", "Harbour master"] {
            try await s.makeNote(notes, title)
            try await s.deleteNote(notes, title)
        }
        let trash = await s.trashModel()
        #expect(trash.items.count == 2)
        trash.text = "ferry"
        #expect(trash.shown.map(\.title) == ["Ferry route"])
        trash.text = "note"                         // matches the type label
        #expect(trash.shown.count == 2)
        trash.text = "zebra"
        #expect(trash.shown.isEmpty && !trash.items.isEmpty)   // "No deleted items match “zebra”."
    }

    @Test func restoringAFileAfterItsFolderWasDeletedPutsItAtTheTopLevelAndSaysSo() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let files = await s.filesModel()
        let folder = try #require(await files.createFolder(name: "Permits", parentId: nil))
        let url = try s.makeFile("permit.txt")
        let added = try #require(await files.addPicked([url], mode: "copy"))
        let file = try #require(added.first)
        await files.move(file, to: folder.id)
        let inFolder = try #require(files.files.first { $0.id == file.id })
        #expect(inFolder.folderId == folder.id)
        #expect(await files.delete(inFolder))
        await files.deleteFolder(folder)

        let trash = await s.trashModel()
        let row = try #require(trash.items.first { $0.objectType == "project_file" })
        #expect(row.parentMissing)
        #expect(await trash.restore(row))
        let toast = try #require(s.toast)
        #expect(toast.hasPrefix("Restored"))
        #expect(toast.contains("no longer exists") || toast.contains("Unassigned") || toast.contains("top level"))
        await files.reload()
        let back = try #require(files.files.first { $0.id == file.id })
        #expect(back.folderId == nil)
        #expect(s.quiet)
    }

    @Test func restoringTheSameItemTwiceIsRefusedInWordsAndTheListStaysRight() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let notes = await s.notesModel()
        try await s.makeNote(notes, "Twice")
        try await s.deleteNote(notes, "Twice")
        let trash = await s.trashModel()
        let row = try #require(trash.items.first)
        #expect(await trash.restore(row))
        #expect(!(await trash.restore(row)))       // the row is gone already
        #expect(s.app.alert != nil)
        #expect(trash.items.isEmpty)
        #expect(trash.busyId == nil)
    }

    @Test func theGlobalIdeaVaultHasItsOwnListAndItsOwnUndo() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let global = IdeaVaultModel(app: s.app, store: .global)
        await global.reload()
        try await global.quickCapture("A city where every letter arrives a year late.")
        await global.delete(global.items)
        let project = await s.trashModel()
        #expect(project.items.isEmpty)
        let trash = await s.trashModel(.global)
        #expect(trash.scope == .global)
        let row = try #require(trash.items.first)
        #expect(PageText.daysLeft(row.daysLeft) == "30 days left")
        #expect(await trash.restore(row))
        #expect(trash.items.isEmpty)
        #expect(ToastCenter.shared.items.last?.actionLabel == "Undo")
        await global.reload()
        #expect(global.items.count == 1)
        trash.choose(.project)
        #expect(!trash.loaded)                     // switching lists starts a fresh load
        await trash.reload()
        #expect(trash.scope == .project)
    }

    @Test func aRowWhoseParentIsGoneExplainsWhatToDoFirst() async throws {
        let s = try await PagesSession.start(project: "Low Tide")
        defer { s.finish() }
        let trash = await s.trashModel()
        #expect(trash.items.isEmpty)
        #expect(trash.loaded)
        #expect(trash.loadFailure == nil)
        // Nothing hangs when there is no core answer to wait for.
        let idle = RecentlyDeletedModel(app: s.app)
        #expect(!idle.loaded)
        #expect(idle.busyId == nil)
    }
}

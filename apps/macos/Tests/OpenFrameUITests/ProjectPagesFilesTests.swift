// Mac phase M6, Project Files: the file cabinet driven through the screen's models against the
// real core. The system panels (NSOpenPanel, NSSavePanel) and a drag from Finder are operating
// system dialogs the test driver cannot press; the operations they call are tested here with the
// paths the panels would hand over, and the drag-to-folder move through the drag payload.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("ProjectPagesFilesTests", .serialized)
@MainActor
struct ProjectPagesFilesTests {
    @Test func anEmptyCabinetSaysWhatToDo() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        #expect(files.loaded)
        #expect(files.files.isEmpty)
        #expect(files.emptyMessage == nil)         // the big empty state shows instead
        #expect(files.count(in: .all) == 0)
        #expect(files.folderTree.isEmpty)
        #expect(files.folderPath(nil) == "Top level")
        #expect(s.quiet)
    }

    @Test func aFileIsCopiedIntoTheProjectAndListedAsStored() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let url = try s.makeFile("location-contract.txt")
        let added = try #require(await files.addPicked([url], mode: "copy"))
        #expect(added.count == 1)
        #expect(s.toast == "Added \u{201C}\(added[0].displayName)\u{201D}")
        let f = try #require(files.files.first)
        #expect(FilesModel.whereOf(f) == .stored)
        #expect(f.asset.storageMode == "managed")
        #expect(PageText.fileKind(mediaType: f.asset.mediaType, name: f.asset.originalName) == "Text")
        #expect(f.asset.originalName == "location-contract.txt")
        // The copy lives inside the project: removing the original does not break it.
        try FileManager.default.removeItem(at: url)
        await files.reload()
        #expect(FilesModel.whereOf(try #require(files.files.first)) == .stored)
        #expect(s.quiet)
    }

    @Test func severalFilesAreAddedTogetherAndAUndoRemovesThem() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let urls = [try s.makeFile("a.txt"), try s.makeFile("b.txt"), try s.makeFile("c.txt")]
        _ = try #require(await files.addPicked(urls, mode: "copy"))
        #expect(s.toast == "Added 3 files")
        #expect(files.files.count == 3)
        await s.app.undo()
        await files.reload()
        #expect(files.files.isEmpty)
    }

    @Test func aMissingFileIsReportedInWordsAndNothingIsAdded() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let ghost = s.root.appendingPathComponent("inbox/ghost.txt")
        let added = await files.addPicked([ghost], mode: "copy")
        #expect(added == nil)
        #expect(s.app.alert != nil)
        #expect(files.files.isEmpty)
    }

    @Test func aLinkedFileStaysWhereItIsAndShowsItsFolder() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let url = try s.makeFile("research.txt")
        _ = try #require(await files.addPicked([url], mode: "link"))
        let f = try #require(files.files.first)
        #expect(FilesModel.whereOf(f) == .linked)
        #expect(f.asset.storageMode == "external")
        #expect(f.asset.path?.hasSuffix("inbox/research.txt") == true)
        #expect(!files.isUnavailableLink(f))
        #expect(s.quiet)
    }

    @Test func aLinkedFileThatWentMissingCanBeRelinkedAndKeepsItsNotesAndFolder() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let folder = try #require(await files.createFolder(name: "Research", parentId: nil))
        let url = try s.makeFile("interview.txt", text: "Interview with the ferry pilot.")
        let added = try #require(await files.addPicked([url], mode: "link"))
        let original = try #require(added.first)
        await files.move(original, to: folder.id)
        #expect(await files.saveNotes(try #require(files.files.first), notes: "Call her back."))

        // The drive goes away.
        try FileManager.default.removeItem(at: url)
        await files.reload()
        let lost = try #require(files.files.first)
        #expect(FilesModel.whereOf(lost) == .unavailable)
        #expect(files.isUnavailableLink(lost))     // Relink is offered, never Open
        #expect(lost.id == original.id)            // the reference is never deleted

        // Opening it says why instead of failing quietly.
        await files.open(lost)
        #expect(s.app.alert != nil)
        s.app.alert = nil
        await files.exportCopy(lost, to: s.root.appendingPathComponent("copy.txt").path)
        #expect(s.app.alert != nil)
        s.app.alert = nil

        // It comes back on a new path.
        let moved = try s.makeFile("interview-moved.txt", text: "Interview with the ferry pilot.")
        #expect(await files.relink(lost, url: moved))
        #expect(s.toast == "Relinked \u{201C}\(lost.displayName)\u{201D}")
        let back = try #require(files.files.first)
        #expect(back.id == original.id)
        #expect(FilesModel.whereOf(back) == .linked)
        #expect(back.notes == "Call her back.")
        #expect(back.folderId == folder.id)
        #expect(back.asset.path?.hasSuffix("interview-moved.txt") == true)
        #expect(s.quiet)
    }

    @Test func relinkingToAFileThatIsNotThereIsRefusedInWords() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let url = try s.makeFile("deck.txt")
        _ = try #require(await files.addPicked([url], mode: "link"))
        let f = try #require(files.files.first)
        #expect(!(await files.relink(f, url: s.root.appendingPathComponent("nothing-here.txt"))))
        #expect(s.app.alert != nil)
        // A copied file cannot be relinked: only links can.
        s.app.alert = nil
        let copy = try #require(await files.addPicked([try s.makeFile("stored.txt")], mode: "copy")?.first)
        #expect(!(await files.relink(copy, url: url)))
        #expect(s.app.alert != nil)
    }

    @Test func foldersNestRenameAndDeleteMovingTheirFilesUp() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let contracts = try #require(await files.createFolder(name: "Contracts", parentId: nil))
        #expect(files.folder == .folder(contracts.id))     // the new folder opens
        #expect(s.toast == "Created folder \u{201C}Contracts\u{201D}")
        let signed = try #require(await files.createFolder(name: "Signed", parentId: contracts.id))
        #expect(files.folderPath(signed.id) == "Contracts / Signed")
        #expect(files.folderTree.map(\.depth) == [0, 1])
        #expect(files.folderName(signed.id) == "\u{201C}Contracts / Signed\u{201D}")

        // Files are added into the folder that is open.
        files.folder = .folder(contracts.id)
        let f = try #require(await files.addPicked([try s.makeFile("nda.txt")], mode: "copy")?.first)
        #expect(files.files.first { $0.id == f.id }?.folderId == contracts.id)
        #expect(files.count(in: .folder(contracts.id)) == 1)
        #expect(files.count(in: .top) == 0)
        #expect(files.shown.map(\.id) == [f.id])
        files.folder = .top
        #expect(files.shown.isEmpty)
        #expect(files.emptyMessage == "This folder is empty. Drag files onto it, or use Move to\u{2026} on a file.")

        #expect(await files.renameFolder(contracts, to: "Agreements"))
        #expect(s.toast == "Renamed folder to \u{201C}Agreements\u{201D}")
        #expect(files.folders.first { $0.id == contracts.id }?.name == "Agreements")

        let renamed = try #require(files.folders.first { $0.id == contracts.id })
        await files.deleteFolder(renamed)
        #expect(s.toast == "Deleted folder \u{201C}Agreements\u{201D}")
        await files.reload()
        #expect(files.files.first { $0.id == f.id }?.folderId == nil)   // no file is deleted
        #expect(files.folders.first { $0.id == contracts.id } == nil)
        #expect(s.quiet)
    }

    @Test func aBlankFolderNameIsRefusedAndDeletedFoldersAreOnTheRecentlyDeletedList() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        #expect(await files.createFolder(name: "   ", parentId: nil) == nil)
        #expect(s.app.alert != nil)
        s.app.alert = nil
        let f = try #require(await files.createFolder(name: "Old", parentId: nil))
        await files.deleteFolder(f)
        let trash = await s.trashModel()
        #expect(trash.items.contains { $0.title == "Old" && $0.typeLabel == "Folder" })
    }

    @Test func draggingAFileOntoAFolderMovesItAndMoveToIsTheKeyboardWay() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let folder = try #require(await files.createFolder(name: "Permits", parentId: nil))
        files.folder = .all                        // a new folder opens; add to the top level instead
        let f = try #require(await files.addPicked([try s.makeFile("permit.txt")], mode: "copy")?.first)
        #expect(f.folderId == nil)
        let payload = FileDragPayload.encode(f.id)
        #expect(FileDragPayload.decode(payload) == f.id)
        #expect(FileDragPayload.decode("something else") == nil)

        #expect(await files.drop(payloads: [payload], onto: folder.id))
        #expect(files.files.first?.folderId == folder.id)
        #expect(s.toast == "Moved \u{201C}\(f.displayName)\u{201D} to \u{201C}Permits\u{201D}")
        // Dropping something that is not a file of ours is ignored.
        #expect(!(await files.drop(payloads: ["hello"], onto: folder.id)))
        // Back to the top level through the menu path.
        await files.move(try #require(files.files.first), to: nil)
        #expect(files.files.first?.folderId == nil)
        #expect(s.toast?.hasSuffix("to the top level") == true)
        // Moving to where it already is changes nothing.
        let toastBefore = s.toast
        await files.move(try #require(files.files.first), to: nil)
        #expect(s.toast == toastBefore)
        #expect(s.quiet)
    }

    @Test func renamingAFileChangesOnlyItsShownNameAndAStaleRenameIsRefused() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let f = try #require(await files.addPicked([try s.makeFile("scan001.txt")], mode: "copy")?.first)
        #expect(await files.rename(f, to: "Location release"))
        #expect(s.toast == "Renamed file to \u{201C}Location release\u{201D}")
        let now = try #require(files.files.first)
        #expect(now.displayName == "Location release")
        #expect(now.asset.originalName == "scan001.txt")   // the file itself is not renamed
        // An empty name or the same name is not a change.
        #expect(!(await files.rename(now, to: "   ")))
        #expect(!(await files.rename(now, to: "Location release")))
        // The old copy of the file has an old revision.
        #expect(!(await files.rename(f, to: "Too late")))
        #expect(s.app.alert != nil)
    }

    @Test func theDetailsDrawerSavesTheNameOnLeavingAndNotesByThemselves() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let f = try #require(await files.addPicked([try s.makeFile("budget-draft.txt")], mode: "copy")?.first)
        files.select(f.id)
        let details = FileDetailsModel(files: files, id: f.id)
        details.delay = .milliseconds(40)
        details.sync()
        #expect(details.name == f.displayName)

        details.name = "Budget draft"
        await details.commitName()
        #expect(files.files.first?.displayName == "Budget draft")

        details.editNotes("Waiting on the producer.")
        try await Task.sleep(for: .milliseconds(400))
        #expect(files.files.first?.notes == "Waiting on the producer.")
        #expect(details.notesSaved)

        // Notes typed just before closing are not lost.
        details.delay = .seconds(30)
        details.editNotes("Waiting on the producer. Sent on Friday.")
        await details.close()
        #expect(files.selectedId == nil)
        #expect(files.files.first?.notes == "Waiting on the producer. Sent on Friday.")
        // Clearing the notes stores nothing.
        let again = FileDetailsModel(files: files, id: f.id)
        again.sync()
        again.delay = .milliseconds(10)
        again.editNotes("   ")
        await again.flushNotes()
        #expect(files.files.first?.notes == nil)
        #expect(s.quiet)
    }

    @Test func anUndoneRenameReachesTheDrawerButNeverOverwritesTheFieldBeingEdited() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let f = try #require(await files.addPicked([try s.makeFile("plain.txt")], mode: "copy")?.first)
        #expect(await files.rename(f, to: "Renamed"))
        let details = FileDetailsModel(files: files, id: f.id)
        details.sync()
        #expect(details.name == "Renamed")
        await s.app.undo()
        await files.reload()
        details.sync()
        #expect(details.name == f.displayName)
        details.nameFocused = true
        details.name = "Half typed"
        await s.app.redo()
        await files.reload()
        details.sync()
        #expect(details.name == "Half typed")
        details.cancelName()
        #expect(details.name == files.files.first?.displayName)
    }

    @Test func searchFindsFilesByNameOriginalNameNotesAndKind() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        _ = try #require(await files.addPicked([try s.makeFile("alpha-permit.txt"), try s.makeFile("beta-deck.txt")], mode: "copy"))
        let beta = try #require(files.files.first { $0.asset.originalName == "beta-deck.txt" })
        #expect(await files.saveNotes(beta, notes: "Pitch for the festival."))
        files.text = "permit"
        #expect(files.shown.map(\.asset.originalName) == ["alpha-permit.txt"])
        files.text = "festival"
        #expect(files.shown.map(\.asset.originalName) == ["beta-deck.txt"])
        files.text = "text"
        #expect(files.shown.count == 2)
        files.text = "zebra"
        #expect(files.shown.isEmpty)
        #expect(files.emptyMessage == "No files match \u{201C}zebra\u{201D}.")
    }

    @Test func deletingAFileMovesItToRecentlyDeletedAndRestoreBringsItBackToItsFolder() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let folder = try #require(await files.createFolder(name: "Permits", parentId: nil))
        let f = try #require(await files.addPicked([try s.makeFile("permit.txt")], mode: "copy")?.first)
        await files.move(f, to: folder.id)
        files.select(f.id)
        let current = try #require(files.current)
        #expect(await files.delete(current))
        #expect(s.toast == "Moved \u{201C}\(f.displayName)\u{201D} to Recently Deleted")
        #expect(files.files.isEmpty)
        #expect(files.selectedId == nil)

        let trash = await s.trashModel()
        let row = try #require(trash.items.first)
        #expect(row.typeLabel == "File")
        #expect(!row.parentMissing)
        #expect(await trash.restore(row))
        await files.reload()
        #expect(files.files.first?.id == f.id)
        #expect(files.files.first?.folderId == folder.id)
        #expect(s.quiet)
    }

    @Test func aCopyIsExportedWhereThePersonChoseAndTheProjectIsUnchanged() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let f = try #require(await files.addPicked([try s.makeFile("one-sheet.txt", text: "The one sheet.")], mode: "copy")?.first)
        let dest = s.root.appendingPathComponent("exported-one-sheet.txt")
        await files.exportCopy(f, to: dest.path)
        #expect(try String(contentsOf: dest, encoding: .utf8) == "The one sheet.")
        #expect(s.toast == "Saved a copy of \u{201C}\(f.displayName)\u{201D}.")
        #expect(ToastCenter.shared.items.last?.actionLabel == "Show")
        #expect(files.files.count == 1)
        #expect(s.quiet)
    }

    @Test func aJumpFromActivityOrSearchSelectsTheNamedFile() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let f = try #require(await files.addPicked([try s.makeFile("target.txt")], mode: "copy")?.first)
        files.apply(PageIntent(fileId: f.id))
        #expect(files.current?.id == f.id)
        // A file that is deleted underneath the open drawer closes it instead of showing a stale one.
        _ = try await s.core.filesDelete(IdArgs(id: f.id))
        await files.reload()
        #expect(files.current == nil)
        #expect(files.selectedId == nil)
        // An activity entry for the new file leads to the Files page.
        let activity = await s.activityModel()
        #expect(activity.entries.contains { $0.nav["workspace"]?.stringValue == "files" || $0.targetType == "project_file" })
    }

    @Test func filesAreKeptAfterARelaunch() async throws {
        var s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        var files = await s.filesModel()
        let folder = try #require(await files.createFolder(name: "Permits", parentId: nil))
        let f = try #require(await files.addPicked([try s.makeFile("kept.txt")], mode: "copy")?.first)
        await files.move(f, to: folder.id)
        _ = await files.saveNotes(try #require(files.files.first), notes: "Keep me.")
        try await s.relaunch(project: "Paper Trail")
        files = await s.filesModel()
        #expect(files.files.count == 1)
        #expect(files.files.first?.notes == "Keep me.")
        #expect(files.folders.map(\.name) == ["Permits"])
        #expect(FilesModel.whereOf(try #require(files.files.first)) == .stored)
        #expect(s.quiet)
    }

    // MARK: The App Sandbox: files kept outside the project

    @Test func aLinkedFilesBookmarkIsRememberedAndAccessIsHeldWhileTheCoreWorks() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let suite = "pages-bookmarks-\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let access = ExternalAccess(defaults: defaults)
        let url = try s.makeFile("outside.txt")
        access.remember(url)
        #expect(access.hasBookmark(for: url.path))
        let ran = await access.holding(path: url.path) { 42 }
        #expect(ran == 42)                         // the work runs with access held
        let none = await access.holding(path: "/nowhere/at/all") { "still runs" }
        #expect(none == "still runs")              // no bookmark: the work still runs, the core decides
        access.forget(path: url.path)
        #expect(!access.hasBookmark(for: url.path))
    }
}

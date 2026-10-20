// Mac phase M6, project pages: the shared test rig (a real core on a temporary folder, a project
// open, screens driven through their models exactly as the views drive them) and the rules of
// the wording helpers. The scenarios themselves are in the other ProjectPages files.

import Foundation
import SQLite3
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

/// One app run on its own data folder, with one project open.
@MainActor
struct PagesSession {
    let root: URL
    var app: AppModel

    static func start(project: String) async throws -> PagesSession {
        ToastCenter.shared.dismissAll()
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: project, projectType: .featureFilm))
        #expect(ok)
        return PagesSession(root: root, app: app)
    }

    var core: Core { app.core! }
    var quiet: Bool { app.alert == nil }
    /// The newest toast, as the person reads it.
    var toast: String? { ToastCenter.shared.items.last?.text }
    var toasts: [String] { ToastCenter.shared.items.map(\.text) }

    func finish() {
        ToastCenter.shared.dismissAll()
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    /// Close the project, quit, start again on the same data and open the project. `whileClosed`
    /// runs with the app stopped (a test setup step, like the shared scenario's file edit).
    mutating func relaunch(project title: String, whileClosed: (URL) throws -> Void = { _ in }) async throws {
        let recent = try await core.projectListRecent(ListRecentArgs(includeArchived: false))
        guard let row = recent.first(where: { $0.title == title }) else {
            Issue.record("project \(title) is not in the recent list")
            return
        }
        await app.closeProject()
        app.shutdown()
        try await Task.sleep(for: .milliseconds(1500))
        try whileClosed(root)
        let next = Fixture.model(root)
        await next.start()
        await next.openProject(path: row.path)
        app = next
    }

    // Models

    func notesModel() async -> NotesModel {
        let m = NotesModel(app: app)
        await m.reload()
        return m
    }

    func trashModel(_ scope: StoreSel = .project) async -> RecentlyDeletedModel {
        let m = RecentlyDeletedModel(app: app)
        m.choose(scope)
        await m.reload()
        return m
    }

    func filesModel() async -> FilesModel {
        let m = FilesModel(app: app, access: ExternalAccess(defaults: UserDefaults(suiteName: "pages-tests-\(root.lastPathComponent)")!))
        await m.reload()
        return m
    }

    func activityModel() async -> ActivityModel {
        let m = ActivityModel(app: app)
        await m.reload()
        return m
    }

    /// "Make a note X": New Note, a title, a body, Close.
    @discardableResult
    func makeNote(_ notes: NotesModel, _ title: String, body: String? = nil) async throws -> NoteDto {
        let created = try #require(await notes.newNote())
        let editor = NoteEditorModel(notes: notes, id: created.id)
        await editor.refresh()
        editor.edit(.title, to: title)
        editor.edit(.body, to: body ?? "\(title): written by the end-to-end suite.")
        await editor.close()
        #expect(notes.openNoteId == nil)
        let found = try #require(notes.notes.first { PageText.noteLabel(title: $0.title, body: $0.body).hasPrefix(title) })
        return found
    }

    /// "Delete the note X": open it, Delete in the drawer.
    func deleteNote(_ notes: NotesModel, _ title: String) async throws {
        let note = try #require(notes.notes.first { PageText.noteLabel(title: $0.title, body: $0.body).hasPrefix(title) })
        notes.showNote(note.id)
        let editor = NoteEditorModel(notes: notes, id: note.id)
        await editor.refresh()
        #expect(await editor.delete())
        #expect(toast == "Moved note \u{201C}\(title)\u{201D} to Recently Deleted")
        #expect(!notes.notes.contains { PageText.noteLabel(title: $0.title, body: $0.body).hasPrefix(title) })
    }

    func labels(_ notes: NotesModel) -> [String] { notes.notes.map { PageText.noteLabel(title: $0.title, body: $0.body) } }

    /// A plain file on disk for the Files tests.
    func makeFile(_ name: String, text: String = "Contract text.") throws -> URL {
        let dir = root.appendingPathComponent("inbox", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = dir.appendingPathComponent(name)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }
}

/// The edit the shared purge scenario makes to the project file while the app is closed: delete
/// times moved back so the items look older.
enum ProjectFileAging {
    static let day = 86_400_000.0

    static func projectDatabase(under root: URL) throws -> URL {
        let found = FileManager.default.enumerator(at: root.appendingPathComponent("projects"), includingPropertiesForKeys: nil)?
            .compactMap { $0 as? URL }
            .filter { $0.lastPathComponent == "project.sqlite" && !$0.pathComponents.contains("backups") } ?? []
        #expect(found.count == 1, "expected one project file, found \(found.map(\.path))")
        return try #require(found.first)
    }

    private static func exec(_ db: OpaquePointer, _ sql: String) throws {
        var err: UnsafeMutablePointer<CChar>?
        guard sqlite3_exec(db, sql, nil, nil, &err) == SQLITE_OK else {
            let message = err.map { String(cString: $0) } ?? "unknown"
            sqlite3_free(err)
            throw NSError(domain: "aging", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
        }
    }

    private static func text(_ db: OpaquePointer, _ sql: String) -> [String] {
        var stmt: OpaquePointer?
        var out: [String] = []
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return out }
        defer { sqlite3_finalize(stmt) }
        while sqlite3_step(stmt) == SQLITE_ROW {
            if let c = sqlite3_column_text(stmt, 0) { out.append(String(cString: c)) }
        }
        return out
    }

    /// Moves the delete of every item with this title back by `days`, the record and the object.
    static func age(root: URL, title: String, days: Double) throws {
        var db: OpaquePointer?
        let path = try projectDatabase(under: root).path
        guard sqlite3_open(path, &db) == SQLITE_OK, let db else { throw NSError(domain: "aging", code: 2) }
        defer { sqlite3_close(db) }
        let safe = title.replacingOccurrences(of: "'", with: "''")
        let rows = text(db, "SELECT id || '|' || object_id || '|' || table_name FROM deleted_item WHERE title = '\(safe)'")
        #expect(rows.count == 1, "expected one Recently Deleted record for \(title), found \(rows.count)")
        for row in rows {
            let p = row.split(separator: "|").map(String.init)
            #expect(p[2].allSatisfy { $0.isLowercase || $0 == "_" }, "a plain table name")
            let shift = Int((days * day).rounded())
            try exec(db, "UPDATE deleted_item SET deleted_at = deleted_at - \(shift) WHERE id = '\(p[0])'")
            try exec(db, "UPDATE \"\(p[2])\" SET deleted_at = deleted_at - \(shift) WHERE id = '\(p[1])' AND deleted_at IS NOT NULL")
        }
    }
}

@Suite("ProjectPagesTextTests")
@MainActor
struct ProjectPagesTextTests {
    private func ms(_ date: Date) -> Double { date.timeIntervalSince1970 * 1000 }

    @Test func daysLeftSaysWholeDaysAndNeverZero() {
        #expect(PageText.daysLeft(30) == "30 days left")
        #expect(PageText.daysLeft(1) == "1 day left")
        #expect(PageText.daysLeft(0.2) == "1 day left")
    }

    @Test func dayWordsFollowTheWindowsWording() {
        let now = Date()
        #expect(PageText.dayWord(ms: ms(now), now: now) == "Today")
        #expect(PageText.dayWord(ms: ms(now.addingTimeInterval(-86_400)), now: now) == "Yesterday")
        #expect(PageText.dayWord(ms: ms(now.addingTimeInterval(-3 * 86_400)), now: now) == "3 days ago")
        #expect(PageText.dayWord(ms: ms(now.addingTimeInterval(-40 * 86_400)), now: now).contains(String(Calendar.current.component(.year, from: now.addingTimeInterval(-40 * 86_400)))))
        #expect(PageText.dayAndTime(ms: ms(now), now: now).hasPrefix("Today "))
        #expect(PageText.dayAndTime(ms: ms(now.addingTimeInterval(-86_400)), now: now).hasPrefix("Yesterday "))
    }

    @Test func aNoteIsNamedByItsTitleThenItsFirstLine() {
        #expect(PageText.noteLabel(title: " Location scout ", body: "x") == "Location scout")
        #expect(PageText.noteLabel(title: nil, body: "\n\nFirst line\nSecond") == "First line")
        #expect(PageText.noteLabel(title: "", body: "   ") == "Untitled note")
        let long = String(repeating: "a", count: 80)
        #expect(PageText.noteLabel(title: nil, body: long) == String(repeating: "a", count: 60) + "\u{2026}")
        #expect(PageText.notePreview(title: nil, body: "First line\nSecond line\nThird") == "Second line Third")
        #expect(PageText.notePreview(title: "T", body: "One\nTwo") == "One Two")
    }

    @Test func aDueDateIsOverdueOnlyBeforeToday() {
        let now = Date()
        #expect(PageText.dueLabel(ms: ms(now.addingTimeInterval(-3 * 86_400)), now: now).overdue)
        #expect(!PageText.dueLabel(ms: PageText.dueMs(day: now), now: now).overdue)
        #expect(!PageText.dueLabel(ms: ms(now.addingTimeInterval(3 * 86_400)), now: now).overdue)
        #expect(PageText.dueLabel(ms: ms(now), now: now).text.hasPrefix("due "))
        // A due date is kept at noon of the chosen day.
        let noon = Date(timeIntervalSince1970: PageText.dueMs(day: now) / 1000)
        #expect(Calendar.current.component(.hour, from: noon) == 12)
        #expect(Calendar.current.isDate(noon, inSameDayAs: now))
    }

    @Test func fileKindsAndBadges() {
        #expect(PageText.fileKind(mediaType: "application/pdf", name: "a.pdf") == "PDF")
        #expect(PageText.fileKind(mediaType: "text/csv", name: "budget.csv") == "Spreadsheet")
        #expect(PageText.fileKind(mediaType: "text/plain", name: "act1.fountain") == "Screenplay")
        #expect(PageText.fileKind(mediaType: "text/plain", name: "notes.txt") == "Text")
        #expect(PageText.fileKind(mediaType: "image/png", name: "a.png") == "Image")
        #expect(PageText.fileKind(mediaType: "application/octet-stream", name: "logo.psd") == "Artwork")
        #expect(PageText.fileKind(mediaType: "application/octet-stream", name: "thing") == "File")
        #expect(PageText.fileBadge(mediaType: "application/pdf", name: "a.pdf") == "PDF")
        #expect(PageText.fileBadge(mediaType: "application/octet-stream", name: "scene.fdx") == "FDX")
        #expect(PageText.fileBadge(mediaType: "application/octet-stream", name: "thing") == "FILE")
        #expect(PageText.fileSize(nil) == "")
        #expect(PageText.fileSize(512) == "512 B")
        #expect(PageText.fileSize(2048) == "2 KB")
        #expect(PageText.fileSize(5 * 1024 * 1024) == "5.0 MB")
    }

    @Test func anActivityLineReadsAfterTheName() {
        #expect(PageText.sentence(summary: "Renamed file to \u{201C}X\u{201D}") == "renamed file to \u{201C}X\u{201D}")
        #expect(PageText.sentence(summary: "Project created") == "Project created")
        #expect(PageText.sentence(summary: "Added 3 files") == "added 3 files")
    }

    @Test func aRestoreSaysWhereTheItemWent() {
        let item = DeletedItemView(id: "d", objectType: "story_scene_card", objectId: "o", title: "Maybe Scene 17", typeLabel: "Scene Card",
                                   wasIn: "Beat Board", parentMissing: false, deletedAt: 0, daysLeft: 30)
        #expect(PageText.restoredMessage(item, restored: nil) == "Restored \u{201C}Maybe Scene 17\u{201D} to Beat Board")
        var gone = item
        gone.parentMissing = true
        #expect(PageText.restoredMessage(gone, restored: nil).hasSuffix("so it was put in Unassigned."))
        var untitled = item
        untitled.title = nil
        #expect(PageText.restoredMessage(untitled, restored: nil) == "Restored the scene card to Beat Board")
        let row = DeletedItemRow(id: "d", objectType: "x", objectId: "o", tableName: "t", deletedAt: 0, restoredNote: "It went to Unassigned.")
        #expect(PageText.restoredMessage(item, restored: row) == "Restored \u{201C}Maybe Scene 17\u{201D}. It went to Unassigned.")
    }

    @Test func aMissingParentIsExplainedPerKind() {
        #expect(PageText.parentMissingNote(objectType: "screenplay_draft") == "Its screenplay is in Recently Deleted too. Restore the screenplay first.")
        #expect(PageText.parentMissingNote(objectType: "screenplay") == "Its episode is in Recently Deleted too. Restore the episode first.")
        #expect(PageText.parentMissingNote(objectType: "project_file") == "That place no longer exists, so it restores to Unassigned")
    }

    @Test func activityFiltersKeepTheirKinds() {
        func entry(_ action: String) -> ActivityEntry {
            ActivityEntry(id: action, at: 0, action: action, summary: "s", origin: "app", nav: .null)
        }
        #expect(ActivityFilter.all.matches(entry("anything")))
        #expect(ActivityFilter.drafts.matches(entry("screenplay.create_draft")))
        #expect(!ActivityFilter.drafts.matches(entry("story.add_act")))
        #expect(ActivityFilter.story.matches(entry("vault.create")))
        #expect(ActivityFilter.production.matches(entry("callsheet.create")))
        #expect(ActivityFilter.exchange.matches(entry("import.fountain")))
        #expect(!ActivityFilter.exchange.matches(entry("notes.create")))
    }

    @Test func aNavTargetOpensOnlyWhenTheCoreSentOne() {
        #expect(!PageNav.canOpen(.null))
        #expect(PageNav.canOpen(.object(["workspace": .string("files"), "fileId": .string("f1")])))
        PageNav.pending = PageIntent()
        PageNav.open(.null)
        #expect(PageNav.pending.isEmpty)
        PageNav.open(.object(["workspace": .string("notes"), "taskId": .string("t1")]))
        #expect(PageNav.pending.taskId == "t1")
        #expect(PageNav.take().taskId == "t1")
        #expect(PageNav.pending.isEmpty)
    }
}

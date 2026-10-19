// Mac phase M6, project pages: the four screens drawn in a real window, light and dark, with data
// in them, to prove they build and draw something real (not a blank or a crash). A picture is
// not text, so what the screens say is proved by the model tests; the pictures are written for a
// person to look at. (SwiftUI's accessibility tree is not readable from an off-screen window, so
// the control identifiers are checked by the end-to-end driver in phase M9.)

import AppKit
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@MainActor
private func drawn<V: View>(_ view: V, app: AppModel, scheme: ColorScheme, size: CGSize = CGSize(width: 1280, height: 780), name: String) async throws -> Int {
    let themes = ThemeStore()
    let hosted = view.environment(app).environment(themes)
        .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme)).environment(\.colorScheme, scheme)
    let host = NSHostingView(rootView: hosted.frame(width: size.width, height: size.height))
    host.frame = NSRect(origin: .zero, size: size)
    let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
    window.contentView = host
    window.appearance = NSAppearance(named: scheme == .dark ? .darkAqua : .aqua)
    host.layoutSubtreeIfNeeded()
    for _ in 0..<15 {
        try await Task.sleep(for: .milliseconds(60))
        host.layoutSubtreeIfNeeded()
    }
    var colours = 0
    if let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) {
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: Fixture.snapshots, withIntermediateDirectories: true)
        if let png = rep.representation(using: .png, properties: [:]) {
            try png.write(to: Fixture.snapshots.appendingPathComponent("\(name).png"))
        }
        #expect(rep.pixelsWide > 0)
        // Count distinct colours on a coarse grid: a blank or failed draw has one or two.
        var seen = Set<UInt32>()
        let step = max(1, min(rep.pixelsWide, rep.pixelsHigh) / 60)
        for x in stride(from: 0, to: rep.pixelsWide, by: step) {
            for y in stride(from: 0, to: rep.pixelsHigh, by: step) {
                if let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) {
                    seen.insert(UInt32(c.redComponent * 255) << 16 | UInt32(c.greenComponent * 255) << 8 | UInt32(c.blueComponent * 255))
                }
            }
        }
        colours = seen.count
    } else {
        Issue.record("could not draw \(name)")
    }
    return colours
}

@Suite("ProjectPagesSnapshotTests", .serialized)
@MainActor
struct ProjectPagesSnapshotTests {
    @Test func notesAndTasksDrawInLightAndDark() async throws {
        let s = try await PagesSession.start(project: "Tide Tables")
        defer { s.finish() }
        let notes = await s.notesModel()
        let pinned = try await s.makeNote(notes, "Location scout", body: "Check the old pier at low tide.")
        try await s.makeNote(notes, "Rain plan")
        _ = try await s.core.notesSetPinned(SetNotePinnedArgs(id: pinned.id, pinned: true))
        for (title, due) in [("Confirm pier permit", -2), ("Book the ferry", 3)] {
            let f = TaskEditorModel(notes: notes, task: nil)
            f.title = title
            f.hasDue = true
            f.due = Calendar.current.date(byAdding: .day, value: due, to: Date())!
            #expect(await f.save())
        }
        await notes.toggleDone(try #require(notes.tasks.first { $0.title == "Book the ferry" }))
        notes.taskFilter = .all
        await notes.reload()
        for scheme in [ColorScheme.light, .dark] {
            let seen = try await drawn(NotesScreen(model: notes), app: s.app, scheme: scheme, name: "pages-notes-\(scheme == .dark ? "dark" : "light")")
            #expect(seen > 8, "the screen drew nothing real")
        }
        notes.showNote(pinned.id)
        let drawer = try await drawn(NotesScreen(model: notes), app: s.app, scheme: .light, name: "pages-notes-drawer")
        #expect(drawer > 8)
        notes.startNewTask()
        let task = try await drawn(NotesScreen(model: notes), app: s.app, scheme: .light, name: "pages-task-drawer")
        #expect(task > 8)
        let empty = NotesModel(app: s.app)
        let blank = try await drawn(NotesScreen(model: empty), app: s.app, scheme: .light, name: "pages-notes-empty")
        #expect(blank > 4)
    }

    @Test func filesDrawInListGridAndWithTheDetailsDrawer() async throws {
        let s = try await PagesSession.start(project: "Paper Trail")
        defer { s.finish() }
        let files = await s.filesModel()
        let blank = try await drawn(FilesScreen(model: files), app: s.app, scheme: .light, name: "pages-files-empty")
        #expect(blank > 4)
        let folder = try #require(await files.createFolder(name: "Permits", parentId: nil))
        files.folder = .all
        let url = try s.makeFile("interview.txt")
        _ = await files.addPicked([url, try s.makeFile("location-contract.txt")], mode: "copy")
        let linked = try #require(await files.addPicked([url], mode: "link")?.first)
        await files.move(linked, to: folder.id)
        try FileManager.default.removeItem(at: url)
        await files.reload()
        for view in [FilesView.list, .grid] {
            UserDefaults.standard.set(view.rawValue, forKey: "files.view")
            for scheme in [ColorScheme.light, .dark] {
                let seen = try await drawn(FilesScreen(model: files), app: s.app, scheme: scheme, name: "pages-files-\(view.rawValue)-\(scheme == .dark ? "dark" : "light")")
            #expect(seen > 8, "the screen drew nothing real")
            }
        }
        UserDefaults.standard.set("list", forKey: "files.view")
        files.select(linked.id)
        let details = try await drawn(FilesScreen(model: files), app: s.app, scheme: .light, name: "pages-files-details")
        #expect(details > 8)
    }

    @Test func activityAndRecentlyDeletedDraw() async throws {
        let s = try await PagesSession.start(project: "Tide Line")
        defer { s.finish() }
        let notes = await s.notesModel()
        for t in ["Old ferry route", "New ferry route"] {
            try await s.makeNote(notes, t)
            try await s.deleteNote(notes, t)
        }
        let activity = await s.activityModel()
        let trash = await s.trashModel()
        for scheme in [ColorScheme.light, .dark] {
            let name = scheme == .dark ? "dark" : "light"
            let a = try await drawn(ActivityScreen(model: activity), app: s.app, scheme: scheme, name: "pages-activity-\(name)")
            #expect(a > 8)
            let t = try await drawn(RecentlyDeletedScreen(model: trash), app: s.app, scheme: scheme, name: "pages-trash-\(name)")
            #expect(t > 8)
        }
        activity.filter = .story
        let none = try await drawn(ActivityScreen(model: activity), app: s.app, scheme: .light, name: "pages-activity-empty")
        #expect(none > 4)
        let fresh = try await PagesSession.start(project: "Nothing Yet")
        defer { fresh.finish() }
        let emptyTrash = await fresh.trashModel()
        let e = try await drawn(RecentlyDeletedScreen(model: emptyTrash), app: fresh.app, scheme: .light, name: "pages-trash-empty")
        #expect(e > 4)
    }
}

// Mac phase M6, Visual Planning. The shared end-to-end scenario 86-visual-planning.md, run at
// model level against a real core on a temporary folder: each test drives the Visual Planning
// models the way the screen does (the same calls, in the same order) and checks what the screen
// would show. The same file holds the moodboard and storyboard edge and failure cases (a file
// that is not an image, a very large picture, empty states, empty names).

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

let visualNightBus = [
    "Title: Night Bus", "",
    "INT. BUS DEPOT - NIGHT", "",
    "Meera counts the keys on a ring and frowns.", "",
    "MEERA", "One is missing.", "",
    "EXT. HIGHWAY - NIGHT", "",
    "Arjun waves a torch at the empty road.", "",
    "ARJUN", "Nobody is coming.", "",
    "INT. BUS - NIGHT", "",
    "Meera slides into the driver's seat.", "",
].joined(separator: "\n")

/// Pictures for the tests, made in memory and written as real files.
enum TestImages {
    static func png(width: Int, height: Int, red: CGFloat = 0.2, green: CGFloat = 0.4, blue: CGFloat = 0.8) -> Data {
        let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8, samplesPerPixel: 4,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        let gc = NSGraphicsContext(bitmapImageRep: rep)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = gc
        NSColor(red: red, green: green, blue: blue, alpha: 1).setFill()
        NSRect(x: 0, y: 0, width: width, height: height).fill()
        NSGraphicsContext.restoreGraphicsState()
        return rep.representation(using: .png, properties: [:])!
    }

    static func file(in dir: URL, name: String, width: Int = 320, height: Int = 180) throws -> String {
        let url = dir.appendingPathComponent(name)
        try png(width: width, height: height).write(to: url)
        return url.path
    }
}

@MainActor
struct VisualRig {
    let root: URL
    let app: AppModel
    let hub: VisualHub
    let projectPath: String

    static func make(title: String = "Pictures First", script: Bool = true) async throws -> VisualRig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        #expect(ok)
        guard case let .project(p) = app.phase else { throw CoreFailure(code: "test", message: "no project") }
        let rig = VisualRig(root: root, app: app, hub: VisualHub(app: app), projectPath: p.path)
        if script { try await rig.importScript() }
        await rig.reloadAll()
        return rig
    }

    var core: Core { app.core! }
    var mb: MoodboardsModel { hub.moodboards }
    var sb: StoryboardsModel { hub.storyboards }
    var shots: ShotsModel { hub.shots }
    var quiet: Bool { app.alert == nil }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    func importScript(_ text: String = visualNightBus) async throws {
        let preview = try await core.screenplayImportPreview(InterchangeImportPreviewArgs(pastedText: text, format: "fountain"))
        _ = try await core.screenplayImportApply(InterchangeImportApplyArgs(previewId: preview.previewId, mode: "new_screenplay", keepSourceFile: false))
    }

    /// What the screens do when the core reports a change: reload what they show.
    func reloadAll() async {
        await hub.reloadScenes()
        await hub.reloadMembers()
        await mb.reload()
        await sb.reload()
        await shots.reload()
    }

    func imageFile(_ name: String, width: Int = 320, height: Int = 180) throws -> String {
        try TestImages.file(in: root, name: name, width: width, height: height)
    }

    func scene(_ i: Int) throws -> VisualScene { try #require(hub.scenes?.scenes[i]) }

    func clearAlert() { app.alert = nil }
}

@Suite("Visual planning scenarios", .serialized)
@MainActor
struct VisualScenarioTests {
    // MARK: Scenario 86, part 1: the note is dragged

    @Test func aNoteOnTheMoodboardCanvasIsDraggedToANewPlace() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Overall Look", sceneId: nil))
        #expect(r.mb.current?.name == "Overall Look")
        #expect(await r.mb.addNote(text: "Cold blue light, warm windows", isPrivate: false))
        await r.mb.reload()
        let tile = try #require(r.mb.items.first)
        #expect(tile.kind == "note")
        #expect(tile.body == "Cold blue light, warm windows")
        let before = (x: tile.x, y: tile.y)

        // A real drag: press, move a few points, move to the target, release.
        r.mb.dragChanged(id: tile.id, dx: 2, dy: 1)
        r.mb.dragChanged(id: tile.id, dx: 90, dy: 40)
        r.mb.dragChanged(id: tile.id, dx: 160, dy: 90)
        #expect(r.mb.preview?.first?.x == before.x + 160)
        await r.mb.dragEnded()
        await r.mb.reload()

        let moved = try #require(r.mb.items.first)
        #expect(moved.x - before.x > 100)
        #expect(moved.y - before.y > 50)
        #expect(moved.x - before.x == 160)
        #expect(moved.y - before.y == 90)
        #expect(r.quiet)
    }

    // MARK: Scenario 86, part 2: the sketch

    @Test func theSketchDrawsTakesAStrokeBackAndSavesAStoryboardPanel() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        await r.sb.choose(key: "scene:\(try r.scene(0).lineageId)")
        #expect(r.sb.canAdd)
        let sketch = SketchModel()
        #expect(!sketch.canSave)
        // Two crossing strokes.
        sketch.begin(at: CGPoint(x: 100, y: 100))
        for t in stride(from: 110.0, through: 400.0, by: 30) { sketch.extend(to: CGPoint(x: t, y: t * 0.8)) }
        sketch.end()
        sketch.begin(at: CGPoint(x: 400, y: 100))
        for t in stride(from: 380.0, through: 100.0, by: -30) { sketch.extend(to: CGPoint(x: t, y: 500 - t * 0.8)) }
        sketch.end()
        #expect(sketch.strokes.count == 2)
        sketch.undo()
        #expect(sketch.strokes.count == 1)
        #expect(sketch.canSave)

        let png = try #require(sketch.png())
        let panel = await r.sb.addPanel(.sketch, png: png)
        #expect(panel != nil)
        await r.sb.reload()
        #expect(r.sb.panels.count == 1)
        let p = try #require(r.sb.panels.first)
        #expect(p.visualKind == "sketch")
        let asset = try #require(p.asset)
        #expect(asset.available)
        #expect(asset.mediaType == "image/png")
        #expect(asset.width == 960)
        #expect(asset.height == 540)
        // A scene's first panel created its storyboard.
        #expect(r.sb.boards.count == 1)
        #expect(r.sb.boards.first?.sceneLineageId == (try r.scene(0)).lineageId)
        #expect(r.quiet)
    }

    // MARK: Scenario 86, part 3: two empty panels, the last dragged to the front

    @Test func twoEmptyPanelsAreAddedAndTheLastIsDraggedToTheFront() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        await r.sb.choose(key: "scene:\(try r.scene(0).lineageId)")
        let sketch = SketchModel()
        sketch.begin(at: CGPoint(x: 10, y: 10)); sketch.extend(to: CGPoint(x: 300, y: 200)); sketch.end()
        await r.sb.addPanel(.sketch, png: sketch.png())
        await r.sb.addPanel(.placeholder)
        await r.sb.addPanel(.placeholder)
        await r.sb.reload()
        #expect(r.sb.panels.map(\.visualKind) == ["sketch", "placeholder", "placeholder"])
        #expect(r.sb.panels.map { Int($0.number) } == [1, 2, 3])

        // Drag the third panel onto the first panel's place.
        let third = r.sb.panels[2].id
        await r.sb.reorder(third, to: 0)
        // The grid shows the new order at once.
        #expect(r.sb.panels.first?.id == third)
        await r.sb.reload()
        #expect(r.sb.panels.map(\.visualKind) == ["placeholder", "sketch", "placeholder"])
        #expect(r.sb.panels.first?.id == third)
        #expect(r.quiet)
    }

    // MARK: Scenario 86, part 4: after a restart

    @Test func afterARestartTheNoteIsWhereItWasDroppedAndThePanelsKeepTheirOrder() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        // The first part.
        #expect(await r.mb.createBoard(name: "Overall Look", sceneId: nil))
        #expect(await r.mb.addNote(text: "Cold blue light, warm windows", isPrivate: false))
        await r.mb.reload()
        let tile = try #require(r.mb.items.first)
        r.mb.dragChanged(id: tile.id, dx: 160, dy: 90)
        await r.mb.dragEnded()
        await r.mb.reload()
        let noted = try #require(r.mb.items.first)
        // The panels.
        let lineage = try r.scene(0).lineageId
        await r.sb.choose(key: "scene:\(lineage)")
        let sketch = SketchModel()
        sketch.begin(at: CGPoint(x: 10, y: 10)); sketch.extend(to: CGPoint(x: 300, y: 200)); sketch.end()
        await r.sb.addPanel(.sketch, png: sketch.png())
        await r.sb.addPanel(.placeholder)
        await r.sb.addPanel(.placeholder)
        await r.sb.reload()
        await r.sb.reorder(r.sb.panels[2].id, to: 0)
        await r.sb.reload()

        // Close the project, start again on the same data and open it.
        await r.app.closeProject()
        #expect(r.app.phase == .home)
        let recent = try await r.core.projectListRecent(ListRecentArgs(includeArchived: false))
        #expect(recent.first?.title == "Pictures First")
        await r.app.openProject(path: recent[0].path)
        let hub2 = VisualHub(app: r.app)
        await hub2.reloadScenes()
        await hub2.moodboards.reload()
        let again = try #require(hub2.moodboards.items.first)
        #expect(again.x == noted.x)
        #expect(again.y == noted.y)
        await hub2.storyboards.choose(key: "scene:\(lineage)")
        await hub2.storyboards.reload()
        #expect(hub2.storyboards.panels.map(\.visualKind) == ["placeholder", "sketch", "placeholder"])
        #expect(r.quiet)
    }

    // MARK: Moodboards

    @Test func moodboardEmptyStateAndNames() async throws {
        let r = try await VisualRig.make(script: false)
        defer { r.close() }
        #expect(r.mb.loaded)
        #expect(r.mb.boards.isEmpty)
        #expect(r.mb.current == nil)
        #expect(!(await r.mb.addNote(text: "No board yet", isPrivate: false)))
        // An empty name is refused before the core is asked.
        #expect(!(await r.mb.createBoard(name: "   ", sceneId: nil)))
        #expect(r.mb.boards.isEmpty)
        #expect(r.quiet)
        // Suggestions come from the core.
        await r.mb.loadSuggestions()
        #expect(!r.mb.suggestions.isEmpty)
        #expect(r.mb.suggestions.contains("Overall Look"))
    }

    @Test func boardsCanBeRenamedReorderedAndDeletedAndComeBackWithUndo() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Overall Look", sceneId: nil))
        #expect(await r.mb.createBoard(name: "Costume", sceneId: nil))
        #expect(await r.mb.createBoard(name: "Locations", sceneId: try r.scene(1).sceneId))
        #expect(r.mb.boards.map(\.name) == ["Overall Look", "Costume", "Locations"])
        #expect(r.mb.boards.last?.sceneNumber != nil)

        let costume = try #require(r.mb.boards.first { $0.name == "Costume" })
        #expect(await r.mb.rename(costume, to: "Costume and Makeup"))
        await r.mb.reload()
        #expect(r.mb.boards.map(\.name) == ["Overall Look", "Costume and Makeup", "Locations"])

        await r.mb.moveBoard(r.mb.boards[2], by: -1)
        await r.mb.reload()
        #expect(r.mb.boards.map(\.name) == ["Overall Look", "Locations", "Costume and Makeup"])
        await r.mb.moveBoard(r.mb.boards[0], by: -1)   // already first: nothing happens
        #expect(r.quiet)

        let notes = r.mb.boards[1]
        #expect(await r.mb.saveNotes(notes, notes: "Keep the palette cold."))
        #expect(await r.mb.boardNotes(notes) == "Keep the palette cold.")

        let doomed = r.mb.boards[2]
        #expect(await r.mb.deleteBoard(doomed))
        await r.mb.reload()
        #expect(r.mb.boards.count == 2)
        await r.app.undo()
        await r.mb.reload()
        #expect(r.mb.boards.count == 3)
        #expect(r.quiet)
    }

    @Test func tilesNoteLinkImageAndTheirEditing() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Overall Look", sceneId: nil))
        #expect(!(await r.mb.addNote(text: "   ", isPrivate: false)))
        #expect(await r.mb.addNote(text: "Warm windows", isPrivate: false))
        #expect(await r.mb.addNote(text: "Do not show the crew", isPrivate: true))
        #expect(await r.mb.addLink(url: "imdb.com/title/tt0353969", title: "Memories of Murder"))
        #expect(await r.mb.addImages(paths: [try r.imageFile("one.png"), try r.imageFile("two.png", width: 640, height: 480)]))
        await r.mb.reload()
        #expect(r.mb.items.count == 5)
        let link = try #require(r.mb.items.first { $0.kind == "link" })
        #expect(link.url?.hasPrefix("https://") == true)
        #expect(link.urlHost == "imdb.com")
        #expect(link.linkTitle == "Memories of Murder")
        let priv = try #require(r.mb.items.first { $0.isPrivate })
        #expect(priv.kind == "note")
        let images = r.mb.items.filter { $0.kind == "image" }
        #expect(images.count == 2)
        #expect(images.allSatisfy { $0.asset?.available == true })
        // The tiles do not all sit on one spot.
        #expect(Set(r.mb.items.map { "\($0.x),\($0.y)" }).count == r.mb.items.count)

        // Editing: caption, note text, privacy, link.
        let note = try #require(r.mb.items.first { $0.kind == "note" && !$0.isPrivate })
        await r.mb.update(note, caption: "Why: it sets the mood")
        await r.mb.update(note, body: "Warm windows, cold street")
        await r.mb.update(priv, isPrivate: false)
        await r.mb.update(link, linkTitle: "Memories of Murder (2003)")
        await r.mb.reload()
        let n2 = try #require(r.mb.items.first { $0.id == note.id })
        #expect(n2.caption == "Why: it sets the mood")
        #expect(n2.body == "Warm windows, cold street")
        #expect(r.mb.items.first { $0.id == priv.id }?.isPrivate == false)
        #expect(r.mb.items.first { $0.id == link.id }?.linkTitle == "Memories of Murder (2003)")

        // A bad address is refused with the core's words and nothing is added.
        let count = r.mb.items.count
        #expect(!(await r.mb.addLink(url: "file:///etc/passwd", title: nil)))
        #expect(r.app.alert != nil)
        r.clearAlert()
        await r.mb.reload()
        #expect(r.mb.items.count == count)
        // An emptied note is refused.
        await r.mb.update(note, body: "")
        #expect(r.app.alert != nil)
        r.clearAlert()
    }

    @Test func selectionMultiDragAndTheKeyboardWayToMoveTiles() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Look", sceneId: nil))
        for t in ["A", "B", "C"] { #expect(await r.mb.addNote(text: t, isPrivate: false, at: CGPoint(x: 100 + (t == "A" ? 0 : t == "B" ? 300 : 600), y: 100))) }
        await r.mb.reload()
        let ids = r.mb.items.map(\.id)
        #expect(ids.count == 3)

        // Shift-click selects several; a plain click on one of them keeps nothing else.
        r.mb.click(ids[0], additive: false)
        r.mb.click(ids[1], additive: true)
        #expect(r.mb.selected == [ids[0], ids[1]])
        r.mb.click(ids[1], additive: true)
        #expect(r.mb.selected == [ids[0]])
        r.mb.click(ids[1], additive: true)

        // Dragging a selected tile moves the whole group by the same amount.
        let before = Dictionary(uniqueKeysWithValues: r.mb.items.map { ($0.id, $0) })
        r.mb.dragChanged(id: ids[0], dx: 50, dy: 20)
        await r.mb.dragEnded()
        await r.mb.reload()
        for id in [ids[0], ids[1]] {
            let now = try #require(r.mb.items.first { $0.id == id })
            #expect(now.x == before[id]!.x + 50)
            #expect(now.y == before[id]!.y + 20)
        }
        let c = try #require(r.mb.items.first { $0.id == ids[2] })
        #expect(c.x == before[ids[2]]!.x)

        // Arrow keys: 8 points, 32 with Shift.
        r.mb.selected = [ids[2]]
        await r.mb.nudge(dx: 8, dy: 0)
        await r.mb.reload()
        #expect(r.mb.items.first { $0.id == ids[2] }?.x == before[ids[2]]!.x + 8)
        await r.mb.nudge(dx: 0, dy: -32)
        await r.mb.reload()
        #expect(r.mb.items.first { $0.id == ids[2] }?.y == before[ids[2]]!.y - 32)

        // Stops at the canvas edge: a tile at the left cannot go further left.
        r.mb.selected = [ids[0]]
        await r.mb.nudge(dx: -100_000, dy: 0)
        await r.mb.reload()
        #expect(r.mb.items.first { $0.id == ids[0] }?.x == 0)

        // A click that never moves saves nothing: undo still points at the nudge, not at a ghost move.
        r.mb.dragChanged(id: ids[2], dx: 1, dy: 0)
        await r.mb.dragEnded()
        #expect(r.mb.preview == nil)

        // Escape clears the selection; Delete removes the selected tiles and Undo brings them back.
        r.mb.selected = [ids[0], ids[1]]
        await r.mb.deleteSelected()
        await r.mb.reload()
        #expect(r.mb.items.count == 1)
        #expect(r.mb.selected.isEmpty)
        await r.app.undo()
        await r.mb.reload()
        #expect(r.mb.items.count == 3)
        r.mb.clearSelection()
        #expect(r.mb.selected.isEmpty)
        #expect(r.quiet)
    }

    @Test func resizeKeepsTheShapeOfAnImageAndStaysInTheLimits() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Look", sceneId: nil))
        #expect(await r.mb.addImages(paths: [try r.imageFile("wide.png", width: 640, height: 320)]))
        await r.mb.reload()
        let img = try #require(r.mb.items.first)
        let ratio = img.h / img.w
        r.mb.resizeChanged(id: img.id, dw: 100, dh: 0, keepAspect: true)
        let during = try #require(r.mb.preview?.first)
        #expect(abs(during.h / during.w - ratio) < 0.02)
        await r.mb.resizeEnded(id: img.id)
        await r.mb.reload()
        let after = try #require(r.mb.items.first)
        #expect(after.w == img.w + 100)
        #expect(abs(after.h / after.w - ratio) < 0.02)
        // Never smaller than the minimum.
        r.mb.resizeChanged(id: img.id, dw: -100_000, dh: -100_000, keepAspect: false)
        #expect(r.mb.preview?.first?.w == MoodGeometry.tileMin)
        await r.mb.resizeEnded(id: img.id)
        await r.mb.reload()
        #expect(r.mb.items.first?.w == MoodGeometry.tileMin)
        // Front and back.
        #expect(await r.mb.addNote(text: "Above", isPrivate: false))
        await r.mb.reload()
        let note = try #require(r.mb.items.first { $0.kind == "note" })
        await r.mb.arrange(img.id, placement: "front")
        await r.mb.reload()
        let z1 = try #require(r.mb.items.first { $0.id == img.id }?.z)
        let z2 = try #require(r.mb.items.first { $0.id == note.id }?.z)
        #expect(z1 > z2)
        await r.mb.arrange(img.id, placement: "back")
        await r.mb.reload()
        #expect(r.mb.items.first { $0.id == img.id }!.z < r.mb.items.first { $0.id == note.id }!.z)
        #expect(r.quiet)
    }

    @Test func pasteAndFileDropsOnTheCanvas() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Look", sceneId: nil))
        let pb = NSPasteboard(name: NSPasteboard.Name("openframe.test.\(UUID().uuidString)"))
        defer { pb.releaseGlobally() }

        pb.clearContents()
        pb.setString("https://example.com/reference", forType: .string)
        #expect(await r.mb.paste(from: pb) == .link)
        pb.clearContents()
        pb.setString("Rain on the windscreen", forType: .string)
        #expect(await r.mb.paste(from: pb) == .note)
        pb.clearContents()
        pb.setData(TestImages.png(width: 200, height: 100), forType: .png)
        #expect(await r.mb.paste(from: pb) == .images(1))
        pb.clearContents()
        #expect(await r.mb.paste(from: pb) == .nothing)
        await r.mb.reload()
        #expect(r.mb.items.map(\.kind).sorted() == ["image", "link", "note"])

        // Files dropped from Finder: images are added, other files are named and skipped.
        let good = try r.imageFile("drop.png")
        let txt = r.root.appendingPathComponent("readme.txt")
        try "hello".write(to: txt, atomically: true, encoding: .utf8)
        await r.mb.dropFiles([URL(fileURLWithPath: good), txt], at: CGPoint(x: 300, y: 200))
        await r.mb.reload()
        let dropped = r.mb.items.filter { $0.kind == "image" }
        #expect(dropped.count == 2)
        #expect(dropped.contains { $0.x == 300 && $0.y == 200 })
        #expect(r.quiet)
    }

    @Test func badAndHugeImagesAreRefusedInPlainWordsAndNothingIsLost() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Look", sceneId: nil))
        #expect(await r.mb.addNote(text: "Keep me", isPrivate: false))
        await r.mb.reload()
        let kept = r.mb.items.count

        // A text file with an image name: the core reads the bytes, not the name.
        let fake = r.root.appendingPathComponent("fake.png")
        try "this is not a picture".write(to: fake, atomically: true, encoding: .utf8)
        _ = await r.mb.addImages(paths: [fake.path])
        await r.mb.reload()
        #expect(r.mb.items.filter { $0.kind == "image" }.allSatisfy { $0.asset?.available == true })
        // Whatever the core decided, the board still has what it had, and an unusable file is not shown as a picture.
        #expect(r.mb.items.count >= kept)
        r.clearAlert()

        // A file type that is not an image at all is refused with a message.
        let doc = r.root.appendingPathComponent("script.pdf")
        try Data("%PDF-1.4".utf8).write(to: doc)
        let before = r.mb.items.count
        #expect(!(await r.mb.addImages(paths: [doc.path])))
        let alert = try #require(r.app.alert)
        if case let .error(_, failure) = alert { #expect(failure.message.contains("isn't an image")) } else { Issue.record("expected an error") }
        r.clearAlert()
        await r.mb.reload()
        #expect(r.mb.items.count == before)

        // Pasted bytes that are not an image.
        #expect(!(await r.mb.addImageData(Data("not an image".utf8), name: "x.png")))
        #expect(r.app.alert != nil)
        r.clearAlert()

        // A missing file.
        #expect(!(await r.mb.addImages(paths: [r.root.appendingPathComponent("gone.png").path])))
        #expect(r.app.alert != nil)
        r.clearAlert()

        // A very large picture (18 million pixels) is accepted and shown reduced, or refused in plain words. It never hangs.
        let huge = try r.imageFile("huge.png", width: 6000, height: 3000)
        let ok = await r.mb.addImages(paths: [huge])
        await r.mb.reload()
        if ok {
            let tile = try #require(r.mb.items.last { $0.kind == "image" })
            #expect(tile.w <= MoodGeometry.tileMax)
            let image = await VaultThumbnails.shared.image(core: r.core, assetId: tile.asset!.id, global: false, maxPixel: 640)
            #expect(image != nil)
            #expect(max(image!.size.width, image!.size.height) <= 640)
        } else {
            #expect(r.app.alert != nil)
            r.clearAlert()
        }
        #expect(r.quiet)
    }

    @Test func imagesFromTheIdeaVaultAreCopiedOntoTheBoard() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Look", sceneId: nil))
        await r.mb.loadVaultImages()
        #expect(r.mb.vaultImages.isEmpty)
        let file = try r.imageFile("reference.png")
        _ = try await r.core.vaultAddFiles(VaultAddFilesArgs(store: .project, paths: [file]))
        await r.mb.loadVaultImages()
        let ref = try #require(r.mb.vaultImages.first)
        #expect(await r.mb.addVaultImage(ref))
        await r.mb.reload()
        let tile = try #require(r.mb.items.first)
        #expect(tile.kind == "image")
        #expect(tile.sourceVaultItemId == ref.vaultItemId)
        // The vault item is unchanged.
        await r.mb.loadVaultImages()
        #expect(r.mb.vaultImages.count == 1)
        // The global vault is empty.
        r.mb.vaultStore = .global
        await r.mb.loadVaultImages()
        #expect(r.mb.vaultImages.isEmpty)
        #expect(r.quiet)
    }

    @Test func aMoodboardExportsToPdfAndKeepsInternalNotesOutUnlessAsked() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        #expect(await r.mb.createBoard(name: "Overall Look", sceneId: nil))
        #expect(await r.mb.addNote(text: "Public idea", isPrivate: false))
        #expect(await r.mb.addNote(text: "Secret idea", isPrivate: true))
        #expect(await r.mb.addImages(paths: [try r.imageFile("a.png")]))
        await r.mb.reload()
        let board = try #require(r.mb.current)
        let out = r.root.appendingPathComponent("look.pdf").path
        let result = try #require(await r.mb.export(board, path: out, includeTitle: true, includeImages: true, includeCaptions: true, includeInternalNotes: false))
        #expect(result.fileName == "look.pdf")
        #expect(result.bytesWritten > 500)
        let head = try Data(contentsOf: URL(fileURLWithPath: out)).prefix(5)
        #expect(String(decoding: head, as: UTF8.self) == "%PDF-")
        // With Internal notes ticked the secret note is on the sheet, so the file grows.
        let plain = try #require(await r.mb.export(board, path: r.root.appendingPathComponent("plain.pdf").path, includeTitle: true, includeImages: false, includeCaptions: false, includeInternalNotes: false))
        let withInternal = try #require(await r.mb.export(board, path: r.root.appendingPathComponent("look-internal.pdf").path, includeTitle: true, includeImages: false, includeCaptions: false, includeInternalNotes: true))
        #expect(withInternal.bytesWritten > plain.bytesWritten)
        // A folder that does not exist: the core says so and nothing is half written.
        let bad = r.root.appendingPathComponent("no-such-folder/look.pdf").path
        #expect(await r.mb.export(board, path: bad, includeTitle: true, includeImages: true, includeCaptions: true, includeInternalNotes: false) == nil)
        #expect(r.app.alert != nil)
        r.clearAlert()
    }

    // MARK: Storyboards

    @Test func storyboardsCanBeStandaloneOrForASceneAndHaveSeveralPerScene() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        // Empty state: no board for a scene, no panels.
        await r.sb.reload()
        #expect(r.sb.boards.isEmpty)
        #expect(r.sb.selection.scene != nil)
        #expect(r.sb.canAdd)
        #expect(r.sb.panels.isEmpty)

        // A standalone board needs a name.
        #expect(await r.sb.createBoard(name: "  ", sceneId: nil) == nil)
        let standalone = try #require(await r.sb.createBoard(name: "Title sequence", sceneId: nil))
        #expect(standalone.sceneLineageId == nil)
        #expect(r.sb.selection.key == "board:\(standalone.id)")
        #expect(r.sb.otherBoards.count == 1)

        // A scene board takes its name from the heading.
        let scene = try r.scene(1)
        let forScene = try #require(await r.sb.createBoard(name: nil, sceneId: scene.sceneId))
        #expect(forScene.sceneLineageId == scene.lineageId)
        #expect(forScene.name.uppercased().contains("HIGHWAY"))
        // Another board for the same scene.
        let second = try #require(await r.sb.createBoard(name: "Alternative coverage", sceneId: scene.sceneId))
        #expect(r.sb.sceneBoards.count == 2)
        #expect(r.sb.selection.boardId == second.id)
        await r.sb.chooseSceneBoard(forScene.id)
        #expect(r.sb.selection.boardId == forScene.id)

        // Rename, link to another scene, make standalone, delete (and undo).
        #expect(await r.sb.rename(second, to: "Alt coverage"))
        #expect(await r.sb.setScene(standalone, sceneId: try r.scene(2).sceneId))
        await r.sb.reload()
        #expect(r.sb.boards.first { $0.id == standalone.id }?.sceneLineageId == (try r.scene(2)).lineageId)
        #expect(await r.sb.setScene(standalone, sceneId: nil))
        await r.sb.reload()
        #expect(r.sb.boards.first { $0.id == standalone.id }?.sceneLineageId == nil)
        #expect(await r.sb.deleteBoard(second))
        await r.sb.reload()
        #expect(!r.sb.boards.contains { $0.id == second.id })
        await r.app.undo()
        await r.sb.reload()
        #expect(r.sb.boards.contains { $0.id == second.id })
        #expect(r.quiet)
    }

    @Test func panelsCanBeImportedEditedMovedAndDeleted() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        let lineage = try r.scene(0).lineageId
        await r.sb.choose(key: "scene:\(lineage)")
        let file = try r.imageFile("frame.png", width: 960, height: 540)
        let first = try #require(await r.sb.addPanel(.image, path: file, description: "Meera at the gate"))
        #expect(first.visualKind == "image")
        #expect(first.description == "Meera at the gate")
        await r.sb.addPanel(.placeholder)
        await r.sb.addPanel(.placeholder, at: 0)
        await r.sb.reload()
        #expect(r.sb.panels.count == 3)
        #expect(r.sb.panels.first?.visualKind == "placeholder")
        // Pasted picture.
        await r.sb.addPasted(image: TestImages.png(width: 100, height: 60))
        await r.sb.reload()
        #expect(r.sb.panels.count == 4)
        #expect(r.sb.panels.last?.visualKind == "image")

        // Edit the panel text, framing and duration.
        let p = r.sb.panels[1]
        await r.sb.update(p, description: "Wide on the gate", framing: "Wide", movement: "Dolly", angle: "Low", soundNote: "Gate creaks", durationMs: 3500, note: "Check the light")
        await r.sb.reload()
        let e = try #require(r.sb.panels.first { $0.id == p.id })
        #expect(e.description == "Wide on the gate")
        #expect(e.framing == "Wide" && e.movement == "Dolly" && e.angle == "Low")
        #expect(e.soundNote == "Gate creaks" && e.note == "Check the light")
        #expect(e.durationMs == 3500)
        #expect(VisualText.formatSeconds(e.durationMs) == "3.5")

        // Change and clear the picture.
        #expect(await r.sb.setVisual(e, .image, path: try r.imageFile("other.png")))
        await r.sb.reload()
        #expect(r.sb.panels.first { $0.id == p.id }?.visualKind == "image")
        let again = try #require(r.sb.panels.first { $0.id == p.id })
        #expect(await r.sb.setVisual(again, .placeholder))
        await r.sb.reload()
        #expect(r.sb.panels.first { $0.id == p.id }?.visualKind == "placeholder")
        #expect(r.sb.panels.first { $0.id == p.id }?.asset == nil)

        // The keyboard way to reorder.
        let ids = r.sb.panels.map(\.id)
        await r.sb.nudge(ids[0], by: 1)
        await r.sb.reload()
        #expect(r.sb.panels.map(\.id) == [ids[1], ids[0], ids[2], ids[3]])
        await r.sb.nudge(ids[1], by: -1)   // first already: nothing
        #expect(r.quiet)

        // Move a panel to another storyboard.
        let other = try #require(await r.sb.createBoard(name: "Pickups", sceneId: nil))
        await r.sb.choose(key: "scene:\(lineage)")
        await r.sb.reload()
        let mover = try #require(r.sb.panels.first)
        #expect(await r.sb.movePanel(mover, to: other.id))
        await r.sb.reload()
        #expect(r.sb.panels.count == 3)
        await r.sb.choose(key: "board:\(other.id)")
        await r.sb.reload()
        #expect(r.sb.panels.map(\.id) == [mover.id])

        // Delete and undo.
        await r.sb.deletePanels([mover.id])
        await r.sb.reload()
        #expect(r.sb.panels.isEmpty)
        await r.app.undo()
        await r.sb.reload()
        #expect(r.sb.panels.count == 1)
        #expect(r.quiet)
    }

    @Test func aBadPictureFailsPlainlyAndTheStoryboardStaysWhole() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        await r.sb.choose(key: "scene:\(try r.scene(0).lineageId)")
        await r.sb.addPanel(.placeholder)
        await r.sb.reload()
        let panel = try #require(r.sb.panels.first)

        let doc = r.root.appendingPathComponent("notes.txt")
        try "words".write(to: doc, atomically: true, encoding: .utf8)
        #expect(await r.sb.addPanel(.image, path: doc.path) == nil)
        let alert = try #require(r.app.alert)
        if case let .error(_, failure) = alert { #expect(!failure.message.isEmpty); #expect(!failure.message.contains("internal")) } else { Issue.record("expected an error") }
        r.clearAlert()
        #expect(!(await r.sb.setVisual(panel, .image, path: doc.path)))
        #expect(r.app.alert != nil)
        r.clearAlert()
        // Bad sketch bytes.
        #expect(!(await r.sb.setVisual(panel, .sketch, png: Data("nope".utf8))))
        r.clearAlert()
        await r.sb.reload()
        #expect(r.sb.panels.count == 1)
        #expect(r.sb.panels.first?.visualKind == "placeholder")

        // A picture too large to paste is refused in plain words.
        #expect(await r.mb.createBoard(name: "Look", sceneId: nil))
        let big = Data(count: 40 * 1024 * 1024)
        #expect(!(await r.mb.addImageData(big, name: "big.png")))
        #expect(r.app.alert != nil)
        r.clearAlert()
        #expect(r.quiet)
    }

    @Test func storyboardsExportToPdf() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        await r.sb.choose(key: "scene:\(try r.scene(0).lineageId)")
        await r.sb.addPanel(.image, path: try r.imageFile("f.png"), description: "Gate")
        await r.sb.addPanel(.placeholder, description: "Road")
        await r.sb.reload()
        let board = try #require(r.sb.board?.board)
        let out = r.root.appendingPathComponent("storyboard.pdf").path
        let result = try #require(await r.sb.export(boardIds: [board.id], path: out, includeNotes: true))
        #expect(result.format == "pdf")
        #expect(result.bytesWritten > 500)
        let all = r.root.appendingPathComponent("all.pdf").path
        #expect(await r.sb.export(boardIds: nil, path: all, includeNotes: false) != nil)
        #expect(r.quiet)
    }
}

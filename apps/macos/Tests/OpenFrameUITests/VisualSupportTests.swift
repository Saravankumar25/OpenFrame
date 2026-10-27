// Mac phase M6, Visual Planning: the plain maths and rules (canvas geometry, text helpers, scene
// selection, the sketch tool and the PNG it writes) and rendered snapshots of the screens.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Visual planning rules", .serialized)
@MainActor
struct VisualSupportTests {
    // MARK: Canvas geometry

    private let boxes = [
        MoodBox(id: "a", x: 100, y: 100, w: 200, h: 100),
        MoodBox(id: "b", x: 400, y: 50, w: 100, h: 100),
        MoodBox(id: "c", x: 0, y: 500, w: 100, h: 100),
    ]

    @Test func aGroupMovesTogetherAndStopsAtTheEdgeAsAUnit() {
        let moved = MoodGeometry.drag(boxes, selected: ["a", "b"], dx: 50.4, dy: -10)
        #expect(moved[0].x == 150 && moved[0].y == 90)
        #expect(moved[1].x == 450 && moved[1].y == 40)
        #expect(moved[2] == boxes[2])
        // Dragging far left stops when the left-most selected tile reaches 0; the gap between them stays.
        let left = MoodGeometry.drag(boxes, selected: ["a", "b"], dx: -5000, dy: 0)
        #expect(left[0].x == 0 && left[1].x == 300)
        // Far up: b is the top-most (y 50).
        let up = MoodGeometry.drag(boxes, selected: ["a", "b"], dx: 0, dy: -5000)
        #expect(up[1].y == 0 && up[0].y == 50)
        // Far right: bounded by the canvas size.
        let right = MoodGeometry.drag(boxes, selected: ["a", "b"], dx: 1_000_000, dy: 0)
        #expect(right[1].x == MoodGeometry.canvasMax)
        // Nothing selected: nothing moves.
        #expect(MoodGeometry.drag(boxes, selected: [], dx: 10, dy: 10) == boxes)
        // JavaScript rounding: half goes up.
        #expect(MoodGeometry.round(2.5) == 3 && MoodGeometry.round(-2.5) == -2 && MoodGeometry.round(-2.6) == -3)
    }

    @Test func resizeKeepsTheRatioClampsAndWorksOnFlatBoxes() {
        let wide = MoodBox(id: "i", x: 0, y: 0, w: 400, h: 200)
        let r = MoodGeometry.resize(wide, dw: 100, dh: 0, keepAspect: true)
        #expect(r.w == 500 && r.h == 250)
        let free = MoodGeometry.resize(wide, dw: 100, dh: -50, keepAspect: false)
        #expect(free.w == 500 && free.h == 150)
        #expect(MoodGeometry.resize(wide, dw: -9999, dh: -9999, keepAspect: false).w == MoodGeometry.tileMin)
        #expect(MoodGeometry.resize(wide, dw: 99999, dh: 99999, keepAspect: false).h == MoodGeometry.tileMax)
        let zero = MoodBox(id: "z", x: 0, y: 0, w: 0, h: 0)
        #expect(MoodGeometry.resize(zero, dw: 10, dh: 10, keepAspect: true).w == MoodGeometry.tileMin)
    }

    @Test func theCanvasGrowsWithItsContentAndOnlyChangedTilesAreSaved() {
        #expect(MoodGeometry.extent([]).width == 240)
        let e = MoodGeometry.extent(boxes)
        #expect(e.width == 500 + 240 && e.height == 600 + 240)
        let after = MoodGeometry.drag(boxes, selected: ["a"], dx: 10, dy: 0)
        let moves = MoodGeometry.changedMoves(before: boxes, after: after)
        #expect(moves.map(\.id) == ["a"])
        #expect(moves.first?.x == 110)
        #expect(MoodGeometry.changedMoves(before: boxes, after: boxes).isEmpty)
    }

    // MARK: Text

    @Test func textHelpersMatchWindows() {
        #expect(VisualText.move(["a", "b", "c"], from: 2, to: 0) == ["c", "a", "b"])
        #expect(VisualText.move(["a", "b", "c"], from: 0, to: 9) == ["b", "c", "a"])
        #expect(VisualText.move(["a"], from: 4, to: 0) == ["a"])
        #expect(VisualText.parseSeconds("3.5") == 3500)
        #expect(VisualText.parseSeconds("3,5 s") == 3500)
        #expect(VisualText.parseSeconds("") == 0)
        #expect(VisualText.parseSeconds("2 seconds") == 2000)
        #expect(VisualText.parseSeconds("-1") == nil)
        #expect(VisualText.parseSeconds("abc") == nil)
        #expect(VisualText.parseSeconds("3601") == nil)
        #expect(VisualText.formatSeconds(3500) == "3.5" && VisualText.formatSeconds(2000) == "2" && VisualText.formatSeconds(nil) == "" && VisualText.formatSeconds(0) == "")
        #expect(VisualText.looksLikeURL("https://example.com/a?b=1"))
        #expect(VisualText.looksLikeURL("imdb.com/title/tt1"))
        #expect(!VisualText.looksLikeURL("Cold blue light"))
        #expect(!VisualText.looksLikeURL("two words.com now"))
        #expect(!VisualText.looksLikeURL(""))
        #expect(VisualText.isImage("/x/a.PNG") && VisualText.isImage("b.jpeg") && !VisualText.isImage("c.pdf") && !VisualText.isImage("noext"))
        #expect(VisualText.plural(1, "panel") == "1 panel" && VisualText.plural(2, "panel") == "2 panels" && VisualText.plural(0, "storyboard") == "0 storyboards")
        #expect(VisualText.formatWhen(nil) == "" && VisualText.formatWhen(0) == "")
        #expect(!VisualText.formatWhen(1_760_000_000_000).isEmpty)
    }

    @Test func scenesAndStoryboardsResolveLikeWindows() {
        func scene(_ n: String, _ lineage: String) -> VisualScene {
            VisualScene(sceneId: "row-\(lineage)", lineageId: lineage, draftId: "d", number: n, heading: "INT. ROOM \(n)", omitted: false,
                        groupLabel: nil, shotCount: 0, storyboardCount: 0, moodboardCount: 0, needsReview: false, changedAt: nil)
        }
        func board(_ id: String, lineage: String?, removed: Bool = false) -> StoryboardSummary {
            StoryboardSummary(id: id, name: id, sceneId: lineage.map { "row-\($0)" }, sceneLineageId: lineage, sceneNumber: nil, sceneHeading: nil,
                              sceneRemoved: removed, needsReview: false, panelCount: 0, cover: nil, rev: 1)
        }
        let scenes = VisualScenes(sourceKind: "currentDraft", sourceLabel: "Draft 1", scenes: [scene("1", "L1"), scene("2", "L2")], removed: [])
        let boards = [board("b2", lineage: "L2"), board("b2b", lineage: "L2"), board("free", lineage: nil), board("gone", lineage: "L9", removed: true)]
        // Default: the first scene that has a storyboard.
        let d = StoryboardSelection.resolve(wanted: nil, sceneBoard: [:], scenes: scenes, boards: boards)
        #expect(d.key == "scene:L2" && d.boardId == "b2" && d.scene?.lineageId == "L2")
        // A remembered board of the scene wins.
        #expect(StoryboardSelection.resolve(wanted: "scene:L2", sceneBoard: ["L2": "b2b"], scenes: scenes, boards: boards).boardId == "b2b")
        // A scene with no board: no board id, the scene is still chosen (its first panel creates the board).
        let s1 = StoryboardSelection.resolve(wanted: "scene:L1", sceneBoard: [:], scenes: scenes, boards: boards)
        #expect(s1.boardId == nil && s1.scene?.lineageId == "L1")
        // A standalone board and a board of a removed scene have no scene.
        #expect(StoryboardSelection.resolve(wanted: "board:free", sceneBoard: [:], scenes: scenes, boards: boards) == StoryboardSelection(key: "board:free", boardId: "free", scene: nil))
        #expect(StoryboardSelection.resolve(wanted: "board:gone", sceneBoard: [:], scenes: scenes, boards: boards).scene == nil)
        // A board of a live scene resolves to the scene.
        #expect(StoryboardSelection.resolve(wanted: "board:b2b", sceneBoard: [:], scenes: scenes, boards: boards).key == "scene:L2")
        // Nothing at all.
        #expect(StoryboardSelection.resolve(wanted: nil, sceneBoard: [:], scenes: nil, boards: []) == StoryboardSelection(key: nil, boardId: nil, scene: nil))
        // Only boards: the first.
        #expect(StoryboardSelection.resolve(wanted: "scene:zzz", sceneBoard: [:], scenes: nil, boards: boards).boardId == "b2")

        #expect(resolveShotSceneKey(wanted: nil, scenes: scenes, showRemoved: false) == "L1")
        #expect(resolveShotSceneKey(wanted: "all", scenes: scenes, showRemoved: false) == "all")
        #expect(resolveShotSceneKey(wanted: "L2", scenes: scenes, showRemoved: false) == "L2")
        #expect(resolveShotSceneKey(wanted: "nope", scenes: scenes, showRemoved: false) == "L1")
        #expect(resolveShotSceneKey(wanted: nil, scenes: nil, showRemoved: false) == nil)
        let onlyRemoved = VisualScenes(sourceKind: "none", sourceLabel: "", scenes: [], removed: [RemovedVisualScene(lineageId: "R", heading: "INT. OLD", shotCount: 2, storyboardCount: 0)])
        #expect(resolveShotSceneKey(wanted: nil, scenes: onlyRemoved, showRemoved: false) == "all")
        #expect(resolveShotSceneKey(wanted: "R", scenes: onlyRemoved, showRemoved: true) == "R")
        #expect(VisualText.sceneLabel(scene("4", "L4")) == "Scene 4: INT. ROOM 4")
    }

    // MARK: Sketch

    @Test func theSketchToolKeepsStrokesUndoesAndClears() {
        let m = SketchModel()
        #expect(!m.canSave && !m.canUndo)
        m.tool = .eraser
        #expect(m.tool.width == 28)
        m.chooseInk(.red)                       // choosing an ink goes back to the pen
        #expect(m.tool == .pen && m.tool.width == 4 && m.ink == .red)
        m.begin(at: CGPoint(x: 5, y: 5))
        m.extend(to: CGPoint(x: -50, y: 9999))  // off the sheet: kept on the sheet
        #expect(m.live?.points.last == CGPoint(x: 0, y: 540))
        #expect(m.allStrokes.count == 1 && m.strokes.isEmpty)
        m.end()
        #expect(m.strokes.count == 1 && m.live == nil)
        m.extend(to: CGPoint(x: 1, y: 1))        // no stroke in progress: ignored
        m.end()
        #expect(m.strokes.count == 1)
        m.tool = .eraser
        m.begin(at: CGPoint(x: 10, y: 10)); m.end()
        #expect(m.strokes.last?.erase == true && m.strokes.last?.width == 28)
        m.undo()
        #expect(m.strokes.count == 1)
        m.clear()
        #expect(m.strokes.isEmpty && !m.canSave)
        m.undo()                                 // nothing to take back
        #expect(m.strokes.isEmpty)
        #expect(SketchInk.black.hex == "#111111" && SketchInk.red.hex == "#c8443a" && SketchInk.blue.hex == "#3b6fd4")
    }

    @Test func theSketchPngIsTheSameFormatWindowsSaves() throws {
        let m = SketchModel()
        m.chooseInk(.blue)
        m.begin(at: CGPoint(x: 100, y: 270))
        for x in stride(from: 110.0, through: 860.0, by: 10) { m.extend(to: CGPoint(x: x, y: 270)) }
        m.end()
        let data = try #require(m.png())
        #expect(Array(data.prefix(8)) == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        let rep = try #require(NSBitmapImageRep(data: data))
        #expect(rep.pixelsWide == 960 && rep.pixelsHigh == 540)
        func rgb(_ x: Int, _ y: Int) -> (Int, Int, Int) {
            let c = rep.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
            return (Int((c.redComponent * 255).rounded()), Int((c.greenComponent * 255).rounded()), Int((c.blueComponent * 255).rounded()))
        }
        // White paper everywhere else, the blue ink on the line (row 270 from the top).
        #expect(rgb(10, 10) == (255, 255, 255))
        #expect(rgb(480, 400) == (255, 255, 255))
        let ink = rgb(480, 270)
        #expect(ink.2 > ink.0 + 60)               // blue, not white or black
        #expect(rgb(480, 270 + 20) == (255, 255, 255))
        // The eraser paints paper over the ink.
        m.tool = .eraser
        m.begin(at: CGPoint(x: 400, y: 270))
        m.extend(to: CGPoint(x: 560, y: 270))
        m.end()
        let png2 = try #require(m.png())
        let rep2 = try #require(NSBitmapImageRep(data: png2))
        let c = rep2.colorAt(x: 480, y: 270)!.usingColorSpace(.deviceRGB)!
        #expect(c.redComponent > 0.99 && c.greenComponent > 0.99 && c.blueComponent > 0.99)
        // A single tap leaves a dot.
        let dot = SketchModel()
        dot.begin(at: CGPoint(x: 50, y: 50)); dot.end()
        let png3 = try #require(dot.png())
        let rep3 = try #require(NSBitmapImageRep(data: png3))
        let d = rep3.colorAt(x: 50, y: 50)!.usingColorSpace(.deviceRGB)!
        #expect(d.redComponent < 0.2)
    }

    @Test func theDrawingViewTurnsMouseEventsIntoStrokes() throws {
        _ = NSApplication.shared
        let model = SketchModel()
        let view = SketchDrawingNSView(frame: NSRect(x: 0, y: 0, width: 480, height: 270))
        view.model = model
        let window = NSWindow(contentRect: view.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = view
        func event(_ type: NSEvent.EventType, _ p: CGPoint) -> NSEvent {
            // The view is flipped, so a point at y from the top is window y = height - y.
            NSEvent.mouseEvent(with: type, location: CGPoint(x: p.x, y: 270 - p.y), modifierFlags: [], timestamp: 0,
                               windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!
        }
        view.mouseDown(with: event(.leftMouseDown, CGPoint(x: 240, y: 135)))
        view.mouseDragged(with: event(.leftMouseDragged, CGPoint(x: 480, y: 270)))
        view.mouseUp(with: event(.leftMouseUp, CGPoint(x: 480, y: 270)))
        let s = try #require(model.strokes.first)
        // The view is half the sheet's size, so points are doubled onto the 960 by 540 sheet.
        #expect(s.points.first == CGPoint(x: 480, y: 270))
        #expect(s.points.last == CGPoint(x: 960, y: 540))
        #expect(view.isAccessibilityElement())
        #expect(view.accessibilityLabel() == "Sketch area")
    }

    // MARK: Pictures and screens

    @Test func screensRenderInLightAndDark() async throws {
        let r = try await VisualRig.make(title: "Night Bus")
        defer { r.close() }
        let lineage = try r.scene(0).lineageId
        #expect(await r.mb.createBoard(name: "Overall Look", sceneId: nil))
        await r.mb.addNote(text: "Cold blue light, warm windows", isPrivate: false)
        await r.mb.addNote(text: "Internal only", isPrivate: true)
        await r.mb.addLink(url: "https://example.com/ref", title: "Reference")
        await r.mb.addImages(paths: [try r.imageFile("look.png", width: 640, height: 360)])
        await r.sb.choose(key: "scene:\(lineage)")
        await r.sb.addPanel(.image, path: try r.imageFile("frame.png", width: 960, height: 540), description: "Wide on the depot")
        await r.sb.addPanel(.placeholder, description: "Meera and the keys")
        await r.shots.chooseScene(lineage)
        await r.shots.addShot(description: "Wide: the depot at night")
        await r.shots.addShot(description: "Insert: the ring of keys")
        await r.reloadAll()
        await r.shots.update(r.shots.shots[0], size: "Wide", movement: "Dolly", angle: "Low")
        await r.shots.reload()
        for scheme in [ColorScheme.light, .dark] {
            let n = scheme == .dark ? "dark" : "light"
            for tab in VisualTab.allCases {
                r.hub.tab = tab
                let image = Fixture.render(host(VisualContent(hub: r.hub), app: r.app, scheme: scheme),
                                           name: "visual-\(tab.rawValue)-\(n)", size: CGSize(width: 1280, height: 760))
                #expect(image != nil)
            }
        }
        #expect(Fixture.render(host(SketchSheet(onSave: { _ in }, onCancel: {}), app: r.app, scheme: .light), name: "visual-sketch", size: CGSize(width: 760, height: 560)) != nil)
        r.mb.selected = [r.mb.items[0].id]
        r.mb.inspectId = r.mb.items[0].id
        r.hub.tab = .moodboards
        #expect(Fixture.render(host(VisualContent(hub: r.hub), app: r.app, scheme: .light), name: "visual-moodboards-selected", size: CGSize(width: 1280, height: 760)) != nil)
        #expect(r.quiet)
    }

    @Test func theEmptyScreensRender() async throws {
        let r = try await VisualRig.make(script: false)
        defer { r.close() }
        for tab in VisualTab.allCases {
            r.hub.tab = tab
            #expect(Fixture.render(host(VisualContent(hub: r.hub), app: r.app, scheme: .light), name: "visual-empty-\(tab.rawValue)", size: CGSize(width: 1100, height: 640)) != nil)
        }
        #expect(r.quiet)
    }

    private func host<V: View>(_ view: V, app: AppModel, scheme: ColorScheme) -> some View {
        let themes = ThemeStore()
        return view
            .environment(app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
            .environment(\.colorScheme, scheme)
    }
}

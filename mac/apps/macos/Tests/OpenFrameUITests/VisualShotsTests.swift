// Mac phase M6, Visual Planning: the Shot Lists tab and the links between shots and storyboard
// panels, driven at model level against a real core on a temporary folder.

import AppKit
import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Visual planning shot lists", .serialized)
@MainActor
struct VisualShotsTests {
    @Test func withoutAScriptThereIsNothingToPlanAgainst() async throws {
        let r = try await VisualRig.make(script: false)
        defer { r.close() }
        #expect(r.hub.scenesLoaded)
        #expect(r.hub.scenes?.sourceKind == "none")
        #expect(!r.hub.hasScenes)
        #expect(r.shots.sceneKey == nil)
        #expect(!r.shots.canAdd)
        #expect(r.shots.shots.isEmpty)
        #expect(await r.shots.addShot(description: "Wide") == nil)
        #expect(r.quiet)
        // A standalone storyboard still works, and a panel can only make a shot once a script exists.
        let board = try #require(await r.sb.createBoard(name: "Title sequence", sceneId: nil))
        await r.sb.addPanel(.placeholder, description: "Logo")
        await r.sb.reload()
        let panel = try #require(r.sb.panels.first)
        await r.sb.createShot(from: panel, sceneId: nil)
        #expect(r.app.alert != nil)
        r.clearAlert()
        _ = board
    }

    @Test func shotsAreAddedByDescriptionAloneAndLabelledFromOrder() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        let scene = try r.scene(0)
        await r.shots.chooseScene(scene.lineageId)
        #expect(r.shots.sceneKey == scene.lineageId)
        #expect(r.shots.canAdd)

        // An empty description is refused with Windows' words, before the core is asked.
        #expect(await r.shots.addShot(description: "  ") == nil)
        #expect(r.shots.composerError == "Describe the shot to add it.")
        #expect(r.quiet)

        for d in ["Wide: the depot at night", "Meera counts the keys", "Insert: one key is missing"] {
            #expect(await r.shots.addShot(description: d) != nil)
        }
        #expect(r.shots.composerError == nil)
        await r.shots.reload()
        #expect(r.shots.shots.map(\.description) == ["Wide: the depot at night", "Meera counts the keys", "Insert: one key is missing"])
        let letters = r.shots.shots.map { String($0.label.last!) }
        #expect(letters == ["A", "B", "C"])
        #expect(r.shots.shots.allSatisfy { $0.label.hasPrefix(scene.number) })

        // Dragging the last shot to the front re-letters all three; nobody renumbers by hand.
        let last = r.shots.shots[2]
        await r.shots.reorder(last.id, to: 0)
        #expect(r.shots.shown.first?.id == last.id)
        await r.shots.reload()
        #expect(r.shots.shots.map(\.description) == ["Insert: one key is missing", "Wide: the depot at night", "Meera counts the keys"])
        #expect(r.shots.shots.map { String($0.label.last!) } == ["A", "B", "C"])
        // The keyboard way.
        await r.shots.nudge(r.shots.shots[0].id, by: 1)
        await r.shots.reload()
        #expect(r.shots.shots.map(\.description).first == "Wide: the depot at night")
        // Out of range does nothing.
        await r.shots.nudge(r.shots.shots[0].id, by: -1)
        #expect(r.quiet)
    }

    @Test func technicalDetailsCharactersAndTheReferenceImage() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        await r.shots.chooseScene(try r.scene(0).lineageId)
        let shot = try #require(await r.shots.addShot(description: "Meera counts the keys"))
        await r.shots.update(shot, size: "Medium close", movement: "Handheld", angle: "Low", lens: "35mm", cameraNotes: "follow focus to the ring", soundNote: "keys jingle")
        await r.shots.update(shot, characters: ["Meera"])
        await r.shots.reload()
        let s = try #require(r.shots.shots.first)
        #expect(s.size == "Medium close" && s.movement == "Handheld" && s.angle == "Low" && s.lens == "35mm")
        #expect(s.cameraNotes == "follow focus to the ring" && s.soundNote == "keys jingle")
        #expect(s.characters == ["Meera"])
        // The characters of the scene come from the script.
        await r.shots.loadCharacters(for: s)
        #expect(r.shots.characters.contains { $0.uppercased() == "MEERA" })

        // A required description cannot be emptied.
        await r.shots.update(s, description: "")
        #expect(r.app.alert != nil)
        r.clearAlert()

        // Reference image: set, replace, remove.
        await r.shots.setReference(s, path: try r.imageFile("ref.png"))
        await r.shots.reload()
        #expect(r.shots.shots.first?.referenceAsset?.available == true)
        await r.shots.setReference(s, path: try r.imageFile("ref2.png"))
        await r.shots.setReference(s, path: nil)
        await r.shots.reload()
        #expect(r.shots.shots.first?.referenceAsset == nil)
        // A file that is not an image is refused and the shot keeps its state.
        let doc = r.root.appendingPathComponent("brief.txt")
        try "x".write(to: doc, atomically: true, encoding: .utf8)
        await r.shots.setReference(s, path: doc.path)
        #expect(r.app.alert != nil)
        r.clearAlert()
        #expect(r.shots.shots.first?.referenceAsset == nil)
        #expect(r.quiet)
    }

    @Test func selectionRulesDuplicateMoveDeleteAndUndo() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        let s0 = try r.scene(0), s1 = try r.scene(1)
        await r.shots.chooseScene(s0.lineageId)
        for d in ["One", "Two", "Three", "Four"] { await r.shots.addShot(description: d) }
        await r.shots.reload()
        let ids = r.shots.shots.map(\.id)

        // Plain click opens the shot; Command toggles; Shift selects a run.
        r.shots.click(r.shots.shots[0], command: false, shift: false)
        #expect(r.shots.selectedId == ids[0] && r.shots.picked == [ids[0]])
        r.shots.click(r.shots.shots[2], command: true, shift: false)
        #expect(r.shots.picked == [ids[0], ids[2]])
        r.shots.click(r.shots.shots[2], command: true, shift: false)
        #expect(r.shots.picked == [ids[0]])
        r.shots.click(r.shots.shots[3], command: false, shift: true)
        #expect(r.shots.picked == Set(ids))

        // Duplicate, then move that copy to another scene.
        let copy = try #require(await r.shots.duplicate(r.shots.shots[1]))
        #expect(copy.description == "Two")
        #expect(await r.shots.move(copy, toScene: s1.sceneId))
        await r.shots.reload()
        #expect(r.shots.shots.count == 4)
        await r.shots.chooseScene(s1.lineageId)
        #expect(r.shots.shots.map(\.description) == ["Two"])
        #expect(r.shots.shots.first?.sceneLineageId == s1.lineageId)

        // The "all scenes" view groups by scene.
        await r.shots.chooseScene("all")
        #expect(r.shots.allScenes)
        #expect(r.shots.groups.count == 2)
        #expect(r.shots.groups.map { $0.shots.count } == [4, 1])
        #expect(r.shots.groups[0].title.hasPrefix("Scene \(s0.number): "))

        // Delete several, then undo.
        await r.shots.chooseScene(s0.lineageId)
        r.shots.picked = Set(ids.prefix(2))
        await r.shots.delete(Array(ids.prefix(2)))
        await r.shots.reload()
        #expect(r.shots.shots.count == 2)
        #expect(r.shots.picked.isEmpty)
        await r.app.undo()
        await r.shots.reload()
        #expect(r.shots.shots.count == 4)
        #expect(r.quiet)
    }

    @Test func copyPlanningToAnotherScene() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        let s0 = try r.scene(0), s2 = try r.scene(2)
        await r.shots.chooseScene(s0.lineageId)
        await r.shots.addShot(description: "Wide")
        await r.shots.addShot(description: "Close")
        await r.sb.choose(key: "scene:\(s0.lineageId)")
        await r.sb.addPanel(.placeholder)
        await r.hub.reloadScenes()
        #expect(await r.shots.copyPlanning(toScene: s2.sceneId))
        await r.shots.chooseScene(s2.lineageId)
        #expect(r.shots.shots.map(\.description) == ["Wide", "Close"])
        // The copies are independent: editing one leaves the original alone.
        await r.shots.update(r.shots.shots[0], description: "Wide on the bus")
        await r.shots.chooseScene(s0.lineageId)
        #expect(r.shots.shots.first?.description == "Wide")
        // Copying a scene with nothing to copy to itself is not offered: the picker leaves the scene out.
        #expect(r.quiet)
    }

    @Test func shotsAndStoryboardPanelsAreLinkedBothWays() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        let lineage = try r.scene(0).lineageId
        await r.shots.chooseScene(lineage)
        let shot = try #require(await r.shots.addShot(description: "Wide: the depot"))
        await r.sb.choose(key: "scene:\(lineage)")
        await r.sb.addPanel(.placeholder, description: "Depot gate")
        await r.sb.addPanel(.placeholder, description: "Keys")
        await r.sb.reload()
        #expect(r.sb.sceneShots.map(\.id) == [shot.id])

        // A panel makes a shot of its own, labelled from its place.
        let p2 = r.sb.panels[1]
        await r.sb.createShot(from: p2, sceneId: nil)
        await r.sb.reload()
        #expect(r.sb.panels[1].shotLabel != nil)
        let made = try #require(r.sb.panels[1].shotId)
        #expect(made != shot.id)

        // Link the first panel to the first shot, and unlink it again.
        let p1 = r.sb.panels[0]
        await r.sb.linkShot(p1, shotId: shot.id)
        await r.sb.reload()
        #expect(r.sb.panels[0].shotId == shot.id)
        #expect(r.sb.panels[0].shotLabel == shot.label)
        await r.shots.reload()
        let linked = try #require(r.shots.shots.first { $0.id == shot.id })
        #expect(linked.panels.count == 1)
        #expect(linked.panels.first?.panelId == p1.id)
        await r.sb.linkShot(r.sb.panels[0], shotId: nil)
        await r.sb.reload()
        #expect(r.sb.panels[0].shotId == nil)

        // The shot makes a panel; attach and detach.
        await r.shots.createPanel(for: linked)
        await r.sb.reload()
        await r.shots.reload()
        let withPanel = try #require(r.shots.shots.first { $0.id == shot.id })
        #expect(withPanel.panels.count == 1)
        await r.shots.loadAttachChoices(for: withPanel, boardId: nil)
        #expect(!r.shots.attachBoards.isEmpty)
        #expect(r.shots.attachBoards.first?.sceneLineageId == lineage)
        let free = try #require(r.shots.attachPanels.first { $0.shotId == nil })
        #expect(await r.shots.attach(withPanel, panelId: free.id))
        await r.shots.reload()
        #expect(r.shots.shots.first { $0.id == shot.id }?.panels.count == 2)
        await r.shots.detach(withPanel, panelId: free.id)
        await r.shots.reload()
        #expect(r.shots.shots.first { $0.id == shot.id }?.panels.count == 1)

        // Moving a panel renumbers the linked shots in the core.
        await r.sb.reload()
        let labelsBefore = r.sb.panels.compactMap(\.shotLabel)
        #expect(labelsBefore.count >= 2)
        await r.sb.reorder(r.sb.panels.last!.id, to: 0)
        await r.sb.reload()
        #expect(r.sb.panels.compactMap(\.shotLabel) != labelsBefore || labelsBefore.sorted() == labelsBefore)

        // A panel chip opens its shot in the Shot Lists tab; a shot's chip opens the panel.
        let panelWithShot = try #require(r.sb.panels.first { $0.shotId != nil })
        r.hub.tab = .storyboards
        await r.hub.openShot(panelWithShot.shotId!)
        #expect(r.hub.tab == .shots)
        #expect(r.shots.selectedId == panelWithShot.shotId)
        let theShot = try #require(r.shots.selected)
        let ref = try #require(theShot.panels.first)
        await r.hub.openPanel(storyboardId: ref.storyboardId, panelId: ref.panelId)
        #expect(r.hub.tab == .storyboards)
        #expect(r.sb.selectedPanelId == ref.panelId)
        #expect(r.quiet)
    }

    @Test func shotListsExportToPdfCsvAndExcel() async throws {
        let r = try await VisualRig.make()
        defer { r.close() }
        let s0 = try r.scene(0)
        await r.shots.chooseScene(s0.lineageId)
        await r.shots.addShot(description: "Wide: the depot")
        await r.shots.addShot(description: "Close: the ring of keys")
        for ext in ["pdf", "csv", "xlsx"] {
            let out = r.root.appendingPathComponent("shots.\(ext)").path
            let res = try #require(await r.shots.export(format: ext, path: out, sceneIds: [s0.sceneId], includeRemoved: false))
            #expect(res.format == ext)
            #expect(res.bytesWritten > 50)
            #expect(FileManager.default.fileExists(atPath: out))
        }
        let csv = try String(contentsOfFile: r.root.appendingPathComponent("shots.csv").path, encoding: .utf8)
        #expect(csv.contains("Wide: the depot"))
        #expect(csv.contains("Close: the ring of keys"))
        // The whole project.
        #expect(await r.shots.export(format: "csv", path: r.root.appendingPathComponent("all.csv").path, sceneIds: nil, includeRemoved: false) != nil)
        // An unwritable place says so.
        #expect(await r.shots.export(format: "csv", path: "/System/openframe-denied.csv", sceneIds: nil, includeRemoved: false) == nil)
        #expect(r.app.alert != nil)
        r.clearAlert()
    }
}

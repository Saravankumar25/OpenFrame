// Keyboard Shortcuts: the model drives the real core exactly as the screen does (scenario 42,
// custom shortcut and remapped page break; restore all defaults).

import Foundation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

@Suite("Settings shortcuts", .serialized)
@MainActor
struct SettingsShortcutsTests {
    private func start() async throws -> (AppModel, ShortcutsSettingsModel, URL) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        ShortcutStore.shared.resetForTests()
        return (app, ShortcutsSettingsModel(app: app), root)
    }

    private func cmd(_ id: String) -> ShortcutCommand { ShortcutCatalogue.command(id)! }

    @Test func customShortcutIsStoredShownAndSurvivesAReload() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await model.add("F3", to: cmd("screenplay.find"))
        #expect(model.lastMessage == "Find in script: F3")
        #expect(model.current(cmd("screenplay.find")) == ["Mod+F", "F3"])
        #expect(model.isChanged(cmd("screenplay.find")))
        // A relaunch reads the same set from the core.
        ShortcutStore.shared.resetForTests()
        await ShortcutStore.shared.reload(core: app.core)
        #expect(ShortcutStore.shared.chords("screenplay.find") == ["Mod+F", "F3"])
    }

    @Test func remappedPageBreakReplacesTheDefault() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let pb = cmd("screenplay.page_break")
        await model.remove("Mod+Enter", from: pb)
        await model.add("Mod+Shift+B", to: pb)
        #expect(ShortcutStore.shared.chords("screenplay.page_break") == ["Mod+Shift+B"])
        #expect(ShortcutStore.shared.shortcut("screenplay.page_break") != nil)
        await model.restore(pb)
        #expect(ShortcutStore.shared.chords("screenplay.page_break") == ["Mod+Enter"])
        #expect(!model.isChanged(pb))
    }

    @Test func aKeyAnotherCommandUsesAsksBeforeMovingIt() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        // Mod+1 runs the Scene Heading element; give it to Action.
        await model.add("Mod+1", to: cmd("screenplay.element_action"))
        let pending = try #require(model.pending)
        #expect(pending.title.contains("is already used"))
        #expect(pending.body.contains("Scene Heading element"))
        #expect(ShortcutStore.shared.chords("screenplay.element_action") == ["Mod+2"])
        await model.confirmPending()
        #expect(ShortcutStore.shared.chords("screenplay.element_action").contains("Mod+1"))
        #expect(ShortcutStore.shared.chords("screenplay.element_scene_heading").isEmpty)
        #expect(model.conflictList.isEmpty)
    }

    @Test func restoringADefaultAnotherCommandNowHoldsAsksFirst() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await model.add("Mod+1", to: cmd("screenplay.element_action"))
        await model.confirmPending()
        await model.restore(cmd("screenplay.element_scene_heading"))
        let pending = try #require(model.pending)
        #expect(pending.title == "Restore the default?")
        await model.confirmPending()
        #expect(ShortcutStore.shared.chords("screenplay.element_scene_heading") == ["Mod+1"])
        #expect(!ShortcutStore.shared.chords("screenplay.element_action").contains("Mod+1"))
    }

    @Test func theCoreRefusesKeysThatWouldStopTyping() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let c = cmd("screenplay.next_scene")
        await model.add("Q", to: c)
        let e1 = try #require(model.error)
        #expect(e1.id == c.id)
        #expect(e1.text.contains("Add Command"))
        await model.add("Mod+C", to: c)
        #expect(model.error?.text.contains("copy") == true)
        await model.add("Mod+Alt+3", to: c)
        #expect(model.error?.text.contains("custom elements") == true)
        #expect(!model.isChanged(c))
        // Already a shortcut of this command.
        await model.add("Alt+ArrowDown", to: c)
        #expect(model.error?.text.contains("already a shortcut") == true)
    }

    @Test func restoreAllDefaultsClearsEverything() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await model.add("F3", to: cmd("screenplay.find"))
        await model.add("F4", to: cmd("screenplay.next_scene"))
        #expect(model.hasChanges)
        model.askingResetAll = true
        await model.resetAll()
        #expect(!model.hasChanges)
        #expect(model.lastMessage == "All shortcuts are back to their defaults")
        #expect(model.current(cmd("screenplay.find")) == ["Mod+F"])
    }

    @Test func atMostFourKeysPerCommand() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let c = cmd("screenplay.find")
        for key in ["F3", "F4", "F5", "F6", "F8"] { await model.add(key, to: c) }
        #expect(model.current(c).count <= ShortcutCatalogue.maxKeysPerCommand)
    }

    @Test func filterFindsByNameGroupOrKeys() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        model.filter = "page break"
        #expect(model.shown.map(\.id) == ["screenplay.page_break"])
        model.filter = "\u{2318}K"
        #expect(model.shown.contains { $0.id == "app.search" })
        model.filter = "zzzz"
        #expect(model.shown.isEmpty)
    }

    @Test func recordingStartsCancelsAndTakesAChord() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let c = cmd("screenplay.find")
        model.startRecording(c)
        #expect(model.recording == c.id)
        model.cancelRecording()
        #expect(model.recording == nil)
        model.startRecording(c)
        await model.record(.ignore, for: c)
        #expect(model.recording == c.id, "a lone modifier keeps listening")
        await model.record(.unsupported, for: c)
        #expect(model.error != nil)
        await model.record(.chord("F3"), for: c)
        #expect(model.recording == nil)
        #expect(model.current(c).contains("F3"))
    }

    @Test func conflictsInADamagedSetAreReported() {
        let overrides = ["screenplay.next_scene": ["Mod+F"]]
        let found = ShortcutCatalogue.conflicts(overrides)
        #expect(found.contains { $0.commandId == "screenplay.next_scene" && $0.otherId == "screenplay.find" })
        #expect(ShortcutCatalogue.conflicts([:]).isEmpty, "the defaults never clash")
    }

    @Test func chordsFromKeyPressesFollowTheGrammar() {
        func press(_ code: UInt16, _ ch: String?, cmd: Bool = false, opt: Bool = false, shift: Bool = false, ctl: Bool = false) -> ShortcutCatalogue.Press {
            ShortcutCatalogue.press(keyCode: code, characters: ch, command: cmd, option: opt, shift: shift, control: ctl)
        }
        #expect(press(45, "n", cmd: true, shift: true) == .chord("Mod+Shift+N"))
        #expect(press(99, nil) == .chord("F3"))
        #expect(press(125, nil, opt: true) == .chord("Alt+ArrowDown"))
        #expect(press(18, "1", cmd: true) == .chord("Mod+1"))
        #expect(press(55, nil, cmd: true) == .ignore, "Command on its own")
        #expect(press(0, "a", ctl: true) == .unsupported)
        // A German layout types Z on the key the US layout calls Y.
        #expect(press(16, "z", cmd: true) == .chord("Mod+Z"))
    }

    @Test func chordsAreShownTheMacWay() {
        #expect(ShortcutCatalogue.display("Mod+Shift+N") == "\u{21E7}\u{2318}N")
        #expect(ShortcutCatalogue.display("Alt+ArrowDown") == "\u{2325}\u{2193}")
        #expect(ShortcutCatalogue.display("F7") == "F7")
        #expect(ShortcutCatalogue.words("Mod+N") == "Command+N")
        #expect(ShortcutCatalogue.keyboardShortcut(for: "Mod+Shift+,") != nil)
        #expect(ShortcutCatalogue.keyboardShortcut(for: "F3") != nil)
    }

    @Test func tooltipNamesTheCurrentKeys() async throws {
        let (app, model, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        #expect(ShortcutStore.shared.tooltipWords("New Project", "home.new_project") == "New Project (Command+N)")
        await model.remove("Mod+N", from: cmd("home.new_project"))
        #expect(ShortcutStore.shared.tooltipWords("New Project", "home.new_project") == "New Project")
    }
}

@Suite("Screenplay key commands", .serialized)
@MainActor
struct ScreenplayKeyCommandTests {
    private func chord(_ keyCode: UInt16, _ chars: String, command: Bool = true, shift: Bool = false) -> String {
        guard case let .chord(c) = ShortcutCatalogue.press(
            keyCode: keyCode, characters: chars, command: command, option: false, shift: shift, control: false)
        else { return "" }
        return c
    }

    @Test func defaultsReachTheEditor() {
        let store = ShortcutStore()
        #expect(ScreenplayKeyCommand.command(forChord: chord(36, "\r"), store: store) == .pageBreak)
        #expect(ScreenplayKeyCommand.command(forChord: chord(18, "1"), store: store) == .element(.sceneHeading))
        #expect(ScreenplayKeyCommand.command(forChord: chord(3, "f"), store: store) == .find)
        #expect(ScreenplayKeyCommand.command(forChord: chord(11, "b", shift: true), store: store) == nil)
    }

    @Test func remappedKeysReplaceTheDefaultsInTheEditor() {
        let store = ShortcutStore()
        store.set(ShortcutOverrides(overrides: [
            ShortcutOverride(commandId: "screenplay.page_break", keys: ["Mod+Shift+B"]),
            ShortcutOverride(commandId: "screenplay.element_action", keys: ["Mod+1"]),
            ShortcutOverride(commandId: "screenplay.element_scene_heading", keys: []),
            ShortcutOverride(commandId: "screenplay.find", keys: ["Mod+Shift+G"]),
        ]))
        #expect(ScreenplayKeyCommand.command(forChord: chord(11, "b", shift: true), store: store) == .pageBreak)
        #expect(ScreenplayKeyCommand.command(forChord: chord(36, "\r"), store: store) == nil)
        #expect(ScreenplayKeyCommand.command(forChord: chord(18, "1"), store: store) == .element(.action))
        #expect(ScreenplayKeyCommand.command(forChord: chord(5, "g", shift: true), store: store) == .find)
        #expect(ScreenplayKeyCommand.command(forChord: chord(3, "f"), store: store) == nil)
    }
}

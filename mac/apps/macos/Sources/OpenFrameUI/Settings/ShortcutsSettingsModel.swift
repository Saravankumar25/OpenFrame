// Keyboard Shortcuts (Windows: workspaces/settings/Shortcuts.tsx). Change a shortcut by pressing
// the new keys, remove one, add several per command, restore one command or everything. A key
// that another command already uses is never taken silently: the screen asks before moving it.
// The core checks every key combination and stores the set; this model only decides what to ask.

import Observation
import OpenFrameCore
import SwiftUI

@MainActor
@Observable
final class ShortcutsSettingsModel {
    struct Change: Equatable {
        var commandId: String
        /// nil restores the command's defaults.
        var keys: [String]?
    }

    struct Pending: Equatable {
        var title: String
        var body: String
        var confirm: String
        var changes: [Change]
    }

    struct RowError: Equatable {
        var id: String
        var text: String
    }

    var filter = ""
    /// The command whose new keys are being recorded.
    private(set) var recording: String?
    private(set) var error: RowError?
    var pending: Pending?
    var askingResetAll = false
    private(set) var busy = false
    /// The last confirmation shown, kept for the screen and for tests.
    private(set) var lastMessage: String?
    /// Where focus goes back to after a change (the row's Add button).
    private(set) var focusId: String?

    private let store: ShortcutStore
    private weak var app: AppModel?

    init(app: AppModel?, store: ShortcutStore = .shared) {
        self.app = app
        self.store = store
    }

    var overrides: [String: [String]] { store.overrides }
    var hasChanges: Bool { !store.overrides.isEmpty }

    var conflictList: [ShortcutCatalogue.Conflict] { ShortcutCatalogue.conflicts(overrides) }

    var shown: [ShortcutCommand] {
        let q = filter.trimmingCharacters(in: .whitespaces).lowercased()
        guard !q.isEmpty else { return ShortcutCatalogue.commands }
        return ShortcutCatalogue.commands.filter { c in
            c.label.lowercased().contains(q) || c.group.lowercased().contains(q)
                || ShortcutCatalogue.bindings(c.id, overrides).contains { ShortcutCatalogue.display($0).lowercased().contains(q) }
        }
    }

    var groups: [String] {
        var seen: [String] = []
        for c in shown where !seen.contains(c.group) { seen.append(c.group) }
        return seen
    }

    func commands(in group: String) -> [ShortcutCommand] { shown.filter { $0.group == group } }

    func current(_ c: ShortcutCommand) -> [String] { ShortcutCatalogue.bindings(c.id, overrides) }
    func isChanged(_ c: ShortcutCommand) -> Bool { overrides[c.id] != nil }

    /// "Also used by ..." for a command that shares a key with another.
    func conflictNames(_ c: ShortcutCommand) -> [String] {
        var seen: [String] = []
        for x in conflictList where x.commandId == c.id {
            let name = ShortcutCatalogue.label(x.otherId)
            if !seen.contains(name) { seen.append(name) }
        }
        return seen
    }

    // MARK: Recording

    func startRecording(_ c: ShortcutCommand) {
        error = nil
        recording = c.id
    }

    func cancelRecording() {
        if let id = recording { focusId = id }
        recording = nil
        error = nil
    }

    /// A key press while recording. Escape cancels; a modifier on its own keeps listening.
    func record(_ press: ShortcutCatalogue.Press, for c: ShortcutCommand) async {
        switch press {
        case .ignore:
            return
        case .unsupported:
            error = RowError(id: c.id, text: "That key combination can't be used.")
        case let .chord(chord):
            recording = nil
            focusId = c.id
            await add(chord, to: c)
        }
    }

    // MARK: Changes

    func add(_ chord: String, to c: ShortcutCommand) async {
        let current = current(c)
        if current.contains(chord) {
            error = RowError(id: c.id, text: "\(ShortcutCatalogue.display(chord)) is already a shortcut for \(c.label).")
            return
        }
        let next = Change(commandId: c.id, keys: current + [chord])
        let others = ShortcutCatalogue.whoUses(chord, scope: c.scope, overrides, except: c.id)
        if others.isEmpty {
            await save([next], done: "\(c.label): \(ShortcutCatalogue.display(chord))", failId: c.id)
            return
        }
        let names = others.map(ShortcutCatalogue.label).joined(separator: " and ")
        pending = Pending(
            title: "\(ShortcutCatalogue.display(chord)) is already used",
            body: "\(ShortcutCatalogue.display(chord)) runs \(names). Move it to \(c.label)? \(names) will keep any other shortcuts and its on-screen route.",
            confirm: "Move it here",
            changes: [next] + takeFrom(others, chord))
    }

    func remove(_ chord: String, from c: ShortcutCommand) async {
        focusId = c.id
        await save(
            [Change(commandId: c.id, keys: current(c).filter { $0 != chord })],
            done: "Removed \(ShortcutCatalogue.display(chord)) from \(c.label)", failId: c.id)
    }

    func restore(_ c: ShortcutCommand) async {
        focusId = c.id
        var without = overrides
        without[c.id] = nil
        var clashes: [(String, String)] = []
        for key in c.defaults {
            for other in ShortcutCatalogue.whoUses(key, scope: c.scope, without, except: c.id) { clashes.append((key, other)) }
        }
        let change = Change(commandId: c.id, keys: nil)
        if clashes.isEmpty {
            await save([change], done: "Restored the default for \(c.label)", failId: c.id)
            return
        }
        let said = clashes.map { "\(ShortcutCatalogue.display($0.0)) is now used by \(ShortcutCatalogue.label($0.1))." }.joined(separator: " ")
        pending = Pending(
            title: "Restore the default?",
            body: said + " Restoring gives it back to \(c.label).",
            confirm: "Restore and move",
            changes: [change] + clashes.flatMap { takeFrom([$0.1], $0.0) })
    }

    func confirmPending() async {
        guard let p = pending else { return }
        pending = nil
        await save(p.changes, done: "Shortcuts updated", failId: p.changes.first?.commandId ?? "")
    }

    func resetAll() async {
        askingResetAll = false
        guard let core = app?.core else { return }
        busy = true
        defer { busy = false }
        do {
            store.set(try await core.appResetShortcuts())
            say("All shortcuts are back to their defaults")
            error = nil
        } catch {
            app?.show("Could not restore the shortcuts", error)
        }
    }

    /// Without `chord` from these commands (the conflict resolution).
    private func takeFrom(_ ids: [String], _ chord: String) -> [Change] {
        ids.map { Change(commandId: $0, keys: ShortcutCatalogue.bindings($0, overrides).filter { $0 != chord }) }
    }

    private func save(_ changes: [Change], done: String, failId: String) async {
        guard let core = app?.core else { return }
        busy = true
        defer { busy = false }
        do {
            let args = SetShortcutsArgs(changes: changes.map { ShortcutChange(commandId: $0.commandId, keys: $0.keys) })
            store.set(try await core.appSetShortcuts(args))
            say(done)
            error = nil
        } catch {
            let failure = CoreFailure(asCore: error)
            if failure.code.hasPrefix("validation") {
                // The core's words name Ctrl; on the Mac the key is Command.
                self.error = RowError(id: failId, text: failure.message.replacingOccurrences(of: "Ctrl", with: "Command"))
            } else {
                app?.show("Could not change the shortcut", error)
            }
        }
    }

    private func say(_ text: String) {
        lastMessage = text
        ToastCenter.shared.show(text)
    }

    func focusHandled() { focusId = nil }
}

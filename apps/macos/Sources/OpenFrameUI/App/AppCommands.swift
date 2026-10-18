// Menus with Mac conventions: File (New Project, Open, Close Project), Edit (project undo and
// redo, beside text undo), View (workspaces), Help.

import SwiftUI
import OpenFrameCore

struct AppCommands: Commands {
    let model: AppModel
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        CommandGroup(replacing: .appInfo) {
            Button("About OpenFrame Studio") { openWindow(id: "about") }
        }
        CommandGroup(after: .textEditing) {
            Divider()
            Button("Search and Commands\u{2026}") { NotificationCenter.default.post(name: .openFrameShowSearch, object: nil) }
                .keyboardShortcut("k", modifiers: .command)
                .disabled(!isHome && !isProject)
        }
        CommandGroup(replacing: .newItem) {
            Button("New Project…") { NotificationCenter.default.post(name: .openFrameNewProject, object: nil) }
                .keyboardShortcut("n", modifiers: .command)
                .disabled(!isHome)
            Button("Open Project…") { NotificationCenter.default.post(name: .openFrameOpenProject, object: nil) }
                .keyboardShortcut("o", modifiers: .command)
                .disabled(!isHome)
            Divider()
            Button("Close Project") { Task { await model.closeProject() } }
                .keyboardShortcut("w", modifiers: [.command, .shift])
                .disabled(!isProject)
        }
        CommandGroup(after: .undoRedo) {
            Divider()
            Button("Undo Project Change") { Task { await model.undo() } }
                .keyboardShortcut("z", modifiers: [.command, .option])
                .disabled(!isProject)
            Button("Redo Project Change") { Task { await model.redo() } }
                .keyboardShortcut("z", modifiers: [.command, .option, .shift])
                .disabled(!isProject)
        }
        CommandMenu("Go") {
            ForEach(Workspace.allCases.filter { $0.shortcut != nil }) { ws in
                Button(ws.title) { NotificationCenter.default.post(name: .openFrameShowWorkspace, object: ws.rawValue) }
                    .keyboardShortcut(ws.shortcut!, modifiers: [.command, .control])
                    .disabled(!isProject)
            }
        }
    }

    private var isHome: Bool { if case .home = model.phase { true } else { false } }
    private var isProject: Bool { if case .project = model.phase { true } else { false } }
}

extension AppModel {
    /// Undo the last project change (history.undo). Text fields keep their own Command+Z.
    public func undo() async {
        do { _ = try await core?.historyUndo(StoreArgs(store: .project)) } catch { show("Could not undo", error) }
    }

    public func redo() async {
        do { _ = try await core?.historyRedo(StoreArgs(store: .project)) } catch { show("Could not redo", error) }
    }
}

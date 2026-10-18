// Menus with Mac conventions: File (New Project, Open, Close Project, Import, Export, Print, Packages), Edit (project undo and
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
        CommandGroup(replacing: .importExport) {
            // Import, export and packages (Mac phase M7): the sheets live in Interchange/ and Packages/.
            Button("Import Screenplay\u{2026}") { show(.importScreenplay(openScreenplayId: nil)) }
                .keyboardShortcut("i", modifiers: [.command, .shift])
                .disabled(!isProject)
            Button("Export Screenplay\u{2026}") { show(.exportScreenplay(draftId: nil)) }
                .keyboardShortcut("e", modifiers: [.command, .shift])
                .disabled(!isProject)
            Button("Export PDF\u{2026}") { Task { await InterchangePresenter.shared.exportPDFDirectly(app: model, draftId: nil) } }
                .keyboardShortcut("e", modifiers: [.command, .option, .shift])
                .disabled(!isProject)
            Divider()
            Button("Back Up Project\u{2026}") { show(.backup) }.disabled(!isProject)
            Button("Export Full Project\u{2026}") { show(.exportProject) }.disabled(!isProject)
            Menu("Share for Review") {
                ForEach(PKInfo.exchangeExportTypes, id: \.self) { t in
                    Button(PKInfo.entry(t).menuLabel) { show(.exportExchange(t)) }
                }
            }
            .disabled(!isProject)
            Button("Import Exchange Package\u{2026}") { show(.importExchange(sessionId: nil)) }.disabled(!isProject)
            Button("Open Review Package\u{2026}") { show(.openReview) }.disabled(!isHome && !isProject)
            Button("Review Queue\u{2026}") { show(.queue) }.disabled(!isProject)
            Button("Import & Export History\u{2026}") { show(.history) }.disabled(!isProject)
            Divider()
            Button("Restore Backup or Package\u{2026}") { show(.openProject) }.disabled(!isHome && !isProject)
        }
        CommandGroup(replacing: .printItem) {
            Button("Print\u{2026}") { show(.printPreview(draftId: nil)) }
                .keyboardShortcut("p", modifiers: .command)
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
            // Every workspace, so the keyboard reaches each one; the main ones have a shortcut.
            ForEach(Workspace.allCases) { ws in
                if let key = ws.shortcut {
                    Button(ws.title) { NotificationCenter.default.post(name: .openFrameShowWorkspace, object: ws.rawValue) }
                        .keyboardShortcut(key, modifiers: [.command, .control])
                        .disabled(!isProject)
                } else {
                    Button(ws.title) { NotificationCenter.default.post(name: .openFrameShowWorkspace, object: ws.rawValue) }
                        .disabled(!isProject)
                }
            }
        }
    }

    private func show(_ dialog: InterchangeDialog) {
        InterchangePresenter.shared.configure(app: model)
        InterchangePresenter.shared.present(dialog)
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

// Menus with Mac conventions: File (New Project, Open, Close Project, Import, Export, Print, Packages), Edit (project undo and
// redo, beside text undo), View (workspaces), Help.

import SwiftUI
import OpenFrameCore

struct AppCommands: Commands {
    let model: AppModel
    @Environment(\.openWindow) private var openWindow
    /// The keys now in force for the commands the person can remap (Keyboard Shortcuts).
    private let keys = ShortcutStore.shared

    var body: some Commands {
        CommandGroup(replacing: .appInfo) {
            Button("About OpenFrame Studio") { openWindow(id: "about") }
        }
        CommandGroup(after: .appSettings) {
            // Settings\u{2026} (Command+Comma) comes from the Settings window; the project's own settings follow it.
            Button("Project Settings\u{2026}") { ProjectSettingsRouter.open() }
                .keyboardShortcut(keys.shortcut("app.project_settings"))
                .disabled(!isProject)
        }
        CommandGroup(after: .help) {
            Divider()
            Button("Keyboard Shortcuts\u{2026}") { NotificationCenter.default.post(name: .openFrameShowShortcuts, object: nil) }
                .disabled(!isHome && !isProject)
        }
        CommandGroup(after: .textEditing) {
            Divider()
            Button("Search and Commands\u{2026}") { NotificationCenter.default.post(name: .openFrameShowSearch, object: nil) }
                .keyboardShortcut(keys.shortcut("app.search"))
                .disabled(!isHome && !isProject)
        }
        CommandGroup(replacing: .newItem) {
            Button("New Project…") { NotificationCenter.default.post(name: .openFrameNewProject, object: nil) }
                .keyboardShortcut(keys.shortcut("home.new_project"))
                .disabled(!isHome)
            Button("Open Project…") { NotificationCenter.default.post(name: .openFrameOpenProject, object: nil) }
                .keyboardShortcut(keys.shortcut("home.open_project"))
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
        CommandGroup(replacing: .sidebar) {
            // The project's workspace sidebar (also the sidebar button in the toolbar).
            Button(ProjectSidebar.shared.shown ? "Hide Sidebar" : "Show Sidebar") {
                NotificationCenter.default.post(name: .openFrameToggleSidebar, object: nil)
            }
            .keyboardShortcut("s", modifiers: [.command, .control])
            .disabled(!isProject)
        }
        CommandMenu("Go") {
            // Back and Forward through the pages visited (Windows: the top-bar arrows).
            Button("Back") { NotificationCenter.default.post(name: .openFrameNavigateBack, object: nil) }
                .keyboardShortcut("[", modifiers: .command)
                .disabled(!isProject || !ProjectSidebar.shared.canGoBack)
            Button("Forward") { NotificationCenter.default.post(name: .openFrameNavigateForward, object: nil) }
                .keyboardShortcut("]", modifiers: .command)
                .disabled(!isProject || !ProjectSidebar.shared.canGoForward)
            Divider()
            // The sidebar workspaces with Control+Command+1 to 8, then the project pages.
            ForEach(Workspace.allCases) { ws in
                if let key = ws.shortcut {
                    Button(ws.title) { ProjectNav.show(ws) }
                        .keyboardShortcut(key, modifiers: [.command, .control])
                        .disabled(!isProject)
                } else {
                    Button(ws.title) { ProjectNav.show(ws) }
                        .disabled(!isProject)
                }
            }
            Menu("Production Tab") {
                ForEach(ProductionTab.allCases) { tab in
                    Button(tab.title) { ProjectNav.show(.production, tab: tab) }
                }
            }
            .disabled(!isProject)
            Divider()
            Button("Project Settings") { ProjectSettingsRouter.open() }
                .disabled(!isProject)
            Button("Keyboard Shortcuts") { ProjectSettingsRouter.open(tab: .shortcuts) }
                .disabled(!isProject)
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

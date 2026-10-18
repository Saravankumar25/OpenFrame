// Project lifecycle from Home and from the project menu: rename, duplicate, archive and restore,
// delete (to the Trash), locate a moved project, show in Finder. Every rule lives in the core
// (project.rename, project.duplicate, project.set_archived, project.delete, project.locate); this
// file asks, shows the Windows wording and reports the answer in a toast.

import AppKit
import SwiftUI
import OpenFrameCore

@MainActor
struct ProjectActions {
    let app: AppModel
    private var toasts: ToastCenter { .shared }

    /// Rename. Throws the core's failure so the sheet can show it next to the field.
    func rename(projectId: String, to title: String) async throws {
        guard let core = app.core else { return }
        let clean = title.trimmingCharacters(in: .whitespacesAndNewlines)
        _ = try await core.projectRename(RenameProjectArgs(projectId: projectId, title: clean))
        toasts.show("Renamed to \u{201C}\(clean)\u{201D}.")
    }

    func duplicate(_ project: RecentProject) async {
        guard let core = app.core else { return }
        do {
            let copy = try await core.projectDuplicate(ProjectIdArgs(projectId: project.projectId))
            toasts.show("Created \u{201C}\(copy.title)\u{201D}. It is a separate project with its own identity.")
        } catch {
            app.show("Could not duplicate the project", error)
        }
    }

    func setArchived(_ project: RecentProject, _ archived: Bool) async {
        guard let core = app.core else { return }
        do {
            _ = try await core.projectSetArchived(SetArchivedArgs(projectId: project.projectId, archived: archived))
            toasts.show(archived
                ? "\u{201C}\(project.title)\u{201D} was archived. It is not deleted; find it with Show archived."
                : "\u{201C}\(project.title)\u{201D} is back in your projects.")
        } catch {
            app.show(archived ? "Could not archive the project" : "Could not restore the project", error)
        }
    }

    /// Moves the project folder to the Trash. Throws so the sheet can stay open on a failure.
    func delete(_ project: RecentProject) async throws {
        guard let core = app.core else { return }
        _ = try await core.projectDelete(ProjectIdArgs(projectId: project.projectId))
        toasts.show("\u{201C}\(project.title)\u{201D} was moved to the Trash.")
    }

    /// Point a moved project at its new folder. The core opens it, so the window closes it again
    /// and opens it the normal way (one place decides what the window shows).
    func locate(_ project: RecentProject, at path: String) async -> Bool {
        guard let core = app.core else { return false }
        do {
            let result = try await core.projectLocate(LocateArgs(projectId: project.projectId, path: path))
            await app.closeProject()
            await app.openProject(path: result.project.path)
            return true
        } catch {
            app.show("Could not locate the project", error)
            return false
        }
    }

    func forget(_ project: RecentProject) async {
        guard let core = app.core else { return }
        do {
            _ = try await core.projectRemoveRecent(ProjectIdArgs(projectId: project.projectId))
            toasts.show("\u{201C}\(project.title)\u{201D} was removed from the list. Nothing was deleted.")
        } catch {
            app.show("Could not remove the project from the list", error)
        }
    }

    func reveal(projectId: String) async {
        do { try await app.core?.revealLocation("project", id: projectId) } catch { app.show("Could not show the folder", error) }
    }

    /// Change the status of the open project; one undoable step, with an Undo toast.
    func setStatus(_ status: ProjectStatus) async {
        guard let core = app.core else { return }
        do {
            _ = try await core.projectSetStatus(SetStatusArgs(status: status))
            toasts.undoable("Changed project status to \(status.rawValue)", app: app)
        } catch {
            app.show("Could not change the status", error)
        }
    }

    /// The folder picker for Locate. Returns nil when the person cancels.
    static func chooseFolder(title: String) -> String? {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Locate"
        panel.message = "Locate \u{201C}\(title)\u{201D}"
        return panel.runModal() == .OK ? panel.url?.path : nil
    }
}

// MARK: Sheets

struct RenameProjectSheet: View {
    let project: RecentProject
    let actions: ProjectActions
    let done: () -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var title: String
    @State private var busy = false
    @State private var error: String?
    @FocusState private var focused: Bool

    init(project: RecentProject, actions: ProjectActions, done: @escaping () -> Void) {
        self.project = project
        self.actions = actions
        self.done = done
        _title = State(initialValue: project.title)
    }

    private var trimmed: String { title.trimmingCharacters(in: .whitespacesAndNewlines) }
    private var invalid: Bool { trimmed.isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Rename Project").font(.title2.weight(.semibold))
            TextField("Title", text: $title)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit { Task { await save() } }
                .onChange(of: title) { _, _ in error = nil }
                .accessibilityIdentifier("rename-title")
            if invalid {
                Text("A title is required.").font(.callout).foregroundStyle(.red)
            } else if let error {
                Text(error).font(.callout).foregroundStyle(.red)
            }
            Text("Renaming changes the name shown everywhere. The project folder and its contents stay where they are.")
                .font(.callout).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button(busy ? "Renaming…" : "Rename") { Task { await save() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(invalid || busy)
                    .accessibilityIdentifier("rename-confirm")
            }
        }
        .padding(24)
        .frame(width: 440)
        .onAppear { focused = true }
    }

    private func save() async {
        guard !invalid, !busy else { return }
        if trimmed == project.title { dismiss(); return }
        busy = true
        defer { busy = false }
        do {
            try await actions.rename(projectId: project.projectId, to: trimmed)
            done()
            dismiss()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

/// The most destructive action: the project name is typed to confirm, and Archive Instead is offered.
struct DeleteProjectSheet: View {
    let project: RecentProject
    let actions: ProjectActions
    let done: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme
    @State private var typed = ""
    @State private var busy = false
    @State private var error: String?

    private var matches: Bool { typed.trimmingCharacters(in: .whitespaces) == project.title.trimmingCharacters(in: .whitespaces) }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Delete \u{201C}\(project.title)\u{201D}?")
                .font(.title2.weight(.semibold))
                .lineLimit(3)
            Label("This is the most destructive action in OpenFrame. The whole project will be removed from this Mac.", systemImage: "exclamationmark.triangle.fill")
                .font(.callout.weight(.semibold))
                .foregroundStyle(theme["red-t"])
                .fixedSize(horizontal: false, vertical: true)
            Text("The project folder (ideas, story, screenplay, production plans and stored files) is moved to the Trash. Files linked from other folders are not touched.")
                .font(.callout)
                .fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: 4) {
                Text("Type the project name to confirm").font(.callout.weight(.medium))
                TextField(project.title, text: $typed)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { Task { await remove() } }
                    .accessibilityIdentifier("delete-confirm-field")
                Text("Archiving instead keeps everything and simply hides the project from your recent list.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            if let error { Text(error).font(.callout).foregroundStyle(.red) }
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                if !project.archived {
                    Button("Archive Instead") {
                        Task { await actions.setArchived(project, true); done(); dismiss() }
                    }
                }
                Button(busy ? "Deleting…" : "Delete Project", role: .destructive) { Task { await remove() } }
                    .disabled(!matches || busy)
                    .accessibilityIdentifier("delete-confirm")
            }
        }
        .padding(24)
        .frame(width: 500)
    }

    private func remove() async {
        guard matches, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            try await actions.delete(project)
            done()
            dismiss()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

/// A project whose folder cannot be found: locate it, try again, or forget it.
struct UnavailableProjectSheet: View {
    let project: RecentProject
    let actions: ProjectActions
    let done: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("This project could not be opened").font(.title2.weight(.semibold))
            Label("The project location is unavailable. It may be on an external drive that is not connected, or the folder was moved.", systemImage: "exclamationmark.triangle.fill")
                .font(.callout.weight(.semibold))
                .foregroundStyle(theme["red-t"])
                .fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: 2) {
                Text("Project: \(project.title)")
                Text("Last known location: \(project.path)").textSelection(.enabled)
            }
            .font(.callout)
            .foregroundStyle(.secondary)
            Text("OpenFrame has not created a replacement project. Reconnect the drive and try again, or locate another copy of the project.")
                .font(.callout)
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                Button("Remove from List") { Task { await actions.forget(project); done(); dismiss() } }
                    .accessibilityIdentifier("unavailable-forget")
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button("Locate Project…") {
                    guard let path = ProjectActions.chooseFolder(title: project.title) else { return }
                    Task {
                        if await actions.locate(project, at: path) { done(); dismiss() }
                    }
                }
                .accessibilityIdentifier("unavailable-locate")
                Button("Try Again") {
                    Task { await actions.app.openProject(path: project.path); done(); dismiss() }
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(24)
        .frame(width: 520)
    }
}

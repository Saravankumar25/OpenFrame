// Settings > Storage (Windows: features/storage/StoragePanel.tsx, VaultActions.tsx,
// ProjectStorage.tsx and library-setup/LibraryActions.tsx): where OpenFrame keeps each kind of
// data on this Mac and the actions that change a place. Every check is in the core; this model
// asks for a folder or a file, shows what the core found, and starts the core's task.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class StorageModel {
    private(set) var info: StorageInfo?
    private(set) var loadError: String?
    /// Files in the Global Idea Vault folder that no item uses.
    private(set) var orphans: VaultOrphanFiles?
    /// Former owners of private notes in the open project (Owner only; empty otherwise).
    private(set) var orphanOwners: [OrphanNoteOwner] = []
    /// The long action in its sheet.
    var action: StorageTaskModel?
    /// What Locate Existing Vault found, before it is connected.
    var vaultFound: VaultFolderInspection?
    /// What Locate Existing Library found, before it is connected.
    var libraryFound: LibraryInspection?
    /// The owner whose notes Make Them Mine asks about.
    var adopting: OrphanNoteOwner?
    /// The last thing that happened, in plain words (the Settings window has no toasts).
    private(set) var notice: String?
    private(set) var error: String?
    private(set) var busy = false

    let app: AppModel
    @ObservationIgnored var picker: StoragePicking = PanelStoragePicker()
    /// Injected by tests; the real one reads the App Sandbox and the licence channel.
    @ObservationIgnored var storeBuild: () -> Bool

    init(app: AppModel) {
        self.app = app
        storeBuild = { [weak app] in app?.isStoreBuild ?? false }
    }

    var canRelocate: Bool { StorageRules.canRelocate(storeBuild: storeBuild()) }

    // MARK: Loading

    func load() async {
        guard let core = app.core else { return }
        do {
            let s = try await core.storageInfo()
            info = s
            loadError = nil
            orphans = s.vaultExists ? try? await core.vaultOrphanFiles(VaultOverviewArgs(store: .global)) : nil
            // Owner only: other roles get a permission error and the section stays hidden.
            orphanOwners = s.projectDir != nil ? ((try? await core.projectOrphanNotes()) ?? []) : []
        } catch {
            loadError = CoreFailure(asCore: error).message
        }
    }

    private func say(_ text: String) {
        notice = text
        error = nil
    }

    private func fail(_ e: Error) {
        error = CoreFailure(asCore: e).message
        notice = nil
    }

    func dismissMessages() {
        notice = nil
        error = nil
    }

    // MARK: Places

    func reveal(_ kind: String) async {
        do { try await app.core?.revealLocation(kind) } catch { fail(error) }
    }

    /// Change Folder for new projects. Projects already made stay where they are.
    func changeProjectsDir() async {
        guard canRelocate, let core = app.core,
              let dir = picker.folder(title: "Choose the folder for new projects") else { return }
        do {
            info = try await core.storageSetProjectsDir(SetProjectsDirArgs(path: dir))
            say("New projects will be created in that folder. Projects you already have stay where they are.")
        } catch {
            fail(error)
        }
    }

    // MARK: Library

    func moveLibrary() {
        guard canRelocate, let info,
              let dest = picker.folder(title: "Choose where your OpenFrame library should live") else { return }
        let target = StorageRules.joinPath(dest, StorageRules.folderName(info.libraryDir))
        action = StorageTaskModel(
            app: app, title: "Move Your OpenFrame Library",
            confirm: [
                "From \(info.libraryDir)\nTo \(target)",
                "This moves \(StorageRules.libraryMoveParts(info).joined(separator: ", ")). New projects are then created in the Projects folder of the moved library.",
                "Everything is copied and checked first. The old folder is removed only after OpenFrame works from the new one. Projects you keep outside the library stay where they are.",
            ],
            confirmLabel: "Move Library", relocates: true,
            start: { try await $0.libraryMove(MoveToFolderArgs(destParent: dest)).taskId },
            done: { StorageRules.libraryMoveDone($0.result(LibraryMoveReport.self), target: target) },
            onFinished: { [weak self] in await self?.load() })
    }

    func locateLibrary() async {
        guard canRelocate, let core = app.core,
              let dir = picker.folder(title: "Choose the folder of an existing OpenFrame library") else { return }
        do {
            libraryFound = try await core.libraryInspect(LibraryDirArgs(dir: dir))
        } catch {
            fail(error)
        }
    }

    /// The found library is the one in use already.
    func isCurrentLibrary(_ found: LibraryInspection) -> Bool {
        guard let root = found.libraryRoot, let current = info?.libraryDir else { return false }
        return StorageRules.inside(current, root) && StorageRules.inside(root, current)
    }

    /// Connect Library. Throws the core's refusal so the sheet keeps it on screen.
    func connectLibrary(_ found: LibraryInspection) async throws {
        guard let core = app.core, let root = found.libraryRoot else { return }
        _ = try await core.libraryConnect(LibraryDirArgs(dir: root))
        libraryFound = nil
        app.changes.bumpAll()
        await load()
        say("The library is connected. Nothing was moved or copied.")
    }

    // MARK: Global Idea Vault

    func archive(_ kind: VaultArchiveKind) {
        guard let info else { return }
        let backup = kind == .backup
        guard let path = picker.saveVault(
            title: backup ? "Save the vault backup to" : "Save the vault package to",
            suggestedName: StorageRules.archiveFileName(kind), directory: info.libraryDir) else { return }
        action = StorageTaskModel(
            app: app,
            title: backup ? "Back Up the Global Idea Vault" : "Export the Global Idea Vault",
            confirm: [backup
                ? "A full copy for recovery on this computer: every idea, picture and Recently Deleted item. Saved to \(path)."
                : "A copy to move your vault to another computer. Recently Deleted items and the change history are left out. Saved to \(path)."],
            confirmLabel: backup ? "Back Up" : "Export",
            start: { core in
                backup ? try await core.vaultBackup(VaultFileArgs(path: path)).taskId
                    : try await core.vaultExportPackage(VaultFileArgs(path: path)).taskId
            },
            done: { StorageRules.archiveDone($0.result(VaultArchiveReport.self)) })
    }

    func restore() async {
        guard let core = app.core, let path = picker.vaultFile(title: "Choose a vault backup or package") else { return }
        let v: VaultInspection
        do {
            v = try await core.vaultInspect(VaultFileArgs(path: path))
        } catch {
            return fail(error)
        }
        action = StorageTaskModel(
            app: app, title: "Restore the Global Idea Vault",
            confirm: [
                StorageRules.restoreSummary(v),
                "It replaces your current vault. Your current vault is not deleted: it is kept beside it in a folder named \u{201C}(before restore)\u{201D}, so you can go back.",
            ],
            confirmLabel: "Replace and Restore",
            start: { try await $0.vaultRestoreBackup(VaultFileArgs(path: path)).taskId },
            done: { StorageRules.restoreDone($0.result(VaultRestoreReport.self)) },
            onFinished: { [weak self] in
                self?.app.changes.bumpAll()
                await self?.load()
            })
    }

    func moveVault() {
        guard canRelocate, let info,
              let dest = picker.folder(title: "Choose where the Global Idea Vault should live") else { return }
        let target = StorageRules.joinPath(dest, StorageRules.folderName(info.vaultDir))
        action = StorageTaskModel(
            app: app, title: "Move the Global Idea Vault",
            confirm: ["The vault moves to \(target). It is copied and checked first; only then is the old copy removed. If anything goes wrong, it stays where it is now."],
            confirmLabel: "Move Vault",
            start: { try await $0.vaultMove(MoveToFolderArgs(destParent: dest)).taskId },
            done: { StorageRules.vaultMoveDone($0.result(VaultMoveReport.self), target: target) },
            onFinished: { [weak self] in
                self?.app.changes.bumpAll()
                await self?.load()
            })
    }

    func locateVault() async {
        guard canRelocate, let core = app.core,
              let dir = picker.folder(title: "Choose the folder of an existing Global Idea Vault") else { return }
        do {
            let v = try await core.vaultInspectFolder(LocateVaultArgs(dir: dir))
            switch StorageRules.locateOutcome(v) {
            case let .refuse(text): error = text; notice = nil
            case let .alreadyInUse(text): say(text)
            case let .confirm(found): vaultFound = found
            }
        } catch {
            fail(error)
        }
    }

    /// Connect Vault, after the preview. Throws the core's refusal so the sheet can show it.
    func connectVault(_ v: VaultFolderInspection) async throws {
        guard let core = app.core, let dir = v.vaultDir else { return }
        _ = try await core.vaultLocate(LocateVaultArgs(dir: dir))
        vaultFound = nil
        app.changes.bumpAll()
        await load()
        say("The Global Idea Vault is connected. Nothing was moved or copied.")
    }

    /// Add Them Back for the Global Idea Vault.
    func recoverVaultFiles() async {
        guard let core = app.core, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.vaultRecoverFiles(VaultOverviewArgs(store: .global))
            let lines = [StorageRules.recoverAdded(r, global: true), StorageRules.recoverFailed(r)].compactMap { $0 }
            if r.failed > 0 { error = lines.joined(separator: " "); notice = nil } else if !lines.isEmpty { say(lines.joined(separator: " ")) }
            orphans = try? await core.vaultOrphanFiles(VaultOverviewArgs(store: .global))
        } catch {
            fail(error)
        }
    }

    // MARK: This project

    func moveProject() {
        guard canRelocate, let path = info?.projectDir,
              let dest = picker.folder(title: "Choose where this project should live") else { return }
        let target = StorageRules.joinPath(dest, StorageRules.folderName(path))
        action = StorageTaskModel(
            app: app, title: "Move This Project",
            confirm: ["The project folder moves to \(target). The project closes for a moment, is copied and checked, then opens from the new place; only then is the old folder removed. If anything goes wrong, it stays where it is now. Linked files outside the project are not moved."],
            confirmLabel: "Move Project", relocates: true,
            start: { try await $0.projectMove(MoveToFolderArgs(destParent: dest)).taskId },
            done: { StorageRules.projectMoveDone($0.result(ProjectMoveReport.self), target: target) },
            onFinished: { [weak self] in await self?.load() })
    }

    /// Make Them Mine, after the confirmation: one owner at a time.
    func adopt(_ owner: OrphanNoteOwner) async {
        guard let core = app.core, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.projectAdoptOrphanNotes(AdoptOrphanNotesArgs(ownerUserId: owner.ownerUserId))
            adopting = nil
            say(StorageRules.adoptedText(r.adopted))
            orphanOwners = (try? await core.projectOrphanNotes()) ?? []
        } catch {
            adopting = nil
            fail(error)
        }
    }
}

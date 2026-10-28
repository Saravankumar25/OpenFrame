// Settings > Storage against the real core on a temporary folder (Windows: StoragePanel.test.tsx,
// VaultActions.test.tsx, LibraryActions.test.tsx and tests/e2e/specs/71-storage.test.mjs): the
// places, Change Folder, the vault backup, restore and locate, Add Them Back, the project and
// library moves (the window shows the moved project again), Make Them Mine, and the store build.

import Foundation
import SQLite3
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Storage panel", .serialized)
@MainActor
struct StoragePanelTests {
    @MainActor struct Session {
        let root: URL
        let app: AppModel
        let model: StorageModel
        let picker: StubStoragePicker
        var core: Core { app.core! }
    }

    /// `library`: a real library set up like on a new Mac (<root>/home/OpenFrame), instead of the
    /// test folder itself, which also holds the app data.
    private func start(project: String? = nil, library: Bool = false) async throws -> Session {
        let root = try Fixture.root()
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        options.testLibrarySetup = library
        let app = AppModel(options: options)
        await app.start()
        if library {
            await app.libraryReady(try await app.core!.libraryUseDefault())
        }
        #expect(app.phase == .home)
        if let project {
            #expect(await app.createProject(CreateProjectArgs(title: project, projectType: .featureFilm)))
        }
        let model = StorageModel(app: app)
        let picker = StubStoragePicker()
        model.picker = picker
        model.storeBuild = { false }
        await model.load()
        return Session(root: root, app: app, model: model, picker: picker)
    }

    private func finish(_ s: Session) {
        s.app.shutdown()
        try? FileManager.default.removeItem(at: s.root)
    }

    private func folder(_ s: Session, _ name: String) throws -> URL {
        let url = s.root.appendingPathComponent(name, isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    /// Starts the action the model prepared and waits for its end.
    private func run(_ s: Session) async throws -> StorageTaskModel {
        let action = try #require(s.model.action)
        #expect(action.stage == .confirm)
        await action.start()
        #expect(action.startError == nil)
        await action.waitUntilSettled()
        return action
    }

    private func completed(_ action: StorageTaskModel) -> String? {
        if case let .completed(text) = action.stage { return text }
        Issue.record("the task did not complete: \(action.stage)")
        return nil
    }

    // MARK: Places

    @Test func thePlacesComeFromTheCore() async throws {
        let s = try await start()
        defer { finish(s) }
        let info = try #require(s.model.info)
        #expect(info.libraryDir == s.root.path || info.libraryDir.hasPrefix(s.root.path))
        #expect(info.projectsDirIsDefault)
        #expect(info.vaultExists == false)
        #expect(info.projectDir == nil)
        #expect(s.model.orphans == nil)
        #expect(s.model.canRelocate)
    }

    @Test func changingTheProjectsFolderLeavesProjectsWhereTheyAre() async throws {
        let s = try await start(project: "Harbour Ledger")
        defer { finish(s) }
        let before = try #require(s.model.info?.projectDir)
        let films = try folder(s, "Films")
        s.picker.folders = [films.path]
        await s.model.changeProjectsDir()
        #expect(s.model.info?.projectsDir == films.path)
        #expect(s.model.info?.projectsDirIsDefault == false)
        #expect(s.model.notice == "New projects will be created in that folder. Projects you already have stay where they are.")
        #expect(try await s.core.storageInfo().projectDir == before)
    }

    @Test func theStoreBuildHidesEveryMoveLocateAndChange() async throws {
        let s = try await start(project: "Store Night")
        defer { finish(s) }
        s.model.storeBuild = { true }
        #expect(!s.model.canRelocate)
        s.picker.folders = Array(repeating: s.root.path, count: 6)
        s.model.moveLibrary()
        await s.model.locateLibrary()
        await s.model.changeProjectsDir()
        s.model.moveVault()
        await s.model.locateVault()
        s.model.moveProject()
        #expect(s.picker.asked.isEmpty, "no folder panel in the store build")
        #expect(s.model.action == nil)
    }

    // MARK: Global Idea Vault

    @Test func backUpThenRestoreTheVault() async throws {
        let s = try await start()
        defer { finish(s) }
        _ = try await s.core.vaultCreate(VaultCreateArgs(store: .global, itemType: .note, title: "Tide tables", body: "High water at 06:40."))
        await s.model.load()
        #expect(s.model.info?.vaultExists == true)

        let backup = s.root.appendingPathComponent("vault-out/Global Idea Vault backup.ofvault")
        try FileManager.default.createDirectory(at: backup.deletingLastPathComponent(), withIntermediateDirectories: true)
        s.picker.save = backup.path
        s.model.archive(.backup)
        #expect(s.model.action?.title == "Back Up the Global Idea Vault")
        let saved = try await run(s)
        #expect(completed(saved)?.hasPrefix("Saved Global Idea Vault backup.ofvault") == true)
        #expect(FileManager.default.fileExists(atPath: backup.path))

        _ = try await s.core.vaultCreate(VaultCreateArgs(store: .global, itemType: .note, title: "Later idea"))
        s.model.action = nil
        s.picker.file = backup.path
        await s.model.restore()
        let restore = try #require(s.model.action)
        #expect(restore.confirm.first?.contains("is a backup from") == true)
        #expect(restore.confirmLabel == "Replace and Restore")
        let done = try await run(s)
        #expect(completed(done)?.hasPrefix("Restored. Your previous vault is kept at") == true)
        let titles = try await s.core.vaultList(VaultListArgs(store: .global, view: .all)).map(\.title)
        #expect(titles.contains("Tide tables"))
        #expect(!titles.contains("Later idea"))
    }

    @Test func locateRefusesAnEmptyFolderAndAProject() async throws {
        let s = try await start(project: "Locate Night")
        defer { finish(s) }
        let empty = try folder(s, "Nothing here")
        s.picker.folders = [empty.path, try #require(s.model.info?.projectDir)]
        await s.model.locateVault()
        #expect(s.model.error == "There is no Global Idea Vault in that folder. Choose the vault folder, or the OpenFrame folder that holds it.")
        #expect(s.model.vaultFound == nil)
        await s.model.locateVault()
        #expect(s.model.error == "That folder is a project, not a vault. Choose the folder that holds your Global Idea Vault.")
    }

    @Test func moveTheVaultThenLocateTheOldPlaceSaysWhatItFound() async throws {
        let s = try await start()
        defer { finish(s) }
        _ = try await s.core.vaultCreate(VaultCreateArgs(store: .global, itemType: .note, title: "Harbour wall"))
        await s.model.load()
        let dest = try folder(s, "Elsewhere")
        s.picker.folders = [dest.path]
        s.model.moveVault()
        let moved = try await run(s)
        #expect(completed(moved)?.hasPrefix("The vault is now at \(dest.path)/") == true)
        #expect(s.model.info?.vaultDir.hasPrefix(dest.path) == true)

        // The vault in use: Locate says so and changes nothing.
        s.picker.folders = [dest.path]
        await s.model.locateVault()
        #expect(s.model.notice == "This vault is already the one in use. Nothing was changed.")
    }

    @Test func addThemBackRecoversFilesNoItemUses() async throws {
        let s = try await start()
        defer { finish(s) }
        let source = s.root.appendingPathComponent("still.txt")
        try "a still from the harbour".write(to: source, atomically: true, encoding: .utf8)
        _ = try await s.core.vaultAddFiles(VaultAddFilesArgs(store: .global, paths: [source.path]))
        try plantOrphan(vaultDir: try await s.core.storageInfo().vaultDir)
        await s.model.load()
        let orphans = try #require(s.model.orphans)
        #expect(orphans.count == 1)
        #expect(StorageRules.orphanLine(orphans).hasPrefix("1 file in the vault folder is not in the vault ("))
        await s.model.recoverVaultFiles()
        #expect(s.model.notice == "Added 1 file back to the Global Idea Vault.")
        #expect(s.model.orphans?.count == 0)
        #expect(try await s.core.vaultList(VaultListArgs(store: .global, view: .all)).count == 2)
    }

    // MARK: Moves

    @Test func movingTheOpenProjectShowsItAgainFromTheNewPlace() async throws {
        let s = try await start(project: "Harbour Ledger")
        defer { finish(s) }
        guard case let .project(before) = s.app.phase else { Issue.record("no project open"); return }
        let dest = try folder(s, "moved-projects")
        s.picker.folders = [dest.path]
        s.model.moveProject()
        let action = try #require(s.model.action)
        #expect(action.relocates)
        #expect(action.confirmLabel == "Move Project")
        let moved = try await run(s)
        #expect(completed(moved)?.hasPrefix("The project is now at \(dest.path)/") == true)
        guard case let .project(after) = s.app.phase else {
            Issue.record("the window did not show the moved project: \(s.app.phase)")
            return
        }
        #expect(after.id == before.id)
        #expect(after.path.hasPrefix(dest.path))
        #expect(!FileManager.default.fileExists(atPath: before.path))
        #expect(s.app.relocations == 0)
        #expect(s.model.info?.projectDir == after.path)
    }

    @Test func movingTheLibraryKeepsTheProjectOpenAndNewProjectsFollow() async throws {
        let s = try await start(project: "Library Night", library: true)
        defer { finish(s) }
        let dest = try folder(s, "Moved Library Parent")
        s.picker.folders = [dest.path]
        s.model.moveLibrary()
        let action = try #require(s.model.action)
        #expect(action.title == "Move Your OpenFrame Library")
        #expect(action.confirm.contains { $0.contains("New projects are then created in the Projects folder of the moved library.") })
        let moved = try await run(s)
        #expect(completed(moved)?.hasPrefix("Your library is now at \(dest.path)/") == true)
        let info = try #require(s.model.info)
        #expect(info.libraryDir.hasPrefix(dest.path))
        #expect(info.projectsDir == info.libraryDir + "/Projects")
        guard case let .project(p) = s.app.phase else {
            Issue.record("the window did not show the project after the library moved: \(s.app.phase)")
            return
        }
        #expect(p.path.hasPrefix(info.libraryDir))
    }

    @Test func aRefusedMoveSaysWhyAndLeavesEverything() async throws {
        let s = try await start(project: "Same Place")
        defer { finish(s) }
        let parent = try #require(s.model.info?.projectDir).split(separator: "/").dropLast().joined(separator: "/")
        s.picker.folders = ["/" + parent]
        s.model.moveProject()
        let action = try #require(s.model.action)
        await action.start()
        await action.waitUntilSettled()
        let refused = action.startError ?? { if case let .failed(t) = action.stage { t } else { nil } }()
        #expect(refused?.isEmpty == false, "moving a project to where it is is refused with the core's words")
        if case .project = s.app.phase {} else { Issue.record("a refused move closed the project") }
        #expect(s.app.relocations == 0)
    }

    // MARK: Former owners' notes

    @Test func makeThemMineTakesOverOneFormerOwnersNotes() async throws {
        // Someone with another identity (a reset of the app data) wrote private notes.
        let s = try await start(project: "Ownership")
        defer { finish(s) }
        guard case let .project(p) = s.app.phase else { return }
        await s.app.closeProject()
        let otherRoot = try Fixture.root()
        let other = Fixture.model(otherRoot)
        await other.start()
        await other.openProject(path: p.path)
        _ = try await other.core!.privateNoteCreate(CreatePrivateNoteArgs(body: "gone one"))
        _ = try await other.core!.privateNoteCreate(CreatePrivateNoteArgs(body: "gone two"))
        let gone = try #require(try await other.core!.appInfo().profile.userId)
        await other.closeProject()
        other.shutdown()
        try? FileManager.default.removeItem(at: otherRoot)
        // Opening made them a member; they left the project since (no screen removes a member yet).
        var db: OpaquePointer?
        #expect(sqlite3_open(p.path + "/project.sqlite", &db) == SQLITE_OK)
        #expect(sqlite3_exec(db, "UPDATE project_member SET deleted_at = 1 WHERE user_id = '\(gone)'", nil, nil, nil) == SQLITE_OK)
        sqlite3_close(db)

        await s.app.openProject(path: p.path)
        await s.model.load()
        let owner = try #require(s.model.orphanOwners.first)
        #expect(owner.noteCount == 2)
        s.model.adopting = owner
        await s.model.adopt(owner)
        #expect(s.model.notice == "2 private notes are yours now.")
        #expect(s.model.orphanOwners.isEmpty)
        #expect(s.model.adopting == nil)
    }

    @Test func theStoragePanelDraws() async throws {
        let s = try await start(project: "Drawn")
        defer { finish(s) }
        let themes = ThemeStore()
        let view = StoragePanelContent(model: s.model).environment(s.app).environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .dark))
        #expect(Fixture.render(view, name: "settings-storage") != nil)
    }
}

/// Copies a managed vault file under a new id, as an add that failed used to leave behind.
@MainActor
func plantOrphan(vaultDir: String) throws {
    let assets = URL(fileURLWithPath: vaultDir).appendingPathComponent("assets", isDirectory: true)
    let fm = FileManager.default
    for shard in try fm.contentsOfDirectory(at: assets, includingPropertiesForKeys: nil) {
        for file in try fm.contentsOfDirectory(at: shard, includingPropertiesForKeys: nil) {
            let ext = file.pathExtension
            let copy = shard.appendingPathComponent(UUID().uuidString.lowercased() + (ext.isEmpty ? "" : ".\(ext)"))
            try fm.copyItem(at: file, to: copy)
            return
        }
    }
    Issue.record("the vault holds no managed file to copy")
}

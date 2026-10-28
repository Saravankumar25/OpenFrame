// The plain rules of the storage screens: the Mac App Store build hides every Move, Locate and
// Change Folder (no security-scoped bookmarks yet), and the Windows wording of each result.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

/// Answers the storage panels with fixed folders and files, and records what was asked.
@MainActor
final class StubStoragePicker: StoragePicking {
    var folders: [String?] = []
    var file: String?
    var save: String?
    private(set) var asked: [String] = []

    func folder(title: String) -> String? {
        asked.append(title)
        return folders.isEmpty ? nil : folders.removeFirst()
    }

    func vaultFile(title: String) -> String? {
        asked.append(title)
        return file
    }

    func saveVault(title: String, suggestedName: String, directory: String) -> String? {
        asked.append(title)
        return save
    }
}

@Suite("Storage rules")
struct StorageRulesTests {
    @Test func theStoreBuildIsTheSandboxOrTheStoreChannel() {
        #expect(StorageRules.isStoreBuild(environment: [:], channel: nil) == false)
        #expect(StorageRules.isStoreBuild(environment: [:], channel: .directInstaller) == false)
        #expect(StorageRules.isStoreBuild(environment: [:], channel: .macAppStore))
        #expect(StorageRules.isStoreBuild(environment: ["APP_SANDBOX_CONTAINER_ID": "studio.openframe"], channel: nil))
        #expect(StorageRules.isStoreBuild(environment: ["APP_SANDBOX_CONTAINER_ID": ""], channel: .directInstaller) == false)
    }

    @Test func theStoreBuildOffersNoRelocation() {
        #expect(StorageRules.canRelocate(storeBuild: true) == false)
        #expect(StorageRules.canRelocate(storeBuild: false))
        #expect(StorageRules.storeBuildNote.contains("Mac App Store"))
    }

    @Test func pathsAreComparedAsFolders() {
        #expect(StorageRules.inside("/Users/a/OpenFrame", "/Users/a/OpenFrame/Global Idea Vault"))
        #expect(StorageRules.inside("/Users/a/OpenFrame/", "/users/a/openframe"))
        #expect(StorageRules.inside("/Users/a/OpenFrame", "/Users/a/OpenFrame2") == false)
        #expect(StorageRules.folderName("/Users/a/Films/Night.openframe/") == "Night.openframe")
        #expect(StorageRules.joinPath("/Volumes/Work", "OpenFrame") == "/Volumes/Work/OpenFrame")
    }

    @Test func aLibraryMoveNamesTheVaultOnlyWhenItIsInside() {
        func info(vault: String) -> StorageInfo {
            StorageInfo(appDataDir: "/a", libraryDir: "/l/OpenFrame", libraryFile: nil, projectsDir: "/l/OpenFrame/Projects",
                        projectsDirIsDefault: true, vaultDir: vault, vaultDirIsDefault: true, vaultExists: true, projectDir: nil)
        }
        #expect(StorageRules.libraryMoveParts(info(vault: "/l/OpenFrame/Global Idea Vault")) ==
            ["library.json", "the Global Idea Vault", "the Projects folder and every project in it", "anything else in the library folder"])
        #expect(!StorageRules.libraryMoveParts(info(vault: "/elsewhere/Global Idea Vault")).contains("the Global Idea Vault"))
    }

    @Test func locateRefusesProjectsAndEmptyFoldersAndSaysWhenInUse() {
        func v(_ kind: VaultFolderKind, dir: String? = "/v", ideas: Double = 2, active: Bool = false) -> VaultFolderInspection {
            VaultFolderInspection(kind: kind, vaultDir: dir, ideas: ideas, filesPresent: 3, filesMissing: 1, active: active)
        }
        #expect(StorageRules.locateOutcome(v(.project)) == .refuse("That folder is a project, not a vault. Choose the folder that holds your Global Idea Vault."))
        #expect(StorageRules.locateOutcome(v(.nothing, dir: nil)) == .refuse("There is no Global Idea Vault in that folder. Choose the vault folder, or the OpenFrame folder that holds it."))
        #expect(StorageRules.locateOutcome(v(.vault, active: true)) == .alreadyInUse("This vault is already the one in use. Nothing was changed."))
        #expect(StorageRules.locateOutcome(v(.parentOfVault)) == .confirm(v(.parentOfVault)))
        #expect(StorageRules.locateSummary(v(.vault)) == "Found a vault at /v with 2 ideas and 4 files. 1 file is missing from its folder.")
    }

    @Test func resultsUseTheWindowsWording() {
        #expect(StorageRules.orphanBanner(1) == "1 file in the vault folder is not in the vault.")
        #expect(StorageRules.orphanBanner(3) == "3 files in the vault folder are not in the vault.")
        #expect(StorageRules.recoverAdded(VaultRecoverResult(added: 2, failed: 0), global: true) == "Added 2 files back to the Global Idea Vault.")
        #expect(StorageRules.recoverAdded(VaultRecoverResult(added: 1, failed: 0), global: false) == "Added 1 file back to the vault")
        #expect(StorageRules.recoverFailed(VaultRecoverResult(added: 0, failed: 2)) ==
            "2 files could not be read. They may be in use by another program. Try again later.")
        #expect(StorageRules.recoverFailed(VaultRecoverResult(added: 1, failed: 0)) == nil)
        #expect(StorageRules.restoreDone(VaultRestoreReport(previousKeptAt: "/v (before restore)")) == "Restored. Your previous vault is kept at /v (before restore).")
        #expect(StorageRules.projectMoveDone(ProjectMoveReport(path: "/n/P.openframe", linkedFiles: 1, sourceLeftAt: nil), target: "x") ==
            "The project is now at /n/P.openframe. 1 linked file stays where it was.")
        #expect(StorageRules.adoptedText(1) == "1 private note is yours now.")
        #expect(StorageRules.noteCount(3) == "3 private notes")
        let date = DateComponents(calendar: .init(identifier: .gregorian), year: 2026, month: 10, day: 27, hour: 12).date!
        #expect(StorageRules.archiveFileName(.backup, date: date) == "Global Idea Vault backup 2026-10-27.ofvault")
        #expect(StorageRules.archiveFileName(.package, date: date) == "Global Idea Vault 2026-10-27.ofvault")
    }

    @Test func noWindowsWordsAreLeft() {
        let texts = [StorageRules.storeBuildNote, StorageRules.orphanBanner(2), StorageRules.locateSummary(
            VaultFolderInspection(kind: .vault, vaultDir: "/v", ideas: 1, filesPresent: 0, filesMissing: 0, active: false))]
        for t in texts {
            #expect(!t.contains("PC") && !t.contains("File Explorer") && !t.contains("Ctrl") && !t.contains("\u{2014}"))
        }
    }
}

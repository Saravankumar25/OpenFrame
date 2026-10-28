// Plain helpers for the storage screens (Windows: features/storage and features/library-setup).
// They only shape what the core answered into the Windows wording; every rule about places,
// moves and checks lives in the Rust core (docs/engineering/40-storage-locations-and-recovery.md).

import AppKit
import Foundation
import OpenFrameCore

enum StorageRules {
    // MARK: Mac App Store build

    /// True in the Mac App Store build. The App Sandbox sets `APP_SANDBOX_CONTAINER_ID` for the
    /// process; the licence channel says the same once the core answered (the internal disk image
    /// build is `directInstaller`).
    static func isStoreBuild(environment: [String: String], channel: Channel?) -> Bool {
        if let id = environment["APP_SANDBOX_CONTAINER_ID"], !id.isEmpty { return true }
        if let channel { return channel == .macAppStore }
        return false
    }

    /// Folders outside the app container need security-scoped bookmarks, which the Mac app does
    /// not keep yet (doc 40 section 7). Until it does, the store build offers no Move, Locate or
    /// Change Folder, and no new location or existing data on the setup screen.
    static func canRelocate(storeBuild: Bool) -> Bool { !storeBuild }

    /// Said where those buttons would be.
    static let storeBuildNote =
        "In the Mac App Store version, OpenFrame keeps your work in its own folder. Moving it, or using another folder, is not available yet."

    // MARK: Paths and counts

    static func folderName(_ path: String) -> String {
        path.split(separator: "/").last.map(String.init) ?? path
    }

    static func joinPath(_ parent: String, _ name: String) -> String {
        parent.hasSuffix("/") ? parent + name : parent + "/" + name
    }

    /// `path` is `root` or inside it.
    static func inside(_ root: String, _ path: String) -> Bool {
        let key: (String) -> String = { s in
            var t = s.lowercased()
            while t.count > 1 && t.hasSuffix("/") { t.removeLast() }
            return t
        }
        return key(path) == key(root) || key(path).hasPrefix(key(root) + "/")
    }

    static func plural(_ n: Double, _ one: String, _ many: String) -> String {
        let count = Int(n)
        return "\(count.formatted()) \(count == 1 ? one : many)"
    }

    static func today(_ date: Date = Date()) -> String {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: date)
    }

    // MARK: Library

    /// What a library move takes along, in the Windows order.
    static func libraryMoveParts(_ info: StorageInfo) -> [String] {
        var parts = ["library.json"]
        if inside(info.libraryDir, info.vaultDir) { parts.append("the Global Idea Vault") }
        parts.append("the Projects folder and every project in it")
        parts.append("anything else in the library folder")
        return parts
    }

    /// The inspect preview line: the core's message and whose library it is.
    static func inspectionText(_ i: LibraryInspection) -> String {
        var text = i.message
        if let owner = i.ownerName { text += " It belongs to \(owner)." }
        return text
    }

    /// A found library's line on the setup screen.
    static func foundText(_ i: LibraryInspection) -> String {
        var text = inspectionText(i)
        if let cloud = i.cloudService { text += " This folder is synced by \(cloud)." }
        return text
    }

    static func libraryMoveDone(_ r: LibraryMoveReport?, target: String) -> String {
        var text = "Your library is now at \(r?.path ?? target)."
        if let left = r?.sourceLeftAt {
            text += " The old folder \(left) was kept because it still holds files OpenFrame did not copy. Delete it yourself when you no longer need them."
        }
        return text
    }

    // MARK: Vault

    static func archiveFileName(_ kind: VaultArchiveKind, date: Date = Date()) -> String {
        kind == .backup ? "Global Idea Vault backup \(today(date)).ofvault" : "Global Idea Vault \(today(date)).ofvault"
    }

    static func archiveDone(_ r: VaultArchiveReport?) -> String {
        guard let r else { return "Saved." }
        var text = "Saved \(r.fileName) (\(PKText.formatBytes(r.bytes))): \(plural(r.ideaCount, "item", "items")) and \(plural(r.assetCount, "file", "files"))."
        if r.missingAssets > 0 {
            text += " \(plural(r.missingAssets, "file was", "files were")) already missing from the vault and could not be included."
        }
        return text
    }

    static func restoreSummary(_ v: VaultInspection) -> String {
        "\(v.fileName) is a \(v.kind == .backup ? "backup" : "package") from \(Formatting.dateTime(ms: v.createdAt)) with \(plural(v.ideaCount, "item", "items")) and \(plural(v.assetCount, "file", "files"))."
    }

    static func restoreDone(_ r: VaultRestoreReport?) -> String {
        if let kept = r?.previousKeptAt { return "Restored. Your previous vault is kept at \(kept)." }
        return "Restored."
    }

    static func vaultMoveDone(_ r: VaultMoveReport?, target: String) -> String {
        var text = "The vault is now at \(r?.path ?? target)."
        if let left = r?.sourceLeftAt {
            text += " The old folder \(left) was kept: it holds your safety backups, or files that could not be removed. Delete it yourself when you no longer need them."
        }
        return text
    }

    /// What Locate Existing Vault does with what the core found: refuse with a plain line, say it
    /// is the vault in use, or ask before connecting.
    enum LocateOutcome: Equatable {
        case refuse(String)
        case alreadyInUse(String)
        case confirm(VaultFolderInspection)
    }

    static func locateOutcome(_ v: VaultFolderInspection) -> LocateOutcome {
        if v.kind == .project {
            return .refuse("That folder is a project, not a vault. Choose the folder that holds your Global Idea Vault.")
        }
        if v.kind == .nothing || v.vaultDir == nil {
            return .refuse("There is no Global Idea Vault in that folder. Choose the vault folder, or the OpenFrame folder that holds it.")
        }
        if v.active { return .alreadyInUse("This vault is already the one in use. Nothing was changed.") }
        return .confirm(v)
    }

    static func locateSummary(_ v: VaultFolderInspection) -> String {
        let files = v.filesPresent + v.filesMissing
        var text = "Found a vault at \(v.vaultDir ?? "") with \(plural(v.ideas, "idea", "ideas")) and \(plural(files, "file", "files"))."
        if v.filesMissing > 0 { text += " \(plural(v.filesMissing, "file is", "files are")) missing from its folder." }
        return text
    }

    /// The Settings > Storage line about files in the vault folder that no item uses.
    static func orphanLine(_ o: VaultOrphanFiles) -> String {
        "\(plural(o.count, "file", "files")) in the vault folder \(Int(o.count) == 1 ? "is" : "are") not in the vault (\(PKText.formatBytes(o.bytes)))."
    }

    /// The Idea Vault banner.
    static func orphanBanner(_ count: Double) -> String {
        Int(count) == 1 ? "1 file in the vault folder is not in the vault." : "\(Int(count)) files in the vault folder are not in the vault."
    }

    /// What Add Them Back did, in two lines: what came back and what could not be read.
    static func recoverAdded(_ r: VaultRecoverResult, global: Bool) -> String? {
        guard r.added > 0 else { return nil }
        if global { return "Added \(plural(r.added, "file", "files")) back to the Global Idea Vault." }
        return Int(r.added) == 1 ? "Added 1 file back to the vault" : "Added \(Int(r.added)) files back to the vault"
    }

    static func recoverFailed(_ r: VaultRecoverResult) -> String? {
        guard r.failed > 0 else { return nil }
        return Int(r.failed) == 1
            ? "1 file could not be read. It may be in use by another program. Try again later."
            : "\(Int(r.failed)) files could not be read. They may be in use by another program. Try again later."
    }

    // MARK: Project

    static func projectMoveDone(_ r: ProjectMoveReport?, target: String) -> String {
        var text = "The project is now at \(r?.path ?? target)."
        if let r, r.linkedFiles > 0 {
            let one = Int(r.linkedFiles) == 1
            text += " \(Int(r.linkedFiles)) linked \(one ? "file stays" : "files stay") where \(one ? "it was" : "they were")."
        }
        if let left = r?.sourceLeftAt { text += " Some files could not be removed from \(left); you can delete that folder yourself." }
        return text
    }

    static func noteCount(_ n: Double) -> String {
        Int(n) == 1 ? "1 private note" : "\(Int(n)) private notes"
    }

    static func adoptedText(_ n: Double) -> String {
        Int(n) == 1 ? "1 private note is yours now." : "\(Int(n)) private notes are yours now."
    }
}

// MARK: Folder panels

/// How the storage screens ask for a folder or a vault file. The real one shows NSOpenPanel and
/// NSSavePanel (answered by the end-to-end driver through `runModal`); tests hand in a stub.
@MainActor
protocol StoragePicking {
    func folder(title: String) -> String?
    func vaultFile(title: String) -> String?
    func saveVault(title: String, suggestedName: String, directory: String) -> String?
}

@MainActor
struct PanelStoragePicker: StoragePicking {
    func folder(title: String) -> String? {
        let panel = NSOpenPanel()
        panel.title = title
        panel.message = title
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Choose"
        return panel.runModal() == .OK ? panel.url?.path : nil
    }

    func vaultFile(title: String) -> String? {
        PanelFilePicker().openFile(title: title, extensions: ["ofvault"], message: nil)?.path
    }

    func saveVault(title: String, suggestedName: String, directory: String) -> String? {
        PanelFilePicker().saveFile(
            title: title, suggestedName: suggestedName, directory: URL(fileURLWithPath: directory, isDirectory: true), message: nil)?.path
    }
}

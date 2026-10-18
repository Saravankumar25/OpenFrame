// Project Files: the screen's model. A light file cabinet: add files (copy into the project or
// link where they are), folders, rename, move, notes, open, reveal, export a copy, relink a link
// that cannot be found, and delete to Recently Deleted. Every change is a `files.*` operation;
// the core decides what is allowed.

import Foundation
import Observation
import OpenFrameCore

/// Which files the list shows.
enum FolderSel: Hashable {
    case all
    case top
    case folder(String)
}

enum FileWhere: Equatable {
    case stored, linked, unavailable
}

enum FilesDialog: Identifiable {
    case newFolder(parentId: String?)
    case renameFolder(FolderDto)
    case deleteFolder(FolderDto)

    var id: String {
        switch self {
        case let .newFolder(p): "new-folder-\(p ?? "")"
        case let .renameFolder(f): "rename-folder-\(f.id)"
        case let .deleteFolder(f): "delete-folder-\(f.id)"
        }
    }
}

enum FilesView: String, CaseIterable, Identifiable {
    case list, grid
    var id: String { rawValue }
    var title: String { self == .list ? "List" : "Grid" }
    var symbol: String { self == .list ? "list.bullet" : "square.grid.2x2" }
}

/// The text a dragged file carries, so a folder in the panel can take it.
enum FileDragPayload {
    static let prefix = "openframe-file:"
    static func encode(_ id: String) -> String { prefix + id }
    static func decode(_ text: String) -> String? { text.hasPrefix(prefix) ? String(text.dropFirst(prefix.count)) : nil }
}

@MainActor
@Observable
final class FilesModel {
    let app: AppModel
    let access: ExternalAccess

    private(set) var folders: [FolderDto] = []
    private(set) var files: [ProjectFileDto] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?

    var folder: FolderSel = .all
    var text = ""
    var selectedId: String?
    /// Counts up each time rename is asked for, so the details field takes focus.
    var renameSignal = 0
    var dialog: FilesDialog?
    /// True while files are dragged over the screen.
    var dropping = false

    @ObservationIgnored private var generation = 0

    init(app: AppModel, access: ExternalAccess = .shared) {
        self.app = app
        self.access = access
    }

    // MARK: Derived

    var current: ProjectFileDto? { files.first { $0.id == selectedId } }

    /// Where new files land: the folder that is open, else the top level.
    var targetFolderId: String? {
        if case let .folder(id) = folder { id } else { nil }
    }

    var query: String { text.trimmingCharacters(in: .whitespaces).lowercased() }

    var shown: [ProjectFileDto] {
        files.filter { f in
            switch folder {
            case .all: break
            case .top: if f.folderId != nil { return false }
            case let .folder(id): if f.folderId != id { return false }
            }
            let q = query
            if q.isEmpty { return true }
            return [f.displayName, f.asset.originalName, f.notes ?? "", PageText.fileKind(mediaType: f.asset.mediaType, name: f.asset.originalName)]
                .contains { $0.lowercased().contains(q) }
        }
    }

    /// What the list says when nothing is shown, or nil.
    var emptyMessage: String? {
        guard loaded, !files.isEmpty, shown.isEmpty else { return nil }
        return query.isEmpty
            ? "This folder is empty. Drag files onto it, or use Move to\u{2026} on a file."
            : "No files match \u{201C}\(text)\u{201D}."
    }

    func count(in sel: FolderSel) -> Int {
        switch sel {
        case .all: files.count
        case .top: files.filter { $0.folderId == nil }.count
        case let .folder(id): files.filter { $0.folderId == id }.count
        }
    }

    nonisolated static func whereOf(_ f: ProjectFileDto) -> FileWhere {
        if !f.asset.available { return .unavailable }
        return f.asset.storageMode == "external" ? .linked : .stored
    }

    func isUnavailableLink(_ f: ProjectFileDto) -> Bool {
        Self.whereOf(f) == .unavailable && f.asset.storageMode == "external"
    }

    static func folderPath(_ folders: [FolderDto], _ id: String?) -> String {
        guard let id else { return "Top level" }
        let byId = Dictionary(folders.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        var parts: [String] = []
        var cur = byId[id]
        var guardCount = 0
        while let c = cur, guardCount < 32 {
            parts.insert(c.name, at: 0)
            cur = c.parentId.flatMap { byId[$0] }
            guardCount += 1
        }
        return parts.isEmpty ? "Top level" : parts.joined(separator: " / ")
    }

    func folderPath(_ id: String?) -> String { Self.folderPath(folders, id) }

    /// Depth-first order with the depth of each folder, for the panel and pickers.
    var folderTree: [(folder: FolderDto, depth: Int)] {
        var out: [(FolderDto, Int)] = []
        let ids = Set(folders.map(\.id))
        func walk(_ parent: String?, _ depth: Int) {
            for f in folders where (f.parentId.flatMap { ids.contains($0) ? $0 : nil }) == parent {
                out.append((f, depth))
                if depth < 8 { walk(f.id, depth + 1) }
            }
        }
        walk(nil, 0)
        return out.map { (folder: $0.0, depth: $0.1) }
    }

    /// "the top level" or "“Contracts”", for the move message.
    func folderName(_ id: String?) -> String { id.map { "\u{201C}\(folderPath($0))\u{201D}" } ?? "the top level" }

    // MARK: Loading

    func reload() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        do {
            let listing = try await core.filesList()
            guard mine == generation else { return }
            folders = listing.folders
            files = listing.files
            loaded = true
            loadFailure = nil
            if let id = selectedId, !listing.files.contains(where: { $0.id == id }) { selectedId = nil }
            if case let .folder(id) = folder, !listing.folders.contains(where: { $0.id == id }) { folder = .all }
        } catch {
            guard mine == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    func apply(_ intent: PageIntent) {
        if let id = intent.fileId { selectedId = id }
    }

    func select(_ id: String?) { selectedId = id }

    func askRename(_ f: ProjectFileDto) {
        selectedId = f.id
        renameSignal += 1
    }

    // MARK: Adding

    private func run<T>(_ title: String, _ body: (Core) async throws -> T) async -> T? {
        guard let core = app.core else { return nil }
        do {
            return try await body(core)
        } catch {
            app.show(title, error)
            return nil
        }
    }

    /// Adds files by path. "copy" stores a copy inside the project; "link" keeps the file where it
    /// is. The caller holds any security-scoped access while this runs.
    @discardableResult
    func addFiles(_ paths: [String], mode: String, folderId: String?) async -> [ProjectFileDto]? {
        guard !paths.isEmpty else { return nil }
        guard let added = await run("Could not add the files", { core in
            try await core.filesAdd(AddFilesArgs(paths: paths, folderId: folderId, mode: mode))
        }) else { return nil }
        app.pageToast(added.count == 1 ? "Added \u{201C}\(added[0].displayName)\u{201D}" : "Added \(added.count) files")
        await reload()
        return added
    }

    /// Adds files the person picked or dropped, holding access to them while the core reads them,
    /// and remembering a bookmark for each linked one so it can be opened later in the sandbox.
    @discardableResult
    func addPicked(_ urls: [URL], mode: String) async -> [ProjectFileDto]? {
        guard !urls.isEmpty else { return nil }
        let held = urls.filter { $0.startAccessingSecurityScopedResource() }
        defer { held.forEach { $0.stopAccessingSecurityScopedResource() } }
        if mode == "link" { urls.forEach { access.remember($0) } }
        return await addFiles(urls.map(\.path), mode: mode, folderId: targetFolderId)
    }

    // MARK: Actions on a file

    /// Opens the file with its default app.
    func open(_ f: ProjectFileDto) async {
        guard let core = app.core else { return }
        do {
            try await access.holding(path: f.asset.path) { try await core.openAsset(f.asset.id) }
        } catch {
            app.show("Could not open the file", error)
        }
    }

    /// Shows the file in Finder.
    func reveal(_ f: ProjectFileDto) async {
        guard let core = app.core else { return }
        do {
            try await access.holding(path: f.asset.path) { try await core.openAsset(f.asset.id, reveal: true) }
        } catch {
            app.show("Could not show the file in Finder", error)
        }
    }

    /// Saves a copy where the person chose; the project is unchanged.
    func exportCopy(_ f: ProjectFileDto, to destination: String) async {
        guard let written = await run("Could not save a copy", { core in
            try await self.access.holding(path: f.asset.path) {
                try await core.filesExportCopy(ExportCopyArgs(id: f.id, destPath: destination))
            }
        }) else { return }
        ToastCenter.shared.show("Saved a copy of \u{201C}\(f.displayName)\u{201D}.", actionLabel: "Show") { [weak self] in
            try? await self?.app.core?.revealLocation("exported", path: written)
        }
    }

    /// Points a linked file at its new place; it keeps its identity, notes and folder.
    @discardableResult
    func relink(_ f: ProjectFileDto, url: URL) async -> Bool {
        let held = url.startAccessingSecurityScopedResource()
        defer { if held { url.stopAccessingSecurityScopedResource() } }
        access.remember(url)
        guard await run("Could not relink the file", { core in try await core.filesRelink(RelinkArgs(id: f.id, path: url.path)) }) != nil else { return false }
        app.pageToast("Relinked \u{201C}\(f.displayName)\u{201D}")
        await reload()
        return true
    }

    @discardableResult
    func delete(_ f: ProjectFileDto) async -> Bool {
        guard await run("Could not delete the file", { core in try await core.filesDelete(IdArgs(id: f.id)) }) != nil else { return false }
        app.pageToast("Moved \u{201C}\(f.displayName)\u{201D} to Recently Deleted")
        if selectedId == f.id { selectedId = nil }
        await reload()
        return true
    }

    func move(_ f: ProjectFileDto, to folderId: String?) async {
        guard f.folderId != folderId else { return }
        guard await run("Could not move the file", { core in try await core.filesMove(MoveArgs(id: f.id, folderId: folderId)) }) != nil else { return }
        app.pageToast("Moved \u{201C}\(f.displayName)\u{201D} to \(folderName(folderId))")
        await reload()
    }

    /// A drag of a file onto a folder in the panel. Returns true when it was taken.
    func drop(payloads: [String], onto folderId: String?) async -> Bool {
        let ids = payloads.compactMap(FileDragPayload.decode)
        guard !ids.isEmpty else { return false }
        for id in ids {
            if let f = files.first(where: { $0.id == id }) { await move(f, to: folderId) }
        }
        return true
    }

    func rename(_ f: ProjectFileDto, to name: String) async -> Bool {
        let clean = name.trimmingCharacters(in: .whitespaces)
        guard !clean.isEmpty, clean != f.displayName else { return false }
        guard await run("Could not rename the file", { core in
            try await core.filesRename(RenameArgs(id: f.id, name: clean, expectedRev: f.rev))
        }) != nil else { return false }
        app.pageToast("Renamed file to \u{201C}\(clean)\u{201D}")
        await reload()
        return true
    }

    func saveNotes(_ f: ProjectFileDto, notes: String) async -> Bool {
        guard await run("Could not save the notes", { core in
            try await core.filesUpdateNotes(NotesArgs(id: f.id, notes: notes.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? nil : notes))
        }) != nil else { return false }
        await reload()
        return true
    }

    // MARK: Folders

    @discardableResult
    func createFolder(name: String, parentId: String?) async -> FolderDto? {
        guard let f = await run("Could not create the folder", { core in
            try await core.filesCreateFolder(FolderArgs(name: name.trimmingCharacters(in: .whitespaces), parentId: parentId))
        }) else { return nil }
        app.pageToast("Created folder \u{201C}\(f.name)\u{201D}")
        await reload()
        folder = .folder(f.id)
        return f
    }

    func renameFolder(_ f: FolderDto, to name: String) async -> Bool {
        let clean = name.trimmingCharacters(in: .whitespaces)
        guard await run("Could not rename the folder", { core in try await core.filesRenameFolder(RenameArgs(id: f.id, name: clean, expectedRev: f.rev)) }) != nil else { return false }
        app.pageToast("Renamed folder to \u{201C}\(clean)\u{201D}")
        await reload()
        return true
    }

    func deleteFolder(_ f: FolderDto) async {
        guard await run("Could not delete the folder", { core in try await core.filesDeleteFolder(IdArgs(id: f.id)) }) != nil else { return }
        if folder == .folder(f.id) { folder = .all }
        app.pageToast("Deleted folder \u{201C}\(f.name)\u{201D}")
        await reload()
    }
}

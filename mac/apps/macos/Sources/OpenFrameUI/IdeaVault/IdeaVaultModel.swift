// The Idea Vault screen's model. It holds what the screen shows (the shown view, the items, the
// selection, the open item, the notice line) and sends every change to a `vault.*` operation. The
// core decides what is allowed; nothing here validates, classifies or stores. Both the project
// vault and the Global Idea Vault use it: `scope` is the `store` argument of every operation.

import Foundation
import Observation
import OpenFrameCore

/// A line at the bottom of the screen that says what just happened. It may offer Undo for the
/// store it changed, and one follow-up action.
struct VaultNotice: Identifiable, Equatable {
    enum Action: Equatable { case openStory }
    let id = UUID()
    let text: String
    var undoStore: StoreSel?
    var action: Action?
}

/// A sheet the screen asks for.
enum VaultDialog: Identifiable {
    case url
    case record
    case quick
    case send(VaultItemDto)
    case copy([VaultItemDto])
    case move([VaultItemDto])
    case collection([VaultItemDto])
    case tag([VaultItemDto])
    case result(VaultAddFilesResult)
    case newCollection
    case newFolder(parentId: String?)
    case renameFolder(VaultFolderDto)
    case renameCollection(VaultCollectionDto)

    var id: String {
        switch self {
        case .url: "url"
        case .record: "record"
        case .quick: "quick"
        case let .send(i): "send-\(i.id)"
        case let .copy(xs): "copy-\(xs.map(\.id).joined(separator: ","))"
        case let .move(xs): "move-\(xs.map(\.id).joined(separator: ","))"
        case let .collection(xs): "collection-\(xs.map(\.id).joined(separator: ","))"
        case let .tag(xs): "tag-\(xs.map(\.id).joined(separator: ","))"
        case .result: "result"
        case .newCollection: "new-collection"
        case let .newFolder(p): "new-folder-\(p ?? "")"
        case let .renameFolder(f): "rename-folder-\(f.id)"
        case let .renameCollection(c): "rename-collection-\(c.id)"
        }
    }
}

@MainActor
@Observable
final class IdeaVaultModel {
    enum Page { case vault, deleted }

    let app: AppModel
    /// The vault the person chose. With no project open the Global vault is the only one.
    var requested: StoreSel
    var page: Page = .vault
    var view: VaultView = .all
    var mode: VaultDisplayMode = .grid
    var folderCursor: String?
    /// The text being typed in the search field.
    var searchText = ""
    /// The search the list was last asked for (after the short pause).
    private(set) var search = ""

    private(set) var overview: VaultOverview?
    private(set) var items: [VaultItemDto] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    private(set) var trash: [VaultTrashRow] = []
    private(set) var trashLoaded = false
    private(set) var storyTargets: VaultStoryTargets?

    var selection: Set<String> = []
    var anchor: String?
    var openId: String?
    var focusTitle = false
    var dialog: VaultDialog?
    var notice: VaultNotice?
    /// Number of files being dragged over the screen, for the drop hint.
    var dropCount: Int?
    private(set) var busy = false

    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var trashGeneration = 0

    init(app: AppModel, store: StoreSel) {
        self.app = app
        self.requested = store
    }

    // MARK: Derived

    var hasProject: Bool { if case .project = app.phase { true } else { false } }
    var project: ProjectSummary? { if case let .project(p) = app.phase { p } else { nil } }

    /// The vault the screen shows: always Global when no project is open.
    var scope: StoreSel { hasProject ? requested : .global }
    var isGlobal: Bool { scope == .global }
    var searching: Bool { !search.trimmingCharacters(in: .whitespaces).isEmpty }

    /// What the list is asked for: the folder at the cursor in Folder mode, else the chosen view.
    var listView: VaultView { mode == .folder ? .folder(folderId: folderCursor) : view }

    /// Where new items land: the folder or collection that is open.
    var addContext: (folderId: String?, collectionId: String?) {
        switch listView {
        case let .folder(id): (id, nil)
        case let .collection(id): (nil, id)
        default: (nil, nil)
        }
    }

    var collections: [VaultCollectionDto] { overview?.collections ?? [] }
    var folders: [VaultFolderDto] { overview?.folders ?? [] }
    var total: Int { Int(overview?.total ?? 0) }
    var selectedItems: [VaultItemDto] { items.filter { selection.contains($0.id) } }

    /// The label of the copy action, or nil when there is no other vault to copy to.
    var copyLabel: String? {
        if scope == .global { return hasProject ? "Copy to Project" : nil }
        return "Copy to Global Vault"
    }

    var projectLabel: String { project.map { "\($0.title) (\($0.projectType.rawValue))" } ?? "" }

    /// Items an action applies to: the whole selection when the item is part of a larger one.
    func targets(for item: VaultItemDto) -> [VaultItemDto] {
        selection.contains(item.id) && selection.count > 1 ? selectedItems : [item]
    }

    /// The store key of the data-change counters for this screen.
    var storeKind: StoreKind { scope == .global ? .global : .project }

    // MARK: Showing

    func chooseStore(_ store: StoreSel) {
        guard store != requested else { return }
        requested = store
        view = .all
        folderCursor = nil
        selection = []
        anchor = nil
        openId = nil
        items = []
        overview = nil
        loaded = false
        trash = []
        trashLoaded = false
    }

    func show(_ v: VaultView) {
        selection = []
        page = .vault
        if case let .folder(id) = v, mode == .folder {
            folderCursor = id
        } else {
            if mode == .folder { mode = .grid }
            view = v
        }
    }

    func openFolder(_ id: String?) {
        selection = []
        if mode == .folder { folderCursor = id } else { view = id.map { .folder(folderId: $0) } ?? .all }
    }

    func setMode(_ m: VaultDisplayMode) {
        if m == .folder, mode != .folder {
            if case let .folder(id) = view { folderCursor = id } else { folderCursor = nil }
        }
        mode = m
    }

    func applySearch() {
        let next = searchText.trimmingCharacters(in: .whitespaces)
        if next != search { search = next }
    }

    func open(_ id: String, focusTitle: Bool = false) {
        openId = id
        self.focusTitle = focusTitle
    }

    func closeDrawer() {
        openId = nil
        focusTitle = false
    }

    func select(_ item: VaultItemDto, command: Bool, shift: Bool) {
        selection = VaultSelection.next(current: selection, ordered: items.map(\.id), id: item.id, anchor: anchor, command: command, shift: shift)
        if !shift { anchor = item.id }
    }

    func toggle(_ item: VaultItemDto) { select(item, command: true, shift: false) }

    // MARK: Loading

    /// Reads the overview and the shown list. A slower answer that arrives after a newer request
    /// is dropped, so a quick switch of view never shows stale items.
    func reload() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        let store = scope
        let view = listView
        let q = search
        do {
            async let ov = core.vaultOverview(VaultOverviewArgs(store: store))
            async let li = core.vaultList(VaultListArgs(store: store, view: view, search: q.isEmpty ? nil : q))
            let (o, l) = try await (ov, li)
            guard mine == generation else { return }
            overview = o
            items = l
            loaded = true
            loadFailure = nil
            let visible = Set(l.map(\.id))
            if !selection.isSubset(of: visible) { selection = selection.intersection(visible) }
        } catch {
            guard mine == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    func reloadTrash() async {
        guard let core = app.core else { return }
        trashGeneration += 1
        let mine = trashGeneration
        do {
            let rows = try await core.vaultTrashList(VaultTrashArgs(store: scope))
            guard mine == trashGeneration else { return }
            trash = rows
            trashLoaded = true
        } catch {
            guard mine == trashGeneration else { return }
            loadFailure = CoreFailure(asCore: error)
            trashLoaded = true
        }
    }

    func loadStoryTargets() async {
        guard let core = app.core else { return }
        storyTargets = try? await core.vaultStoryTargets()
    }

    // MARK: Notices

    func say(_ text: String, undo: StoreSel? = nil, action: VaultNotice.Action? = nil) {
        notice = VaultNotice(text: text, undoStore: undo, action: action)
    }

    func dismissNotice() { notice = nil }

    /// Runs a change. A failure shows the core's plain message in an alert and returns nil.
    private func run<T>(_ title: String, _ body: (Core) async throws -> T) async -> T? {
        guard let core = app.core else { return nil }
        do {
            return try await body(core)
        } catch {
            app.show(title, error)
            return nil
        }
    }

    // MARK: Adding

    /// Quick Capture: one field, saves a note. Throws the core's refusal so the sheet keeps the text.
    func quickCapture(_ text: String) async throws {
        guard let core = app.core else { throw CoreFailure(code: "internal.unexpected", message: "OpenFrame is not ready.") }
        let ctx = addContext
        _ = try await core.vaultCreate(VaultCreateArgs(store: scope, itemType: .note, body: text, folderId: ctx.folderId, collectionId: ctx.collectionId))
        say("Saved to the \(VaultText.scopeLabel(scope))", undo: scope)
        await reload()
    }

    /// Add Note or Add Quote: creates an empty item and opens it for typing.
    func createText(_ type: VaultItemType) async {
        let ctx = addContext
        let store = scope
        if let item = await run("Could not add the \(VaultText.typeLabel(type).lowercased())", { core in
            try await core.vaultCreate(VaultCreateArgs(store: store, itemType: type, folderId: ctx.folderId, collectionId: ctx.collectionId))
        }) {
            await reload()
            open(item.id)
        }
    }

    func addURL(_ url: String, note: String) async throws -> VaultItemDto {
        guard let core = app.core else { throw CoreFailure(code: "internal.unexpected", message: "OpenFrame is not ready.") }
        let ctx = addContext
        let item = try await core.vaultCreate(VaultCreateArgs(
            store: scope, itemType: .url, body: note.isEmpty ? nil : note, url: url, folderId: ctx.folderId, collectionId: ctx.collectionId))
        say("Added “\(item.displayName)”", undo: scope)
        await reload()
        return item
    }

    /// Adds files from the picker or a Finder drop. Files that fail are listed; the rest are kept.
    @discardableResult
    func addFiles(_ paths: [String], itemType: VaultItemType? = nil) async -> VaultAddFilesResult? {
        guard !paths.isEmpty else { return nil }
        let ctx = addContext
        let store = scope
        guard let r = await run("Could not add the files", { core in
            try await core.vaultAddFiles(VaultAddFilesArgs(store: store, paths: paths, folderId: ctx.folderId, collectionId: ctx.collectionId, itemType: itemType))
        }) else { return nil }
        if !r.failed.isEmpty {
            dialog = .result(r)
        } else {
            say(r.added.count == 1 ? "Added “\(r.added[0].displayName)”" : "Added \(r.added.count) items", undo: store)
        }
        await reload()
        return r
    }

    /// A finished voice recording, kept exactly as recorded.
    func addVoiceNote(data: Data, fileName: String, mediaType: String, durationMs: Double, title: String?) async throws -> VaultItemDto {
        guard let core = app.core else { throw CoreFailure(code: "internal.unexpected", message: "OpenFrame is not ready.") }
        let ctx = addContext
        let name = title?.trimmingCharacters(in: .whitespaces)
        let item = try await core.vaultIngestBytes(VaultIngestBytesArgs(
            store: scope, itemType: .voice, fileName: fileName, mediaType: mediaType, dataBase64: data.base64EncodedString(),
            durationMs: durationMs, title: (name?.isEmpty ?? true) ? nil : name, folderId: ctx.folderId, collectionId: ctx.collectionId))
        say("Added “\(item.displayName)”", undo: scope)
        await reload()
        return item
    }

    // MARK: Changing items

    func delete(_ xs: [VaultItemDto]) async {
        guard !xs.isEmpty else { return }
        let store = scope
        let ids = xs.map(\.id)
        if await run("Could not delete", { core in try await core.vaultDelete(VaultIdsArgs(store: store, ids: ids)) }) != nil {
            say("Deleted \(VaultText.itemsPhrase(xs)). You can restore from Recently Deleted.", undo: store)
            selection = []
            if let open = openId, ids.contains(open) { closeDrawer() }
            await reload()
        }
    }

    func setPinned(_ xs: [VaultItemDto], _ pinned: Bool) async {
        let store = scope
        let ids = xs.map(\.id)
        if await run("Could not change the pin", { core in try await core.vaultSetPinned(VaultSetPinnedArgs(store: store, ids: ids, pinned: pinned)) }) != nil {
            say("\(pinned ? "Pinned" : "Unpinned") \(VaultText.itemsPhrase(xs))", undo: store)
            await reload()
        }
    }

    /// The research flag is the person's own judgement; OpenFrame never sets it.
    func setResearchFlag(_ xs: [VaultItemDto], _ flag: VaultResearchFlag?) async {
        let store = scope
        let ids = xs.map(\.id)
        if await run("Could not change the research flag", { core in
            try await core.vaultSetResearchFlag(VaultSetResearchFlagArgs(store: store, ids: ids, flag: flag))
        }) != nil {
            if let label = VaultText.researchFlag(flag)?.label {
                say("Marked \(VaultText.itemsPhrase(xs)) as \(label)", undo: store)
            } else {
                say("Cleared the research flag on \(VaultText.itemsPhrase(xs))", undo: store)
            }
            await reload()
        }
    }

    func move(_ xs: [VaultItemDto], to folderId: String?) async throws {
        guard let core = app.core else { return }
        _ = try await core.vaultMoveToFolder(VaultMoveArgs(store: scope, ids: xs.map(\.id), folderId: folderId))
        let name = folders.first { $0.id == folderId }?.name
        say("Moved \(VaultText.itemsPhrase(xs)) to \(name ?? "All items (no folder)")", undo: scope)
        await reload()
    }

    /// Creates a folder (at the top level or inside another) and moves the items into it.
    func createFolderAndMove(_ xs: [VaultItemDto], name: String) async throws {
        guard let core = app.core else { return }
        let f = try await core.vaultCreateFolder(VaultCreateFolderArgs(store: scope, name: name))
        _ = try await core.vaultMoveToFolder(VaultMoveArgs(store: scope, ids: xs.map(\.id), folderId: f.id))
        say("Moved \(VaultText.itemsPhrase(xs)) to \(f.name)", undo: scope)
        await reload()
    }

    func addToCollection(_ xs: [VaultItemDto], collectionId: String) async throws {
        guard let core = app.core else { return }
        _ = try await core.vaultAddToCollection(VaultMembershipArgs(store: scope, ids: xs.map(\.id), collectionId: collectionId))
        let name = collections.first { $0.id == collectionId }?.name ?? "the collection"
        say("Added \(VaultText.itemsPhrase(xs)) to \(name)", undo: scope)
        await reload()
    }

    func createCollectionAndAdd(_ xs: [VaultItemDto], name: String) async throws {
        guard let core = app.core else { return }
        let c = try await core.vaultCreateCollection(VaultCreateCollectionArgs(store: scope, name: name))
        _ = try await core.vaultAddToCollection(VaultMembershipArgs(store: scope, ids: xs.map(\.id), collectionId: c.id))
        say("Added \(VaultText.itemsPhrase(xs)) to \(name)", undo: scope)
        await reload()
    }

    func removeFromCollection(_ item: VaultItemDto, collectionId: String) async {
        let store = scope
        _ = await run("Could not change the collection", { core in
            try await core.vaultRemoveFromCollection(VaultMembershipArgs(store: store, ids: [item.id], collectionId: collectionId))
        })
        await reload()
    }

    func addTags(_ xs: [VaultItemDto], _ tags: [String], announce: Bool = true) async throws {
        guard let core = app.core else { return }
        _ = try await core.vaultAddTags(VaultTagsArgs(store: scope, ids: xs.map(\.id), tags: tags))
        if announce { say("Tagged \(VaultText.itemsPhrase(xs))", undo: scope) }
        await reload()
    }

    func removeTag(_ item: VaultItemDto, tag: String) async {
        let store = scope
        _ = await run("Could not remove the tag", { core in
            try await core.vaultRemoveTag(VaultRemoveTagArgs(store: store, ids: [item.id], tag: tag))
        })
        await reload()
    }

    // MARK: Folders and collections

    func createFolder(name: String, parentId: String?) async throws {
        guard let core = app.core else { return }
        _ = try await core.vaultCreateFolder(VaultCreateFolderArgs(store: scope, name: name, parentId: parentId))
        await reload()
    }

    func renameFolder(_ f: VaultFolderDto, name: String) async throws {
        guard let core = app.core else { return }
        _ = try await core.vaultRenameFolder(VaultRenameFolderArgs(store: scope, id: f.id, name: name))
        await reload()
    }

    func deleteFolder(_ f: VaultFolderDto) async {
        let store = scope
        if await run("Could not delete the folder", { core in try await core.vaultDeleteFolder(VaultStoreIdArgs(store: store, id: f.id)) }) != nil {
            say("Deleted folder “\(f.name)”. Its items were kept and moved up a level.", undo: store)
            if folderCursor == f.id { folderCursor = f.parentId }
            if case let .folder(id) = view, id == f.id { view = .all }
            await reload()
        }
    }

    /// Returns the new collection so the screen can show it.
    @discardableResult
    func createCollection(name: String) async throws -> VaultCollectionDto? {
        guard let core = app.core else { return nil }
        let c = try await core.vaultCreateCollection(VaultCreateCollectionArgs(store: scope, name: name))
        await reload()
        return c
    }

    func renameCollection(_ c: VaultCollectionDto, name: String) async throws {
        guard let core = app.core else { return }
        _ = try await core.vaultRenameCollection(VaultRenameCollectionArgs(store: scope, id: c.id, name: name))
        await reload()
    }

    func deleteCollection(_ c: VaultCollectionDto) async {
        let store = scope
        if await run("Could not delete the collection", { core in try await core.vaultDeleteCollection(VaultStoreIdArgs(store: store, id: c.id)) }) != nil {
            say("Deleted collection “\(c.name)”. Its items were kept.", undo: store)
            if case let .collection(id) = view, id == c.id { view = .all }
            await reload()
        }
    }

    // MARK: Between vaults and Story

    /// Copies are independent: nothing is synced. Returns the failures, if any.
    func copy(_ xs: [VaultItemDto], collectionId: String?) async throws {
        guard let core = app.core else { return }
        let from = scope
        let to: StoreSel = from == .global ? .project : .global
        let r = try await core.vaultCopyToStore(VaultCopyArgs(from: from, to: to, ids: xs.map(\.id), collectionId: collectionId))
        if r.failed.isEmpty {
            say("Copied \(VaultText.itemsPhrase(xs)) to the \(VaultText.scopeLabel(to))", undo: to)
        } else {
            let why = r.failed.map { "\($0.name) (\($0.reason))" }.joined(separator: "; ")
            say("\(r.copied.count) copied. \(r.failed.count) could not be copied: \(why)")
        }
        selection = []
        await reload()
    }

    /// What the destination vault holds, for the collection choice in the copy sheet.
    func destinationCollections() async -> [VaultCollectionDto] {
        guard let core = app.core else { return [] }
        let to: StoreSel = scope == .global ? .project : .global
        return (try? await core.vaultOverview(VaultOverviewArgs(store: to)).collections) ?? []
    }

    /// Creates a copy in Story; the Vault original stays.
    func sendToStory(_ item: VaultItemDto, target: VaultStoryTarget, parentType: String?, parentId: String?) async throws {
        guard let core = app.core else { return }
        let r = try await core.vaultSendToStory(VaultSendToStoryArgs(store: scope, id: item.id, target: target, parentType: parentType, parentId: parentId))
        say(r.message, action: .openStory)
    }

    // MARK: Files

    func openExternally(_ item: VaultItemDto, reveal: Bool = false) async {
        guard let asset = item.asset else { return }
        let store = scope
        _ = await run("Could not open the file", { core in try await core.openAsset(asset.id, global: store == .global, reveal: reveal) })
    }

    func openLink(_ url: String) async {
        _ = await run("Could not open the link", { core in try await core.openURL(url) })
    }

    func relink(_ item: VaultItemDto, path: String) async {
        let store = scope
        if await run("Could not relink the file", { core in try await core.vaultRelink(VaultRelinkArgs(store: store, id: item.id, path: path)) }) != nil {
            say("File relinked. The item and its notes are unchanged.", undo: store)
            await reload()
        }
    }

    // MARK: Recently Deleted

    func restore(_ row: VaultTrashRow) async {
        let store = scope
        if let r = await run("Could not restore", { core in try await core.vaultRestore(VaultTrashItemArgs(store: store, id: row.deleted.id)) }) {
            say(r.message, undo: store)
            await reloadTrash()
            await reload()
        }
    }

    func purge(_ row: VaultTrashRow) async {
        let store = scope
        if await run("Could not delete", { core in try await core.vaultPurge(VaultTrashItemArgs(store: store, id: row.deleted.id)) }) != nil {
            say("Permanently deleted “\(row.deleted.title ?? "item")”.")
            await reloadTrash()
        }
    }

    // MARK: History

    /// Undo or redo in this vault's own history (the Global vault has its own).
    func undo(in store: StoreSel? = nil) async {
        let s = store ?? scope
        _ = await run("Could not undo") { core in try await core.historyUndo(StoreArgs(store: s)) }
        await reload()
    }

    func redo(in store: StoreSel? = nil) async {
        let s = store ?? scope
        _ = await run("Could not redo") { core in try await core.historyRedo(StoreArgs(store: s)) }
        await reload()
    }
}

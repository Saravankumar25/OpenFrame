// The Moodboards tab's model: the named boards, the open board with its tiles, the selection, the
// gesture preview and every change to a board. A drag or a resize is shown here while it runs and
// saved as ONE undoable core command when it ends (moodboard.move_items, moodboard.resize_item).
// Mirrors apps/desktop/src/workspaces/production/tabs/moodboards.tsx.

import AppKit
import Foundation
import Observation
import OpenFrameCore

enum MoodSheet: Identifiable {
    case newBoard
    case rename(MoodboardSummary)
    case notes(MoodboardSummary)
    case deleteBoard(MoodboardSummary)
    case note
    case link
    case vault
    case export(MoodboardSummary)

    var id: String {
        switch self {
        case .newBoard: "new-board"
        case let .rename(b): "rename-\(b.id)"
        case let .notes(b): "notes-\(b.id)"
        case let .deleteBoard(b): "delete-\(b.id)"
        case .note: "note"
        case .link: "link"
        case .vault: "vault"
        case let .export(b): "export-\(b.id)"
        }
    }
}

/// What a paste put on the board.
enum MoodPasteResult: Equatable {
    case images(Int)
    case link
    case note
    case nothing
}

@MainActor
@Observable
final class MoodboardsModel {
    unowned let hub: VisualHub

    private(set) var boards: [MoodboardSummary] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    private(set) var detail: MoodboardDto?
    private(set) var suggestions: [String] = []
    private(set) var vaultImages: [VaultImageRef] = []
    private(set) var vaultLoading = false
    var vaultStore: StoreSel = .project

    // The person's view.
    var currentId: String?
    var selected: Set<String> = []
    var inspectId: String?
    var sheet: MoodSheet?
    /// The tiles as they are while a gesture runs (nil when none).
    private(set) var preview: [MoodBox]?
    /// Where the canvas scroll is, so a new tile lands in view.
    var visibleOrigin: CGPoint = .zero

    @ObservationIgnored private var dragBefore: [MoodBox]?
    @ObservationIgnored private var resizeStart: MoodBox?
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var placed = 0

    init(hub: VisualHub) { self.hub = hub }

    // MARK: Derived

    var current: MoodboardSummary? { boards.first { $0.id == currentId } ?? boards.first }
    var items: [MoodboardItemDto] { detail?.items ?? [] }
    var boxes: [MoodBox] { items.map { MoodBox(id: $0.id, x: $0.x, y: $0.y, w: $0.w, h: $0.h) } }
    var shown: [MoodBox] { preview ?? boxes }
    var single: MoodboardItemDto? { selected.count == 1 ? items.first { selected.contains($0.id) } : nil }
    var extent: (width: Double, height: Double) { MoodGeometry.extent(shown) }

    func box(_ id: String) -> MoodBox? { shown.first { $0.id == id } }

    // MARK: Loading

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        do {
            let list = try await core.moodboardList()
            guard gen == generation else { return }
            boards = list
            loadFailure = nil
            if currentId == nil || !list.contains(where: { $0.id == currentId }) { currentId = list.first?.id }
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
            return
        }
        loaded = true
        await reloadBoard(gen: gen)
    }

    private func reloadBoard(gen: Int) async {
        guard let core = hub.core, let id = current?.id else {
            detail = nil
            return
        }
        do {
            let d = try await core.moodboardGet(MoodboardIdArgs(id: id))
            guard gen == generation else { return }
            detail = d
            preview = nil
            selected = selected.filter { sel in d.items.contains { $0.id == sel } }
            if let i = inspectId, !d.items.contains(where: { $0.id == i }) { inspectId = nil }
        } catch {
            guard gen == generation else { return }
            preview = nil
            hub.app.show("Could not open the moodboard", error)
        }
    }

    func loadSuggestions() async {
        guard let core = hub.core else { return }
        suggestions = (try? await core.moodboardSuggestions()) ?? []
    }

    func loadVaultImages() async {
        guard let core = hub.core else { return }
        vaultLoading = true
        defer { vaultLoading = false }
        do { vaultImages = try await core.visualVaultImages(VaultImagesArgs(store: vaultStore)) } catch {
            vaultImages = []
            hub.app.show("Could not read the Idea Vault", error)
        }
    }

    func pick(_ id: String) async {
        guard id != currentId else { return }
        currentId = id
        selected = []
        inspectId = nil
        preview = nil
        generation += 1
        await reloadBoard(gen: generation)
    }

    /// A search result opens the board and selects the tile.
    func focus(boardId: String, itemId: String?) async {
        currentId = boardId
        await reload()
        if let itemId, items.contains(where: { $0.id == itemId }) { selected = [itemId] }
    }

    // MARK: Boards

    func createBoard(name: String, sceneId: String?) async -> Bool {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        guard let b = await hub.perform("Could not create the moodboard", done: nil, {
            try await $0.moodboardCreate(CreateMoodboardArgs(name: name, sceneId: sceneId))
        }) else { return false }
        ToastCenter.shared.undoable("Created moodboard \u{201C}\(b.name)\u{201D}", app: hub.app)
        currentId = b.id
        selected = []
        await reload()
        return true
    }

    func rename(_ board: MoodboardSummary, to name: String) async -> Bool {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        let r = await hub.perform("Could not rename the moodboard", done: "Renamed moodboard") {
            try await $0.moodboardRename(RenameMoodboardArgs(id: board.id, name: name, expectedRev: board.rev))
        }
        return r != nil
    }

    func saveNotes(_ board: MoodboardSummary, notes: String) async -> Bool {
        let r = await hub.perform("Could not save the board notes", done: "Saved board notes") {
            try await $0.moodboardUpdateNotes(MoodboardNotesArgs(id: board.id, notes: notes))
        }
        return r != nil
    }

    func boardNotes(_ board: MoodboardSummary) async -> String {
        guard let core = hub.core else { return "" }
        return (try? await core.moodboardGet(MoodboardIdArgs(id: board.id)).notes) ?? ""
    }

    func deleteBoard(_ board: MoodboardSummary) async -> Bool {
        let r = await hub.perform("Could not delete the moodboard", done: "Deleted moodboard \u{201C}\(board.name)\u{201D}") {
            try await $0.moodboardDelete(MoodboardIdArgs(id: board.id))
        }
        if r != nil, currentId == board.id { currentId = nil }
        return r != nil
    }

    func moveBoard(_ board: MoodboardSummary, by delta: Int) async {
        guard let i = boards.firstIndex(where: { $0.id == board.id }) else { return }
        let to = i + delta
        guard to >= 0, to < boards.count else { return }
        await hub.perform("Could not reorder the moodboards", done: "Reordered moodboards") {
            try await $0.moodboardReorder(ReorderMoodboardArgs(id: board.id, index: Double(to)))
        }
    }

    // MARK: Adding

    /// Where the next tile goes: in view, stepping down a little each time so tiles do not stack.
    func dropPoint() -> CGPoint {
        placed += 1
        let step = Double((placed - 1) % 8) * 24
        return CGPoint(x: (visibleOrigin.x + 24 + step).rounded(), y: (visibleOrigin.y + 24 + step).rounded())
    }

    func addNote(text: String, isPrivate: Bool, at: CGPoint? = nil) async -> Bool {
        guard let board = current, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return false }
        let p = at ?? dropPoint()
        let r = await hub.perform("Could not add the note", done: "Added note") {
            try await $0.moodboardAddNote(AddMoodboardNoteArgs(moodboardId: board.id, text: text, isPrivate: isPrivate, x: p.x, y: p.y))
        }
        return r != nil
    }

    func addLink(url: String, title: String?, at: CGPoint? = nil) async -> Bool {
        guard let board = current, !url.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        let p = at ?? dropPoint()
        let t = title.flatMap { $0.isEmpty ? nil : $0 }
        let r = await hub.perform("Could not add the link", done: "Added link") {
            try await $0.moodboardAddLink(AddMoodboardLinkArgs(moodboardId: board.id, url: url, title: t, x: p.x, y: p.y))
        }
        return r != nil
    }

    /// Image files from a file picker or dropped from Finder. The core checks each file.
    func addImages(paths: [String], at: CGPoint? = nil) async -> Bool {
        guard let board = current, !paths.isEmpty else { return false }
        let p = at ?? dropPoint()
        let r = await hub.perform("Could not add the image", done: paths.count == 1 ? "Added image" : "Added \(paths.count) images") {
            try await $0.moodboardAddImages(AddMoodboardImagesArgs(moodboardId: board.id, paths: paths, x: p.x, y: p.y))
        }
        return r != nil
    }

    func addImageData(_ data: Data, name: String?, at: CGPoint? = nil) async -> Bool {
        guard let board = current else { return false }
        let p = at ?? dropPoint()
        let r = await hub.perform("Could not paste the image", done: "Pasted image") {
            try await $0.moodboardAddImageData(AddMoodboardImageDataArgs(
                moodboardId: board.id, dataBase64: data.base64EncodedString(), fileName: name ?? "pasted-image", x: p.x, y: p.y))
        }
        return r != nil
    }

    func addVaultImage(_ ref: VaultImageRef) async -> Bool {
        guard let board = current else { return false }
        let p = dropPoint()
        let store = vaultStore
        let r = await hub.perform("Could not add the Idea Vault image", done: "Added Idea Vault image") {
            try await $0.moodboardAddVaultImage(AddMoodboardVaultImageArgs(moodboardId: board.id, vaultItemId: ref.vaultItemId, store: store, x: p.x, y: p.y))
        }
        return r != nil
    }

    /// Files dropped from Finder: images only, the rest are named and skipped.
    func dropFiles(_ urls: [URL], at: CGPoint) async {
        let images = urls.filter { VisualText.isImage($0.path) }
        if images.count < urls.count { hub.info("Only images can be dropped on a moodboard. Other files were skipped.") }
        _ = await addImages(paths: images.map(\.path), at: at)
    }

    /// Paste (Command V) on the canvas: pictures, then a web address or text.
    @discardableResult
    func paste(from pasteboard: NSPasteboard = .general) async -> MoodPasteResult {
        if let urls = pasteboard.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL] {
            let images = urls.filter { VisualText.isImage($0.path) }
            if !images.isEmpty { return await addImages(paths: images.map(\.path)) ? .images(images.count) : .nothing }
        }
        if let png = pasteboard.data(forType: .png) ?? pasteboard.data(forType: .tiff) {
            return await addImageData(png, name: "pasted-image") ? .images(1) : .nothing
        }
        if let text = pasteboard.string(forType: .string), !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            if VisualText.looksLikeURL(text) { return await addLink(url: text.trimmingCharacters(in: .whitespacesAndNewlines), title: nil) ? .link : .nothing }
            return await addNote(text: text.trimmingCharacters(in: .whitespacesAndNewlines), isPrivate: false) ? .note : .nothing
        }
        return .nothing
    }

    // MARK: Items

    func update(_ item: MoodboardItemDto, caption: String? = nil, body: String? = nil, url: String? = nil, linkTitle: String? = nil, isPrivate: Bool? = nil) async {
        await hub.perform("Could not save the change") {
            try await $0.moodboardUpdateItem(UpdateMoodboardItemArgs(id: item.id, caption: caption, body: body, url: url, linkTitle: linkTitle, isPrivate: isPrivate))
        }
    }

    func arrange(_ id: String, placement: String) async {
        await hub.perform("Could not change the order of the tiles") {
            try await $0.moodboardArrangeItem(ArrangeMoodboardItemArgs(id: id, placement: placement))
        }
    }

    func deleteItems(_ ids: [String]) async {
        guard !ids.isEmpty else { return }
        let r = await hub.perform("Could not delete the item", done: ids.count == 1 ? "Deleted moodboard item" : "Deleted \(ids.count) moodboard items") {
            try await $0.moodboardDeleteItems(DeleteMoodboardItemsArgs(ids: ids))
        }
        if r != nil {
            selected = []
            inspectId = nil
        }
    }

    func deleteSelected() async { await deleteItems(Array(selected)) }

    func openLink(_ item: MoodboardItemDto) async {
        guard let url = item.url else { return }
        await hub.perform("Could not open the link") { try await $0.openURL(url) }
    }

    // MARK: Selection and gestures

    /// A click on a tile: plain selects it alone, Shift or Command adds or removes it.
    func click(_ id: String, additive: Bool) {
        if additive {
            if selected.contains(id) { selected.remove(id) } else { selected.insert(id) }
        } else if !selected.contains(id) || selected.count > 1 {
            selected = [id]
        }
    }

    func clearSelection() { selected = [] }

    /// The tiles follow the pointer. A tile outside the selection becomes the selection first.
    func dragChanged(id: String, dx: Double, dy: Double) {
        if dragBefore == nil {
            if !selected.contains(id) { selected = [id] }
            dragBefore = boxes
        }
        guard abs(dx) + abs(dy) >= 3 || preview != nil, let before = dragBefore else { return }
        preview = MoodGeometry.drag(before, selected: selected, dx: dx, dy: dy)
    }

    func dragEnded() async {
        defer { dragBefore = nil }
        guard let before = dragBefore, let after = preview else { return }
        await commitMoves(MoodGeometry.changedMoves(before: before, after: after))
    }

    func resizeChanged(id: String, dw: Double, dh: Double, keepAspect: Bool) {
        if resizeStart == nil { resizeStart = boxes.first { $0.id == id } }
        guard let start = resizeStart else { return }
        let nb = MoodGeometry.resize(start, dw: dw, dh: dh, keepAspect: keepAspect)
        preview = boxes.map { $0.id == id ? nb : $0 }
    }

    func resizeEnded(id: String) async {
        defer { resizeStart = nil }
        guard let start = resizeStart, let nb = preview?.first(where: { $0.id == id }) else { return }
        if nb.w != start.w || nb.h != start.h {
            let ok = await hub.perform("Could not resize the tile") {
                try await $0.moodboardResizeItem(ResizeMoodboardItemArgs(id: id, w: nb.w, h: nb.h))
            }
            if ok == nil { preview = nil }
        } else {
            preview = nil
        }
    }

    /// Arrow keys move the selection by 8 (32 with Shift): the keyboard way to place a tile.
    func nudge(dx: Double, dy: Double) async {
        guard !selected.isEmpty else { return }
        let moves = MoodGeometry.changedMoves(before: boxes, after: MoodGeometry.drag(boxes, selected: selected, dx: dx, dy: dy))
        await commitMoves(moves)
    }

    private func commitMoves(_ moves: [MoodboardItemMove]) async {
        guard !moves.isEmpty else {
            preview = nil
            return
        }
        let ok = await hub.perform("Could not move the tiles") { try await $0.moodboardMoveItems(MoveMoodboardItemsArgs(moves: moves)) }
        if ok == nil { preview = nil }
    }

    // MARK: Export

    func export(_ board: MoodboardSummary, path: String, includeTitle: Bool, includeImages: Bool, includeCaptions: Bool, includeInternalNotes: Bool) async -> ExportResult? {
        let r = await hub.perform("Could not export the moodboard") {
            try await $0.moodboardExportPdf(ExportMoodboardArgs(
                moodboardId: board.id, format: "pdf", path: path, includeTitle: includeTitle, includeImages: includeImages,
                includeCaptions: includeCaptions, includeInternalNotes: includeInternalNotes))
        }
        if let r { hub.info("Exported \(r.fileName)") }
        return r
    }
}

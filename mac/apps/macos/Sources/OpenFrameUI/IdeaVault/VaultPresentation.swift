// Pure presentation rules for the Idea Vault screen: labels, dates, sizes, folder trees, selection.
// No core calls here. Mirrors apps/desktop/src/workspaces/idea-vault/model.ts, which has the same
// unit tests on Windows. The rules about the data itself (what a folder is, what a flag means)
// live in the Rust core.

import Foundation
import OpenFrameCore

/// How the items are laid out.
enum VaultDisplayMode: String, CaseIterable, Identifiable {
    case grid, card, list, folder
    var id: String { rawValue }
    var title: String {
        switch self {
        case .grid: "Grid"
        case .card: "Card"
        case .list: "List"
        case .folder: "Folder"
        }
    }
    var symbol: String {
        switch self {
        case .grid: "square.grid.2x2"
        case .card: "rectangle.grid.1x2"
        case .list: "list.bullet"
        case .folder: "folder"
        }
    }
}

/// The tables every vault read depends on (the Windows `VAULT_TABLES`).
enum VaultTables {
    static let all = ["vault_item", "vault_item_tag", "vault_item_collection", "vault_collection", "vault_folder", "asset"]
    static let trash = ["deleted_item"] + all
    static let story = ["story_act", "story_sequence"]
}

/// The person's own verification flag on a research source, shown as words (colour only repeats it).
struct ResearchFlagInfo: Equatable {
    enum Tone { case plain, good, warn }
    let value: VaultResearchFlag
    let label: String
    let tone: Tone
}

enum VaultText {
    static let researchFlags: [ResearchFlagInfo] = [
        .init(value: .unchecked, label: "Unchecked", tone: .plain),
        .init(value: .checked, label: "Checked", tone: .good),
        .init(value: .needsExpertCheck, label: "Needs Expert Check", tone: .warn),
    ]

    static func researchFlag(_ flag: VaultResearchFlag?) -> ResearchFlagInfo? {
        guard let flag else { return nil }
        return researchFlags.first { $0.value == flag }
    }

    static func typeLabel(_ type: VaultItemType) -> String {
        switch type {
        case .note: "Note"
        case .image: "Image"
        case .url: "URL"
        case .pdf: "PDF"
        case .document: "Document"
        case .audio: "Audio"
        case .voice: "Voice note"
        case .video: "Video"
        case .sketch: "Sketch"
        case .quote: "Quote"
        case .screenshot: "Screenshot"
        case .file: "File"
        }
    }

    static func symbol(_ type: VaultItemType) -> String {
        switch type {
        case .note: "note.text"
        case .image, .screenshot: "photo"
        case .url: "link"
        case .pdf: "doc.richtext"
        case .document: "doc.text"
        case .audio: "music.note"
        case .voice: "mic"
        case .video: "film"
        case .sketch: "scribble"
        case .quote: "quote.opening"
        case .file: "doc"
        }
    }

    static func isTextType(_ t: VaultItemType) -> Bool { t == .note || t == .quote }
    static func isVisualType(_ t: VaultItemType) -> Bool { t == .image || t == .screenshot || t == .sketch }
    static func isPlayableType(_ t: VaultItemType) -> Bool { t == .audio || t == .voice || t == .video }
    static func isAudioType(_ t: VaultItemType) -> Bool { t == .audio || t == .voice }

    static func scopeLabel(_ store: StoreSel) -> String {
        store == .global ? "Global Idea Vault" : "Project Idea Vault"
    }

    /// "0:42", "12:05", "1:02:03". Nil when the length is not known.
    static func duration(ms: Double?) -> String? {
        guard let ms, ms.isFinite, ms >= 0 else { return nil }
        let total = Int((ms / 1000).rounded())
        let h = total / 3600, m = (total / 60) % 60, s = total % 60
        return h > 0 ? String(format: "%d:%02d:%02d", h, m, s) : String(format: "%d:%02d", m, s)
    }

    /// "today", "yesterday", "12 Sep", "12 Sep 2024".
    static func relativeDay(ms: Double, now: Date = Date(), calendar: Calendar = .current) -> String {
        let date = Date(timeIntervalSince1970: ms / 1000)
        let days = calendar.dateComponents([.day], from: calendar.startOfDay(for: date), to: calendar.startOfDay(for: now)).day ?? 0
        if days == 0 { return "today" }
        if days == 1 { return "yesterday" }
        let parts = calendar.dateComponents([.day, .month, .year], from: date)
        let nowYear = calendar.component(.year, from: now)
        let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        let day = parts.day ?? 1, month = months[max(0, min(11, (parts.month ?? 1) - 1))]
        return parts.year == nowYear ? "\(day) \(month)" : "\(day) \(month) \(parts.year ?? nowYear)"
    }

    static func bytes(_ n: Double?) -> String? {
        guard let n else { return nil }
        return ByteCountFormatter.string(fromByteCount: Int64(n), countStyle: .file)
    }

    /// Short extension label for file cards ("PDF", "DOCX").
    static func extLabel(_ name: String?) -> String {
        let ext = ((name ?? "") as NSString).pathExtension
        return (1...5).contains(ext.count) ? ext.uppercased() : "FILE"
    }

    /// The host and path of a link for a card line ("example.com/story").
    static func host(of url: String?) -> String {
        guard let url, !url.isEmpty else { return "" }
        guard let range = url.range(of: "://") else { return url }
        let rest = String(url[range.upperBound...])
        let hostEnd = rest.firstIndex(where: { "/?#".contains($0) }) ?? rest.endIndex
        let host = String(rest[..<hostEnd])
        var path = String(rest[hostEnd...])
        if path == "/" { path = "" }
        return String((host + path).prefix(80))
    }

    /// Card meta line: "Voice note · 0:42 · today", "Image · Visual References".
    static func metaLine(_ item: VaultItemDto, collections: [VaultCollectionDto], now: Date = Date()) -> String {
        var parts = [typeLabel(item.itemType)]
        if isPlayableType(item.itemType), let d = duration(ms: item.asset?.durationMs) { parts.append(d) }
        if let name = item.collectionIds.lazy.compactMap({ id in collections.first { $0.id == id }?.name }).first {
            parts.append(name)
        } else if let tag = item.tags.first {
            parts.append(tag)
        } else {
            parts.append(relativeDay(ms: item.createdAt, now: now))
        }
        return parts.joined(separator: " · ")
    }

    static func isUnavailable(_ item: VaultItemDto) -> Bool { item.asset.map { !$0.available } ?? false }
    static func isExternal(_ item: VaultItemDto) -> Bool { item.asset?.storageMode == "external" }

    /// "“Old platform”" for one item, "3 items" for several.
    static func itemsPhrase(_ items: [VaultItemDto]) -> String {
        items.count == 1 ? "“\(items[0].displayName)”" : "\(items.count) items"
    }

    /// Split a comma or line separated string into clean, distinct tags.
    static func parseTags(_ text: String) -> [String] {
        var out: [String] = []
        for raw in text.split(whereSeparator: { $0 == "," || $0.isNewline }) {
            var t = raw.trimmingCharacters(in: .whitespaces)
            while t.hasPrefix("#") { t.removeFirst() }
            t = t.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ")
            if !t.isEmpty, !out.contains(where: { $0.caseInsensitiveCompare(t) == .orderedSame }) { out.append(t) }
        }
        return out
    }

    static func daysLeft(_ days: Double) -> String {
        let n = max(1, Int(days.rounded()))
        return "\(n) day\(n == 1 ? "" : "s") left"
    }

    /// The text a screen reader reads for a card.
    static func accessibilityLabel(_ item: VaultItemDto) -> String {
        item.displayName + (isUnavailable(item) ? " (file unavailable)" : "")
    }
}

// MARK: Folders

enum VaultFolders {
    static func path(_ folders: [VaultFolderDto], to id: String?) -> [VaultFolderDto] {
        let byId = Dictionary(folders.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        var out: [VaultFolderDto] = []
        var seen = Set<String>()
        var cur = id.flatMap { byId[$0] }
        while let c = cur, !seen.contains(c.id) {
            seen.insert(c.id)
            out.insert(c, at: 0)
            cur = c.parentId.flatMap { byId[$0] }
        }
        return out
    }

    static func children(_ folders: [VaultFolderDto], of parent: String?) -> [VaultFolderDto] {
        folders.filter { $0.parentId == parent }
    }

    /// Depth-first flattened tree for the sidebar and pickers. A folder whose parent is missing is
    /// shown at the top level.
    static func tree(_ folders: [VaultFolderDto]) -> [(folder: VaultFolderDto, depth: Int)] {
        let ids = Set(folders.map(\.id))
        var out: [(VaultFolderDto, Int)] = []
        func walk(_ parent: String?, _ depth: Int) {
            for f in folders where (f.parentId.flatMap { ids.contains($0) ? $0 : nil }) == parent {
                out.append((f, depth))
                if depth < 12 { walk(f.id, depth + 1) }
            }
        }
        walk(nil, 0)
        return out.map { (folder: $0.0, depth: $0.1) }
    }
}

// MARK: Selection and sections

enum VaultSelection {
    /// Click, Command+click and Shift+click over the shown order.
    static func next(current: Set<String>, ordered: [String], id: String, anchor: String?, command: Bool, shift: Bool) -> Set<String> {
        if shift, let anchor, let a = ordered.firstIndex(of: anchor), let b = ordered.firstIndex(of: id) {
            var range = command ? current : []
            for x in ordered[min(a, b)...max(a, b)] { range.insert(x) }
            return range
        }
        if command {
            var s = current
            if s.contains(id) { s.remove(id) } else { s.insert(id) }
            return s
        }
        return [id]
    }
}

/// One block of the canvas: an optional header and the items under it.
struct VaultSection: Identifiable, Equatable {
    let id: String
    let header: String?
    let items: [VaultItemDto]
}

enum VaultSections {
    /// All items view splits Pinned from "Everything else"; other views are one block.
    static func build(_ items: [VaultItemDto], splitPinned: Bool, header: String? = nil) -> [VaultSection] {
        guard !items.isEmpty else { return [] }
        let pinned = splitPinned ? items.filter(\.pinned) : []
        if pinned.isEmpty { return [VaultSection(id: "all", header: header, items: items)] }
        let rest = items.filter { !$0.pinned }
        var out = [VaultSection(id: "pinned", header: "Pinned", items: pinned)]
        if !rest.isEmpty { out.append(VaultSection(id: "rest", header: "Everything else", items: rest)) }
        return out
    }
}

extension VaultView {
    /// A stable key for comparing views (tag names compare without case).
    var key: String {
        switch self {
        case .all: "all"
        case .pinned: "pinned"
        case .recentlyAdded: "recentlyAdded"
        case .recentlyModified: "recentlyModified"
        case let .folder(id): "folder:\(id ?? "")"
        case let .collection(id): "collection:\(id)"
        case let .tag(tag): "tag:\(tag.lowercased())"
        }
    }
}

// Shared pieces of the Breakdown workspace: the tables each screen reads (the same lists as
// apps/desktop/src/api/production.ts), how a breakdown category looks (colour, icon, text and
// tooltip all come from the core, as on Windows), the highlight and selection maths of the script
// pane (pure, so it is tested without a window), and the saved scene filter.

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI

// MARK: Tables

enum BDTables {
    /// Read by the production queries (Production Source, Breakdown, Catalog, Locations, Cast and Crew).
    static let production = [
        "production_source", "production_scene_state", "breakdown_element", "breakdown_dismissed",
        "catalog_item", "catalog_alias", "location", "location_photo", "cast_member", "crew_member",
        "availability_window", "screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element",
        "story_character", "asset",
    ]
    static let people = production + ["project_member"]
    static let categoryStyle = ["breakdown_category_style"]
    static let members = ["project_member"]
}

// MARK: Category names

enum BDCategory {
    /// The eleven categories, in the order of the specification.
    static let all: [BreakdownCategory] = BreakdownCategory.allCases

    /// Short singular names used in the catalog table.
    static func short(_ c: BreakdownCategory) -> String {
        switch c {
        case .cast: "Cast"
        case .extrasBackground: "Extras"
        case .locationSet: "Location"
        case .props: "Prop"
        case .wardrobe: "Wardrobe"
        case .vehicles: "Vehicle"
        case .hairMakeup: "Hair / Makeup"
        case .specialEffects: "Special Effect"
        case .vfx: "VFX"
        case .sound: "Sound"
        case .animals: "Animal"
        }
    }
}

extension CatalogStatus {
    /// The colour family of a status chip (BDChip tones).
    var tone: BDTone {
        switch self {
        case .confirmed: .green
        case .searching: .yellow
        case .shortlisted: .accent
        case .notRequired: .plain
        case .required: .blue
        }
    }
}

extension ProductionLocationStatus {
    var tone: BDTone {
        switch self {
        case .confirmed: .green
        case .shortlisted: .accent
        case .rejected: .red
        case .idea: .plain
        }
    }
}

extension ProductionSceneRef {
    var chipLabel: String { number.map { "Scene \($0)" } ?? "Removed scene" }
}

// MARK: Category look

/// How one category is drawn. Colour is never the only signal: the label, the icon and the
/// tooltip always go with it.
struct CategoryLook: Equatable {
    var category: BreakdownCategory
    var label: String
    var tooltip: String
    /// The hex colour from the core, or nil before the styles have loaded (a neutral stand-in).
    var hex: String?
    var iconKey: String

    /// SF Symbol for each icon key the core stores (categories.rs ICONS).
    static let symbols: [String: String] = [
        "user": "person", "users": "person.2", "map-pin": "mappin.and.ellipse", "box": "shippingbox",
        "shirt": "tshirt", "car": "car", "scissors": "scissors", "flame": "flame", "sparkles": "sparkles",
        "volume": "speaker.wave.2", "paw": "pawprint", "star": "star", "camera": "camera",
        "palette": "paintpalette", "wrench": "wrench", "tag": "tag",
    ]

    static func symbol(forKey key: String) -> String { symbols[key] ?? "tag" }

    var symbol: String { Self.symbol(forKey: iconKey) }
    var color: Color? { hex.flatMap(BDColor.color(hex:)) }
    var nsColor: NSColor? { hex.flatMap(BDColor.nsColor(hex:)) }
}

struct CategoryLooks: Equatable {
    var styles: [BreakdownCategoryStyleDto] = []

    func look(_ c: BreakdownCategory) -> CategoryLook {
        let s = styles.first { $0.category == c }
        return CategoryLook(category: c, label: s?.label ?? c.rawValue, tooltip: s?.tooltip ?? c.rawValue,
                            hex: s?.colorHex, iconKey: s?.icon ?? "tag")
    }
}

enum BDColor {
    static func parts(hex: String) -> (r: Double, g: Double, b: Double)? {
        var s = hex.trimmingCharacters(in: .whitespaces)
        if s.hasPrefix("#") { s.removeFirst() }
        if s.count == 3 { s = s.map { "\($0)\($0)" }.joined() }
        guard s.count == 6, let v = UInt32(s, radix: 16) else { return nil }
        return (Double((v >> 16) & 0xFF) / 255, Double((v >> 8) & 0xFF) / 255, Double(v & 0xFF) / 255)
    }

    static func color(hex: String) -> Color? {
        parts(hex: hex).map { Color(.sRGB, red: $0.r, green: $0.g, blue: $0.b, opacity: 1) }
    }

    static func nsColor(hex: String) -> NSColor? {
        parts(hex: hex).map { NSColor(srgbRed: $0.r, green: $0.g, blue: $0.b, alpha: 1) }
    }
}

// MARK: Script highlighting (pure)

struct ScriptMark: Equatable {
    enum Kind: Int { case jump = 0, tag = 1, suggestion = 2 }
    var start: Int
    var end: Int
    var kind: Kind
    /// For the tooltip, for example "Props: Pistol".
    var label: String
    var category: BreakdownCategory?
}

struct ScriptSegment: Equatable {
    var text: String
    var mark: ScriptMark?
}

/// A selection inside one script line, in UTF-16 offsets (the unit the core stores).
struct TextSelection: Equatable {
    var elementId: String
    var start: Int
    var end: Int
    var text: String
}

enum Highlight {
    /// The marks that fit the line and do not overlap, earliest first. Overlaps keep the earlier
    /// mark (the jump first, then a tag).
    static func resolve(_ marks: [ScriptMark], length: Int) -> [ScriptMark] {
        let valid = marks
            .filter { $0.start >= 0 && $0.end <= length && $0.start < $0.end }
            .sorted { $0.start != $1.start ? $0.start < $1.start : $0.kind.rawValue < $1.kind.rawValue }
        var out: [ScriptMark] = []
        var pos = 0
        for m in valid where m.start >= pos {
            out.append(m)
            pos = m.end
        }
        return out
    }

    /// Split a line into plain and marked pieces.
    static func segments(_ text: String, marks: [ScriptMark]) -> [ScriptSegment] {
        let ns = text as NSString
        var out: [ScriptSegment] = []
        var pos = 0
        for m in resolve(marks, length: ns.length) {
            if m.start > pos { out.append(ScriptSegment(text: ns.substring(with: NSRange(location: pos, length: m.start - pos)), mark: nil)) }
            out.append(ScriptSegment(text: ns.substring(with: NSRange(location: m.start, length: m.end - m.start)), mark: m))
            pos = m.end
        }
        if pos < ns.length || out.isEmpty { out.append(ScriptSegment(text: ns.substring(from: pos), mark: nil)) }
        return out
    }

    /// The selection inside one line, trimmed of surrounding white space. A selection that
    /// touches two lines (or none) is not taggable. `lines` are the character ranges of the lines
    /// in `full`.
    static func selection(lines: [(id: String, range: NSRange)], selected: NSRange, in full: NSString) -> TextSelection? {
        guard selected.length > 0 else { return nil }
        guard let line = lines.first(where: { NSLocationInRange(selected.location, $0.range) }),
              NSMaxRange(selected) <= NSMaxRange(line.range)
        else { return nil }
        var start = selected.location - line.range.location
        var end = NSMaxRange(selected) - line.range.location
        let text = full.substring(with: line.range) as NSString
        func isSpace(_ i: Int) -> Bool {
            guard let u = Unicode.Scalar(text.character(at: i)) else { return false }
            return CharacterSet.whitespacesAndNewlines.contains(u)
        }
        while start < end, isSpace(start) { start += 1 }
        while end > start, isSpace(end - 1) { end -= 1 }
        guard start < end else { return nil }
        return TextSelection(elementId: line.id, start: start, end: end, text: text.substring(with: NSRange(location: start, length: end - start)))
    }

    /// "RED FOLDER" becomes "Red Folder": the suggested catalog name for tagged words.
    static func suggestName(_ text: String) -> String {
        let collapsed = text.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ")
        let trimmed = collapsed.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        guard !trimmed.isEmpty else { return "" }
        return trimmed.split(separator: " ").map { word in
            guard let first = word.first else { return String(word) }
            return first.uppercased() + word.dropFirst().lowercased()
        }.joined(separator: " ")
    }
}

// MARK: Saved scene filter

/// The breakdown scene filter is a view preference of this Mac, never project data. A stored
/// value the core would refuse falls back to "all scenes".
struct SavedFilter: Codable, Equatable {
    var filter: BreakdownSceneFilter = .all
    var category: BreakdownCategory?

    private static let key = "openframe.breakdown.sceneFilter"

    static func read(_ defaults: UserDefaults = .standard) -> SavedFilter {
        guard let data = defaults.data(forKey: key),
              let v = try? JSONDecoder().decode(SavedFilter.self, from: data) else { return SavedFilter() }
        return v
    }

    func write(_ defaults: UserDefaults = .standard) {
        if let data = try? JSONEncoder().encode(self) { defaults.set(data, forKey: Self.key) }
    }
}

extension BreakdownSceneFilter {
    var title: String {
        switch self {
        case .all: "All scenes"
        case .unreviewed: "Only unreviewed"
        case .suggested: "Only suggested"
        case .missing: "Only missing a breakdown"
        case .needsReview: "Script changed"
        case .complete: "Complete"
        case .incomplete: "Not complete"
        }
    }

    func count(in c: BreakdownFilterCounts) -> Int {
        switch self {
        case .all: Int(c.all)
        case .unreviewed: Int(c.unreviewed)
        case .suggested: Int(c.suggested)
        case .missing: Int(c.missing)
        case .needsReview: Int(c.needsReview)
        case .complete: Int(c.complete)
        case .incomplete: Int(c.incomplete)
        }
    }
}

// MARK: Plain words

enum BDText {
    static func plural(_ n: Int, _ one: String, _ many: String? = nil) -> String {
        n == 1 ? "\(n) \(one)" : "\(n) \(many ?? one + "s")"
    }

    /// "Scenes 2, 5" or "Scene 4" turned into "scenes 2, 5" for the middle of a sentence.
    static func usedIn(_ label: String) -> String {
        if label.hasPrefix("Scenes") { return "scenes" + label.dropFirst("Scenes".count) }
        if label.hasPrefix("Scene") { return "scene" + label.dropFirst("Scene".count) }
        return label
    }

    static func quoted(_ s: String) -> String { "\u{201C}\(s)\u{201D}" }
}

extension Double {
    var asInt: Int { Int(self) }
}

// Plain helpers of the Visual Planning workspace: the tables each screen reads, the geometry of
// the moodboard canvas (the canvas shows these values only while a gesture runs; the result goes
// to the core as one undoable command), text helpers and the storyboard selection rules. They
// mirror apps/desktop/src/api/visual.ts and workspaces/production/visual/canvas.ts. Nothing
// here stores project data.

import Foundation
import OpenFrameCore

/// The project tables each Visual Planning screen reads (it reloads when one of them changes).
enum VisualTables {
    static let scene = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element", "production_source"]
    static let shot = scene + ["shot", "storyboard", "storyboard_panel", "asset"]
    static let storyboard = scene + ["storyboard", "storyboard_panel", "shot", "asset"]
    static let moodboard = ["moodboard", "moodboard_item", "asset"] + scene
    static let scenes = scene + ["shot", "storyboard", "moodboard"]
    static let characters = ["screenplay_element", "screenplay_scene"]
    static let vault = ["vault_item", "asset"]
}

enum VisualTab: String, CaseIterable, Identifiable {
    case moodboards, storyboards, shots
    var id: String { rawValue }
    var title: String {
        switch self {
        case .moodboards: "Moodboards"
        case .storyboards: "Storyboards"
        case .shots: "Shot Lists"
        }
    }
    var subtitle: String {
        switch self {
        case .moodboards: "What should this film feel like?"
        case .storyboards: "What will the audience see?"
        case .shots: "How will I capture this scene?"
        }
    }
}

// MARK: Canvas geometry

/// One tile of the moodboard canvas.
struct MoodBox: Equatable, Identifiable {
    var id: String
    var x: Double
    var y: Double
    var w: Double
    var h: Double
}

enum MoodGeometry {
    static let tileMin = 40.0
    static let tileMax = 4000.0
    static let canvasMax = 20000.0

    /// JavaScript's Math.round, so Windows and the Mac land on the same pixel.
    static func round(_ v: Double) -> Double { (v + 0.5).rounded(.down) }
    static func clamp(_ v: Double, _ lo: Double, _ hi: Double) -> Double { min(hi, max(lo, v)) }

    /// Offset every selected box by (dx, dy). The group moves together and stops at the canvas
    /// edge as a unit, so relative positions never change.
    static func drag(_ boxes: [MoodBox], selected: Set<String>, dx: Double, dy: Double) -> [MoodBox] {
        let moving = boxes.filter { selected.contains($0.id) }
        guard !moving.isEmpty else { return boxes }
        let minX = moving.map(\.x).min() ?? 0
        let minY = moving.map(\.y).min() ?? 0
        let maxX = moving.map(\.x).max() ?? 0
        let maxY = moving.map(\.y).max() ?? 0
        let ddx = clamp(round(dx), -minX, canvasMax - maxX)
        let ddy = clamp(round(dy), -minY, canvasMax - maxY)
        return boxes.map { b in
            guard selected.contains(b.id) else { return b }
            var c = b
            c.x += ddx
            c.y += ddy
            return c
        }
    }

    /// New size while dragging the resize handle; `keepAspect` keeps the ratio (images).
    static func resize(_ b: MoodBox, dw: Double, dh: Double, keepAspect: Bool) -> MoodBox {
        var w = clamp(round(b.w + dw), tileMin, tileMax)
        var h = clamp(round(b.h + dh), tileMin, tileMax)
        if keepAspect, b.w > 0 {
            let ratio = b.h / b.w
            h = clamp(round(w * ratio), tileMin, tileMax)
            w = clamp(round(h / ratio), tileMin, tileMax)
        }
        var c = b
        c.w = w
        c.h = h
        return c
    }

    /// Scrollable canvas size: the content plus room to keep arranging.
    static func extent(_ boxes: [MoodBox], pad: Double = 240) -> (width: Double, height: Double) {
        var width = 0.0
        var height = 0.0
        for b in boxes {
            width = max(width, b.x + b.w)
            height = max(height, b.y + b.h)
        }
        return (width + pad, height + pad)
    }

    /// The moves to save after a drag: only boxes whose position changed.
    static func changedMoves(before: [MoodBox], after: [MoodBox]) -> [MoodboardItemMove] {
        let prev = Dictionary(uniqueKeysWithValues: before.map { ($0.id, $0) })
        return after.compactMap { b in
            guard let p = prev[b.id], p.x != b.x || p.y != b.y else { return nil }
            return MoodboardItemMove(id: b.id, x: b.x, y: b.y)
        }
    }
}

// MARK: Text and lists

enum VisualText {
    static let imageExtensions = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"]
    static let shotSizes = ["Wide", "Full", "Medium", "Medium close", "Close", "Extreme close", "Two-shot", "OTS", "Insert", "POV", "Establishing"]
    static let movements = ["Static", "Handheld", "Pan", "Tilt", "Dolly", "Tracking", "Crane", "Steadicam", "Zoom"]
    static let angles = ["Eye level", "Low", "High", "Overhead", "Dutch", "Ground level"]
    static let shotHint = "Add only the shots you need. Camera details are optional. Shot letters are generated from order, so you never renumber by hand."

    /// "Scene 12: INT. POLICE STATION - NIGHT" (the episode title first when several screenplays are planned).
    static func sceneLabel(_ s: VisualScene) -> String {
        let head = s.heading.trimmingCharacters(in: .whitespaces)
        let h = head.isEmpty ? "Untitled scene" : head
        return "\(s.groupLabel.map { "\($0) \u{00B7} " } ?? "")Scene \(s.number): \(h)"
    }

    static func isImage(_ path: String) -> Bool {
        imageExtensions.contains((path as NSString).pathExtension.lowercased())
    }

    /// Shot letter for a zero-based order index (display only; the core makes the real labels): A to Z, AA, AB.
    static func shotLetters(_ index: Int) -> String {
        var n = index + 1
        var out = ""
        while n > 0 {
            let r = (n - 1) % 26
            out = String(UnicodeScalar(UInt8(65 + r))) + out
            n = (n - 1) / 26
        }
        return out
    }

    /// Move an element within a list (the drag preview of a reorder).
    static func move<T>(_ list: [T], from: Int, to: Int) -> [T] {
        var out = list
        guard from >= 0, from < out.count else { return out }
        let x = out.remove(at: from)
        out.insert(x, at: max(0, min(to, out.count)))
        return out
    }

    /// "3 Oct, 14:02" for the review notes (say when).
    static func formatWhen(_ ms: Double?) -> String {
        guard let ms, ms > 0 else { return "" }
        let f = DateFormatter()
        f.dateFormat = "d MMM, HH:mm"
        return f.string(from: Date(timeIntervalSince1970: ms / 1000))
    }

    /// Seconds text to milliseconds (0 clears). Nil when the text is not a valid duration.
    static func parseSeconds(_ text: String) -> Double? {
        var t = text.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ",", with: ".")
        for suffix in ["seconds", "second", "secs", "sec", "s"] where t.lowercased().hasSuffix(suffix) {
            t = String(t.dropLast(suffix.count)).trimmingCharacters(in: .whitespaces)
            break
        }
        if t.isEmpty { return 0 }
        guard let v = Double(t), v.isFinite, v >= 0, v <= 3600 else { return nil }
        return MoodGeometry.round(v * 1000)
    }

    static func formatSeconds(_ ms: Double?) -> String {
        guard let ms, ms > 0 else { return "" }
        let s = ms / 1000
        return s == s.rounded() ? String(Int(s)) : String(format: "%.1f", s)
    }

    /// Crude but safe web address detection for pasted text.
    static func looksLikeURL(_ text: String) -> Bool {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, t.rangeOfCharacter(from: .whitespacesAndNewlines) == nil else { return false }
        return t.range(of: #"^(https?://)?[\w-]+(\.[\w-]+)+(/\S*)?$"#, options: [.regularExpression, .caseInsensitive]) != nil
    }

    static func plural(_ n: Int, _ one: String, _ many: String? = nil) -> String {
        "\(n) \(n == 1 ? one : (many ?? one + "s"))"
    }
}

// MARK: Storyboard selection

/// Which scene and storyboard the Storyboards tab shows. `wanted` is "scene:<lineage>" or "board:<id>".
struct StoryboardSelection: Equatable {
    var key: String?
    var boardId: String?
    var scene: VisualScene?

    static func resolve(wanted: String?, sceneBoard: [String: String], scenes: VisualScenes?, boards: [StoryboardSummary]) -> StoryboardSelection {
        func sceneFor(_ lineage: String) -> VisualScene? { scenes?.scenes.first { $0.lineageId == lineage } }
        func pickSceneBoard(_ lineage: String) -> String? {
            let list = boards.filter { $0.sceneLineageId == lineage }
            return list.first { $0.id == sceneBoard[lineage] }?.id ?? list.first?.id
        }
        if let wanted, wanted.hasPrefix("scene:") {
            let lineage = String(wanted.dropFirst(6))
            if let sc = sceneFor(lineage) { return StoryboardSelection(key: wanted, boardId: pickSceneBoard(lineage), scene: sc) }
        }
        if let wanted, wanted.hasPrefix("board:") {
            let id = String(wanted.dropFirst(6))
            if let b = boards.first(where: { $0.id == id }) {
                if let lineage = b.sceneLineageId, !b.sceneRemoved, let sc = sceneFor(lineage) {
                    return StoryboardSelection(key: "scene:\(lineage)", boardId: b.id, scene: sc)
                }
                return StoryboardSelection(key: wanted, boardId: b.id, scene: nil)
            }
        }
        // Default: the first scene that has a storyboard, else the first scene, else the first board.
        let withBoard = scenes?.scenes.first { s in boards.contains { $0.sceneLineageId == s.lineageId } }
        if let first = withBoard ?? scenes?.scenes.first {
            return StoryboardSelection(key: "scene:\(first.lineageId)", boardId: pickSceneBoard(first.lineageId), scene: first)
        }
        if let b = boards.first { return StoryboardSelection(key: "board:\(b.id)", boardId: b.id, scene: nil) }
        return StoryboardSelection(key: nil, boardId: nil, scene: nil)
    }
}

/// The shot list's scene choice: "all", a scene lineage id, or nil when there is nothing to show.
func resolveShotSceneKey(wanted: String?, scenes: VisualScenes?, showRemoved: Bool) -> String? {
    guard let scenes else { return nil }
    if wanted == "all" { return "all" }
    if let wanted, scenes.scenes.contains(where: { $0.lineageId == wanted }) { return wanted }
    if let wanted, showRemoved, scenes.removed.contains(where: { $0.lineageId == wanted }) { return wanted }
    return scenes.scenes.first?.lineageId ?? (scenes.removed.isEmpty ? nil : "all")
}

/// The shots of the "All scenes" view, grouped by scene in order.
struct ShotGroup: Identifiable, Equatable {
    var lineage: String
    var title: String
    var needsReview: Bool
    var shots: [ShotDto]
    var id: String { lineage }

    static func group(_ shots: [ShotDto]) -> [ShotGroup] {
        var out: [ShotGroup] = []
        var index: [String: Int] = [:]
        for s in shots {
            if index[s.sceneLineageId] == nil {
                let title = s.sceneRemoved
                    ? "Removed scene: \(s.sceneHeading.isEmpty ? "untitled" : s.sceneHeading)"
                    : "Scene \(s.sceneNumber ?? ""): \(s.sceneHeading.isEmpty ? "Untitled scene" : s.sceneHeading)"
                index[s.sceneLineageId] = out.count
                out.append(ShotGroup(lineage: s.sceneLineageId, title: title, needsReview: false, shots: []))
            }
            let i = index[s.sceneLineageId]!
            out[i].needsReview = out[i].needsReview || s.needsReview
            out[i].shots.append(s)
        }
        return out
    }
}

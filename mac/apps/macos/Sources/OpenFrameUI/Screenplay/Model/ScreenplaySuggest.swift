// Smart suggestions while typing, ported from the Windows editor's `suggest.ts`.
//
// Deterministic, no AI model: the list comes from fixed screenplay terms, the live script and
// project values (Story Board characters, cast, production locations). It only ever offers; text
// changes only when the writer accepts an item with Return or a click, and only the part being
// typed is replaced. The list appears only with the caret at the end of the element.
//
// Offsets are UTF-16 units, the unit a text view uses.

import Foundation
import OpenFrameCore

public enum SuggestKind: String, Sendable, Hashable {
    case prefix, location, time, character, `extension`, parenthetical, transition
}

public enum SuggestSource: String, Sendable, Hashable {
    case standard = "Standard"
    case script = "Script"
    case storyBoard = "Story Board"
    case cast = "Cast"
    case locations = "Locations"
}

public enum SuggestCreate: String, Sendable, Hashable {
    case character, location
}

public struct SuggestContext: Hashable, Sendable {
    public var kind: SuggestKind
    /// Offsets in the element text of the part being typed (replaced on accept).
    public var from: Int
    public var to: Int
    public var query: String

    public init(kind: SuggestKind, from: Int, to: Int, query: String) {
        self.kind = kind
        self.from = from
        self.to = to
        self.query = query
    }
}

public struct SuggestItem: Hashable, Sendable {
    public var label: String
    /// Text that replaces [from, to).
    public var insert: String
    public var source: SuggestSource
    /// "Create New": keeps the typed text and adds it to the project.
    public var create: SuggestCreate?

    public init(label: String, insert: String, source: SuggestSource, create: SuggestCreate? = nil) {
        self.label = label
        self.insert = insert
        self.source = source
        self.create = create
    }
}

/// Values already in the script, most used first.
public struct ScriptValues: Hashable, Sendable {
    public var characters: [String] = []
    public var locations: [String] = []
    public var times: [String] = []
    public var prefixes: [String] = []
    public var parentheticals: [String] = []

    public init() {}
}

public struct HeadingParts: Hashable, Sendable {
    public var prefix: String
    public var location: String
    public var time: String
}

public enum ScreenplaySuggest {
    public static let headingPrefixes = ["INT.", "EXT.", "INT./EXT.", "I/E"]
    public static let times = ["DAY", "NIGHT", "MORNING", "EVENING", "LATER", "CONTINUOUS", "SAME", "MOMENTS LATER"]
    public static let extensions = ["V.O.", "O.S.", "O.C.", "CONT'D"]
    public static let transitions = ["CUT TO:", "FADE OUT.", "FADE IN:", "DISSOLVE TO:", "SMASH CUT TO:", "MATCH CUT TO:"]
    static let maxItems = 8

    /// Recognised prefixes, longest first so "INT./EXT." wins over "INT.".
    private static let prefixPattern = try! NSRegularExpression(
        pattern: #"^(INT\.\/EXT\.|EXT\.\/INT\.|INT\/EXT|I\/E\.?|INT\.|EXT\.|EST\.)\s+"#, options: [.caseInsensitive])
    private static let timeSeparator = try! NSRegularExpression(pattern: " [-\u{2013}\u{2014}] ")
    private static let cueTail = try! NSRegularExpression(pattern: #"^(.*\S)\s*\([^()]*\)\s*$"#)
    private static let blanks = try! NSRegularExpression(pattern: #"\s+"#)

    private static func full(_ s: NSString) -> NSRange { NSRange(location: 0, length: s.length) }

    private static func trimStart(_ s: String) -> String {
        String(s.drop(while: { $0.isWhitespace || $0.isNewline }))
    }

    private static func trimmed(_ s: String) -> String {
        s.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func collapse(_ s: String) -> String {
        let ns = s as NSString
        return blanks.stringByReplacingMatches(in: s, range: full(ns), withTemplate: " ")
    }

    /// The last time separator (" - ") in `text`, as a range.
    private static func lastSeparator(in text: String) -> NSRange? {
        let ns = text as NSString
        return timeSeparator.matches(in: text, range: full(ns)).last?.range
    }

    /// What the writer is typing at `offset` in an element of `type`, or nil for no list.
    public static func contextAt(_ type: ElementType, _ text: String, _ offset: Int) -> SuggestContext? {
        let ns = text as NSString
        if offset != ns.length || text.contains("\n") { return nil }
        switch type {
        case .sceneHeading:
            guard let p = prefixPattern.firstMatch(in: text, range: full(ns)) else {
                return SuggestContext(kind: .prefix, from: 0, to: offset, query: trimStart(text))
            }
            let prefixLength = p.range.length
            if let last = lastSeparator(in: text), last.location >= prefixLength {
                let from = last.location + last.length
                return SuggestContext(kind: .time, from: from, to: offset, query: ns.substring(from: from))
            }
            return SuggestContext(kind: .location, from: prefixLength, to: offset, query: ns.substring(from: prefixLength))
        case .character:
            let open = ns.range(of: "(", options: .backwards)
            if open.location != NSNotFound {
                if !ns.substring(from: open.location).contains(")") {
                    return SuggestContext(kind: .extension, from: open.location + 1, to: offset, query: ns.substring(from: open.location + 1))
                }
                return nil
            }
            return SuggestContext(kind: .character, from: 0, to: offset, query: text)
        case .parenthetical:
            let start = text.hasPrefix("(") ? 1 : 0
            return SuggestContext(kind: .parenthetical, from: start, to: offset, query: ns.substring(from: start))
        case .transition:
            return SuggestContext(kind: .transition, from: 0, to: offset, query: text)
        default:
            return nil
        }
    }

    /// "MEERA (V.O.)" becomes "MEERA".
    public static func cueName(_ raw: String) -> String {
        var s = trimmed(raw)
        if s.hasSuffix("^") { s.removeLast() }
        s = trimmed(s).uppercased()
        while true {
            let ns = s as NSString
            guard let m = cueTail.firstMatch(in: s, range: full(ns)) else { break }
            s = ns.substring(with: m.range(at: 1))
        }
        return collapse(s)
    }

    /// "INT. KITCHEN - NIGHT" becomes prefix "INT.", location "KITCHEN", time "NIGHT".
    public static func headingParts(_ heading: String) -> HeadingParts {
        let h = trimmed(heading)
        let padded = (h + " ") as NSString
        let p = prefixPattern.firstMatch(in: padded as String, range: full(padded))
        let prefix = p.map { padded.substring(with: $0.range(at: 1)).uppercased() } ?? ""
        let rest = p.map { trimmed(padded.substring(from: $0.range.length)) } ?? h
        var location = rest
        var time = ""
        if let last = lastSeparator(in: rest) {
            let r = rest as NSString
            location = r.substring(to: last.location)
            time = r.substring(from: last.location + last.length)
        }
        return HeadingParts(prefix: prefix, location: collapse(trimmed(location)).uppercased(), time: trimmed(time).uppercased())
    }

    // MARK: Script values

    private enum ValueList { case characters, locations, times, prefixes, parentheticals }

    /// What one element adds to the script values, as (list, value) pairs.
    private static func contributions(_ kind: ElementType, _ text: String) -> [(ValueList, String)] {
        switch kind {
        case .character:
            return [(.characters, cueName(text))]
        case .sceneHeading:
            let p = headingParts(text)
            return [(.prefixes, p.prefix), (.locations, p.location), (.times, p.time)]
        case .parenthetical:
            var t = trimmed(text)
            if t.hasPrefix("(") { t.removeFirst() }
            if t.hasSuffix(")") { t.removeLast() }
            return [(.parentheticals, trimmed(t))]
        default:
            return []
        }
    }

    /// Counts that remember the order in which values first appeared (the sort by use is stable).
    private struct Counter {
        var keys: [String] = []
        var counts: [String: Int] = [:]

        mutating func add(_ v: String) {
            if counts[v] == nil { keys.append(v) }
            counts[v, default: 0] += 1
        }

        mutating func remove(_ v: String) {
            guard let n = counts[v] else { return }
            if n > 1 {
                counts[v] = n - 1
            } else {
                counts[v] = nil
                keys.removeAll { $0 == v }
            }
        }

        /// Most used first; equal counts keep the order they appear in.
        var byUse: [String] {
            keys.enumerated()
                .sorted { a, b in
                    let ca = counts[a.element] ?? 0
                    let cb = counts[b.element] ?? 0
                    return ca != cb ? ca > cb : a.offset < b.offset
                }
                .map(\.element)
        }
    }

    private struct Counts {
        var characters = Counter()
        var locations = Counter()
        var times = Counter()
        var prefixes = Counter()
        var parentheticals = Counter()

        mutating func add(_ list: ValueList, _ v: String) {
            switch list {
            case .characters: characters.add(v)
            case .locations: locations.add(v)
            case .times: times.add(v)
            case .prefixes: prefixes.add(v)
            case .parentheticals: parentheticals.add(v)
            }
        }

        mutating func remove(_ list: ValueList, _ v: String) {
            switch list {
            case .characters: characters.remove(v)
            case .locations: locations.remove(v)
            case .times: times.remove(v)
            case .prefixes: prefixes.remove(v)
            case .parentheticals: parentheticals.remove(v)
            }
        }

        var values: ScriptValues {
            var v = ScriptValues()
            v.characters = characters.byUse
            v.locations = locations.byUse
            v.times = times.byUse
            v.prefixes = prefixes.byUse
            v.parentheticals = Array(parentheticals.byUse.prefix(30))
            return v
        }
    }

    private static func counts(of blocks: [EditorBlock]) -> Counts {
        var c = Counts()
        for b in blocks {
            for (list, value) in contributions(b.kind, b.text) where !value.isEmpty { c.add(list, value) }
        }
        return c
    }

    /// Values already in the script, most used first.
    public static func scriptValues(_ blocks: [EditorBlock]) -> ScriptValues {
        counts(of: blocks).values
    }

    /// Values in the script leaving out the element at `skipIndex` (the one being typed).
    public static func scriptValues(_ blocks: [EditorBlock], without skipIndex: Int) -> ScriptValues {
        var c = counts(of: blocks)
        guard blocks.indices.contains(skipIndex) else { return c.values }
        let skip = blocks[skipIndex]
        for (list, value) in contributions(skip.kind, skip.text) where !value.isEmpty { c.remove(list, value) }
        return c.values
    }

    // MARK: The list

    private static func rank(_ label: String, _ q: String) -> Int {
        let l = label.lowercased()
        if q.isEmpty { return 1 }
        if l.hasPrefix(q) { return 0 }
        let seps = CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: "./-"))
        if l.components(separatedBy: seps).contains(where: { $0.hasPrefix(q) }) { return 1 }
        return -1
    }

    /// The list for a context: typing filters, the exact typed value is not offered again, and
    /// "Create New" is added for a new character or location name.
    public static func suggestions(
        _ ctx: SuggestContext, script: ScriptValues, project: WritingSuggestionSources?, canCreate: Bool
    ) -> [SuggestItem] {
        let q = trimmed(ctx.query).lowercased()
        var pool: [(label: String, source: SuggestSource)] = []
        func add(_ labels: [String], _ source: SuggestSource) { labels.forEach { pool.append(($0, source)) } }
        switch ctx.kind {
        case .prefix:
            add(headingPrefixes, .standard)
            add(script.prefixes, .script)
        case .location:
            add(script.locations, .script)
            for l in project?.locations ?? [] { pool.append((l.name, l.source == .story ? .storyBoard : .locations)) }
        case .time:
            add(times, .standard)
            add(script.times, .script)
        case .character:
            add(script.characters, .script)
            for c in project?.characters ?? [] { pool.append((c.name, c.source == .cast ? .cast : .storyBoard)) }
        case .extension:
            add(extensions, .standard)
        case .parenthetical:
            add(script.parentheticals, .script)
        case .transition:
            add(transitions, .standard)
        }
        var seen = Set<String>()
        var ranked: [(item: SuggestItem, r: Int, i: Int)] = []
        var exact = false
        for (i, p) in pool.enumerated() {
            let key = p.label.lowercased()
            if trimmed(p.label).isEmpty || seen.contains(key) { continue }
            seen.insert(key)
            if key == q {
                exact = true
                continue
            }
            let r = rank(p.label, q)
            if r < 0 { continue }
            ranked.append((SuggestItem(label: p.label, insert: insertFor(ctx, p.label), source: p.source), r, i))
        }
        ranked.sort { $0.r != $1.r ? $0.r < $1.r : $0.i < $1.i }
        var items = ranked.prefix(maxItems).map(\.item)
        let creatable: SuggestCreate? = ctx.kind == .character ? .character : ctx.kind == .location ? .location : nil
        if let creatable, canCreate, !q.isEmpty, !exact {
            let name = trimmed(ctx.query).uppercased()
            items.append(SuggestItem(
                label: "Create New: \(name)", insert: ctx.query,
                source: creatable == .character ? .storyBoard : .locations, create: creatable))
        }
        return items
    }

    /// What accepting an item writes: a prefix leaves room for the location, a location leads into
    /// the time of day, an extension closes its bracket.
    public static func insertFor(_ ctx: SuggestContext, _ label: String) -> String {
        switch ctx.kind {
        case .prefix: return "\(label) "
        case .location: return "\(label) - "
        case .extension: return "\(label))"
        case .parenthetical: return ctx.from == 0 ? "(\(label))" : "\(label))"
        default: return label
        }
    }

    /// Applies an accepted item to the element text: only [from, to) changes.
    public static func apply(_ item: SuggestItem, to text: String, in ctx: SuggestContext) -> String {
        let ns = text as NSString
        let from = min(max(ctx.from, 0), ns.length)
        let to = min(max(ctx.to, from), ns.length)
        return ns.substring(to: from) + item.insert + ns.substring(from: to)
    }
}

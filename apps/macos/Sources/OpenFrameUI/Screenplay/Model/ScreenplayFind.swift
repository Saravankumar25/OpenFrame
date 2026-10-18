// In-script find, ported from the Windows editor's `find.ts`. Same semantics as the core's
// `screenplay.replace_all`: literal text, optional case sensitivity, optional whole-word boundaries
// where letters, digits and "_" are word characters. Scope: this scene or the whole script.

import Foundation
import OpenFrameCore

public struct ScreenplayFindOptions: Hashable, Sendable {
    public var query: String
    public var matchCase: Bool
    public var wholeWord: Bool
    /// "This scene": only the scene with this id. Nil: the whole script.
    public var sceneId: String?

    public init(query: String, matchCase: Bool = false, wholeWord: Bool = false, sceneId: String? = nil) {
        self.query = query
        self.matchCase = matchCase
        self.wholeWord = wholeWord
        self.sceneId = sceneId
    }
}

public struct ScreenplayMatch: Hashable, Sendable {
    public var block: Int
    /// UTF-16 range inside the block text.
    public var range: NSRange
}

public enum ScreenplayFind {
    private static func isWordScalar(_ s: Unicode.Scalar) -> Bool {
        s == "_" || CharacterSet.letters.contains(s) || CharacterSet.decimalDigits.contains(s)
            || s.properties.numericType != nil
    }

    /// UTF-16 ranges of matches in `text`, non-overlapping, left to right.
    public static func findInText(_ text: String, _ o: ScreenplayFindOptions) -> [NSRange] {
        if o.query.isEmpty { return [] }
        let ns = text as NSString
        var opts: NSString.CompareOptions = [.literal]
        if !o.matchCase { opts.insert(.caseInsensitive) }
        var out: [NSRange] = []
        var from = 0
        while from < ns.length {
            let r = ns.range(of: o.query, options: opts, range: NSRange(location: from, length: ns.length - from))
            if r.location == NSNotFound || r.length == 0 { break }
            from = r.location + r.length
            if o.wholeWord {
                let before = r.location > 0 ? (ns.substring(to: r.location)).unicodeScalars.last : nil
                let after = from < ns.length ? ns.substring(from: from).unicodeScalars.first : nil
                if let b = before, isWordScalar(b) { continue }
                if let a = after, isWordScalar(a) { continue }
            }
            out.append(r)
        }
        return out
    }

    /// Matches in the editor blocks, limited to one scene when the scope is "This scene", and
    /// skipping notes while they are hidden.
    public static func matches(in blocks: [EditorBlock], options: ScreenplayFindOptions?, showNotes: Bool) -> [ScreenplayMatch] {
        guard let options, !options.query.isEmpty else { return [] }
        var out: [ScreenplayMatch] = []
        var scene: String?
        for (i, b) in blocks.enumerated() {
            if b.kind == .sceneHeading { scene = b.id }
            if let want = options.sceneId, scene != want { continue }
            if !showNotes && b.kind == .note { continue }
            for r in findInText(b.text, options) { out.append(ScreenplayMatch(block: i, range: r)) }
        }
        return out
    }
}

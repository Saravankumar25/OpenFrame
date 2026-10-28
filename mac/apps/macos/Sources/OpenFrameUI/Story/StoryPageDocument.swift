// The Story page document (Windows storyDoc.ts, commits ed78cf85 and 69e17c98): the text with
// its formatting (bold, italic, underline, highlight, a point size; underline and highlight carry
// a colour from a small palette), and the mapping to what the project stores: the plain text plus
// a formatting that lists the runs of each line (Rust: openframe_import_export::prose). The runs
// of a line joined together are exactly that line, so the plain text never depends on the
// formatting; the core checks the formatting against the text on every save and read.
//
// In the editor the formatting lives in five attributes of their own (`StoryPageDocument.Key`);
// the font, underline and background the screen draws are worked out from them. Their colours
// follow the appearance the editor is pinned to, so the same text shows on the light and the
// dark page (and Undo never brings back the colours of the other page).

import AppKit
import Foundation

enum StoryPageDocument {
    /// Size of text with no size of its own, in points (the PDF uses the same).
    static let bodySize: Double = 12
    static let sizeMin: Double = 6
    static let sizeMax: Double = 96
    /// The sizes offered in the toolbar.
    static let sizes: [Double] = [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 60, 72]

    /// A palette colour: its name as stored, and its shade on the light and on the dark page.
    struct Swatch: Equatable {
        let name: String
        let light: String
        let dark: String
    }

    /// Highlight colours (the first is the default). Rust: HIGHLIGHTS in prose.rs; the dark shades
    /// are the Dark page values of story.css.
    static let highlights: [Swatch] = [
        Swatch(name: "yellow", light: "#ffe27a", dark: "#6b5a12"),
        Swatch(name: "green", light: "#b8f0a8", dark: "#2f5a2a"),
        Swatch(name: "blue", light: "#b3dcff", dark: "#1f4a6e"),
        Swatch(name: "pink", light: "#ffc4e1", dark: "#6e2a4f"),
        Swatch(name: "orange", light: "#ffcc99", dark: "#70421a"),
        Swatch(name: "purple", light: "#dcc8ff", dark: "#4a3478"),
    ]

    /// Underline colours; "ink" is the colour of the text. Rust: UNDERLINES in prose.rs.
    static let underlines: [Swatch] = [
        Swatch(name: "ink", light: "#111111", dark: "#ece9e2"),
        Swatch(name: "red", light: "#d32f2f", dark: "#ff6b6b"),
        Swatch(name: "blue", light: "#1e63d6", dark: "#6aa8ff"),
        Swatch(name: "green", light: "#2e8b3e", dark: "#6fcf7f"),
        Swatch(name: "orange", light: "#e67e22", dark: "#ffa552"),
        Swatch(name: "purple", light: "#8e44ad", dark: "#c58af0"),
    ]

    /// The sheet: A4 (210 by 297 mm) with 46 pt margins and 12 pt text, as the exported PDF.
    struct Sheet: Equatable {
        let paper: String
        let ink: String
        let rule: String
        let placeholder: String
    }
    static let lightSheet = Sheet(paper: "#ffffff", ink: "#111111", rule: "#d6d2ca", placeholder: "#8a867c")
    static let darkSheet = Sheet(paper: "#1e1e22", ink: "#ece9e2", rule: "#3a3a40", placeholder: "#8a867c")
    static func sheet(dark: Bool) -> Sheet { dark ? darkSheet : lightSheet }
    static let pageWidth: CGFloat = 210 / 25.4 * 72
    static let pageHeight: CGFloat = 297 / 25.4 * 72
    static let margin: CGFloat = 46
    static let lineHeight: CGFloat = 1.35

    /// `name` when it is in the palette, else the palette's default.
    static func inPalette(_ list: [Swatch], _ name: String?) -> String {
        list.contains { $0.name == name } ? name! : list[0].name
    }

    static func clampSize(_ pt: Double) -> Double {
        (min(sizeMax, max(sizeMin, pt)) * 2).rounded() / 2
    }

    // MARK: Stored form

    /// One stretch of a line in one style, as stored. `hc` and `uc` name the highlight and
    /// underline colour; they are left out for the defaults (yellow, ink), so older pages read the same.
    struct Run: Codable, Equatable {
        var t: String
        var b: Bool?
        var i: Bool?
        var u: Bool?
        var h: Bool?
        var s: Double?
        var hc: String?
        var uc: String?

        init(t: String, b: Bool? = nil, i: Bool? = nil, u: Bool? = nil, h: Bool? = nil, s: Double? = nil, hc: String? = nil, uc: String? = nil) {
            self.t = t; self.b = b; self.i = i; self.u = u; self.h = h; self.s = s; self.hc = hc; self.uc = uc
        }

        var styled: Bool { b == true || i == true || u == true || h == true || s != nil }

        func sameStyle(_ o: Run) -> Bool {
            (b ?? false) == (o.b ?? false) && (i ?? false) == (o.i ?? false) && (u ?? false) == (o.u ?? false)
                && (h ?? false) == (o.h ?? false) && s == o.s && hc == o.hc && uc == o.uc
        }
    }

    /// The stored formatting when it parses and matches the text (else nil: show the plain text).
    static func parseFormat(_ text: String, _ format: String?) -> [[Run]]? {
        guard let format, !format.isEmpty, let data = format.data(using: .utf8),
              let lines = try? JSONDecoder().decode([[Run]].self, from: data) else { return nil }
        let want = text.components(separatedBy: "\n")
        guard lines.count == want.count else { return nil }
        for (n, line) in lines.enumerated() where line.map(\.t).joined() != want[n] { return nil }
        return lines
    }

    // MARK: Editor form

    /// The attributes that hold the formatting in the editor.
    enum Key {
        static let bold = NSAttributedString.Key("of.story.b")
        static let italic = NSAttributedString.Key("of.story.i")
        /// The underline colour name.
        static let underline = NSAttributedString.Key("of.story.u")
        /// The highlight colour name.
        static let highlight = NSAttributedString.Key("of.story.h")
        /// The point size when it is not the body size.
        static let size = NSAttributedString.Key("of.story.s")
        static let all = [bold, italic, underline, highlight, size]
    }

    /// The formatting of one stretch, as the editor holds it.
    struct Style: Equatable {
        var bold = false
        var italic = false
        var underline: String?
        var highlight: String?
        var size: Double = StoryPageDocument.bodySize

        init(bold: Bool = false, italic: Bool = false, underline: String? = nil, highlight: String? = nil, size: Double = StoryPageDocument.bodySize) {
            self.bold = bold; self.italic = italic; self.underline = underline; self.highlight = highlight; self.size = size
        }

        init(_ r: Run) {
            bold = r.b == true
            italic = r.i == true
            underline = r.u == true ? StoryPageDocument.inPalette(StoryPageDocument.underlines, r.uc) : nil
            highlight = r.h == true ? StoryPageDocument.inPalette(StoryPageDocument.highlights, r.hc) : nil
            if let s = r.s, s != StoryPageDocument.bodySize { size = StoryPageDocument.clampSize(s) }
        }

        init(_ attrs: [NSAttributedString.Key: Any]) {
            bold = attrs[Key.bold] as? Bool == true
            italic = attrs[Key.italic] as? Bool == true
            underline = attrs[Key.underline] as? String
            highlight = attrs[Key.highlight] as? String
            size = attrs[Key.size] as? Double ?? StoryPageDocument.bodySize
        }

        func run(_ t: String) -> Run {
            var r = Run(t: t)
            if bold { r.b = true }
            if italic { r.i = true }
            if let u = underline {
                r.u = true
                if u != StoryPageDocument.underlines[0].name { r.uc = u }
            }
            if let h = highlight {
                r.h = true
                if h != StoryPageDocument.highlights[0].name { r.hc = h }
            }
            if size != StoryPageDocument.bodySize { r.s = size }
            return r
        }

        /// The formatting attributes and what the screen draws for them.
        func attributes() -> [NSAttributedString.Key: Any] {
            var a: [NSAttributedString.Key: Any] = [:]
            if bold { a[Key.bold] = true }
            if italic { a[Key.italic] = true }
            if let underline { a[Key.underline] = underline }
            if let highlight { a[Key.highlight] = highlight }
            if size != StoryPageDocument.bodySize { a[Key.size] = size }
            return a.merging(StoryPageDocument.look(self)) { _, new in new }
        }
    }

    /// The font, colour, underline, highlight and line spacing the screen draws for a style.
    static func look(_ s: Style) -> [NSAttributedString.Key: Any] {
        var a: [NSAttributedString.Key: Any] = [
            .font: font(size: s.size, bold: s.bold, italic: s.italic),
            .foregroundColor: ink,
            .paragraphStyle: paragraph,
        ]
        if let u = s.underline {
            a[.underlineStyle] = NSUnderlineStyle.single.rawValue
            // An ink underline takes the colour of the text.
            if u != underlines[0].name, let c = underlineColors[u] { a[.underlineColor] = c }
        }
        if let h = s.highlight, let c = highlightColors[h] { a[.backgroundColor] = c }
        return a
    }

    /// A colour with one shade on the light page and one on the dark page.
    static func pageColor(light: String, dark: String) -> NSColor {
        ScreenplayPageColors.dynamic(light: light, dark: dark)
    }

    nonisolated(unsafe) static let ink = pageColor(light: lightSheet.ink, dark: darkSheet.ink)
    nonisolated(unsafe) static let highlightColors = Dictionary(uniqueKeysWithValues: highlights.map { ($0.name, pageColor(light: $0.light, dark: $0.dark)) })
    nonisolated(unsafe) static let underlineColors = Dictionary(uniqueKeysWithValues: underlines.map { ($0.name, pageColor(light: $0.light, dark: $0.dark)) })

    /// Arial, as the Windows sheet and the exported PDF.
    static func font(size: Double, bold: Bool, italic: Bool) -> NSFont {
        let base = NSFont(name: "Arial", size: size) ?? NSFont(name: "Helvetica", size: size) ?? .systemFont(ofSize: size)
        var traits: NSFontDescriptor.SymbolicTraits = []
        if bold { traits.insert(.bold) }
        if italic { traits.insert(.italic) }
        guard !traits.isEmpty else { return base }
        return NSFont(descriptor: base.fontDescriptor.withSymbolicTraits(traits), size: size) ?? base
    }

    /// Line height 1.35 of the font size, as the sheet and the PDF.
    nonisolated(unsafe) static let paragraph: NSParagraphStyle = {
        let p = NSMutableParagraphStyle()
        p.lineHeightMultiple = lineHeight / 1.15
        return p
    }()

    static func color(_ hex: String) -> NSColor { ScreenplayPageColors.color(hex) }

    // MARK: Mapping

    /// The editor text of a stored page. A formatting that does not match the text is ignored.
    static func attributed(_ text: String, _ format: String?) -> NSAttributedString {
        let lines = parseFormat(text, format) ?? text.components(separatedBy: "\n").map { $0.isEmpty ? [] : [Run(t: $0)] }
        let out = NSMutableAttributedString()
        for (n, runs) in lines.enumerated() {
            if n > 0 { out.append(NSAttributedString(string: "\n", attributes: Style().attributes())) }
            for r in runs where !r.t.isEmpty {
                out.append(NSAttributedString(string: r.t, attributes: Style(r).attributes()))
            }
        }
        return out
    }

    /// What the project stores for the editor text: the plain text and the formatting (nil when
    /// nothing is formatted). Line endings become `\n`; the runs always match the text.
    static func content(of s: NSAttributedString) -> (text: String, format: String?) {
        var lines: [[Run]] = [[]]
        let ns = s.string as NSString
        var i = 0
        while i < ns.length {
            var range = NSRange()
            let attrs = s.attributes(at: i, longestEffectiveRange: &range, in: NSRange(location: i, length: ns.length - i))
            let style = Style(attrs)
            // A stretch may hold line breaks: each one starts a new line ("\r\n" and "\r" count as one).
            let piece = ns.substring(with: range).replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
            for (k, part) in piece.components(separatedBy: "\n").enumerated() {
                if k > 0 { lines.append([]) }
                guard !part.isEmpty else { continue }
                let r = style.run(part)
                if let last = lines[lines.count - 1].last, last.sameStyle(r) {
                    lines[lines.count - 1][lines[lines.count - 1].count - 1].t += part
                } else {
                    lines[lines.count - 1].append(r)
                }
            }
            i = NSMaxRange(range)
        }
        let text = lines.map { $0.map(\.t).joined() }.joined(separator: "\n")
        guard lines.contains(where: { $0.contains(where: \.styled) }),
              let data = try? encoder.encode(lines), let json = String(data: data, encoding: .utf8) else {
            return (text, nil)
        }
        return (text, json)
    }

    nonisolated(unsafe) private static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.outputFormatting = [.withoutEscapingSlashes]
        return e
    }()

    static func wordCount(_ text: String) -> Int {
        text.split(whereSeparator: { $0.isWhitespace }).count
    }
}

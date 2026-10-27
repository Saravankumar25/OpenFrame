// What a paste keeps on the Story page. Windows pastes through the ProseMirror schema of
// storyDoc.ts, so only what its parse rules read survives; this file applies the same rules:
//  - bold: <strong>, <b>, or font-weight bold or 600 to 900;
//  - italic: <em>, <i>, or font-style: italic;
//  - underline: <u> (colour from its data-color when it is in the palette, else the text
//    colour), or text-decoration: underline (the text colour);
//  - highlight: <mark> only (colour from its data-color when it is in the palette, else yellow);
//    a background colour on other text is not a highlight;
//  - size: font-size in pt or px (px times 0.75), kept between 6 and 96 points in half points,
//    and none when it is the body size;
//  - text colours and fonts are dropped; paragraphs and other blocks start a new line; runs of
//    white space (a <br> too) become one space.
// Rich text (RTF) from Mac apps goes through the same rules by way of its HTML form. A copy made
// on the Story page carries its own runs, so it pastes back exactly, as on Windows.

import AppKit
import Foundation

enum StoryPagePaste {
    /// The Story page's own copy: the runs of the copied lines (StoryPageDocument.Run).
    static let runsType = NSPasteboard.PasteboardType("studio.openframe.story-page-runs")

    /// The pasteboard's content as Story page text, or nil when it holds no text.
    static func read(_ pb: NSPasteboard) -> NSAttributedString? {
        if let json = pb.string(forType: runsType), let s = fromRuns(json) { return s }
        if let html = pb.data(forType: .html), let s = fromHTML(html) { return s }
        if let rtf = pb.data(forType: .rtf), let s = fromRTF(rtf) { return s }
        guard let text = pb.string(forType: .string) else { return nil }
        return NSAttributedString(string: clean(text))
    }

    static func clean(_ s: String) -> String {
        s.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
    }

    // MARK: The page's own runs

    static func runs(of s: NSAttributedString) -> String? {
        let c = StoryPageDocument.content(of: s)
        let lines = StoryPageDocument.parseFormat(c.text, c.format)
            ?? c.text.components(separatedBy: "\n").map { $0.isEmpty ? [] : [StoryPageDocument.Run(t: $0)] }
        guard let data = try? JSONEncoder().encode(lines) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    static func fromRuns(_ json: String) -> NSAttributedString? {
        guard let lines = try? JSONDecoder().decode([[StoryPageDocument.Run]].self, from: Data(json.utf8)) else { return nil }
        let text = lines.map { $0.map(\.t).joined() }.joined(separator: "\n")
        guard !text.isEmpty else { return nil }
        return StoryPageDocument.attributed(text, json)
    }

    // MARK: Rich text

    /// RTF as HTML (AppKit writes bold, italic and underline as tags), then the HTML rules.
    static func fromRTF(_ data: Data) -> NSAttributedString? {
        guard let s = NSAttributedString(rtf: data, documentAttributes: nil),
              let html = try? s.data(from: NSRange(location: 0, length: s.length), documentAttributes: [.documentType: NSAttributedString.DocumentType.html])
        else { return nil }
        return fromHTML(html)
    }

    static func fromHTML(_ data: Data) -> NSAttributedString? {
        guard let html = String(data: data, encoding: .utf8) ?? String(data: data, encoding: .isoLatin1) else { return nil }
        return fromHTML(html)
    }

    static func fromHTML(_ html: String) -> NSAttributedString? {
        // The HTML cleaner predates <mark> and would drop it: it travels as a marked <span>.
        let marked = html
            .replacingOccurrences(of: "<mark(?=[\\s>])", with: "<span data-of-mark=\"1\"", options: [.regularExpression, .caseInsensitive])
            .replacingOccurrences(of: "</mark\\s*>", with: "</span>", options: [.regularExpression, .caseInsensitive])
        let wrapped = marked.range(of: "<html", options: .caseInsensitive) == nil
            ? "<html><head><meta charset=\"utf-8\"></head><body>\(marked)</body></html>" : marked
        guard let doc = try? XMLDocument(xmlString: wrapped, options: [.documentTidyHTML]),
              let root = doc.rootElement() else { return nil }
        let body = (try? root.nodes(forXPath: "//body").first) ?? root
        var w = Walker()
        w.walk(body, StoryPageDocument.Style())
        let out = w.finish()
        return out.length == 0 ? nil : out
    }

    /// A size from pasted CSS ("14pt", "18.67px"), or nil (Windows cssSize).
    static func cssSize(_ v: String) -> Double? {
        let t = v.trimmingCharacters(in: .whitespaces).lowercased()
        for (unit, factor) in [("pt", 1.0), ("px", 0.75)] where t.hasSuffix(unit) {
            let num = String(t.dropLast(2))
            guard !num.isEmpty, num.allSatisfy({ $0.isNumber || $0 == "." }), num.first != ".", num.last != ".",
                  num.filter({ $0 == "." }).count <= 1, let n = Double(num) else { return nil }
            return StoryPageDocument.clampSize(n * factor)
        }
        return nil
    }

    /// The style an element gives its text, after the style it is in.
    static func style(of e: XMLElement, in outer: StoryPageDocument.Style) -> StoryPageDocument.Style {
        var s = outer
        let name = (e.name ?? "").lowercased()
        let css = cssPairs(e.attribute(forName: "style")?.stringValue ?? "")
        let dataColor = e.attribute(forName: "data-color")?.stringValue
        switch name {
        case "strong", "b": s.bold = true
        case "em", "i": s.italic = true
        case "u": s.underline = StoryPageDocument.inPalette(StoryPageDocument.underlines, dataColor)
        case "mark", "span" where e.attribute(forName: "data-of-mark") != nil: s.highlight = StoryPageDocument.inPalette(StoryPageDocument.highlights, dataColor)
        default: break
        }
        if let w = css["font-weight"], w == "bold" || ["600", "700", "800", "900"].contains(w) { s.bold = true }
        if css["font-style"] == "italic" { s.italic = true }
        if css["text-decoration"] == "underline", name != "u" { s.underline = StoryPageDocument.underlines[0].name }
        if let v = css["font-size"], let pt = cssSize(v), pt != StoryPageDocument.bodySize { s.size = pt }
        return s
    }

    private static func cssPairs(_ style: String) -> [String: String] {
        var out: [String: String] = [:]
        for part in style.split(separator: ";") {
            let kv = part.split(separator: ":", maxSplits: 1)
            guard kv.count == 2 else { continue }
            out[kv[0].trimmingCharacters(in: .whitespaces).lowercased()] = kv[1].trimmingCharacters(in: .whitespaces)
        }
        return out
    }

    static let blockTags: Set<String> = [
        "address", "article", "aside", "blockquote", "dd", "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer",
        "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hgroup", "hr", "li", "ol", "p", "pre", "section", "table",
        "tbody", "thead", "tfoot", "tr", "ul",
    ]
    static let ignoredTags: Set<String> = ["head", "noscript", "object", "script", "style", "title"]

    /// Walks the HTML into lines of styled text.
    private struct Walker {
        var lines: [[(String, StoryPageDocument.Style)]] = [[]]

        private var lineIsEmpty: Bool { lines[lines.count - 1].allSatisfy { $0.0.isEmpty } }
        private var endsWithSpace: Bool { lines[lines.count - 1].last.map { $0.0.hasSuffix(" ") } ?? true }

        mutating func breakLine() { if !lineIsEmpty { lines.append([]) } }

        mutating func text(_ raw: String, _ style: StoryPageDocument.Style) {
            var t = raw.replacingOccurrences(of: "[ \\t\\r\\n\\f]+", with: " ", options: .regularExpression)
            if t.hasPrefix(" "), endsWithSpace { t.removeFirst() }
            if !t.isEmpty { lines[lines.count - 1].append((t, style)) }
        }

        mutating func walk(_ node: XMLNode, _ style: StoryPageDocument.Style) {
            for child in node.children ?? [] {
                switch child.kind {
                case .text:
                    text(child.stringValue ?? "", style)
                case .element:
                    guard let e = child as? XMLElement else { continue }
                    let name = (e.name ?? "").lowercased()
                    if StoryPagePaste.ignoredTags.contains(name) { continue }
                    if name == "br" { text(" ", style); continue }
                    let block = StoryPagePaste.blockTags.contains(name)
                    if block { breakLine() }
                    walk(e, StoryPagePaste.style(of: e, in: style))
                    if block { breakLine() }
                default:
                    continue
                }
            }
        }

        func finish() -> NSAttributedString {
            var rows = lines
            while let last = rows.last, last.allSatisfy({ $0.0.isEmpty }), rows.count > 1 { rows.removeLast() }
            let out = NSMutableAttributedString()
            for (n, row) in rows.enumerated() {
                if n > 0 { out.append(NSAttributedString(string: "\n", attributes: StoryPageDocument.Style().attributes())) }
                var parts = row
                // A line never ends in the space a collapsed break left.
                if let i = parts.indices.last, parts[i].0.hasSuffix(" ") { parts[i].0.removeLast() }
                for (t, s) in parts where !t.isEmpty { out.append(NSAttributedString(string: t, attributes: s.attributes())) }
            }
            return out
        }
    }
}

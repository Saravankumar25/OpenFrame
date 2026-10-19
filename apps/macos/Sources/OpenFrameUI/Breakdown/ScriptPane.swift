// The script reading pane (FSD 96: "A script reading pane is available while tagging"). Read-only:
// the screenplay is never edited from Breakdown. It is an NSTextView so that selecting words works
// natively (mouse, Shift+arrows, double click). A selection inside one line offers "Tag as…" in the
// context menu with the eleven categories, and Shift+Command+T opens the Add Element sheet with the
// words filled in. Offsets are UTF-16 units, the unit the core stores.

import AppKit
import SwiftUI
import OpenFrameCore

// MARK: The text, built from the scene

struct ScriptColors: Equatable {
    var ink: [Double]
    var muted: [Double]
    var jump: [Double]
    var background: [Double]

    var inkColor: NSColor { Self.ns(ink) }
    var mutedColor: NSColor { Self.ns(muted) }
    var jumpColor: NSColor { Self.ns(jump) }
    var backgroundColor: NSColor { Self.ns(background) }

    static func ns(_ c: [Double]) -> NSColor { NSColor(srgbRed: c[0], green: c[1], blue: c[2], alpha: c[3]) }

    init(theme: ResolvedTheme) {
        func v(_ t: String) -> [Double] { let c = theme.rgba(t); return [c.r, c.g, c.b, c.a] }
        ink = v("ink"); muted = v("muted"); jump = v("yellow"); background = v("panel")
    }
}

struct ScriptContent: Equatable {
    var sceneId: String
    var number: String?
    var heading: String
    var lines: [BreakdownScriptLine]
    var marks: [String: [ScriptMark]]
    /// Category colour per mark, as hex.
    var colors: [String: String]
    var palette: ScriptColors
}

/// The attributed text of a scene and where each line sits in it.
struct ScriptDocument {
    var text: NSAttributedString
    var lineRanges: [(id: String, range: NSRange)]
    /// Where each mark was drawn, to scroll to a jump.
    var markRanges: [(elementId: String, mark: ScriptMark, range: NSRange)]

    static func displayLines(_ c: ScriptContent) -> [BreakdownScriptLine] {
        // The first line is dropped when it repeats the heading, as the heading is shown above.
        c.lines.enumerated().filter { !($0.offset == 0 && $0.element.elementType == "scene_heading" && $0.element.text.trimmingCharacters(in: .whitespaces) == c.heading.trimmingCharacters(in: .whitespaces)) }.map(\.element)
    }

    static func build(_ c: ScriptContent) -> ScriptDocument {
        let out = NSMutableAttributedString()
        var lineRanges: [(String, NSRange)] = []
        var markRanges: [(String, ScriptMark, NSRange)] = []
        let font = NSFont(name: "Courier New", size: 13) ?? NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        let bold = NSFont(name: "Courier New Bold", size: 13) ?? NSFont.monospacedSystemFont(ofSize: 13, weight: .bold)
        let italic = NSFont(name: "Courier New Italic", size: 13) ?? font

        func style(_ type: String) -> (NSFont, NSMutableParagraphStyle, NSColor) {
            let p = NSMutableParagraphStyle()
            p.paragraphSpacing = 10
            p.lineSpacing = 2
            var f = font
            var color = c.palette.inkColor
            switch type {
            case "scene_heading": f = bold; p.paragraphSpacing = 12
            case "character": p.firstLineHeadIndent = 150; p.headIndent = 150; p.paragraphSpacing = 0
            case "dialogue": p.firstLineHeadIndent = 70; p.headIndent = 70; p.tailIndent = -70
            case "parenthetical": p.firstLineHeadIndent = 110; p.headIndent = 110; p.tailIndent = -110; p.paragraphSpacing = 0
            case "transition": p.alignment = .right
            case "shot": f = bold
            case "note": f = italic; color = c.palette.mutedColor
            default: break
            }
            return (f, p, color)
        }

        // The heading above the lines, with the scene number on both sides as in a shooting script.
        let headingText = (c.number.map { "\($0)  " } ?? "") + (c.heading.isEmpty ? "(no heading)" : c.heading)
        let hp = NSMutableParagraphStyle()
        hp.paragraphSpacing = 14
        out.append(NSAttributedString(string: headingText, attributes: [.font: bold, .foregroundColor: c.palette.inkColor, .paragraphStyle: hp]))

        let lines = displayLines(c)
        if lines.isEmpty {
            out.append(NSAttributedString(string: "\nThis scene has no text yet.", attributes: [.font: font, .foregroundColor: c.palette.mutedColor]))
        }
        for l in lines {
            out.append(NSAttributedString(string: "\n"))
            let (f, p, color) = style(l.elementType)
            let start = out.length
            let piece = NSMutableAttributedString(string: l.text, attributes: [.font: f, .foregroundColor: color, .paragraphStyle: p])
            for m in Highlight.resolve(c.marks[l.elementId] ?? [], length: (l.text as NSString).length) {
                let r = NSRange(location: m.start, length: m.end - m.start)
                let cat = m.category.flatMap { c.colors[$0.rawValue] }.flatMap(BDColor.nsColor(hex:)) ?? c.palette.mutedColor
                switch m.kind {
                case .tag:
                    piece.addAttributes([.backgroundColor: cat.withAlphaComponent(0.30), .underlineStyle: NSUnderlineStyle.single.rawValue, .underlineColor: cat], range: r)
                case .suggestion:
                    piece.addAttributes([.backgroundColor: cat.withAlphaComponent(0.10),
                                         .underlineStyle: NSUnderlineStyle.single.union(.patternDash).rawValue, .underlineColor: cat], range: r)
                case .jump:
                    piece.addAttributes([.backgroundColor: c.palette.jumpColor.withAlphaComponent(0.55),
                                         .underlineStyle: NSUnderlineStyle.thick.rawValue, .underlineColor: c.palette.inkColor], range: r)
                }
                piece.addAttribute(.toolTip, value: m.label, range: r)
                markRanges.append((l.elementId, m, NSRange(location: start + r.location, length: r.length)))
            }
            out.append(piece)
            lineRanges.append((l.elementId, NSRange(location: start, length: (l.text as NSString).length)))
        }
        return ScriptDocument(text: out, lineRanges: lineRanges, markRanges: markRanges)
    }
}

// MARK: The text view

final class ScriptNSTextView: NSTextView {
    var categories: [(category: BreakdownCategory, label: String)] = []
    var canTag = false
    var currentSelection: () -> TextSelection? = { nil }
    var onTag: (BreakdownCategory, TextSelection) -> Void = { _, _ in }

    override func menu(for event: NSEvent) -> NSMenu? {
        let menu = super.menu(for: event) ?? NSMenu()
        guard canTag, let sel = currentSelection() else { return menu }
        let shown = sel.text.count > 28 ? String(sel.text.prefix(28)) + "\u{2026}" : sel.text
        let item = NSMenuItem(title: "Tag \u{201C}\(shown)\u{201D} as\u{2026}", action: nil, keyEquivalent: "")
        let sub = NSMenu(title: "Tag as")
        for (i, c) in categories.enumerated() {
            let mi = NSMenuItem(title: c.label, action: #selector(tagAs(_:)), keyEquivalent: "")
            mi.target = self
            mi.tag = i
            sub.addItem(mi)
        }
        item.submenu = sub
        menu.insertItem(item, at: 0)
        menu.insertItem(.separator(), at: 1)
        return menu
    }

    @objc private func tagAs(_ item: NSMenuItem) {
        guard canTag, categories.indices.contains(item.tag), let sel = currentSelection() else { return }
        onTag(categories[item.tag].category, sel)
    }
}

struct ScriptTextView: NSViewRepresentable {
    let content: ScriptContent
    let canTag: Bool
    let categories: [(category: BreakdownCategory, label: String)]
    let jump: BreakdownSourceAnchor?
    let onSelection: (TextSelection?) -> Void
    let onTag: (BreakdownCategory, TextSelection) -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        let tv = ScriptNSTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 400))
        tv.isEditable = false
        tv.isSelectable = true
        tv.isRichText = true
        tv.importsGraphics = false
        tv.allowsUndo = false
        tv.isVerticallyResizable = true
        tv.isHorizontallyResizable = false
        tv.autoresizingMask = [.width]
        tv.minSize = NSSize(width: 0, height: 0)
        tv.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        tv.textContainer?.widthTracksTextView = true
        tv.textContainerInset = NSSize(width: 28, height: 24)
        tv.delegate = context.coordinator
        tv.setAccessibilityIdentifier("script-text")
        scroll.documentView = tv
        context.coordinator.textView = tv
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let co = context.coordinator
        guard let tv = co.textView else { return }
        co.onSelection = onSelection
        tv.canTag = canTag
        tv.categories = categories
        tv.onTag = onTag
        tv.currentSelection = { [weak co] in co?.selection() }
        tv.backgroundColor = content.palette.backgroundColor
        tv.drawsBackground = true
        tv.setAccessibilityLabel("Script, Scene \(content.number ?? "")")

        if co.shown != content {
            let sameScene = co.shown?.sceneId == content.sceneId
            let doc = ScriptDocument.build(content)
            co.document = doc
            co.shown = content
            let saved = tv.selectedRange()
            tv.textStorage?.setAttributedString(doc.text)
            if sameScene, NSMaxRange(saved) <= doc.text.length { tv.setSelectedRange(saved) } else {
                tv.setSelectedRange(NSRange(location: 0, length: 0))
                if !sameScene { tv.scroll(NSPoint.zero) }
            }
        }
        if let jump, jump.elementId != nil, co.shownJump != jump {
            co.shownJump = jump
            if let hit = co.document?.markRanges.first(where: { $0.mark.kind == .jump }) {
                tv.scrollRangeToVisible(hit.range)
                tv.showFindIndicator(for: hit.range)
            }
        } else if jump == nil {
            co.shownJump = nil
        }
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        weak var textView: ScriptNSTextView?
        var document: ScriptDocument?
        var shown: ScriptContent?
        var shownJump: BreakdownSourceAnchor?
        var onSelection: (TextSelection?) -> Void = { _ in }

        func selection() -> TextSelection? {
            guard let tv = textView, let doc = document else { return nil }
            return Highlight.selection(lines: doc.lineRanges, selected: tv.selectedRange(), in: tv.string as NSString)
        }

        func textViewDidChangeSelection(_ notification: Notification) {
            let sel = selection()
            // Not while SwiftUI is still updating this view.
            DispatchQueue.main.async { [onSelection] in onSelection(sel) }
        }
    }
}

// MARK: The pane

struct ScriptPaneView: View {
    @Bindable var model: BreakdownModel
    let detail: BreakdownSceneDetail
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private var content: ScriptContent {
        var marks: [String: [ScriptMark]] = [:]
        for g in detail.groups {
            for (list, kind) in [(g.confirmed, ScriptMark.Kind.tag), (g.suggestions, .suggestion)] {
                for e in list {
                    guard let line = e.spanElementId, let s = e.spanStart, let en = e.spanEnd, !e.archived else { continue }
                    marks[line, default: []].append(ScriptMark(
                        start: Int(s), end: Int(en), kind: kind,
                        label: "\(e.category.rawValue): \(e.name)\(kind == .suggestion ? " (suggested)" : "")", category: e.category))
                }
            }
        }
        if let j = model.jump, j.sceneId == detail.sceneId, let line = j.elementId, let s = j.start, let e = j.end {
            marks[line, default: []].append(ScriptMark(start: Int(s), end: Int(e), kind: .jump, label: "Source phrase: \(j.phrase ?? "")", category: nil))
        }
        var colors: [String: String] = [:]
        for c in BDCategory.all { if let h = hub.look(c).hex { colors[c.rawValue] = h } }
        return ScriptContent(sceneId: detail.sceneId, number: detail.number, heading: detail.heading, lines: detail.script,
                             marks: marks, colors: colors, palette: ScriptColors(theme: theme))
    }

    /// The pane title and the tag menu. In a narrow pane the menu keeps only its icon.
    private func paneHeader(iconOnly: Bool) -> some View {
        HStack(spacing: 8) {
            Text("Script").font(.headline).fixedSize()
            if !detail.inSource { BDChip(text: "Scene removed from current source") }
            Spacer(minLength: 0)
            if model.canEdit, detail.inSource {
                Menu {
                    ForEach(BDCategory.all, id: \.self) { c in
                        Button(hub.look(c).label) { model.tagSelection(category: c) }
                    }
                } label: {
                    if iconOnly {
                        Label("Tag Selection as", systemImage: "tag").labelStyle(.iconOnly)
                    } else {
                        Label("Tag Selection as", systemImage: "tag")
                    }
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .disabled(model.selection == nil)
                .help("Select words in the script, then choose a category")
                .accessibilityLabel("Tag Selection as")
                .accessibilityIdentifier("script-tag-menu")
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            ViewThatFits(in: .horizontal) {
                paneHeader(iconOnly: false)
                paneHeader(iconOnly: true)
            }
            .padding(.horizontal, 14).padding(.vertical, 8)
            Divider()
            ScriptTextView(
                content: content,
                canTag: model.canEdit && detail.inSource,
                categories: BDCategory.all.map { ($0, hub.look($0).label) },
                jump: model.jump?.sceneId == detail.sceneId ? model.jump : nil,
                onSelection: { model.selection = $0 },
                onTag: { c, sel in
                    model.selection = sel
                    model.tagSelection(category: c)
                })
        }
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Script, Scene \(detail.number ?? "")")
        .accessibilityIdentifier("script-pane")
    }
}

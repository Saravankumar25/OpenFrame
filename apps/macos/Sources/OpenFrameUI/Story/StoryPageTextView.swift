// The Story page editor (Windows StoryPage.tsx, commits ed78cf85, 69e17c98 and b1527711): a text
// view on an A4 sheet that keeps the formatting in its own attributes (StoryPageDocument.Key) and
// draws it for the light or the dark page. Command+B, I and U, Command+Shift+H for the highlight,
// and Command+[ and Command+] for one size down or up are handled here while the page has the
// keyboard; elsewhere Command+[ and Command+] stay Back and Forward of the Go menu.

import AppKit

/// What the toolbar shows for the selection: a style is on when any of the selected text has it
/// (a click then takes it off, as in Word); the size and the colours are nil when the selection
/// mixes them.
struct StoryPageTools: Equatable {
    enum Colored: String { case underline, highlight }

    var bold = false
    var italic = false
    var underline = false
    var highlight = false
    var underlineColor: String?
    var highlightColor: String?
    var size: Double? = StoryPageDocument.bodySize

    func isOn(_ kind: Colored) -> Bool { kind == .underline ? underline : highlight }
    func color(_ kind: Colored) -> String? { kind == .underline ? underlineColor : highlightColor }
}

final class StoryPageTextView: NSTextView {
    /// Called after every change of the text or its formatting.
    var onChange: ((NSAttributedString) -> Void)?
    /// Called when what the toolbar shows changes.
    var onTools: ((StoryPageTools) -> Void)?
    /// Called when the page loses the keyboard (the text is stored then).
    var onLeave: (() -> Void)?
    /// The colour the main Underline and Highlight buttons (and Command+U, Command+Shift+H) put on:
    /// the last one picked from the list, for this visit to the page.
    var lastColor: [StoryPageTools.Colored: String] = [
        .underline: StoryPageDocument.underlines[0].name,
        .highlight: StoryPageDocument.highlights[0].name,
    ]
    private(set) var tools = StoryPageTools()

    static let placeholder = "Start writing your story here."

    static func make() -> StoryPageTextView {
        let view = StoryPageTextView(frame: NSRect(x: 0, y: 0, width: StoryPageDocument.pageWidth, height: StoryPageDocument.pageHeight))
        view.configure()
        return view
    }

    private func configure() {
        isRichText = true
        importsGraphics = false
        allowsImageEditing = false
        allowsUndo = true
        usesFontPanel = false
        usesRuler = false
        isContinuousSpellCheckingEnabled = true
        isAutomaticQuoteSubstitutionEnabled = false
        isAutomaticDashSubstitutionEnabled = false
        isAutomaticTextReplacementEnabled = false
        drawsBackground = true
        textContainerInset = NSSize(width: StoryPageDocument.margin, height: StoryPageDocument.margin)
        textContainer?.lineFragmentPadding = 0
        textContainer?.widthTracksTextView = true
        isHorizontallyResizable = false
        isVerticallyResizable = true
        autoresizingMask = []
        minSize = NSSize(width: StoryPageDocument.pageWidth, height: StoryPageDocument.pageHeight)
        maxSize = NSSize(width: StoryPageDocument.pageWidth, height: .greatestFiniteMagnitude)
        typingAttributes = StoryPageDocument.Style().attributes()
        showPage(dark: false)
        setAccessibilityLabel("Story text")
        setAccessibilityIdentifier("story-page-text")
    }

    /// The light or the dark sheet (screen only; exports stay white). The colours of the text,
    /// the highlights and the underlines follow the appearance the view is pinned to.
    func showPage(dark: Bool) {
        let wanted = NSAppearance(named: dark ? .darkAqua : .aqua)
        if appearance?.name != wanted?.name { appearance = wanted }
        let sheet = StoryPageDocument.sheet(dark: dark)
        backgroundColor = StoryPageDocument.color(sheet.paper)
        insertionPointColor = StoryPageDocument.color(sheet.ink)
        needsDisplay = true
    }

    var isDarkPage: Bool { appearance?.name == .darkAqua }

    /// Shows a stored page. Not an edit: nothing is saved and nothing can be undone past it.
    func load(_ s: NSAttributedString) {
        textStorage?.setAttributedString(s)
        undoManager?.removeAllActions(withTarget: self)
        setSelectedRange(NSRange(location: 0, length: 0))
        typingAttributes = s.length > 0 ? StoryPageDocument.Style(s.attributes(at: 0, effectiveRange: nil)).attributes() : StoryPageDocument.Style().attributes()
        refreshTools()
    }

    // MARK: Drawing

    /// A faint rule marks where each A4 page ends and the edge of the sheet; the placeholder
    /// shows on an empty page.
    override func drawBackground(in rect: NSRect) {
        super.drawBackground(in: rect)
        let sheet = StoryPageDocument.sheet(dark: isDarkPage)
        StoryPageDocument.color(sheet.rule).setStroke()
        NSBezierPath(rect: bounds.insetBy(dx: 0.5, dy: 0.5)).stroke()
        StoryPageDocument.color(sheet.rule).setFill()
        let h = StoryPageDocument.pageHeight
        var y = h
        while y < bounds.maxY {
            if y >= rect.minY - 1, y <= rect.maxY + 1 { NSRect(x: bounds.minX, y: y - 1, width: bounds.width, height: 1).fill() }
            y += h
        }
        if string.isEmpty {
            let attrs: [NSAttributedString.Key: Any] = [
                .font: StoryPageDocument.font(size: StoryPageDocument.bodySize, bold: false, italic: false),
                .foregroundColor: StoryPageDocument.color(sheet.placeholder),
            ]
            NSAttributedString(string: Self.placeholder, attributes: attrs)
                .draw(at: NSPoint(x: textContainerOrigin.x, y: textContainerOrigin.y))
        }
    }

    // MARK: Keys

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard window?.firstResponder === self, handleShortcut(event) else { return super.performKeyEquivalent(with: event) }
        return true
    }

    /// The formatting shortcuts. Only Command (and Shift for the highlight) count.
    func handleShortcut(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask).subtracting([.capsLock, .numericPad, .function])
        let key = (event.charactersIgnoringModifiers ?? "").lowercased()
        if flags == .command {
            switch key {
            case "b": toggle(\.bold); return true
            case "i": toggle(\.italic); return true
            case "u": toggleColored(.underline); return true
            case "]": stepSize(1); return true
            case "[": stepSize(-1); return true
            default: return false
            }
        }
        if flags == [.command, .shift], key == "h" {
            toggleColored(.highlight)
            return true
        }
        return false
    }

    /// Shift+Return starts a new line like Return (the stored text has one kind of line break).
    override func insertLineBreak(_ sender: Any?) { insertNewline(sender) }

    // MARK: Paste and drop: words only, in the style at the caret

    override var readablePasteboardTypes: [NSPasteboard.PasteboardType] { [.string] }

    override func paste(_ sender: Any?) { pasteAsPlainText(sender) }

    override func readSelection(from pboard: NSPasteboard, type: NSPasteboard.PasteboardType) -> Bool {
        guard let s = pboard.string(forType: .string) else { return false }
        let clean = s.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
        insertText(clean, replacementRange: rangeForUserTextChange)
        return true
    }

    override func didChangeText() {
        super.didChangeText()
        if let s = textStorage { onChange?(s) }
        refreshTools()
    }

    override func setSelectedRanges(_ ranges: [NSValue], affinity: NSSelectionAffinity, stillSelecting: Bool) {
        super.setSelectedRanges(ranges, affinity: affinity, stillSelecting: stillSelecting)
        if !stillSelecting { refreshTools() }
    }

    override func resignFirstResponder() -> Bool {
        let ok = super.resignFirstResponder()
        if ok { onLeave?() }
        return ok
    }

    // MARK: Formatting

    /// The selected text without its line breaks (the formatting of a line break means nothing).
    private func selectedStyles() -> [StoryPageDocument.Style] {
        let range = selectedRange()
        guard let storage = textStorage, range.length > 0 else { return [] }
        var out: [StoryPageDocument.Style] = []
        let ns = storage.string as NSString
        storage.enumerateAttributes(in: range) { attrs, r, _ in
            if ns.substring(with: r).contains(where: { $0 != "\n" && $0 != "\r" }) { out.append(StoryPageDocument.Style(attrs)) }
        }
        return out
    }

    func refreshTools() {
        let range = selectedRange()
        var t = StoryPageTools()
        if range.length == 0 {
            let s = StoryPageDocument.Style(typingAttributes)
            t = StoryPageTools(bold: s.bold, italic: s.italic, underline: s.underline != nil, highlight: s.highlight != nil,
                               underlineColor: s.underline, highlightColor: s.highlight, size: s.size)
        } else {
            let styles = selectedStyles()
            let one = { (xs: [String?]) -> String? in Set(xs).count == 1 ? xs.first ?? nil : nil }
            let sizes = Set(styles.map(\.size))
            t = StoryPageTools(
                bold: styles.contains { $0.bold }, italic: styles.contains { $0.italic },
                underline: styles.contains { $0.underline != nil }, highlight: styles.contains { $0.highlight != nil },
                underlineColor: one(styles.map(\.underline)), highlightColor: one(styles.map(\.highlight)),
                size: sizes.isEmpty ? StoryPageDocument.bodySize : sizes.count == 1 ? sizes.first : nil)
        }
        if t != tools {
            tools = t
            onTools?(t)
        }
    }

    /// Changes the style of the selection (or of what is typed next when nothing is selected) as
    /// one step that Undo takes back.
    func restyle(_ change: (inout StoryPageDocument.Style) -> Void) {
        let range = selectedRange()
        if range.length == 0 {
            var s = StoryPageDocument.Style(typingAttributes)
            change(&s)
            typingAttributes = s.attributes()
            refreshTools()
            return
        }
        guard let storage = textStorage, shouldChangeText(in: range, replacementString: nil) else { return }
        var parts: [(NSRange, StoryPageDocument.Style)] = []
        storage.enumerateAttributes(in: range) { attrs, r, _ in
            var s = StoryPageDocument.Style(attrs)
            change(&s)
            parts.append((r, s))
        }
        storage.beginEditing()
        for (r, s) in parts { storage.setAttributes(s.attributes(), range: r) }
        storage.endEditing()
        didChangeText()
        setSelectedRange(range)
    }

    func toggle(_ key: WritableKeyPath<StoryPageDocument.Style, Bool>) {
        let on = key == \StoryPageDocument.Style.bold ? tools.bold : tools.italic
        restyle { $0[keyPath: key] = !on }
    }

    func toggleColored(_ kind: StoryPageTools.Colored) {
        let on = tools.isOn(kind)
        setColor(kind, on ? nil : lastColor[kind])
    }

    /// Gives the selection the underline or highlight `color`, or takes it off when nil.
    func setColor(_ kind: StoryPageTools.Colored, _ color: String?) {
        restyle { s in
            if kind == .underline { s.underline = color } else { s.highlight = color }
        }
    }

    /// Picks a colour from the list: it becomes the colour of the main button too.
    func pickColor(_ kind: StoryPageTools.Colored, _ color: String) {
        lastColor[kind] = color
        setColor(kind, color)
    }

    func setSize(_ pt: Double) {
        let v = StoryPageDocument.clampSize(pt)
        restyle { $0.size = v }
    }

    /// One step up or down the toolbar sizes from the size at the caret.
    func stepSize(_ dir: Int) {
        let now = tools.size ?? StoryPageDocument.bodySize
        let next = dir > 0 ? StoryPageDocument.sizes.first { $0 > now } : StoryPageDocument.sizes.last { $0 < now }
        if let next { setSize(next) }
    }
}

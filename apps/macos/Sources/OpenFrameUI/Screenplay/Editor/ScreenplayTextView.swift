// The native screenplay text engine (Mac phase M4): an NSTextView on TextKit 2 where every
// paragraph is one screenplay element. The element's type, id and custom element id travel as
// paragraph attributes, so selection, input methods, spelling, VoiceOver and undo are the
// system's own. The Rust core stays the source of truth: the editor reports its paragraphs as
// blocks and the sync controller sends the difference as element operations.

import AppKit
import OpenFrameCore

extension NSAttributedString.Key {
    /// The element type of the paragraph (`ElementType.rawValue`).
    static let ofKind = NSAttributedString.Key("studio.openframe.kind")
    /// The element id (a client-made UUID for new paragraphs, the core's id otherwise).
    static let ofElementId = NSAttributedString.Key("studio.openframe.elementId")
    /// The custom element of a `custom` paragraph.
    static let ofCustomId = NSAttributedString.Key("studio.openframe.customId")
    /// The scene id, carried on scene heading paragraphs.
    static let ofSceneId = NSAttributedString.Key("studio.openframe.sceneId")
}

/// One paragraph of the editor as plain data.
public struct EditorParagraph: Equatable, Sendable {
    public var kind: ElementType
    public var text: String
    public var elementId: String?
    public var customElementId: String?
    public var sceneId: String?

    public init(kind: ElementType, text: String, elementId: String? = nil, customElementId: String? = nil, sceneId: String? = nil) {
        self.kind = kind
        self.text = text
        self.elementId = elementId
        self.customElementId = customElementId
        self.sceneId = sceneId
    }
}

/// How one element type looks on the page, from the shared element rules.
public struct ElementLook: Sendable {
    public var leftInches: Double
    public var widthInches: Double
    public var alignment: NSTextAlignment
    public var uppercase: Bool
    public var bold: Bool
    public var italic: Bool
    public var linesBefore: Double

    public init(leftInches: Double, widthInches: Double, alignment: NSTextAlignment = .left, uppercase: Bool = false,
                bold: Bool = false, italic: Bool = false, linesBefore: Double = 1) {
        self.leftInches = leftInches
        self.widthInches = widthInches
        self.alignment = alignment
        self.uppercase = uppercase
        self.bold = bold
        self.italic = italic
        self.linesBefore = linesBefore
    }

    /// The industry layout the Windows editor uses (elementRules.json), for tests and as a default.
    public static func standard(_ kind: ElementType) -> ElementLook {
        switch kind {
        case .sceneHeading: ElementLook(leftInches: 1.5, widthInches: 6.0, uppercase: true, bold: true, linesBefore: 2)
        case .action: ElementLook(leftInches: 1.5, widthInches: 6.0)
        case .character: ElementLook(leftInches: 3.7, widthInches: 3.8, uppercase: true)
        case .parenthetical: ElementLook(leftInches: 3.1, widthInches: 2.6, linesBefore: 0)
        case .dialogue: ElementLook(leftInches: 2.5, widthInches: 3.5, linesBefore: 0)
        case .transition: ElementLook(leftInches: 5.5, widthInches: 2.0, alignment: .right, uppercase: true)
        case .shot: ElementLook(leftInches: 1.5, widthInches: 6.0, uppercase: true)
        case .note: ElementLook(leftInches: 1.5, widthInches: 6.0, italic: true)
        case .pageBreak: ElementLook(leftInches: 1.5, widthInches: 6.0)
        case .custom: ElementLook(leftInches: 1.5, widthInches: 6.0)
        }
    }
}

/// Keyboard and layout decisions the text view asks for. The app injects the shared element rules.
@MainActor
public protocol ScreenplayEditorRules: AnyObject {
    func look(for kind: ElementType) -> ElementLook
    /// Shift+Enter makes a line break inside these elements; the others treat it as Enter.
    func isMultiline(_ kind: ElementType) -> Bool
    /// The element a new paragraph gets after Enter at the end of `kind` (empty paragraphs too).
    func kindAfterEnter(_ kind: ElementType, text: String) -> ElementType
    /// The element Tab (forward) or Shift+Tab (backward) turns the paragraph into, or nil.
    func kindAfterTab(_ kind: ElementType, text: String, backward: Bool) -> ElementType?
    /// Enter in an empty paragraph converts it to this element instead of adding a line; nil keeps
    /// it as it is (an empty scene heading stays put).
    func kindForEmptyEnter(_ kind: ElementType) -> ElementType?
    /// The element of Command+digit (the Windows Ctrl+digit shortcuts), or nil.
    func kind(forShortcutDigit digit: Int) -> ElementType?
}

extension ScreenplayEditorRules {
    public func isMultiline(_ kind: ElementType) -> Bool { false }
}

/// Everything the editor tells its owner.
@MainActor
public protocol ScreenplayTextViewDelegate: AnyObject {
    /// The text changed by the person (not by `load`).
    func screenplayTextDidChange(_ view: ScreenplayTextView)
    /// The caret moved into another paragraph.
    func screenplayTextView(_ view: ScreenplayTextView, didMoveInto paragraph: EditorParagraph, index: Int)
}

public final class ScreenplayTextView: NSTextView, @preconcurrency NSTextContentStorageDelegate {
    public weak var rules: ScreenplayEditorRules?
    public weak var screenplayDelegate: ScreenplayTextViewDelegate?
    /// Points per inch on screen (zoom). 72 is print size.
    public var pointsPerInch: CGFloat = 80 { didSet { restyleAll() } }
    public private(set) var isLoading = false
    private var lastParagraphIndex = -1
    /// The scene number shown beside each scene heading, by scene id (the document's labels).
    public var sceneLabels: [String: String] = [:] { didSet { needsDisplay = true } }
    /// The custom elements of the open screenplay; their look comes from these.
    public var customDefinitions: [ScreenplayCustomElementDto] = [] { didSet { restyleCustom() } }
    /// Characters and locations the suggestion list offers besides the script's own values.
    public var suggestionSources: WritingSuggestionSources? {
        didSet { suggestions.invalidateValues(); refreshSuggestions(textChanged: false) }
    }
    /// "Create New" was accepted: kind, name, and whether the same key press moves on to Dialogue.
    /// Called before the Dialogue line is added, so the owner can record the new value in order.
    public var onCreateSuggestion: ((SuggestCreate, String, Bool) -> Void)?
    /// A known name filled by Enter is about to move on to Dialogue: save the filled name first.
    public var beforeSuggestionMoveOn: (() -> Void)?
    /// "Learn Spelling" on a misspelled word.
    public var onLearnSpelling: ((String) -> Void)?
    /// Escape pressed with no suggestion list open (Focus Mode listens for it).
    public var onEscape: (() -> Void)?
    /// An empty last paragraph has no character to carry its element, id or custom element. Its
    /// attributes are kept here while the caret is elsewhere; the typing attributes hold them while
    /// the caret is in it.
    private var trailingAttributes: [NSAttributedString.Key: Any]?
    /// Names the spell checker must not underline (characters, locations), as single words.
    var ignoredWordList: [String] = []
    let suggestions = ScreenplaySuggestionController()
    var suggestionPopover: SuggestionPopover?

    public static let pageTextWidthInches: CGFloat = 6.0
    public static let pageLeftMarginInches: CGFloat = 1.5

    /// A text view on TextKit 2 with the screenplay defaults.
    public static func make() -> ScreenplayTextView {
        let view = ScreenplayTextView(usingTextLayoutManager: true)
        view.configure()
        return view
    }

    /// When the text last changed. Typing more than 4 seconds after the last change starts a new
    /// undo step, as in the Windows editor.
    private var lastEditAt = Date.distantPast
    static let undoPauseSeconds: TimeInterval = 4

    private func configure() {
        isRichText = false
        importsGraphics = false
        allowsUndo = true
        isContinuousSpellCheckingEnabled = true
        isGrammarCheckingEnabled = false
        isAutomaticQuoteSubstitutionEnabled = false
        isAutomaticDashSubstitutionEnabled = false
        isAutomaticTextReplacementEnabled = false
        isAutomaticSpellingCorrectionEnabled = false
        smartInsertDeleteEnabled = false
        usesFindBar = true
        isIncrementalSearchingEnabled = true
        textContainerInset = NSSize(width: 0, height: 48)
        textContainer?.lineFragmentPadding = 0
        textContainer?.widthTracksTextView = false
        isHorizontallyResizable = false
        isVerticallyResizable = true
        updateContainerWidth()
        if let storage = textLayoutManager?.textContentManager as? NSTextContentStorage {
            storage.delegate = self
        }
        typingAttributes = attributes(for: .action, elementId: UUID().uuidString.lowercased(), custom: nil, sceneId: nil)
        setAccessibilityLabel("Screenplay")
        setAccessibilityIdentifier("screenplay-editor")
    }

    private func updateContainerWidth() {
        textContainer?.size = NSSize(width: Self.pageTextWidthInches * pointsPerInch + 2 * pointsPerInch,
                                     height: CGFloat.greatestFiniteMagnitude)
    }

    // MARK: Styling

    public static var screenplayFont: NSFont {
        NSFont(name: "Courier Prime", size: 12) ?? NSFont(name: "Courier New", size: 12) ?? NSFont.userFixedPitchFont(ofSize: 12)!
    }

    func font(for look: ElementLook) -> NSFont {
        let base = NSFont(descriptor: Self.screenplayFont.fontDescriptor, size: 12 * pointsPerInch / 72) ?? Self.screenplayFont
        var traits: NSFontDescriptor.SymbolicTraits = []
        if look.bold { traits.insert(.bold) }
        if look.italic { traits.insert(.italic) }
        guard !traits.isEmpty else { return base }
        return NSFont(descriptor: base.fontDescriptor.withSymbolicTraits(traits), size: base.pointSize) ?? base
    }

    func paragraphStyle(for look: ElementLook) -> NSParagraphStyle {
        let style = NSMutableParagraphStyle()
        let indent = (CGFloat(look.leftInches) - Self.pageLeftMarginInches + 1) * pointsPerInch
        style.firstLineHeadIndent = indent
        style.headIndent = indent
        style.tailIndent = indent + CGFloat(look.widthInches) * pointsPerInch
        style.alignment = look.alignment
        let line = 12 * pointsPerInch / 72
        style.minimumLineHeight = line
        style.maximumLineHeight = line
        style.paragraphSpacingBefore = CGFloat(look.linesBefore) * line
        return style
    }

    /// The look of an element; a custom element takes the look its definition names.
    func look(for kind: ElementType, custom: String?) -> ElementLook {
        guard kind == .custom, let id = custom, let def = customDefinitions.first(where: { $0.id == id }) else {
            return rules?.look(for: kind) ?? .standard(kind)
        }
        let base: ElementType
        switch def.look {
        case "character": base = .character
        case "parenthetical": base = .parenthetical
        case "dialogue": base = .dialogue
        case "transition": base = .transition
        case "shot": base = .shot
        default: base = .action
        }
        var look = rules?.look(for: base) ?? .standard(base)
        if def.look == "centered" { look.alignment = .center }
        look.uppercase = def.uppercase
        look.italic = def.italic
        look.bold = false
        look.linesBefore = def.spaceBefore
        return look
    }

    func attributes(for kind: ElementType, elementId: String?, custom: String?, sceneId: String?) -> [NSAttributedString.Key: Any] {
        let look = self.look(for: kind, custom: custom)
        var attrs: [NSAttributedString.Key: Any] = [
            .font: font(for: look),
            .paragraphStyle: paragraphStyle(for: look),
            .foregroundColor: NSColor.textColor,
            .ofKind: kind.rawValue,
        ]
        if let elementId { attrs[.ofElementId] = elementId }
        if let custom { attrs[.ofCustomId] = custom }
        if let sceneId { attrs[.ofSceneId] = sceneId }
        return attrs
    }

    // Display-only upper case (as the Windows editor's CSS text-transform): the stored text keeps
    // what was typed; TextKit 2 shows the paragraph in capitals.
    public func textContentStorage(_ textContentStorage: NSTextContentStorage, textParagraphWith range: NSRange) -> NSTextParagraph? {
        guard let storage = textContentStorage.textStorage, range.location < storage.length else { return nil }
        let kindRaw = storage.attribute(.ofKind, at: range.location, effectiveRange: nil) as? String
        let customId = storage.attribute(.ofCustomId, at: range.location, effectiveRange: nil) as? String
        guard let kind = kindRaw.flatMap(ElementType.init(rawValue:)),
              look(for: kind, custom: customId).uppercase else { return nil }
        let original = storage.attributedSubstring(from: range)
        let upper = original.string.uppercased()
        // Only when the capitals keep every character's position (true for almost all scripts).
        guard (upper as NSString).length == (original.string as NSString).length else { return nil }
        let display = NSMutableAttributedString(attributedString: original)
        display.replaceCharacters(in: NSRange(location: 0, length: display.length), with: upper)
        original.enumerateAttributes(in: NSRange(location: 0, length: original.length)) { attrs, r, _ in
            display.setAttributes(attrs, range: r)
        }
        return NSTextParagraph(attributedString: display)
    }

    // MARK: Loading and reading

    /// Replace the content with paragraphs from the core. Keeps the caret near where it was.
    public func load(_ paragraphs: [EditorParagraph]) {
        isLoading = true
        defer { isLoading = false }
        let caret = selectedRange()
        let text = NSMutableAttributedString()
        for (i, p) in paragraphs.enumerated() {
            let attrs = attributes(for: p.kind, elementId: p.elementId, custom: p.customElementId, sceneId: p.sceneId)
            text.append(NSAttributedString(string: p.text + (i < paragraphs.count - 1 ? "\n" : ""), attributes: attrs))
        }
        if paragraphs.isEmpty {
            // Nothing from the previous document may leak into this one (ids, element).
            typingAttributes = attributes(for: .sceneHeading, elementId: nil, custom: nil, sceneId: nil)
        }
        textStorage?.setAttributedString(text)
        if let last = paragraphs.last, last.text.isEmpty {
            // An empty last paragraph has no characters to carry its element and id.
            trailingAttributes = attributes(for: last.kind, elementId: last.kind == .sceneHeading ? nil : last.elementId, custom: last.customElementId, sceneId: last.sceneId)
            if text.length == 0, let trailing = trailingAttributes {
                // A document that is one empty paragraph (a new screenplay's first scene).
                typingAttributes = trailing
            }
        } else {
            trailingAttributes = nil
        }
        let location = min(caret.location, text.length)
        setSelectedRange(NSRange(location: location, length: 0))
        // The typing style must belong to the new content at the caret, never the old document.
        if location >= text.length, let trailing = trailingAttributes {
            typingAttributes = trailing
        } else if text.length > 0 {
            let ns = text.string as NSString
            let range = ns.paragraphRange(for: NSRange(location: min(location, text.length - 1), length: 0))
            typingAttributes = text.attributes(at: range.location, effectiveRange: nil)
        }
        undoManager?.removeAllActions()
        lastParagraphIndex = -1
        suggestions.invalidateValues()
        needsDisplay = true
    }

    /// The paragraphs as the person sees them now.
    public func paragraphs() -> [EditorParagraph] {
        guard let storage = textStorage else { return [] }
        let ns = storage.string as NSString
        var out: [EditorParagraph] = []
        var location = 0
        repeat {
            let range = ns.paragraphRange(for: NSRange(location: location, length: 0))
            var textRange = range
            if textRange.length > 0, ns.character(at: NSMaxRange(textRange) - 1) == 10 { textRange.length -= 1 }
            let attrsAt = range.length > 0 ? range.location : max(0, min(location, storage.length - 1))
            let attrs = storage.length > 0 ? storage.attributes(at: attrsAt, effectiveRange: nil) : typingAttributes
            let kind = (attrs[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
            out.append(EditorParagraph(
                kind: kind,
                text: ns.substring(with: textRange),
                elementId: attrs[.ofElementId] as? String,
                customElementId: attrs[.ofCustomId] as? String,
                sceneId: attrs[.ofSceneId] as? String))
            location = NSMaxRange(range)
        } while location < ns.length
        if ns.length > 0, ns.character(at: ns.length - 1) == 10 {
            // A final empty paragraph after a trailing newline.
            let attrs = trailingParagraphAttributes()
            let kind = (attrs[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
            out.append(EditorParagraph(
                kind: kind, text: "", elementId: attrs[.ofElementId] as? String,
                customElementId: attrs[.ofCustomId] as? String, sceneId: attrs[.ofSceneId] as? String))
        }
        return out
    }

    /// The element of the empty last paragraph: the typing style while the caret is in it.
    func trailingParagraphAttributes() -> [NSAttributedString.Key: Any] {
        let length = textStorage?.length ?? 0
        if selectedRange().location >= length { return typingAttributes }
        return trailingAttributes ?? typingAttributes
    }

    /// The caret is about to leave the empty last paragraph: keep its element.
    private func captureTrailingAttributes() {
        // While a document loads, the typing style still belongs to the previous content; the
        // load sets the trailing element itself (found in M4: stale ids leaked into a new script).
        guard !isLoading, let storage = textStorage, storage.length > 0, selectedRange().location >= storage.length,
              (storage.string as NSString).character(at: storage.length - 1) == 10 else { return }
        trailingAttributes = typingAttributes
    }

    /// The paragraph index of the caret.
    public var caretParagraphIndex: Int {
        guard let storage = textStorage else { return 0 }
        let ns = storage.string as NSString
        let loc = min(selectedRange().location, ns.length)
        var index = 0
        var i = 0
        while i < loc {
            if ns.character(at: i) == 10 { index += 1 }
            i += 1
        }
        return index
    }

    /// Where the caret's paragraph text ends (before its line end).
    func currentParagraphContentEnd() -> Int {
        let range = currentParagraphRange()
        let ns = (textStorage?.string ?? "") as NSString
        var end = NSMaxRange(range)
        if range.length > 0, ns.character(at: end - 1) == 10 { end -= 1 }
        return end
    }

    func currentParagraphRange() -> NSRange {
        let ns = (textStorage?.string ?? "") as NSString
        return ns.paragraphRange(for: NSRange(location: min(selectedRange().location, ns.length), length: 0))
    }

    func currentKind() -> ElementType {
        let range = currentParagraphRange()
        if let storage = textStorage, storage.length > 0 {
            let at = min(range.location, storage.length - 1)
            if range.length > 0 || at == storage.length - 1,
               let raw = storage.attribute(.ofKind, at: at, effectiveRange: nil) as? String,
               let kind = ElementType(rawValue: raw), range.length > 0 {
                return kind
            }
        }
        return (typingAttributes[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
    }

    func currentText() -> String {
        let range = currentParagraphRange()
        var text = ((textStorage?.string ?? "") as NSString).substring(with: range)
        if text.hasSuffix("\n") { text.removeLast() }
        return text
    }

    // MARK: Element changes

    /// Turn the caret's paragraph into another element (menu, shortcut, Tab).
    public func setKind(_ kind: ElementType, customElementId: String? = nil) {
        let range = currentParagraphRange()
        let existingId = range.length > 0 ? textStorage?.attribute(.ofElementId, at: range.location, effectiveRange: nil) as? String : nil
        let sceneId = range.length > 0 ? textStorage?.attribute(.ofSceneId, at: range.location, effectiveRange: nil) as? String : nil
        let id = existingId ?? (typingAttributes[.ofElementId] as? String) ?? UUID().uuidString.lowercased()
        let attrs = attributes(for: kind, elementId: id, custom: customElementId, sceneId: sceneId)
        if range.length > 0, shouldChangeText(in: range, replacementString: nil) {
            textStorage?.setAttributes(attrs, range: range)
            didChangeText()
        }
        typingAttributes = attrs
        notifyMove(force: true)
    }

    func currentCustomId() -> String? {
        let range = currentParagraphRange()
        guard range.length > 0 else { return typingAttributes[.ofCustomId] as? String }
        return textStorage?.attribute(.ofCustomId, at: range.location, effectiveRange: nil) as? String
    }

    /// The element Enter makes after a custom element: the one its definition names.
    private func elementAfterCustom(_ customId: String?) -> (ElementType, String?) {
        let next = customDefinitions.first { $0.id == customId }?.nextElement ?? "action"
        if next.hasPrefix("custom:") {
            let id = String(next.dropFirst("custom:".count))
            if customDefinitions.contains(where: { $0.id == id }) { return (.custom, id) }
            return (.action, nil)
        }
        if let t = ElementType(rawValue: next), t != .custom, t != .pageBreak { return (t, nil) }
        return (.action, nil)
    }

    public override func insertNewline(_ sender: Any?) {
        if acceptHighlightedSuggestion(moveOn: true) { return }
        performEnter(sender)
    }

    /// Enter, as the shared element rules say: end of an element starts the next one, an empty
    /// element converts, the middle splits (a heading continues as action), the start keeps the
    /// text with its element and adds a blank one above.
    func performEnter(_ sender: Any?) {
        suggestions.invalidateValues()
        let kind = currentKind()
        let custom = currentCustomId()
        if kind == .pageBreak {
            // Enter on a page break adds an action after it and keeps the break.
            setSelectedRange(NSRange(location: currentParagraphContentEnd(), length: 0))
            super.insertNewline(sender)
            applyToCaretParagraph(kind: .action, custom: nil, newId: true)
            return
        }
        if currentText().isEmpty, selectedRange().length == 0 {
            if let converted = rules?.kindForEmptyEnter(kind) { setKind(converted) }
            return
        }
        if selectedRange().length > 0 { delete(nil) }
        let range = currentParagraphRange()
        let text = currentText() as NSString
        let offset = selectedRange().location - range.location
        let atEnd = offset >= text.length
        let atStart = offset == 0
        let paragraphAttrs = range.length > 0 ? textStorage?.attributes(at: range.location, effectiveRange: nil) : nil
        if atEnd, selectedRange().length == 0, kind != .custom, let target = followingEmptyParagraph(after: range),
           target.kind == (rules?.kindAfterEnter(kind, text: text as String) ?? .action) {
            // The element that follows is already the empty one Enter would create: go into it
            // instead of stacking another empty element in front of it.
            if let attrs = target.implicitAttributes { typingAttributes = attrs }
            setSelectedRange(NSRange(location: target.location, length: 0))
            notifyMove(force: true)
            return
        }
        super.insertNewline(sender)
        if atEnd {
            let (nextKind, nextCustom): (ElementType, String?) = kind == .custom
                ? elementAfterCustom(custom)
                : (rules?.kindAfterEnter(kind, text: text as String) ?? .action, nil)
            applyToCaretParagraph(kind: nextKind, custom: nextCustom, newId: true)
        } else if atStart {
            // The blank paragraph above is the new one; the text keeps its element and id.
            let blank = currentParagraphRange(before: true)
            if blank.length > 0 {
                textStorage?.setAttributes(attributes(for: kind, elementId: UUID().uuidString.lowercased(), custom: custom, sceneId: nil), range: blank)
            }
            if let paragraphAttrs { typingAttributes = paragraphAttrs }
            notifyMove(force: true)
        } else {
            // Split: the part after the caret is new; a heading continues as action.
            let tail: ElementType = kind == .sceneHeading ? .action : kind
            applyToCaretParagraph(kind: tail, custom: kind == .custom ? custom : nil, newId: true)
        }
    }

    /// The paragraph after `range` when it has no text: where it starts, its element, and for the
    /// empty last paragraph (no characters of its own) the attributes kept for it.
    private func followingEmptyParagraph(after range: NSRange) -> (location: Int, kind: ElementType, implicitAttributes: [NSAttributedString.Key: Any]?)? {
        let ns = (textStorage?.string ?? "") as NSString
        let start = NSMaxRange(range)
        guard range.length > 0, ns.character(at: start - 1) == 10 else { return nil }
        if start >= ns.length {
            guard let attrs = trailingAttributes else { return nil }
            let kind = (attrs[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
            return (ns.length, kind, attrs)
        }
        let next = ns.paragraphRange(for: NSRange(location: start, length: 0))
        let content = next.length > 0 && ns.character(at: NSMaxRange(next) - 1) == 10 ? next.length - 1 : next.length
        guard content == 0 else { return nil }
        return (start, kind(atParagraphContaining: start), nil)
    }

    /// The paragraph before the caret's paragraph (the blank one a split at the start leaves).
    private func currentParagraphRange(before: Bool) -> NSRange {
        let ns = (textStorage?.string ?? "") as NSString
        let current = currentParagraphRange()
        guard before, current.location > 0 else { return current }
        return ns.paragraphRange(for: NSRange(location: current.location - 1, length: 0))
    }

    /// Give the caret's paragraph an element, a new id, and make it the typing style.
    private func applyToCaretParagraph(kind: ElementType, custom: String?, newId: Bool) {
        let attrs = attributes(for: kind, elementId: newId ? UUID().uuidString.lowercased() : nil, custom: custom, sceneId: nil)
        typingAttributes = attrs
        let range = currentParagraphRange()
        if range.length > 0 { textStorage?.setAttributes(attrs, range: range) }
        notifyMove(force: true)
    }

    // Shift+Enter: a line break inside a multi-line element, the same as Enter in the others.
    public override func insertLineBreak(_ sender: Any?) {
        let kind = currentKind()
        if kind != .pageBreak, kind == .custom || (rules?.isMultiline(kind) ?? false) {
            insertText(ScreenplayParagraphs.lineSeparator, replacementRange: selectedRange())
        } else {
            insertNewline(sender)
        }
    }

    public override func insertNewlineIgnoringFieldEditor(_ sender: Any?) { insertLineBreak(sender) }

    public override func insertTab(_ sender: Any?) {
        if let next = rules?.kindAfterTab(currentKind(), text: currentText(), backward: false) {
            setKind(next)
        }
    }

    public override func insertBacktab(_ sender: Any?) {
        if let next = rules?.kindAfterTab(currentKind(), text: currentText(), backward: true) {
            setKind(next)
        }
    }

    // MARK: Suggestion keys

    public override func moveDown(_ sender: Any?) {
        if suggestions.move(by: 1) { redrawSuggestions() } else { super.moveDown(sender) }
    }

    public override func moveUp(_ sender: Any?) {
        if suggestions.move(by: -1) { redrawSuggestions() } else { super.moveUp(sender) }
    }

    public override func cancelOperation(_ sender: Any?) {
        if suggestions.dismiss() { redrawSuggestions() } else { onEscape?() }
    }

    /// The page sits in the middle of the view when the view is wider than the page.
    public override var textContainerOrigin: NSPoint {
        var origin = super.textContainerOrigin
        if let width = textContainer?.size.width, width < bounds.width {
            origin.x = ((bounds.width - width) / 2).rounded()
        }
        return origin
    }

    // MARK: Page break and custom elements

    /// Insert page break (Command+Return): an explicit page break after the current paragraph,
    /// then a new paragraph of the same element (an action after a heading or a page break).
    public func insertPageBreak() {
        guard isEditable else { return }
        suggestions.invalidateValues()
        let kind = currentKind()
        let custom = currentCustomId()
        let range = currentParagraphRange()
        let end = currentParagraphContentEnd()
        let followKind: ElementType = kind == .sceneHeading || kind == .pageBreak ? .action : kind
        let followCustom = followKind == .custom ? custom : nil
        let breakAttrs = attributes(for: .pageBreak, elementId: UUID().uuidString.lowercased(), custom: nil, sceneId: nil)
        let endsParagraph = attributes(for: kind, elementId: nil, custom: custom, sceneId: nil)
        let insert = NSMutableAttributedString()
        // The first newline ends the current paragraph (it keeps its own attributes), the second
        // ends the page break.
        var closing = endsParagraph
        if range.length > 0, let existing = textStorage?.attributes(at: range.location, effectiveRange: nil) { closing = existing }
        insert.append(NSAttributedString(string: "\n", attributes: closing))
        insert.append(NSAttributedString(string: "\n", attributes: breakAttrs))
        let at = NSRange(location: end, length: 0)
        guard shouldChangeText(in: at, replacementString: "\n\n") else { return }
        textStorage?.replaceCharacters(in: at, with: insert)
        didChangeText()
        // The caret goes into the new paragraph after the break.
        setSelectedRange(NSRange(location: end + 2, length: 0))
        applyToCaretParagraph(kind: followKind, custom: followCustom, newId: true)
        scrollRangeToVisible(selectedRange())
    }

    /// Turn the caret's paragraph into a custom element.
    public func setCustomElement(_ id: String) {
        guard customDefinitions.contains(where: { $0.id == id }) else { return }
        setKind(.custom, customElementId: id)
    }

    public override func performKeyEquivalent(with event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if window?.firstResponder === self, isEditable {
            if flags == .command, let chars = event.charactersIgnoringModifiers, let digit = Int(chars), (1...9).contains(digit),
               let kind = rules?.kind(forShortcutDigit: digit) {
                setKind(kind)
                return true
            }
            if flags == .command, event.keyCode == 36 || event.keyCode == 76 {
                insertPageBreak()
                return true
            }
            if flags == [.command, .option], let chars = event.charactersIgnoringModifiers, let digit = Int(chars), (1...9).contains(digit),
               let def = customDefinitions.first(where: { $0.shortcut == "Ctrl+Alt+\(digit)" || $0.shortcut == "Cmd+Option+\(digit)" }) {
                setKind(.custom, customElementId: def.id)
                return true
            }
        }
        return super.performKeyEquivalent(with: event)
    }

    public override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        drawScreenplayMarks()
    }

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        // The system restores its own spelling preference when the view joins a window.
        isContinuousSpellCheckingEnabled = true
    }

    public override func resignFirstResponder() -> Bool {
        suggestionPopover?.hide()
        return super.resignFirstResponder()
    }

    public override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        if became { refreshSuggestions(textChanged: false) }
        return became
    }

    public override func menu(for event: NSEvent) -> NSMenu? {
        let menu = super.menu(for: event)
        adjustSpellingMenu(menu, for: event)
        return menu
    }

    public override func didChangeText() {
        super.didChangeText()
        needsDisplay = true
        if !isLoading {
            screenplayDelegate?.screenplayTextDidChange(self)
            refreshSuggestions(textChanged: true)
        }
    }

    // A page break holds no text; a line break or paste that touches the script's structure
    // makes the cached suggestion values stale.
    public override func shouldChangeText(in affectedCharRange: NSRange, replacementString: String?) -> Bool {
        if !isLoading {
            let now = Date()
            if now.timeIntervalSince(lastEditAt) > Self.undoPauseSeconds { breakUndoCoalescing() }
            lastEditAt = now
            let ns = (textStorage?.string ?? "") as NSString
            let replaced = affectedCharRange.length > 0 && affectedCharRange.location + affectedCharRange.length <= ns.length
                ? ns.substring(with: affectedCharRange) : ""
            if (replacementString ?? "").contains("\n") || replaced.contains("\n") { suggestions.invalidateValues() }
            if let text = replacementString, !text.isEmpty, !text.contains("\n"), !replaced.contains("\n"),
               kind(atParagraphContaining: affectedCharRange.location) == .pageBreak {
                return false
            }
        }
        return super.shouldChangeText(in: affectedCharRange, replacementString: replacementString)
    }

    func kind(atParagraphContaining location: Int) -> ElementType {
        guard let storage = textStorage, storage.length > 0 else {
            return (typingAttributes[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
        }
        let ns = storage.string as NSString
        let loc = min(location, ns.length)
        let range = ns.paragraphRange(for: NSRange(location: loc, length: 0))
        if range.length == 0 || range.location >= storage.length {
            return (typingAttributes[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
        }
        return (storage.attribute(.ofKind, at: range.location, effectiveRange: nil) as? String).flatMap(ElementType.init(rawValue:)) ?? .action
    }

    public override func setSelectedRanges(_ ranges: [NSValue], affinity: NSSelectionAffinity, stillSelecting: Bool) {
        if !isLoading { captureTrailingAttributes() }
        super.setSelectedRanges(ranges, affinity: affinity, stillSelecting: stillSelecting)
        if !stillSelecting && !isLoading {
            notifyMove(force: false)
            refreshSuggestions(textChanged: false)
        }
    }

    func notifyMove(force: Bool) {
        let index = caretParagraphIndex
        guard force || index != lastParagraphIndex else { return }
        lastParagraphIndex = index
        let all = paragraphs()
        guard index < all.count else { return }
        // Typing at the start of an empty paragraph keeps its element.
        if let storage = textStorage, storage.length > 0 {
            let range = currentParagraphRange()
            if range.length > 0 {
                typingAttributes = storage.attributes(at: range.location, effectiveRange: nil)
            }
        }
        screenplayDelegate?.screenplayTextView(self, didMoveInto: all[index], index: index)
    }

    /// Custom definitions arrived or changed: restyle the custom paragraphs. Not an edit.
    private func restyleCustom() {
        guard let storage = textStorage, storage.length > 0 else { return }
        let ns = storage.string as NSString
        var location = 0
        isLoading = true
        defer { isLoading = false }
        storage.beginEditing()
        repeat {
            let range = ns.paragraphRange(for: NSRange(location: location, length: 0))
            if range.length > 0, (storage.attribute(.ofKind, at: range.location, effectiveRange: nil) as? String) == ElementType.custom.rawValue {
                let custom = storage.attribute(.ofCustomId, at: range.location, effectiveRange: nil) as? String
                let look = self.look(for: .custom, custom: custom)
                storage.addAttributes([.font: font(for: look), .paragraphStyle: paragraphStyle(for: look)], range: range)
            }
            location = NSMaxRange(range)
        } while location < ns.length
        storage.endEditing()
        needsDisplay = true
    }

    private func restyleAll() {
        updateContainerWidth()
        load(paragraphs())
    }

    /// Write ids back onto paragraphs (after duplicates from a paste or a split were renamed), so
    /// the next read gives the same ids. Not an edit: no undo step and no change notice.
    public func assign(ids: [String?]) {
        guard let storage = textStorage else { return }
        let ns = storage.string as NSString
        var location = 0
        var index = 0
        isLoading = true
        defer { isLoading = false }
        storage.beginEditing()
        repeat {
            let range = ns.paragraphRange(for: NSRange(location: location, length: 0))
            if index < ids.count, let id = ids[index], range.length > 0 {
                let kind = storage.attribute(.ofKind, at: range.location, effectiveRange: nil) as? String
                let key: NSAttributedString.Key = kind == ElementType.sceneHeading.rawValue ? .ofSceneId : .ofElementId
                storage.addAttribute(key, value: id, range: range)
            }
            location = NSMaxRange(range)
            index += 1
        } while location < ns.length
        storage.endEditing()
    }

    /// Scroll so that the paragraph at `index` is visible, and put the caret at its start.
    public func reveal(paragraph index: Int) {
        let ns = (textStorage?.string ?? "") as NSString
        var location = 0
        var count = 0
        while count < index && location < ns.length {
            let r = ns.paragraphRange(for: NSRange(location: location, length: 0))
            location = NSMaxRange(r)
            count += 1
        }
        setSelectedRange(NSRange(location: min(location, ns.length), length: 0))
        scrollRangeToVisible(NSRange(location: min(location, ns.length), length: 0))
    }
}

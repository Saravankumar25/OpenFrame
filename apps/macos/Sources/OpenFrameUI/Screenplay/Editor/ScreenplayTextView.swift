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

    public static let pageTextWidthInches: CGFloat = 6.0
    public static let pageLeftMarginInches: CGFloat = 1.5

    /// A text view on TextKit 2 with the screenplay defaults.
    public static func make() -> ScreenplayTextView {
        let view = ScreenplayTextView(usingTextLayoutManager: true)
        view.configure()
        return view
    }

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

    private func font(for look: ElementLook) -> NSFont {
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

    func attributes(for kind: ElementType, elementId: String?, custom: String?, sceneId: String?) -> [NSAttributedString.Key: Any] {
        let look = rules?.look(for: kind) ?? .standard(kind)
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
        guard let kind = kindRaw.flatMap(ElementType.init(rawValue:)),
              (rules?.look(for: kind) ?? .standard(kind)).uppercase else { return nil }
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
            text.append(NSAttributedString(string: "", attributes: typingAttributes))
        }
        textStorage?.setAttributedString(text)
        let location = min(caret.location, text.length)
        setSelectedRange(NSRange(location: location, length: 0))
        undoManager?.removeAllActions()
        lastParagraphIndex = -1
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
            let attrs = typingAttributes
            let kind = (attrs[.ofKind] as? String).flatMap(ElementType.init(rawValue:)) ?? .action
            out.append(EditorParagraph(kind: kind, text: "", elementId: attrs[.ofElementId] as? String))
        }
        return out
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

    private func currentParagraphRange() -> NSRange {
        let ns = (textStorage?.string ?? "") as NSString
        return ns.paragraphRange(for: NSRange(location: min(selectedRange().location, ns.length), length: 0))
    }

    private func currentKind() -> ElementType {
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

    private func currentText() -> String {
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

    public override func insertNewline(_ sender: Any?) {
        let kind = currentKind()
        if currentText().isEmpty, selectedRange().length == 0 {
            if let converted = rules?.kindForEmptyEnter(kind) { setKind(converted) }
            return
        }
        let next = rules?.kindAfterEnter(kind, text: currentText()) ?? .action
        super.insertNewline(sender)
        // The new paragraph is a new element with its own id.
        let attrs = attributes(for: next, elementId: UUID().uuidString.lowercased(), custom: nil, sceneId: nil)
        typingAttributes = attrs
        let range = currentParagraphRange()
        if range.length > 0 {
            textStorage?.setAttributes(attrs, range: range)
        }
        notifyMove(force: true)
    }

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

    public override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command,
           let chars = event.charactersIgnoringModifiers, let digit = Int(chars), (1...9).contains(digit),
           window?.firstResponder === self, isEditable,
           let kind = rules?.kind(forShortcutDigit: digit) {
            setKind(kind)
            return true
        }
        return super.performKeyEquivalent(with: event)
    }

    public override func didChangeText() {
        super.didChangeText()
        if !isLoading { screenplayDelegate?.screenplayTextDidChange(self) }
    }

    public override func setSelectedRanges(_ ranges: [NSValue], affinity: NSSelectionAffinity, stillSelecting: Bool) {
        super.setSelectedRanges(ranges, affinity: affinity, stillSelecting: stillSelecting)
        if !stillSelecting && !isLoading { notifyMove(force: false) }
    }

    private func notifyMove(force: Bool) {
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

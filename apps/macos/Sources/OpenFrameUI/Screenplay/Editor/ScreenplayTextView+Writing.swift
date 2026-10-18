// The writing-comfort parts of the screenplay text view: the suggestion list, scene numbers and
// page breaks drawn in the margin, the position on the page, and spelling that also teaches the
// core's dictionary. The element and keyboard rules stay in `ScreenplayTextView.swift`.

import AppKit
import OpenFrameCore

extension ScreenplayTextView {
    // MARK: Suggestions

    /// Read the caret's element and recompute the list. Cheap: the script values are kept while
    /// the writer stays in the same element.
    func refreshSuggestions(textChanged: Bool) {
        guard !isLoading else { return }
        let range = currentParagraphRange()
        let offset = selectedRange().location - range.location
        let attrs: [NSAttributedString.Key: Any] = range.length > 0
            ? (textStorage?.attributes(at: range.location, effectiveRange: nil) ?? [:]) : typingAttributes
        let blockId = (attrs[.ofElementId] as? String) ?? (attrs[.ofSceneId] as? String)
        let changed = suggestions.update(
            kind: currentKind(), text: currentText(), offset: offset, blockId: blockId,
            selectionIsEmpty: selectedRange().length == 0, canEdit: isEditable, sources: suggestionSources,
            textChanged: textChanged,
            values: { [self] in
                ScreenplaySuggest.scriptValues(ScreenplayParagraphs.blocks(paragraphs()), without: caretParagraphIndex)
            })
        if changed { redrawSuggestions() }
    }

    func redrawSuggestions() {
        guard suggestions.isOpen, window?.firstResponder === self, let ctx = suggestions.context else {
            suggestionPopover?.hide()
            return
        }
        if suggestionPopover == nil {
            let popover = SuggestionPopover(textView: self)
            popover.onPick = { [weak self] i in
                guard let self else { return }
                _ = self.accept(suggestion: i, moveOn: false)
                self.window?.makeFirstResponder(self)
            }
            suggestionPopover = popover
        }
        let at = NSRange(location: currentParagraphRange().location + ctx.from, length: 0)
        var screen = NSRect.zero
        screen = firstRect(forCharacterRange: at, actualRange: nil)
        var rect = NSRect(x: 0, y: 0, width: 2, height: 16)
        if let window, !screen.isEmpty || screen.origin != .zero {
            rect = convert(window.convertFromScreen(screen), from: nil)
            rect.size.width = max(rect.width, 2)
        }
        suggestionPopover?.show(items: suggestions.items, index: suggestions.index, caretRect: rect)
    }

    /// The list as the person sees it, for tests and VoiceOver.
    public var suggestionLabels: [String] { suggestions.isOpen ? suggestions.items.map(\.label) : [] }
    public var suggestionHighlight: Int { suggestions.isOpen ? suggestions.index : -1 }

    /// Return on a highlighted item. `moveOn`: a character name also moves on to Dialogue.
    @discardableResult
    func acceptHighlightedSuggestion(moveOn: Bool) -> Bool {
        guard suggestions.isOpen, suggestions.index >= 0, !hasMarkedText() else { return false }
        return accept(suggestion: suggestions.index, moveOn: moveOn)
    }

    /// Accept item `index`: replace only the typed part, or for "Create New" keep the text as typed
    /// and tell the owner to add the name to the project.
    @discardableResult
    func accept(suggestion index: Int, moveOn: Bool) -> Bool {
        guard suggestions.items.indices.contains(index), let ctx = suggestions.context else { return false }
        let item = suggestions.items[index]
        let start = currentParagraphRange().location
        if let create = item.create {
            let movesOn = moveOn && create == .character
            onCreateSuggestion?(create, ctx.query.trimmingCharacters(in: .whitespaces).uppercased(), movesOn)
            if movesOn {
                performEnter(nil)
            } else {
                _ = suggestions.dismiss()
                redrawSuggestions()
            }
            return true
        }
        let target = NSRange(location: start + ctx.from, length: max(0, ctx.to - ctx.from))
        let ns = (textStorage?.string ?? "") as NSString
        if NSMaxRange(target) <= ns.length, ns.substring(with: target) != item.insert {
            insertText(item.insert, replacementRange: target)
        }
        if moveOn && ctx.kind == .character {
            beforeSuggestionMoveOn?()
            performEnter(nil)
        }
        return true
    }

    // MARK: Position on the page

    /// The scene the caret is in and how far through it the caret is (0 to 1).
    public func caretSceneProgress() -> (sceneId: String, fraction: Double)? {
        guard let storage = textStorage, storage.length > 0 else { return nil }
        let ns = storage.string as NSString
        let caret = min(selectedRange().location, ns.length)
        func heading(at location: Int) -> String? {
            guard location < storage.length else { return nil }
            let a = storage.attributes(at: location, effectiveRange: nil)
            guard (a[.ofKind] as? String) == ElementType.sceneHeading.rawValue else { return nil }
            return (a[.ofSceneId] as? String) ?? (a[.ofElementId] as? String)
        }
        // Walk back to the heading above, then forward to the next one.
        var start = ns.paragraphRange(for: NSRange(location: min(caret, max(0, ns.length - 1)), length: 0))
        var sceneId = heading(at: start.location)
        while sceneId == nil, start.location > 0 {
            start = ns.paragraphRange(for: NSRange(location: start.location - 1, length: 0))
            sceneId = heading(at: start.location)
        }
        guard let sceneId else { return nil }
        var end = ns.length
        var next = NSMaxRange(ns.paragraphRange(for: NSRange(location: start.location, length: 0)))
        while next < ns.length {
            if heading(at: next) != nil {
                end = next
                break
            }
            next = NSMaxRange(ns.paragraphRange(for: NSRange(location: next, length: 0)))
        }
        let span = max(1, end - start.location)
        return (sceneId, min(1, max(0, Double(caret - start.location) / Double(span))))
    }

    // MARK: Margin marks

    func drawScreenplayMarks() {
        guard let tlm = textLayoutManager else { return }
        let origin = textContainerOrigin
        let visible = visibleRect.insetBy(dx: 0, dy: -120)
        let ppi = pointsPerInch
        let start = tlm.textViewportLayoutController.viewportRange?.location ?? tlm.documentRange.location
        let labelFont = NSFont(descriptor: Self.screenplayFont.fontDescriptor, size: 12 * ppi / 72) ?? Self.screenplayFont
        let labelAttrs: [NSAttributedString.Key: Any] = [.font: labelFont, .foregroundColor: NSColor.secondaryLabelColor]
        tlm.enumerateTextLayoutFragments(from: start, options: []) { fragment in
            let frame = fragment.layoutFragmentFrame.offsetBy(dx: origin.x, dy: origin.y)
            if frame.minY > visible.maxY { return false }
            if frame.maxY < visible.minY { return true }
            guard let paragraph = fragment.textElement as? NSTextParagraph, paragraph.attributedString.length > 0 else { return true }
            let attrs = paragraph.attributedString.attributes(at: 0, effectiveRange: nil)
            let kind = (attrs[.ofKind] as? String).flatMap(ElementType.init(rawValue:))
            guard let line = fragment.textLineFragments.first else { return true }
            let lineRect = line.typographicBounds.offsetBy(dx: frame.minX, dy: frame.minY)
            if kind == .sceneHeading, let id = attrs[.ofSceneId] as? String, let label = self.sceneLabels[id] {
                let text = NSAttributedString(string: label, attributes: labelAttrs)
                let width = text.size().width
                let left = origin.x + 0.85 * ppi
                text.draw(at: NSPoint(x: left - width, y: lineRect.midY - text.size().height / 2))
                text.draw(at: NSPoint(x: origin.x + 7.1 * ppi, y: lineRect.midY - text.size().height / 2))
            } else if kind == .pageBreak {
                self.drawPageBreak(in: lineRect, origin: origin)
            }
            return true
        }
    }

    private func drawPageBreak(in line: NSRect, origin: NSPoint) {
        let ppi = pointsPerInch
        let x0 = origin.x + 1.0 * ppi
        let x1 = origin.x + 7.0 * ppi
        let path = NSBezierPath()
        path.move(to: NSPoint(x: x0, y: line.midY))
        path.line(to: NSPoint(x: x1, y: line.midY))
        path.setLineDash([5, 4], count: 2, phase: 0)
        path.lineWidth = 1
        NSColor.tertiaryLabelColor.setStroke()
        path.stroke()
        let caption = NSAttributedString(string: "PAGE BREAK", attributes: [
            .font: NSFont.systemFont(ofSize: 9 * ppi / 72, weight: .medium), .foregroundColor: NSColor.secondaryLabelColor,
        ])
        let size = caption.size()
        let box = NSRect(x: (x0 + x1) / 2 - size.width / 2 - 6, y: line.midY - size.height / 2, width: size.width + 12, height: size.height)
        backgroundColor.setFill()
        box.fill()
        caption.draw(at: NSPoint(x: box.minX + 6, y: box.minY))
    }

    // MARK: Spelling

    /// Names that are spelled right on purpose (characters, locations) are never underlined.
    public func setKnownNames(_ names: [String]) {
        var words = Set<String>()
        for name in names {
            for part in name.split(whereSeparator: { !$0.isLetter && !$0.isNumber && $0 != "'" }) {
                words.insert(String(part))
                words.insert(String(part).capitalized)
                words.insert(String(part).lowercased())
            }
        }
        ignoredWordList = Array(words)
        NSSpellChecker.shared.setIgnoredWords(ignoredWordList, inSpellDocumentWithTag: spellCheckerDocumentTag)
    }

    /// Learn a word: the system dictionary and the core's dictionary, so it is known on both platforms.
    public func learnSpelling(word: String) {
        let trimmed = word.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        NSSpellChecker.shared.learnWord(trimmed)
        // The checker may drop the document's ignore list when the dictionary changes.
        NSSpellChecker.shared.setIgnoredWords(ignoredWordList, inSpellDocumentWithTag: spellCheckerDocumentTag)
        onLearnSpelling?(trimmed)
    }

    @objc func learnSpellingFromMenu(_ sender: NSMenuItem) {
        if let word = sender.representedObject as? String { learnSpelling(word: word) }
    }

    /// The word under the click when it is misspelled, else nil.
    func misspelledWord(at event: NSEvent) -> String? {
        guard isContinuousSpellCheckingEnabled else { return nil }
        let point = convert(event.locationInWindow, from: nil)
        let index = characterIndexForInsertion(at: point)
        let ns = (textStorage?.string ?? "") as NSString
        guard index <= ns.length, ns.length > 0 else { return nil }
        let range = selectionRange(forProposedRange: NSRange(location: min(index, ns.length - 1), length: 0), granularity: .selectByWord)
        guard range.length > 0, NSMaxRange(range) <= ns.length else { return nil }
        let word = ns.substring(with: range)
        guard word.rangeOfCharacter(from: .letters) != nil else { return nil }
        let found = NSSpellChecker.shared.checkSpelling(
            of: word, startingAt: 0, language: nil, wrap: false, inSpellDocumentWithTag: spellCheckerDocumentTag, wordCount: nil)
        return found.location == NSNotFound ? nil : word
    }

    /// "Learn Spelling" in the context menu teaches the core's dictionary as well.
    func adjustSpellingMenu(_ menu: NSMenu?, for event: NSEvent) {
        guard let menu, let word = misspelledWord(at: event) else { return }
        let learnTitle = NSLocalizedString("Learn Spelling", comment: "")
        var replaced = false
        for item in menu.items where item.title == learnTitle || item.action == NSSelectorFromString("learnSpelling:") {
            item.title = learnTitle
            item.target = self
            item.action = #selector(learnSpellingFromMenu(_:))
            item.representedObject = word
            replaced = true
        }
        if !replaced {
            let item = NSMenuItem(title: learnTitle, action: #selector(learnSpellingFromMenu(_:)), keyEquivalent: "")
            item.target = self
            item.representedObject = word
            menu.insertItem(item, at: 0)
        }
    }
}

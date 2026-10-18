// The suggestion list drawn as a popover under the caret. It never takes the keyboard: the text
// view keeps the first responder and handles arrows, Return and Escape itself. Rows are plain
// views (a click accepts), the list is announced to VoiceOver, and the popover does not animate
// when Reduce Motion is on.

import AppKit
import OpenFrameCore

@MainActor
final class SuggestionPopover: NSObject {
    private let popover = NSPopover()
    private let listView = SuggestionListView()
    private weak var textView: NSTextView?
    var onPick: ((Int) -> Void)? {
        get { listView.onPick }
        set { listView.onPick = newValue }
    }

    init(textView: NSTextView) {
        self.textView = textView
        super.init()
        let controller = NSViewController()
        controller.view = listView
        popover.contentViewController = controller
        popover.behavior = .applicationDefined
        popover.delegate = nil
    }

    var isShown: Bool { popover.isShown }

    func show(items: [SuggestItem], index: Int, caretRect: NSRect) {
        guard let textView, textView.window?.isVisible == true else { return }
        let previous = listView.index
        listView.set(items: items, index: index)
        popover.animates = !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        popover.contentSize = listView.preferredSize
        let rect = caretRect.insetBy(dx: 0, dy: -2)
        if popover.isShown {
            popover.positioningRect = rect
        } else {
            popover.show(relativeTo: rect, of: textView, preferredEdge: .maxY)
        }
        if previous != index, items.indices.contains(index) {
            let item = items[index]
            NSAccessibility.post(
                element: textView, notification: .announcementRequested,
                userInfo: [.announcement: "\(item.label), \(item.source.rawValue)", .priority: NSAccessibilityPriorityLevel.medium.rawValue])
        }
    }

    /// Closing at once: a closing animation that is still running swallows the next show, and the
    /// list would stay hidden while the writer types the next word.
    func hide() {
        guard popover.isShown else { return }
        let animated = popover.animates
        popover.animates = false
        popover.close()
        popover.animates = animated
    }
}

/// The rows: the label on the left, where it comes from on the right.
@MainActor
final class SuggestionListView: NSView {
    private(set) var items: [SuggestItem] = []
    private(set) var index = -1
    var onPick: ((Int) -> Void)?
    static let rowHeight: CGFloat = 24

    override var isFlipped: Bool { true }

    var preferredSize: NSSize {
        NSSize(width: 280, height: CGFloat(max(items.count, 1)) * Self.rowHeight + 8)
    }

    func set(items: [SuggestItem], index: Int) {
        self.items = items
        self.index = index
        setAccessibilityElement(true)
        setAccessibilityRole(.list)
        setAccessibilityLabel("Suggestions")
        setAccessibilityIdentifier("screenplay-suggestions")
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        let labelFont = NSFont.systemFont(ofSize: 13)
        let sourceFont = NSFont.systemFont(ofSize: 11)
        for (i, item) in items.enumerated() {
            let row = NSRect(x: 4, y: 4 + CGFloat(i) * Self.rowHeight, width: bounds.width - 8, height: Self.rowHeight)
            let selected = i == index
            if selected {
                NSColor.selectedContentBackgroundColor.setFill()
                NSBezierPath(roundedRect: row, xRadius: 5, yRadius: 5).fill()
            }
            let ink: NSColor = selected ? .alternateSelectedControlTextColor : .labelColor
            let quiet: NSColor = selected ? .alternateSelectedControlTextColor : .secondaryLabelColor
            let sourceText = NSAttributedString(string: item.source.rawValue, attributes: [.font: sourceFont, .foregroundColor: quiet])
            let sourceWidth = sourceText.size().width
            sourceText.draw(at: NSPoint(x: row.maxX - sourceWidth - 8, y: row.midY - sourceText.size().height / 2))
            let label = NSAttributedString(string: item.label, attributes: [.font: labelFont, .foregroundColor: ink])
            let labelRect = NSRect(x: row.minX + 8, y: row.midY - label.size().height / 2, width: row.width - sourceWidth - 24, height: label.size().height)
            label.draw(with: labelRect, options: [.truncatesLastVisibleLine, .usesLineFragmentOrigin])
        }
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        let i = Int((point.y - 4) / Self.rowHeight)
        if items.indices.contains(i) { onPick?(i) }
    }

    override func accessibilityChildren() -> [Any]? {
        items.enumerated().map { i, item in
            let element = NSAccessibilityElement()
            element.setAccessibilityRole(.row)
            element.setAccessibilityLabel("\(item.label), \(item.source.rawValue)")
            element.setAccessibilityParent(self)
            element.setAccessibilitySelected(i == index)
            element.setAccessibilityFrameInParentSpace(NSRect(x: 4, y: 4 + CGFloat(i) * Self.rowHeight, width: bounds.width - 8, height: Self.rowHeight))
            return element
        }
    }
}

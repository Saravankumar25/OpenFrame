// Shared pieces of the screenplay panels (Mac phase M4, editor B): where the caret is, how a
// comment finds its paragraph, plain-language dates, and the small views every panel uses.
// The core decides everything; nothing here holds a rule.

import AppKit
import SwiftUI
import OpenFrameCore

// MARK: Caret

/// What the caret is on, read from the editor's paragraphs.
struct ScreenplayCaretContext: Equatable {
    var paragraphIndex: Int
    /// The scene the caret is in (the nearest scene heading above, or at, the caret).
    var sceneId: String?
    /// The element under the caret. Nil on a scene heading.
    var elementId: String?
    var kind: ElementType
    var text: String
    /// The selection inside this paragraph, in Unicode scalars (how the core counts), when the
    /// selection is not empty and stays in one paragraph.
    var selection: Range<Int>?
    var selectedText: String?
}

/// Where a new comment attaches.
enum ScreenplayCommentTarget: Equatable {
    case scene(id: String)
    case element(id: String, start: Int, end: Int, quote: String)
    case draft

    var quote: String? {
        if case let .element(_, _, _, quote) = self { return quote }
        return nil
    }
}

enum ScreenplayCaret {
    /// The scene a paragraph belongs to.
    static func sceneId(in paragraphs: [EditorParagraph], at index: Int) -> String? {
        guard !paragraphs.isEmpty else { return nil }
        var i = min(max(index, 0), paragraphs.count - 1)
        while i >= 0 {
            if paragraphs[i].kind == .sceneHeading, let id = paragraphs[i].sceneId { return id }
            i -= 1
        }
        return nil
    }

    static func context(
        paragraphs: [EditorParagraph], index: Int, selection: Range<Int>? = nil, selectedText: String? = nil
    ) -> ScreenplayCaretContext? {
        guard !paragraphs.isEmpty else { return nil }
        let i = min(max(index, 0), paragraphs.count - 1)
        let p = paragraphs[i]
        return ScreenplayCaretContext(
            paragraphIndex: i,
            sceneId: sceneId(in: paragraphs, at: i),
            elementId: p.kind == .sceneHeading ? nil : p.elementId,
            kind: p.kind,
            text: p.text,
            selection: selection,
            selectedText: selectedText)
    }

    /// The caret of a live editor, with its selection when the selection stays inside one paragraph.
    @MainActor
    static func current(in view: ScreenplayTextView) -> ScreenplayCaretContext? {
        let paragraphs = view.paragraphs()
        let index = view.caretParagraphIndex
        var selection: Range<Int>?
        var selected: String?
        let range = view.selectedRange()
        if range.length > 0 {
            let ns = view.string as NSString
            let para = ns.paragraphRange(for: NSRange(location: min(range.location, ns.length), length: 0))
            if NSMaxRange(range) <= NSMaxRange(para) {
                let before = ns.substring(with: NSRange(location: para.location, length: range.location - para.location))
                let text = ns.substring(with: range).replacingOccurrences(of: "\n", with: "")
                let start = before.unicodeScalars.count
                let length = text.unicodeScalars.count
                if length > 0 {
                    selection = start..<(start + length)
                    selected = text
                }
            }
        }
        return context(paragraphs: paragraphs, index: index, selection: selection, selectedText: selected)
    }

    /// What "Add Comment" attaches to for the caret: the selected text, else the whole element
    /// when it has text, else the scene.
    static func commentTarget(for c: ScreenplayCaretContext) -> ScreenplayCommentTarget? {
        guard let sceneId = c.sceneId else { return nil }
        if c.kind == .sceneHeading || c.kind == .pageBreak { return .scene(id: sceneId) }
        guard let element = c.elementId else { return .scene(id: sceneId) }
        if let sel = c.selection, let quote = c.selectedText {
            return .element(id: element, start: sel.lowerBound, end: sel.upperBound, quote: quote)
        }
        let length = c.text.unicodeScalars.count
        if length > 0 { return .element(id: element, start: 0, end: length, quote: c.text) }
        return .scene(id: sceneId)
    }

    static func paragraphIndex(ofScene sceneId: String, in paragraphs: [EditorParagraph]) -> Int? {
        paragraphs.firstIndex { $0.kind == .sceneHeading && $0.sceneId == sceneId }
    }

    static func paragraphIndex(ofElement elementId: String, in paragraphs: [EditorParagraph]) -> Int? {
        paragraphs.firstIndex { $0.kind != .sceneHeading && $0.elementId == elementId }
    }

    /// The paragraph a comment points at: its element, else its scene, else the nearest surviving
    /// scene. Nil for a comment on the whole draft.
    static func paragraphIndex(for comment: CommentDto, in paragraphs: [EditorParagraph]) -> Int? {
        if comment.targetType == "screenplay_element", !comment.targetDeleted,
           let i = paragraphIndex(ofElement: comment.targetId, in: paragraphs) {
            return i
        }
        if comment.targetType == "screenplay_scene", !comment.targetDeleted,
           let i = paragraphIndex(ofScene: comment.targetId, in: paragraphs) {
            return i
        }
        for id in [comment.sceneId, comment.nearestSceneId] {
            if let id, let i = paragraphIndex(ofScene: id, in: paragraphs) { return i }
        }
        return nil
    }
}

// MARK: Wording

enum PanelText {
    static func quoted(_ s: String) -> String { "\u{201C}\(s)\u{201D}" }

    static func date(_ ms: Double) -> String {
        Date(timeIntervalSince1970: ms / 1000).formatted(date: .abbreviated, time: .omitted)
    }

    /// "Today 10:39", "Yesterday 21:12", "26 Sep 2026, 10:39".
    static func when(_ ms: Double, now: Date = Date()) -> String {
        let d = Date(timeIntervalSince1970: ms / 1000)
        let time = d.formatted(date: .omitted, time: .shortened)
        let cal = Calendar.current
        if cal.isDate(d, inSameDayAs: now) { return "Today \(time)" }
        if let y = cal.date(byAdding: .day, value: -1, to: now), cal.isDate(d, inSameDayAs: y) { return "Yesterday \(time)" }
        return "\(d.formatted(date: .abbreviated, time: .omitted)), \(time)"
    }

    static func ago(_ ms: Double, now: Date = Date()) -> String {
        let diff = max(0, now.timeIntervalSince1970 * 1000 - ms)
        if diff < 60_000 { return "just now" }
        if diff < 3_600_000 { return "\(Int((diff / 60_000).rounded())) min ago" }
        if diff < 86_400_000 { return "\(Int((diff / 3_600_000).rounded())) h ago" }
        let days = Int((diff / 86_400_000).rounded())
        if days == 1 { return "yesterday" }
        if days < 7 { return "\(days) days ago" }
        return date(ms)
    }

    static func plural(_ n: Int, _ one: String, _ many: String? = nil) -> String {
        "\(n) \(n == 1 ? one : (many ?? one + "s"))"
    }

    /// The label of a draft's state: Locked, In review, the revision label and colour, or Draft.
    static func status(_ d: ScreenplayDraftDto) -> String {
        switch d.status {
        case .locked: return "Locked"
        case .review: return "In review"
        case .revision:
            guard let label = d.revisionLabel, !label.isEmpty else { return "Revision" }
            if let c = d.revisionColor, !c.isEmpty { return "\(label) \u{00B7} \(c)" }
            return label
        case .draft: return "Draft"
        }
    }

    /// The standard industry revision page colours.
    static let revisionColours = ["Blue", "Pink", "Yellow", "Green"]

    static func swatch(_ colour: String?) -> Color? {
        switch colour {
        case "Blue": Color(red: 0.56, green: 0.70, blue: 0.94)
        case "Pink": Color(red: 0.95, green: 0.65, blue: 0.76)
        case "Yellow": Color(red: 0.95, green: 0.89, blue: 0.48)
        case "Green": Color(red: 0.62, green: 0.84, blue: 0.65)
        default: nil
        }
    }

    /// The message to show for a failure: validation messages are plain already.
    static func message(_ error: Error, fallback: String) -> String {
        let f = CoreFailure(asCore: error)
        return f.message.isEmpty ? fallback : f.message
    }

    static func isValidation(_ error: Error) -> Bool {
        CoreFailure(asCore: error).code.hasPrefix("validation")
    }
}

// MARK: Small views

/// A dialog: title, optional sub line, content, buttons at the bottom right.
struct PanelSheetFrame<Content: View, Footer: View>: View {
    let title: String
    var sub: String?
    var width: CGFloat = 480
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            VStack(spacing: PageSpacing.tight) {
                Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
                if let sub {
                    Text(sub).font(.callout).foregroundStyle(theme["muted"])
                        .multilineTextAlignment(.center)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .frame(maxWidth: .infinity)
            content()
            HStack(spacing: PageSpacing.small) {
                Spacer(minLength: 0)
                footer()
            }
        }
        .padding(PageSpacing.large)
        .frame(width: width)
        .background(theme["panel"])
    }
}

/// The centred title of a dialog (and its optional sub line).
struct PanelTitle: View {
    let title: String
    var sub: String?
    @Environment(\.ofTheme) private var theme

    init(_ title: String, sub: String? = nil) { self.title = title; self.sub = sub }

    var body: some View {
        VStack(spacing: PageSpacing.tight) {
            Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            if let sub {
                Text(sub).font(.callout).foregroundStyle(theme["muted"])
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .frame(maxWidth: .infinity)
    }
}

struct PanelBanner: View {
    enum Tone { case info, warn, err, ok }
    let tone: Tone
    let text: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let (bg, fg, symbol): (String, String, String) = switch tone {
        case .info: ("blue-soft", "blue-t", "info.circle")
        case .warn: ("yellow-soft", "yellow-t", "exclamationmark.triangle")
        case .err: ("red-soft", "red-t", "xmark.octagon")
        case .ok: ("green-soft", "green-t", "checkmark.circle")
        }
        Label {
            Text(text).fixedSize(horizontal: false, vertical: true)
        } icon: {
            Image(systemName: symbol)
        }
        .font(.callout)
        .foregroundStyle(theme[fg])
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme[bg], in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
    }
}

/// A short label in a capsule: a status or a count.
struct PanelChip: View {
    enum Tone { case plain, green, blue, yellow, red, purple, accent }
    let text: String
    var tone: Tone = .plain
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let (bg, fg): (String, String) = switch tone {
        case .plain: ("surface2", "ink2")
        case .green: ("green-soft", "green-t")
        case .blue: ("blue-soft", "blue-t")
        case .yellow: ("yellow-soft", "yellow-t")
        case .red: ("red-soft", "red-t")
        case .purple: ("purple-soft", "purple-t")
        case .accent: ("accent-soft", "accent-ink")
        }
        Text(text)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(theme[bg], in: Capsule())
            .foregroundStyle(theme[fg])
    }
}

/// A field with a label above it.
struct PanelField<Content: View>: View {
    let label: String
    var hint: String?
    @ViewBuilder var content: () -> Content
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
            content()
            if let hint { Text(hint).font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true) }
        }
    }
}

/// A multi-line text field with a thin border (a TextEditor draws none).
struct PanelTextArea: View {
    let label: String
    @Binding var text: String
    var minHeight: CGFloat = 64
    @Environment(\.ofTheme) private var theme

    var body: some View {
        TextEditor(text: $text)
            .font(.body)
            .scrollContentBackground(.hidden)
            .padding(4)
            .frame(minHeight: minHeight)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
            .overlay(RoundedRectangle(cornerRadius: 6).stroke(theme["line"]))
            .accessibilityLabel(label)
    }
}

// How an assistant answer is drawn (Windows: ai/answerText.tsx): paragraphs on blank lines, simple
// lists, and the "Idea:" label in bold. The answer is plain text. Every piece is drawn with
// Text(verbatim:), so nothing in an answer is read as Markdown, a link or markup.

import Foundation
import SwiftUI

enum AIAnswerBlock: Equatable {
    case paragraph([String])
    case bullets([String])
    case numbered([String])
}

enum AIAnswerText {
    private static let bullet = try! NSRegularExpression(pattern: #"^\s{0,3}[-*\x{2022}]\s+(.*\S)\s*$"#)
    private static let numbered = try! NSRegularExpression(pattern: #"^\s{0,3}\d{1,3}[.)]\s+(.*\S)\s*$"#)
    private static let idea = try! NSRegularExpression(pattern: #"^(\s*)(Idea(?:\s+\d+)?:)(\s*)(.*)$"#, options: [.dotMatchesLineSeparators])

    /// Split an answer into blocks. A line break inside a paragraph stays a line break.
    static func parse(_ text: String) -> [AIAnswerBlock] {
        var blocks: [AIAnswerBlock] = []
        func push(_ kind: Character, _ line: String) {
            switch (kind, blocks.last) {
            case ("p", .paragraph(let lines)?): blocks[blocks.count - 1] = .paragraph(lines + [line])
            case ("p", _): blocks.append(.paragraph([line]))
            case ("u", .bullets(let items)?): blocks[blocks.count - 1] = .bullets(items + [line])
            case ("u", _): blocks.append(.bullets([line]))
            case ("o", .numbered(let items)?): blocks[blocks.count - 1] = .numbered(items + [line])
            default: blocks.append(.numbered([line]))
            }
        }
        let normalised = text.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
        for raw in normalised.components(separatedBy: "\n") {
            if raw.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                // A blank line ends the paragraph or list, so the next line starts a new block.
                blocks.append(.paragraph([]))
            } else if let item = firstGroup(bullet, raw) {
                push("u", item)
            } else if let item = firstGroup(numbered, raw) {
                push("o", item)
            } else {
                push("p", trimEnd(raw))
            }
        }
        return blocks.filter { $0 != .paragraph([]) }
    }

    /// One run of a line: plain, or the bold "Idea:" label.
    struct Run: Equatable {
        var text: String
        var bold = false
    }

    /// One line of an answer: an "Idea:" label in bold, the rest as written.
    static func runs(_ line: String) -> [Run] {
        let ns = line as NSString
        guard let m = idea.firstMatch(in: line, range: NSRange(location: 0, length: ns.length)) else { return [Run(text: line)] }
        let parts = (1...4).map { ns.substring(with: m.range(at: $0)) }
        return [Run(text: parts[0]), Run(text: parts[1], bold: true), Run(text: parts[2] + parts[3])].filter { !$0.text.isEmpty }
    }

    /// The line as one Text, verbatim.
    static func text(_ line: String) -> Text {
        runs(line).reduce(Text(verbatim: "")) { sum, run in
            sum + (run.bold ? Text(verbatim: run.text).bold() : Text(verbatim: run.text))
        }
    }

    private static func firstGroup(_ re: NSRegularExpression, _ s: String) -> String? {
        let ns = s as NSString
        guard let m = re.firstMatch(in: s, range: NSRange(location: 0, length: ns.length)) else { return nil }
        return ns.substring(with: m.range(at: 1))
    }

    private static func trimEnd(_ s: String) -> String {
        var out = Substring(s)
        while let last = out.last, last.isWhitespace { out.removeLast() }
        return String(out)
    }
}

/// The answer, drawn block by block.
struct AIAnswerTextView: View {
    let text: String
    var color: Color?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(AIAnswerText.parse(text).enumerated()), id: \.offset) { _, block in
                switch block {
                case .paragraph(let lines):
                    lines.enumerated().reduce(Text(verbatim: "")) { sum, pair in
                        sum + (pair.offset > 0 ? Text(verbatim: "\n") : Text(verbatim: "")) + AIAnswerText.text(pair.element)
                    }
                    .fixedSize(horizontal: false, vertical: true)
                case .bullets(let items):
                    list(items) { _ in "\u{2022}" }
                case .numbered(let items):
                    list(items) { "\($0 + 1)." }
                }
            }
        }
        .lineSpacing(2)
        .foregroundStyle(color ?? theme["ink"])
        .textSelection(.enabled)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func list(_ items: [String], marker: @escaping (Int) -> String) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            ForEach(Array(items.enumerated()), id: \.offset) { i, item in
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text(verbatim: marker(i)).monospacedDigit()
                    AIAnswerText.text(item).fixedSize(horizontal: false, vertical: true)
                }
            }
        }
        .padding(.leading, 4)
    }
}

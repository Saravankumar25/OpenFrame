// Answers show line breaks, simple lists and the "Idea:" label, drawn as plain text (Windows:
// ai/answerText.test.tsx, every case).

import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@Suite("AI answer text")
struct AIAnswerTextTests {
    // MARK: parseAnswer

    @Test func makesAParagraphOfEachRunOfLinesAndStartsANewOneOnABlankLine() {
        #expect(AIAnswerText.parse("One.\nStill one.\n\nTwo.") == [.paragraph(["One.", "Still one."]), .paragraph(["Two."])])
    }

    @Test func readsLinesStartingWithADashAsBulletsAndWithANumberAsANumberedList() {
        #expect(AIAnswerText.parse("The scenes:\n- Kitchen\n- Garden\n\n1. First\n2) Second") == [
            .paragraph(["The scenes:"]),
            .bullets(["Kitchen", "Garden"]),
            .numbered(["First", "Second"]),
        ])
        // The other bullet marks the answers use.
        #expect(AIAnswerText.parse("* One\n\u{2022} Two") == [.bullets(["One", "Two"])])
    }

    @Test func doesNotTreatADashOrNumberInsideALineAsAList() {
        #expect(AIAnswerText.parse("Scene 1. Devi - alone, 3. maybe") == [.paragraph(["Scene 1. Devi - alone, 3. maybe"])])
    }

    @Test func copesWithEmptyTextWindowsLineEndsAndTrailingBlankLines() {
        #expect(AIAnswerText.parse("").isEmpty)
        #expect(AIAnswerText.parse("\n\n").isEmpty)
        #expect(AIAnswerText.parse("A\r\n- b\r\n\r\n") == [.paragraph(["A"]), .bullets(["b"])])
    }

    // MARK: AnswerText

    @Test func drawsParagraphsABulletListAndANumberedList() {
        let blocks = AIAnswerText.parse("Two places.\n\n- Kitchen\n- Garden\n\n1. Open\n2. Close")
        #expect(blocks.filter { if case .paragraph = $0 { true } else { false } }.count == 1)
        #expect(blocks.contains(.bullets(["Kitchen", "Garden"])))
        #expect(blocks.contains(.numbered(["Open", "Close"])))
    }

    @Test func keepsALineBreakInsideAParagraph() {
        #expect(AIAnswerText.parse("First line\nSecond line") == [.paragraph(["First line", "Second line"])])
    }

    @Test func writesTheIdeaLabelInBoldAndNothingElse() {
        let lines = ["Idea: let the kite fall.", "Idea: cut to the station.", "Not an Idea: here"]
        let bold = lines.flatMap(AIAnswerText.runs).filter(\.bold).map(\.text)
        #expect(bold == ["Idea:", "Idea:"])
        #expect(AIAnswerText.runs("Idea: let the kite fall.") == [.init(text: "Idea:", bold: true), .init(text: " let the kite fall.")])
        #expect(AIAnswerText.runs("Idea 2: a flood").first == .init(text: "Idea 2:", bold: true))
        // Inside a list item too.
        guard case .bullets(let items)? = AIAnswerText.parse("- Idea: a flood").first else {
            Issue.record("not a list")
            return
        }
        #expect(AIAnswerText.runs(items[0]).first == .init(text: "Idea:", bold: true))
    }

    @Test func showsMarkupAsTheCharactersItIsNeverAsElements() {
        let hostile = ["<img src=x onerror=alert(1)>", "- <script>alert(1)</script>", "1. [link](https://evil.example)", "Idea: <b>bold</b>"]
            .joined(separator: "\n")
        let blocks = AIAnswerText.parse(hostile)
        #expect(blocks == [
            .paragraph(["<img src=x onerror=alert(1)>"]),
            .bullets(["<script>alert(1)</script>"]),
            .numbered(["[link](https://evil.example)"]),
            .paragraph(["Idea: <b>bold</b>"]),
        ])
        // The only bold run is the label; the rest stays the characters it is.
        let runs = ["<script>alert(1)</script>", "[link](https://evil.example)", "Idea: <b>bold</b>", "**not bold**"].flatMap(AIAnswerText.runs)
        #expect(runs.filter(\.bold).map(\.text) == ["Idea:"])
        #expect(runs.map(\.text).joined().contains("<script>alert(1)</script>"))
        #expect(runs.map(\.text).joined().contains("[link](https://evil.example)"))
        #expect(runs.map(\.text).joined().contains("**not bold**"))
    }

    @MainActor
    @Test func theAnswerDrawsInLightAndDark() {
        for scheme in [ColorScheme.light, .dark] {
            let themes = ThemeStore()
            let view = AIAnswerTextView(text: "Two options.\n\n- Cut the kite\n- Keep the kite\n\n1. First\n\nIdea: end on the station clock.")
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
                .environment(\.colorScheme, scheme)
                .frame(width: 320)
            #expect(Fixture.render(view, name: "ai-answer-text-\(scheme)", size: CGSize(width: 320, height: 240)) != nil)
        }
    }
}

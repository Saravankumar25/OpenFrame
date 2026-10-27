// Pasting on the Story page keeps what the Windows page keeps: the parse rules of storyDoc.ts
// (Windows has no paste test of its own; these check each rule of that schema). Bold, italic,
// underline and highlight from their tags and styles, palette colours only from the page's own
// data-color, sizes from pt and px, everything else dropped; a copy from the page pastes back
// exactly.

import AppKit
import Foundation
import Testing
@testable import OpenFrameUI

private typealias Doc = StoryPageDocument

@MainActor
private func runs(_ s: NSAttributedString?) -> [[Doc.Run]]? {
    guard let s, let f = Doc.content(of: s).format else { return nil }
    return try? JSONDecoder().decode([[Doc.Run]].self, from: Data(f.utf8))
}

@MainActor
private func text(_ s: NSAttributedString?) -> String? { s.map { Doc.content(of: $0).text } }

@Suite("StoryPagePasteTests", .serialized)
@MainActor
struct StoryPagePasteTests {
    @Test func boldItalicAndUnderlineFromTheirTagsAndStyles() {
        let s = StoryPagePaste.fromHTML(#"<b>A</b><strong>B</strong><span style="font-weight:700">C</span><span style="font-weight:normal">D</span><i>E</i><em>F</em><span style="font-style: italic">G</span><u>H</u><span style="text-decoration: underline">I</span>"#)
        #expect(text(s) == "ABCDEFGHI")
        #expect(runs(s) == [[Doc.Run(t: "ABC", b: true), Doc.Run(t: "D"), Doc.Run(t: "EFG", i: true), Doc.Run(t: "HI", u: true)]])
    }

    @Test func coloursComeOnlyFromThePalette() {
        let s = StoryPagePaste.fromHTML(#"<u data-color="red">a</u><u data-color="teal">b</u><mark data-color="green">c</mark><mark>d</mark><mark data-color="cyan">e</mark>"#)
        #expect(runs(s) == [[Doc.Run(t: "a", u: true, uc: "red"), Doc.Run(t: "b", u: true),
                             Doc.Run(t: "c", h: true, hc: "green"), Doc.Run(t: "de", h: true)]])
    }

    @Test func aBackgroundColourOrATextColourIsNotKept() {
        let s = StoryPagePaste.fromHTML(#"<span style="background-color: yellow; color: red; font-family: Georgia">plain</span> <span style="text-decoration-color: red; text-decoration: underline">ink</span>"#)
        #expect(runs(s) == [[Doc.Run(t: "plain "), Doc.Run(t: "ink", u: true)]])
    }

    @Test func sizesComeFromPointsAndPixelsWithinRange() {
        let s = StoryPagePaste.fromHTML(#"<span style="font-size:18pt">a</span><span style="font-size:24px">b</span><span style="font-size:12pt">c</span><span style="font-size:2em">d</span><span style="font-size:400pt">e</span><span style="font-size:18.67px">f</span>"#)
        #expect(runs(s) == [[Doc.Run(t: "ab", s: 18), Doc.Run(t: "cd"), Doc.Run(t: "e", s: 96), Doc.Run(t: "f", s: 14)]])
        #expect(StoryPagePaste.cssSize("14pt") == 14)
        #expect(StoryPagePaste.cssSize("1.5rem") == nil)
    }

    @Test func blocksStartNewLinesAndWhiteSpaceCollapses() {
        let s = StoryPagePaste.fromHTML("<p>One   two\n three</p><div>Four<br>five</div><h1>Six</h1><script>no()</script>")
        #expect(text(s) == "One two three\nFour five\nSix")
        #expect(runs(s) == nil)
    }

    @Test func richTextFromMacAppsFollowsTheSameRules() throws {
        let src = NSMutableAttributedString(string: "Bold plain", attributes: [.font: NSFont(name: "Helvetica", size: 12)!])
        src.addAttribute(.font, value: NSFont(name: "Helvetica-Bold", size: 12)!, range: NSRange(location: 0, length: 4))
        src.addAttribute(.backgroundColor, value: NSColor.yellow, range: NSRange(location: 5, length: 5))
        let rtf = try #require(src.rtf(from: NSRange(location: 0, length: src.length)))
        let s = StoryPagePaste.fromRTF(rtf)
        #expect(text(s) == "Bold plain")
        let r = try #require(runs(s))
        #expect(r.first?.first == Doc.Run(t: "Bold", b: true))
        #expect(r.flatMap { $0 }.allSatisfy { $0.h == nil })
    }

    /// A copy from the page carries its runs and pastes back with every colour and size.
    @Test func aCopyFromThePagePastesBackExactly() throws {
        _ = NSApplication.shared
        let from = StoryPageTextView.make()
        from.load(Doc.attributed("Green and red\nline two", #"[[{"t":"Green","h":true,"hc":"green","s":20},{"t":" and "},{"t":"red","u":true,"uc":"red"}],[{"t":"line two","i":true}]]"#))
        from.setSelectedRange(NSRange(location: 0, length: 22))
        let pb = NSPasteboard(name: NSPasteboard.Name("story-page-copy-test"))
        pb.clearContents()
        #expect(from.writeSelection(to: pb, types: from.writablePasteboardTypes))
        #expect(pb.string(forType: StoryPagePaste.runsType) != nil)
        let to = StoryPageTextView.make()
        to.load(Doc.attributed("", nil))
        #expect(to.insertPasted(from: pb))
        #expect(runs(to.textStorage) == [[Doc.Run(t: "Green", h: true, s: 20, hc: "green"), Doc.Run(t: " and "), Doc.Run(t: "red", u: true, uc: "red")],
                                          [Doc.Run(t: "line two", i: true)]])
        // The text and the formatting always match, so the core accepts the save.
        let c = Doc.content(of: to.textStorage!)
        #expect(Doc.parseFormat(c.text, c.format) != nil)
    }

    @Test func pastedHTMLReplacesTheSelection() {
        _ = NSApplication.shared
        let v = StoryPageTextView.make()
        v.load(Doc.attributed("Keep XX here", nil))
        v.setSelectedRange(NSRange(location: 5, length: 2))
        let pb = NSPasteboard(name: NSPasteboard.Name("story-page-html-test"))
        pb.clearContents()
        pb.setData(Data("<b>bold</b>".utf8), forType: .html)
        pb.setString("bold", forType: .string)
        #expect(v.insertPasted(from: pb))
        #expect(runs(v.textStorage) == [[Doc.Run(t: "Keep "), Doc.Run(t: "bold", b: true), Doc.Run(t: " here")]])
    }
}

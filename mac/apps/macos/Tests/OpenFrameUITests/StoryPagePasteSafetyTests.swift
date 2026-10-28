// Pasted HTML can come from any web page's copy handler, so a paste never reads a local file
// through an entity, never expands entities without limit, never overflows the stack on deep
// nesting, and pastes oversized rich text as plain text.

import AppKit
import Foundation
import Testing
@testable import OpenFrameUI

@MainActor
private func text(_ s: NSAttributedString?) -> String? { s.map { StoryPageDocument.content(of: $0).text } }

@Suite("StoryPagePasteSafetyTests", .serialized)
@MainActor
struct StoryPagePasteSafetyTests {
    private static let secret = "OPENFRAME-SECRET-7f3a"

    private func secretFile() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("story-paste-\(UUID().uuidString).txt")
        try Self.secret.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    @Test func anExternalEntityNeverReadsALocalFile() throws {
        let file = try secretFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let inputs = [
            #"<!DOCTYPE html [<!ENTITY e SYSTEM "\#(file.absoluteString)">]><p>A &e; B</p>"#,
            #"<?xml version="1.0"?><!DOCTYPE html [<!ENTITY e SYSTEM "\#(file.absoluteString)">]><html><body><p>A &e; B</p></body></html>"#,
            #"<!doctype html [<!ENTITY a "]>"><!ENTITY e SYSTEM "\#(file.absoluteString)">]><p>A &e; B</p>"#,
            #"<p>A</p><!ENTITY e SYSTEM "\#(file.absoluteString)"><p>&e;</p>"#,
            #"<!DOC<!x>TYPE html [<!ENTITY e SYSTEM "\#(file.absoluteString)">]><p>A &e; B</p>"#,
        ]
        for html in inputs {
            let s = StoryPagePaste.fromHTML(html)
            #expect(!(text(s) ?? "").contains(Self.secret), "\(html)")
        }
        // A plain DOCTYPE is still pasted as before.
        #expect(text(StoryPagePaste.fromHTML("<!DOCTYPE html><html><body><p><b>Hi</b></p></body></html>")) == "Hi")
    }

    @Test func nestedEntitiesReturnQuicklyWithBoundedText() {
        var decl = #"<!ENTITY a0 "aaaaaaaaaa">"#
        for i in 1...8 { decl += #"<!ENTITY a\#(i) ""# + String(repeating: "&a\(i - 1);", count: 10) + #"">"# }
        let html = #"<?xml version="1.0"?><!DOCTYPE html [\#(decl)]><html><body><p>&a8;</p></body></html>"#
        let start = Date()
        let s = StoryPagePaste.fromHTML(html)
        #expect(Date().timeIntervalSince(start) < 1)
        #expect((s?.length ?? 0) < 1_000)
    }

    @Test func deepNestingDoesNotCrashAndPastesPlainText() {
        let n = 20_000
        let html = String(repeating: "<div><span>", count: n) + "x" + String(repeating: "</span></div>", count: n)
        let start = Date()
        #expect(StoryPagePaste.fromHTML(html) == nil)
        #expect(Date().timeIntervalSince(start) < 1)
        let pb = NSPasteboard(name: NSPasteboard.Name("story-page-deep-test"))
        pb.clearContents()
        pb.setData(Data(html.utf8), forType: .html)
        pb.setString("x", forType: .string)
        let s = StoryPagePaste.read(pb)
        #expect(s?.string == "x")
        #expect(s.map { $0.attribute(.font, at: 0, effectiveRange: nil) == nil } == true)
    }

    /// Nesting deeper than the old recursion's comfort still keeps its formatting.
    @Test func nestingWithinTheLimitKeepsItsFormatting() {
        let n = 400
        let s = StoryPagePaste.fromHTML(String(repeating: "<b>", count: n) + "deep" + String(repeating: "</b>", count: n) + "<p>next</p>")
        #expect(text(s) == "deep\nnext")
        #expect(s.map { StoryPageDocument.content(of: $0).format?.contains(#""b":true"#) ?? false } == true)
    }

    @Test func oversizedRichTextPastesAsPlainText() {
        let big = "<p><b>" + String(repeating: "word ", count: StoryPagePaste.maxRichBytes / 5 + 10) + "</b></p>"
        #expect(StoryPagePaste.fromHTML(big) == nil)
        #expect(StoryPagePaste.fromHTML(Data(big.utf8)) == nil)
        #expect(StoryPagePaste.fromRTF(Data(repeating: 0x61, count: StoryPagePaste.maxRichBytes + 1)) == nil)
        let pb = NSPasteboard(name: NSPasteboard.Name("story-page-big-test"))
        pb.clearContents()
        pb.setData(Data(big.utf8), forType: .html)
        pb.setString("plain words", forType: .string)
        let start = Date()
        let s = StoryPagePaste.read(pb)
        #expect(Date().timeIntervalSince(start) < 1)
        #expect(s?.string == "plain words")
    }
}

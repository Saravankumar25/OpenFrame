// The Story page, ported from the Windows storyDoc.test.ts and StoryPage.test.tsx (commits
// ed78cf85, 69e17c98 and b1527711): the stored text and formatting round trip, the editor's
// formatting commands and shortcuts, saving 400 ms after typing with a silent retry when another
// window saved first, the export formats, zoom and the Dark page. Run against a real core on a
// temporary folder.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

private typealias Doc = StoryPageDocument

// MARK: Document

@Suite("StoryPageTests document")
struct StoryPageDocumentTests {
    static let format = #"[[{"t":"Hero","b":true,"s":20},{"t":" walks ","i":true},{"t":"away","u":true,"h":true}],[],[{"t":"The end"}]]"#
    static let text = "Hero walks away\n\nThe end"

    private func runs(_ json: String?) -> [[Doc.Run]]? {
        guard let json else { return nil }
        return try? JSONDecoder().decode([[Doc.Run]].self, from: Data(json.utf8))
    }

    @Test @MainActor func roundTripsTheTextAndItsFormatting() {
        let s = Doc.attributed(Self.text, Self.format)
        let back = Doc.content(of: s)
        #expect(back.text == Self.text)
        #expect(runs(back.format) == runs(Self.format))
    }

    @Test @MainActor func storesPlainTextWithNoFormattingAtAll() {
        let two = Doc.content(of: Doc.attributed("One\n\nTwo", nil))
        #expect(two.text == "One\n\nTwo" && two.format == nil)
        let empty = Doc.content(of: Doc.attributed("", nil))
        #expect(empty.text == "" && empty.format == nil)
    }

    @Test @MainActor func ignoresAFormattingThatDoesNotMatchTheText() {
        #expect(Doc.parseFormat("Other words", Self.format) == nil)
        #expect(Doc.parseFormat(Self.text, "not json") == nil)
        #expect(Doc.parseFormat(Self.text, #"[[{"t":"Hero walks away"}]]"#) == nil)
        let other = Doc.content(of: Doc.attributed("Other words", Self.format))
        #expect(other.text == "Other words" && other.format == nil)
    }

    @Test @MainActor func joinsNeighbouringTextOfTheSameStyleAndDropsTheBodySize() {
        let s = NSMutableAttributedString()
        s.append(NSAttributedString(string: "Ab", attributes: Doc.Style(bold: true).attributes()))
        s.append(NSAttributedString(string: "cd", attributes: Doc.Style(bold: true, size: Doc.bodySize).attributes()))
        s.append(NSAttributedString(string: "ef", attributes: Doc.Style(size: 18).attributes()))
        #expect(runs(Doc.content(of: s).format) == [[Doc.Run(t: "Abcd", b: true), Doc.Run(t: "ef", s: 18)]])
    }

    @Test @MainActor func roundTripsColoursAndReadsOldRunsAsYellowAndInk() {
        let coloured = #"[[{"t":"Green","h":true,"hc":"green"},{"t":" red","u":true,"uc":"red"},{"t":" old","u":true,"h":true},{"t":" both","u":true,"uc":"purple","h":true,"hc":"pink"}]]"#
        let text = "Green red old both"
        let s = Doc.attributed(text, coloured)
        #expect(runs(Doc.content(of: s).format) == runs(coloured))
        let old = Doc.Style(s.attributes(at: 10, effectiveRange: nil))
        #expect(old.underline == "ink" && old.highlight == "yellow")
        // Neighbouring text with different colours stays in separate runs.
        let two = NSMutableAttributedString()
        two.append(NSAttributedString(string: "ab", attributes: Doc.Style(highlight: "blue").attributes()))
        two.append(NSAttributedString(string: "cd", attributes: Doc.Style(highlight: "orange").attributes()))
        #expect(runs(Doc.content(of: two).format) == [[Doc.Run(t: "ab", h: true, hc: "blue"), Doc.Run(t: "cd", h: true, hc: "orange")]])
        // A name outside the palette (never saved by the app) falls back to the default.
        let teal = Doc.attributed("x", #"[[{"t":"x","h":true,"hc":"teal"}]]"#)
        #expect(Doc.Style(teal.attributes(at: 0, effectiveRange: nil)).highlight == "yellow")
    }

    @Test func countsWords() {
        #expect(Doc.wordCount("  one two\n\nthree ") == 3)
        #expect(Doc.wordCount("") == 0)
    }

    @Test func thePaletteMatchesTheCore() {
        #expect(Doc.highlights.map(\.name) == ["yellow", "green", "blue", "pink", "orange", "purple"])
        #expect(Doc.underlines.map(\.name) == ["ink", "red", "blue", "green", "orange", "purple"])
        // prose.rs HIGHLIGHTS and UNDERLINES (the PDF uses the same light shades).
        #expect(Doc.highlights.map(\.light) == ["#ffe27a", "#b8f0a8", "#b3dcff", "#ffc4e1", "#ffcc99", "#dcc8ff"])
        #expect(Doc.underlines.dropFirst().map(\.light) == ["#d32f2f", "#1e63d6", "#2e8b3e", "#e67e22", "#8e44ad"])
    }

    @Test @MainActor func lineBreaksOfAnyKindBecomeOneLine() {
        let s = NSAttributedString(string: "a\r\nb\rc", attributes: Doc.Style(bold: true).attributes())
        let c = Doc.content(of: s)
        #expect(c.text == "a\nb\nc")
        #expect(runs(c.format)?.count == 3)
    }
}

// MARK: Editor

/// A text view in a window, as the page shows it.
@MainActor
private final class EditorBox {
    let view = StoryPageTextView.make()
    let window: NSWindow
    var changes: [NSAttributedString] = []

    init(_ text: String, _ format: String? = nil) {
        _ = NSApplication.shared
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 800, height: 900))
        scroll.documentView = view
        window = NSWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = scroll
        view.load(Doc.attributed(text, format))
        view.onChange = { [weak self] s in self?.changes.append(NSAttributedString(attributedString: s)) }
        window.makeFirstResponder(view)
    }

    var content: (text: String, format: String?) { Doc.content(of: view.textStorage!) }
    var runs: [[Doc.Run]]? {
        content.format.flatMap { try? JSONDecoder().decode([[Doc.Run]].self, from: Data($0.utf8)) }
    }

    func select(_ from: Int, _ to: Int) { view.setSelectedRange(NSRange(location: from, length: to - from)) }

    func key(_ chars: String, _ code: UInt16, _ flags: NSEvent.ModifierFlags) -> Bool {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0, windowNumber: window.windowNumber,
                                 context: nil, characters: chars, charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        return view.performKeyEquivalent(with: e)
    }
}

@Suite("StoryPageTests editor", .serialized)
@MainActor
struct StoryPageEditorTests {
    @Test func makesSelectedTextBoldItalicUnderlinedHighlightedAndBigger() {
        let e = EditorBox("In the beginning.")
        e.select(0, 6)
        e.view.toggle(\.bold)
        e.view.toggle(\.italic)
        e.view.toggleColored(.underline)
        e.view.toggleColored(.highlight)
        e.view.setSize(24)
        let t = e.view.tools
        #expect(t.bold && t.italic && t.underline && t.highlight && t.size == 24)
        #expect(e.content.text == "In the beginning.")
        #expect(e.runs == [[Doc.Run(t: "In the", b: true, i: true, u: true, h: true, s: 24), Doc.Run(t: " beginning.")]])
        #expect(!e.changes.isEmpty)
        // A second click takes the style off again.
        e.view.toggle(\.bold)
        #expect(e.view.tools.bold == false)
    }

    @Test func picksHighlightAndUnderlineColoursFromTheLists() {
        let e = EditorBox("In the beginning.")
        e.select(0, 6)
        e.view.pickColor(.highlight, "green")
        e.view.pickColor(.underline, "red")
        #expect(e.view.tools.highlight && e.view.tools.highlightColor == "green" && e.view.tools.underlineColor == "red")
        #expect(e.runs == [[Doc.Run(t: "In the", u: true, h: true, hc: "green", uc: "red"), Doc.Run(t: " beginning.")]])

        // The main button now puts on the last colour picked; a selection of two colours shows none.
        e.select(7, 16)
        #expect(e.view.tools.highlightColor == nil)
        e.view.toggleColored(.highlight)
        #expect(e.runs == [[Doc.Run(t: "In the", u: true, h: true, hc: "green", uc: "red"), Doc.Run(t: " "),
                            Doc.Run(t: "beginning", h: true, hc: "green"), Doc.Run(t: ".")]])
        e.view.pickColor(.highlight, "blue")
        e.select(0, 16)
        #expect(e.view.tools.highlight)
        #expect(e.view.tools.highlightColor == nil)

        // None takes the highlight off the whole selection.
        e.view.setColor(.highlight, nil)
        #expect(e.runs == [[Doc.Run(t: "In the", u: true, uc: "red"), Doc.Run(t: " beginning.")]])
    }

    @Test func showsASavedFormattingAndAMixedSizeAsMixed() {
        let e = EditorBox("Big and small", #"[[{"t":"Big","b":true,"s":30},{"t":" and small"}]]"#)
        let a = e.view.textStorage!.attributes(at: 0, effectiveRange: nil)
        #expect((a[.font] as? NSFont)?.pointSize == 30)
        #expect((a[.font] as? NSFont)?.fontDescriptor.symbolicTraits.contains(.bold) == true)
        e.select(0, 8)
        #expect(e.view.tools.size == nil)
        e.select(4, 8)
        #expect(e.view.tools.size == 12)
    }

    @Test func theShortcutsWorkWhileThePageHasTheKeyboard() {
        let e = EditorBox("Night bus")
        e.select(0, 5)
        #expect(e.key("b", 11, .command))
        #expect(e.view.tools.bold)
        #expect(e.key("i", 34, .command))
        #expect(e.key("u", 32, .command))
        #expect(e.key("h", 4, [.command, .shift]))
        #expect(e.view.tools.italic && e.view.tools.underline && e.view.tools.highlight)
        // Command+] and Command+[ step through the toolbar sizes.
        #expect(e.key("]", 30, .command))
        #expect(e.view.tools.size == 14)
        #expect(e.key("[", 33, .command))
        #expect(e.key("[", 33, .command))
        #expect(e.view.tools.size == 11)
        // A key that is not a formatting shortcut is left alone.
        #expect(e.view.handleShortcut(NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: .command, timestamp: 0,
                                                        windowNumber: e.window.windowNumber, context: nil, characters: "k",
                                                        charactersIgnoringModifiers: "k", isARepeat: false, keyCode: 40)!) == false)
    }

    /// Elsewhere Command+[ and Command+] stay Back and Forward of the Go menu.
    @Test func commandBracketsAreLeftToTheGoMenuWhenThePageDoesNotHaveTheKeyboard() {
        let e = EditorBox("Night bus")
        e.select(0, 5)
        e.window.makeFirstResponder(nil)
        #expect(e.key("[", 33, .command) == false)
        #expect(e.view.tools.size == 12)
    }

    @Test func typedTextTakesTheStyleAtTheCaretAndPasteKeepsOnlyTheWords() {
        let e = EditorBox("")
        e.view.toggle(\.bold)
        e.view.insertText("Bold start", replacementRange: e.view.selectedRange())
        #expect(e.runs == [[Doc.Run(t: "Bold start", b: true)]])
        let pb = NSPasteboard(name: NSPasteboard.Name("story-page-test"))
        pb.clearContents()
        pb.setString(" more\r\nlines", forType: .string)
        #expect(e.view.readSelection(from: pb, type: .string))
        #expect(e.content.text == "Bold start more\nlines")
    }

    @Test func theDarkPageTurnsTheSheetAndItsColoursAndBack() {
        let e = EditorBox("Marked", #"[[{"t":"Marked","h":true,"hc":"green","u":true,"uc":"red"}]]"#)
        e.view.showPage(dark: false)
        #expect(e.view.backgroundColor == Doc.color("#ffffff"))
        e.view.showPage(dark: true)
        #expect(e.view.isDarkPage)
        #expect(e.view.backgroundColor == Doc.color("#1e1e22"))
        let a = e.view.textStorage!.attributes(at: 0, effectiveRange: nil)
        var bg: NSColor?
        var ul: NSColor?
        var ink: NSColor?
        e.view.effectiveAppearance.performAsCurrentDrawingAppearance {
            bg = (a[.backgroundColor] as? NSColor)?.usingColorSpace(.sRGB)
            ul = (a[.underlineColor] as? NSColor)?.usingColorSpace(.sRGB)
            ink = (a[.foregroundColor] as? NSColor)?.usingColorSpace(.sRGB)
        }
        #expect(bg == Doc.color("#2f5a2a").usingColorSpace(.sRGB))
        #expect(ul == Doc.color("#ff6b6b").usingColorSpace(.sRGB))
        #expect(ink == Doc.color("#ece9e2").usingColorSpace(.sRGB))
        e.view.showPage(dark: false)
        e.view.effectiveAppearance.performAsCurrentDrawingAppearance {
            bg = (a[.backgroundColor] as? NSColor)?.usingColorSpace(.sRGB)
        }
        #expect(bg == Doc.color("#b8f0a8").usingColorSpace(.sRGB))
        // The text and its formatting do not change with the page.
        #expect(e.content.format != nil)
    }
}

// MARK: Zoom and Dark page preferences

@Suite("StoryPageTests preferences")
struct StoryPagePrefsTests {
    @Test func zoomStaysBetweenHalfAndDoubleAndTheWheelZoomsBothWays() {
        #expect(StoryPagePrefs.zoom(stored: 0) == 1)
        #expect(StoryPagePrefs.zoom(stored: 7) == 1)
        #expect(StoryPagePrefs.zoom(stored: 1.1) == 1.1)
        #expect(StoryPagePrefs.clamp(1 + StoryPagePrefs.zoomStep) == 1.1)
        #expect(StoryPagePrefs.clamp(0.1) == 0.5)
        #expect(StoryPagePrefs.clamp(9) == 2)
        #expect(StoryPagePrefs.wheel(1.1, deltaY: 400) > 1.1)
        #expect(StoryPagePrefs.wheel(1.1, deltaY: -400) < 1.1)
        #expect(StoryPagePrefs.label(1.1) == "110%")
    }

    /// The Story page has keys of its own: the Beat Board zoom is `of.story.zoom`.
    @Test func theKeysAreTheStoryPagesOwn() {
        #expect(StoryPagePrefs.zoomKey != "of.story.zoom")
        #expect(StoryPagePrefs.darkKey == "openframe.story.darkPage")
    }
}

// MARK: Saving

@Suite("StoryPageTests saving", .serialized)
@MainActor
struct StoryPageSavingTests {
    private func project(_ title: String) async throws -> (URL, AppModel) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        #expect(await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm)))
        return (root, app)
    }

    private func stored(_ app: AppModel) async throws -> StoryPageDto { try await app.core!.storyPageGet() }

    private func until(_ condition: () -> Bool) async -> Bool {
        for _ in 0..<100 {
            if condition() { return true }
            try? await Task.sleep(for: .milliseconds(20))
        }
        return condition()
    }

    @Test func savesTypedTextAfterAShortPauseWithTheRevisionItWasBasedOn() async throws {
        let (root, app) = try await project("Night Bus")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        #expect(m.loaded == .init(text: "", format: nil))
        #expect(m.statusLabel == "Saved" && m.words == 0)
        m.changed(text: "In the beginning.\n\nThen a bus.", format: nil)
        #expect(m.statusLabel == "Saving\u{2026}")
        #expect(try await stored(app).rev == 0)
        #expect(await until { m.status == .saved })
        let first = try await stored(app)
        #expect(first.text == "In the beginning.\n\nThen a bus." && first.rev == 1)
        #expect(m.wordsLabel == "6 words")
        m.changed(text: "Second pass.", format: nil)
        #expect(await until { m.status == .saved })
        #expect(try await stored(app).rev == 2)
    }

    @Test func savesTheFormattingFromTheEditor() async throws {
        let (root, app) = try await project("Formatting")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        let e = EditorBox("In the beginning.")
        e.view.onChange = { m.changed($0) }
        e.select(0, 6)
        e.view.toggle(\.bold)
        e.view.setSize(24)
        #expect(await m.flush())
        let p = try await stored(app)
        #expect(p.text == "In the beginning.")
        let runs = try JSONDecoder().decode([[Doc.Run]].self, from: Data(try #require(p.format).utf8))
        #expect(runs == [[Doc.Run(t: "In the", b: true, s: 24), Doc.Run(t: " beginning.")]])
        // A new visit shows the same formatting.
        let again = StoryPageModel(app: app)
        await again.load()
        let shown = Doc.attributed(again.loaded!.text, again.loaded!.format)
        #expect(Doc.Style(shown.attributes(at: 0, effectiveRange: nil)) == Doc.Style(bold: true, size: 24))
    }

    @Test func storesTheTextWhenLeavingAndWhenTheAppAsks() async throws {
        let (root, app) = try await project("Leaving")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let first = StoryPageModel(app: app)
        await first.load()
        first.changed(text: "Leave me", format: nil)
        first.leave()
        // The page that opens again waits for that save and shows the newest text.
        let second = StoryPageModel(app: app)
        await second.load()
        #expect(second.loaded?.text == "Leave me")
        second.changed(text: "Close me", format: nil)
        await StoryPageModel.flushAll()
        #expect(try await stored(app).text == "Close me")
        second.changed(text: "Quit me", format: nil)
        await ScreenplayWorkspaceModel.flushAll()
        #expect(try await stored(app).text == "Quit me")
    }

    @Test func keepsTheWritersTextAndRetriesOnTheNewestRevisionAfterAConflict() async throws {
        let (root, app) = try await project("Two Windows")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        // Another window saves first.
        _ = try await app.core!.storyPageSave(StoryPageSaveArgs(text: "Theirs", format: nil, expectedRev: 0))
        m.changed(text: "Mine", format: nil)
        #expect(await m.flush())
        #expect(m.status == .saved)
        let p = try await stored(app)
        #expect(p.text == "Mine" && p.rev == 2)
    }

    @Test func saysWhenASaveFailedAndStoresTheTextOnTheNextTry() async throws {
        let (root, app) = try await project("Keep Safe")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        guard case let .project(project) = app.phase else { Issue.record("no project"); return }
        let m = StoryPageModel(app: app)
        await m.load()
        m.changed(text: "Keep this safe", format: nil)
        _ = try await app.core!.projectClose()
        #expect(await m.flush() == false)
        #expect(m.statusLabel == "Not saved yet")
        await app.openProject(path: project.path)
        #expect(await m.flush())
        #expect(m.statusLabel == "Saved")
        #expect(try await stored(app).text == "Keep this safe")
    }

    @Test func offersPlainTextPDFAndWordAndStoresTheTextBeforeTheDialogOpens() async throws {
        let (root, app) = try await project("Export Night")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        m.changed(text: "Export me", format: nil)
        await m.export()
        let e = try #require(m.exportModel)
        #expect(e.title == "Export Story")
        #expect(e.formats.map(\.label) == ["Plain text (.txt)", "PDF", "Word (.docx)"])
        #expect(e.formats.map(\.value) == ["txt", "pdf", "docx"])
        #expect(try await stored(app).text == "Export me")
        // The core writes the file with the formatting; the page does not change.
        let out = root.appendingPathComponent("Story.pdf").path
        let r = try await app.core!.storyPageExport(ExportStoryPageArgs(format: "pdf", path: out))
        #expect(FileManager.default.fileExists(atPath: out))
        #expect(r.fileName.hasSuffix(".pdf"))
    }

    @Test func doesNotOpenThePageForWritingWhenItCannotBeRead() async throws {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        #expect(m.loaded == nil)
        #expect(m.loadError?.isEmpty == false)
    }

    @Test func thePageRendersOnALightAndADarkSheet() async throws {
        let (root, app) = try await project("Render")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        _ = try await app.core!.storyPageSave(StoryPageSaveArgs(
            text: "The night bus.", format: #"[[{"t":"The","b":true,"s":24},{"t":" night ","h":true,"hc":"green"},{"t":"bus.","u":true,"uc":"red"}]]"#, expectedRev: 0))
        let themes = ThemeStore()
        for (scheme, dark) in [(ColorScheme.light, false), (.dark, true)] {
            UserDefaults.standard.set(dark, forKey: StoryPagePrefs.darkKey)
            let m = StoryPageModel(app: app)
            await m.load()
            let view = StoryPageView(model: m).environment(app).environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme)).environment(\.colorScheme, scheme)
            #expect(Fixture.render(view, name: "story-page-\(dark ? "dark" : "light")", size: CGSize(width: 1000, height: 760)) != nil)
        }
        UserDefaults.standard.removeObject(forKey: StoryPagePrefs.darkKey)
    }
}

// MARK: Where Story opens

@Suite("StoryPageTests opening")
@MainActor
struct StoryPageOpeningTests {
    @Test func storyOpensOnTheStoryPageAndBoardLinksOpenTheBoard() async throws {
        #expect(StoryTab.allCases.first == .page)
        #expect(StoryTab.page.title == "Story")
        #expect(StoryTab.initial(params: nil) == .page)
        #expect(StoryTab.initial(params: ["episodeId": "e1"]) == .page)
        #expect(StoryTab.initial(params: ["cardId": "c1"]) == .board)
        #expect(StoryTab.initial(params: ["actId": "a1"]) == .board)
        #expect(StoryTab.initial(params: ["characterId": "x"]) == .characters)
        // The request other screens fire with the workspace (the assistant uses the same names).
        #expect(StoryTab.board.showIntent == "story.show_board")
        #expect(StoryTab.characters.showIntent == "story.show_characters")
        let root = try Fixture.root()
        let app = Fixture.model(root)
        defer { try? FileManager.default.removeItem(at: root) }
        #expect(StoryModel(app: app).tab == .page)
    }
}

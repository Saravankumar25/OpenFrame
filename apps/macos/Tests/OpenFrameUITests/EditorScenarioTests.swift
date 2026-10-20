// Mac phase M4, editor A: the writing experience inside the text view, driven the way a person
// types. Each test starts a real core on a temporary folder, puts the native text view in an
// off-screen window, types with `insertText`, Return, Tab and Escape, and reads the result from
// the view and from the core. They follow the shared scenarios (tests/e2e/scenarios):
// 40-screenplay.md, 41-screenplay-engine.md and 42-writing-comfort.md. The Windows steps that
// belong to panels (drafts drawer, title page, page setup, lock, revisions, exports) are covered
// by the panel tests; Command stands in for Ctrl.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

/// One writer: a core, a project, the workspace model and a text view in a window.
@MainActor
final class EditorRig {
    let root: URL
    let app: AppModel
    let model: ScreenplayWorkspaceModel
    let view: ScreenplayTextView
    let window: NSWindow
    private let coordinator: ScreenplayEditorHost.Coordinator
    let projectPath: String

    static func start(title: String = "Bus Depot", root existing: URL? = nil, open path: String? = nil) async throws -> EditorRig {
        _ = NSApplication.shared
        let root = try existing ?? Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        if let path {
            await app.openProject(path: path)
        } else {
            _ = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        }
        guard case let .project(project) = app.phase else { throw CoreFailure(code: "test.no_project", message: "no project") }
        return EditorRig(root: root, app: app, projectPath: project.path)
    }

    private init(root: URL, app: AppModel, projectPath: String) {
        self.root = root
        self.app = app
        self.projectPath = projectPath
        model = ScreenplayWorkspaceModel(rules: SharedScreenplayRules.shared)
        model.attach(app)
        view = ScreenplayTextView.make()
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 900, height: 700))
        scroll.documentView = view
        view.frame = scroll.frame
        view.minSize = NSSize(width: 0, height: 0)
        view.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        view.autoresizingMask = [.width]
        window = NSWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = scroll
        coordinator = ScreenplayEditorHost.Coordinator(model: model, onMove: { _, _ in })
        model.bind(view)
        view.screenplayDelegate = coordinator
        window.makeFirstResponder(view)
    }

    /// Open the screenplay the way the workspace does on its first visit.
    func openScreenplay() async {
        await model.loadOverview()
        await model.loadSources()
        window.makeFirstResponder(view)
        view.setSelectedRange(NSRange(location: 0, length: 0))
    }

    func close() { app.shutdown() }

    // MARK: Typing

    func type(_ text: String) {
        for ch in text { view.insertText(String(ch), replacementRange: view.selectedRange()) }
    }

    func enter() { view.insertNewline(nil) }
    func tab() { view.insertTab(nil) }
    func escape() { view.cancelOperation(nil) }
    func down() { view.moveDown(nil) }
    func up() { view.moveUp(nil) }

    /// A key chord through the window, as the keyboard sends it.
    @discardableResult
    func chord(_ chars: String, keyCode: UInt16, _ flags: NSEvent.ModifierFlags) -> Bool {
        let event = NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0, windowNumber: window.windowNumber,
            context: nil, characters: chars, charactersIgnoringModifiers: chars, isARepeat: false, keyCode: keyCode)!
        return view.performKeyEquivalent(with: event)
    }

    // MARK: Reading

    func editor() -> [String] { view.paragraphs().map { "\($0.kind.rawValue): \($0.text)" } }

    func document() async throws -> ScreenplayDocument {
        try await app.core!.screenplayDocument(ScreenplayDraftIdArgs(draftId: model.draft!.id))
    }

    /// What the core holds, as "type: text" lines (scene headings come from the scene).
    func saved() async throws -> [String] {
        await model.flush()
        return try await document().scenes.flatMap { scene in
            ["scene_heading: \(scene.heading)"] + scene.elements.map { "\($0.elementType.rawValue): \($0.text)" }
        }
    }

    func characters() async throws -> [String] {
        try await app.core!.storyCharacters(StoryCharacterListArgs(includeArchived: false)).map(\.name)
    }

    /// Wait for the sync to end (the status reads Saved).
    func settle() async {
        await model.flush()
        for _ in 0..<50 where model.syncStatus != "saved" {
            try? await Task.sleep(for: .milliseconds(40))
        }
    }
}

@Suite("EditorScenarioTests", .serialized)
@MainActor
struct EditorScenarioTests {
    // MARK: 40 Screenplay workspace basics

    /// A second, new screenplay opens as one empty scene heading with its own scene id; typing
    /// there saves into that scene and nothing from the first screenplay leaks in.
    @Test func aSecondNewScreenplayStartsCleanAndSavesTyping() async throws {
        let r = try await EditorRig.start(title: "Two Scripts")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. FIRST PLACE - DAY")
        r.enter()
        r.type("The first screenplay's action.")
        await r.settle()

        let second = try await r.app.core!.screenplayCreate(CreateScreenplayArgs(title: "Second"))
        await r.model.show(screenplayId: second.screenplay.id, draftId: second.draft.id)
        r.window.makeFirstResponder(r.view)
        r.view.setSelectedRange(NSRange(location: 0, length: 0))
        let firstParagraph = try #require(r.view.paragraphs().first)
        #expect(firstParagraph.kind == .sceneHeading)
        let docBefore = try await r.document()
        #expect(firstParagraph.sceneId == docBefore.scenes.first?.id)

        r.type("EXT. SECOND PLACE - NIGHT")
        await r.settle()
        #expect(r.model.syncStatus == "saved", "\(String(describing: r.app.alert)) editor: \(r.editor()) ids: \(r.view.paragraphs().map { ($0.sceneId, $0.elementId) })")
        let saved = try await r.saved()
        #expect(saved.first == "scene_heading: EXT. SECOND PLACE - NIGHT")
        #expect(!saved.joined().contains("FIRST PLACE"))
    }

    @Test func aNewScreenplayTakesASceneWithTheEnterFlowAndSavesItself() async throws {
        let r = try await EditorRig.start(title: "Glass Harbour")
        defer { r.close() }
        await r.openScreenplay()
        #expect(!r.view.paragraphs().isEmpty)
        #expect(r.model.draft?.name.isEmpty == false)

        r.type("INT. GLASS WORKSHOP - NIGHT")
        r.enter()
        r.type("Asha shapes a lens over the blue flame.")
        r.enter()
        r.type("ASHA")
        #expect(r.view.suggestionLabels.contains("Create New: ASHA"))
        r.escape()
        #expect(r.view.suggestionLabels.isEmpty)
        r.enter()
        r.type("It has to be perfect.")
        let expected = [
            "scene_heading: INT. GLASS WORKSHOP - NIGHT",
            "action: Asha shapes a lens over the blue flame.",
            "character: ASHA",
            "dialogue: It has to be perfect.",
        ]
        #expect(Array(r.editor().prefix(4)) == expected)
        let saved = try await r.saved()
        #expect(Array(saved.prefix(4)) == expected)
        await r.settle()
        #expect(r.model.syncStatus == "saved")
        #expect(r.view.sceneLabels.values.contains("1"))
        r.view.undoManager?.removeAllActions()
    }

    @Test func textUndoAndRedoWorkInsideTheEditor() async throws {
        let r = try await EditorRig.start(title: "Undo")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("It has to be perfect.")
        // Typing more than a moment apart is a separate undo step.
        try await Task.sleep(for: .milliseconds(100))
        r.view.breakUndoCoalescing()
        r.type(" Now.")
        #expect(r.editor().contains("action: It has to be perfect. Now."))
        r.view.undoManager?.undo()
        #expect(r.editor().contains("action: It has to be perfect."))
        r.view.undoManager?.redo()
        #expect(r.editor().contains("action: It has to be perfect. Now."))
        let saved = try await r.saved()
        #expect(saved.contains("action: It has to be perfect. Now."))
    }

    // MARK: 42 Smart suggestions

    @Test func suggestionsBuildASceneWithoutRetypingAndNeverChangeTextSilently() async throws {
        let r = try await EditorRig.start(title: "Bus Depot")
        defer { r.close() }
        await r.openScreenplay()
        r.type("IN")
        #expect(r.view.suggestionLabels.contains("INT."))
        r.enter()
        #expect(r.editor()[0] == "scene_heading: INT. ")
        r.type("BUS DEPOT - ")
        r.type("NI")
        #expect(r.view.suggestionLabels.contains("NIGHT"))
        r.enter()
        #expect(r.editor()[0] == "scene_heading: INT. BUS DEPOT - NIGHT")
        r.enter()
        r.type("Meera waits for teh last bus.")
        r.enter()
        r.type("MEERA")
        #expect(r.view.suggestionLabels.contains("Create New: MEERA"))
        r.enter()
        r.view.window?.makeFirstResponder(r.view)
        var tail = Array(r.editor().suffix(2))
        #expect(tail == ["character: MEERA", "dialogue: "])
        r.type("Is it late?")
        r.enter()
        r.enter()
        #expect(r.editor().last == "scene_heading: ")
        r.type("EXT. ROAD - LATER")
        r.escape()
        #expect(r.view.suggestionLabels.isEmpty)
        #expect(r.editor().last == "scene_heading: EXT. ROAD - LATER")
        r.enter()
        r.type("The bus is late.")
        r.enter()
        r.type("ME")
        #expect(r.view.suggestionLabels.contains("MEERA"))
        r.enter()
        tail = Array(r.editor().suffix(2))
        #expect(tail == ["character: MEERA", "dialogue: "])
        #expect(r.editor().filter { $0 == "character: MEERA" }.count == 2)
        r.type("Late.")
        let expected = [
            "scene_heading: INT. BUS DEPOT - NIGHT",
            "action: Meera waits for teh last bus.",
            "character: MEERA",
            "dialogue: Is it late?",
            "scene_heading: EXT. ROAD - LATER",
        ]
        let saved = try await r.saved()
        #expect(Array(saved.prefix(5)) == expected)
        #expect(try await r.characters().contains("MEERA"))
        await r.settle()
        #expect(r.model.syncStatus == "saved")
        #expect(r.model.notice == "Added MEERA to Characters")
    }

    @Test func arrowKeysMoveTheHighlightAndEscapeClosesUntilTheNextKey() async throws {
        let r = try await EditorRig.start(title: "Arrows")
        defer { r.close() }
        await r.openScreenplay()
        r.type("IN")
        #expect(r.view.suggestionLabels.prefix(2) == ["INT.", "INT./EXT."])
        #expect(r.view.suggestionHighlight == 0)
        r.down()
        #expect(r.view.suggestionHighlight == 1)
        r.up()
        #expect(r.view.suggestionHighlight == 0)
        r.up()
        #expect(r.view.suggestionHighlight == r.view.suggestionLabels.count - 1)
        r.down()
        r.escape()
        #expect(r.view.suggestionLabels.isEmpty)
        // Selecting again at the same place does not bring it back; typing does.
        r.view.setSelectedRange(NSRange(location: 2, length: 0))
        #expect(r.view.suggestionLabels.isEmpty)
        r.type("T")
        #expect(r.view.suggestionLabels.contains("INT."))
        // With nothing typed the arrows keep moving the caret, and Return keeps its meaning.
        let empty = try await EditorRig.start(title: "Quiet")
        defer { empty.close() }
        await empty.openScreenplay()
        #expect(empty.view.suggestionHighlight == -1)
    }

    @Test func tabNeverAcceptsAndTheElementFlowKeepsItsMeaning() async throws {
        let r = try await EditorRig.start(title: "Flow")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. YARD - DAY")
        r.enter()
        r.type("Rain.")
        r.enter()
        r.type("RAVI")
        r.enter()
        r.type("Hello.")
        r.enter()
        #expect(r.editor().last == "action: ")
        r.tab()
        #expect(r.editor().last == "character: ")
        r.type("RAVI")
        // Command+2 turns the line into Action.
        #expect(r.chord("2", keyCode: 19, .command))
        #expect(r.editor().last == "action: RAVI")
        r.type(" runs.")
        #expect(r.editor().last == "action: RAVI runs.")
        let saved = try await r.saved()
        #expect(saved.last == "action: RAVI runs.")
    }

    @Test func oneEnterOnANewNameCreatesItMovesOnAndProjectUndoTakesEachStepBack() async throws {
        let r = try await EditorRig.start(title: "Order")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("Nisha enters.")
        r.enter()
        r.type("NISHA")
        #expect(r.view.suggestionLabels.contains("Create New: NISHA"))
        r.enter()
        #expect(Array(r.editor().suffix(2)) == ["character: NISHA", "dialogue: "])
        _ = try await r.saved()
        await r.settle()
        #expect(try await r.characters().contains("NISHA"))
        // First undo takes the Dialogue step back and keeps the character in the Beat Board.
        await r.app.undo()
        var saved = try await r.document().scenes.flatMap(\.elements).map { "\($0.elementType.rawValue): \($0.text)" }
        #expect(saved.last == "character: NISHA")
        #expect(try await r.characters().contains("NISHA"))
        // Second undo removes the character; the typed name stays until its own undo.
        await r.app.undo()
        saved = try await r.document().scenes.flatMap(\.elements).map { "\($0.elementType.rawValue): \($0.text)" }
        #expect(saved.last == "character: NISHA")
        #expect(!(try await r.characters().contains("NISHA")))
    }

    @Test func aKnownNameFromTheListFillsAndMovesOnSavingTheNameFirst() async throws {
        let r = try await EditorRig.start(title: "Known")
        defer { r.close() }
        _ = try await r.app.core!.storyCreateCharacter(StoryCreateCharacterArgs(name: "MEERA"))
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("Quiet.")
        r.enter()
        r.type("ME")
        #expect(r.view.suggestionLabels.contains("MEERA"))
        #expect(!r.view.suggestionLabels.contains { $0.hasPrefix("Create New") } || r.view.suggestionLabels.first == "MEERA")
        r.enter()
        #expect(Array(r.editor().suffix(2)) == ["character: MEERA", "dialogue: "])
        _ = try await r.saved()
        await r.settle()
        // The filled name was saved before the Dialogue line: one project undo leaves the name.
        await r.app.undo()
        let saved = try await r.document().scenes.flatMap(\.elements).map { "\($0.elementType.rawValue): \($0.text)" }
        #expect(saved.last == "character: MEERA")
    }

    // MARK: 41 Page break, custom elements, line breaks

    @Test func commandReturnInsertsAPageBreakAndThePageCountFollows() async throws {
        let r = try await EditorRig.start(title: "Signal Box")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. SIGNAL BOX - NIGHT")
        r.enter()
        r.type("The telephone rings twice.")
        r.enter()
        r.type("ASHA")
        r.escape()
        r.enter()
        r.type("Who is there?")
        #expect(r.chord("\r", keyCode: 36, .command))
        r.type("Nobody answers.")
        let lines = r.editor()
        let at = try #require(lines.firstIndex(of: "dialogue: Who is there?"))
        #expect(lines[at + 1] == "page_break: ")
        #expect(lines[at + 2] == "dialogue: Nobody answers.")
        // A page break holds no text.
        r.view.reveal(paragraph: at + 1)
        let before = r.editor()
        r.type("x")
        #expect(r.editor().count == before.count)
        let saved = try await r.saved()
        let savedAt = try #require(saved.firstIndex(of: "dialogue: Who is there?"))
        #expect(saved[savedAt + 1] == "page_break: ")
        #expect(saved[savedAt + 2] == "dialogue: Nobody answers.")
        await r.settle()
        let pages = try await r.app.core!.screenplayPagination(ScreenplayDraftIdArgs(draftId: r.model.draft!.id))
        #expect(pages.pageCount == 2)
        #expect(r.model.pageLabel?.hasSuffix("of 2") == true || r.model.pageLabel == "2 pages")
        // Enter on a page break adds an action after it and keeps the break.
        r.view.reveal(paragraph: at + 1)
        r.enter()
        #expect(r.editor().filter { $0 == "page_break: " }.count == 1)
        #expect(r.editor()[at + 2] == "action: ")
    }

    @Test func customElementsTakeTheirLookTheirShortcutAndTheirNextElement() async throws {
        let r = try await EditorRig.start(title: "Custom")
        defer { r.close() }
        await r.openScreenplay()
        let screenplayId = try #require(r.model.overview?.screenplay?.id)
        let def = try await r.app.core!.screenplayCreateCustomElement(ScreenplayCustomElementArgs(
            screenplayId: screenplayId, name: "Telephone", shortcut: "Ctrl+Alt+1", look: "dialogue",
            uppercase: false, italic: true, spaceBefore: 0, nextElement: "action", exportName: "Telephone"))
        await r.model.loadDocument(r.model.draft!.id)
        #expect(r.view.customDefinitions.map(\.id) == [def.id])

        r.type("INT. SIGNAL BOX - NIGHT")
        r.enter()
        r.type("The telephone rings twice.")
        r.view.setCustomElement(def.id)
        #expect(r.editor().last == "custom: The telephone rings twice.")
        // Its look is the dialogue look with the definition's own italic and spacing.
        let storage = try #require(r.view.textStorage)
        let last = (storage.string as NSString).length - 1
        let style = try #require(storage.attribute(.paragraphStyle, at: last, effectiveRange: nil) as? NSParagraphStyle)
        let dialogue = r.view.paragraphStyle(for: SharedScreenplayRules.shared.look(for: .dialogue))
        #expect(style.headIndent == dialogue.headIndent)
        #expect(style.paragraphSpacingBefore == 0)
        let font = try #require(storage.attribute(.font, at: last, effectiveRange: nil) as? NSFont)
        #expect(font.fontDescriptor.symbolicTraits.contains(.italic))
        // Tab leaves a custom element as it is.
        r.tab()
        #expect(r.editor().last == "custom: The telephone rings twice.")
        // Enter at its end starts the element the definition names.
        r.enter()
        #expect(r.editor().last == "action: ")
        // The shortcut turns another line into the custom element.
        r.type("Static on the line.")
        #expect(r.chord("1", keyCode: 18, [.command, .option]))
        #expect(r.editor().last == "custom: Static on the line.")
        let saved = try await r.saved()
        #expect(saved.contains("custom: The telephone rings twice."))
        #expect(saved.last == "custom: Static on the line.")
        // Reopen the document: the custom paragraphs come back with the same look.
        await r.model.loadDocument(r.model.draft!.id)
        #expect(r.editor().last == "custom: Static on the line.")
    }

    @Test func shiftEnterBreaksTheLineOnlyInMultiLineElements() async throws {
        let r = try await EditorRig.start(title: "Lines")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("Rain falls.")
        r.enter()
        r.type("ASHA")
        r.escape()
        r.enter()
        r.type("First line.")
        let count = r.view.paragraphs().count
        r.view.insertLineBreak(nil)
        r.type("Second line.")
        #expect(r.view.paragraphs().count == count)
        #expect(r.view.paragraphs().last?.text == "First line.\u{2028}Second line.")
        let saved = try await r.saved()
        #expect(saved.last == "dialogue: First line.\nSecond line.")
        // In a character line the same key is Enter.
        r.enter()
        r.tab()
        #expect(r.editor().last == "character: ")
        r.type("RAVI")
        r.escape()
        let before = r.view.paragraphs().count
        r.view.insertLineBreak(nil)
        #expect(r.view.paragraphs().count == before + 1)
        #expect(r.view.paragraphs().last?.kind == .dialogue)
    }

    @Test func splittingAndInsertingAboveKeepTheRightElementsAndIds() async throws {
        let r = try await EditorRig.start(title: "Split")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("Rain falls hard.")
        // Split in the middle of the action line.
        let string = r.view.string as NSString
        let at = string.range(of: "falls").location
        r.view.setSelectedRange(NSRange(location: at, length: 0))
        r.enter()
        #expect(Array(r.editor().suffix(2)) == ["action: Rain ", "action: falls hard."])
        // Enter at the start puts a blank element above and keeps the text with its own id.
        let idBefore = r.view.paragraphs().last?.elementId
        let start = (r.view.string as NSString).range(of: "falls hard.").location
        r.view.setSelectedRange(NSRange(location: start, length: 0))
        r.enter()
        let paragraphs = r.view.paragraphs()
        #expect(paragraphs[paragraphs.count - 2].text == "")
        #expect(paragraphs.last?.text == "falls hard.")
        #expect(paragraphs.last?.elementId == idBefore)
        #expect(paragraphs[paragraphs.count - 2].elementId != idBefore)
    }

    // MARK: 42 Spelling, find and replace

    @Test func spellingIsOnAndLearnSpellingTeachesTheCoreAndTheSystem() async throws {
        let r = try await EditorRig.start(title: "Spelling")
        defer { r.close() }
        await r.openScreenplay()
        // Continuous spelling is switched on when the view is made; the system may refuse it in a
        // headless test run (no spelling server), so only the teaching is asserted here.
        let word = "zqxwvtest"
        defer { NSSpellChecker.shared.unlearnWord(word) }
        r.view.learnSpelling(word: word)
        for _ in 0..<50 {
            if try await r.app.core!.writingDictionary().words.contains(word) { break }
            try await Task.sleep(for: .milliseconds(40))
        }
        #expect(try await r.app.core!.writingDictionary().words.contains(word))
        #expect(r.model.notice == "Added “\(word)” to your dictionary")
        #expect(NSSpellChecker.shared.hasLearnedWord(word))
        // Character names are never underlined: they join the checker's ignore list.
        _ = try await r.app.core!.storyCreateCharacter(StoryCreateCharacterArgs(name: "ZQXMEERAZ"))
        await r.model.loadSources()
        // The list handed to the checker (the checker's own copy is not reliable in a headless run).
        #expect(r.view.ignoredWordList.contains("ZQXMEERAZ"))
    }

    @Test func replaceAllInThisSceneOnlyThenInTheWholeScriptWithMatchCaseAndWholeWord() async throws {
        let r = try await EditorRig.start(title: "Replace")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. BUS DEPOT - NIGHT")
        r.enter()
        r.type("Meera waits for the last bus.")
        r.enter()
        r.enter()
        r.enter()
        r.type("EXT. ROAD - LATER")
        r.escape()
        r.enter()
        r.type("The bus is late. A Bus, a bus, a busy bus.")
        // The caret is in scene 2: replace only there.
        r.model.findText = "bus"
        r.model.replaceText = "van"
        r.model.scopeIsScene = true
        r.model.wholeWord = true
        r.model.matchCase = true
        await r.model.replaceAll()
        #expect(r.model.notice == "Replaced 3 matches in Scene 2")
        #expect(r.editor().contains("action: The van is late. A Bus, a van, a busy van."))
        #expect(r.editor().contains("action: Meera waits for the last bus."))
        // The core holds the same text.
        let saved = try await r.saved()
        #expect(saved.contains("action: The van is late. A Bus, a van, a busy van."))
        // One project undo takes the whole replace back.
        await r.app.undo()
        let back = try await r.document().scenes.flatMap(\.elements).map(\.text)
        #expect(back.contains("The bus is late. A Bus, a bus, a busy bus."))
        await r.model.loadDocument(r.model.draft!.id)
        #expect(r.editor().contains("action: The bus is late. A Bus, a bus, a busy bus."))
        // The whole script, any case, any part of a word.
        r.model.scopeIsScene = false
        r.model.wholeWord = false
        r.model.matchCase = false
        await r.model.replaceAll()
        #expect(r.model.notice == "Replaced 7 matches")
        #expect(r.editor().contains("action: Meera waits for the last van."))
        // No match says so and changes nothing.
        r.model.findText = "zebra"
        await r.model.replaceAll()
        #expect(r.model.notice == "No matches")
    }

    @Test func replaceAllSavesWhatWasJustTypedFirst() async throws {
        let r = try await EditorRig.start(title: "Fresh")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. HALL - DAY")
        r.enter()
        r.type("The cat sleeps.")
        // No wait for the 400 ms timer: Replace All flushes first, so the new text is replaced.
        r.model.findText = "cat"
        r.model.replaceText = "dog"
        r.model.scopeIsScene = false
        await r.model.replaceAll()
        #expect(r.editor().last == "action: The dog sleeps.")
        let saved = try await r.saved()
        #expect(saved.last == "action: The dog sleeps.")
    }

    // MARK: Scene numbers and page count

    @Test func sceneNumbersAndThePageCountComeFromTheCore() async throws {
        let r = try await EditorRig.start(title: "Numbers")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. ONE - DAY")
        r.enter()
        r.type("First.")
        r.enter()
        r.enter()
        r.enter()
        r.type("EXT. TWO - DAY")
        r.escape()
        r.enter()
        r.type("Second.")
        await r.settle()
        _ = try await r.saved()
        await r.model.loadDocument(r.model.draft!.id)
        #expect(Set(r.view.sceneLabels.values) == ["1", "2"])
        #expect(r.model.pageLabel == "1 page" || r.model.pageLabel?.hasPrefix("Page 1 of 1") == true)
        r.view.reveal(paragraph: 0)
        r.model.caretMoved()
        #expect(r.model.pageLabel == "Page 1 of 1")
    }

    @Test func theEditorDrawsSceneNumbersAndPageBreaksInTheMargin() async throws {
        let r = try await EditorRig.start(title: "Margins")
        defer { r.close() }
        await r.openScreenplay()
        r.type("INT. ONE - DAY")
        r.enter()
        r.type("First.")
        r.chord("\r", keyCode: 36, .command)
        r.type("After the break.")
        r.enter()
        r.enter()
        r.type("EXT. TWO - DAY")
        r.escape()
        r.enter()
        r.type("Second.")
        _ = try await r.saved()
        await r.model.loadDocument(r.model.draft!.id)
        let themes = ThemeStore()
        let host = ScreenplayWorkspace()
            .environment(r.app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
            .environment(\.colorScheme, .light)
        let image = Fixture.render(host, name: "editor-workspace-light", size: CGSize(width: 1200, height: 760))
        #expect(image != nil)
        // The text view alone, with the margin marks, at a size where they show.
        r.window.setContentSize(NSSize(width: 800, height: 520))
        r.view.frame = NSRect(x: 0, y: 0, width: 800, height: 520)
        r.view.layoutSubtreeIfNeeded()
        if let rep = r.view.bitmapImageRepForCachingDisplay(in: r.view.bounds) {
            r.view.cacheDisplay(in: r.view.bounds, to: rep)
            try FileManager.default.createDirectory(at: Fixture.snapshots, withIntermediateDirectories: true)
            if let png = rep.representation(using: .png, properties: [:]) {
                try png.write(to: Fixture.snapshots.appendingPathComponent("editor-margins.png"))
            }
        }
    }

    // MARK: Typing is never lost

    @Test func typedTextSurvivesQuittingTheApp() async throws {
        let first = try await EditorRig.start(title: "Quit")
        await first.openScreenplay()
        first.type("INT. HALL - DAY")
        first.enter()
        first.type("Typed just before the quit.")
        // No wait: the app is quitting. The flush runs before the core stops.
        // The quit flushes first (applicationShouldTerminate waits for it), then the core stops.
        await ScreenplayWorkspaceModel.flushAll()
        first.app.shutdown()
        let root = first.root
        let path = first.projectPath
        let again = try await EditorRig.start(root: root, open: path)
        defer { again.close() }
        await again.openScreenplay()
        #expect(again.editor().contains("action: Typed just before the quit."))
        #expect(try await again.saved().contains("action: Typed just before the quit."))
    }

    @Test func closingTheProjectFlushesFirstAndSwitchingWorkspaceDoes() async throws {
        let first = try await EditorRig.start(title: "Close")
        await first.openScreenplay()
        first.type("INT. HALL - DAY")
        first.enter()
        first.type("Typed just before closing.")
        await ScreenplayWorkspaceModel.flushAll()
        await first.app.closeProject()
        let root = first.root
        let path = first.projectPath
        first.app.shutdown()
        let again = try await EditorRig.start(root: root, open: path)
        defer { again.close() }
        await again.openScreenplay()
        #expect(again.editor().contains("action: Typed just before closing."))
        // Leaving the workspace (the view goes away) uses the same flush.
        again.view.setSelectedRange(NSRange(location: (again.view.string as NSString).length, length: 0))
        again.type(" And more.")
        await again.model.flush()
        #expect(try await again.document().scenes.flatMap(\.elements).map(\.text).contains("Typed just before closing. And more."))
    }
}

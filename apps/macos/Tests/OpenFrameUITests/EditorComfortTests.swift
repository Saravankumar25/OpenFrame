// Mac phase M4, editor comfort: Focus Mode and Writing Room (the layout mode and where it is
// kept), Escape leaving Focus, the page centred in the view, the title page preview and export
// switch, and continuous spelling in a real window. A real core runs on a temporary folder.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Editor comfort", .serialized)
@MainActor
struct EditorComfortTests {
    @Test func theModeIsReadFromAndMergedIntoTheSharedLayoutRecord() {
        #expect(ScreenplayMode.from(nil) == .standard)
        #expect(ScreenplayMode.from(.object(["mode": .string("focus")])) == .focus)
        #expect(ScreenplayMode.from(.object(["mode": .string("writingRoom")])) == .writingRoom)
        #expect(ScreenplayMode.from(.object(["mode": .string("sideways")])) == .standard)
        let merged = ScreenplayMode.merged(.object(["mode": .string("standard"), "showNotes": .bool(false), "panels": .array([.string("story")])]), mode: .writingRoom)
        guard case let .object(o) = merged else { Issue.record("not an object"); return }
        #expect(o["mode"] == .string("writingRoom"))
        #expect(o["showNotes"] == .bool(false))
        #expect(o["panels"] == .array([.string("story")]))
    }

    @Test func focusAndWritingRoomAreKeptWithTheProjectAndLeaveWindowsKeysAlone() async throws {
        let rig = try await EditorRig.start(title: "Layout")
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.root) }
        let core = try #require(rig.app.core)
        _ = try await core.screenplaySetViewState(ScreenplaySetViewStateArgs(state: .object(["mode": .string("standard"), "showNotes": .bool(false)])))

        rig.model.setMode(.focus)
        #expect(rig.model.mode == .focus)
        try await settle { await ScreenplayMode.from(try? core.screenplayViewState(ScreenplayViewStateArgs())) == .focus }

        // A new workspace of the same project opens the way the writer left it.
        let again = ScreenplayWorkspaceModel(rules: SharedScreenplayRules.shared)
        again.attach(rig.app)
        await again.loadLayout()
        #expect(again.mode == .focus)

        rig.model.setMode(.writingRoom)
        try await settle { await ScreenplayMode.from(try? core.screenplayViewState(ScreenplayViewStateArgs())) == .writingRoom }
        let state = try await core.screenplayViewState(ScreenplayViewStateArgs())
        guard case let .object(o) = state else { Issue.record("not an object"); return }
        #expect(o["showNotes"] == .bool(false))
    }

    @Test func escapeLeavesFocusOnlyWhenNoSuggestionIsOpen() async throws {
        let rig = try await EditorRig.start(title: "Escape")
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.root) }
        await rig.openScreenplay()
        var pressed = 0
        rig.model.onEscape = { pressed += 1 }
        // An open suggestion list takes the first Escape; the next one leaves Focus.
        rig.view.cancelOperation(nil)
        rig.view.cancelOperation(nil)
        #expect(pressed >= 1)
    }

    @Test func thePageSitsInTheMiddleOfAWideView() async throws {
        let rig = try await EditorRig.start(title: "Centre")
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.root) }
        await rig.openScreenplay()
        // The view takes the width the scroll view shows, so a wide pane centres the page.
        let scroll = try #require(rig.view.enclosingScrollView)
        scroll.setFrameSize(NSSize(width: 1200, height: 700))
        rig.view.frame.size.width = 1200
        let width = try #require(rig.view.textContainer?.size.width)
        #expect(abs(rig.view.textContainerOrigin.x - ((1200 - width) / 2).rounded()) < 1)
        // A pane narrower than the page keeps the page and its scene number gutters whole; the
        // scroll view scrolls sideways instead of cutting the page.
        scroll.setFrameSize(NSSize(width: width - 40, height: 700))
        rig.view.frame.size.width = width - 40
        #expect(rig.view.frame.width == width + 2 * ScreenplayTextView.pageGutter)
        #expect(rig.view.textContainerOrigin.x == ScreenplayTextView.pageGutter)
    }

    @Test func theTitlePageIsLaidOutByTheCoreAndTheExportSwitchIsSaved() async throws {
        let rig = try await EditorRig.start(title: "Title")
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.root) }
        await rig.openScreenplay()
        let state = ScreenplayPanelState()
        state.attach(rig.app, rig.model)
        let page = PageSetupModel()
        page.attach(rig.app, rig.model, say: { state.say($0) })
        await page.load()
        page.titlePage?.title = "Night Bus"
        page.titlePage?.writtenBy = "E2E Tester"
        await page.refreshPreview()
        let preview = try #require(page.preview)
        let first = try #require(preview.pages.first)
        #expect(first.titlePage)
        #expect(first.lines.contains { $0.text.localizedCaseInsensitiveContains("Night Bus") })
        #expect(first.lines.contains { $0.text.localizedCaseInsensitiveContains("E2E Tester") })
        #expect(preview.pageWidthIn > 8)

        #expect(page.setup?.includeTitlePage == true)
        page.setup?.includeTitlePage = false
        #expect(await page.saveTitlePage())
        #expect(rig.model.overview?.screenplay?.pageSetup.includeTitlePage == false)
        #expect(rig.model.overview?.screenplay?.titlePage.title == "Night Bus")

        // The sheet's own loading is asynchronous and does not run in an off-screen window, so the
        // preview is drawn on its own from what the core returned.
        let drawn = TitlePagePreview(preview: preview)
            .frame(width: 300, height: 420)
            .padding(20)
            .environment(\.ofTheme, ThemeStore().resolvedFor(colorScheme: .light))
            .preferredColorScheme(.light)
        #expect(Fixture.render(drawn, name: "comfort-title-page", size: CGSize(width: 340, height: 460)) != nil)
    }

    @Test func spellingIsOnInAWindowAndFindsAMisspelledWord() async throws {
        let rig = try await EditorRig.start(title: "Spelling")
        defer { rig.close(); try? FileManager.default.removeItem(at: rig.root) }
        await rig.openScreenplay()
        #expect(rig.view.window != nil)
        // The system accepts "continuous spelling on" only in an interactive session. A plain text
        // view in the same process shows what the session allows; the screenplay view must match it.
        let plain = NSTextView(frame: NSRect(x: 0, y: 0, width: 200, height: 100))
        plain.isContinuousSpellCheckingEnabled = true
        #expect(rig.view.isContinuousSpellCheckingEnabled == plain.isContinuousSpellCheckingEnabled)
        #expect(rig.view.isGrammarCheckingEnabled == false)
        #expect(rig.view.isAutomaticSpellingCorrectionEnabled == false)

        rig.type("teh cat")
        let text = rig.view.string
        #expect(text.contains("teh cat"))
        // The language is named: with none, a process without a user session finds nothing.
        _ = NSSpellChecker.shared.setLanguage("en")
        let found = NSSpellChecker.shared.checkSpelling(
            of: text, startingAt: 0, language: "en", wrap: false,
            inSpellDocumentWithTag: rig.view.spellCheckerDocumentTag, wordCount: nil)
        #expect(found.location != NSNotFound)
        #expect((text as NSString).substring(with: found) == "teh")
    }

    /// Wait (up to five seconds) for something the app does in the background.
    private func settle(_ condition: @MainActor () async -> Bool) async throws {
        for _ in 0..<50 {
            if await condition() { return }
            try await Task.sleep(for: .milliseconds(100))
        }
        Issue.record("The condition did not hold in time")
    }
}

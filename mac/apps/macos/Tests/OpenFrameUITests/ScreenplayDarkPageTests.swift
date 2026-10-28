// Screenplay "Dark page" (Windows commit 2dc8e6e7, paperColors.test.ts and darkPaper.test.tsx):
// every text and background pair the page uses stays readable on the dark page, the light page is
// exactly the paper it always was (white, also in the Mac Dark theme), and the choice is a view
// preference kept in the `screenplay.view_state` record under "darkPaper".

import AppKit
import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("ScreenplayDarkPageTests", .serialized)
@MainActor
struct ScreenplayDarkPageTests {
    typealias C = ScreenplayPageColors

    /// [text colour, background, what it is, minimum]. The placeholder is a hint: 3:1 there.
    private static let pairs: [(KeyPath<C.Palette, String>, KeyPath<C.Palette, String>, String, Double)] = [
        (\.ink, \.paper, "script text", 4.5),
        (\.muted, \.paper, "scene numbers", 4.5),
        (\.faint, \.paper, "page-break label", 4.5),
        (\.noteInk, \.noteBg, "note", 4.5),
        (\.noteInk, \.noteCurBg, "note under the cursor", 4.5),
        (\.ink, \.curBg, "line under the cursor", 4.5),
        (\.ink, \.cmtBg, "commented text", 4.5),
        (\.hlInk, \.hlBg, "find match", 4.5),
        (\.hlInk, \.hlcBg, "current find match", 4.5),
        (\.hint, \.paper, "placeholder", 3),
    ]

    @Test func theDarkPageIsDarkerThanItsText() {
        #expect(C.luminance(C.dark.paper) < 0.05)
        #expect(C.luminance(C.dark.ink) > 0.6)
    }

    @Test func everyPairStaysReadableOnTheDarkPage() {
        for (fg, bg, what, min) in Self.pairs {
            let ratio = C.contrast(C.dark[keyPath: fg], C.dark[keyPath: bg])
            #expect(ratio >= min, "\(what): \(ratio)")
        }
        let sel = C.contrast(try! #require(C.dark.selInk), try! #require(C.dark.selBg))
        #expect(sel >= 4.5, "selection: \(sel)")
    }

    @Test func theLightPageIsExactlyAsItWas() {
        #expect(C.light.paper == "#ffffff")
        #expect(C.light.ink == "#111111")
        #expect(C.light.muted == "#555555")
        #expect(C.light.noteBg == "#eee8fb")
        #expect(C.light.hlBg == "#ffe27a")
        #expect(C.light.selBg == nil)
    }

    @Test func onlyTrueIsADarkPageAndOtherKeysStay() {
        #expect(C.isDark(nil) == false)
        #expect(C.isDark(.object(["mode": .string("standard"), "panels": .array([])])) == false)
        #expect(C.isDark(.object(["darkPaper": .string("yes")])) == false)
        #expect(C.isDark(.object(["darkPaper": .bool(true)])) == true)
        let merged = C.merged(.object(["mode": .string("focus")]), dark: true)
        #expect(merged == .object(["mode": .string("focus"), "darkPaper": .bool(true)]))
        #expect(C.menuTitle(dark: false) == "Dark page")
        #expect(C.menuTitle(dark: true) == "Light page")
    }

    /// The page view: white paper even when the app is in Dark; Dark page turns only the page.
    @Test func thePageIsWhiteByDefaultInBothThemesAndDarkWhenChosen() {
        let view = ScreenplayTextView.make()
        view.appearance = NSAppearance(named: .darkAqua)
        view.showPaper(dark: false)
        #expect(view.appearance?.name == .aqua)
        #expect(view.backgroundColor == C.color("#ffffff"))
        var ink = NSColor.black
        view.effectiveAppearance.performAsCurrentDrawingAppearance { ink = C.ink.usingColorSpace(.sRGB) ?? .black }
        #expect(ink == C.color("#111111").usingColorSpace(.sRGB))
        view.showPaper(dark: true)
        #expect(view.appearance?.name == .darkAqua)
        #expect(view.backgroundColor == C.color("#1e1e22"))
        view.effectiveAppearance.performAsCurrentDrawingAppearance { ink = C.ink.usingColorSpace(.sRGB) ?? .black }
        #expect(ink == C.color("#ece9e2").usingColorSpace(.sRGB))
    }

    @Test func theChoiceIsSavedWithTheLayoutAndComesBack() async throws {
        let r = try await EditorRig.start(title: "Night Page")
        defer { r.close() }
        await r.openScreenplay()
        let core = r.app.core!
        _ = try await core.screenplaySetViewState(ScreenplaySetViewStateArgs(state: .object(["mode": .string("writingRoom")])))
        await r.model.loadLayout()
        #expect(r.model.darkPaper == false)
        r.model.toggleDarkPaper()
        #expect(r.model.darkPaper)
        var saved = false
        for _ in 0..<100 where !saved {
            saved = C.isDark(try? await core.screenplayViewState(ScreenplayViewStateArgs()))
            if !saved { try? await Task.sleep(for: .milliseconds(40)) }
        }
        #expect(saved)
        let state = try await core.screenplayViewState(ScreenplayViewStateArgs())
        #expect(ScreenplayMode.from(state) == .writingRoom)
        let again = ScreenplayWorkspaceModel(rules: SharedScreenplayRules.shared)
        again.attach(r.app)
        await again.loadLayout()
        #expect(again.darkPaper)
        r.model.toggleDarkPaper()
        var light = false
        for _ in 0..<100 where !light {
            light = !C.isDark(try? await core.screenplayViewState(ScreenplayViewStateArgs()))
            if !light { try? await Task.sleep(for: .milliseconds(40)) }
        }
        #expect(light)
    }
}

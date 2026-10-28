// The colours of the screenplay page on screen (Windows: the --paper-* properties of .spx-page in
// screenplay.css, commit 2dc8e6e7). The page is white paper by default, also in the Mac Dark
// theme, as on Windows. "Dark page" in Screenplay Tools is a view preference kept in the
// `screenplay.view_state` record (key "darkPaper", shared with Windows); it changes the screen
// only. Print, the print preview and the PDF are drawn by the core and never read these colours.

import AppKit
import Foundation
import OpenFrameCore

enum ScreenplayPageColors {
    /// One set of page colours, as hex strings so the contrast tests read the same values.
    struct Palette: Equatable {
        var paper: String
        var ink: String
        var muted: String
        var faint: String
        var rule: String
        var hint: String
        var noteInk: String
        var noteBg: String
        var noteCurBg: String
        var noteEdge: String
        var curBg: String
        var cmtBg: String
        var cmtEdge: String
        var hlBg: String
        var hlcBg: String
        var hlcEdge: String
        var hlInk: String
        var selBg: String?
        var selInk: String?
    }

    /// The paper the page has always had (screenplay.css .spx-page).
    static let light = Palette(
        paper: "#ffffff", ink: "#111111", muted: "#555555", faint: "#6b665c", rule: "#9a9488", hint: "#b3ada1",
        noteInk: "#4c33a0", noteBg: "#eee8fb", noteCurBg: "#efe6ff", noteEdge: "#7a5ac8",
        curBg: "#fff6e6", cmtBg: "#fff1c2", cmtEdge: "#e0b54a",
        hlBg: "#ffe27a", hlcBg: "#ffc94a", hlcEdge: "#d9a520", hlInk: "#111111",
        selBg: nil, selInk: nil)

    /// Dark page (screenplay.css .spx-page[data-paper="dark"]); the find colours stay as on the light page.
    static let dark = Palette(
        paper: "#1e1e22", ink: "#ece9e2", muted: "#a9a59b", faint: "#a9a59b", rule: "#6f6b62", hint: "#8a867c",
        noteInk: "#d4c8fa", noteBg: "#2d2646", noteCurBg: "#382f59", noteEdge: "#8f74dc",
        curBg: "#2b281d", cmtBg: "#4d4220", cmtEdge: "#d4a93c",
        hlBg: "#ffe27a", hlcBg: "#ffc94a", hlcEdge: "#d9a520", hlInk: "#111111",
        selBg: "#3f6ac4", selInk: "#ffffff")

    static func palette(dark: Bool) -> Palette { dark ? Self.dark : Self.light }

    // MARK: The saved choice

    /// The choice a saved layout holds: only `true` is a dark page (Windows normalizeLayout).
    static func isDark(_ state: JSONValue?) -> Bool {
        if case let .object(o)? = state, case .bool(true)? = o["darkPaper"] { return true }
        return false
    }

    /// The saved layout with only "darkPaper" changed (the other keys stay).
    static func merged(_ state: JSONValue?, dark: Bool) -> JSONValue {
        var o: [String: JSONValue] = [:]
        if case let .object(existing)? = state { o = existing }
        o["darkPaper"] = .bool(dark)
        return .object(o)
    }

    /// The View menu item: it names the page the person can switch to.
    static func menuTitle(dark: Bool) -> String { dark ? "Light page" : "Dark page" }

    // MARK: Drawing

    /// The appearance the text view takes so the dynamic colours below pick the page's set.
    static func appearance(dark: Bool) -> NSAppearance? { NSAppearance(named: dark ? .darkAqua : .aqua) }

    /// A colour that follows the page: the text view's appearance is pinned to the page choice.
    static func dynamic(light: String, dark: String) -> NSColor {
        NSColor(name: nil) { appearance in
            color(appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? dark : light)
        }
    }

    nonisolated(unsafe) static let ink = dynamic(light: light.ink, dark: dark.ink)
    nonisolated(unsafe) static let paper = dynamic(light: light.paper, dark: dark.paper)
    nonisolated(unsafe) static let muted = dynamic(light: light.muted, dark: dark.muted)
    nonisolated(unsafe) static let faint = dynamic(light: light.faint, dark: dark.faint)
    nonisolated(unsafe) static let rule = dynamic(light: light.rule, dark: dark.rule)

    /// An sRGB colour from "#rrggbb".
    static func color(_ hex: String) -> NSColor {
        let (r, g, b) = rgb(hex)
        return NSColor(srgbRed: r, green: g, blue: b, alpha: 1)
    }

    static func rgb(_ hex: String) -> (Double, Double, Double) {
        let s = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
        let v = UInt32(s, radix: 16) ?? 0
        return (Double((v >> 16) & 0xff) / 255, Double((v >> 8) & 0xff) / 255, Double(v & 0xff) / 255)
    }

    /// Relative luminance (WCAG 2), as the Windows contrast test computes it.
    static func luminance(_ hex: String) -> Double {
        let (r, g, b) = rgb(hex)
        func lin(_ v: Double) -> Double { v <= 0.03928 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }
        return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    }

    static func contrast(_ a: String, _ b: String) -> Double {
        let x = luminance(a), y = luminance(b)
        return (max(x, y) + 0.05) / (min(x, y) + 0.05)
    }
}

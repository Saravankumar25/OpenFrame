// Theme engine for the Mac shell. It computes the same tokens as the CSS engine in
// apps/desktop/src/theme/engine.ts from the same themes.json: an OKLCH ladder per mode, the accent,
// status colours, derived colours, role tokens, logo colours, then per-theme overrides.
import Foundation
import SwiftUI

/// Display values of one token, gamma-encoded sRGB with alpha, each 0 to 1.
public struct TokenColor: Sendable, Equatable {
    public let r: Double
    public let g: Double
    public let b: Double
    public let a: Double

    /// `#rrggbb` (alpha left out) for logs and tests.
    public var hex: String {
        func byte(_ v: Double) -> Int { Int((v * 255).rounded()) }
        return String(format: "#%02x%02x%02x", byte(r), byte(g), byte(b))
    }
}

/// Every token of one theme in one mode.
public struct ResolvedTheme: Sendable {
    public let id: String
    public let mode: ColorMode
    /// Tokens that could not be evaluated (and use a neutral stand-in). Empty for every shipped theme.
    public let unresolved: [String]
    private let colors: [String: TokenColor]

    init(id: String, mode: ColorMode, colors: [String: TokenColor], unresolved: [String]) {
        self.id = id
        self.mode = mode
        self.colors = colors
        self.unresolved = unresolved
    }

    public func has(_ token: String) -> Bool { colors[token] != nil }

    public func tokenColor(_ token: String) -> TokenColor? { colors[token] }

    /// A missing token shows as magenta so the mistake is visible instead of silent.
    public subscript(token: String) -> Color {
        let c = rgba(token)
        return Color(.sRGB, red: c.r, green: c.g, blue: c.b, opacity: c.a)
    }

    public func rgba(_ token: String) -> (r: Double, g: Double, b: Double, a: Double) {
        guard let c = colors[token] else { return (1, 0, 1, 1) }
        return (c.r, c.g, c.b, c.a)
    }

    /// WCAG 2 contrast ratio between two opaque tokens.
    public func contrast(_ first: String, _ second: String) -> Double {
        func lum(_ t: String) -> Double {
            let c = rgba(t)
            return 0.2126 * ColorMath.toLinear(c.r) + 0.7152 * ColorMath.toLinear(c.g)
                + 0.0722 * ColorMath.toLinear(c.b)
        }
        let x = lum(first), y = lum(second)
        return (max(x, y) + 0.05) / (min(x, y) + 0.05)
    }

    /// The three logo colours, for a gradient.
    public var logoColors: [Color] { [self["lg1"], self["lg2"], self["lg3"]] }
}

public enum ThemeEngine {
    // MARK: validation (port of sanitize in engine.ts)

    private static let colourPatterns: [NSRegularExpression] = [
        try! NSRegularExpression(pattern: "^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"),
    ]
    private static let oklchPattern = try! NSRegularExpression(
        pattern: "^oklch\\((\\d{1,3}(?:\\.\\d{1,4})?)% (\\d?\\.?\\d{1,4}) (\\d{1,3}(?:\\.\\d{1,4})?)\\)$")
    private static let alphaHex = try! NSRegularExpression(pattern: "^#(?:[0-9a-fA-F]{4}|[0-9a-fA-F]{8})$")

    private static func whole(_ re: NSRegularExpression, _ s: String) -> NSTextCheckingResult? {
        re.firstMatch(in: s, range: NSRange(s.startIndex..., in: s))
    }

    private static func isValidId(_ s: String) -> Bool {
        (3...40).contains(s.utf8.count)
            && s.utf8.allSatisfy { ($0 >= 97 && $0 <= 122) || ($0 >= 48 && $0 <= 57) || $0 == 45 }
    }

    private static func isValidToken(_ s: String) -> Bool {
        (1...40).contains(s.utf8.count)
            && s.utf8.allSatisfy { ($0 >= 97 && $0 <= 122) || ($0 >= 48 && $0 <= 57) || $0 == 45 }
    }

    /// True for a colour value the engine accepts: `#hex` or a literal `oklch(L% C H)` in range.
    public static func isSafeColour(_ v: String) -> Bool {
        if whole(colourPatterns[0], v) != nil { return true }
        guard let m = whole(oklchPattern, v) else { return false }
        func part(_ i: Int) -> Double? {
            Range(m.range(at: i), in: v).flatMap { Double(v[$0]) }
        }
        guard let l = part(1), let c = part(2), let h = part(3) else { return false }
        return l >= 0 && l <= 100 && c >= 0 && c <= 0.4 && h >= 0 && h <= 360
    }

    private static func inRange(_ v: Double, _ lo: Double, _ hi: Double) -> Bool {
        v.isFinite && v >= lo && v <= hi
    }

    /// The theme when every value is within bounds, otherwise nil.
    /// Imported themes (`builtIn` false) may set only the importable tokens, without alpha.
    public static func sanitize(_ t: ThemeData, importable: [String]) -> ThemeData? {
        guard isValidId(t.id) else { return nil }
        let s = t.seeds
        guard inRange(s.neutralHue, 0, 360), inRange(s.neutralTint, 0, 0.04),
            inRange(s.accentHue, 0, 360), inRange(s.accentChroma, 0, 0.25),
            inRange(s.accentLightnessNudge, -12, 16)
        else { return nil }
        func side(_ m: [String: String]) -> Bool {
            for (k, v) in m {
                guard isValidToken(k), isSafeColour(v) else { return false }
                if !t.builtIn && (!importable.contains(k) || whole(alphaHex, v) != nil) { return false }
            }
            return true
        }
        return side(t.light) && side(t.dark) ? t : nil
    }

    // MARK: resolving

    /// The tokens of a theme in a mode. An unknown or invalid theme falls back to OpenFrame Default.
    public static func resolve(themeId: String, mode: ColorMode, themes: ThemeCatalog) -> ResolvedTheme {
        let engine = themes.engine
        var theme = themes.theme(id: themeId)
        if let found = theme {
            // Trust the built-in flag only for themes that come from themes.json.
            let isBuiltIn = themes.builtIn.contains { $0.id == found.id }
            var checked = found
            checked.builtIn = isBuiltIn
            theme = sanitize(checked, importable: engine.importableTokens)
        }
        let chosen = theme
            ?? themes.theme(id: ThemeCatalog.defaultThemeId)
            ?? themes.builtIn.first
        guard let chosen else {
            return ResolvedTheme(id: ThemeCatalog.defaultThemeId, mode: mode, colors: [:], unresolved: ["catalog-empty"])
        }
        return build(chosen, mode: mode, engine: engine)
    }

    private static func fmt(_ n: Double) -> String {
        let r = (n * 10000).rounded() / 10000
        return r == r.rounded() ? String(Int(r)) : String(r)
    }

    private static func build(_ theme: ThemeData, mode: ColorMode, engine e: EngineSpec) -> ResolvedTheme {
        let m = mode.rawValue
        let seeds = theme.seeds
        let r = FormulaResolver()
        var parseFailures: [String] = []

        func define(_ name: String, _ formula: String) {
            if let expr = FormulaParser.parse(formula) {
                r.exprs[name] = expr
            } else {
                parseFailures.append(name)
            }
        }

        // Ladder: oklch(L% calc(var(--c) * k) var(--h)).
        for (name, pair) in e.ladder[m] ?? [:] where pair.count == 2 {
            r.exprs[name] = .color(OKLab(oklchL: pair[0] / 100, c: seeds.neutralTint * pair[1], h: seeds.neutralHue))
        }
        // Accent: oklch(calc(L% + var(--ak)) var(--ac) var(--ah)).
        let accentL = ((e.accentLightness[m] ?? 56) + seeds.accentLightnessNudge) / 100
        r.exprs["accent"] = .color(OKLab(oklchL: accentL, c: seeds.accentChroma, h: seeds.accentHue))
        if mode == .light {
            define("accent-ink", e.lightAccentInk)
        } else if e.darkAccentInk.count == 2 {
            r.exprs["accent-ink"] = .color(OKLab(oklchL: e.darkAccentInk[0] / 100, c: e.darkAccentInk[1], h: seeds.accentHue))
        }
        let softMix = e.softMix[m] ?? 14
        let statusL = e.statusLightness[m] ?? 54
        r.scalars["mix"] = (softMix, true)
        r.scalars["sl"] = (statusL, true)
        for (name, v) in e.status where v.count == 3 {
            r.exprs[name] = .color(OKLab(oklchL: (statusL + v[0]) / 100, c: v[1], h: v[2]))
        }

        // Derived colours.
        define("accent-d", "color-mix(in oklab,var(--accent) \(fmt(e.accentStrongMix))%,var(--ink))")
        define("accent-soft", "color-mix(in oklab,var(--accent) var(--mix),var(--panel))")
        define("accent-line", "color-mix(in oklab,var(--accent) \(fmt(e.accentLineMix))%,var(--panel))")
        define("accent-ring", "color-mix(in oklab,var(--accent) \(fmt(e.accentRingMix))%,transparent)")
        for name in e.status.keys {
            let text = name == "yellow" ? e.warningTextMix : e.statusTextMix
            let line = name == "yellow" ? e.warningLineMix : e.statusLineMix
            define("\(name)-soft", "color-mix(in oklab,var(--\(name)) var(--mix),var(--panel))")
            define("\(name)-t", "color-mix(in oklab,var(--\(name)) \(fmt(text))%,var(--ink))")
            define("\(name)-line", "color-mix(in oklab,var(--\(name)) \(fmt(line))%,var(--panel))")
        }
        for (name, formula) in e.roles { define(name, formula) }
        for (name, formula) in e.logo { define(name, formula) }

        // Per-theme overrides win over everything above.
        for (name, value) in theme.overrides(for: mode) {
            if let expr = FormulaParser.parse(value) {
                r.exprs[name] = expr
            } else {
                parseFailures.append(name)
            }
        }

        if r.exprs["muted"] != nil { r.fallback = r.value(of: "muted") }

        var colors: [String: TokenColor] = [:]
        for name in r.exprs.keys.sorted() {
            let lab = r.value(of: name)
            let rgb = lab.srgb
            colors[name] = TokenColor(r: rgb.r, g: rgb.g, b: rgb.b, a: min(1, max(0, lab.alpha)))
        }
        let unresolved = Set(parseFailures).union(r.failed).sorted()
        return ResolvedTheme(id: theme.id, mode: mode, colors: colors, unresolved: unresolved)
    }
}

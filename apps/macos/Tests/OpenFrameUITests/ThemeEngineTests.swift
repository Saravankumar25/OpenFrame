// Tests for the Mac theme engine against themes.json and an independent reference (see
// scripts/gen-theme-reference.mjs). Swift Testing, no XCTest.
import Foundation
import Testing

@testable import OpenFrameUI

private enum Fixtures {
    static let themesRelativePath = "crates/openframe-application/src/modules/appearance/themes.json"

    /// themes.json, found by walking up from this file, or from OPENFRAME_REPO_ROOT.
    static func themesURL() -> URL {
        var dir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        for _ in 0..<8 {
            let candidate = dir.appendingPathComponent(themesRelativePath)
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
            dir.deleteLastPathComponent()
        }
        let root = ProcessInfo.processInfo.environment["OPENFRAME_REPO_ROOT"] ?? ""
        return URL(fileURLWithPath: root).appendingPathComponent(themesRelativePath)
    }

    static func catalog() throws -> ThemeCatalog {
        try ThemeCatalog(json: Data(contentsOf: themesURL()))
    }

    struct Reference: Decodable {
        var tokens: [String]
        var themes: [String: [String: [String: String]]]
    }

    static func reference() throws -> Reference {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("theme-reference.json")
        return try JSONDecoder().decode(Reference.self, from: Data(contentsOf: url))
    }

    static func channels(_ hex: String) -> [Int] {
        let d = Array(hex.dropFirst())
        return [0, 2, 4].map { Int(String(d[$0...$0 + 1]), radix: 16) ?? -1 }
    }

    static func importedTheme(
        id: String = "my-theme", builtIn: Bool = false, hue: Double = 200,
        light: [String: String] = [:], dark: [String: String] = [:]
    ) -> ThemeData {
        ThemeData(
            id: id, name: "Mine", description: "", author: "Me", builtIn: builtIn,
            seeds: ThemeSeeds(
                neutralHue: hue, neutralTint: 0.01, accentHue: 30, accentChroma: 0.12,
                accentLightnessNudge: 0),
            light: light, dark: dark)
    }
}

@Suite struct ThemeEngineTests {
    @Test func sixteenBuiltInThemes() throws {
        let catalog = try Fixtures.catalog()
        #expect(catalog.builtIn.count == 16)
        #expect(catalog.builtIn.first?.id == "openframe-default")
    }

    @Test func everyThemeAndModeResolvesWithoutUnresolvedTokens() throws {
        let catalog = try Fixtures.catalog()
        let e = catalog.engine
        for theme in catalog.builtIn {
            for mode in ColorMode.allCases {
                let r = ThemeEngine.resolve(themeId: theme.id, mode: mode, themes: catalog)
                #expect(r.id == theme.id)
                #expect(r.unresolved.isEmpty, "\(theme.id) \(mode): \(r.unresolved)")
                var expected = Set(e.ladder[mode.rawValue]?.keys.map { $0 } ?? [])
                expected.formUnion(["accent", "accent-ink", "accent-d", "accent-soft", "accent-line", "accent-ring"])
                for name in e.status.keys {
                    expected.formUnion([name, "\(name)-soft", "\(name)-t", "\(name)-line"])
                }
                expected.formUnion(e.roles.keys)
                expected.formUnion(e.logo.keys)
                for token in expected {
                    #expect(r.has(token), "\(theme.id) \(mode) lacks \(token)")
                }
            }
        }
    }

    @Test func matchesIndependentReference() throws {
        let catalog = try Fixtures.catalog()
        let ref = try Fixtures.reference()
        #expect(ref.themes.count == 16)
        for theme in catalog.builtIn {
            for mode in ColorMode.allCases {
                let r = ThemeEngine.resolve(themeId: theme.id, mode: mode, themes: catalog)
                for token in ref.tokens {
                    let want = Fixtures.channels(try #require(ref.themes[theme.id]?[mode.rawValue]?[token]))
                    let got = Fixtures.channels(try #require(r.tokenColor(token)).hex)
                    for i in 0..<3 {
                        #expect(
                            abs(want[i] - got[i]) <= 1,
                            "\(theme.id) \(mode) \(token): want \(want) got \(got)")
                    }
                }
            }
        }
    }

    @Test func transparentMixKeepsAlpha() throws {
        let catalog = try Fixtures.catalog()
        // The default theme overrides accent-ring to #d9822b33; Studio Amber computes it.
        let r = ThemeEngine.resolve(themeId: "studio-amber", mode: .light, themes: catalog)
        let ring = try #require(r.tokenColor("accent-ring"))
        #expect(abs(ring.a - 0.28) < 0.001)
        let accent = try #require(r.tokenColor("accent"))
        #expect(abs(ring.r - accent.r) < 0.01 && abs(ring.g - accent.g) < 0.01)
        let d = ThemeEngine.resolve(themeId: "openframe-default", mode: .light, themes: catalog)
        #expect(abs(try #require(d.tokenColor("accent-ring")).a - 0.2) < 0.01)
    }

    @Test func relativeLogoColoursFollowTheAccent() throws {
        let catalog = try Fixtures.catalog()
        let r = ThemeEngine.resolve(themeId: "neon-reel", mode: .dark, themes: catalog)
        #expect(r.hasLogo)
        #expect(r.tokenColor("lg2")?.hex == r.tokenColor("accent")?.hex)
        #expect(r.tokenColor("lg1")?.hex != r.tokenColor("lg3")?.hex)
        #expect(r.logoColors.count == 3)
    }

    @Test func invalidImportedThemesFallBackToDefault() throws {
        let base = try Fixtures.catalog()
        let bad: [ThemeData] = [
            Fixtures.importedTheme(id: "Bad Id"),
            Fixtures.importedTheme(id: "ab"),
            Fixtures.importedTheme(hue: 400),
            Fixtures.importedTheme(light: ["ink2": "#112233", "red": "#ff0000"]),  // red is not importable
            Fixtures.importedTheme(light: ["bg": "#11223344"]),  // alpha not allowed
            Fixtures.importedTheme(light: ["bg": "rgb(1,2,3)"]),
            Fixtures.importedTheme(light: ["bg": "oklch(120% 0.1 10)"]),
            Fixtures.importedTheme(dark: ["BG": "#112233"]),
        ]
        for theme in bad {
            let catalog = base.withImported([theme])
            let r = ThemeEngine.resolve(themeId: theme.id, mode: .light, themes: catalog)
            #expect(r.id == "openframe-default", "\(theme.id) \(theme.light) should be rejected")
        }
        let unknown = ThemeEngine.resolve(themeId: "does-not-exist", mode: .dark, themes: base)
        #expect(unknown.id == "openframe-default")
        #expect(unknown.mode == .dark)
    }

    @Test func importedThemeCannotClaimToBeBuiltIn() throws {
        let base = try Fixtures.catalog()
        let sneaky = Fixtures.importedTheme(builtIn: true, light: ["red": "#ff0000"])
        let r = ThemeEngine.resolve(themeId: sneaky.id, mode: .light, themes: base.withImported([sneaky]))
        #expect(r.id == "openframe-default")
    }

    @Test func validImportedThemeResolves() throws {
        let base = try Fixtures.catalog()
        let good = Fixtures.importedTheme(light: ["bg": "#102030", "accent": "oklch(60% 0.12 40)"])
        let r = ThemeEngine.resolve(themeId: good.id, mode: .light, themes: base.withImported([good]))
        #expect(r.id == good.id)
        #expect(r.unresolved.isEmpty)
        #expect(r.tokenColor("bg")?.hex == "#102030")
        // Tokens derived from an overridden accent follow it.
        let plain = Fixtures.importedTheme(id: "plain-one")
        let p = ThemeEngine.resolve(themeId: plain.id, mode: .light, themes: base.withImported([plain]))
        #expect(p.tokenColor("accent-soft")?.hex != r.tokenColor("accent-soft")?.hex)
    }

    @Test func unknownFormulaIsReportedNotCrashed() {
        #expect(FormulaParser.parse("hsl(10 20% 30%)") == nil)
        #expect(FormulaParser.parse("color-mix(in srgb,#fff 50%,#000)") == nil)
        #expect(FormulaParser.parse("var(--ink)") != nil)
        let r = FormulaResolver()
        r.exprs["a"] = FormulaParser.parse("var(--b)")
        r.exprs["b"] = FormulaParser.parse("var(--a)")
        _ = r.value(of: "a")
        #expect(r.failed.contains("a") || r.failed.contains("b"))
    }

    @Test func highContrastMeetsAAA() throws {
        let catalog = try Fixtures.catalog()
        for mode in ColorMode.allCases {
            let r = ThemeEngine.resolve(themeId: "high-contrast", mode: mode, themes: catalog)
            #expect(r.contrast("ink", "bg") >= 7, "\(mode) ink on bg")
            #expect(r.contrast("ink", "panel") >= 7, "\(mode) ink on panel")
            #expect(r.contrast("side-ink", "side") >= 7, "\(mode) side ink on side")
        }
    }

    @Test func defaultAccentInkIsReadableOnAccent() throws {
        let catalog = try Fixtures.catalog()
        for mode in ColorMode.allCases {
            let r = ThemeEngine.resolve(themeId: "openframe-default", mode: mode, themes: catalog)
            #expect(r.contrast("accent", "accent-ink") >= 4.5, "\(mode) accent-ink on accent")
        }
    }
}

extension ResolvedTheme {
    fileprivate var hasLogo: Bool { has("lg1") && has("lg2") && has("lg3") }
}

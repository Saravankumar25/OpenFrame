// Theme data model (ADR-0014). Mirrors crates/openframe-application/src/modules/appearance/themes.json,
// the single source of truth shared with the Windows engine (apps/desktop/src/theme/engine.ts).
import Foundation

public enum ColorMode: String, Sendable, Hashable, CaseIterable {
    case light
    case dark
}

public enum AppearancePreference: String, Sendable, Hashable, CaseIterable, Codable {
    case system
    case light
    case dark
}

/// The five numbers that drive a whole theme.
public struct ThemeSeeds: Codable, Sendable, Hashable {
    public var neutralHue: Double
    public var neutralTint: Double
    public var accentHue: Double
    public var accentChroma: Double
    public var accentLightnessNudge: Double

    public init(
        neutralHue: Double, neutralTint: Double, accentHue: Double,
        accentChroma: Double, accentLightnessNudge: Double
    ) {
        self.neutralHue = neutralHue
        self.neutralTint = neutralTint
        self.accentHue = accentHue
        self.accentChroma = accentChroma
        self.accentLightnessNudge = accentLightnessNudge
    }
}

/// A theme as the engine needs it. Built-in entries of themes.json and the ThemeDto of an imported
/// theme share this shape (`light` and `dark` hold the per-theme token overrides).
public struct ThemeData: Codable, Sendable, Hashable, Identifiable {
    public var id: String
    public var name: String
    public var description: String
    public var author: String
    public var builtIn: Bool
    public var seeds: ThemeSeeds
    public var light: [String: String]
    public var dark: [String: String]

    public init(
        id: String, name: String, description: String, author: String, builtIn: Bool,
        seeds: ThemeSeeds, light: [String: String] = [:], dark: [String: String] = [:]
    ) {
        self.id = id
        self.name = name
        self.description = description
        self.author = author
        self.builtIn = builtIn
        self.seeds = seeds
        self.light = light
        self.dark = dark
    }

    public func overrides(for mode: ColorMode) -> [String: String] {
        mode == .light ? light : dark
    }
}

/// The engine constants of themes.json.
public struct EngineSpec: Codable, Sendable {
    public var ladder: [String: [String: [Double]]]
    public var accentLightness: [String: Double]
    public var statusLightness: [String: Double]
    public var softMix: [String: Double]
    public var darkAccentInk: [Double]
    public var lightAccentInk: String
    public var accentStrongMix: Double
    public var accentLineMix: Double
    public var accentRingMix: Double
    public var statusTextMix: Double
    public var warningTextMix: Double
    public var statusLineMix: Double
    public var warningLineMix: Double
    public var status: [String: [Double]]
    public var roles: [String: String]
    public var logo: [String: String]
    public var importableTokens: [String]
}

/// One theme entry as written in themes.json.
struct FileTheme: Codable {
    struct Overrides: Codable {
        var light: [String: String]?
        var dark: [String: String]?
    }

    var id: String
    var name: String
    var description: String
    var author: String
    var seeds: ThemeSeeds
    var overrides: Overrides?
}

private struct ThemesFile: Codable {
    var engine: EngineSpec
    var themes: [FileTheme]
}

/// The engine constants, the built-in themes and any imported themes.
public struct ThemeCatalog: Sendable {
    public static let defaultThemeId = "openframe-default"

    public let engine: EngineSpec
    public let builtIn: [ThemeData]
    public let imported: [ThemeData]

    /// Parse themes.json (the app passes the bundled file).
    public init(json: Data, imported: [ThemeData] = []) throws {
        let file = try JSONDecoder().decode(ThemesFile.self, from: json)
        self.engine = file.engine
        self.builtIn = file.themes.map {
            ThemeData(
                id: $0.id, name: $0.name, description: $0.description, author: $0.author,
                builtIn: true, seeds: $0.seeds,
                light: $0.overrides?.light ?? [:], dark: $0.overrides?.dark ?? [:])
        }
        self.imported = imported
    }

    private init(engine: EngineSpec, builtIn: [ThemeData], imported: [ThemeData]) {
        self.engine = engine
        self.builtIn = builtIn
        self.imported = imported
    }

    public func withImported(_ themes: [ThemeData]) -> ThemeCatalog {
        ThemeCatalog(engine: engine, builtIn: builtIn, imported: themes)
    }

    /// Built-in themes first, then imported ones whose id does not clash with a built-in.
    public var all: [ThemeData] {
        builtIn + imported.filter { t in !t.builtIn && !builtIn.contains { $0.id == t.id } }
    }

    public func theme(id: String) -> ThemeData? {
        all.first { $0.id == id }
    }
}

/// Parse an imported theme (the ThemeDto JSON).
public func decodeImportedTheme(_ data: Data) -> ThemeData? {
    try? JSONDecoder().decode(ThemeData.self, from: data)
}

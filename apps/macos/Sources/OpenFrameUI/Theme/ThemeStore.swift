// Observable theme state for the SwiftUI shell: the preference (System, Light, Dark), the chosen
// theme and the imported themes. Views read colours through the `\.ofTheme` environment value.
import Observation
import SwiftUI

@MainActor @Observable
public final class ThemeStore {
    public var preference: AppearancePreference {
        didSet { if preference != oldValue { invalidate() } }
    }
    public var themeId: String {
        didSet { if themeId != oldValue { invalidate() } }
    }
    public private(set) var catalog: ThemeCatalog

    @ObservationIgnored private var cache: [CacheKey: ResolvedTheme] = [:]

    private struct CacheKey: Hashable {
        let themeId: String
        let mode: ColorMode
    }

    public init(
        catalog: ThemeCatalog, preference: AppearancePreference = .system,
        themeId: String = ThemeCatalog.defaultThemeId
    ) {
        self.catalog = catalog
        self.preference = preference
        self.themeId = themeId
    }

    /// Built-in themes first, then imported ones.
    public var themes: [ThemeData] { catalog.all }

    public func setImported(_ themes: [ThemeData]) {
        catalog = catalog.withImported(themes)
        invalidate()
    }

    /// The scheme to force on the window, or nil to follow the system.
    public var preferredColorScheme: ColorScheme? {
        switch preference {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }

    /// The colours for the effective appearance. With the System preference the given scheme
    /// (the live system scheme) decides.
    public func resolvedFor(colorScheme: ColorScheme) -> ResolvedTheme {
        let mode: ColorMode
        switch preference {
        case .light: mode = .light
        case .dark: mode = .dark
        case .system: mode = colorScheme == .dark ? .dark : .light
        }
        let key = CacheKey(themeId: themeId, mode: mode)
        if let hit = cache[key] { return hit }
        let resolved = ThemeEngine.resolve(themeId: themeId, mode: mode, themes: catalog)
        cache[key] = resolved
        return resolved
    }

    private func invalidate() { cache.removeAll() }
}

private struct ThemeKey: EnvironmentKey {
    static let defaultValue = ResolvedTheme(id: "unset", mode: .light, colors: [:], unresolved: [])
}

extension EnvironmentValues {
    public var ofTheme: ResolvedTheme {
        get { self[ThemeKey.self] }
        set { self[ThemeKey.self] = newValue }
    }
}

private struct OpenFrameThemeModifier: ViewModifier {
    let store: ThemeStore
    @Environment(\.colorScheme) private var systemScheme

    func body(content: Content) -> some View {
        // Light and Dark force the scheme; System leaves it alone so `systemScheme` stays live.
        content
            .environment(\.ofTheme, store.resolvedFor(colorScheme: systemScheme))
            .preferredColorScheme(store.preferredColorScheme)
    }
}

extension View {
    /// Injects the resolved theme and applies the Light, Dark or System choice to this view tree.
    public func openFrameTheme(_ store: ThemeStore) -> some View {
        modifier(OpenFrameThemeModifier(store: store))
    }
}

extension Color {
    /// A token of the current theme, for example `Color.of("accent", in: theme)`.
    public static func of(_ token: String, in theme: ResolvedTheme) -> Color { theme[token] }
}

/// Reads one token from the environment theme: `ThemeColor("accent")` as a view or `.foregroundStyle`.
public struct ThemeColor: View {
    private let token: String
    @Environment(\.ofTheme) private var theme

    public init(_ token: String) { self.token = token }

    public var body: some View { theme[token] }
}

extension ResolvedTheme {
    /// The logo gradient, lg1 to lg3.
    public var logoGradient: LinearGradient {
        LinearGradient(colors: logoColors, startPoint: .topLeading, endPoint: .bottomTrailing)
    }
}

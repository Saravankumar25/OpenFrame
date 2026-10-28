// The appearance setting is stored by the core (appearance.get / set_mode / set_theme), like on
// Windows; this file keeps the Mac theme store in step with it.

import Foundation
import OpenFrameCore

extension ThemeStore {
    /// A store with the bundled built-in themes (the same themes.json the Windows app reads), with
    /// the Mac palette of OpenFrame Default laid over it.
    public convenience init() {
        let catalog: ThemeCatalog
        if let url = PackagedResources.url("themes", "json"),
           let data = try? Data(contentsOf: url),
           let loaded = try? ThemeCatalog(json: data),
           let macURL = PackagedResources.url("macDefaultTheme", "json"),
           let macData = try? Data(contentsOf: macURL),
           let mac = try? loaded.withMacDefault(json: macData) {
            catalog = mac
        } else {
            // Unreachable in a correct build (the file is a package resource); fail visibly in
            // development instead of showing a broken palette.
            assertionFailure("themes.json is missing from the OpenFrameUI resources")
            catalog = try! ThemeCatalog(json: Data(#"{"engine":{},"themes":[]}"#.utf8))
        }
        self.init(catalog: catalog)
    }

    /// Read the saved appearance from the core.
    public func load(from model: AppModel) async {
        guard let core = model.core else { return }
        guard let state = try? await core.appearanceGet() else { return }
        apply(state)
    }

    public func apply(_ state: AppearanceState) {
        setImported(state.myThemes.compactMap { mine in
            mine.theme.map { dto in
                ThemeData(
                    id: dto.id, name: dto.name, description: dto.description, author: dto.author,
                    builtIn: false,
                    seeds: ThemeSeeds(
                        neutralHue: dto.seeds.neutralHue, neutralTint: dto.seeds.neutralTint,
                        accentHue: dto.seeds.accentHue, accentChroma: dto.seeds.accentChroma,
                        accentLightnessNudge: dto.seeds.accentLightnessNudge),
                    light: dto.light, dark: dto.dark)
            }
        })
        preference = AppearancePreference(rawValue: state.mode.rawValue) ?? .system
        themeId = state.themeId
    }

    /// Save a new mode. The screen changes at once; the core's answer then confirms it.
    public func choose(mode: AppearancePreference, core: Core?) async throws {
        preference = mode
        guard let core, let m = AppearanceMode(rawValue: mode.rawValue) else { return }
        apply(try await core.appearanceSetMode(SetModeArgs(mode: m)))
    }

    public func choose(themeId id: String, core: Core?) async throws {
        themeId = id
        guard let core else { return }
        apply(try await core.appearanceSetTheme(SetThemeArgs(themeId: id)))
    }
}

extension AppModel {
    /// True once the core has started (used to reload settings that live in the core).
    public var isRunning: Bool { core != nil }
}

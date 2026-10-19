// Import a theme (Windows: features/appearance/ThemeImportDialog.tsx). A file the person picked,
// or pasted text, goes to the core to be checked (format, limits, contrast); the sheet shows the
// result in Light and Dark; nothing is stored until the person saves, and the core checks the
// source again on save (the digest makes it refuse a file that changed after the preview).

import Observation
import OpenFrameCore
import SwiftUI

extension ThemeData {
    /// An imported theme as the colour engine takes it.
    init(core dto: ThemeDto) {
        self.init(
            id: dto.id, name: dto.name, description: dto.description, author: dto.author, builtIn: false,
            seeds: ThemeSeeds(
                neutralHue: dto.seeds.neutralHue, neutralTint: dto.seeds.neutralTint,
                accentHue: dto.seeds.accentHue, accentChroma: dto.seeds.accentChroma,
                accentLightnessNudge: dto.seeds.accentLightnessNudge),
            light: dto.light, dark: dto.dark)
    }
}

@MainActor
@Observable
final class ThemeImportModel {
    enum Source: Equatable {
        case file(String)
        case text
    }

    /// Largest theme accepted (bytes), as in the core.
    static let maxBytes = 10 * 1024

    let source: Source
    var text = ""
    private(set) var result: ThemeCheck?
    private(set) var checking = false
    private(set) var saving = false
    private(set) var error: String?
    /// Set once the theme is saved; the sheet closes on it.
    private(set) var savedMessage: String?

    private weak var app: AppModel?
    private let themes: ThemeStore

    init(source: Source, app: AppModel?, themes: ThemeStore) {
        self.source = source
        self.app = app
        self.themes = themes
    }

    var bytes: Int { text.utf8.count }
    var tooLarge: Bool { bytes > Self.maxBytes }
    var sizeHint: String { String(format: "%.1f KB of 10 KB.", Double(bytes) / 1024) }
    var canCheck: Bool { !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !tooLarge && !checking }
    var usable: Bool { result.map { $0.status != .rejected } ?? false }
    var failures: [ThemeContrastRow] { result?.contrast.filter { !$0.passes } ?? [] }

    var summary: String {
        if checking { return "Checking the theme." }
        guard let result else { return "" }
        switch result.status {
        case .ok: return "Check finished: readable in Light and Dark."
        case .corrected: return "Check finished: small readability changes were made."
        case .rejected: return "Check finished: this theme cannot be used."
        }
    }

    var readabilitySummary: String {
        failures.isEmpty ? "all pass" : "\(failures.count) below 4.5 to 1"
    }

    /// The checked theme as the colour engine takes it, for the Light and Dark previews.
    var previewTheme: ThemeData? { result?.theme.map { ThemeData(core: $0) } }

    func editText() { result = nil; error = nil }

    /// A file is checked as soon as the sheet opens; pasted text when the person asks.
    func check() async {
        guard let core = app?.core, !checking else { return }
        checking = true
        error = nil
        defer { checking = false }
        do {
            switch source {
            case let .file(path): result = try await core.appearanceCheckTheme(ThemeSourceArgs(path: path))
            case .text: result = try await core.appearanceCheckTheme(ThemeSourceArgs(text: text))
            }
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    /// Save to My Themes, and apply it when asked. Returns true when it is saved.
    @discardableResult
    func save(apply: Bool) async -> Bool {
        guard let core = app?.core, !saving, usable else { return false }
        saving = true
        error = nil
        defer { saving = false }
        let digest = result?.digest
        do {
            let args: ImportThemeArgs
            switch source {
            case let .file(path): args = ImportThemeArgs(path: path, apply: apply, expectedDigest: digest)
            case .text: args = ImportThemeArgs(text: text, apply: apply, expectedDigest: digest)
            }
            let state = try await core.appearanceImportTheme(args)
            themes.apply(state)
            let name = result?.theme?.name ?? "The theme"
            let message = apply ? "\(name) is saved and applied." : "\(name) is saved to My Themes."
            savedMessage = message
            ToastCenter.shared.show(message)
            return true
        } catch {
            self.error = CoreFailure(asCore: error).message
            return false
        }
    }
}

/// The My Themes list: apply, rename, export, delete (Windows: AppearancePanel MyThemes).
@MainActor
@Observable
final class MyThemesModel {
    private(set) var error: String?
    private(set) var busy = false
    private weak var app: AppModel?
    private let themes: ThemeStore

    init(app: AppModel?, themes: ThemeStore) {
        self.app = app
        self.themes = themes
    }

    /// The most recent state from the core, kept so the list shows without another read.
    private(set) var state: AppearanceState?

    func reload() async {
        guard let core = app?.core else { return }
        do {
            let loaded = try await core.appearanceGet()
            state = loaded
            themes.apply(loaded)
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    var mine: [MyTheme] { state?.myThemes ?? [] }
    var activeId: String { state?.themeId ?? themes.themeId }

    func apply(_ theme: MyTheme) async {
        await run { core in
            let s = try await core.appearanceSetTheme(SetThemeArgs(themeId: theme.id))
            return (s, "\(theme.name) theme applied.")
        }
    }

    /// Names are 1 to 60 characters after trimming (the core enforces it too).
    func rename(_ theme: MyTheme, to name: String) async -> Bool {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.count <= 60 else { return false }
        return await run { core in
            (try await core.appearanceRenameTheme(RenameThemeArgs(id: theme.id, name: trimmed)), "Renamed to \(trimmed).")
        }
    }

    func delete(_ theme: MyTheme) async {
        let wasActive = theme.id == activeId
        await run { core in
            (try await core.appearanceDeleteTheme(DeleteThemeArgs(id: theme.id)),
             wasActive ? "Theme deleted. OpenFrame Default is applied." : "Theme deleted.")
        }
    }

    func export(_ theme: MyTheme, to path: String) async {
        guard let core = app?.core else { return }
        do {
            let r = try await core.appearanceExportTheme(ExportThemeArgs(id: theme.id, path: path))
            ToastCenter.shared.show("Saved \(r.fileName).")
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    @discardableResult
    private func run(_ work: (Core) async throws -> (AppearanceState, String)) async -> Bool {
        guard let core = app?.core, !busy else { return false }
        busy = true
        error = nil
        defer { busy = false }
        do {
            let (new, message) = try await work(core)
            state = new
            themes.apply(new)
            ToastCenter.shared.show(message)
            return true
        } catch {
            self.error = CoreFailure(asCore: error).message
            return false
        }
    }
}

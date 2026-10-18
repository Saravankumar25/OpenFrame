// Appearance: System, Light or Dark, and a grid of themes. Each card is drawn with that theme's
// own colours so the choice can be judged before it is made.

import AppKit
import OpenFrameCore
import SwiftUI
import UniformTypeIdentifiers

struct AppearanceSettingsView: View {
    var body: some View {
        ScrollView { AppearanceSettingsContent() }
    }
}

/// The Appearance controls without the scroll view (which an image renderer cannot draw).
struct AppearanceSettingsContent: View {
    @Environment(AppModel.self) private var app
    @Environment(ThemeStore.self) private var themes
    @Environment(\.ofTheme) private var theme
    @State private var error: String?
    @State private var notice: String?
    @State private var message: String?

    private let columns = [GridItem(.adaptive(minimum: 170, maximum: 220), spacing: 12)]

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Mode").font(.headline)
                    Picker("Mode", selection: modeBinding) {
                        Text("System").tag(AppearancePreference.system)
                        Text("Light").tag(AppearancePreference.light)
                        Text("Dark").tag(AppearancePreference.dark)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .accessibilityLabel("Mode")
                    .accessibilityIdentifier("settings-mode")
                    .frame(maxWidth: 320)
                    Text("System follows your Mac. Light and Dark stay as you set them.")
                        .font(.callout).foregroundStyle(.secondary)
                }
                if let notice {
                    Label(notice, systemImage: "info.circle")
                        .font(.callout)
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(theme["yellow-soft"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        .foregroundStyle(theme["yellow-t"])
                        .accessibilityIdentifier("settings-fallback-notice")
                }
                VStack(alignment: .leading, spacing: 8) {
                    Text("Theme").font(.headline)
                    LazyVGrid(columns: columns, spacing: 12) {
                        ForEach(themes.themes) { item in
                            ThemeCard(
                                data: item, mode: theme.mode, catalog: themes.catalog,
                                selected: item.id == themes.themeId
                            ) { Task { await choose(item.id) } }
                        }
                    }
                }
                HStack {
                    Button("Import Theme…") { Task { await importTheme() } }
                        .accessibilityIdentifier("settings-import-theme")
                    Button("Export Selected Theme…") { Task { await exportTheme() } }
                        .accessibilityIdentifier("settings-export-theme")
                }
                if let message { Text(message).font(.callout).foregroundStyle(.secondary) }
                if let error { SettingsErrorText(message: error) }
            }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .task(id: app.isRunning) { await reload() }
    }

    private var modeBinding: Binding<AppearancePreference> {
        Binding(
            get: { themes.preference },
            set: { newValue in
                Task {
                    do {
                        error = nil
                        try await themes.choose(mode: newValue, core: app.core)
                    } catch {
                        self.error = CoreFailure(asCore: error).message
                    }
                }
            })
    }

    private func reload() async {
        guard let core = app.core else { return }
        do {
            let state = try await core.appearanceGet()
            themes.apply(state)
            notice = state.fallbackNotice
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    private func choose(_ id: String) async {
        error = nil
        message = nil
        do {
            try await themes.choose(themeId: id, core: app.core)
        } catch {
            self.error = CoreFailure(asCore: error).message
            await reload()
        }
    }

    private func importTheme() async {
        guard let core = app.core else { return }
        let panel = NSOpenPanel()
        panel.title = "Import a Theme"
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        error = nil
        message = nil
        do {
            let check = try await core.appearanceCheckTheme(ThemeSourceArgs(path: url.path))
            if check.status == .rejected {
                error = check.problems.first ?? "That file is not a theme OpenFrame can use."
                return
            }
            let state = try await core.appearanceImportTheme(
                ImportThemeArgs(path: url.path, apply: false, expectedDigest: check.digest))
            themes.apply(state)
            message = "Imported \(check.theme?.name ?? "the theme")."
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    private func exportTheme() async {
        guard let core = app.core else { return }
        let panel = NSSavePanel()
        panel.title = "Export Theme"
        panel.allowedContentTypes = [.json]
        let id = themes.themeId
        panel.nameFieldStringValue = "\(id).json"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        error = nil
        message = nil
        do {
            let result = try await core.appearanceExportTheme(ExportThemeArgs(id: id, path: url.path))
            message = "Saved \(result.fileName)."
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

/// One theme, drawn with its own colours.
struct ThemeCard: View {
    let data: ThemeData
    let mode: ColorMode
    let catalog: ThemeCatalog
    let selected: Bool
    let action: () -> Void

    var body: some View {
        let own = ThemeEngine.resolve(themeId: data.id, mode: mode, themes: catalog)
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 0) {
                    own["side"].frame(width: 28)
                    ZStack(alignment: .topLeading) {
                        own["bg"]
                        VStack(alignment: .leading, spacing: 4) {
                            RoundedRectangle(cornerRadius: 3).fill(own["panel"]).frame(height: 14)
                            RoundedRectangle(cornerRadius: 3).fill(own["accent"]).frame(width: 36, height: 8)
                        }
                        .padding(8)
                    }
                }
                .frame(height: 64)
                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(own["line"], lineWidth: 1))
                VStack(alignment: .leading, spacing: 2) {
                    HStack {
                        Text(data.name).font(.callout.weight(.semibold))
                        Spacer(minLength: 0)
                        if selected {
                            Image(systemName: "checkmark.circle.fill").foregroundStyle(own["accent"])
                                .accessibilityHidden(true)
                        }
                    }
                    Text(data.description)
                        .font(.caption).foregroundStyle(.secondary)
                        .lineLimit(2).fixedSize(horizontal: false, vertical: true)
                        .frame(minHeight: 32, alignment: .top)
                }
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(own["panel"].opacity(0.35), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .stroke(selected ? own["accent"] : Color.secondary.opacity(0.25), lineWidth: selected ? 2.5 : 1))
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(data.name). \(data.description)")
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
        .accessibilityIdentifier("theme-card-\(data.id)")
    }
}

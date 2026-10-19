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
    @State private var mine: MyThemesModel?
    @State private var importing: ThemeImportModel?
    @State private var renaming: MyTheme?
    @State private var deleting: MyTheme?

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
                    CenteredCards(cardWidth: 190, spacing: 12) {
                        ForEach(themes.themes) { item in
                            ThemeCard(
                                data: item, mode: theme.mode, catalog: themes.catalog,
                                selected: item.id == themes.themeId
                            ) { Task { await choose(item.id) } }
                        }
                    }
                }
                myThemesSection
                HStack {
                    Button("Export Selected Theme…") { Task { await exportTheme() } }
                        .accessibilityIdentifier("settings-export-theme")
                }
                if let message { Text(message).font(.callout).foregroundStyle(.secondary) }
                if let error { SettingsErrorText(message: error) }
            }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .task(id: app.isRunning) {
            if mine == nil { mine = MyThemesModel(app: app, themes: themes) }
            await reload()
            await mine?.reload()
        }
        // A saved theme joins My Themes: read the list again when the import sheet closes.
        .sheet(item: $importing, onDismiss: { Task { await mine?.reload() } }) { model in
            ThemeImportSheet(model: model)
        }
        .sheet(item: $renaming) { theme in
            if let mine { RenameThemeSheet(theme: theme, model: mine) }
        }
        .confirmationDialog(
            "Delete the theme \u{201C}\(deleting?.name ?? "")\u{201D}?",
            isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } }),
            titleVisibility: .visible
        ) {
            Button("Delete Theme", role: .destructive) {
                if let t = deleting { Task { await mine?.delete(t) } }
                deleting = nil
            }
            .accessibilityIdentifier("theme-delete-confirm")
            Button("Cancel", role: .cancel) { deleting = nil }
                .accessibilityIdentifier("theme-delete-cancel")
        } message: {
            Text("The theme is removed from My Themes. This cannot be undone; export it first if you may want it again."
                + (deleting?.id == mine?.activeId ? " It is in use, so OpenFrame Default is applied." : "")
                + " Your projects are not affected.")
        }
    }

    // MARK: My Themes

    private var myThemesSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("My Themes").font(.headline)
                Spacer()
                Button("Import Theme File…") { Task { await pickThemeFile() } }
                    .accessibilityIdentifier("settings-import-theme")
                Button("Paste Theme…") {
                    importing = ThemeImportModel(source: .text, app: app, themes: themes)
                }
                .accessibilityIdentifier("settings-paste-theme")
            }
            let list = mine?.mine ?? []
            if list.isEmpty {
                Text("No imported themes yet. Theme files are small .json files, for example from the OpenFrame website.")
                    .font(.callout).foregroundStyle(.secondary)
                    .accessibilityIdentifier("my-themes-empty")
            } else {
                VStack(spacing: 0) {
                    ForEach(list, id: \.id) { item in
                        myThemeRow(item)
                        if item.id != list.last?.id { Divider() }
                    }
                }
                .background(theme["panel"].opacity(0.4), in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"], lineWidth: 1))
                .accessibilityElement(children: .contain)
                .accessibilityLabel("My Themes")
                .accessibilityIdentifier("my-themes-list")
            }
            if let failure = mine?.error { SettingsErrorText(message: failure) }
        }
    }

    private func myThemeRow(_ item: MyTheme) -> some View {
        let active = item.id == (mine?.activeId ?? themes.themeId)
        return HStack(spacing: 10) {
            if let dto = item.theme {
                let own = ThemeEngine.resolve(themeId: dto.id, mode: theme.mode, themes: themes.catalog)
                HStack(spacing: 0) {
                    own["side"].frame(width: 10)
                    own["bg"]
                    own["accent"].frame(width: 10)
                }
                .frame(width: 42, height: 24)
                .clipShape(RoundedRectangle(cornerRadius: 4))
                .accessibilityHidden(true)
            } else {
                RoundedRectangle(cornerRadius: 4).stroke(theme["line"]).frame(width: 42, height: 24)
                    .accessibilityHidden(true)
            }
            VStack(alignment: .leading, spacing: 1) {
                Text(item.name).fontWeight(.medium)
                if let author = item.theme?.author, !author.isEmpty {
                    Text("By \(author)").font(.caption).foregroundStyle(.secondary)
                }
                if let problem = item.problem {
                    Text("This theme cannot be used. \(problem)").font(.caption).foregroundStyle(theme["red-t"])
                }
            }
            Spacer(minLength: 8)
            if active {
                Text("In use").font(.caption.weight(.semibold))
                    .padding(.horizontal, 8).padding(.vertical, 2)
                    .background(theme["accent-soft"], in: Capsule())
                    .accessibilityIdentifier("my-theme-inuse-\(item.id)")
            }
            if item.theme != nil && !active {
                Button("Apply") { Task { await mine?.apply(item) } }
                    .accessibilityLabel("Apply \(item.name)")
                    .accessibilityIdentifier("my-theme-apply-\(item.id)")
            }
            if item.theme != nil {
                Button("Rename") { renaming = item }
                    .accessibilityLabel("Rename \(item.name)")
                    .accessibilityIdentifier("my-theme-rename-\(item.id)")
                Button("Export") { Task { await exportMine(item) } }
                    .accessibilityLabel("Export \(item.name)")
                    .accessibilityIdentifier("my-theme-export-\(item.id)")
            }
            Button("Delete") { deleting = item }
                .accessibilityLabel("Delete \(item.name)")
                .accessibilityIdentifier("my-theme-delete-\(item.id)")
        }
        .controlSize(.small)
        .padding(.horizontal, 10).padding(.vertical, 6)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(item.name)
        .accessibilityIdentifier("my-theme-\(item.id)")
    }

    private func exportMine(_ item: MyTheme) async {
        let panel = NSSavePanel()
        panel.title = "Export Theme"
        panel.allowedContentTypes = [.json]
        // A name is text, not a path: no folder separators or reserved characters in the suggestion.
        let safe = item.name.components(separatedBy: CharacterSet(charactersIn: "\\/:*?\"<>|")).joined(separator: " ")
            .trimmingCharacters(in: .whitespaces)
        panel.nameFieldStringValue = "\(safe.isEmpty ? item.id : safe).json"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        await mine?.export(item, to: url.path)
    }

    private func pickThemeFile() async {
        let panel = NSOpenPanel()
        panel.title = "Import a Theme"
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        importing = ThemeImportModel(source: .file(url.path), app: app, themes: themes)
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
extension ThemeImportModel: Identifiable { nonisolated var id: ObjectIdentifier { ObjectIdentifier(self) } }
extension MyTheme: Identifiable {}

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

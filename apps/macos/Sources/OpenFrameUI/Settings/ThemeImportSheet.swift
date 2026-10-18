// The Import Theme sheet and the Rename Theme sheet.

import OpenFrameCore
import SwiftUI

struct ThemeImportSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme
    @Environment(ThemeStore.self) private var themes
    @State var model: ThemeImportModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Import theme").font(.title2.weight(.semibold))
                Text(model.source == .text
                     ? "Paste the text of an OpenFrame theme file. It is checked before anything is saved."
                     : "Checked before anything is saved.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    if model.source == .text && model.result == nil { pasteField }
                    if model.checking {
                        ProgressView("Checking the theme").accessibilityIdentifier("theme-import-checking")
                    }
                    if let result = model.result { resultView(result) }
                    if let error = model.error { SettingsErrorText(message: error) }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            Text(model.summary).font(.caption).foregroundStyle(.clear)
                .accessibilityLabel(model.summary)
                .accessibilityIdentifier("theme-import-summary")
                .frame(height: 0)
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("theme-import-cancel")
                if model.source == .text && model.result == nil {
                    Button("Check Theme") { Task { await model.check() } }
                        .keyboardShortcut(.defaultAction)
                        .disabled(!model.canCheck)
                        .accessibilityIdentifier("theme-import-check")
                }
                if model.result != nil {
                    Button("Save to My Themes") { Task { if await model.save(apply: false) { dismiss() } } }
                        .disabled(!model.usable || model.saving)
                        .accessibilityIdentifier("theme-import-save")
                    Button("Save and Apply") { Task { if await model.save(apply: true) { dismiss() } } }
                        .keyboardShortcut(.defaultAction)
                        .disabled(!model.usable || model.saving)
                        .accessibilityIdentifier("theme-import-apply")
                }
            }
        }
        .padding(20)
        .frame(width: 640, height: 560)
        .task { if case .file = model.source { await model.check() } }
        .accessibilityIdentifier("theme-import-sheet")
    }

    private var pasteField: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Theme text").font(.headline)
            TextEditor(text: $model.text)
                .font(.system(.callout, design: .monospaced))
                .frame(height: 220)
                .overlay(RoundedRectangle(cornerRadius: 6).stroke(model.tooLarge ? theme["red"] : theme["line"], lineWidth: 1))
                .accessibilityLabel("Theme text")
                .accessibilityIdentifier("theme-import-text")
            if model.tooLarge {
                Text("This is larger than 10 KB. Theme files are small; this is not one.")
                    .font(.caption).foregroundStyle(theme["red-t"])
                    .accessibilityIdentifier("theme-import-toolarge")
            }
            Text(model.sizeHint).font(.caption).foregroundStyle(.secondary)
                .accessibilityIdentifier("theme-import-size")
        }
    }

    @ViewBuilder private func resultView(_ result: ThemeCheck) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            if let t = result.theme {
                VStack(alignment: .leading, spacing: 2) {
                    Text(t.name).font(.headline).accessibilityIdentifier("theme-import-name")
                    Text("\(t.author.isEmpty ? "" : "By \(t.author). ")\(t.description)")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if let preview = model.previewTheme {
                    let catalog = themes.catalog.withImported(themes.catalog.imported + [preview])
                    HStack(spacing: 12) {
                        ForEach([ColorMode.light, .dark], id: \.self) { mode in
                            VStack(alignment: .leading, spacing: 4) {
                                ThemeCard(data: preview, mode: mode, catalog: catalog, selected: false) {}
                                    .allowsHitTesting(false)
                                Text(mode == .light ? "Light" : "Dark").font(.caption.weight(.semibold))
                            }
                            .accessibilityElement(children: .contain)
                            .accessibilityLabel("\(mode == .light ? "Light" : "Dark") preview")
                            .accessibilityIdentifier("theme-import-preview-\(mode.rawValue)")
                        }
                    }
                }
            }
            switch result.status {
            case .ok:
                banner("This theme is readable in Light and Dark.", symbol: "checkmark.circle", tone: "green", id: "theme-import-ok")
            case .corrected:
                VStack(alignment: .leading, spacing: 2) {
                    banner("Small changes were made so text stays readable:", symbol: "info.circle", tone: "blue", id: "theme-import-corrected")
                    ForEach(result.notes, id: \.self) { Text("\u{2022} \($0)").font(.callout) }
                }
            case .rejected:
                VStack(alignment: .leading, spacing: 2) {
                    banner("This theme cannot be used. Nothing was changed.", symbol: "xmark.octagon", tone: "red", id: "theme-import-rejected")
                    ForEach(result.problems, id: \.self) { Text("\u{2022} \($0)").font(.callout) }
                }
            }
            if !result.contrast.isEmpty {
                DisclosureGroup("Readability check (\(model.readabilitySummary))") {
                    Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 4) {
                        GridRow {
                            Text("Mode").bold(); Text("Text").bold(); Text("Contrast").bold(); Text("Result").bold()
                        }
                        ForEach(Array(result.contrast.enumerated()), id: \.offset) { _, row in
                            GridRow {
                                Text(row.mode == .light ? "Light" : "Dark")
                                Text(row.label)
                                Text(String(format: "%.2f to 1", row.ratio))
                                Text(row.passes ? "Pass" : "Too low")
                            }
                        }
                    }
                    .font(.callout)
                    .padding(.top, 4)
                }
                .accessibilityIdentifier("theme-import-contrast")
            }
            if model.source == .text {
                Button("Edit the text") { model.editText() }
                    .buttonStyle(.link)
                    .accessibilityIdentifier("theme-import-edit")
            }
        }
    }

    private func banner(_ text: String, symbol: String, tone: String, id: String) -> some View {
        Label(text, systemImage: symbol)
            .font(.callout)
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .foregroundStyle(theme["\(tone)-t"])
            .background(theme["\(tone)-soft"], in: RoundedRectangle(cornerRadius: 6))
            .accessibilityIdentifier(id)
    }
}

struct RenameThemeSheet: View {
    @Environment(\.dismiss) private var dismiss
    let theme: MyTheme
    let model: MyThemesModel
    @State private var name: String

    init(theme: MyTheme, model: MyThemesModel) {
        self.theme = theme
        self.model = model
        _name = State(initialValue: theme.name)
    }

    private var trimmed: String { name.trimmingCharacters(in: .whitespacesAndNewlines) }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Rename theme").font(.title3.weight(.semibold))
            TextField("Theme name", text: $name)
                .accessibilityLabel("Theme name")
                .accessibilityIdentifier("theme-rename-name")
                .onSubmit { commit() }
            Text("Up to 60 characters.").font(.caption).foregroundStyle(.secondary)
            if let error = model.error { SettingsErrorText(message: error) }
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("theme-rename-cancel")
                Button("Rename") { commit() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(trimmed.isEmpty || trimmed.count > 60)
                    .accessibilityIdentifier("theme-rename-save")
            }
        }
        .padding(20)
        .frame(width: 380)
    }

    private func commit() {
        guard !trimmed.isEmpty, trimmed.count <= 60 else { return }
        Task { if await model.rename(theme, to: trimmed) { dismiss() } }
    }
}

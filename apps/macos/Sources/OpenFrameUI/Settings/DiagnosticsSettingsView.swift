// Diagnostics: shows exactly what a bundle holds, then writes it where the person chooses.

import AppKit
import OpenFrameCore
import SwiftUI
import UniformTypeIdentifiers

struct DiagnosticsSettingsView: View {
    @Environment(AppModel.self) private var app
    @State private var preview: DiagnosticsPreview?
    @State private var result: DiagnosticsExportResult?
    @State private var error: String?
    @State private var exporting = false

    var body: some View {
        Form {
            Section {
                Text("A diagnostics bundle helps the team find a problem. It never contains your project text. You choose where it is saved and whether to send it.")
                    .fixedSize(horizontal: false, vertical: true)
            }
            if let preview {
                Section("What the bundle includes") {
                    ForEach(preview.files, id: \.name) { file in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(file.name).font(.body.weight(.medium))
                            Text("\(file.description) (\(formattedBytes(file.bytes)))")
                                .font(.callout).foregroundStyle(.secondary)
                        }
                        .accessibilityElement(children: .combine)
                    }
                }
                Section("What it never includes") {
                    ForEach(preview.neverIncluded, id: \.self) { Text($0) }
                }
            }
            Section {
                Button("Export Diagnostics…") { Task { await export() } }
                    .disabled(exporting || app.core == nil)
                    .accessibilityIdentifier("export-diagnostics")
                if let result {
                    HStack {
                        Text(result.summary).fixedSize(horizontal: false, vertical: true)
                        Spacer()
                        Button("Show in Finder") {
                            Task { try? await app.core?.revealLocation("exported", path: result.path) }
                        }
                        .buttonStyle(.link)
                        .accessibilityIdentifier("show-exported-diagnostics")
                    }
                }
                if let error { SettingsErrorText(message: error) }
            }
        }
        .formStyle(.grouped)
        .task(id: app.isRunning) { await load() }
    }

    private func load() async {
        guard let core = app.core else { return }
        do {
            preview = try await core.diagnosticsPreview()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    private func export() async {
        guard let core = app.core else { return }
        let panel = NSSavePanel()
        panel.title = "Export Diagnostics"
        panel.allowedContentTypes = [.zip]
        panel.nameFieldStringValue = preview?.suggestedFileName ?? "OpenFrame-diagnostics.zip"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        exporting = true
        error = nil
        result = nil
        defer { exporting = false }
        do {
            result = try await core.diagnosticsExportBundle(DiagnosticsExportArgs(path: url.path))
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

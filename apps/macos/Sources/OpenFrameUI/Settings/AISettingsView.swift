// Offline AI: what the core reports about the assistant. Downloading and using it on the Mac comes
// in Mac phase M8, so this tab only reads.

import OpenFrameCore
import SwiftUI

struct AISettingsView: View {
    @Environment(AppModel.self) private var app
    @State private var status: AiStatusDto?
    @State private var diagnostics: AiDiagnosticsDto?
    @State private var error: String?

    var body: some View {
        Form {
            Section {
                Text("The offline assistant runs on this Mac and never sends your writing anywhere. Downloading and using it on the Mac comes in Mac phase M8. Nothing is downloaded now.")
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("settings-ai-note")
            }
            if let error { Section { SettingsErrorText(message: error) } }
            if let status {
                Section("Status") {
                    LabeledContent("Assistant", value: status.mode)
                    LabeledContent("Files", value: status.bundleState)
                    LabeledContent("Engine", value: status.runtimeState)
                    if let problem = status.problem {
                        Text(problem.message).foregroundStyle(.secondary)
                    }
                }
            }
            if let diagnostics {
                Section("Components") {
                    if diagnostics.components.isEmpty {
                        Text("No assistant files are installed.").foregroundStyle(.secondary)
                    }
                    ForEach(diagnostics.components, id: \.id) { part in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(part.role).font(.body.weight(.medium))
                            Text("\(part.name) \(part.version), \(formattedBytes(part.bytes)), licence \(part.licenseId)")
                                .font(.callout).foregroundStyle(.secondary)
                        }
                        .accessibilityElement(children: .combine)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .task(id: app.isRunning) { await load() }
    }

    private func load() async {
        guard let core = app.core else { return }
        do {
            status = try await core.aiStatus()
            diagnostics = try await core.aiDiagnostics()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

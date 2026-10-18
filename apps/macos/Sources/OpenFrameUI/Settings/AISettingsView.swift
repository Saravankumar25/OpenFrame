// Settings, Offline AI (Windows: Settings, Offline AI). The engine ships inside the app; the models
// are a one-time download the person starts here or in the AI panel. This page shows the status,
// the download while OpenFrame AI is not installed, a way to free memory, Remove, the graphics
// card choice, the technical details ordinary screens never show, the AI health card with Export
// AI report, and the learned-memory screen.

import AppKit
import OpenFrameCore
import SwiftUI

struct AISettingsView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: AISettingsModel?
    @State private var download: AIDownloadModel?

    var body: some View {
        ScrollView {
            if let model {
                VStack(alignment: .leading, spacing: 16) {
                    if let error = model.error { SettingsErrorText(message: error) }
                    statusCard(model)
                    if model.mode == .download, let download {
                        AIDownloadCard(model: download, damaged: model.status.flatMap(AILogic.damagedModels))
                    }
                    AIHealthCard(model: model)
                    AILearnedMemoryCard()
                    AITechnicalDetails(model: model)
                }
                .padding(20)
            } else {
                ProgressView().frame(maxWidth: .infinity).padding(40)
            }
        }
        .accessibilityIdentifier("settings-ai")
        .onAppear {
            if model == nil {
                model = AISettingsModel(app: app)
                let d = AIDownloadModel(app: app)
                d.pickFile = { AIFilePicker.chooseModelFile() }
                download = d
            }
        }
        .task(id: app.isRunning) { await poll() }
    }

    /// Reads the state while the page is open; faster while a download runs, and the status page
    /// follows a finished download.
    private func poll() async {
        while !Task.isCancelled {
            await model?.load()
            if model?.mode == .download { await download?.refresh() }
            try? await Task.sleep(for: .milliseconds(download?.isRunning == true ? 500 : 2000))
        }
    }

    private func statusCard(_ m: AISettingsModel) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Offline AI").font(.headline)
                if let s = m.status {
                    BDChip(text: AILogic.stateText(s), tone: s.available ? .green : .plain)
                        .accessibilityIdentifier("settings-ai-state")
                }
                Spacer()
            }
            Text(AIText.promise)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("settings-ai-note")
            if let s = m.status {
                if m.mode == .off { BDBanner(.err, AILogic.offReason(s)).accessibilityIdentifier("settings-ai-off-reason") }
                if s.available {
                    HStack {
                        Button("Stop Offline AI (free memory)") { Task { await m.stopRuntime() } }
                            .accessibilityIdentifier("settings-ai-stop")
                        Button("Remove OpenFrame AI\u{2026}") { m.removing = true }
                            .accessibilityIdentifier("settings-ai-remove")
                    }
                    graphics(m)
                }
            } else {
                ProgressView().controlSize(.small)
            }
            Text("OpenFrame works normally without OpenFrame AI.").font(.caption).foregroundStyle(theme["muted"])
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("settings-ai-card")
        .confirmationDialog("Remove OpenFrame AI?", isPresented: Binding(get: { m.removing }, set: { m.removing = $0 }), titleVisibility: .visible) {
            Button("Remove", role: .destructive) { Task { await m.remove() } }
                .accessibilityIdentifier("settings-ai-remove-confirm")
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This frees the space the AI models use on this computer. AI stays off until you download it again. Your projects are not changed.")
        }
    }

    @ViewBuilder private func graphics(_ m: AISettingsModel) -> some View {
        if let d = m.diagnostics {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text("Graphics card").bold()
                    Picker("Graphics card", selection: Binding(get: { d.graphicsOff }, set: { off in Task { await m.setGraphics(off: off) } })) {
                        Text("Automatic").tag(false)
                        Text("Off").tag(true)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(width: 180)
                    .accessibilityLabel("Graphics card")
                    .accessibilityIdentifier("settings-ai-graphics")
                }
                Text("\(d.placement) Automatic uses a graphics card only after it passes a short check; the processor is always the fallback.")
                    .font(.caption).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("settings-ai-placement")
            }
        }
    }
}

// MARK: - Technical details

/// The only place model and runtime names appear (spec sections 1 and 36).
struct AITechnicalDetails: View {
    let model: AISettingsModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        DisclosureGroup("Technical details") {
            if let x = model.diagnostics {
                VStack(alignment: .leading, spacing: 8) {
                    if x.components.isEmpty {
                        Text("No assistant files are installed.").foregroundStyle(theme["muted"])
                            .accessibilityIdentifier("settings-ai-no-components")
                    }
                    ForEach(x.components, id: \.id) { part in
                        VStack(alignment: .leading, spacing: 2) {
                            Text("\(part.role): \(part.name)").font(.body.weight(.medium))
                            Text("Version \(part.version), \(formattedBytes(part.bytes)), licence \(part.licenseId)")
                                .font(.callout).foregroundStyle(theme["muted"])
                            Text(part.sha256).font(.caption.monospaced()).textSelection(.enabled)
                                .help("SHA-256")
                        }
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("settings-ai-component")
                    }
                    if x.components.contains(where: { $0.licenseId == "Gemma-Terms-of-Use" }) {
                        Text(AIText.gemmaNotice).font(.caption).foregroundStyle(theme["muted"])
                    }
                    Divider()
                    Text("Package: \(x.bundleId) v\(x.bundleVersion) \u{00B7} files: \(AILogic.fileCheckText(x.bundleState))")
                    if let problem = x.problem { Text(problem) }
                    Text("Language model: \(x.chatState) \u{00B7} search model: \(x.searchState) (search always on the processor)")
                    Text("This computer: \(formattedBytes(x.memoryBytes)) memory \u{00B7} \(x.processor) (\(Int(x.processorThreads)) threads)")
                    ForEach(x.warnings, id: \.self) { Text($0) }
                    Text("Engine in: \(x.bundleDir.isEmpty ? "unknown" : x.bundleDir) (read-only) \u{00B7} engine and models: \(formattedBytes(x.bundleBytes))")
                        .textSelection(.enabled)
                    Text("Log: \(x.logFile)").textSelection(.enabled)
                    Text("Everything runs on this computer. Prompts, project text and answers are never sent anywhere. The only network use is the model download you start.")
                        .font(.caption).foregroundStyle(theme["muted"])
                }
                .font(.callout)
                .padding(.top, 6)
            } else {
                Text("Not available.").foregroundStyle(theme["muted"]).padding(.top, 6)
            }
        }
        .accessibilityIdentifier("settings-ai-technical")
    }
}

// MARK: - AI health card and Export AI report

struct AIHealthCard: View {
    @Bindable var model: AISettingsModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("AI health").font(.headline).accessibilityAddTraits(.isHeader)
            if let h = model.health {
                if h.requests == 0 {
                    Text("No requests yet. The figures appear after you use the AI Assistant.")
                        .foregroundStyle(theme["muted"])
                        .accessibilityIdentifier("ai-health-empty")
                } else {
                    Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 4) {
                        ForEach(AIHealthText.rows(h), id: \.label) { row in
                            GridRow {
                                Text(row.label).foregroundStyle(theme["ink2"])
                                Text(row.value)
                            }
                        }
                    }
                    .accessibilityElement(children: .contain)
                    .accessibilityLabel("AI health figures")
                    .accessibilityIdentifier("ai-health-figures")
                }
            } else {
                ProgressView().controlSize(.small)
            }
            Text("Only numbers are kept on this computer: no questions, answers or project text. Nothing is sent. Export AI report saves them to a file you choose, to share if you want to.")
                .font(.caption).foregroundStyle(theme["muted"])
            HStack {
                Button(model.busy ? "Saving\u{2026}" : "Export AI report\u{2026}") { Task { await model.exportReport() } }
                    .disabled(model.busy)
                    .accessibilityIdentifier("ai-export-report")
                if let line = model.lastExport {
                    Text(line).font(.caption).foregroundStyle(theme["muted"]).accessibilityIdentifier("ai-export-result")
                }
            }
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("AI health")
        .accessibilityIdentifier("ai-health-card")
    }
}

// MARK: - Learned memory

/// Learned memory is visible, editable and deletable (ADR-0028). This build of the core keeps no
/// learned memory and has no operation to list or change it, so this screen says that plainly and
/// lists nothing. It lists entries as soon as the core offers them; it never invents any.
struct AILearnedMemoryCard: View {
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Learned memory").font(.headline).accessibilityAddTraits(.isHeader)
            Text("OpenFrame AI has not learned anything from your writing, and nothing is stored to tune it. If that changes, everything it keeps will be listed here, and you will be able to edit or delete each entry.")
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("ai-memory-empty")
            Text("Your questions, answers and project text are never kept in the AI log. It holds numbers only.")
                .font(.caption).foregroundStyle(theme["muted"])
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Learned memory")
        .accessibilityIdentifier("ai-memory-card")
    }
}

// The AI Assistant panel (Windows: ai/AiPanel.tsx). It docks beside the page as an inspector, so
// opening it narrows the page and never covers a control. Until Straft AI is downloaded, and
// when a downloaded file is missing or damaged, it offers the download; a missing or damaged
// engine says to reinstall. Everything else in Straft Studio works without it.

import AppKit
import StraftCore
import SwiftUI

struct AIPanelView: View {
    let workspace: Workspace
    let onClose: () -> Void
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: AIAssistantModel?
    @State private var download: AIDownloadModel?

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            content
        }
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Straft AI")
        .accessibilityIdentifier("ai-panel")
        .onAppear { makeModels() }
        .onChange(of: workspace) { _, new in model?.workspace = new }
        .task(id: app.isRunning) { await poll() }
    }

    private func makeModels() {
        if model == nil { model = AIAssistantModel(app: app) }
        if download == nil {
            let d = AIDownloadModel(app: app)
            d.pickFile = { AIFilePicker.chooseModelFile() }
            download = d
        }
        model?.workspace = workspace
    }

    /// Reads the engine state while the panel is open; faster while a download runs.
    private func poll() async {
        makeModels()
        while !Task.isCancelled {
            await model?.refreshStatus()
            if model?.mode == .ready { await model?.refreshIndex() }
            if model?.mode == .download { await download?.refresh() }
            let fast = download?.isRunning == true || model?.index?.preparing == true
            try? await Task.sleep(for: .milliseconds(fast ? 500 : 2000))
        }
    }

    // MARK: Header

    private var header: some View {
        HStack(spacing: 8) {
            Image(systemName: "sparkles").accessibilityHidden(true)
            Text("Straft AI").font(.headline)
            Spacer()
            BDChip(text: model?.status?.mode ?? "Off", tone: model?.status?.available == true ? .green : .plain)
                .help(model?.status?.available == true ? "Runs on this computer" : "Straft AI is off")
                .accessibilityIdentifier("ai-mode-chip")
            if model?.status != nil { settingsMenu }
            Button(action: onClose) { Label("Close", systemImage: "xmark") }
                .buttonStyle(.borderless)
                .help("Close Straft AI")
                .accessibilityLabel("Close Straft AI")
                .accessibilityIdentifier("ai-close")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var settingsMenu: some View {
        Menu {
            Text("Straft AI")
            Button("Straft AI settings\u{2026}") { SettingsRouter.open(tab: .ai) }
            .accessibilityIdentifier("ai-open-settings")
            if model?.status?.available == true {
                Button("Stop Straft AI (free memory)") { Task { await model?.stopRuntime() } }
                    .accessibilityIdentifier("ai-stop-runtime")
            }
        } label: {
            Label("Settings", systemImage: "ellipsis.circle")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .help("Straft AI settings")
        .accessibilityLabel("Straft AI settings")
        .accessibilityIdentifier("ai-settings-menu")
    }

    // MARK: Body by mode

    @ViewBuilder private var content: some View {
        if let model {
            if model.status == nil && !model.statusFailed {
                VStack(spacing: 10) {
                    ProgressView().controlSize(.small)
                    Text("Checking Straft AI\u{2026}").font(.callout).foregroundStyle(theme["muted"])
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .accessibilityIdentifier("ai-loading")
            } else {
                switch model.mode {
                case .unavailable: AIUnavailableView(text: AIText.unavailable)
                case .off: AIUnavailableView(text: model.status.map(AILogic.offReason) ?? AIText.unavailable)
                case .download:
                    ScrollView {
                        if let download {
                            AIDownloadCard(model: download, damaged: model.status.flatMap(AILogic.damagedModels))
                                .padding(12)
                        }
                    }
                case .ready: AIConversationView(model: model)
                }
            }
        } else {
            Color.clear
        }
    }
}

/// Straft AI cannot run: the subsystem is unreachable, or the engine in the app is missing or damaged.
struct AIUnavailableView: View {
    let text: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "sparkles").font(.largeTitle).foregroundStyle(theme["muted"]).accessibilityHidden(true)
            Text("AI is not available right now").font(.headline)
            Text(text).font(.callout).multilineTextAlignment(.center).fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("ai-unavailable-reason")
            Text("Everything else in Straft Studio works normally.").font(.caption).foregroundStyle(theme["muted"])
        }
        .padding(20)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ai-unavailable")
    }
}

enum AIFilePicker {
    /// The Open panel for a model file the person already has.
    @MainActor static func chooseModelFile() -> URL? {
        let panel = NSOpenPanel()
        panel.title = "Import a Straft AI model file"
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.allowedContentTypes = []
        panel.message = "Choose a model file (.gguf)."
        panel.prompt = "Import"
        return panel.runModal() == .OK ? panel.url : nil
    }

    /// The Save panel for a JSON file (AI report, conversation for review).
    @MainActor static func chooseSaveLocation(title: String, name: String) -> URL? {
        let panel = NSSavePanel()
        panel.title = title
        panel.nameFieldStringValue = name
        panel.allowedContentTypes = [.json]
        panel.canCreateDirectories = true
        return panel.runModal() == .OK ? panel.url : nil
    }
}

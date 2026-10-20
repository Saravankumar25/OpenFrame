// The AI Assistant panel (Windows: ai/AiPanel.tsx). It docks beside the page as an inspector, so
// opening it narrows the page and never covers a control. Until OpenFrame AI is downloaded, and
// when a downloaded file is missing or damaged, it offers the download; a missing or damaged
// engine says to reinstall. Everything else in OpenFrame works without it.

import AppKit
import OpenFrameCore
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
        .accessibilityLabel("AI Assistant")
        .accessibilityIdentifier("ai-panel")
        .onAppear { makeModels() }
        .onChange(of: AIScopeHints.shared.context(for: workspace)) { _, new in model?.setContext(new) }
        .task(id: app.isRunning) { await poll() }
    }

    private func makeModels() {
        if model == nil { model = AIAssistantModel(app: app) }
        if download == nil {
            let d = AIDownloadModel(app: app)
            d.pickFile = { AIFilePicker.chooseModelFile() }
            download = d
        }
        model?.setContext(AIScopeHints.shared.context(for: workspace))
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
            Text("AI Assistant").font(.headline)
            Spacer()
            BDChip(text: model?.status?.mode ?? "Off", tone: model?.status?.available == true ? .green : .plain)
                .help(model?.status?.available == true ? "Runs on this computer" : "OpenFrame AI is off")
                .accessibilityIdentifier("ai-mode-chip")
            if model?.status != nil { settingsMenu }
            Button(action: onClose) { Label("Close", systemImage: "xmark") }
                .buttonStyle(.borderless)
                .help("Close AI Assistant")
                .accessibilityLabel("Close AI Assistant")
                .accessibilityIdentifier("ai-close")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var settingsMenu: some View {
        Menu {
            Text("Offline AI")
            Button("Offline AI settings\u{2026}") { SettingsRouter.open(tab: .ai) }
            .accessibilityIdentifier("ai-open-settings")
            if model?.status?.available == true {
                Button("Stop Offline AI (free memory)") { Task { await model?.stopRuntime() } }
                    .accessibilityIdentifier("ai-stop-runtime")
            }
        } label: {
            Label("Settings", systemImage: "ellipsis.circle")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .help("Offline AI settings")
        .accessibilityLabel("Offline AI settings")
        .accessibilityIdentifier("ai-settings-menu")
    }

    // MARK: Body by mode

    @ViewBuilder private var content: some View {
        if let model {
            if model.status == nil && !model.statusFailed {
                VStack(spacing: 10) {
                    ProgressView().controlSize(.small)
                    Text("Checking OpenFrame AI\u{2026}").font(.callout).foregroundStyle(theme["muted"])
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

/// OpenFrame AI cannot run: the subsystem is unreachable, or the engine in the app is missing or damaged.
struct AIUnavailableView: View {
    let text: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "sparkles").font(.largeTitle).foregroundStyle(theme["muted"]).accessibilityHidden(true)
            Text("AI is not available right now").font(.headline)
            Text(text).font(.callout).multilineTextAlignment(.center).fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("ai-unavailable-reason")
            Text("Everything else in OpenFrame works normally.").font(.caption).foregroundStyle(theme["muted"])
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
        panel.title = "Import an OpenFrame AI model file"
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.allowedContentTypes = []
        panel.message = "Choose a model file (.gguf)."
        panel.prompt = "Import"
        return panel.runModal() == .OK ? panel.url : nil
    }

    /// The Save panel for a JSON file (AI report, reported output).
    @MainActor static func chooseSaveLocation(title: String, name: String) -> URL? {
        let panel = NSSavePanel()
        panel.title = title
        panel.nameFieldStringValue = name
        panel.allowedContentTypes = [.json]
        panel.canCreateDirectories = true
        return panel.runModal() == .OK ? panel.url : nil
    }
}

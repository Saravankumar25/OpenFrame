// The "Download OpenFrame AI" card (Windows: ai/DownloadAi.tsx). Shown in the assistant panel and in
// Settings, Offline AI, while OpenFrame AI is not installed or a downloaded file is damaged.
// Before the click it shows the size, the free space needed and that the download uses the
// internet; nothing touches the network until the click. Then: progress with Pause, Resume where
// it stopped, a plain reason and Try again when it fails, and a way to import files the person has.

import OpenFrameCore
import SwiftUI

struct AIDownloadCard: View {
    @Bindable var model: AIDownloadModel
    var damaged: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let d = model.download {
                content(d)
            } else if model.loadFailed {
                BDBanner(tone: .err, content: { Text("OpenFrame could not show the download right now.") }) {
                    Button("Try again") { Task { await model.refresh() } }
                        .accessibilityIdentifier("ai-download-reload")
                }
            } else {
                ProgressView().controlSize(.small)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Download OpenFrame AI")
        .accessibilityIdentifier("ai-download-card")
        .task { await model.refresh() }
    }

    @ViewBuilder private func content(_ d: AiDownloadDto) -> some View {
        let isDamaged = damaged != nil
        let running = AILogic.running.contains(d.phase)
        let noSpace = AILogic.tooLittleSpace(d, damaged: isDamaged)
        Text(isDamaged ? "Download OpenFrame AI again" : "Download OpenFrame AI")
            .font(.headline)
            .accessibilityAddTraits(.isHeader)
            .accessibilityIdentifier("ai-download-title")
        if let damaged { BDBanner(.err, damaged).accessibilityIdentifier("ai-download-damaged") }
        if !d.canDownload {
            Text("This copy of OpenFrame Studio cannot download OpenFrame AI. You can import model files you already have. Everything else works normally.")
                .font(.callout)
                .accessibilityIdentifier("ai-download-cannot")
            HStack { importButton }
        } else {
            Text(AILogic.downloadLine(d)).font(.callout)
                .accessibilityIdentifier("ai-download-line")
            if running || d.phase == "Paused" { progress(d) }
            if d.phase == "Failed", let problem = d.problem {
                BDBanner(.err, problem.message).accessibilityIdentifier("ai-download-problem")
            }
            if let error = model.error, d.problem?.message != error {
                BDBanner(.err, error).accessibilityIdentifier("ai-download-error")
            }
            if !running && noSpace {
                BDBanner(.warn, "There is not enough free space on this drive. Free some space, then download.")
                    .accessibilityIdentifier("ai-download-no-space")
            }
            if !running {
                Text("The download uses the internet once. After it, OpenFrame AI works on this computer without the internet, and your projects are never sent anywhere.")
                    .font(.caption).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("ai-download-privacy")
            }
            HStack {
                if d.phase == "Downloading" {
                    Button("Pause") { Task { await model.pause() } }
                        .disabled(model.busy)
                        .accessibilityIdentifier("ai-download-pause")
                } else if !running {
                    Button(startLabel(d, damaged: isDamaged)) { Task { await model.start(damaged: isDamaged) } }
                        .buttonStyle(.ofProminent)
                        .disabled(model.busy || noSpace)
                        .accessibilityIdentifier("ai-download-start")
                }
                if !running { importButton }
            }
        }
    }

    private func startLabel(_ d: AiDownloadDto, damaged: Bool) -> String {
        switch d.phase {
        case "Paused": "Resume download"
        case "Failed": "Try again"
        default: damaged ? "Download again" : "Download OpenFrame AI"
        }
    }

    private var importButton: some View {
        Button("Import a model file\u{2026}") { Task { await model.importFile() } }
            .disabled(model.busy)
            .accessibilityIdentifier("ai-download-import")
    }

    @ViewBuilder private func progress(_ d: AiDownloadDto) -> some View {
        Group {
            if let fraction = AILogic.downloadFraction(d) {
                ProgressView(value: fraction)
            } else {
                // Unknown size (getting the list, checking the files): no value, so it reads as busy.
                ProgressView().progressViewStyle(.linear)
            }
        }
        .accessibilityLabel("OpenFrame AI download")
        .accessibilityValue(d.totalBytes > 0 ? AILogic.downloadLine(d) : AILogic.phaseAnnouncement(d.phase))
        .accessibilityIdentifier("ai-download-progress")
    }
}

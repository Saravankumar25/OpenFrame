// The exchange export sheet (mockup 159): Script Review and Beat Board packages, the two
// exchange types still offered (owner change list item 9, 2026-10-19).

import SwiftUI
import OpenFrameCore

struct ExchangeExportSheet: View {
    @State var model: ExchangeExportModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        IXSheetFrame(title: model.heading.title, sub: model.heading.sub.ixNonEmpty, width: 600, idPrefix: "exchange-export-sheet") {
            if let src = model.sources {
                if let d = model.done {
                    VStack(alignment: .leading, spacing: 8) {
                        PKDoneBanner(title: "Package exported.", path: d.path, app: model.app) {
                            Text("Send it by any normal file transfer. Nothing in the package changes your project, and later changes to your project don't change the package.")
                        }
                    }
                } else {
                    form(src)
                }
            } else if let e = model.loadError {
                IXBanner(.err, e)
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 140)
            }
        } footer: {
            if model.done != nil {
                Button("Close", action: close).keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                    .accessibilityIdentifier("exchange-export-close")
            } else {
                Button("Cancel", action: close).keyboardShortcut(.cancelAction).accessibilityIdentifier("exchange-export-cancel")
                Button(model.busy ? "Exporting\u{2026}" : "Export Package\u{2026}") {
                    guard let s = model.summary,
                          let url = picker.saveFile(title: "Export \(PKInfo.entry(model.packageType).label)", suggestedName: s.suggestedFileName, directory: nil, message: nil)
                    else { return }
                    Task { await model.export(to: url) }
                }
                .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                .disabled(model.summary == nil || model.busy || !model.ready)
                .accessibilityIdentifier("exchange-export-confirm")
            }
        }
        .task { await model.load() }
        .onChange(of: model.argsKey) { _, _ in Task { await model.refreshPreview() } }
    }

    @ViewBuilder private func form(_ src: PackageExchangeSources) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            if model.packageType == .scriptReview {
                field("Source draft") {
                    Picker("Source draft", selection: $model.draftId) {
                        ForEach(src.drafts, id: \.id) { Text("\($0.name)\($0.isCurrent ? " (current)" : "") \u{00B7} \(Int($0.sceneCount)) scenes").tag($0.id) }
                    }
                    .labelsHidden().fixedSize()
                    .accessibilityLabel("Source draft").accessibilityIdentifier("exchange-draft")
                }
            }
            if !model.scopeChoices.isEmpty {
                field("Scope") {
                    Picker("Scope", selection: Binding(get: { model.scope }, set: { s in Task { await model.setScope(s) } })) {
                        ForEach(model.scopeChoices, id: \.value) { Text($0.label).tag($0.value) }
                    }
                    .pickerStyle(.segmented).labelsHidden().fixedSize()
                    .accessibilityLabel("Scope").accessibilityIdentifier("exchange-scope")
                }
            }
            if model.scope == .act {
                field("Act") {
                    Picker("Act", selection: $model.actId) {
                        ForEach(src.acts, id: \.id) { Text($0.label).tag($0.id) }
                    }
                    .labelsHidden().fixedSize()
                    .accessibilityLabel("Act").accessibilityIdentifier("exchange-act")
                }
            }
            if model.scope == .scenes {
                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        if model.scenesLoading { ProgressView().controlSize(.small) }
                        ForEach(model.scenes, id: \.id) { s in
                            Toggle("\(s.label)  \(s.heading)", isOn: Binding(
                                get: { model.sceneIds.contains(s.id) }, set: { model.toggleScene(s.id, $0) }))
                                .toggleStyle(.checkbox).lineLimit(1)
                                .accessibilityIdentifier("exchange-scene-\(s.id)")
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 150).padding(8)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Scenes to include")
                .accessibilityIdentifier("exchange-scene-list")
            }
            VStack(alignment: .leading, spacing: 4) {
                Toggle("Include comments", isOn: Binding(get: { model.includeComments || model.scope == .comments }, set: { model.includeComments = $0 }))
                    .disabled(model.scope == .comments).accessibilityIdentifier("exchange-include-comments")
                let available = Int(model.summary?.attachmentsAvailable ?? 0)
                Toggle("Include attachments\(model.summary != nil ? " (\(available))" : "")",
                       isOn: Binding(get: { model.includeAttachments && available > 0 }, set: { model.includeAttachments = $0 }))
                    .disabled(available == 0).accessibilityIdentifier("exchange-include-attachments")
                Toggle("Include private notes (not allowed in review packages)", isOn: .constant(false))
                    .disabled(true).accessibilityIdentifier("exchange-include-private")
            }
            .toggleStyle(.checkbox)
            if let e = model.previewError {
                IXBanner(.warn, e).accessibilityIdentifier("exchange-preview-error")
            } else if let s = model.summary {
                summaryCard(s)
            } else if !model.ready {
                IXBanner(.info, "Choose at least one scene.").accessibilityIdentifier("exchange-choose-scene")
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 60)
            }
            if let e = model.exportError { IXBanner(.err, e).accessibilityIdentifier("exchange-export-error") }
        }
    }

    private func field<C: View>(_ title: String, @ViewBuilder _ c: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title).font(.callout.weight(.semibold))
            c()
        }
    }

    private func summaryCard(_ s: PackageExportSummary) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("This package will contain").font(.callout.weight(.semibold))
            (Text("Project: ") + Text(s.projectTitle).bold() + Text(" \u{00B7} Target: ") + Text(s.target).bold()
             + Text(" \u{00B7} Scope: \(s.scopeLabel) \u{00B7} Exported by: \(s.exportedBy) \u{00B7} Date: \(PKText.day(ms: s.date))"))
                .font(.callout)
            Text(s.counts.map { "\(Int($0.count)) \($0.label.lowercased())" }.joined(separator: " \u{00B7} ")).font(.callout)
            Label("Private notes: excluded", systemImage: "lock").font(.callout.weight(.semibold))
            Text(s.explanation).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("exchange-summary")
    }
}

// Small shared pieces of the Visual Planning screens and the Budget screen: the page header,
// banners, a picture of an asset, fields that save when the person leaves them, a form sheet and
// the export sheet. Mirrors workspaces/production/visual/shared.tsx and the shared export dialog.

import AppKit
import OpenFrameCore
import SwiftUI

// MARK: Page header and banners

struct VisualPageHeader<Actions: View>: View {
    let title: String
    let subtitle: String
    @ViewBuilder var actions: () -> Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.title2.weight(.semibold)).accessibilityAddTraits(.isHeader)
                Text(subtitle).font(.callout).foregroundStyle(theme["muted"])
            }
            Spacer()
            actions()
        }
        .padding(.horizontal, 16).padding(.top, 12).padding(.bottom, 8)
    }
}

enum VisualTone { case info, warn, error }

struct VisualBanner<Content: View, Actions: View>: View {
    var tone: VisualTone = .info
    var identifier: String
    @ViewBuilder var content: () -> Content
    @ViewBuilder var actions: () -> Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            content().font(.callout).frame(maxWidth: .infinity, alignment: .leading)
            actions()
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(fill, in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(edge))
        .padding(.horizontal, 16).padding(.bottom, 8)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(identifier)
    }

    private var fill: Color {
        switch tone {
        case .info: theme["blue-soft"]
        case .warn: theme["yellow-soft"]
        case .error: theme["red-soft"]
        }
    }
    private var edge: Color {
        switch tone {
        case .info: theme["line"]
        case .warn: theme["yellow-line"]
        case .error: theme["red-t"]
        }
    }
}

/// "Scene 12 changed since planning" (what, when, action).
struct SceneReviewBanner: View {
    let hub: VisualHub
    let scene: VisualScene
    let what: String

    var body: some View {
        if scene.needsReview {
            let when = VisualText.formatWhen(scene.changedAt)
            VisualBanner(tone: .warn, identifier: "scene-review-banner") {
                Text("**Scene \(scene.number) changed since planning\(when.isEmpty ? "" : " (\(when))").** Your \(what) are kept. Check them against the script, then mark the scene reviewed.")
            } actions: {
                Button("Mark reviewed") { Task { await hub.markReviewed(scene) } }
                    .accessibilityIdentifier("scene-mark-reviewed")
            }
        }
    }
}

struct RemovedSceneBanner: View {
    let heading: String
    let what: String

    var body: some View {
        VisualBanner(tone: .info, identifier: "removed-scene-banner") {
            Text("**Scene removed from the current script\(heading.isEmpty ? "" : ": \(heading)").** Its \(what) are kept for reference and hidden from active planning.")
        } actions: { EmptyView() }
    }
}

/// Shown when the project has no screenplay scenes to plan against.
struct VisualNoScriptState: View {
    let what: String

    var body: some View {
        ContentUnavailableView {
            Label("No screenplay scenes yet", systemImage: "doc.text")
        } description: {
            Text("\(what) are planned scene by scene. Write or import a screenplay first. Scene headings come from the script, so you never type them twice.")
        } actions: {
            Button("Open Screenplay") {
                NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.screenplay.rawValue)
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("visual-open-screenplay")
        }
        .accessibilityIdentifier("visual-no-script")
    }
}

// MARK: Pictures

/// The picture of an asset, with a plain stand-in while it loads or when the file is missing or
/// cannot be read. Large pictures are reduced while they are read.
struct VisualAssetImage: View {
    let asset: AssetInfo?
    var fill = true
    var maxPixel = 640
    var missingText = "Image unavailable"
    var emptyText: String?

    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var image: NSImage?
    @State private var failed = false

    var body: some View {
        ZStack {
            theme["surface2"]
            if let image {
                Image(nsImage: image).resizable().aspectRatio(contentMode: fill ? .fill : .fit)
            } else if asset == nil {
                if let emptyText { Text(emptyText).font(.caption).foregroundStyle(theme["muted"]) }
            } else {
                VStack(spacing: 4) {
                    Image(systemName: failed || asset?.available == false ? "photo.badge.exclamationmark" : "photo")
                        .foregroundStyle(theme["muted"])
                    if failed || asset?.available == false {
                        Text("\(missingText)\(asset.map { ": \($0.originalName)" } ?? "")")
                            .font(.caption2).foregroundStyle(theme["muted"]).multilineTextAlignment(.center).lineLimit(2)
                    }
                }
                .padding(6)
            }
        }
        .clipped()
        .task(id: asset?.id) {
            image = nil
            failed = false
            guard let asset, asset.available, let core = app.core else { return }
            if let loaded = await VaultThumbnails.shared.image(core: core, assetId: asset.id, global: false, maxPixel: maxPixel) {
                image = loaded
            } else {
                failed = true
            }
        }
    }
}

// MARK: Fields that save when the person leaves them

/// A one-line field that saves on Return and when focus leaves it, and shows the stored text again
/// when the stored text changes elsewhere.
struct CommitField: View {
    let label: String
    let value: String
    var prompt = ""
    var identifier: String
    var suggestions: [String] = []
    var onCommit: (String) -> Void

    @State private var text = ""
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: 4) {
            TextField(label, text: $text, prompt: Text(prompt))
                .focused($focused)
                .onSubmit(commit)
                .onChange(of: focused) { _, now in if !now { commit() } }
                .accessibilityIdentifier(identifier)
            if !suggestions.isEmpty {
                Menu {
                    ForEach(suggestions, id: \.self) { s in
                        Button(s) {
                            text = s
                            commit()
                        }
                    }
                } label: {
                    Image(systemName: "chevron.down.circle")
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .accessibilityLabel("\(label) suggestions")
                .accessibilityIdentifier("\(identifier)-suggestions")
            }
        }
        .onAppear { text = value }
        .onChange(of: value) { _, v in if !focused { text = v } }
    }

    private func commit() {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if t != value.trimmingCharacters(in: .whitespacesAndNewlines) { onCommit(t) }
    }
}

/// A several-line field that saves when focus leaves it.
struct CommitEditor: View {
    let label: String
    let value: String
    var identifier: String
    var minHeight: CGFloat = 54
    var onCommit: (String) -> Void

    @State private var text = ""
    @FocusState private var focused: Bool
    @Environment(\.ofTheme) private var theme

    var body: some View {
        TextEditor(text: $text)
            .font(.body)
            .focused($focused)
            .scrollContentBackground(.hidden)
            .padding(4)
            .frame(minHeight: minHeight)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 6))
            .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line2"]))
            .onChange(of: focused) { _, now in if !now, text != value { onCommit(text) } }
            .onAppear { text = value }
            .onChange(of: value) { _, v in if !focused { text = v } }
            .accessibilityLabel(label)
            .accessibilityIdentifier(identifier)
    }
}

struct LabeledBlock<Content: View>: View {
    let label: String
    var hint: String?
    var error: String?
    @ViewBuilder var content: () -> Content
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
            content()
            if let error, !error.isEmpty {
                Text(error).font(.caption).foregroundStyle(theme["red-t"])
                    .accessibilityIdentifier("field-error")
            } else if let hint {
                Text(hint).font(.caption).foregroundStyle(theme["muted"])
            }
        }
    }
}

// MARK: Form sheet

/// A dialog with a title, a body and Cancel and a main button.
struct VisualFormSheet<Content: View>: View {
    let title: String
    var primary: String
    var primaryEnabled = true
    var busy = false
    var identifier: String
    var width: CGFloat = 440
    var onCancel: () -> Void
    var onPrimary: () -> Void
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
            content()
            HStack {
                Spacer()
                Button("Cancel", role: .cancel, action: onCancel)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("\(identifier)-cancel")
                Button(primary, action: onPrimary)
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(!primaryEnabled || busy)
                    .accessibilityIdentifier("\(identifier)-primary")
            }
        }
        .padding(20)
        .frame(width: width)
        .accessibilityIdentifier(identifier)
    }
}

/// A sheet that asks for one scene.
struct PickSceneSheet: View {
    let title: String
    let scenes: [VisualScene]
    var excluding: String?
    var initial: String?
    var confirm: String
    var identifier: String
    var onCancel: () -> Void
    var onPick: (String) -> Void

    @State private var scene = ""

    var body: some View {
        let choices = scenes.filter { $0.lineageId != excluding }
        VisualFormSheet(title: title, primary: confirm, primaryEnabled: !scene.isEmpty, identifier: identifier, onCancel: onCancel, onPrimary: { onPick(scene) }) {
            LabeledBlock(label: "Scene") {
                Picker("Scene", selection: $scene) {
                    ForEach(choices, id: \.lineageId) { Text(VisualText.sceneLabel($0)).tag($0.sceneId) }
                }
                .labelsHidden()
                .accessibilityIdentifier("\(identifier)-scene")
            }
        }
        .onAppear {
            scene = choices.first { $0.sceneId == initial }?.sceneId ?? choices.first?.sceneId ?? ""
        }
    }
}

// MARK: Export

struct VisualExportOption: Identifiable {
    var key: String
    var label: String
    var isOn: Bool
    var id: String { key }
}

struct VisualExportScope: Identifiable {
    var id: String
    var label: String
    var disabled = false
    /// The scope asks for a list of items to pick (for example "Selected scenes").
    var picks = false
}

struct VisualExportRequest {
    var format: String
    var path: String
    var scope: String
    var picked: [String]
    var flags: [String: Bool]
}

/// The export sheet: format, what to include, then Save. The result is shown when it is done.
struct VisualExportSheet: View {
    let title: String
    var note: String?
    let documentName: String
    let formats: [VisualExportFormat]
    var scopes: [VisualExportScope] = []
    var pickItems: [(id: String, label: String)] = []
    var options: [VisualExportOption]
    var identifier: String
    var onCancel: () -> Void
    var onExport: (VisualExportRequest) async -> ExportResult?

    @State private var format = "pdf"
    @State private var scope = ""
    @State private var picked: Set<String> = []
    @State private var flags: [String: Bool] = [:]
    @State private var working = false
    @State private var result: ExportResult?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
            if let result {
                done(result)
            } else {
                form
            }
        }
        .padding(20)
        .frame(width: 480)
        .accessibilityIdentifier(identifier)
        .onAppear {
            format = formats.first?.id ?? "pdf"
            scope = scopes.first { !$0.disabled }?.id ?? ""
            flags = Dictionary(uniqueKeysWithValues: options.map { ($0.key, $0.isOn) })
        }
    }

    private var needsPick: Bool { scopes.first { $0.id == scope }?.picks ?? false }
    private var canExport: Bool { !working && (!needsPick || !picked.isEmpty) }

    private var form: some View {
        VStack(alignment: .leading, spacing: 12) {
            if formats.count > 1 {
                LabeledBlock(label: "Format") {
                    Picker("Format", selection: $format) { ForEach(formats) { Text($0.label).tag($0.id) } }
                        .pickerStyle(.segmented).labelsHidden()
                        .accessibilityIdentifier("\(identifier)-format")
                }
            }
            if !scopes.isEmpty {
                LabeledBlock(label: "What to export") {
                    Picker("What to export", selection: $scope) {
                        ForEach(scopes) { Text($0.label).tag($0.id).disabled($0.disabled) }
                    }
                    .pickerStyle(.radioGroup).labelsHidden()
                    .accessibilityIdentifier("\(identifier)-scope")
                }
                if needsPick {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(pickItems, id: \.id) { item in
                                Toggle(item.label, isOn: Binding(get: { picked.contains(item.id) }, set: { on in
                                    if on { picked.insert(item.id) } else { picked.remove(item.id) }
                                }))
                                .accessibilityIdentifier("\(identifier)-pick-\(item.id)")
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .frame(maxHeight: 140)
                }
            }
            if !options.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(options) { o in
                        Toggle(o.label, isOn: Binding(get: { flags[o.key] ?? o.isOn }, set: { flags[o.key] = $0 }))
                            .accessibilityIdentifier("\(identifier)-\(o.key)")
                    }
                }
            }
            if let note { Text(note).font(.caption).foregroundStyle(theme["muted"]) }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel, action: onCancel)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("\(identifier)-cancel")
                Button("Export\u{2026}") { Task { await run() } }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(!canExport)
                    .accessibilityIdentifier("\(identifier)-primary")
            }
        }
    }

    private func done(_ r: ExportResult) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Exported \(r.fileName)", systemImage: "checkmark.circle")
                .accessibilityIdentifier("\(identifier)-done")
            Text(r.summary).font(.callout).foregroundStyle(theme["ink2"])
            ForEach(r.warnings, id: \.self) { Text($0).font(.caption).foregroundStyle(theme["yellow-t"]) }
            HStack {
                Spacer()
                Button("Show in Finder") { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: r.path)]) }
                    .accessibilityIdentifier("\(identifier)-reveal")
                Button("Done", action: onCancel)
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("\(identifier)-close")
            }
        }
    }

    private func run() async {
        guard let path = VisualPanels.chooseSavePath(name: "\(documentName).\(format)", message: "Choose where to save the export") else { return }
        working = true
        defer { working = false }
        result = await onExport(VisualExportRequest(format: format, path: path, scope: scope, picked: Array(picked), flags: flags))
    }
}

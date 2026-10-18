// The Import Screenplay sheet: Select source, Preview interpretation, Import. Same words and
// steps as the Windows dialog (mocks 091 to 094).

import SwiftUI
import OpenFrameCore

struct ImportScreenplaySheet: View {
    @State var model: ImportScreenplayModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            switch model.step {
            case .source: sourceStep
            case .preview: previewStep
            case .report: reportStep
            case .failed: failedStep
            }
        }
        .task { await model.loadSamples() }
        .onDisappear { model.finish() }
    }

    private func closeSheet() { model.finish(); close() }

    // MARK: Steps bar

    private func steps(_ active: Int) -> some View {
        HStack(spacing: 6) {
            ForEach(Array(["1  Select source", "2  Preview interpretation", "3  Import"].enumerated()), id: \.offset) { i, label in
                IXChip(text: label, tone: i == active ? .dark : .plain)
                    .accessibilityAddTraits(i == active ? .isSelected : [])
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Import steps")
        .accessibilityIdentifier("import-steps")
    }

    // MARK: 1 Source

    private var sourceStep: some View {
        IXSheetFrame(
            title: "Import Screenplay",
            sub: "What existing script do you want to bring into OpenFrame?",
            width: 700, idPrefix: "import-sheet"
        ) {
            VStack(alignment: .leading, spacing: 12) {
                steps(0)
                HStack(alignment: .top, spacing: 14) {
                    fileCard
                    pasteBox
                }
                if !model.samples.isEmpty { samplesSection }
                IXBanner(.info, "Importing never replaces your current screenplay. If one exists, the script arrives as a new screenplay or a new draft.")
            }
        } footer: {
            Button("Cancel", action: closeSheet).keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("import-cancel")
            Button(model.busy ? "Reading\u{2026}" : "Preview") { Task { await model.runPreview() } }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(.borderedProminent)
                .disabled(!model.canPreview)
                .accessibilityIdentifier("import-preview")
        }
    }

    private var fileCard: some View {
        VStack(spacing: 6) {
            Image(systemName: model.path == nil ? "folder" : "doc.text").font(.system(size: 26)).accessibilityHidden(true)
            Text(model.pathName ?? "Choose a file").font(.body.weight(.semibold)).lineLimit(2)
                .accessibilityIdentifier("import-file-name")
            Text(IXImportFilter.summary).font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center)
            HStack {
                Button("Browse\u{2026}") {
                    if let url = picker.openFile(title: "Choose a screenplay to import", extensions: IXImportFilter.extensions) {
                        model.choose(file: url)
                    }
                }
                .accessibilityIdentifier("import-browse")
                if model.path != nil {
                    Button("Clear") { model.clearFile() }.buttonStyle(.borderless)
                        .accessibilityIdentifier("import-clear")
                }
            }
        }
        .frame(width: 250, height: 190)
        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line2"], style: StrokeStyle(lineWidth: 1, dash: [5, 4])))
        .onDrop(of: [.fileURL], isTargeted: nil) { providers in
            guard let provider = providers.first else { return false }
            _ = provider.loadObject(ofClass: URL.self) { url, _ in
                if let url { Task { @MainActor in model.choose(file: url) } }
            }
            return true
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Screenplay file")
        .accessibilityIdentifier("import-file-card")
    }

    private var pasteBox: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("\u{2026}or paste screenplay text").font(.callout.weight(.semibold))
            TextEditor(text: $model.pasted)
                .font(.system(size: 12.5, design: .monospaced))
                .scrollContentBackground(.hidden)
                .padding(6)
                .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
                .disabled(model.path != nil || model.sample != nil)
                .opacity(model.path != nil || model.sample != nil ? 0.5 : 1)
                .frame(height: 170)
                .accessibilityLabel("Screenplay text")
                .accessibilityIdentifier("import-paste")
            if model.pasted.isEmpty && model.path == nil && model.sample == nil {
                Text("INT. POLICE STATION - NIGHT, then the action, then a character name and the line.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var samplesSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label("Or open a sample script", systemImage: "book").font(.callout.weight(.semibold))
            Text("Fictional scripts written and owned by OpenFrame, for learning the app.")
                .font(.caption).foregroundStyle(.secondary)
            HStack {
                ForEach(model.samples, id: \.id) { s in
                    Button(s.title) { model.toggleSample(s.id) }
                        .buttonStyle(.bordered)
                        .tint(model.sample == s.id ? theme["accent"] : nil)
                        .help("\(s.description) \(s.formatLabel), \(Int(s.sceneCount)) \(s.sceneCount == 1 ? "scene" : "scenes").")
                        .accessibilityAddTraits(model.sample == s.id ? .isSelected : [])
                        .accessibilityIdentifier("import-sample-\(s.id)")
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Sample scripts")
    }

    // MARK: 2 Preview

    @ViewBuilder private var previewStep: some View {
        if let p = model.preview {
            let banner = IXLabels.previewBanner(p)
            let conf = IXLabels.confidence(p.confidence)
            IXSheetFrame(
                title: "Import Screenplay \u{00B7} Preview",
                sub: "Check how the script was interpreted before anything is created.",
                width: 760, idPrefix: "import-sheet"
            ) {
                VStack(alignment: .leading, spacing: 10) {
                    steps(1)
                    HStack(spacing: 8) {
                        IXFact(label: "Source", value: p.sourceName, detail: p.formatLabel, id: "import-fact-source")
                        IXFact(label: "Scenes", value: "\(Int(p.sceneCount))", id: "import-fact-scenes")
                        IXFact(label: "Characters", value: "\(Int(p.characterCount))", id: "import-fact-characters")
                        IXFact(label: "Pages", value: IXLabels.pages(p.approxPages), id: "import-fact-pages")
                    }
                    HStack(spacing: 6) {
                        IXChip(text: conf.label, tone: conf.tone).help("Parsing confidence \(Int((p.confidenceScore * 100).rounded()))%")
                        if let t = p.title { IXChip(text: "Title: \(t)") }
                        if let a = p.author { IXChip(text: "Written by \(a)") }
                        IXChip(text: "\(Int(p.dialogueCount)) dialogue blocks")
                    }
                    IXBanner(tone: banner.tone == .ok ? .ok : banner.tone == .warn ? .warn : .info,
                             text: Text(banner.title).bold() + Text(" " + banner.text)) {
                        if !p.warnings.isEmpty {
                            Button(model.showWarnings ? "Hide warnings" : "Show warnings") { model.showWarnings.toggle() }
                                .controlSize(.small)
                                .accessibilityIdentifier("import-show-warnings")
                        }
                    }
                    if model.showWarnings { IXWarningList(warnings: p.warnings, id: "import-warnings") }
                    sceneTable(p)
                    destination(p)
                    if !p.canCreateScreenplay {
                        Text("Screenplays in a series live inside episodes. Add an episode first, then import the script into it.")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    HStack(alignment: .bottom, spacing: 16) {
                        VStack(alignment: .leading, spacing: 3) {
                            Text("Draft name").font(.callout.weight(.semibold))
                            TextField(model.draftNamePlaceholder, text: $model.draftName)
                                .textFieldStyle(.roundedBorder)
                                .onChange(of: model.draftName) { _, v in if v.count > 120 { model.draftName = String(v.prefix(120)) } }
                                .accessibilityIdentifier("import-draft-name")
                            Text(model.mode == .newDraft ? "Leave empty to name it after the file. Existing drafts are not changed." : "Leave empty to name it after the file.")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                        Toggle(p.sourceFormat == "pasted" ? "Keep the pasted text in Project Files" : "Keep the original file in Project Files",
                               isOn: $model.keepSource)
                            .toggleStyle(.checkbox)
                            .accessibilityIdentifier("import-keep-source")
                    }
                    if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("import-error") }
                }
            } footer: {
                Button("Back") { model.backToSource() }.disabled(model.busy)
                    .accessibilityIdentifier("import-back")
                Button("Cancel", action: closeSheet).keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("import-cancel")
                Button(model.busy ? "Importing\u{2026}" : "Import") { Task { await model.runImport() } }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .disabled(!model.canImport)
                    .accessibilityIdentifier("import-confirm")
            }
        }
    }

    private func sceneTable(_ p: InterchangeImportPreview) -> some View {
        VStack(spacing: 0) {
            HStack {
                Text("#").frame(width: 36, alignment: .leading)
                Text("Detected scene heading").frame(maxWidth: .infinity, alignment: .leading)
                Text("Status").frame(width: 150, alignment: .leading)
            }
            .font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            .padding(.horizontal, 10).padding(.vertical, 6)
            .background(theme["surface2"])
            Divider()
            ScrollView {
                LazyVStack(spacing: 0) {
                    if p.scenes.isEmpty {
                        Text("No scenes were detected.").foregroundStyle(.secondary).padding(10).frame(maxWidth: .infinity, alignment: .leading)
                    }
                    ForEach(Array(p.scenes.enumerated()), id: \.offset) { i, s in
                        let status = IXLabels.sceneStatus(s.status)
                        let number = p.scenes.prefix(i + 1).filter { !$0.heading.trimmingCharacters(in: .whitespaces).isEmpty }.count
                        let heading = s.heading.trimmingCharacters(in: .whitespaces)
                        HStack(alignment: .firstTextBaseline) {
                            Text(heading.isEmpty ? "" : "\(number)").frame(width: 36, alignment: .leading)
                            VStack(alignment: .leading, spacing: 1) {
                                if heading.isEmpty {
                                    Text("(material before the first scene)").foregroundStyle(.secondary)
                                } else {
                                    Text(heading)
                                }
                                if let n = s.sourceNumber { Text("numbered \(n) in the source").font(.caption).foregroundStyle(.secondary) }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            IXChip(text: status.label, tone: status.tone).frame(width: 150, alignment: .leading)
                        }
                        .font(.callout)
                        .padding(.horizontal, 10).padding(.vertical, 5)
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("import-scene-\(i)")
                        Divider()
                    }
                }
            }
            .frame(maxHeight: 190)
        }
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Detected scenes")
        .accessibilityIdentifier("import-scenes")
    }

    private func destination(_ p: InterchangeImportPreview) -> some View {
        HStack(spacing: 10) {
            Text("Import as").font(.callout.weight(.semibold))
            Picker("Import as", selection: $model.mode) {
                ForEach(model.modes, id: \.mode) { Text($0.label).tag($0.mode) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .accessibilityIdentifier("import-mode")
            if model.mode == .newDraft, p.existingScreenplays.count > 1 {
                Picker("Screenplay that receives the new draft", selection: $model.screenplayId) {
                    ForEach(p.existingScreenplays, id: \.id) { Text(IXLabels.targetLabel(title: $0.title, episodeTitle: $0.episodeTitle)).tag($0.id) }
                }
                .labelsHidden()
                .accessibilityLabel("Screenplay that receives the new draft")
                .accessibilityIdentifier("import-screenplay")
            }
            if model.mode == .newScreenplay, p.episodic {
                Picker("Episode for the new screenplay", selection: $model.episodeId) {
                    ForEach(p.episodes, id: \.id) { Text($0.title).tag($0.id) }
                }
                .labelsHidden()
                .accessibilityLabel("Episode for the new screenplay")
                .accessibilityIdentifier("import-episode")
            }
            Spacer()
        }
    }

    // MARK: 3 Report

    @ViewBuilder private var reportStep: some View {
        if let r = model.report {
            IXSheetFrame(title: "Import complete", width: 560, idPrefix: "import-sheet") {
                VStack(alignment: .leading, spacing: 10) {
                    IXBanner(tone: .ok, text: Text(r.mode == "new_screenplay"
                        ? "Imported as a new screenplay, \u{201C}\(r.screenplayTitle)\u{201D} (\(r.draftLabel))."
                        : "Imported as \(r.draftLabel).").bold()
                        + Text(" " + (r.mode == "new_draft" ? "Your other drafts were not changed." : "Nothing else in your project was changed.")))
                    IXKeyValueTable(rows: reportRows(r), id: "import-report")
                    if model.showWarnings { IXWarningList(warnings: r.warnings, id: "import-warnings") }
                }
            } footer: {
                if !r.warnings.isEmpty {
                    Button(model.showWarnings ? "Hide Warnings" : "Review Warnings") { model.showWarnings.toggle() }
                        .accessibilityIdentifier("import-review-warnings")
                }
                Button("Open Screenplay") { model.openScreenplay(); closeSheet() }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("import-open-screenplay")
                Button("Close", action: closeSheet).keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("import-close")
            }
        }
    }

    private func reportRows(_ r: InterchangeImportReport) -> [(label: String, value: String)] {
        var rows: [(String, String)] = [
            ("Source file", r.sourceName),
            ("Imported scenes", "\(Int(r.importedScenes))"),
            ("Detected characters", "\(Int(r.detectedCharacters))"),
            ("Pages / length", "\(IXLabels.pages(r.approxPages)) pages"),
            ("Warnings needing attention", "\(Int(r.warningsNeedingAttention))"),
        ]
        if r.keptFileId != nil { rows.append(("Original file", "Kept in Project Files")) }
        return rows.map { (label: $0.0, value: $0.1) }
    }

    // MARK: Failed

    @ViewBuilder private var failedStep: some View {
        if let f = model.failure {
            let canKeep = f.path != nil && IXLabels.canKeepAsProjectFile(code: f.code)
            IXSheetFrame(
                title: "Import failed. Your current project was not changed.",
                width: 560, idPrefix: "import-sheet"
            ) {
                VStack(alignment: .leading, spacing: 10) {
                    IXKeyValueTable(rows: [
                        (f.path != nil ? "File" : "Source", f.path.map(IXNames.fileName) ?? "Pasted text"),
                        ("Stage", IXLabels.failureStage(code: f.code, pasted: f.pasted)),
                        ("Problem", Self.cleanProblem(f.message)),
                        ("Your project", "Unchanged"),
                    ] + (f.path != nil ? [("Source file", "Untouched")] : []), id: "import-failure")
                    Text(canKeep
                         ? "You can still import it as a project document, or paste the screenplay text into a new draft."
                         : "You can paste the screenplay text instead, or choose another file.")
                        .font(.callout)
                    if let e = model.errorText { IXBanner(.err, e) }
                }
            } footer: {
                Button("Paste Text Instead") { model.pasteInstead() }
                    .accessibilityIdentifier("import-paste-instead")
                if canKeep {
                    Button("Add to Project Files") {
                        Task { if await model.keepAsProjectFile() { closeSheet() } }
                    }
                    .disabled(model.busy)
                    .accessibilityIdentifier("import-add-to-files")
                }
                Button("Close", action: closeSheet).keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("import-close")
            }
        }
    }

    /// The core ends its messages with "Your project was not changed."; the sheet says that itself.
    static func cleanProblem(_ message: String) -> String {
        message.replacingOccurrences(of: "\\s*Your (current )?project was not changed\\.?$", with: "", options: .regularExpression)
    }
}

struct IXWarningList: View {
    let warnings: [InterchangeWarning]
    let id: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(Array(IXLabels.attentionFirst(warnings).enumerated()), id: \.offset) { _, w in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    IXChip(text: w.level == "attention" ? "Needs attention" : "Info", tone: w.level == "attention" ? .yellow : .plain)
                    Text(w.message).font(.callout).fixedSize(horizontal: false, vertical: true)
                }
                .accessibilityElement(children: .combine)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Import warnings")
        .accessibilityIdentifier(id)
    }
}

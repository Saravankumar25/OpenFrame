// The Export Screenplay sheet (mock 095) and its Print Preview (mock 096). The preview shows the
// real PDF with PDFKit; Print and Export PDF use that same document.

import AppKit
import PDFKit
import SwiftUI
import OpenFrameCore

struct ExportScreenplaySheet: View {
    @State var model: ExportScreenplayModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void
    @State private var previewing = false
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            if previewing, !model.draft.isEmpty {
                PrintPreviewSheet(
                    model: PrintPreviewModel(app: model.owner, draftId: model.draft, screenplayTitle: model.choice?.screenplayTitle ?? "", draftName: model.choice?.draftName ?? "", options: model.options),
                    picker: picker, back: { previewing = false }, close: close)
            } else if let r = model.result {
                doneView(r)
            } else {
                form
            }
        }
        .task { await model.load() }
    }

    @ViewBuilder private var form: some View {
        if let f = model.loadFailure {
            IXSheetFrame(title: "Export Screenplay", width: 560, idPrefix: "export-sheet") {
                IXBanner(.err, "The screenplay list could not be loaded. \(f)")
            } footer: {
                Button("Close", action: close).keyboardShortcut(.cancelAction).accessibilityIdentifier("export-close")
            }
        } else if !model.loaded {
            IXSheetFrame(title: "Export Screenplay", width: 560, idPrefix: "export-sheet") {
                ProgressView().frame(maxWidth: .infinity, minHeight: 140)
            } footer: {
                Button("Cancel", action: close).keyboardShortcut(.cancelAction).accessibilityIdentifier("export-cancel")
            }
        } else if model.empty {
            IXSheetFrame(title: "Export Screenplay", width: 520, idPrefix: "export-sheet") {
                VStack(alignment: .leading, spacing: 4) {
                    Text("No screenplay to export yet").font(.headline)
                    Text("Write or import a screenplay first. Exports are made from a screenplay draft.").foregroundStyle(.secondary)
                }
                .accessibilityIdentifier("export-empty")
            } footer: {
                Button("Close", action: close).keyboardShortcut(.defaultAction).accessibilityIdentifier("export-close")
            }
        } else if let choice = model.choice {
            options(choice)
        }
    }

    private func options(_ choice: IXDraftChoice) -> some View {
        IXSheetFrame(
            title: "Export Screenplay",
            sub: "What am I exporting, which version, in which format, and where will it be saved?",
            width: 680, idPrefix: "export-sheet"
        ) {
            VStack(alignment: .leading, spacing: 12) {
                source(choice)
                HStack(alignment: .top, spacing: 20) {
                    formatColumn
                    scopeColumn
                }
                Divider()
                HStack(spacing: 18) {
                    Toggle("Title page", isOn: $model.titlePage).accessibilityIdentifier("export-title-page")
                    Toggle("Scene numbers", isOn: $model.sceneNumbers).accessibilityIdentifier("export-scene-numbers")
                    Toggle("Revision info", isOn: $model.revisionInfo).disabled(!model.titlePage).accessibilityIdentifier("export-revision-info")
                    Toggle("Include notes", isOn: $model.includeNotes).accessibilityIdentifier("export-include-notes")
                }
                .toggleStyle(.checkbox)
                if model.scope == .selected && model.sceneNumbers {
                    Text("Selected scenes keep their numbers from the full draft.").font(.caption).foregroundStyle(.secondary)
                }
                if let e = model.error { IXBanner(.err, e).accessibilityIdentifier("export-error") }
                IXBanner(tone: .priv, text: Text("Private notes excluded.").bold() + Text(" Export creates a snapshot; your project does not change."))
                    .accessibilityIdentifier("export-private-note")
                if !model.formatsShown.contains(where: \.additional) {
                    Button("Additional formats\u{2026}") { model.showMore = true }
                        .buttonStyle(.link)
                        .accessibilityIdentifier("export-more-formats")
                }
            }
        } footer: {
            Button("Cancel", action: close).keyboardShortcut(.cancelAction).accessibilityIdentifier("export-cancel")
            Button("Preview\u{2026}") { previewing = true }
                .disabled(!model.scopeOk)
                .accessibilityIdentifier("export-preview")
            Button(model.busy ? "Exporting\u{2026}" : "Export\u{2026}") { runExport() }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(.ofProminent)
                .disabled(model.busy || !model.scopeOk)
                .accessibilityIdentifier("export-confirm")
        }
    }

    private func source(_ choice: IXDraftChoice) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Source").font(.callout.weight(.semibold))
            HStack(spacing: 8) {
                if model.choices.count > 1 {
                    Picker("Draft to export", selection: Binding(get: { model.draft }, set: { id in Task { await model.changeDraft(id) } })) {
                        ForEach(model.choices) { Text($0.label).tag($0.id) }
                    }
                    .labelsHidden()
                    .fixedSize()
                    .accessibilityLabel("Draft to export")
                    .accessibilityIdentifier("export-draft")
                } else {
                    Text(choice.label).font(.body.weight(.semibold)).accessibilityIdentifier("export-draft")
                }
                IXChip(text: choice.status, tone: IXLabels.statusTone(choice.status))
                if let r = choice.revision { IXChip(text: r, tone: .yellow) }
                Text("\(choice.sceneCount) scene\(choice.sceneCount == 1 ? "" : "s")").font(.caption).foregroundStyle(.secondary)
            }
        }
    }

    private var formatColumn: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Format").font(.callout.weight(.semibold))
            ForEach(model.formatsShown) { f in
                Button { model.format = f } label: {
                    HStack(spacing: 6) {
                        Image(systemName: model.format == f ? "largecircle.fill.circle" : "circle")
                            .foregroundStyle(model.format == f ? theme["accent"] : theme["muted"])
                            .accessibilityHidden(true)
                        Text(f.label).fontWeight(.semibold)
                        if let h = f.hint { Text(h).font(.caption).foregroundStyle(.secondary) }
                    }
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(model.format == f ? .isSelected : [])
                .accessibilityIdentifier("export-format-\(f.rawValue)")
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Format")
    }

    private var scopeColumn: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Scope").font(.callout.weight(.semibold))
            radio("Entire draft", on: model.scope == .all, id: "export-scope-all") { model.scope = .all }
            radio("Selected scenes", on: model.scope == .selected, id: "export-scope-selected") { model.scope = .selected }
            if model.scope == .selected {
                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        if model.scenesLoading { ProgressView().controlSize(.small) }
                        ForEach(model.scenes, id: \.id) { s in
                            Toggle("\(s.number). \(s.omitted ? "OMITTED" : s.heading)", isOn: Binding(
                                get: { model.picked.contains(s.id) }, set: { model.setPicked(s.id, $0) }))
                                .toggleStyle(.checkbox)
                                .lineLimit(1)
                                .accessibilityIdentifier("export-scene-\(s.id)")
                        }
                        if !model.scenesLoading && model.scenes.isEmpty {
                            Text("This draft has no scenes yet.").font(.caption).foregroundStyle(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 110)
                .padding(6)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Scenes to export")
                .accessibilityIdentifier("export-scene-list")
            }
            Divider()
            radio("Clean script", on: !model.revisionMarks, id: "export-clean") { model.revisionMarks = false }
            radio("Revision-marked version", on: model.revisionMarks, id: "export-revision-marked") { model.revisionMarks = true }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Scope")
    }

    private func radio(_ title: String, on: Bool, id: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: on ? "largecircle.fill.circle" : "circle")
                    .foregroundStyle(on ? theme["accent"] : theme["muted"]).accessibilityHidden(true)
                Text(title)
            }
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(on ? .isSelected : [])
        .accessibilityIdentifier(id)
    }

    private func runExport() {
        guard let c = model.choice else { return }
        let name = IXNames.defaultExportFileName(screenplayTitle: c.screenplayTitle, draftName: c.draftName, format: model.format)
        guard let url = picker.saveFile(title: "Export Screenplay", suggestedName: name, directory: nil, message: nil) else { return }
        Task { await model.export(to: url) }
    }

    private func doneView(_ r: InterchangeScreenplayExportResult) -> some View {
        IXSheetFrame(title: "Export complete", width: 560, idPrefix: "export-sheet") {
            VStack(alignment: .leading, spacing: 10) {
                IXBanner(tone: .ok, text: Text(r.fileName).bold() + Text(" was saved."))
                IXKeyValueTable(rows: [
                    ("Source", r.sourceLabel),
                    ("Format", IXExportFormat(rawValue: r.format)?.label ?? r.format),
                    ("Scenes", "\(Int(r.sceneCount))"),
                ] + (r.pages.map { [("Pages", "\(Int($0))")] } ?? []) + [("Location", r.path)], id: "export-report")
                ForEach(Array(r.warnings.enumerated()), id: \.offset) { _, w in
                    IXBanner(w.level == "attention" ? .warn : .info, w.message)
                }
                IXBanner(tone: .priv, text: Text("Private notes excluded.").bold() + Text(" Export creates a snapshot; your project does not change."))
            }
        } footer: {
            Button("Show in Folder") { Task { await model.reveal() } }.accessibilityIdentifier("export-show-in-folder")
            Button("Close", action: close).keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                .accessibilityIdentifier("export-close")
        }
    }
}

// MARK: Print Preview

struct PrintPreviewSheet: View {
    @State var model: PrintPreviewModel
    var picker: FilePicking = PanelFilePicker()
    let back: (() -> Void)?
    let close: () -> Void
    @State private var window: NSWindow?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            ZStack {
                theme["surface2"]
                if model.document != nil {
                    IXPDFView(document: model.document, pageIndex: Binding(get: { model.pageIndex }, set: { model.pageIndex = $0 }), zoom: model.zoom)
                        .accessibilityLabel("Print preview. \(model.pageLabel)")
                        .accessibilityIdentifier("print-preview-pdf")
                } else if let e = model.error {
                    IXBanner(.err, e).padding(20)
                } else if model.blocking {
                    Text("The PDF preview is not available for this draft.").foregroundStyle(.secondary)
                } else {
                    ProgressView("Preparing the preview\u{2026}")
                }
            }
            .frame(minWidth: 480, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Preview")
            Divider()
            side.frame(width: 240)
        }
        .sheetSize(width: 820, height: 640)
        .background(theme["panel"])
        .background(WindowReader(window: $window))
        .task { await model.load() }
        .onChange(of: model.settingsKey) { _, _ in Task { await model.load() } }
        .onDisappear { model.cleanup() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("print-preview-sheet")
    }

    private var side: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Print Preview").font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
            if !model.sourceLabel.isEmpty { Text(model.sourceLabel).font(.callout).foregroundStyle(.secondary) }
            Text("Preview settings").font(.callout.weight(.semibold))
            VStack(alignment: .leading, spacing: 6) {
                Toggle("Title page", isOn: $model.titlePage).accessibilityIdentifier("print-title-page")
                Toggle("Revision info", isOn: $model.revisionInfo).disabled(!model.titlePage).accessibilityIdentifier("print-revision-info")
                Toggle("Include notes", isOn: $model.includeNotes).accessibilityIdentifier("print-include-notes")
            }
            .toggleStyle(.checkbox)
            Divider()
            HStack {
                Button { model.previous() } label: { Label("Previous", systemImage: "chevron.left") }
                    .disabled(model.currentPage <= 0)
                    .accessibilityLabel("Previous page").accessibilityIdentifier("print-previous")
                IXChip(text: model.pageCount > 0 ? model.pageLabel : "\u{2026}").accessibilityIdentifier("print-page-label")
                Button { model.next() } label: { Label("Next", systemImage: "chevron.right") }
                    .disabled(model.currentPage >= model.pageCount - 1)
                    .accessibilityLabel("Next page").accessibilityIdentifier("print-next")
            }
            HStack {
                Button { model.zoomOut() } label: { Label("Zoom Out", systemImage: "minus") }
                    .disabled(!model.canZoomOut).accessibilityLabel("Zoom out").accessibilityIdentifier("print-zoom-out")
                IXChip(text: "Zoom \(model.zoomLabel)").accessibilityIdentifier("print-zoom-label")
                Button { model.zoomIn() } label: { Label("Zoom In", systemImage: "plus") }
                    .disabled(!model.canZoomIn).accessibilityLabel("Zoom in").accessibilityIdentifier("print-zoom-in")
            }
            Divider()
            ForEach(Array(model.warnings.enumerated()), id: \.offset) { _, w in IXBanner(.warn, w.message) }
            if let e = model.exportError { IXBanner(.err, e).accessibilityIdentifier("print-export-error") }
            if let r = model.exported { IXBanner(.ok, "Exported \(r.fileName).").accessibilityIdentifier("print-exported") }
            Text("The PDF numbers pages from page 2. Changing these settings affects only the outgoing document, never the screenplay itself.")
                .font(.caption).foregroundStyle(.secondary)
            Spacer()
            Button("Print\u{2026}") { model.print(from: window) }
                .keyboardShortcut("p", modifiers: .command)
                .disabled(model.document == nil)
                .accessibilityIdentifier("print-print")
            Button(model.exporting ? "Exporting\u{2026}" : "Export PDF") { exportPDF() }
                .buttonStyle(.ofProminent)
                .keyboardShortcut(.defaultAction)
                .disabled(model.document == nil || model.exporting || model.blocking)
                .accessibilityIdentifier("print-export-pdf")
            HStack {
                if let back { Button("Back to Export Options", action: back).accessibilityIdentifier("print-back") }
                Spacer()
                Button("Close", action: close).keyboardShortcut(.cancelAction).accessibilityIdentifier("print-close")
            }
        }
        .padding(16)
        .frame(maxHeight: .infinity, alignment: .top)
    }

    private func exportPDF() {
        guard let url = picker.saveFile(title: "Export PDF", suggestedName: model.suggestedName, directory: nil, message: nil) else { return }
        Task { await model.exportPDF(to: url) }
    }
}

/// PDFKit page view of the core's PDF. The page and zoom come from the model.
struct IXPDFView: NSViewRepresentable {
    let document: PDFDocument?
    @Binding var pageIndex: Int
    let zoom: Double?

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> PDFView {
        let view = PDFView()
        view.displayMode = .singlePage
        view.displayDirection = .vertical
        view.autoScales = true
        view.displaysPageBreaks = false
        view.backgroundColor = .clear
        NotificationCenter.default.addObserver(
            context.coordinator, selector: #selector(Coordinator.pageChanged(_:)), name: .PDFViewPageChanged, object: view)
        return view
    }

    func updateNSView(_ view: PDFView, context: Context) {
        context.coordinator.parent = self
        if view.document !== document { view.document = document }
        if let zoom {
            view.autoScales = false
            view.scaleFactor = zoom
        } else if !view.autoScales {
            view.autoScales = true
        }
        if let document, document.pageCount > 0 {
            let i = min(max(0, pageIndex), document.pageCount - 1)
            if let page = document.page(at: i), view.currentPage !== page { view.go(to: page) }
        }
    }

    final class Coordinator: NSObject {
        var parent: IXPDFView
        init(_ parent: IXPDFView) { self.parent = parent }

        @MainActor @objc func pageChanged(_ note: Notification) {
            guard let view = note.object as? PDFView, let page = view.currentPage, let doc = view.document else { return }
            let i = doc.index(for: page)
            if i != parent.pageIndex { parent.pageIndex = i }
        }
    }
}

/// Finds the window that hosts a view, for sheet-modal print panels.
struct WindowReader: NSViewRepresentable {
    @Binding var window: NSWindow?
    func makeNSView(context: Context) -> NSView {
        let v = NSView()
        DispatchQueue.main.async { window = v.window }
        return v
    }
    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async { if window !== nsView.window { window = nsView.window } }
    }
}

// Export of a call sheet, sides or a report (Windows features/export). The core writes the file to
// the path the person chose in the Save panel; the project never changes and private notes are
// never included. After a PDF is written the sheet shows it with PDFKit.

import AppKit
import Observation
import PDFKit
import SwiftUI
import OpenFrameCore

struct CSExportFormat: Identifiable, Hashable {
    let value: String
    let label: String
    let hint: String
    var id: String { value }

    static let pdf = CSExportFormat(value: "pdf", label: "PDF", hint: "printable snapshot")
    static let tables: [CSExportFormat] = [
        CSExportFormat(value: "pdf", label: "PDF", hint: "printable"),
        CSExportFormat(value: "csv", label: "CSV", hint: "spreadsheet-friendly text"),
        CSExportFormat(value: "xlsx", label: "Spreadsheet (XLSX)", hint: "one sheet per table"),
    ]
}

struct CSExportOption: Identifiable, Hashable {
    let key: String
    let label: String
    let defaultValue: Bool
    var id: String { key }
}

@MainActor
@Observable
final class CSExportModel: Identifiable {
    nonisolated let id = UUID()
    let title: String
    let sub: String?
    /// "Source: Call Sheet: Day 1 \u{00B7} Draft".
    let source: String
    let documentName: String
    let note: String
    let formats: [CSExportFormat]
    let options: [CSExportOption]
    let projectTitle: String?
    /// The file name without the project and date, when the document has a name of its own
    /// ("Shoot Day 4 - Daily view").
    let plainName: Bool

    var format: String
    var values: [String: Bool]
    private(set) var busy = false
    private(set) var error: String?
    private(set) var result: ExportResult?

    /// Runs the core operation with (format, path, option values).
    private let run: (String, String, [String: Bool]) async throws -> ExportResult

    init(
        title: String, sub: String? = nil, source: String, documentName: String, note: String,
        formats: [CSExportFormat], options: [CSExportOption] = [], projectTitle: String?, plainName: Bool = false,
        run: @escaping (String, String, [String: Bool]) async throws -> ExportResult
    ) {
        self.title = title
        self.sub = sub
        self.source = source
        self.documentName = documentName
        self.note = note
        self.formats = formats
        self.options = options
        self.projectTitle = projectTitle
        self.plainName = plainName
        self.run = run
        format = formats.first?.value ?? "pdf"
        values = Dictionary(uniqueKeysWithValues: options.map { ($0.key, $0.defaultValue) })
    }

    var suggestedName: String {
        plainName ? "\(documentName).\(format)" : CSFormat.exportFileName(project: projectTitle, document: documentName, format: format)
    }

    /// Writes the file. The message of a refusal stays in `error`; nothing is lost.
    @discardableResult
    func export(to path: String) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        defer { busy = false }
        do {
            result = try await run(format, path, values)
            return true
        } catch {
            self.error = CoreFailure(asCore: error).message
            return false
        }
    }

    /// The Save panel. Returns the chosen path, or nil when the person cancelled.
    func choosePath() -> String? {
        let panel = NSSavePanel()
        panel.title = title
        panel.nameFieldStringValue = suggestedName
        panel.canCreateDirectories = true
        panel.isExtensionHidden = false
        guard panel.runModal() == .OK, let url = panel.url else { return nil }
        return url.path
    }
}

// MARK: Sheet

struct CSExportSheet: View {
    @Bindable var model: CSExportModel
    var onClose: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if let r = model.result {
            resultView(r)
        } else {
            form
        }
    }

    private var form: some View {
        CSSheetFrame(title: model.title, sub: model.sub) {
            VStack(alignment: .leading, spacing: 10) {
                Text(model.source).font(.callout)
                    .accessibilityIdentifier("export-source")
                if model.formats.count > 1 {
                    Picker("Format", selection: $model.format) {
                        ForEach(model.formats) { Text($0.label).tag($0.value) }
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("export-format")
                }
                ForEach(model.options) { o in
                    Toggle(o.label, isOn: Binding(
                        get: { model.values[o.key] ?? o.defaultValue },
                        set: { model.values[o.key] = $0 }
                    ))
                    .accessibilityIdentifier("export-option-\(o.key)")
                }
                Text(model.note).font(.footnote).foregroundStyle(theme["muted"])
                    .fixedSize(horizontal: false, vertical: true)
                if let e = model.error {
                    CSBanner(.err, e).accessibilityIdentifier("export-error")
                }
            }
        } footer: {
            Button("Cancel", action: onClose)
                .keyboardShortcut(.cancelAction)
                .disabled(model.busy)
                .accessibilityIdentifier("export-cancel")
            Button(model.busy ? "Exporting\u{2026}" : "Export\u{2026}") {
                guard let path = model.choosePath() else { return }
                Task { await model.export(to: path) }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(model.busy)
            .accessibilityIdentifier("export-run")
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("export-sheet")
    }

    private func resultView(_ r: ExportResult) -> some View {
        CSSheetFrame(title: "Export complete", width: r.format == "pdf" ? 680 : 520) {
            VStack(alignment: .leading, spacing: 10) {
                CSBanner(.ok, "\(r.fileName) was saved.").accessibilityIdentifier("export-saved")
                Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 4) {
                    row("Document", r.document)
                    row("Source", r.sourceLabel)
                    row("Scope", r.scopeLabel)
                    if !r.contentsLabel.isEmpty { row("Contents", r.contentsLabel) }
                    row("Format", model.formats.first { $0.value == r.format }?.label ?? r.format.uppercased())
                    if let p = r.pages { row("Pages", String(Int(p))) }
                    row("Location", r.path)
                }
                .font(.callout)
                ForEach(r.warnings, id: \.self) { w in CSBanner(.warn, w) }
                if r.format == "pdf" {
                    CSPDFPreview(path: r.path)
                        .frame(height: 340)
                        .overlay(RoundedRectangle(cornerRadius: 6).stroke(theme["line"]))
                        .accessibilityLabel("PDF preview")
                        .accessibilityIdentifier("export-pdf-preview")
                }
                Text("Private notes excluded. Export creates a snapshot; your project does not change.")
                    .font(.footnote).foregroundStyle(theme["muted"])
            }
        } footer: {
            Button("Reveal in Finder") {
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: r.path)])
            }
            .accessibilityIdentifier("export-reveal")
            Button("Close", action: onClose)
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("export-close")
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("export-result")
    }

    private func row(_ k: String, _ v: String) -> some View {
        GridRow {
            Text(k).foregroundStyle(theme["muted"])
            Text(v).textSelection(.enabled).lineLimit(2)
        }
    }
}

/// A PDF shown with PDFKit.
struct CSPDFPreview: NSViewRepresentable {
    let path: String

    func makeNSView(context: Context) -> PDFView {
        let v = PDFView()
        v.autoScales = true
        v.displayMode = .singlePageContinuous
        v.document = PDFDocument(url: URL(fileURLWithPath: path))
        return v
    }

    func updateNSView(_ v: PDFView, context: Context) {
        if v.document?.documentURL?.path != path { v.document = PDFDocument(url: URL(fileURLWithPath: path)) }
    }
}

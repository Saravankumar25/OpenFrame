// Print Preview (Windows: PrintPreview). On the Mac the preview is the real thing: the core writes
// the deterministic screenplay PDF, PDFKit shows that file, and Print hands the same PDF to the
// system print panel. The page count of the preview is the page count of the PDF, so what you
// see, print and export are one document (FSD 123).

import AppKit
import Foundation
import Observation
import PDFKit
import OpenFrameCore

/// Zoom stops of the preview. `nil` fits the page to the window.
enum IXZoom {
    static let stops: [Double?] = [nil, 0.5, 0.75, 1.0, 1.25, 1.5]
    static func label(_ z: Double?) -> String { z.map { "\(Int(($0 * 100).rounded()))%" } ?? "Fit" }
}

@MainActor
@Observable
final class PrintPreviewModel {
    private(set) var loading = false
    private(set) var document: PDFDocument?
    private(set) var pdfURL: URL?
    private(set) var layout: InterchangePrintPreview?
    private(set) var warnings: [InterchangeWarning] = []
    private(set) var error: String?
    private(set) var exportError: String?
    private(set) var exporting = false
    private(set) var exported: InterchangeScreenplayExportResult?
    private(set) var sourceLabel = ""
    var pageIndex = 0
    var zoomIndex = 0

    // Settings the preview and the exported PDF share.
    var titlePage: Bool
    var sceneNumbers: Bool
    var revisionMarks: Bool
    var revisionInfo: Bool
    var includeNotes: Bool
    var sceneIds: [String]?

    @ObservationIgnored let app: AppModel
    @ObservationIgnored let draftId: String
    @ObservationIgnored let screenplayTitle: String
    @ObservationIgnored let draftName: String
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private let folder: URL

    init(app: AppModel, draftId: String, screenplayTitle: String = "", draftName: String = "", options: InterchangeScreenplayExportOptions? = nil) {
        self.app = app
        self.draftId = draftId
        self.screenplayTitle = screenplayTitle
        self.draftName = draftName
        titlePage = options?.titlePage ?? true
        sceneNumbers = options?.sceneNumbers ?? true
        revisionMarks = options?.revisionMarks ?? false
        revisionInfo = options?.revisionInfo ?? true
        includeNotes = options?.includeNotes ?? false
        sceneIds = options?.sceneIds
        folder = FileManager.default.temporaryDirectory.appendingPathComponent("OpenFrame Print Preview", isDirectory: true)
    }

    var options: InterchangeScreenplayExportOptions {
        InterchangeScreenplayExportOptions(
            titlePage: titlePage, sceneNumbers: sceneNumbers, includeNotes: includeNotes,
            revisionMarks: revisionMarks, revisionInfo: revisionInfo, sceneIds: sceneIds)
    }

    /// Changes whenever a setting that changes the document changes.
    var settingsKey: String { "\(titlePage)-\(sceneNumbers)-\(revisionMarks)-\(revisionInfo)-\(includeNotes)" }

    var suggestedName: String {
        IXNames.defaultExportFileName(screenplayTitle: screenplayTitle.isEmpty ? sourceLabel : screenplayTitle, draftName: draftName, format: .pdf)
    }

    var pageCount: Int { document?.pageCount ?? 0 }
    var currentPage: Int { min(max(0, pageIndex), max(0, pageCount - 1)) }
    var zoom: Double? { IXZoom.stops[zoomIndex] }
    var zoomLabel: String { IXZoom.label(zoom) }
    var canZoomOut: Bool { zoomIndex > 0 }
    var canZoomIn: Bool { zoomIndex < IXZoom.stops.count - 1 }
    /// The PDF font cannot print some characters in this draft: PDF export is not offered.
    var blocking: Bool { warnings.contains { $0.code == "pdf_unsupported_characters" } }

    /// "Title page" or "Page 12", from the same layout the PDF was drawn from.
    func pageTitle(_ index: Int) -> String {
        guard let layout, layout.pages.indices.contains(index) else { return "Page \(index + 1)" }
        let p = layout.pages[index]
        return p.titlePage ? "Title page" : "Page \(p.number.map { Int($0) } ?? index + 1)"
    }
    var pageLabel: String { pageCount == 0 ? "" : "\(pageTitle(currentPage)) \u{00B7} \(currentPage + 1) of \(pageCount)" }

    // MARK: Loading

    func load() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        loading = true
        error = nil
        await ScreenplayWorkspaceModel.flushAll()
        do {
            let layout = try await core.interchangeExportPreview(
                InterchangeExportPreviewArgs(draftId: draftId, options: options, pageNumbers: true))
            guard mine == generation else { return }
            self.layout = layout
            warnings = layout.warnings
            sourceLabel = layout.sourceLabel
            if layout.warnings.contains(where: { $0.code == "pdf_unsupported_characters" }) {
                document = nil
                removeTemp()
                loading = false
                return
            }
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            let url = folder.appendingPathComponent("\(UUID().uuidString).pdf")
            let result = try await core.screenplayExport(
                InterchangeScreenplayExportArgs(draftId: draftId, format: "pdf", path: url.path, options: options))
            guard mine == generation else { try? FileManager.default.removeItem(at: url); return }
            guard let data = try? Data(contentsOf: url), let doc = PDFDocument(data: data) else {
                try? FileManager.default.removeItem(at: url)
                error = "The preview could not be prepared. The PDF could not be read."
                loading = false
                return
            }
            _ = result
            removeTemp()
            pdfURL = url
            document = doc
            pageIndex = min(pageIndex, max(0, doc.pageCount - 1))
        } catch {
            guard mine == generation else { return }
            document = nil
            self.error = "The preview could not be prepared. \(CoreFailure(asCore: error).message)"
        }
        loading = false
    }

    func next() { if currentPage < pageCount - 1 { pageIndex = currentPage + 1 } }
    func previous() { if currentPage > 0 { pageIndex = currentPage - 1 } }
    func zoomIn() { if canZoomIn { zoomIndex += 1 } }
    func zoomOut() { if canZoomOut { zoomIndex -= 1 } }

    // MARK: Export and print

    /// Write the same PDF to the place the person chose. The core makes it again from the same draft and
    /// settings, so it is the file that was previewed.
    @discardableResult
    func exportPDF(to url: URL) async -> Bool {
        guard let core = app.core, !blocking else { return false }
        exportError = nil
        exporting = true
        defer { exporting = false }
        let scoped = ScopedAccess()
        scoped.hold(url.deletingLastPathComponent())
        scoped.hold(url)
        defer { scoped.release() }
        do {
            let r = try await core.screenplayExport(
                InterchangeScreenplayExportArgs(draftId: draftId, format: "pdf", path: url.path, options: options))
            exported = r
            ToastCenter.shared.show("Exported \(r.fileName)")
            return true
        } catch {
            exportError = CoreFailure(asCore: error).message
            return false
        }
    }

    /// The system print job for the previewed PDF: same pages, paper size of the PDF, no scaling.
    func printOperation() -> NSPrintOperation? {
        guard let document, document.pageCount > 0, let first = document.page(at: 0) else { return nil }
        let info = (NSPrintInfo.shared.copy() as? NSPrintInfo) ?? NSPrintInfo()
        info.paperSize = first.bounds(for: .mediaBox).size
        info.topMargin = 0; info.bottomMargin = 0; info.leftMargin = 0; info.rightMargin = 0
        info.isHorizontallyCentered = true
        info.isVerticallyCentered = true
        return document.printOperation(for: info, scalingMode: .pageScaleNone, autoRotate: false)
    }

    func print(from window: NSWindow?) {
        guard let op = printOperation() else { return }
        op.jobTitle = sourceLabel.isEmpty ? "Screenplay" : sourceLabel
        op.showsPrintPanel = true
        op.showsProgressPanel = true
        if let window {
            op.runModal(for: window, delegate: nil, didRun: nil, contextInfo: nil)
        } else {
            op.run()
        }
    }

    // MARK: Clean up

    private func removeTemp() {
        if let pdfURL { try? FileManager.default.removeItem(at: pdfURL) }
        pdfURL = nil
    }

    func cleanup() {
        generation += 1
        removeTemp()
        try? FileManager.default.removeItem(at: folder)
    }
}

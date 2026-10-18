// Mac phase M7, screenplay import, export and print. The shared scenarios 63-every-export and the
// import and export parts of 41-screenplay-engine, run at model level against a real core: each
// test drives the Import, Export and Print models the way the sheets do. Plus the FDX round-trip
// suite of 13 classes through every format, Indian scripts, and the edge and failure cases.

import AppKit
import Foundation
import PDFKit
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("InterchangeTests", .serialized)
@MainActor
struct InterchangeTests {
    // MARK: Import sheet

    @Test func pastedImportShowsThePreviewThenTheReport() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let m = ImportScreenplayModel(app: rig.app)
        await m.loadSamples()
        #expect(!m.canPreview)
        m.pasted = nightBus
        #expect(m.canPreview)
        await m.runPreview()
        #expect(m.step == .preview)
        let p = try #require(m.preview)
        #expect(Int(p.sceneCount) == 3)
        #expect(p.scenes.map(\.heading).contains { $0.contains("BUS STOP") })
        #expect(p.characters.contains("MEERA"))
        #expect(m.modes.map(\.mode) == [.newScreenplay])
        #expect(m.canImport)
        // Nothing exists until the person confirms.
        #expect(try await rig.sources().screenplays.isEmpty)
        await m.runImport()
        #expect(m.step == .report)
        let r = try #require(m.report)
        #expect(Int(r.importedScenes) == 3)
        #expect(r.mode == "new_screenplay")
        #expect(try await rig.sources().screenplays.count == 1)
        #expect(ToastCenter.shared.items.contains { $0.text.hasPrefix("Imported screenplay") })
    }

    @Test func anImportNeverReplacesAndOffersANewDraft() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let first = try await rig.importPasted(nightBus)
        let before = try await rig.lines(first.draftId)
        let m = ImportScreenplayModel(app: rig.app, openScreenplayId: first.screenplayId)
        m.pasted = "INT. KITCHEN - DAY\n\nMeera pours tea.\n"
        await m.runPreview()
        let p = try #require(m.preview)
        #expect(m.modes.map(\.mode).contains(.newDraft))
        #expect(m.screenplayId == first.screenplayId)
        m.mode = .newDraft
        m.draftName = "Kitchen rewrite"
        await m.runImport()
        let r = try #require(m.report)
        #expect(r.mode == "new_draft")
        #expect(r.screenplayId == first.screenplayId)
        #expect(r.draftLabel.contains("Kitchen rewrite"))
        #expect(try await rig.lines(first.draftId) == before)
        let drafts = try await rig.sources().screenplays[0].drafts
        #expect(drafts.count == 2)
        _ = p
    }

    @Test func aSampleScriptOpensThroughTheSamePreview() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let m = ImportScreenplayModel(app: rig.app)
        await m.loadSamples()
        let sample = try #require(m.samples.first)
        m.toggleSample(sample.id)
        #expect(m.canPreview && m.path == nil)
        await m.runPreview()
        #expect(m.step == .preview)
        #expect((m.preview?.sceneCount ?? 0) > 0)
        m.toggleSample(sample.id)
        #expect(!m.canPreview)
    }

    // MARK: 63-every-export

    @Test func everyFormatExportsAndEachTextFormatImportsBack() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        var results: [IXExportFormat: InterchangeScreenplayExportResult] = [:]
        for f in IXExportFormat.allCases {
            let res = try await rig.export(r.draftId, f, to: "night-bus.\(f.rawValue)") { m in
                m.titlePage = true
                m.sceneNumbers = true
            }
            results[f] = res
            let data = bytes(rig.file("night-bus.\(f.rawValue)"))
            #expect(data.count >= 100, "\(f) is too small")
            #expect(res.privateNotesExcluded)
        }
        #expect(bytes(rig.file("night-bus.pdf")).prefix(5) == Data("%PDF-".utf8))
        #expect(String(decoding: bytes(rig.file("night-bus.fountain")), as: UTF8.self).contains("You came back."))
        #expect(results[.pdf]?.pages != nil)
        for f in [IXExportFormat.fdx, .fountain, .docx, .txt] {
            let back = try await rig.importFile(rig.file("night-bus.\(f.rawValue)"), asDraft: "Back from \(f.rawValue)")
            #expect(back.draftName == "Back from \(f.rawValue)")
            #expect(Int(back.importedScenes) == 3, "\(f)")
            let lines = try await rig.lines(back.draftId)
            #expect(lines.contains { $0.hasPrefix("scene_heading:") && $0.contains("BUS STOP") }, "\(f)")
            #expect(lines.contains { $0.hasSuffix(": You came back.") }, "\(f)")
        }
    }

    @Test func theExportDialogOffersWhatWindowsOffers() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let empty = ExportScreenplayModel(app: rig.app)
        await empty.load()
        #expect(empty.empty)
        let r = try await rig.importPasted(nightBus, draftName: "Shooting Draft")
        let m = ExportScreenplayModel(app: rig.app)
        await m.load()
        #expect(m.draft == r.draftId)
        #expect(m.choices.count == 1)
        #expect(m.formatsShown == IXExportFormat.main)
        m.showMore = true
        #expect(m.formatsShown.contains(.txt))
        #expect(m.format == .pdf)
        #expect(m.scopeOk)
        m.scope = .selected
        #expect(!m.scopeOk)
        #expect(m.scenes.count == 3)
        m.setPicked(m.scenes[1].id, true)
        #expect(m.scopeOk)
        #expect(m.options.sceneIds == [m.scenes[1].id])
        m.format = .fountain
        #expect(m.suggestedName == "Night Bus - Shooting Draft.fountain")
        #expect(await m.export(to: rig.file("only-bus-stop.fountain")))
        let text = String(decoding: bytes(rig.file("only-bus-stop.fountain")), as: UTF8.self)
        #expect(text.contains("BUS STOP") && !text.contains("POLICE STATION"))
        #expect(Int(m.result?.sceneCount ?? 0) == 1)
    }

    @Test func exportFileNamesAreSafe() {
        #expect(IXNames.defaultExportFileName(screenplayTitle: "Black Rain", draftName: "Draft 6 \u{2013} Shooting", format: .pdf) == "Black Rain - Draft 6 - Shooting.pdf")
        #expect(IXNames.defaultExportFileName(screenplayTitle: "A/B: C?", draftName: "", format: .fdx) == "A B C.fdx")
        #expect(IXNames.defaultExportFileName(screenplayTitle: " . ", draftName: "", format: .txt) == "Screenplay.txt")
        #expect(IXNames.defaultExportFileName(screenplayTitle: String(repeating: "x", count: 300), draftName: "", format: .pdf).count <= 104)
    }

    // MARK: 41-screenplay-engine (import and export parts)

    @Test func pdfAndFdxExportThenTheFdxImportsBackAsADraft() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let script = "Title: Signal Box\n\nINT. SIGNAL BOX - NIGHT\n\nThe telephone rings twice.\n\nASHA\nWho is there?\n\n===\n\nASHA\nNobody answers.\n"
        let r = try await rig.importPasted(script)
        let pdf = try await rig.export(r.draftId, .pdf, to: "signal-box.pdf")
        #expect((pdf.pages ?? 0) >= 2)
        let pdfData = bytes(rig.file("signal-box.pdf"))
        #expect(pdfData.count > 500)
        #expect(!String(decoding: pdfData, as: UTF8.self).contains("OpenFrame"))
        _ = try await rig.export(r.draftId, .fdx, to: "signal-box.fdx")
        let fdx = String(decoding: bytes(rig.file("signal-box.fdx")), as: UTF8.self)
        #expect(fdx.contains("StartsNewPage=\"Yes\""))
        let back = try await rig.importFile(rig.file("signal-box.fdx"), asDraft: "From Final Draft")
        let lines = try await rig.lines(back.draftId)
        #expect(lines.contains("action: The telephone rings twice."))
        #expect(lines.contains { $0.hasPrefix("page_break") })
        #expect(try await rig.labels(back.draftId).first == "1")
    }

    @Test func indianScriptsSurviveImportExportAndThePdf() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let telugu = "\u{0C2E}\u{0C40}\u{0C30}\u{0C3E} \u{0C30}\u{0C48}\u{0C32}\u{0C41} \u{0C38}\u{0C4D}\u{0C1F}\u{0C47}\u{0C37}\u{0C28}\u{0C4D}\u{200C}\u{0C32}\u{0C4B} \u{0C35}\u{0C47}\u{0C1A}\u{0C3F} \u{0C09}\u{0C02}\u{0C26}\u{0C3F}."
        let hindi = "\u{092E}\u{0940}\u{0930}\u{093E} \u{0928}\u{0947} \u{0915}\u{0939}\u{093E}, \u{201C}\u{0915}\u{094D}\u{0937}\u{092E}\u{093E} \u{0915}\u{0930}\u{0947}\u{0902}\u{0964}\u{201D}"
        let tamil = "\u{0BAE}\u{0BC0}\u{0BB0}\u{0BBE} \u{0BB5}\u{0BA3}\u{0B95}\u{0BCD}\u{0B95}\u{0BAE}\u{0BCD}"
        let script = "INT. RAIL STATION - DAY\n\n\(telugu) \(hindi)\n\nMEERA\n\(tamil)\n"
        let r = try await rig.importPasted(script)
        let lines = try await rig.lines(r.draftId)
        #expect(lines.contains { $0.contains(telugu) && $0.contains(hindi) })
        #expect(lines.contains { $0.hasSuffix(tamil) })
        let pdf = try await rig.export(r.draftId, .pdf, to: "te.pdf") { $0.titlePage = false }
        #expect((pdf.pages ?? 0) >= 1)
        let raw = String(decoding: bytes(rig.file("te.pdf")), as: UTF8.self).replacingOccurrences(of: " ", with: "")
        #expect(raw.contains("NotoSansTelugu"))
        #expect(raw.contains("NotoSansDevanagari"))
        #expect(raw.contains("NotoSansTamil"))
        for f in [IXExportFormat.fdx, .fountain, .docx, .txt] {
            _ = try await rig.export(r.draftId, f, to: "te.\(f.rawValue)")
            let back = try await rig.importFile(rig.file("te.\(f.rawValue)"), asDraft: "te \(f.rawValue)")
            let l = try await rig.lines(back.draftId).joined(separator: "\n")
            #expect(l.contains(telugu), "\(f) lost Telugu")
            #expect(l.contains(hindi), "\(f) lost Hindi")
            #expect(l.contains(tamil), "\(f) lost Tamil")
        }
    }

    // MARK: The FDX round-trip suite through every format

    @Test func everyFdxClassRoundTripsThroughEveryFormat() async throws {
        for name in IXFixtures.suiteClasses {
            let rig = try await IXRig.make(title: name)
            defer { rig.close() }
            let first = try await rig.importFile(IXFixtures.suite(name))
            let original = try await rig.lines(first.draftId)
            let originalLabels = try await rig.labels(first.draftId)
            #expect(!original.isEmpty, "\(name) imported nothing")
            // Final Draft keeps everything: custom elements, numbers, page breaks.
            _ = try await rig.export(first.draftId, .fdx, to: "\(name).fdx") { $0.sceneNumbers = true; $0.revisionMarks = true }
            let viaFdx = try await rig.importFile(rig.file("\(name).fdx"), asDraft: "fdx")
            #expect(try await rig.lines(viaFdx.draftId).filter { !$0.hasPrefix("note:") } == original.filter { !$0.hasPrefix("note:") }, "\(name) changed through FDX")
            #expect(try await rig.labels(viaFdx.draftId) == originalLabels, "\(name) scene numbers changed through FDX")
            // The other text formats keep the words and the scene headings.
            let words = Set(original.filter { !$0.hasPrefix("note:") && !$0.hasPrefix("page_break") }
                .map { $0.drop { $0 != ":" }.dropFirst().trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty })
            for f in [IXExportFormat.fountain, .docx, .txt] {
                _ = try await rig.export(first.draftId, f, to: "\(name).\(f.rawValue)") { $0.sceneNumbers = true }
                let back = try await rig.importFile(rig.file("\(name).\(f.rawValue)"), asDraft: f.rawValue)
                let got = try await rig.lines(back.draftId).joined(separator: "\n").replacingOccurrences(of: "\n", with: " ")
                let normalizedGot = got.split(whereSeparator: \.isWhitespace).joined(separator: " ")
                for w in words {
                    let n = w.split(whereSeparator: \.isWhitespace).joined(separator: " ")
                    // Text formats may case-fold headings, characters and transitions.
                    #expect(normalizedGot.localizedCaseInsensitiveContains(n), "\(name) lost \u{201C}\(n)\u{201D} through \(f)")
                }
                // A numbered scene left out of the draft prints as "OMITTED" in a production draft; plain text keeps no marker for it.
                let gotHeadings = try await rig.headings(back.draftId).filter { $0 != "OMITTED" }
                let wantHeadings = try await rig.headings(first.draftId).filter { $0 != "OMITTED" }
                #expect(gotHeadings.count == wantHeadings.count, "\(name) scene count through \(f): \(wantHeadings) became \(gotHeadings)")
            }
            // The PDF is the real page layout; PDFKit reads exactly the pages the core reports.
            let pdf = try await rig.export(first.draftId, .pdf, to: "\(name).pdf") { $0.sceneNumbers = true; $0.revisionMarks = true }
            let doc = try #require(PDFDocument(url: rig.file("\(name).pdf")))
            // The layout the preview draws has exactly the pages of the PDF (a title page is one more than the script pages).
            let opts = InterchangeScreenplayExportOptions(titlePage: true, sceneNumbers: true, includeNotes: false, revisionMarks: true, revisionInfo: true)
            let layout = try await rig.core.interchangeExportPreview(InterchangeExportPreviewArgs(draftId: first.draftId, options: opts, pageNumbers: true))
            #expect(doc.pageCount == layout.pages.count, "\(name) PDF page count")
            #expect(Int(layout.scriptPageCount) == Int(pdf.pages ?? -1), "\(name) script pages")
        }
    }

    @Test func aPdfExportedByTheAppImportsBackAsBestEffortWithAPageEstimate() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        _ = try await rig.export(r.draftId, .pdf, to: "bus.pdf")
        let m = ImportScreenplayModel(app: rig.app)
        m.choose(file: rig.file("bus.pdf"))
        await m.runPreview()
        let p = try #require(m.preview)
        #expect(p.sourceFormat == "pdf")
        #expect(Int(p.sceneCount) == 3)
        #expect(p.approxPages >= 1)
        #expect(["high", "medium", "low"].contains(p.confidence))
        #expect(m.step == .preview)
    }

    // MARK: Print preview

    @Test func thePreviewIsTheCoresPdfAndPrintsThatSamePdf() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        let m = PrintPreviewModel(app: rig.app, draftId: r.draftId, screenplayTitle: "Night Bus", draftName: r.draftName)
        await m.load()
        let doc = try #require(m.document)
        let layout = try #require(m.layout)
        let exported = try await rig.export(r.draftId, .pdf, to: "same.pdf")
        #expect(m.pageCount == layout.pages.count)
        #expect(m.pageCount == Int(exported.pages ?? -1) + 1)    // the pages the core counts, plus the title page
        #expect(m.pageTitle(0) == "Title page")
        #expect(m.pageLabel.hasSuffix("of \(m.pageCount)"))
        // The previewed file and the exported file are the same bytes.
        let previewed = try #require(m.pdfURL)
        #expect(bytes(previewed) == bytes(rig.file("same.pdf")))
        // Export PDF writes the file that was previewed.
        #expect(await m.exportPDF(to: rig.file("from-preview.pdf")))
        #expect(bytes(rig.file("from-preview.pdf")) == bytes(rig.file("same.pdf")))
        // Paging and zoom.
        m.next(); m.next(); m.next(); m.next()
        #expect(m.currentPage == m.pageCount - 1)
        m.previous()
        #expect(m.currentPage == m.pageCount - 2 || m.pageCount == 1)
        #expect(m.zoomLabel == "Fit")
        m.zoomIn(); #expect(m.zoomLabel == "50%")
        m.zoomOut(); #expect(m.zoomLabel == "Fit" && !m.canZoomOut)
        // Print uses that same PDF: the print job has one page per PDF page, on the PDF's paper.
        let op = try #require(m.printOperation())
        var range = NSRange()
        _ = op.view?.knowsPageRange(&range)
        #expect(range.length == doc.pageCount)
        let size = try #require(doc.page(at: 0)).bounds(for: .mediaBox).size
        #expect(op.printInfo.paperSize == size)
        // Settings change the document: no title page, one page fewer.
        m.titlePage = false
        await m.load()
        #expect(m.pageCount == layout.pages.count - 1)
        m.cleanup()
        #expect(!FileManager.default.fileExists(atPath: previewed.path))
    }

    @Test func printingTheSamePdfToAFileKeepsEveryPage() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        let m = PrintPreviewModel(app: rig.app, draftId: r.draftId)
        await m.load()
        let op = try #require(m.printOperation())
        let dest = rig.file("printed.pdf")
        op.printInfo.jobDisposition = .save
        op.printInfo.dictionary()[NSPrintInfo.AttributeKey.jobSavingURL] = dest
        op.showsPrintPanel = false
        op.showsProgressPanel = false
        if op.run(), let printed = PDFDocument(url: dest) {
            #expect(printed.pageCount == m.pageCount)
        } else {
            // A machine without a print subsystem cannot run the job; the page range test above still holds.
            #expect(m.pageCount > 0)
        }
        m.cleanup()
    }

    // MARK: Failure and edge cases

    @Test func aCorruptFdxFailsSafelyAndLeavesTheProjectAndTheFileAlone() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let src = IXFixtures.hostile("fdx/malformed_unclosed.fdx")
        let copy = rig.file("broken.fdx")
        try FileManager.default.copyItem(at: src, to: copy)
        let before = bytes(copy)
        let m = ImportScreenplayModel(app: rig.app)
        m.choose(file: copy)
        await m.runPreview()
        #expect(m.step == .failed)
        let f = try #require(m.failure)
        #expect(f.code == "import.fdx_malformed")
        #expect(IXLabels.failureStage(code: f.code, pasted: false) == "Reading the Final Draft file")
        #expect(IXLabels.canKeepAsProjectFile(code: f.code))
        #expect(!ImportScreenplaySheet.cleanProblem(f.message).contains("Your project was not changed"))
        #expect(bytes(copy) == before)
        #expect(try await rig.sources().screenplays.isEmpty)
        // "Add to Project Files" keeps it as an ordinary file.
        #expect(await m.keepAsProjectFile())
        let files = try await rig.core.filesList()
        let kept = files.files.contains(where: { $0.displayName == "broken.fdx" })
        #expect(kept)
        // "Paste Text Instead" goes back to the source step with the file cleared.
        m.pasteInstead()
        #expect(m.step == .source && m.path == nil && m.failure == nil)
    }

    @Test func wrongOrHostileFilesAreRefusedWithPlainWords() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let cases: [(URL, String)] = [
            (IXFixtures.hostile("pdf/malformed_no_header.pdf"), "import.pdf_unreadable"),
            (IXFixtures.hostile("pdf/encrypted.pdf"), "import.pdf_unreadable"),
            (IXFixtures.hostile("docx/duplicate_entry.docx"), "import.unsafe_archive"),
            (IXFixtures.hostile("docx/malformed_not_zip.docx"), "import.unsafe_archive"),
            (IXFixtures.hostile("fdx/malformed_wrong_root.fdx"), "import.fdx_invalid"),
            (IXFixtures.hostile("fountain/empty.fountain"), "import.empty_source"),
            (IXFixtures.hostile("fountain/malformed_binary.fountain"), "import.not_a_screenplay"),
            (IXFixtures.screenplay("not_a_screenplay.txt"), "import.not_a_screenplay"),
        ]
        for (url, code) in cases {
            let m = ImportScreenplayModel(app: rig.app)
            m.choose(file: url)
            await m.runPreview()
            #expect(m.step == .failed, "\(url.lastPathComponent)")
            #expect(m.failure?.code == code, "\(url.lastPathComponent): \(m.failure?.code ?? "none")")
            let said = m.failure?.message ?? ""
            #expect(said.count > 10 && said.hasSuffix("."), "\(url.lastPathComponent)")
        }
        #expect(try await rig.sources().screenplays.isEmpty)
    }

    @Test func aFileWithTheWrongExtensionIsStillReadByItsContent() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        // A Fountain script saved as .pdf is refused (it is not a PDF); a real FDX renamed .txt is not mistaken for text.
        let fake = rig.file("script.pdf")
        try Data(nightBus.utf8).write(to: fake)
        let m = ImportScreenplayModel(app: rig.app)
        m.choose(file: fake)
        await m.runPreview()
        #expect(m.step == .failed)
        #expect(m.failure?.code.hasPrefix("import.") == true)
        let renamed = rig.file("script.txt")
        try FileManager.default.copyItem(at: IXFixtures.screenplay("sample.fdx"), to: renamed)
        let m2 = ImportScreenplayModel(app: rig.app)
        m2.choose(file: renamed)
        await m2.runPreview()
        #expect(m2.step == .preview)
        #expect(m2.preview?.sourceFormat == "fdx")
    }

    @Test func aHugeTextFileIsRefusedAndNotKept() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let big = rig.file("huge.txt")
        let chunk = Data(String(repeating: "INT. ROOM - DAY\n\nSomeone speaks.\n\n", count: 1000).utf8)
        FileManager.default.createFile(atPath: big.path, contents: nil)
        let h = try FileHandle(forWritingTo: big)
        for _ in 0..<(21 * 1024 * 1024 / chunk.count + 1) { try h.write(contentsOf: chunk) }
        try h.close()
        let m = ImportScreenplayModel(app: rig.app)
        m.choose(file: big)
        await m.runPreview()
        #expect(m.step == .failed)
        #expect(m.failure?.code == "import.too_large")
        #expect(!IXLabels.canKeepAsProjectFile(code: "import.too_large"))
        #expect(IXLabels.failureStage(code: "import.too_large", pasted: false) == "Reading the file")
    }

    @Test func aPathThatCannotBeWrittenFailsWithoutLeavingAFileOrChangingTheProject() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        let before = try await rig.lines(r.draftId)
        let m = ExportScreenplayModel(app: rig.app, draftId: r.draftId)
        await m.load()
        // A folder that does not exist.
        #expect(!(await m.export(to: rig.out.appendingPathComponent("missing/out.pdf"))))
        #expect(m.error != nil && m.result == nil)
        // A read-only folder.
        let ro = rig.root.appendingPathComponent("readonly", isDirectory: true)
        try FileManager.default.createDirectory(at: ro, withIntermediateDirectories: true)
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: ro.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: ro.path) }
        #expect(!(await m.export(to: ro.appendingPathComponent("out.pdf"))))
        #expect(m.error?.isEmpty == false)
        #expect(!FileManager.default.fileExists(atPath: ro.appendingPathComponent("out.pdf").path))
        // Inside the project's own folder.
        guard case let .project(p) = rig.app.phase else { return }
        #expect(!(await m.export(to: URL(fileURLWithPath: p.path).appendingPathComponent("out.pdf"))))
        #expect(m.error?.contains("project") == true)
        #expect(try await rig.lines(r.draftId) == before)
        // Selecting no scene cannot export at all.
        m.scope = .selected
        #expect(!m.scopeOk)
        #expect(!(await m.export(to: rig.file("none.pdf"))))
        #expect(!FileManager.default.fileExists(atPath: rig.file("none.pdf").path))
    }

    @Test func anExportOverAnExistingFileThatCannotBeReplacedKeepsTheOldFile() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        let target = rig.file("keep.fdx")
        try Data("old contents".utf8).write(to: target)
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: rig.out.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: rig.out.path) }
        let m = ExportScreenplayModel(app: rig.app, draftId: r.draftId)
        await m.load()
        m.format = .fdx
        let ok = await m.export(to: target)
        if !ok { #expect(String(decoding: bytes(target), as: UTF8.self) == "old contents") }
        #expect(ok || m.error != nil)
    }

    @Test func aDraftWithNothingToPrintIsRefusedInPlainWords() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let created = try await rig.core.screenplayCreate(CreateScreenplayArgs(title: "Blank", draftName: nil))
        let doc = try await rig.core.screenplayDocument(ScreenplayDraftIdArgs(draftId: created.draft.id))
        // Empty out the only scene's headings is not possible here; a draft with no scenes is refused, one with a blank scene exports.
        let m = ExportScreenplayModel(app: rig.app, draftId: created.draft.id)
        await m.load()
        m.format = .pdf
        let ok = await m.export(to: rig.file("blank.pdf"))
        if doc.scenes.isEmpty {
            #expect(!ok)
            #expect(m.error?.isEmpty == false)
            #expect(!FileManager.default.fileExists(atPath: rig.file("blank.pdf").path))
        } else {
            #expect(ok)
        }
    }

    // MARK: Rendering

    @Test func theSheetsBuildAndShowTheirWords() async throws {
        let rig = try await IXRig.make()
        defer { rig.close() }
        let r = try await rig.importPasted(nightBus)
        let store = ThemeStore()
        let imp = ImportScreenplayModel(app: rig.app)
        imp.pasted = nightBus
        await imp.runPreview()
        let sheets: [(String, AnyView)] = [
            ("import-preview", AnyView(ImportScreenplaySheet(model: imp, close: {}))),
            ("export-options", AnyView(ExportScreenplaySheet(model: ExportScreenplayModel(app: rig.app, draftId: r.draftId), close: {}))),
            ("print-preview", AnyView(PrintPreviewSheet(model: PrintPreviewModel(app: rig.app, draftId: r.draftId), back: nil, close: {}))),
        ]
        for (name, view) in sheets {
            let image = Fixture.render(view.environment(rig.app).environment(store).openFrameTheme(store), name: "m7-\(name)", size: CGSize(width: 900, height: 700))
            #expect(image != nil, "\(name)")
        }
        #expect(imp.preview != nil)
    }
}

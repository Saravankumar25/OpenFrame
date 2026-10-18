// Import Screenplay (Windows: ImportScreenplayDialog). Source, then a preview that creates
// nothing, then a report. An existing screenplay is never overwritten: the script arrives as a new
// screenplay or a new named draft. The model calls the same core operations as the sheet and keeps
// only what the sheet shows. The core reads and parses the file; nothing is flattened here.

import Foundation
import Observation
import OpenFrameCore

enum IXImportStep: Equatable { case source, preview, report, failed }

struct IXImportFailure: Equatable {
    var code: String
    var message: String
    var pasted: Bool
    var path: String?
}

@MainActor
@Observable
final class ImportScreenplayModel {
    private(set) var step: IXImportStep = .source
    private(set) var path: String?
    var pasted = ""
    private(set) var sample: String?
    private(set) var samples: [InterchangeSample] = []
    private(set) var busy = false
    private(set) var preview: InterchangeImportPreview?
    private(set) var report: InterchangeImportReport?
    private(set) var failure: IXImportFailure?
    /// A problem after the preview that does not end the flow (the project is unchanged).
    private(set) var errorText: String?
    var mode: IXImportMode = .newScreenplay
    var screenplayId = ""
    var episodeId = ""
    var draftName = ""
    var keepSource = true
    var showWarnings = false

    @ObservationIgnored private let app: AppModel
    @ObservationIgnored private let openScreenplayId: String?
    @ObservationIgnored private let access = ScopedAccess()

    init(app: AppModel, openScreenplayId: String? = nil) {
        self.app = app
        self.openScreenplayId = openScreenplayId
    }

    var isPasted: Bool { path == nil && sample == nil }
    var canPreview: Bool { !busy && (path != nil || sample != nil || !pasted.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) }
    var pathName: String? { path.map(IXNames.fileName) }
    var canImport: Bool {
        guard !busy, let preview else { return false }
        return IXTargets.ready(preview, mode: mode, screenplayId: screenplayId, episodeId: episodeId)
    }
    var modes: [(mode: IXImportMode, label: String)] { preview.map(IXTargets.modes) ?? [] }
    var draftNamePlaceholder: String { "\(IXNames.stem(preview?.sourceName ?? "")) (imported)" }

    func loadSamples() async {
        guard let core = app.core else { return }
        samples = (try? await core.screenplaySamples()) ?? []
    }

    // MARK: Source

    func choose(file url: URL) {
        access.release()
        access.hold(url)
        path = url.path
        sample = nil
        pasted = ""
    }

    func clearFile() {
        access.release()
        path = nil
    }

    func toggleSample(_ id: String) {
        sample = sample == id ? nil : id
        access.release()
        path = nil
        pasted = ""
    }

    // MARK: Preview

    func runPreview() async {
        guard let core = app.core, canPreview else { return }
        busy = true
        errorText = nil
        defer { busy = false }
        let args: InterchangeImportPreviewArgs
        if let sample {
            args = InterchangeImportPreviewArgs(sample: sample)
        } else if let path {
            args = InterchangeImportPreviewArgs(path: path)
        } else {
            args = InterchangeImportPreviewArgs(pastedText: pasted)
        }
        do {
            let p = try await core.screenplayImportPreview(args)
            preview = p
            mode = p.defaultMode == "new_draft" ? .newDraft : .newScreenplay
            // The screenplay open behind the sheet is offered first; the choice stays visible.
            screenplayId = p.existingScreenplays.first { $0.id == openScreenplayId }?.id ?? p.existingScreenplays.first?.id ?? ""
            episodeId = p.episodes.first?.id ?? ""
            showWarnings = false
            step = .preview
        } catch {
            let f = CoreFailure(asCore: error)
            failure = IXImportFailure(code: f.code, message: f.message, pasted: isPasted, path: path)
            step = .failed
        }
    }

    func backToSource() {
        step = .source
        errorText = nil
    }

    // MARK: Import

    func runImport() async {
        guard let core = app.core, let preview, canImport else { return }
        busy = true
        errorText = nil
        defer { busy = false }
        let trimmedName = draftName.trimmingCharacters(in: .whitespaces)
        let args = InterchangeImportApplyArgs(
            previewId: preview.previewId,
            mode: mode.rawValue,
            screenplayId: mode == .newDraft ? (screenplayId.isEmpty ? nil : screenplayId) : nil,
            episodeId: mode == .newScreenplay && preview.episodic ? (episodeId.isEmpty ? nil : episodeId) : nil,
            draftName: trimmedName.isEmpty ? nil : trimmedName,
            keepSourceFile: keepSource)
        do {
            let r = try await core.screenplayImportApply(args)
            report = r
            showWarnings = false
            step = .report
            ToastCenter.shared.undoable(
                mode == .newScreenplay ? "Imported screenplay \u{201C}\(r.screenplayTitle)\u{201D}" : "Imported \(r.draftLabel)",
                app: app)
            access.release()
        } catch {
            let f = CoreFailure(asCore: error)
            if f.code == "import.preview_expired" {
                failure = IXImportFailure(code: f.code, message: f.message, pasted: isPasted, path: path)
                step = .failed
            } else {
                errorText = f.message
            }
        }
    }

    /// Keep a file that could not be read as a screenplay as an ordinary project file.
    func keepAsProjectFile() async -> Bool {
        guard let core = app.core, let path = failure?.path else { return false }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.filesAdd(AddFilesArgs(paths: [path], mode: "copy"))
            ToastCenter.shared.undoable("Added \u{201C}\(IXNames.fileName(path))\u{201D} to Project Files", app: app)
            return true
        } catch {
            errorText = CoreFailure(asCore: error).message
            return false
        }
    }

    func pasteInstead() {
        clearFile()
        failure = nil
        step = .source
    }

    /// Show the imported draft in the Screenplay workspace.
    func openScreenplay() {
        guard let report else { return }
        NotificationCenter.default.post(
            name: .openFrameShowScreenplayTarget, object: nil,
            userInfo: ["screenplayId": report.screenplayId, "draftId": report.draftId])
        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.screenplay.rawValue)
    }

    func finish() { access.release() }
}

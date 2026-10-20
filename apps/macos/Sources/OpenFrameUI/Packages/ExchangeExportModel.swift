// Export an exchange or review package (Windows: ExportExchangeDialog). The sheet says exactly what
// the package will contain before it is written; private notes are never allowed in it.

import Foundation
import Observation
import OpenFrameCore

enum PKScope: String, Hashable { case full, comments, scenes, act }

struct PKExchangeTitle {
    let title: String
    let sub: String

    static func of(_ t: PackageType) -> PKExchangeTitle {
        switch t {
        case .scriptReview: PKExchangeTitle(title: "Script Review Package", sub: "How do I send this draft to someone for review and safely bring their notes back?")
        case .story: PKExchangeTitle(title: "Story Board Package", sub: "Which part of the Story Board do I share, and what comes with it?")
        default: PKExchangeTitle(title: PKInfo.entry(t).label, sub: "")
        }
    }
}

@MainActor
@Observable
final class ExchangeExportModel {
    let packageType: PackageType
    private(set) var sources: PackageExchangeSources?
    private(set) var loadError: String?
    private(set) var scenes: [PackageSceneOption] = []
    private(set) var scenesLoading = false
    private(set) var summary: PackageExportSummary?
    private(set) var previewError: String?
    private(set) var busy = false
    private(set) var exportError: String?
    private(set) var done: PackageExportResult?

    var draftId = ""
    var scope: PKScope = .full
    var sceneIds: [String] = []
    var actId = ""
    var includeComments = true
    var includeAttachments = false

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let access = ScopedAccess()
    @ObservationIgnored private var previewGeneration = 0

    init(app: AppModel, packageType: PackageType) {
        self.app = app
        self.packageType = packageType
    }

    var heading: PKExchangeTitle { PKExchangeTitle.of(packageType) }

    var scopeChoices: [(value: PKScope, label: String)] {
        switch packageType {
        case .scriptReview: [(.full, "Full script"), (.comments, "Script + comments"), (.scenes, "Specific scenes only")]
        case .story: [(.full, "Whole Story Board"), (.act, "One Act")]
        default: []
        }
    }

    var ready: Bool {
        if scope == .scenes && sceneIds.isEmpty { return false }
        if packageType == .scriptReview && draftId.isEmpty && !(sources?.drafts.isEmpty ?? true) { return false }
        return true
    }

    var args: PackageExchangeArgs {
        PackageExchangeArgs(
            packageType: packageType,
            draftId: packageType == .scriptReview ? draftId.ixNonEmpty : nil,
            scope: scopeChoices.isEmpty ? nil : scope.rawValue,
            sceneIds: scope == .scenes ? sceneIds : [],
            actId: scope == .act ? actId.ixNonEmpty : nil,
            callSheetId: nil,
            includeComments: includeComments || scope == .comments,
            includeAttachments: includeAttachments,
            includePrivateNotes: false,
            path: nil)
    }

    /// Changes whenever the package that would be written changes.
    var argsKey: String {
        "\(draftId)|\(scope.rawValue)|\(sceneIds.joined(separator: ","))|\(actId)|\(includeComments)|\(includeAttachments)"
    }

    func load() async {
        guard let core = app.core else { return }
        await ScreenplayWorkspaceModel.flushAll()
        do {
            let s = try await core.packagesExchangeSources()
            sources = s
            if draftId.isEmpty, let d = s.drafts.first(where: \.isCurrent) ?? s.drafts.first { draftId = d.id }
            if actId.isEmpty, let a = s.acts.first { actId = a.id }
            await refreshPreview()
        } catch {
            loadError = CoreFailure(asCore: error).message
        }
    }

    func setScope(_ s: PKScope) async {
        scope = s
        if s == .scenes { await loadScenes() }
        await refreshPreview()
    }

    func loadScenes() async {
        guard let core = app.core else { return }
        scenesLoading = true
        defer { scenesLoading = false }
        let draft: String? = packageType == .scriptReview ? draftId.ixNonEmpty : packageType == .breakdown ? sources?.productionSource?.id : nil
        scenes = (try? await core.packagesExchangeScenes(PackageScenesArgs(draftId: draft))) ?? []
    }

    func toggleScene(_ id: String, _ on: Bool) {
        if on { if !sceneIds.contains(id) { sceneIds.append(id) } } else { sceneIds.removeAll { $0 == id } }
    }

    func refreshPreview() async {
        guard let core = app.core, sources != nil else { return }
        previewGeneration += 1
        let mine = previewGeneration
        guard ready else { summary = nil; previewError = nil; return }
        do {
            let s = try await core.packagesExchangePreview(args)
            guard mine == previewGeneration else { return }
            summary = s
            previewError = nil
        } catch {
            guard mine == previewGeneration else { return }
            summary = nil
            previewError = CoreFailure(asCore: error).message
        }
    }

    func export(to url: URL) async {
        guard let core = app.core, summary != nil else { return }
        exportError = nil
        busy = true
        defer { busy = false }
        access.hold(url.deletingLastPathComponent())
        access.hold(url)
        defer { access.release() }
        var a = args
        a.path = url.path
        do {
            let r = try await core.packagesExportExchange(a)
            done = r
            ToastCenter.shared.show("Exported \(r.fileName)")
        } catch {
            exportError = CoreFailure(asCore: error).message
        }
    }
}

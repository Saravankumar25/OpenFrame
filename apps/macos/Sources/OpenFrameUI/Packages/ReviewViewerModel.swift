// The review package viewer (Windows: ReviewViewer, mockup 160). The screenplay snapshot is
// read-only. A reviewer's comments live in a local review workspace, separate from any project,
// and go back to the author as a Response package. The same viewer shows a received package or a
// kept review record read-only.

import Foundation
import Observation
import OpenFrameCore

enum PKViewerSource: Hashable { case review, session, record }

@MainActor
@Observable
final class ReviewViewerModel {
    let source: PKViewerSource
    private(set) var key: String
    private(set) var workspace: PackageReviewWorkspace?
    var sceneId: String?
    var body = ""
    private(set) var quote = ""
    private(set) var busy = false
    private(set) var errorText: String?
    private(set) var exportedPath: String?

    @ObservationIgnored let app: AppModel
    @ObservationIgnored private let access = ScopedAccess()

    init(app: AppModel, source: PKViewerSource, key: String) {
        self.app = app
        self.source = source
        self.key = key
    }

    var scene: PackageReviewScene? {
        guard let ws = workspace else { return nil }
        return ws.scenes.first { $0.id == sceneId } ?? ws.scenes.first
    }
    var comments: [PackageReviewComment] { workspace?.comments.filter { $0.sceneId == scene?.id } ?? [] }
    var myCount: Int { workspace?.comments.filter(\.mine).count ?? 0 }
    var canComment: Bool { workspace?.canComment ?? false }
    func count(for sceneId: String) -> Int { workspace?.comments.filter { $0.sceneId == sceneId }.count ?? 0 }
    var canAdd: Bool { !busy && scene != nil && !body.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    var banner: String {
        source == .record ? "Review record, kept separately from your working content." : "Review package. Comments are separate from the original project."
    }

    func load() async {
        guard let core = app.core else { return }
        do {
            switch source {
            case .review: workspace = try await core.packagesReviewGet(PackageReviewArgs(packageId: key))
            case .session: workspace = try await core.packagesSessionView(PackageSessionArgs(sessionId: key))
            case .record: workspace = try await core.packagesRecordView(PackageRecordArgs(recordId: key))
            }
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    /// Open a review package file. The key becomes the package id the core made.
    func open(file url: URL) async {
        guard let core = app.core else { return }
        access.release()
        access.hold(url)
        busy = true
        errorText = nil
        defer { busy = false }
        do {
            let ws = try await core.packagesReviewOpen(PackagePathArgs(path: url.path))
            workspace = ws
            key = ws.key
            sceneId = ws.scenes.first?.id
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    /// The selected text of the scene becomes the quote of the next comment (at most 500 characters).
    @discardableResult
    func quoteSelection(_ text: String) -> Bool {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty else {
            ToastCenter.shared.show("Select some text in the scene first.")
            return false
        }
        quote = String(t.prefix(500))
        return true
    }

    func removeQuote() { quote = "" }

    func addComment() async {
        guard let core = app.core, let ws = workspace, let scene, canAdd else { return }
        busy = true
        errorText = nil
        defer { busy = false }
        do {
            workspace = try await core.packagesReviewComment(PackageReviewCommentArgs(
                packageId: ws.key, sceneId: scene.id, body: body, quotedText: quote.trimmingCharacters(in: .whitespaces).ixNonEmpty))
            body = ""
            quote = ""
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    func deleteComment(_ id: String) async {
        guard let core = app.core, let ws = workspace else { return }
        do { workspace = try await core.packagesReviewDeleteComment(PackageReviewCommentIdArgs(packageId: ws.key, commentId: id)) }
        catch { errorText = CoreFailure(asCore: error).message }
    }

    var suggestedResponseName: String { "\(workspace?.draftName ?? "Screenplay") - review response.ofresponse" }

    func exportResponse(to url: URL) async {
        guard let core = app.core, let ws = workspace else { return }
        errorText = nil
        let scoped = ScopedAccess()
        scoped.hold(url.deletingLastPathComponent())
        scoped.hold(url)
        defer { scoped.release() }
        do {
            let r = try await core.packagesReviewExportResponse(PackageReviewExportArgs(packageId: ws.key, path: url.path))
            exportedPath = r.path
            ToastCenter.shared.show("Response package exported with \(Int(r.commentCount)) comment\(r.commentCount == 1 ? "" : "s").")
        } catch {
            errorText = CoreFailure(asCore: error).message
        }
    }

    func finish() { access.release() }
}

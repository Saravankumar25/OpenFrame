// Comments (Mac phase M4), as CommentsPanel and ThreadCard in panels.tsx on Windows: the threads
// of the open draft, new comments on the caret's text or scene, replies, resolve and reopen,
// delete. Choosing a thread shows its paragraph in the editor. All changes go through the
// comment operations of the core.

import AppKit
import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class CommentsPanelModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?

    var threads: [CommentThread] = []
    var loading = false
    /// The composer: what the comment being written will attach to.
    var pending: ScreenplayCommentTarget?
    var focusedId: String?

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel) {
        self.app = app
        self.workspace = workspace
    }

    private var core: Core? { app?.core }
    var permissions: ScreenplayPermissions? { workspace?.overview?.permissions }
    var canComment: Bool { permissions?.comment == true }

    var openThreads: [CommentThread] {
        threads.filter { $0.comment.status != "Resolved" }.sorted { ($0.comment.sceneNumber ?? 0) < ($1.comment.sceneNumber ?? 0) }
    }
    var resolvedThreads: [CommentThread] {
        threads.filter { $0.comment.status == "Resolved" }.sorted { ($0.comment.sceneNumber ?? 0) < ($1.comment.sceneNumber ?? 0) }
    }
    /// Shown on the toolbar button: "Comments (2)".
    var openCount: Int { openThreads.count }

    func threads(forScene sceneId: String?) -> [CommentThread] {
        guard let sceneId else { return [] }
        return openThreads.filter { $0.comment.sceneId == sceneId }
    }

    func reload() async {
        guard let core, let draftId = workspace?.draft?.id else { threads = []; return }
        do {
            threads = try await core.commentList(CommentListArgs(draftId: draftId, status: "all"))
        } catch {
            app?.show("Could not load the comments", error)
        }
    }

    // MARK: Writing

    /// Post the pending comment. Returns the message to show when the core refuses.
    func post(_ body: String) async -> String? {
        guard let core, let target = pending, let draftId = workspace?.draft?.id else { return nil }
        // The text the comment points at must be saved first, so the core knows its ids.
        await workspace?.flush()
        let args: CommentCreateArgs
        switch target {
        case let .element(id, start, end, _):
            args = CommentCreateArgs(targetType: "screenplay_element", targetId: id, body: body,
                                     anchor: CommentAnchorInput(start: Double(start), end: Double(end)))
        case let .scene(id):
            args = CommentCreateArgs(targetType: "screenplay_scene", targetId: id, body: body)
        case .draft:
            args = CommentCreateArgs(targetType: "screenplay_draft", targetId: draftId, body: body)
        }
        do {
            let t = try await core.commentCreate(args)
            pending = nil
            focusedId = t.comment.id
            await reload()
            return nil
        } catch {
            return PanelText.message(error, fallback: "The comment could not be posted.")
        }
    }

    func reply(to thread: CommentThread, body: String) async -> String? {
        await act("The reply could not be posted.") { core in _ = try await core.commentReply(CommentReplyArgs(parentId: thread.comment.id, body: body)) }
    }

    func edit(_ comment: CommentDto, body: String) async -> String? {
        await act("The comment could not be changed.") { core in _ = try await core.commentUpdate(CommentUpdateArgs(id: comment.id, body: body)) }
    }

    func setStatus(_ comment: CommentDto, _ status: String) async {
        _ = await act("The comment could not be changed.") { core in _ = try await core.commentUpdate(CommentUpdateArgs(id: comment.id, status: status)) }
    }

    func resolve(_ comment: CommentDto) async {
        _ = await act("The comment could not be resolved.") { core in _ = try await core.commentResolve(CommentIdArgs(id: comment.id)) }
    }

    func reopen(_ comment: CommentDto) async {
        _ = await act("The comment could not be reopened.") { core in _ = try await core.commentReopen(CommentIdArgs(id: comment.id)) }
    }

    func delete(_ comment: CommentDto) async {
        _ = await act("The comment could not be deleted.") { core in _ = try await core.commentDelete(CommentIdArgs(id: comment.id)) }
    }

    private func act(_ fallback: String, _ work: (Core) async throws -> Void) async -> String? {
        guard let core else { return fallback }
        do {
            try await work(core)
            await reload()
            return nil
        } catch {
            return PanelText.message(error, fallback: fallback)
        }
    }

    // MARK: Showing the paragraph

    /// Show the paragraph a comment points at and put the caret there.
    @discardableResult
    func reveal(_ comment: CommentDto) -> Bool {
        guard let view = workspace?.textView else { return false }
        focusedId = comment.id
        guard let index = ScreenplayCaret.paragraphIndex(for: comment, in: view.paragraphs()) else { return false }
        view.reveal(paragraph: index)
        view.window?.makeFirstResponder(view)
        return true
    }

    /// "Scene 3 \u{00B7} text comment".
    static func targetLabel(_ c: CommentDto) -> String {
        let scene = c.sceneNumber.map { "Scene \(Int($0))" } ?? (c.sceneId != nil ? "Deleted scene" : "")
        switch c.targetType {
        case "screenplay_element": return "\(scene) \u{00B7} text comment"
        case "screenplay_scene": return "\(scene) \u{00B7} scene comment"
        case "screenplay_draft": return "Draft comment"
        default: return c.targetType.replacingOccurrences(of: "_", with: " ")
        }
    }

    static func composerLabel(_ target: ScreenplayCommentTarget, scenes: [ScreenplaySceneDto], elementScene: String?) -> String {
        func scene(_ id: String?) -> String { scenes.first { $0.id == id }.map { "Scene \($0.label)" } ?? "Scene" }
        switch target {
        case let .scene(id): return "\(scene(id)) \u{00B7} scene comment"
        case .element: return "\(scene(elementScene)) \u{00B7} text comment"
        case .draft: return "Draft comment"
        }
    }
}

// MARK: - Panel

struct CommentsPanel: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(\.ofTheme) private var theme
    @State private var tab = Tab.open

    enum Tab: String, CaseIterable, Identifiable { case open = "Open", resolved = "Resolved"; var id: String { rawValue } }

    var body: some View {
        let comments = state.comments
        VStack(alignment: .leading, spacing: 10) {
            Picker("Comment status", selection: $tab) {
                Text("Open (\(comments.openThreads.count))").tag(Tab.open)
                Text("Resolved (\(comments.resolvedThreads.count))").tag(Tab.resolved)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .accessibilityLabel("Comment status")

            if let target = comments.pending {
                CommentComposer(comments: comments, target: target, workspace: workspace)
            } else if comments.canComment {
                HStack {
                    Button { state.addCommentAtCaret() } label: { Label("Add Comment", systemImage: "text.bubble") }
                        .help("Comment on the selected text, the line, or the scene")
                        .accessibilityIdentifier("add-comment")
                    Button("Comment on Draft") { comments.pending = .draft }
                }
            }
            let shown = tab == .open ? comments.openThreads : comments.resolvedThreads
            if shown.isEmpty {
                Text(tab == .open ? "No comments yet." : "No resolved comments.").foregroundStyle(theme["muted"])
            }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    ForEach(shown, id: \.comment.id) { t in
                        ThreadCard(thread: t, comments: comments, focused: comments.focusedId == t.comment.id)
                    }
                }
            }
        }
        .onChange(of: comments.focusedId) { _, id in
            if let id, comments.resolvedThreads.contains(where: { $0.comment.id == id }) { tab = .resolved }
        }
        .accessibilityIdentifier("comments-panel")
    }
}

struct CommentComposer: View {
    let comments: CommentsPanelModel
    let target: ScreenplayCommentTarget
    let workspace: ScreenplayWorkspaceModel
    @Environment(\.ofTheme) private var theme
    @State private var body_ = ""
    @State private var error: String?
    @State private var busy = false

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(CommentsPanelModel.composerLabel(target, scenes: workspace.document?.scenes ?? [], elementScene: elementScene))
                .font(.caption).foregroundStyle(theme["muted"])
            if let quote = target.quote {
                Text("\u{201C}\(quote)\u{201D}").font(.callout).italic().lineLimit(3)
                    .padding(6).frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
            }
            PanelTextArea(label: "Comment", text: $body_, minHeight: 70)
                .accessibilityIdentifier("comment-field")
            if let error { PanelBanner(tone: .err, text: error) }
            HStack {
                Spacer()
                Button("Cancel") { comments.pending = nil }
                Button("Comment") {
                    busy = true
                    Task {
                        error = await comments.post(body_)
                        busy = false
                        if error == nil { body_ = "" }
                    }
                }
                .keyboardShortcut(.return, modifiers: .command)
                .disabled(body_.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || busy)
                .accessibilityIdentifier("comment-post")
            }
        }
        .padding(10)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["accent"]))
        .accessibilityElement(children: .contain)
    }

    /// The scene of a text comment: read from the editor, as the element may be new.
    private var elementScene: String? {
        guard case let .element(id, _, _, _) = target, let view = workspace.textView else { return nil }
        let ps = view.paragraphs()
        guard let i = ScreenplayCaret.paragraphIndex(ofElement: id, in: ps) else { return nil }
        return ScreenplayCaret.sceneId(in: ps, at: i)
    }
}

/// One comment with its replies and actions.
struct ThreadCard: View {
    let thread: CommentThread
    let comments: CommentsPanelModel
    var focused = false
    @Environment(\.ofTheme) private var theme
    @State private var reply: String?
    @State private var editing: String?
    @State private var error: String?

    var body: some View {
        let c = thread.comment
        let permissions = comments.permissions
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(c.authorName).fontWeight(.semibold)
                Button(CommentsPanelModel.targetLabel(c)) { comments.reveal(c) }
                    .buttonStyle(.link)
                    .font(.caption)
                    .help("Show in the script")
                    .accessibilityLabel("Show \(CommentsPanelModel.targetLabel(c)) in the script")
                Spacer()
                Menu {
                    if c.isMine { Button("Edit") { editing = c.body } }
                    if c.status == "Open", comments.canComment { Button("Mark In Discussion") { Task { await comments.setStatus(c, "In Discussion") } } }
                    if c.status == "In Discussion", comments.canComment { Button("Mark Open") { Task { await comments.setStatus(c, "Open") } } }
                    if c.isMine || permissions?.delete == true {
                        Divider()
                        Button("Delete", role: .destructive) { Task { await comments.delete(c) } }
                    }
                } label: { Image(systemName: "ellipsis") }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .accessibilityLabel("Comment actions")
            }
            HStack(spacing: 6) {
                if c.status != "Open" { PanelChip(text: c.status, tone: c.status == "Resolved" ? .green : .blue) }
                if c.contextMoved { PanelChip(text: "Context moved", tone: .red) }
                if c.targetDeleted && !c.contextMoved { PanelChip(text: "Target deleted", tone: .red) }
            }
            if let q = c.quotedText, !q.isEmpty {
                Button { comments.reveal(c) } label: {
                    Text("\u{201C}\(q)\u{201D}").italic().lineLimit(3).frame(maxWidth: .infinity, alignment: .leading)
                }
                .buttonStyle(.plain)
                .padding(6).background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
            }
            if let text = editing {
                editor(text: text, label: "Edit comment") { new in
                    error = await comments.edit(c, body: new)
                    if error == nil { editing = nil }
                } cancel: { editing = nil }
            } else {
                Text(c.body).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
            }
            Text("\(PanelText.ago(c.createdAt))\(c.resolvedBy.map { " \u{00B7} Resolved by \($0)" } ?? "")")
                .font(.caption).foregroundStyle(theme["muted"])
            ForEach(thread.replies, id: \.id) { r in
                VStack(alignment: .leading, spacing: 2) {
                    HStack {
                        Text(r.authorName).font(.callout.weight(.semibold))
                        Text(PanelText.ago(r.createdAt)).font(.caption).foregroundStyle(theme["muted"])
                        Spacer()
                        if r.isMine || permissions?.delete == true {
                            Button { Task { await comments.delete(r) } } label: { Image(systemName: "xmark") }
                                .buttonStyle(.borderless)
                                .help("Delete reply")
                                .accessibilityLabel("Delete reply")
                        }
                    }
                    Text(r.body).textSelection(.enabled)
                }
                .padding(.leading, 12)
            }
            if let text = reply {
                editor(text: text, label: "Reply") { new in
                    error = await comments.reply(to: thread, body: new)
                    if error == nil { reply = nil }
                } cancel: { reply = nil }
            }
            if let error { PanelBanner(tone: .err, text: error) }
            HStack {
                if comments.canComment, reply == nil { Button("Reply") { reply = "" } }
                if permissions?.resolveComments == true {
                    if c.status == "Resolved" {
                        Button("Reopen") { Task { await comments.reopen(c) } }
                    } else {
                        Button("Resolve") { Task { await comments.resolve(c) } }
                    }
                }
            }
            .controlSize(.small)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"].opacity(c.status == "Resolved" ? 0.5 : 1), in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(focused ? theme["accent"] : theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Comment by \(c.authorName): \(c.body)")
    }

    private func editor(text: String, label: String, save: @escaping (String) async -> Void, cancel: @escaping () -> Void) -> some View {
        let binding = Binding<String>(
            get: { label == "Reply" ? (reply ?? text) : (editing ?? text) },
            set: { if label == "Reply" { reply = $0 } else { editing = $0 } })
        return VStack(alignment: .leading, spacing: 6) {
            PanelTextArea(label: label, text: binding, minHeight: 54)
            HStack {
                Spacer()
                Button("Cancel", action: cancel)
                Button(label == "Reply" ? "Reply" : "Save") { Task { await save(binding.wrappedValue) } }
                    .disabled(binding.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            .controlSize(.small)
        }
    }
}

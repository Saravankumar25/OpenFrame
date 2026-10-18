// What the screenplay workspace needs to place the panels (Mac phase M4): one state object, one
// view modifier that adds the inspector, the sheets and the notice line, and the toolbar items.
// Nothing here holds a rule; the panels ask the core.

import AppKit
import SwiftUI
import OpenFrameCore

enum ScreenplayInspectorTab: String, CaseIterable, Identifiable {
    case scene = "Scene"
    case comments = "Comments"
    case draft = "Draft"
    var id: String { rawValue }
}

/// The dialogs the panels present.
enum ScreenplayPanelSheet: Identifiable, Equatable {
    case drafts
    case newDraft
    case compare(a: String?, b: String?)
    case review(startFor: String?)
    case pageSetup
    case titlePage
    case newScreenplay
    case renameScreenplay
    case deleteScreenplay

    var id: String {
        switch self {
        case .drafts: "drafts"
        case .newDraft: "new-draft"
        case let .compare(a, b): "compare-\(a ?? "-")-\(b ?? "-")"
        case let .review(d): "review-\(d ?? "-")"
        case .pageSetup: "page-setup"
        case .titlePage: "title-page"
        case .newScreenplay: "new-screenplay"
        case .renameScreenplay: "rename-screenplay"
        case .deleteScreenplay: "delete-screenplay"
        }
    }
}

@MainActor
@Observable
final class ScreenplayPanelState {
    var inspectorShown = false
    var tab: ScreenplayInspectorTab = .scene
    var sheet: ScreenplayPanelSheet?
    /// A scene chosen from a menu: the Scene tab shows it until the caret moves.
    var pinnedSceneId: String?
    /// A line that says what just happened ("Created draft \u{201C}Rewrite B\u{201D}").
    var notice: String?

    let comments = CommentsPanelModel()
    @ObservationIgnored private(set) weak var workspace: ScreenplayWorkspaceModel?
    @ObservationIgnored private var noticeTask: Task<Void, Never>?

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel) {
        self.workspace = workspace
        comments.attach(app, workspace)
    }

    func say(_ text: String) {
        notice = text
        noticeTask?.cancel()
        noticeTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(6))
            if !Task.isCancelled { self?.notice = nil }
        }
    }

    // MARK: Entry points for menus and buttons

    /// Show the Scene Hub of a scene (the navigator's context menu).
    func showSceneHub(sceneId: String) {
        pinnedSceneId = sceneId
        tab = .scene
        inspectorShown = true
    }

    func toggleComments() {
        if inspectorShown && tab == .comments { inspectorShown = false } else { tab = .comments; inspectorShown = true }
    }

    /// Comment on a scene (the navigator's context menu, or the Scene tab).
    func addComment(sceneId: String) {
        comments.pending = .scene(id: sceneId)
        tab = .comments
        inspectorShown = true
    }

    /// Comment on the selected text, the caret's line, or the caret's scene.
    func addCommentAtCaret() {
        guard let view = workspace?.textView, let context = ScreenplayCaret.current(in: view),
              let target = ScreenplayCaret.commentTarget(for: context) else {
            if let scene = workspace?.document?.scenes.first { addComment(sceneId: scene.id) }
            return
        }
        comments.pending = target
        tab = .comments
        inspectorShown = true
    }
}

// MARK: - Placement

extension View {
    /// Adds the screenplay panels to the workspace: the inspector on the right, the dialogs, and
    /// the notice line. `caretIndex` is the paragraph index the workspace already tracks.
    func screenplayPanels(model: ScreenplayWorkspaceModel, state: ScreenplayPanelState, caretIndex: Int) -> some View {
        modifier(ScreenplayPanelsHost(model: model, state: state, caretIndex: caretIndex))
    }
}

struct ScreenplayPanelsHost: ViewModifier {
    let model: ScreenplayWorkspaceModel
    @Bindable var state: ScreenplayPanelState
    let caretIndex: Int
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    private static let tables = ["comment", "screenplay_draft", "screenplay_scene", "screenplay_element"]

    func body(content: Content) -> some View {
        content
            .inspector(isPresented: $state.inspectorShown) {
                ScreenplayInspector(model: model, state: state, caretIndex: caretIndex)
                    .inspectorColumnWidth(min: 280, ideal: 340, max: 480)
            }
            .sheet(item: $state.sheet) { sheet in
                switch sheet {
                case .drafts: DraftsSheet(workspace: model, state: state)
                case .newDraft: NewDraftHost(workspace: model, state: state)
                case let .compare(a, b): CompareSheet(workspace: model, state: state, initialA: a, initialB: b)
                case let .review(startFor): ReviewSheet(workspace: model, state: state, startFor: startFor)
                case .pageSetup: PageSetupSheet(workspace: model, state: state)
                case .titlePage: PageSetupSheet(workspace: model, state: state, initialPage: .titlePage)
                case .newScreenplay: NewScreenplaySheet(workspace: model, state: state)
                case .renameScreenplay: RenameScreenplaySheet(workspace: model, state: state)
                case .deleteScreenplay: DeleteScreenplaySheet(workspace: model, state: state)
                }
            }
            .overlay(alignment: .top) {
                if let text = state.notice {
                    Label(text, systemImage: "checkmark.circle")
                        .font(.callout)
                        .padding(.horizontal, 12).padding(.vertical, 7)
                        .background(theme["green-soft"], in: Capsule())
                        .foregroundStyle(theme["green-t"])
                        .padding(.top, 8)
                        .transition(.opacity)
                        .accessibilityIdentifier("panel-notice")
                        .accessibilityAddTraits(.updatesFrequently)
                        .onTapGesture { state.notice = nil }
                }
            }
            .animation(.default, value: state.notice)
            .task {
                state.attach(app, model)
                await state.comments.reload()
            }
            .task(id: "\(model.draft?.id ?? "-")-\(app.changes.key(Self.tables))") {
                await state.comments.reload()
            }
            .onChange(of: caretIndex) { state.pinnedSceneId = nil }
    }
}

/// "New Draft" from the toolbar: a draft from the current draft.
struct NewDraftHost: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @State private var model = DraftsPanelModel()

    var body: some View {
        Group {
            if let source = workspace.overview?.drafts.first(where: { $0.isCurrent }) ?? workspace.overview?.drafts.first {
                NewDraftSheet(model: model, source: source)
            } else {
                PanelSheetFrame(title: "New Draft") { Text("There is no draft to copy yet.") } footer: {
                    Button("Cancel") { state.sheet = nil }
                }
            }
        }
        .task { model.attach(app, workspace, say: { state.say($0) }) }
    }
}

// MARK: - Toolbar

/// Toolbar items: Compare, the Comments button with its count, and a menu for the rest.
struct ScreenplayPanelToolbar: ToolbarContent {
    @Bindable var state: ScreenplayPanelState
    let model: ScreenplayWorkspaceModel

    var body: some ToolbarContent {
        ToolbarItem(placement: .navigation) {
            ScreenplayPickerMenu(workspace: model, state: state)
        }
        ToolbarItem(placement: .automatic) {
            Button { state.sheet = .compare(a: nil, b: nil) } label: { Label("Compare", systemImage: "rectangle.split.2x1") }
                .help("Compare two drafts")
                .accessibilityIdentifier("compare-button")
        }
        ToolbarItem(placement: .automatic) {
            Button { state.toggleComments() } label: {
                Label("Comments (\(state.comments.openCount))", systemImage: "text.bubble")
            }
            .help("Show the comments of this draft")
            .accessibilityLabel("Comments (\(state.comments.openCount))")
            .accessibilityIdentifier("comments-button")
        }
        ToolbarItem(placement: .automatic) {
            Menu {
                Button("Add Comment") { state.addCommentAtCaret() }.keyboardShortcut("m", modifiers: [.command, .option])
                Button("Scene Hub") { state.tab = .scene; state.inspectorShown = true }
                Button(state.inspectorShown ? "Hide Inspector" : "Show Inspector") { state.inspectorShown.toggle() }
                Divider()
                Button("Drafts\u{2026}") { state.sheet = .drafts }
                Button("Review\u{2026}") { state.sheet = .review(startFor: nil) }
                Divider()
                Button("Page Setup\u{2026}") { state.sheet = .pageSetup }
                Button("Title Page\u{2026}") { state.sheet = .titlePage }
            } label: {
                Label("Screenplay Tools", systemImage: "ellipsis.circle")
            }
            .help("Comments, drafts, review and page setup")
            .accessibilityLabel("Screenplay Tools")
            .accessibilityIdentifier("screenplay-tools-menu")
        }
    }
}

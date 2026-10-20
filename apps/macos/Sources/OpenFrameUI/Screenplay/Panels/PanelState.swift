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
    case pageSetup
    case titlePage
    case newScreenplay
    case renameScreenplay
    case deleteScreenplay
    case customElements
    case revisions
    case lock(draftId: String)
    case pageView

    var id: String {
        switch self {
        case .drafts: "drafts"
        case .newDraft: "new-draft"
        case let .compare(a, b): "compare-\(a ?? "-")-\(b ?? "-")"
        case .pageSetup: "page-setup"
        case .titlePage: "title-page"
        case .newScreenplay: "new-screenplay"
        case .renameScreenplay: "rename-screenplay"
        case .deleteScreenplay: "delete-screenplay"
        case .customElements: "custom-elements"
        case .revisions: "revisions"
        case let .lock(draftId): "lock-\(draftId)"
        case .pageView: "page-view"
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
                case .pageSetup: PageSetupSheet(workspace: model, state: state)
                case .titlePage: PageSetupSheet(workspace: model, state: state, initialPage: .titlePage)
                case .newScreenplay: NewScreenplaySheet(workspace: model, state: state)
                case .renameScreenplay: RenameScreenplaySheet(workspace: model, state: state)
                case .deleteScreenplay: DeleteScreenplaySheet(workspace: model, state: state)
                case .customElements: CustomElementsSheet(workspace: model, state: state)
                case .revisions: RevisionsSheet(workspace: model, state: state)
                case let .lock(draftId): LockDraftHost(workspace: model, state: state, draftId: draftId)
                case .pageView: PageViewSheet(workspace: model, state: state)
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

/// "Lock as Shooting Draft": the confirmation with the core's summary of the draft.
struct LockDraftHost: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    let draftId: String
    @Environment(AppModel.self) private var app
    @State private var model = DraftsPanelModel()

    var body: some View {
        Group {
            if let draft = workspace.overview?.drafts.first(where: { $0.id == draftId }) {
                LockDraftSheet(model: model, draft: draft)
            } else {
                PanelSheetFrame(title: "Lock this draft as the Shooting Draft?") { Text("This draft is no longer there.") } footer: {
                    Button("Cancel") { state.sheet = nil }
                }
            }
        }
        .task { model.attach(app, workspace, say: { state.say($0) }) }
    }
}

// MARK: - Toolbar

/// What the screenplay toolbar holds. The window is 1360 points wide by default and the toolbar
/// shares it with the project's own items, so only what the writer reaches for all the time has
/// a place of its own. The rest lives in the Screenplay Tools menu, where nothing is lost to the
/// system's overflow menu ("more toolbar items"), which the keyboard and VoiceOver reach badly.
enum ScreenplayToolbarLayout {
    /// The items on the toolbar itself, left to right.
    static let primary = [
        "screenplay-picker", "element-picker", "compare-button", "comments-button",
        "replace-toggle", "draft-menu", "screenplay-tools-menu", "editor-save-status",
    ]
    /// The entries of the Screenplay Tools menu that used to be toolbar items.
    static let toolsMenuEntries = ["layout-menu", "screenplay-interchange-menu", "custom-elements-menu-item", "page-view-menu-item"]
}

/// Toolbar items: Compare, the Comments button with its count, and a menu for the rest.
struct ScreenplayPanelToolbar: ToolbarContent {
    @Bindable var state: ScreenplayPanelState
    let model: ScreenplayWorkspaceModel
    var setMode: (ScreenplayMode) -> Void = { _ in }

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
    }
}

/// The Screenplay Tools menu: comments, drafts, revisions, page setup and the rarer tools.
struct ScreenplayToolsMenu: View {
    @Bindable var state: ScreenplayPanelState
    let model: ScreenplayWorkspaceModel
    let setMode: (ScreenplayMode) -> Void

    var body: some View {
        Menu {
            Button("Add Comment") { state.addCommentAtCaret() }.keyboardShortcut("m", modifiers: [.command, .option])
            Button("Scene Hub") { state.tab = .scene; state.inspectorShown = true }
            Button(state.inspectorShown ? "Hide Inspector" : "Show Inspector") { state.inspectorShown.toggle() }
            Divider()
            Button("Drafts\u{2026}") { state.sheet = .drafts }
            if let draft = model.draft {
                if model.overview?.permissions.lock == true, draft.status != .locked {
                    Button("Lock as Shooting Draft\u{2026}") { state.sheet = .lock(draftId: draft.id) }
                        .accessibilityIdentifier("lock-button")
                }
                if draft.status == .locked || draft.status == .revision {
                    Button("Revisions\u{2026}") { state.sheet = .revisions }
                        .accessibilityIdentifier("revisions-button")
                }
            }
            Divider()
            Button("Page Setup\u{2026}") { state.sheet = .pageSetup }
            Button("Title Page\u{2026}") { state.sheet = .titlePage }
            Button("Custom Elements\u{2026}") { state.sheet = .customElements }
                .accessibilityIdentifier("custom-elements-menu-item")
            Button("Page View\u{2026}") { state.sheet = .pageView }
                .accessibilityIdentifier("page-view-menu-item")
            Divider()
            Menu("Layout") {
                ForEach(ScreenplayMode.allCases, id: \.self) { m in
                    Button { setMode(m) } label: {
                        if model.mode == m { Label(m.title, systemImage: "checkmark") } else { Text(m.title) }
                    }
                }
            }
            .help("Standard, Writing Room (side pane) or Focus (only the page). Focus: Shift+Command+F. Writing Room: Shift+Command+R")
            .accessibilityIdentifier("layout-menu")
            Menu("Import and Export") {
                Button("Import Screenplay\u{2026}") { InterchangePresenter.shared.present(.importScreenplay(openScreenplayId: model.overview?.screenplay?.id)) }
                    .disabled(!(model.overview?.permissions.edit ?? true))
                    .accessibilityIdentifier("screenplay-import")
                Button("Export Screenplay\u{2026}") { InterchangePresenter.shared.present(.exportScreenplay(draftId: model.draft?.id)) }
                    .accessibilityIdentifier("screenplay-export")
                Button("Print\u{2026}") { InterchangePresenter.shared.present(.printPreview(draftId: model.draft?.id)) }
                    .accessibilityIdentifier("screenplay-print")
            }
            .accessibilityIdentifier("screenplay-interchange-menu")
        } label: {
            Label("Screenplay Tools", systemImage: "ellipsis.circle")
        }
        .help("Comments, drafts, revisions, page setup, layout, import and export")
        .accessibilityLabel("Screenplay Tools")
        .accessibilityIdentifier("screenplay-tools-menu")
    }
}

// Revisions (Mac phase M4): the revision history of the screenplay, the scenes a revision
// changed and the printed pages that differ from the locked pages. Behaviour follows
// RevisionDrawer in lock.tsx on Windows, plus the Page view with revision marks (PageViewDrawer
// in pagesetup.tsx). The core compares and counts; this file asks and shows.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class RevisionsPanelModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?

    var changes: ScreenplayCompareResult?
    var pages: ScreenplayRevisionPages?
    var loading = false

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel) {
        self.app = app
        self.workspace = workspace
    }

    private var core: Core? { app?.core }
    var drafts: [ScreenplayDraftDto] { workspace?.overview?.drafts ?? [] }
    var openDraft: ScreenplayDraftDto? { workspace?.draft }
    var permissions: ScreenplayPermissions? { workspace?.overview?.permissions }

    /// Revision drafts, newest first.
    var revisions: [ScreenplayDraftDto] {
        drafts.filter { $0.status == .revision || ($0.revisionLabel != nil && $0.status != .locked) }
            .sorted { $0.createdAt > $1.createdAt }
    }

    /// Locked drafts, newest first: the production baselines.
    var baselines: [ScreenplayDraftDto] { drafts.filter { $0.status == .locked }.sorted { $0.createdAt > $1.createdAt } }

    /// The revision the changes are shown for: the open draft when it is a revision, else the newest.
    var shown: ScreenplayDraftDto? {
        if let open = openDraft, open.status == .revision || open.revisionLabel != nil { return open }
        return revisions.first
    }

    /// The locked draft a new revision starts from.
    var lockedSource: ScreenplayDraftDto? {
        if let open = openDraft, open.status == .locked { return open }
        return baselines.first
    }

    func load() async {
        changes = nil
        pages = nil
        guard let core, let shown else { return }
        loading = true
        defer { loading = false }
        if shown.createdFromDraftId != nil {
            changes = try? await core.screenplayRevisionChanges(ScreenplayRevisionChangesArgs(draftId: shown.id))
        }
        pages = try? await core.screenplayRevisionPages(ScreenplayDraftIdArgs(draftId: shown.id))
    }

    /// A changed scene as the small chip shows it: "Sc 2", "Sc 3 new", "Sc 4 removed", "Sc 5 moved".
    struct Chip: Equatable {
        var label: String
        var sceneId: String?
        var kind: ScreenplaySceneChangeKind
        var heading: String
    }

    var chips: [Chip] {
        (changes?.scenes ?? []).compactMap { c in
            guard let side = c.b ?? c.a else { return nil }
            let suffix: String
            switch c.kind {
            case .added: suffix = " new"
            case .removed: suffix = " removed"
            case .moved: suffix = " moved"
            default: suffix = ""
            }
            return Chip(label: "Sc \(side.label)\(suffix)", sceneId: c.b?.sceneId, kind: c.kind, heading: side.heading)
        }
    }

    /// "Changed pages: 2" (or "none"). Nil while the draft has no locked baseline to compare with.
    var changedPagesLine: String? {
        guard let pages, pages.baselineDraftName != nil else { return nil }
        let list = pages.changedPages.map { String(Int($0)) }
        return "Changed pages: " + (list.isEmpty ? "none" : list.joined(separator: ", "))
    }

    /// The small print after the changed pages.
    var changedPagesDetail: String? {
        guard let pages, let name = pages.baselineDraftName else { return nil }
        var s = "(compared with the locked pages of \(name): \(Int(pages.baselinePageCount)) pages then, \(Int(pages.pageCount)) now"
        if let date = pages.revisionDate, !date.isEmpty { s += "; dated \(date)" }
        return s + ")"
    }

    var summaryLine: String? {
        guard let changes else { return nil }
        let s = changes.summary
        return "Compared with \(changes.draftA.name): \(Int(s.changed)) changed \u{00B7} \(Int(s.added)) added \u{00B7} \(Int(s.removed)) removed \u{00B7} \(Int(s.moved)) moved."
    }

    /// Put the caret on a scene's heading in the editor.
    func jump(to sceneId: String) {
        guard let paragraph = workspace?.paragraphIndex(ofScene: sceneId) else { return }
        workspace?.textView?.reveal(paragraph: paragraph)
    }
}

struct RevisionsSheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = RevisionsPanelModel()
    @State private var drafts = DraftsPanelModel()
    @State private var startFrom: ScreenplayDraftDto?
    @State private var unlockError: String?

    private static let tables = ["screenplay_draft", "screenplay_scene", "screenplay_element", "screenplay_revision", "screenplay_lock_baseline"]

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            PanelTitle("Revisions")
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    Text("Revision history").font(.headline).accessibilityAddTraits(.isHeader)
                    if model.revisions.isEmpty { Text("No revisions yet.").foregroundStyle(theme["muted"]) }
                    VStack(spacing: 0) {
                        ForEach(model.revisions, id: \.id) { revisionRow($0) }
                        ForEach(model.baselines, id: \.id) { baselineRow($0) }
                    }
                    .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
                    .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("revisions-list")

                    if let source = model.lockedSource, model.permissions?.edit == true {
                        HStack {
                            Button("Start Revision") { startFrom = source }
                                .accessibilityIdentifier("revisions-start")
                            Text("From \(source.name). The locked draft stays exactly as it is.")
                                .font(.caption).foregroundStyle(theme["muted"])
                        }
                    }
                    if let open = model.openDraft, open.status == .locked, model.permissions?.unlock == true {
                        PanelBanner(tone: .info, text: "Only the project owner can unlock a shooting draft.")
                        Button("Unlock draft") {
                            Task {
                                unlockError = await drafts.unlock(open)
                                await model.load()
                            }
                        }
                        .accessibilityIdentifier("revisions-unlock")
                        if let unlockError { PanelBanner(tone: .err, text: unlockError) }
                    }
                    changesSection
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            HStack {
                Spacer()
                Button("Close") { state.sheet = nil }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("revisions-close")
            }
        }
        .padding(PageSpacing.large)
        .sheetSize(width: 600, height: 560)
        .background(theme["panel"])
        .task(id: "\(workspace.draft?.id ?? "-")-\(app.changes.key(Self.tables))") {
            model.attach(app, workspace)
            drafts.attach(app, workspace, say: { state.say($0) })
            await model.load()
        }
        .sheet(isPresented: Binding(get: { startFrom != nil }, set: { if !$0 { startFrom = nil } })) {
            if let source = startFrom { StartRevisionSheet(model: drafts, source: source) }
        }
        .accessibilityIdentifier("revisions-sheet")
    }

    @ViewBuilder private var changesSection: some View {
        if let shown = model.shown {
            Text("Changed scenes in \(shown.revisionLabel ?? shown.name)")
                .font(.headline).padding(.top, 8).accessibilityAddTraits(.isHeader)
            if model.loading && model.changes == nil {
                ProgressView().controlSize(.small)
            } else if model.chips.isEmpty {
                Text("No scenes changed yet.").foregroundStyle(theme["muted"])
            } else {
                RevisionChipFlow(spacing: 6) {
                    ForEach(Array(model.chips.enumerated()), id: \.offset) { _, chip in
                        Button(chip.label) {
                            if let id = chip.sceneId { model.jump(to: id); state.sheet = nil }
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .tint(chip.kind == .removed ? .red : chip.kind == .added ? .green : .blue)
                        .disabled(chip.sceneId == nil || shown.id != model.openDraft?.id)
                        .help(chip.heading)
                        .accessibilityIdentifier("revisions-scene-chip")
                    }
                }
            }
            if let line = model.changedPagesLine {
                (Text(line).bold() + Text(" ") + Text(model.changedPagesDetail ?? "").font(.caption).foregroundStyle(theme["muted"]))
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("revisions-changed-pages")
            }
            if let summary = model.summaryLine {
                Text(summary).font(.caption).foregroundStyle(theme["muted"])
            }
        }
    }

    private func revisionRow(_ r: ScreenplayDraftDto) -> some View {
        HStack(spacing: 8) {
            Circle().fill(PanelText.swatch(r.revisionColor) ?? Color.gray.opacity(0.4)).frame(width: 10, height: 10).accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 2) {
                Text(r.revisionLabel ?? r.name).fontWeight(.semibold) + Text(r.revisionColor.map { " \u{00B7} \($0)" } ?? "").fontWeight(.semibold)
                let reason = r.revisionReason.flatMap { $0.isEmpty ? nil : " \u{00B7} Reason: \($0)" } ?? ""
                Text(PanelText.date(r.createdAt) + reason).font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer(minLength: 8)
            if r.isCurrent { PanelChip(text: "Current revision", tone: .accent) }
            if r.status == .locked { PanelChip(text: "Locked", tone: .green) }
            Button("Open") { Task { await drafts.open(r); await model.load() } }
                .disabled(r.id == model.openDraft?.id)
                .accessibilityLabel("Open \(r.revisionLabel ?? r.name)")
        }
        .padding(.horizontal, 10).padding(.vertical, 7)
        .background(r.id == model.openDraft?.id ? theme["accent-soft"].opacity(0.4) : Color.clear)
        .accessibilityElement(children: .contain)
    }

    private func baselineRow(_ b: ScreenplayDraftDto) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "lock").accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 2) {
                Text(b.name).fontWeight(.semibold)
                Text("Locked \(b.lockedAt.map(PanelText.date) ?? "") \u{00B7} baseline").font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer(minLength: 8)
            PanelChip(text: "Locked", tone: .green)
            Button("Open") { Task { await drafts.open(b); await model.load() } }
                .disabled(b.id == model.openDraft?.id)
                .accessibilityLabel("Open \(b.name)")
        }
        .padding(.horizontal, 10).padding(.vertical, 7)
        .background(b.id == model.openDraft?.id ? theme["accent-soft"].opacity(0.4) : Color.clear)
        .accessibilityElement(children: .contain)
    }
}

/// A row of items that wraps onto more lines.
struct RevisionChipFlow: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0, maxX: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > 0, x + size.width > width { x = 0; y += rowHeight + spacing; rowHeight = 0 }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
            maxX = max(maxX, x - spacing)
        }
        return CGSize(width: maxX, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowHeight: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > bounds.minX, x + size.width > bounds.maxX { x = bounds.minX; y += rowHeight + spacing; rowHeight = 0 }
            s.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}

// MARK: - Page view

/// The pages exactly as they print (the same layout as the PDF), with the option to show the
/// revision marks. The core lays the lines out; this view draws them.
@MainActor
@Observable
final class PageViewModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?

    var preview: InterchangePrintPreview?
    var failed = false
    var marks = false

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel) {
        self.app = app
        self.workspace = workspace
    }

    func load() async {
        guard let core = app?.core, let id = workspace?.draft?.id else { return }
        await workspace?.flush()
        do {
            preview = try await core.screenplayPageView(ScreenplayPageViewArgs(draftId: id, revisionMarks: marks))
            failed = false
        } catch {
            failed = true
        }
    }

    /// "Title page" or "Page 2".
    static func title(_ preview: InterchangePrintPreview, _ index: Int) -> String {
        guard preview.pages.indices.contains(index) else { return "" }
        let page = preview.pages[index]
        if page.titlePage { return "Title page" }
        return "Page \(page.number.map { String(Int($0)) } ?? String(index + 1))"
    }

    /// Lines of a page that carry a revision mark.
    static func markedLines(_ page: InterchangePrintPage) -> Int {
        page.lines.filter { $0.style == "revision_mark" }.count
    }
}

struct PageViewSheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = PageViewModel()

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            PanelTitle("Page view")
            HStack(spacing: 12) {
                Toggle("Show revision marks", isOn: Binding(get: { model.marks }, set: { model.marks = $0; Task { await model.load() } }))
                    .toggleStyle(.checkbox)
                    .accessibilityIdentifier("page-view-marks")
                if let p = model.preview {
                    let n = Int(p.scriptPageCount)
                    Text("\(p.paper) \u{00B7} \(n) page\(n == 1 ? "" : "s")").font(.caption).foregroundStyle(theme["muted"])
                }
            }
            ScrollView {
                VStack(spacing: 14) {
                    if let p = model.preview {
                        ForEach(p.warnings, id: \.code) { PanelBanner(tone: .warn, text: $0.message) }
                        ForEach(Array(p.pages.enumerated()), id: \.offset) { i, page in
                            VStack(spacing: 4) {
                                Text(PageViewModel.title(p, i)).font(.caption).foregroundStyle(theme["muted"])
                                PrintedPage(preview: p, page: page)
                                    .frame(width: 330, height: 330 * p.pageHeightIn / p.pageWidthIn)
                                    .accessibilityElement()
                                    .accessibilityLabel(PageViewModel.title(p, i))
                                    .accessibilityValue(page.lines.map(\.text).filter { !$0.isEmpty }.joined(separator: ", "))
                            }
                        }
                    } else if model.failed {
                        Text("The page view is not available.").foregroundStyle(theme["muted"])
                    } else {
                        ProgressView().controlSize(.small)
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .accessibilityIdentifier("page-view-pages")
            HStack {
                Spacer()
                Button("Close") { state.sheet = nil }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("page-view-close")
            }
        }
        .padding(PageSpacing.large)
        .sheetSize(width: 440, height: 640)
        .background(theme["panel"])
        .task {
            model.attach(app, workspace)
            await model.load()
        }
        .accessibilityIdentifier("page-view-sheet")
    }
}

/// One page of the core's layout, drawn at true proportions on white paper.
struct PrintedPage: View {
    let preview: InterchangePrintPreview
    let page: InterchangePrintPage
    private static let ascentPerPoint = 0.125 / 12

    var body: some View {
        GeometryReader { geo in
            let scale = geo.size.width / preview.pageWidthIn
            Canvas { context, _ in
                let points = preview.fontSizePt / 72 * scale
                for line in page.lines where !line.text.isEmpty {
                    var text = AttributedString(line.text)
                    text.font = .system(size: points, design: .monospaced).weight(line.style == "title_main" ? .bold : .regular)
                    text.foregroundColor = line.style == "revision_mark" ? .blue : .black
                    let y = (line.yIn - Self.ascentPerPoint * preview.fontSizePt) * scale
                    context.draw(Text(text), at: CGPoint(x: line.xIn * scale, y: y), anchor: .topLeading)
                }
            }
            .frame(width: geo.size.width, height: geo.size.height)
            .background(Color.white.shadow(.drop(color: .black.opacity(0.25), radius: 3, y: 1)))
        }
    }
}

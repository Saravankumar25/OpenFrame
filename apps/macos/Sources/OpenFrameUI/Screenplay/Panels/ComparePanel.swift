// Compare two drafts (Mac phase M4): a read-only view of screenplay.compare, as CompareView.tsx
// on Windows. A list of changed scenes, and for the chosen scene the two drafts side by side or
// as one inline column. Nothing here changes a draft.

import SwiftUI
import OpenFrameCore

/// The views of a comparison, with the Windows names first (CompareView.tsx): Scene-only is the
/// list of changed scenes, Text-only the chosen scene's text before and after side by side. Text
/// inline (both versions in one column) is the Mac's own third view.
enum CompareMode: String, CaseIterable, Identifiable {
    case scenes = "Scene-only"
    case sideBySide = "Text-only"
    case inline = "Text inline"
    var id: String { rawValue }
}

@MainActor
@Observable
final class ComparePanelModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?

    var a = ""
    var b = ""
    var result: ScreenplayCompareResult?
    var error: String?
    var loading = false
    var mode: CompareMode = .sideBySide
    var selection: Int?

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel, a: String?, b: String?) {
        self.app = app
        self.workspace = workspace
        let sorted = drafts.sorted { $0.createdAt < $1.createdAt }
        let current = drafts.first { $0.isCurrent }
        self.b = b ?? current?.id ?? sorted.last?.id ?? ""
        self.a = a ?? sorted.first { $0.id != self.b }?.id ?? sorted.first?.id ?? ""
    }

    var drafts: [ScreenplayDraftDto] { workspace?.overview?.drafts ?? [] }
    var sortedDrafts: [ScreenplayDraftDto] { drafts.sorted { $0.createdAt < $1.createdAt } }
    var changes: [ScreenplaySceneChange] { result?.scenes ?? [] }
    /// Indexes of the scenes that differ, in document order.
    var changedIndexes: [Int] { changes.indices.filter { changes[$0].kind != .unchanged } }
    var selected: ScreenplaySceneChange? { selection.flatMap { changes.indices.contains($0) ? changes[$0] : nil } }

    func load() async {
        result = nil
        error = nil
        selection = nil
        guard let core = app?.core, !a.isEmpty, !b.isEmpty, a != b else { return }
        loading = true
        defer { loading = false }
        do {
            result = try await core.screenplayCompare(ScreenplayCompareArgs(draftA: a, draftB: b))
            selection = changedIndexes.first
        } catch {
            self.error = PanelText.message(error, fallback: "The drafts could not be compared.")
        }
    }

    func step(_ direction: Int) {
        let list = changedIndexes
        guard !list.isEmpty else { return }
        let pos = selection.flatMap { list.firstIndex(of: $0) } ?? (direction > 0 ? -1 : 0)
        selection = list[(pos + direction + list.count) % list.count]
    }

    // MARK: Wording

    static func name(_ c: ScreenplaySceneChange) -> String {
        let side = c.b ?? c.a
        return side.map { "Scene \($0.label)" } ?? "Scene"
    }

    static func describe(_ c: ScreenplaySceneChange) -> String {
        let n = name(c)
        switch c.kind {
        case .added: return "\(n) added"
        case .removed: return "\(c.a.map { "Scene \($0.label)" } ?? n) removed"
        case .moved: return "\(n) moved (was \(c.a?.label ?? "?"))"
        case .changed: return "\(PanelText.plural(Int(c.linesChanged), "line")) changed in \(n)"
        case .unchanged: return "\(n) unchanged"
        }
    }

    static func kindLabel(_ k: ScreenplaySceneChangeKind) -> String {
        switch k {
        case .added: "Added"
        case .removed: "Removed"
        case .changed: "Changed"
        case .moved: "Moved"
        case .unchanged: "Same"
        }
    }

    static func summaryLine(_ s: ScreenplayCompareSummary) -> String {
        "\(PanelText.plural(Int(s.added), "scene")) added \u{00B7} \(Int(s.removed)) removed \u{00B7} \(Int(s.changed)) changed \u{00B7} \(Int(s.moved)) moved \u{00B7} \(Int(s.unchanged)) unchanged"
    }

    /// One line with its changed words marked. Removed words are struck through and added words
    /// are underlined, so the marks do not rely on colour alone.
    static func attributed(_ line: ScreenplayDiffLine, theme: ResolvedTheme) -> AttributedString {
        var out = AttributedString()
        for seg in line.segments {
            var part = AttributedString(seg.text)
            switch seg.kind {
            case .equal: break
            case .removed:
                part.backgroundColor = theme["red-soft"]
                part.foregroundColor = theme["red-t"]
                part.strikethroughStyle = .single
            case .added:
                part.backgroundColor = theme["green-soft"]
                part.foregroundColor = theme["green-t"]
                part.underlineStyle = .single
            }
            out += part
        }
        if line.segments.isEmpty { out = AttributedString(line.text) }
        return out
    }

    /// The text of the changed scene as the plain text of both sides, for tests and accessibility.
    static func plainRows(_ c: ScreenplaySceneChange) -> [(left: String?, right: String?)] {
        c.rows.map { ($0.left?.text, $0.right?.text) }
    }
}

struct CompareSheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    let initialA: String?
    let initialB: String?
    @Environment(AppModel.self) private var app
    @State private var model = ComparePanelModel()

    var body: some View {
        CompareView(model: model, state: state)
            .task {
                model.attach(app, workspace, a: initialA, b: initialB)
                await model.load()
            }
    }
}

/// The comparison itself, over a model that is already attached.
struct CompareView: View {
    @Bindable var model: ComparePanelModel
    let state: ScreenplayPanelState
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            toolbar
                .padding(PageSpacing.medium)
            Divider()
            content
        }
        .sheetSize(width: 940, height: 640)
        .background(theme["panel"])
        .accessibilityIdentifier("compare-view")
    }

    private var toolbar: some View {
        HStack(spacing: 10) {
            Picker("Before", selection: $model.a) {
                ForEach(model.sortedDrafts, id: \.id) { Text($0.name).tag($0.id) }
            }
            .frame(width: 210)
            .accessibilityLabel("Before")
            Image(systemName: "arrow.right").accessibilityHidden(true)
            Picker("After", selection: $model.b) {
                ForEach(model.sortedDrafts, id: \.id) { Text($0.name).tag($0.id) }
            }
            .frame(width: 210)
            .accessibilityLabel("After")
            Spacer()
            Picker("Comparison view", selection: $model.mode) {
                ForEach(CompareMode.allCases) { Text($0.rawValue).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 270)
            .accessibilityLabel("Comparison view")
            Button { model.step(-1) } label: { Image(systemName: "chevron.up") }
                .disabled(model.changedIndexes.isEmpty)
                .help("Previous change")
                .accessibilityLabel("Previous change")
            Button { model.step(1) } label: { Image(systemName: "chevron.down") }
                .disabled(model.changedIndexes.isEmpty)
                .help("Next change")
                .accessibilityLabel("Next change")
            Button("Done") { state.sheet = nil }.keyboardShortcut(.cancelAction)
        }
        .onChange(of: model.a) { Task { await model.load() } }
        .onChange(of: model.b) { Task { await model.load() } }
    }

    @ViewBuilder private var content: some View {
        if model.a == model.b {
            PanelBanner(tone: .info, text: "Choose two different drafts to compare.").padding(16)
            Spacer()
        } else if let error = model.error {
            PanelBanner(tone: .err, text: error).padding(16)
            Spacer()
        } else if model.loading || model.result == nil {
            ProgressView().controlSize(.small).frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            HSplitView {
                changeList.frame(minWidth: 220, idealWidth: 250, maxWidth: 320)
                detail.frame(minWidth: 520)
            }
        }
    }

    private var changeList: some View {
        let groups: [(ScreenplaySceneChangeKind, [Int])] = [.removed, .added, .changed, .moved].map { k in
            (k, model.changes.indices.filter { model.changes[$0].kind == k })
        }
        return List(selection: $model.selection) {
            Section("Changes") {
                ForEach(groups, id: \.0) { _, indexes in
                    ForEach(indexes, id: \.self) { i in
                        let c = model.changes[i]
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            PanelChip(text: ComparePanelModel.kindLabel(c.kind), tone: chipTone(c.kind))
                            Text(ComparePanelModel.describe(c) + (c.ambiguous ? " \u{00B7} check match" : ""))
                                .font(.callout).lineLimit(2)
                        }
                        .tag(i)
                    }
                }
                if let s = model.result?.summary, s.unchanged > 0 {
                    HStack(spacing: 6) {
                        PanelChip(text: "Same")
                        Text(PanelText.plural(Int(s.unchanged), "scene") + " unchanged").font(.callout).foregroundStyle(theme["muted"])
                    }
                    .selectionDisabled()
                }
            }
        }
        .listStyle(.sidebar)
        .accessibilityLabel("Changes")
        .accessibilityIdentifier("compare-changes")
    }

    private func chipTone(_ k: ScreenplaySceneChangeKind) -> PanelChip.Tone {
        switch k {
        case .added: .green
        case .removed: .red
        case .changed: .blue
        case .moved: .yellow
        case .unchanged: .plain
        }
    }

    @ViewBuilder private var detail: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                if let s = model.result?.summary {
                    Text("Summary: \(ComparePanelModel.summaryLine(s))").font(.callout)
                    if s.ambiguous > 0 {
                        PanelBanner(tone: .warn, text: "\(PanelText.plural(Int(s.ambiguous), "scene match", "scene matches")) \(s.ambiguous == 1 ? "is" : "are") uncertain (same heading, no shared history). Check them before relying on this comparison.")
                    }
                }
                if model.changedIndexes.isEmpty {
                    Text("No differences found.").font(.headline).padding(.top, 20)
                } else if model.mode == .scenes {
                    sceneTable
                } else if let c = model.selected {
                    HStack {
                        Text(ComparePanelModel.describe(c)).fontWeight(.semibold)
                        if c.ambiguous { PanelChip(text: "Uncertain match", tone: .yellow) }
                    }
                    if c.rows.isEmpty {
                        Text("The text of this scene is identical; only its position changed.").foregroundStyle(theme["muted"])
                    } else if model.mode == .sideBySide {
                        sideBySide(c)
                    } else {
                        inline(c)
                    }
                } else {
                    Text("Choose a change on the left.").foregroundStyle(theme["muted"])
                }
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func sideBySide(_ c: ScreenplaySceneChange) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .top, spacing: 12) {
                Text("\(model.result?.draftA.name ?? "") (before)").font(.caption.weight(.semibold)).frame(maxWidth: .infinity, alignment: .leading)
                Text("\(model.result?.draftB.name ?? "") (after)").font(.caption.weight(.semibold)).frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.bottom, 6)
            ForEach(Array(c.rows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .top, spacing: 12) {
                    diffLine(row.left, side: "removed")
                    diffLine(row.right, side: "added")
                }
                .padding(.vertical, 3)
                .background(row.kind == .equal ? Color.clear : theme["surface2"].opacity(0.5))
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Side-by-side comparison")
        .accessibilityIdentifier("compare-side-by-side")
    }

    private func inline(_ c: ScreenplaySceneChange) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(Array(c.rows.enumerated()), id: \.offset) { _, row in
                switch row.kind {
                case .equal, .added: diffLine(row.right ?? row.left, side: "added")
                case .removed: diffLine(row.left, side: "removed")
                case .changed:
                    diffLine(row.left, side: "removed")
                    diffLine(row.right, side: "added")
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Inline comparison")
        .accessibilityIdentifier("compare-inline")
    }

    @ViewBuilder private func diffLine(_ line: ScreenplayDiffLine?, side: String) -> some View {
        if let line {
            Text(ComparePanelModel.attributed(line, theme: theme))
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .accessibilityLabel(line.text.isEmpty ? "empty line" : line.text)
        } else {
            Color.clear.frame(maxWidth: .infinity, minHeight: 1).accessibilityHidden(true)
        }
    }

    private var sceneTable: some View {
        Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
            GridRow {
                Text("Change").font(.caption.weight(.semibold))
                Text(model.result?.draftA.name ?? "").font(.caption.weight(.semibold))
                Text(model.result?.draftB.name ?? "").font(.caption.weight(.semibold))
                Text("Matched by").font(.caption.weight(.semibold))
            }
            Divider()
            ForEach(model.changes.indices, id: \.self) { i in
                let c = model.changes[i]
                GridRow {
                    PanelChip(text: ComparePanelModel.kindLabel(c.kind), tone: chipTone(c.kind))
                    Text(c.a.map { "\($0.label). \($0.heading.isEmpty ? "Untitled scene" : $0.heading)" } ?? "Not in this draft")
                    Text(c.b.map { "\($0.label). \($0.heading.isEmpty ? "Untitled scene" : $0.heading)" } ?? "Not in this draft")
                    Text(c.matchedBy == "identity" ? "Same scene" : c.matchedBy == "heading" ? "Heading\(c.ambiguous ? " (uncertain)" : "")" : "Not matched")
                        .font(.caption).foregroundStyle(theme["muted"])
                }
                .contentShape(Rectangle())
                .onTapGesture { model.selection = i; model.mode = .sideBySide }
            }
        }
        .accessibilityIdentifier("compare-scenes")
    }
}

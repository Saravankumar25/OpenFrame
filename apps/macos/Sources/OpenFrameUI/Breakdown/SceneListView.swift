// The scene list (left pane): derived numbers, headings and short status chips. Filters run in the
// core (`breakdown.scenes`), so they stay fast on large projects; the counts cover the whole
// source. Arrow keys move through the scenes.

import SwiftUI
import OpenFrameCore

struct SceneListView: View {
    @Bindable var model: BreakdownModel
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private var filterLabel: String {
        let s = model.saved
        if s.filter == .all, s.category == nil { return "Filter" }
        return [s.filter == .all ? nil : s.filter.title, s.category?.rawValue].compactMap { $0 }.joined(separator: " \u{00B7} ")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            if let c = model.counts {
                Text("\(Int(c.complete)) of \(Int(c.all)) complete")
                    .font(.caption).foregroundStyle(theme["muted"])
                    .padding(.horizontal, 14).padding(.bottom, 6)
                    .accessibilityIdentifier("scene-complete-count")
            }
            if model.rows.isEmpty {
                Text("No scenes match this filter.").font(.callout).foregroundStyle(theme["muted"]).padding(14)
                    .accessibilityIdentifier("scene-list-empty")
            }
            List(selection: Binding(get: { model.mode == .scene ? model.sceneId : nil }, set: { if let id = $0 { model.selectScene(id) } })) {
                ForEach(model.rows, id: \.sceneId) { row in
                    SceneRowView(row: row, selected: model.mode == .scene && row.sceneId == model.sceneId)
                        .tag(row.sceneId)
                        .listRowSeparator(.hidden)
                }
            }
            .listStyle(.sidebar)
            .scrollContentBackground(.hidden)
            .accessibilityLabel("Scenes in the Production Source")
            .accessibilityIdentifier("scene-list")
            if model.needsReviewCount > 0 || model.historicalCount > 0 {
                Divider()
                VStack(alignment: .leading, spacing: 2) {
                    if model.needsReviewCount > 0 {
                        Button { model.mode = .review } label: {
                            Label("Needs Review (\(model.needsReviewCount))", systemImage: "exclamationmark.triangle")
                        }
                        .accessibilityIdentifier("open-needs-review")
                    }
                    if model.historicalCount > 0 {
                        Button { model.mode = .history } label: {
                            Label("Removed scenes (\(model.historicalCount))", systemImage: "clock.arrow.circlepath")
                        }
                        .accessibilityIdentifier("open-removed-scenes")
                    }
                }
                .buttonStyle(.plain)
                .font(.callout)
                .padding(10)
            }
        }
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
    }

    private var header: some View {
        HStack(spacing: 8) {
            Text("Scenes").font(.headline)
            BDChip(text: model.saved.filter == .all && model.saved.category == nil ? "\(model.total)" : "\(model.rows.count) of \(model.total)")
            Spacer()
            Menu {
                Picker("Show", selection: Binding(get: { model.saved.filter }, set: { model.setFilter($0) })) {
                    ForEach(BreakdownSceneFilter.allCases, id: \.self) { f in
                        Text(model.counts.map { "\(f.title) (\(f.count(in: $0)))" } ?? f.title).tag(f)
                    }
                }
                .pickerStyle(.inline)
                Divider()
                Picker("Category", selection: Binding(get: { model.saved.category }, set: { model.setCategory($0) })) {
                    Text("Any category").tag(BreakdownCategory?.none)
                    ForEach(BDCategory.all, id: \.self) { Text($0.rawValue).tag(BreakdownCategory?.some($0)) }
                }
                .pickerStyle(.inline)
            } label: {
                Label(filterLabel, systemImage: "line.3.horizontal.decrease.circle")
            }
            .menuStyle(.borderlessButton)
            .fixedSize()
            .accessibilityLabel("Filter scenes: \(model.saved.filter.title)\(model.saved.category.map { ", \($0.rawValue)" } ?? "")")
            .accessibilityIdentifier("scene-filter")
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
    }
}

private struct SceneRowView: View {
    let row: BreakdownSceneRow
    let selected: Bool
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(row.number).font(.callout.weight(.semibold)).monospacedDigit().frame(minWidth: 26, alignment: .leading)
                Text(row.omitted ? "OMITTED: \(row.heading)" : (row.heading.isEmpty ? "(no heading)" : row.heading))
                    .font(.callout)
                    .lineLimit(2)
                    .foregroundStyle(row.omitted ? theme["muted"] : theme["ink"])
                    .strikethrough(row.omitted)
            }
            if row.complete || row.suggestedCount > 0 || row.needsReview || row.needsBreakdown || !row.categories.isEmpty {
                BDFlowLayout(spacing: 4) {
                    ForEach(row.categories, id: \.self) { CategoryMark(look: hub.look($0), showsLabel: false, size: 10) }
                    if row.complete { BDChip(text: "Complete", tone: .green, symbol: "checkmark") }
                    if row.suggestedCount > 0 { BDChip(text: "\(Int(row.suggestedCount)) suggested", tone: .accent) }
                    if row.needsReview { BDChip(text: "Needs review", tone: .yellow) }
                    if row.needsBreakdown { BDChip(text: "Needs breakdown", tone: .red) }
                }
                .padding(.leading, 34)
            }
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("scene-row-\(row.number)")
    }
}

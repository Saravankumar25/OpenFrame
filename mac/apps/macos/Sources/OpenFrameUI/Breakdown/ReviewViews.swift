// "Needs Review" (scenes whose script changed after production planning began) and the kept
// breakdown of scenes that are no longer in the Production Source (FSD 54: kept, never deleted).

import SwiftUI
import OpenFrameCore

struct NeedsReviewView: View {
    @Bindable var model: BreakdownModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: PageSpacing.medium) {
            ReviewHeader(title: "Needs Review", sub: "Scenes whose script changed after production planning began.") { model.mode = .scene }
            let list = model.reviewRows.filter(\.needsReview)
            if list.isEmpty {
                ContentUnavailableView("Nothing to review.", systemImage: "checkmark.circle",
                                       description: Text("Every planned scene matches the Production Source text."))
            } else {
                List(list, id: \.sceneId) { r in
                    HStack(alignment: .top, spacing: 10) {
                        Text(r.number).font(.callout.weight(.semibold)).frame(width: 34, alignment: .leading)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(r.heading).font(.callout.weight(.semibold))
                            Text("\(r.changeMessage ?? "") \(r.changedAt.map { "Changed \(Formatting.dateTime(ms: $0))." } ?? "") \(BDText.plural(r.confirmedCount.asInt, "element")) kept.")
                                .font(.caption).foregroundStyle(theme["muted"])
                        }
                        Spacer()
                        if r.headingChanged { BDChip(text: "Heading changed", tone: .yellow) }
                        if r.textChanged { BDChip(text: "Text changed", tone: .yellow) }
                        Button("Review") {
                            model.selectScene(r.sceneId)
                            model.sheet = .changes(r.sceneId)
                        }
                        .controlSize(.small)
                        .accessibilityLabel("Review Scene \(r.number)")
                    }
                    .accessibilityElement(children: .combine)
                }
                .listStyle(.inset)
            }
        }
        .pageColumn(maxWidth: PageSpacing.column)
        .padding(.bottom, PageSpacing.medium)
        .accessibilityIdentifier("needs-review")
    }
}

struct HistoricalView: View {
    @Bindable var model: BreakdownModel
    @State private var showArchived = false
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let scenes = (model.view?.historical ?? []).map { s -> BreakdownHistoricalScene in
            var s = s
            s.elements = s.elements.filter { showArchived || !$0.archived }
            return s
        }.filter { !$0.elements.isEmpty }
        VStack(spacing: PageSpacing.medium) {
            ReviewHeader(title: "Removed scenes", sub: "Scene removed from current source. Production data is kept here so nothing is lost; archive what you no longer need.") { model.mode = .scene }
            Toggle("Show archived", isOn: $showArchived).toggleStyle(.checkbox).frame(maxWidth: .infinity, alignment: .center)
            if scenes.isEmpty {
                ContentUnavailableView("Nothing kept from removed scenes.", systemImage: "clock.arrow.circlepath",
                                       description: Text("When a script update removes a scene, its breakdown stays here."))
            } else {
                List {
                    ForEach(scenes, id: \.sceneId) { s in
                        Section {
                            ForEach(s.elements, id: \.id) { e in
                                HStack(spacing: 8) {
                                    Circle().fill(theme["muted"]).frame(width: 8, height: 8).accessibilityHidden(true)
                                    Text(e.name).font(.callout.weight(.semibold))
                                    Spacer()
                                    Text(e.category.rawValue).font(.caption).foregroundStyle(theme["muted"])
                                    if e.archived { BDChip(text: "Archived") }
                                    if model.canEdit {
                                        Button(e.archived ? "Unarchive" : "Archive") { Task { await model.setArchived(e, !e.archived) } }
                                            .controlSize(.small)
                                            .accessibilityLabel("\(e.archived ? "Unarchive" : "Archive") \(e.name)")
                                    }
                                }
                                .opacity(e.archived ? 0.6 : 1)
                            }
                        } header: {
                            HStack {
                                Text(s.heading.isEmpty ? "(no heading)" : s.heading).font(.callout.weight(.semibold))
                                if let d = s.draftName { Text("from \(d)").font(.caption).foregroundStyle(theme["muted"]) }
                                Spacer()
                                Button("Open") { model.selectScene(s.sceneId) }.controlSize(.small)
                            }
                        }
                    }
                }
                .listStyle(.inset)
            }
        }
        .pageColumn(maxWidth: PageSpacing.column)
        .padding(.bottom, PageSpacing.medium)
        .accessibilityIdentifier("removed-scenes")
    }
}

private struct ReviewHeader: View {
    let title: String
    let sub: String
    let back: () -> Void

    var body: some View {
        VStack(spacing: PageSpacing.small) {
            HStack {
                Button { back() } label: { Label("Back to Breakdown", systemImage: "chevron.left") }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("back-to-breakdown")
                Spacer(minLength: 0)
            }
            PageHeader(title, subtitle: sub)
        }
    }
}

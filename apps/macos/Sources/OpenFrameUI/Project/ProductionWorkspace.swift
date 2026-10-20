// The Production workspace (Windows: workspaces/production): one place for the thirteen planning
// tabs, in the Windows order. Each tab shows the Mac page that already does that work; the tab
// strip wraps onto a second line in a narrow window instead of scrolling sideways.

import OpenFrameCore
import SwiftUI

struct ProductionWorkspace: View {
    @Binding var tab: ProductionTab
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            // As tall as the segmented bars the pages had before, so every page keeps its room.
            ProductionTabBar(selection: $tab)
                .padding(.horizontal, PageSpacing.medium)
                .padding(.vertical, 5)
            Divider()
            page.frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(theme["bg"])
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("production-workspace")
    }

    @ViewBuilder private var page: some View {
        switch tab {
        case .overview: ProductionOverviewView()
        case .catalog: BreakdownWorkspace(fixed: .catalog)
        case .locations: BreakdownWorkspace(fixed: .locations)
        case .castCrew: BreakdownWorkspace(fixed: .castCrew)
        case .moodboards: VisualPlanningWorkspace(fixed: .moodboards)
        case .storyboards: VisualPlanningWorkspace(fixed: .storyboards)
        case .shotLists: VisualPlanningWorkspace(fixed: .shots)
        case .schedule: ScheduleWorkspace(fixed: .schedule)
        case .dayOutOfDays: ScheduleWorkspace(fixed: .dood)
        case .daily: CallSheetsWorkspace(fixed: .daily)
        case .sides: CallSheetsWorkspace(fixed: .sides)
        case .reports: CallSheetsWorkspace(fixed: .reports)
        case .budget: BudgetWorkspace()
        }
    }
}

/// The Production tabs: labelled buttons in the style of the other Mac tab strips, on one line.
/// When the pane is too narrow for all thirteen (a small window with the AI Assistant open), the
/// strip becomes a menu that names the tab shown, so the page keeps its height.
struct ProductionTabBar: View {
    @Binding var selection: ProductionTab
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ViewThatFits(in: .horizontal) {
            strip
            compact
        }
    }

    private var compact: some View {
        Picker("Production", selection: $selection) {
            ForEach(ProductionTab.allCases) { Text($0.title).tag($0) }
        }
        .fixedSize()
        .frame(maxWidth: .infinity, alignment: .center)
        .accessibilityIdentifier("production-tab-menu")
    }

    private var strip: some View {
        HStack(spacing: 2) {
            ForEach(ProductionTab.allCases) { tab in
                let on = tab == selection
                Button {
                    selection = tab
                } label: {
                    Text(tab.title)
                        .font(.callout.weight(on ? .semibold : .regular))
                        .lineLimit(1)
                        .fixedSize()
                        .padding(.horizontal, 10).padding(.vertical, 3)
                        .foregroundStyle(on ? theme["accent-ink"] : theme["ink2"])
                        .background(on ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                        .contentShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
                .accessibilityIdentifier("production-tab-\(tab.rawValue)")
            }
        }
        .padding(2)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .fixedSize()
        .frame(maxWidth: .infinity, alignment: .center)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Production")
        .accessibilityIdentifier("production-tabs")
    }
}

// MARK: Overview

/// What to do next to prepare the shoot (Windows: production/tabs/overview.tsx): the script the
/// production follows, breakdown progress, what is still unresolved, and what needs attention.
struct ProductionOverviewView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var overview: ProductionOverview?
    @State private var failure: CoreFailure?

    private static let tables = ["production_source", "screenplay_draft", "screenplay_scene", "breakdown_element",
                                 "breakdown_suggestion", "breakdown_scene_status", "location", "cast_assignment",
                                 "catalog_item", "project_member"]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: PageSpacing.large) {
                PageHeader("Production", subtitle: "What do I need to do next to prepare the shoot?") {
                    if overview != nil {
                        Button(overview?.source == nil ? "Start Production Setup" : "Change Production Source") {
                            // The source is chosen where the breakdown is made.
                            ProjectNav.show(.breakdown)
                        }
                        .accessibilityIdentifier("production-source-button")
                    }
                }
                if let failure {
                    BDBanner(.err, failure.message)
                }
                if let o = overview {
                    if o.source == nil {
                        BDBanner(.info, "Production starts from a named screenplay draft. Choose it with Start Production Setup. Your screenplay is not changed, and you do not recreate any scene by hand.")
                    }
                    tiles(o)
                    attention(o)
                } else if failure == nil {
                    ProgressView().frame(maxWidth: .infinity)
                }
            }
            .padding(.bottom, PageSpacing.page)
            .pageColumn(maxWidth: PageSpacing.wideColumn)
        }
        .task(id: app.changes.key(Self.tables)) {
            do {
                overview = try await app.core?.productionOverview()
                failure = nil
            } catch {
                failure = CoreFailure(asCore: error)
            }
        }
        .accessibilityIdentifier("production-overview")
    }

    private func tiles(_ o: ProductionOverview) -> some View {
        let complete = Int(o.completeCount), scenes = Int(o.sceneCount)
        let unresolvedCast = o.unresolvedCharacters.count
        return CenteredCards(cardWidth: 220, spacing: PageSpacing.medium) {
            tile("Script status",
                 value: o.source?.draft.name ?? "No source yet",
                 sub: o.source.map { "\($0.draft.screenplayTitle) \u{00B7} \($0.draft.status == "Locked" ? "Locked" : "Not locked") \u{00B7} used as Production Source" }
                     ?? "Choose the draft production is based on",
                 id: "script") { ProjectNav.show(.breakdown) }
            tile("Breakdown progress",
                 value: "\(complete) / \(scenes)",
                 sub: "scenes marked complete \u{00B7} \(Int(o.plannedCount)) with confirmed elements"
                     + (o.pendingSuggestions > 0 ? " \u{00B7} \(Int(o.pendingSuggestions)) suggestions to review" : ""),
                 id: "breakdown", progress: scenes > 0 ? Double(complete) / Double(scenes) : 0) { ProjectNav.show(.breakdown) }
            tile("Unresolved locations",
                 value: "\(Int(o.unresolvedLocations))",
                 sub: o.locationCount > 0 ? "still Idea or Shortlisted (of \(Int(o.locationCount)))" : "No locations yet",
                 id: "locations") { ProjectNav.show(.production, tab: .locations) }
            tile("Unresolved cast",
                 value: "\(unresolvedCast)",
                 sub: "character\(unresolvedCast == 1 ? "" : "s") without an actor (of \(Int(o.characterCount)))",
                 id: "cast") { ProjectNav.show(.production, tab: .castCrew) }
            tile("Catalog",
                 value: "\(Int(o.catalogCount))",
                 sub: "reusable production items",
                 id: "catalog") { ProjectNav.show(.production, tab: .catalog) }
        }
    }

    private func tile(_ title: String, value: String, sub: String, id: String, progress: Double? = nil,
                      open: @escaping () -> Void) -> some View {
        Button(action: open) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
                Text(value).font(.system(size: 22, weight: .semibold)).lineLimit(2).foregroundStyle(theme["ink"])
                Text(sub).font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                if let progress {
                    ProgressView(value: progress)
                        .accessibilityLabel("Scenes marked complete")
                }
            }
            .padding(PageSpacing.medium)
            .frame(maxWidth: .infinity, minHeight: 104, alignment: .topLeading)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
        .help("Open \(title)")
        .accessibilityIdentifier("production-tile-\(id)")
    }

    private struct Attention: Identifiable {
        let id: String
        let title: String
        let detail: String
        let action: String
        let run: () -> Void
    }

    private func attentionItems(_ o: ProductionOverview) -> [Attention] {
        var out: [Attention] = []
        if let src = o.source, let newer = src.newer {
            out.append(Attention(id: "newer", title: "Newer revision available",
                                 detail: "\(newer.name) is newer. Production stays on \(src.draft.name) until you review the update.",
                                 action: "Review") { ProjectNav.show(.breakdown) })
        }
        if !o.needsReview.isEmpty {
            let n = o.needsReview.count
            out.append(Attention(id: "review", title: "\(n) scene\(n == 1 ? " needs" : "s need") breakdown review",
                                 detail: "Script changed in Scene\(n == 1 ? "" : "s") \(Self.sceneList(o.needsReview)).",
                                 action: "Review") { ProjectNav.show(.breakdown) })
        }
        if !o.needsBreakdown.isEmpty {
            let n = o.needsBreakdown.count
            out.append(Attention(id: "breakdown", title: "\(n) new scene\(n == 1 ? " needs" : "s need") breakdown",
                                 detail: "Scene\(n == 1 ? "" : "s") \(Self.sceneList(o.needsBreakdown)) arrived with a script update.",
                                 action: "Open") { ProjectNav.show(.breakdown) })
        }
        if o.historicalElements > 0 {
            let n = Int(o.historicalElements)
            out.append(Attention(id: "historical", title: "\(n) breakdown element\(n == 1 ? "" : "s") from removed scenes",
                                 detail: "Kept from scenes that are no longer in the Production Source. Archive them when you no longer need them.",
                                 action: "Review") { ProjectNav.show(.breakdown) })
        }
        if !o.unresolvedCharacters.isEmpty {
            let names = o.unresolvedCharacters
            out.append(Attention(id: "cast", title: "Needs casting",
                                 detail: names.prefix(6).joined(separator: ", ") + (names.count > 6 ? " and \(names.count - 6) more" : ""),
                                 action: "Open") { ProjectNav.show(.production, tab: .castCrew) })
        }
        return out
    }

    private func attention(_ o: ProductionOverview) -> some View {
        let items = attentionItems(o)
        return VStack(alignment: .leading, spacing: PageSpacing.small) {
            SectionHeader("Needs attention")
            VStack(alignment: .leading, spacing: 0) {
                if items.isEmpty {
                    Text("Nothing needs attention right now.").foregroundStyle(theme["muted"]).padding(PageSpacing.medium)
                }
                ForEach(items) { a in
                    HStack(alignment: .firstTextBaseline, spacing: PageSpacing.small) {
                        (Text(a.title + ": ").bold() + Text(a.detail))
                            .font(.callout)
                            .fixedSize(horizontal: false, vertical: true)
                        Spacer(minLength: PageSpacing.small)
                        Button(a.action, action: a.run).controlSize(.small)
                            .accessibilityLabel("\(a.action): \(a.title)")
                    }
                    .padding(.horizontal, PageSpacing.medium).padding(.vertical, 10)
                    .accessibilityIdentifier("production-attention-\(a.id)")
                    if a.id != items.last?.id { Divider() }
                }
            }
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        }
    }

    /// "4", "4 and 7", "4, 7 and 9", or the first five and how many more.
    static func sceneList(_ refs: [ProductionSceneRef]) -> String {
        let nums = refs.compactMap(\.number).filter { !$0.isEmpty }
        if nums.count <= 1 { return nums.first ?? "" }
        if nums.count > 6 { return nums.prefix(5).joined(separator: ", ") + " and \(nums.count - 5) more" }
        return nums.dropLast().joined(separator: ", ") + " and " + nums.last!
    }
}

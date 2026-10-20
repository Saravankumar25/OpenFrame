// The Breakdown workspace: "What does this scene require to shoot?" Five tabs share one model
// root: Breakdown (scenes of the Production Source, the script, the scene's elements), Catalog,
// Locations, Cast and Crew, and Categories (colours and icons). Mirrors
// apps/desktop/src/workspaces/breakdown and the Catalog, Locations and Cast and Crew tabs of the
// Production workspace, with Mac conventions (a tab strip, an inspector on the right, context
// menus, sheets, Command shortcuts). The Mac has no separate Production workspace in Chapter 1.

import AppKit
import SwiftUI
import OpenFrameCore

struct BreakdownWorkspace: View {
    /// The one tab this place shows (the Breakdown workspace, or a Production tab); nil shows
    /// the hub with its own tab bar (tests and previews).
    var fixed: BDTab?
    @Environment(AppModel.self) private var app
    @Environment(ProjectHubs.self) private var hubs: ProjectHubs?
    @State private var own: BreakdownHub?

    var body: some View {
        Group {
            if let hub = hubs?.breakdown ?? own {
                BreakdownContent(hub: hub, fixed: fixed)
            } else {
                ProgressView()
            }
        }
        .fitsWindow()
        .task { if hubs == nil, own == nil { own = BreakdownHub(app: app) } }
    }
}

struct BreakdownContent: View {
    @Bindable var hub: BreakdownHub
    var fixed: BDTab?
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            if fixed == nil {
                tabBar
                Divider()
            }
            content.frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .followsHubTab(hub.tab, fixed: fixed, set: { hub.tab = $0 }, place: { $0.place })
        .background(theme["bg"])
        .overlay(alignment: .bottom) { BDNoticeBar(hub: hub) }
        .environment(hub)
        .task(id: app.changes.key(BDTables.categoryStyle)) { await hub.reloadStyles() }
        .task(id: app.changes.key(BDTables.members)) { await hub.reloadMembers() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("breakdown-workspace")
    }

    private var tabBar: some View {
        HStack(spacing: 12) {
            Picker("Breakdown views", selection: $hub.tab) {
                ForEach(BDTab.allCases) { Text($0.title).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .accessibilityLabel("Production")
            .accessibilityIdentifier("breakdown-tabs")
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, PageSpacing.small)
    }

    @ViewBuilder private var content: some View {
        switch fixed ?? hub.tab {
        case .breakdown: BreakdownTabView(model: hub.breakdown)
        case .catalog: CatalogView(model: hub.catalog)
        case .locations: LocationsView(model: hub.locations)
        case .castCrew: CastCrewView(model: hub.people)
        case .categories: CategoryStylesView(model: hub.categories)
        }
    }
}

// MARK: Breakdown tab

struct BreakdownTabView: View {
    @Bindable var model: BreakdownModel
    @Environment(AppModel.self) private var app
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private var dataKey: String {
        "\(app.changes.key(BDTables.production))|\(model.saved.filter.rawValue)|\(model.saved.category?.rawValue ?? "")|\(model.catalogItemId ?? "")"
    }
    private var pickKey: String { "\(model.picked ?? "")|\(model.mode)" }

    var body: some View {
        Group {
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if model.view == nil, let f = model.loadFailure {
                ContentUnavailableView {
                    Label("The breakdown could not be loaded.", systemImage: "exclamationmark.triangle")
                } description: {
                    Text(f.message)
                } actions: {
                    Button("Try Again") { Task { await model.reload() } }
                }
            } else if model.source == nil {
                noSource
            } else {
                loadedBody
            }
        }
        .task(id: dataKey) { await model.reload() }
        .task(id: pickKey) { await model.reloadScene() }
        .sheet(item: $model.sheet) { SheetHost(model: model, sheet: $0) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("breakdown-tab")
    }

    private var noSource: some View {
        ContentUnavailableView {
            Label("Choose a screenplay source to begin the breakdown.", systemImage: "list.bullet.rectangle")
        } description: {
            Text("Open a screenplay to begin breaking down scenes. Choose the draft that production should be based on.")
        } actions: {
            HStack {
                Button("Select Source") { model.sheet = .selectSource }
                    .buttonStyle(.borderedProminent)
                    .disabled(!model.canEdit)
                    .accessibilityIdentifier("select-source")
                Button { NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.screenplay.rawValue) } label: {
                    Label("Import Screenplay", systemImage: "doc.text")
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("breakdown-no-source")
    }

    // MARK: Loaded

    private var loadedBody: some View {
        VStack(spacing: 0) {
            if let src = model.source { sourceBar(src) }
            if let newer = model.newerDraft { newerBar(newer) }
            if let f = model.view?.catalogFilter { catalogBar(f) }
            GeometryReader { geo in
                HStack(spacing: 0) {
                    SceneListView(model: model).frame(width: geo.size.width < 1000 ? 210 : 250)
                    Divider()
                    center
                }
            }
        }
    }

    private func sourceBar(_ src: ProductionSourceInfo) -> some View {
        let locked = src.draft.status == "Locked"
        return HStack(spacing: 8) {
            Spacer(minLength: 0)
            Image(systemName: locked ? "lock" : "doc.text").accessibilityHidden(true)
            (Text("Production Source: \(src.draft.name) of \(BDText.quoted(src.draft.screenplayTitle))").bold()
                + Text((locked ? " \u{00B7} Locked" : " \u{00B7} \(src.draft.status)") + " \u{00B7} " + BDText.plural(model.total, "scene")
                       + (model.detail?.number.map { " \u{00B7} Scene \($0) of \(model.total)" } ?? "")))
                .lineLimit(2)
                .multilineTextAlignment(.center)
                .accessibilityIdentifier("production-source-line")
            if model.canEdit {
                Button("Change source") { model.sheet = .selectSource }.controlSize(.small)
                    .accessibilityIdentifier("change-source")
            }
        }
        .font(.callout)
        .foregroundStyle(theme["blue-t"])
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, 7)
        .frame(maxWidth: .infinity)
        .background(theme["blue-soft"])
        .accessibilityElement(children: .contain)
    }

    private func newerBar(_ newer: ProductionDraftInfo) -> some View {
        HStack(spacing: 8) {
            Spacer(minLength: 0)
            Image(systemName: "exclamationmark.triangle").accessibilityHidden(true)
            (Text("Newer revision available: ").bold() + Text("\(newer.name). Production stays on \(model.source?.draft.name ?? "the current draft") until you review the update."))
                .multilineTextAlignment(.center)
            if model.canEdit {
                Button("Review Production Update") { model.sheet = .updateSource(newer.id) }.controlSize(.small)
            }
            Button("Dismiss") { model.dismissedNewer = newer.id }.controlSize(.small)
            Spacer(minLength: 0)
        }
        .font(.callout)
        .foregroundStyle(theme["yellow-t"])
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, 7)
        .frame(maxWidth: .infinity)
        .background(theme["yellow-soft"])
    }

    private func catalogBar(_ f: BreakdownCatalogFilter) -> some View {
        HStack(spacing: 8) {
            Spacer(minLength: 0)
            Image(systemName: "line.3.horizontal.decrease.circle").accessibilityHidden(true)
            Text("Showing the \(BDText.plural(model.rows.count, "scene")) that use \(f.name) (\(f.category.rawValue)).")
                .accessibilityIdentifier("catalog-filter-line")
            Button("Show all scenes") { model.clearCatalogFilter() }.controlSize(.small)
                .accessibilityIdentifier("show-all-scenes")
            Spacer(minLength: 0)
        }
        .font(.callout)
        .foregroundStyle(theme["blue-t"])
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, 7)
        .frame(maxWidth: .infinity)
        .background(theme["blue-soft"])
    }

    @ViewBuilder private var center: some View {
        switch model.mode {
        case .review: NeedsReviewView(model: model)
        case .history: HistoricalView(model: model)
        case .scene:
            if model.total == 0 {
                ContentUnavailableView("This draft has no scenes yet.", systemImage: "film",
                                       description: Text("Scenes written in the Screenplay workspace appear here automatically."))
            } else if model.sceneId == nil {
                ContentUnavailableView("No scenes match this filter.", systemImage: "line.3.horizontal.decrease.circle",
                                       description: Text("Choose another filter, or show all scenes."))
            } else if let d = model.detail {
                // The elements panel takes up to 360 points, less when the window is narrow, and the
                // script takes the rest. Fixed widths from the room given, so no pane asks for more.
                GeometryReader { geo in
                    let side = min(360, max(250, geo.size.width * 0.38))
                    HStack(spacing: 0) {
                        ScriptPaneView(model: model, detail: d).frame(width: max(120, geo.size.width - side - 1))
                        Divider()
                        ElementsPanelView(model: model, detail: d).frame(width: side)
                    }
                }
            } else if let f = model.detailFailure {
                BDBanner(.err, f.message).padding(14).frame(maxHeight: .infinity, alignment: .top)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }
}

// MARK: Sheets

private struct SheetHost: View {
    @Bindable var model: BreakdownModel
    let sheet: BDSheet

    var body: some View {
        switch sheet {
        case .selectSource:
            SelectSourceSheet(model: model)
        case let .updateSource(draftId):
            UpdateSourceSheet(model: model, draftId: draftId)
        case let .add(initial):
            if let d = model.detail { AddElementHost(model: model, initial: initial, sceneNumber: d.number ?? "") }
        case let .edit(element, chooser):
            EditSuggestionHost(model: model, element: element, chooser: chooser, sceneNumber: model.number)
        case .review:
            if let d = model.detail { SuggestionReviewSheet(model: model, detail: d) }
        case let .changes(id):
            SceneChangesSheet(model: model, sceneId: id)
        case let .notes(element):
            ElementNotesSheet(model: model, element: element)
        }
    }
}

private struct AddElementHost: View {
    let model: BreakdownModel
    let initial: AddInitial
    let sceneNumber: String
    @State private var form: AddElementForm?

    var body: some View {
        if let form {
            AddElementSheet(form: form, sceneNumber: sceneNumber)
        } else {
            Color.clear.frame(width: 1, height: 1).onAppear { form = AddElementForm(model: model, initial: initial) }
        }
    }
}

private struct EditSuggestionHost: View {
    let model: BreakdownModel
    let element: BreakdownElementDto
    let chooser: Bool
    let sceneNumber: String
    @State private var form: EditSuggestionForm?

    var body: some View {
        if let form {
            EditSuggestionSheet(form: form, sceneNumber: sceneNumber)
        } else {
            Color.clear.frame(width: 1, height: 1).onAppear { form = EditSuggestionForm(model: model, element: element, chooser: chooser) }
        }
    }
}

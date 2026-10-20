// The Call Sheets workspace (Mac phase M6): "Which shooting day document do I need?" Four tabs
// share one model root: Call Sheets (the list and the paper-style editor), Daily View, Sides and
// Reports. On Windows the last three are tabs of the Production workspace; the Mac has no separate
// Production workspace in Chapter 1, so they live here. Mirrors apps/desktop/src/workspaces/
// call-sheets and production/tabs/{daily,sides,reports}.tsx.

import SwiftUI
import OpenFrameCore

struct CallSheetsWorkspace: View {
    /// The one tab this place shows (Call Sheets, or a Production tab); nil shows the hub with
    /// its own tab bar (tests and previews).
    var fixed: CSTab?
    @Environment(AppModel.self) private var app
    @Environment(ProjectHubs.self) private var hubs: ProjectHubs?
    @State private var own: CallSheetsHub?

    var body: some View {
        Group {
            if let hub = hubs?.callSheets ?? own {
                CallSheetsContent(hub: hub, fixed: fixed)
            } else {
                ProgressView()
            }
        }
        .fitsWindow()
        .task { if hubs == nil, own == nil { own = CallSheetsHub(app: app) } }
    }
}

struct CallSheetsContent: View {
    @Bindable var hub: CallSheetsHub
    var fixed: CSTab?
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
        .environment(hub)
        .sheet(item: $hub.exportModel) { m in
            CSExportSheet(model: m) { hub.exportModel = nil }
        }
        .task(id: app.changes.key(CSTables.members)) { await hub.reloadMembers() }
        .task(id: CallSheetsRoute.shared.pending) { await hub.consumeRoute() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("callsheets-workspace")
    }

    private var tabBar: some View {
        HStack(spacing: 12) {
            Picker("Call sheet views", selection: $hub.tab) {
                ForEach(CSTab.allCases) { Text($0.title).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .accessibilityLabel("Call sheet views")
            .accessibilityIdentifier("callsheets-tabs")
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, PageSpacing.small)
    }

    @ViewBuilder private var content: some View {
        switch fixed ?? hub.tab {
        case .callSheets:
            if let editor = hub.editor {
                CallSheetEditorView(model: editor).id(editor.id)
            } else {
                CallSheetListView(model: hub.list)
            }
        case .daily: CSDailyView(model: hub.daily)
        case .sides: CSSidesView(model: hub.sides)
        case .reports: CSReportsView(model: hub.reports)
        }
    }
}

/// A page title and a line under it, centred, with the actions in a row below (Windows PageHeader).
struct CSPageHeader<Actions: View>: View {
    let title: String
    var sub: String?
    @ViewBuilder var actions: () -> Actions

    var body: some View {
        PageHeader(title, subtitle: sub) { HStack(spacing: PageSpacing.small) { actions() } }
    }
}

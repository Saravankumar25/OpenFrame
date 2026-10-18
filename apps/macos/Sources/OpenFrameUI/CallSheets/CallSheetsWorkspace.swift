// The Call Sheets workspace (Mac phase M6): "Which shooting day document do I need?" Four tabs
// share one model root: Call Sheets (the list and the paper-style editor), Daily View, Sides and
// Reports. On Windows the last three are tabs of the Production workspace; the Mac has no separate
// Production workspace in Chapter 1, so they live here. Mirrors apps/desktop/src/workspaces/
// call-sheets and production/tabs/{daily,sides,reports}.tsx.

import SwiftUI
import OpenFrameCore

struct CallSheetsWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var hub: CallSheetsHub?

    var body: some View {
        Group {
            if let hub {
                CallSheetsContent(hub: hub)
            } else {
                ProgressView()
            }
        }
        .task { if hub == nil { hub = CallSheetsHub(app: app) } }
    }
}

struct CallSheetsContent: View {
    @Bindable var hub: CallSheetsHub
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            tabBar
            Divider()
            content
        }
        .background(theme["bg"])
        .environment(hub)
        .sheet(item: $hub.exportModel) { m in
            CSExportSheet(model: m) { hub.exportModel = nil }
        }
        .task(id: app.changes.key(CSTables.members)) { await hub.reloadMembers() }
        .task(id: CallSheetsRoute.shared.pending) { await hub.consumeRoute() }
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
            Spacer()
        }
        .padding(.horizontal, 16).padding(.vertical, 8)
    }

    @ViewBuilder private var content: some View {
        switch hub.tab {
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

/// A page title with a line under it and actions on the right (Windows PageHeader).
struct CSPageHeader<Actions: View>: View {
    let title: String
    var sub: String?
    @ViewBuilder var actions: () -> Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.title2.weight(.semibold)).accessibilityAddTraits(.isHeader)
                if let sub { Text(sub).font(.callout).foregroundStyle(theme["muted"]) }
            }
            Spacer(minLength: 8)
            actions()
        }
    }
}

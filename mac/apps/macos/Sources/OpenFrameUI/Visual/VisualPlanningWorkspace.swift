// The Visual Planning workspace: Moodboards, Storyboards and Shot Lists in one place. It mirrors
// the three Windows Production tabs (moodboards, storyboards, shots) with Mac conventions: a tab
// strip, an inspector on the right, context menus, sheets and native drag and drop with a keyboard
// way for every drag. Every screen reloads on the core's change events.

import SwiftUI
import OpenFrameCore

struct VisualPlanningWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var hub: VisualHub?

    var body: some View {
        Group {
            if let hub {
                VisualContent(hub: hub)
            } else {
                ProgressView()
            }
        }
        .fitsWindow()
        .task { if hub == nil { hub = VisualHub(app: app) } }
    }
}

struct VisualContent: View {
    @Bindable var hub: VisualHub
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            tabBar
            Divider()
            Group {
                if let f = hub.scenesFailure, hub.scenes == nil {
                    ContentUnavailableView {
                        Label("Visual Planning could not be loaded.", systemImage: "exclamationmark.triangle")
                    } description: {
                        Text(f.message)
                    } actions: {
                        Button("Try Again") { Task { await hub.reloadScenes() } }
                            .accessibilityIdentifier("visual-retry")
                    }
                    .accessibilityIdentifier("visual-load-failed")
                } else if !hub.scenesLoaded {
                    ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    switch hub.tab {
                    case .moodboards: MoodboardsView(model: hub.moodboards)
                    case .storyboards: StoryboardsView(model: hub.storyboards)
                    case .shots: ShotsView(model: hub.shots)
                    }
                }
            }
        }
        .background(theme["bg"])
        .environment(hub)
        .task(id: app.changes.key(VisualTables.scenes)) { await hub.reloadScenes() }
        .task(id: app.changes.key(["project_member"])) { await hub.reloadMembers() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("visual-workspace")
    }

    private var tabBar: some View {
        HStack(spacing: 12) {
            Picker("Visual Planning views", selection: $hub.tab) {
                ForEach(VisualTab.allCases) { Text($0.title).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .accessibilityLabel("Visual Planning")
            .accessibilityIdentifier("visual-tabs")
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, PageSpacing.medium).padding(.vertical, PageSpacing.small)
    }
}

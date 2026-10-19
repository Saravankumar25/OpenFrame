// Activity History (Windows: workspaces/activity). "What important actions have happened in this
// project? Read-only." Entries come from the core's command pipeline, never from single keystrokes.
// "Open" appears only while the affected object still exists.

import OpenFrameCore
import SwiftUI

enum ActivityFilter: String, CaseIterable, Identifiable {
    case all, drafts, story, production, exchange
    var id: String { rawValue }
    var title: String {
        switch self {
        case .all: "All"
        case .drafts: "Drafts"
        case .story: "Story"
        case .production: "Production"
        case .exchange: "Exchange"
        }
    }

    /// Which entries the filter keeps, by the start of the action name.
    fileprivate var pattern: String? {
        switch self {
        case .all: nil
        case .drafts: "^screenplay\\."
        case .story: "^(story|vault)\\."
        case .production: "^(production|breakdown|catalog|location|cast|crew|visual|moodboard|storyboard|shot|schedule|callsheet|call_sheet|budget|report)"
        case .exchange: "^(exchange|package|import|export)"
        }
    }

    func matches(_ entry: ActivityEntry) -> Bool {
        guard let pattern else { return true }
        return entry.action.range(of: pattern, options: .regularExpression) != nil
    }
}

@MainActor
@Observable
final class ActivityModel {
    static let page = 200
    static let ceiling = 1000

    let app: AppModel
    var filter: ActivityFilter = .all
    private(set) var limit = ActivityModel.page
    private(set) var entries: [ActivityEntry] = []
    private(set) var loaded = false
    private(set) var loading = false
    private(set) var loadFailure: CoreFailure?
    @ObservationIgnored private var generation = 0

    init(app: AppModel) { self.app = app }

    var shown: [ActivityEntry] { entries.filter { filter.matches($0) } }
    var canShowOlder: Bool { entries.count >= limit && limit < Self.ceiling }
    /// True when there may be more than are listed, so the button is worth showing.
    var mayHaveMore: Bool { entries.count >= limit }

    var emptyText: String {
        filter == .all
            ? "Important actions, such as drafts, moves, locks and production changes, are listed here as you work. Ordinary typing is never logged."
            : "Nothing of this kind has happened yet."
    }

    func showOlder() async {
        limit = min(limit + Self.page, Self.ceiling)
        await reload()
    }

    func reload() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        loading = true
        defer { if mine == generation { loading = false } }
        do {
            let rows = try await core.historyActivity(ActivityArgs(store: .project, limit: Double(limit)))
            guard mine == generation else { return }
            entries = rows
            loaded = true
            loadFailure = nil
        } catch {
            guard mine == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }
}

struct ActivityWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var model: ActivityModel?

    var body: some View {
        Group {
            if let model {
                ActivityScreen(model: model)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .fitsWindow()
        .onAppear {
            if model == nil { model = ActivityModel(app: app) }
        }
    }
}

struct ActivityScreen: View {
    @Bindable var model: ActivityModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ProjectPageHeader(title: "Activity History", subtitle: "What important actions have happened in this project? Read-only.", identifier: "activity") {
                Picker("Show activity", selection: $model.filter) {
                    ForEach(ActivityFilter.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 330)
                .accessibilityLabel("Show activity")
                .accessibilityIdentifier("activity-filter")
            }
            ScrollView {
                VStack(spacing: 12) {
                    if let failure = model.loadFailure {
                        Text(failure.message).foregroundStyle(theme["red-t"]).multilineTextAlignment(.center).frame(maxWidth: .infinity)
                            .accessibilityIdentifier("activity-load-error")
                    }
                    if !model.loaded {
                        ProgressView().padding(.top, 30)
                    } else if model.shown.isEmpty {
                        PageCard {
                            ContentUnavailableView {
                                Label("No major activity yet.", systemImage: "clock.arrow.circlepath")
                            } description: {
                                Text(model.emptyText)
                            }
                            .padding(.vertical, 12)
                        }
                        .accessibilityIdentifier("activity-empty")
                    } else {
                        PageCard {
                            LazyVStack(spacing: 0) {
                                ForEach(model.shown, id: \.id) { entry in
                                    row(entry)
                                    Divider().overlay(theme["line"])
                                }
                            }
                        }
                        .accessibilityElement(children: .contain)
                        .accessibilityLabel("Activity")
                        .accessibilityIdentifier("activity-list")
                    }
                    if model.mayHaveMore {
                        Button("Show older activity") { Task { await model.showOlder() } }
                            .disabled(!model.canShowOlder || model.loading)
                            .accessibilityIdentifier("activity-older")
                    }
                }
                .pageColumn(maxWidth: PageSpacing.column)
                .padding(.bottom, PageSpacing.large)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["bg"])
        .task(id: model.app.changes.any) { await model.reload() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("activity-workspace")
    }

    private func row(_ e: ActivityEntry) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(PageText.dayAndTime(ms: e.at)).font(.caption).foregroundStyle(theme["muted"])
                .frame(width: 120, alignment: .leading)
            Group {
                if let actor = e.actorName {
                    Text("\(Text(actor).fontWeight(.semibold)) \(PageText.sentence(summary: e.summary))")
                } else {
                    Text(e.summary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if PageNav.canOpen(e.nav) {
                Button { PageNav.open(e.nav) } label: { Label("Open", systemImage: "arrow.right") }
                    .controlSize(.small)
                    .accessibilityLabel("Open: \(e.summary)")
                    .accessibilityIdentifier("activity-open-\(e.id)")
            }
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("activity-row-\(e.id)")
    }
}

// The Schedule workspace (Mac phase M6). Windows has it as the Production tabs Schedule and Day
// Out of Days; here one workspace holds both. "What do we shoot, and when?" The stripboard, list
// and calendar of the shooting schedule, the shooting days with their times and timeline, the
// warnings and grouping advice (never applied on their own), script change review, the exports,
// and the Day Out of Days. Every read and change is a `schedule.*` operation of the core.

import SwiftUI
import OpenFrameCore

struct ScheduleWorkspace: View {
    /// The one section this place shows (Production > Schedule or Day Out of Days); nil shows
    /// both with their own tab bar (tests and previews).
    var fixed: SchedSection?
    @Environment(AppModel.self) private var app
    @Environment(ProjectHubs.self) private var hubs: ProjectHubs?
    @State private var own: ScheduleModel?

    var body: some View {
        Group {
            if let model = hubs?.schedule ?? own {
                ScheduleContent(model: model, fixed: fixed)
            } else {
                ProgressView()
            }
        }
        .task { if hubs == nil, own == nil { own = ScheduleModel(app: app) } }
    }
}

struct ScheduleContent: View {
    @Bindable var model: ScheduleModel
    var fixed: SchedSection?
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    private var dataKey: Int { app.changes.key(SchedTables.all) }
    private var doodKey: String { "\(dataKey)|\(model.section.rawValue)|\(model.doodCast)|\(model.doodFrom)|\(model.doodTo)" }

    /// The screen takes the size the window gives it. Without this the two scroll lists ask for
    /// about 1500 points of height and the window clips the top of the page.
    var body: some View {
        GeometryReader { geo in
            content.frame(width: geo.size.width, height: geo.size.height)
        }
    }

    private var content: some View {
        VStack(spacing: 0) {
            if fixed == nil {
                sectionBar
                Divider()
            }
            Group {
                switch fixed ?? model.section {
                case .schedule: scheduleTab
                case .dood: SchedDoodScreen(model: model)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(theme["bg"])
        .followsHubTab(model.section, fixed: fixed, set: { model.section = $0 }, place: { $0.place })
        .task(id: dataKey) { await model.reload() }
        .task(id: doodKey) { if model.section == .dood { await model.reloadDood() } }
        .sheet(item: $model.sheet) { ScheduleSheetHost(model: model, sheet: $0) }
        .sheet(item: $model.pendingKeep) { SchedKeepAnywaySheet(model: model, pending: $0) }
        .hubContainer("schedule-workspace", standalone: fixed == nil)
    }

    private var sectionBar: some View {
        SchedTabs(
            items: SchedSection.allCases.map { .init(value: $0, title: $0.title, symbol: nil, id: "schedule-section-\($0.rawValue)") },
            selection: $model.section, label: "Production", identifier: "schedule-sections")
            .frame(maxWidth: .infinity, alignment: .center)
            .padding(.horizontal, PageSpacing.medium).padding(.vertical, PageSpacing.small)
    }

    // MARK: Schedule tab

    @ViewBuilder private var scheduleTab: some View {
        if !model.loaded {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if model.view == nil, let f = model.loadFailure {
            ContentUnavailableView {
                Label("The schedule could not be loaded.", systemImage: "exclamationmark.triangle")
            } description: {
                Text(f.message)
            } actions: {
                Button("Try Again") { Task { await model.reload() } }.accessibilityIdentifier("schedule-retry")
            }
        } else if model.schedule == nil {
            noSchedule
        } else if let day = model.openDay {
            HStack(spacing: 0) {
                ScheduleDayView(model: model, day: day)
                drawer
            }
        } else {
            board
        }
    }

    private var noSchedule: some View {
        Group {
            if let src = model.view?.activeSource {
                ContentUnavailableView {
                    Label("Create a shooting schedule from your screenplay.", systemImage: "calendar")
                } description: {
                    Text("Break down your script first, then move scenes into shooting days. Production source: \(src.label) \u{00B7} \(Int(src.sceneCount)) scenes.")
                } actions: {
                    Button("Create Shooting Schedule") { model.sheet = .createSchedule }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("create-shooting-schedule")
                }
            } else {
                ContentUnavailableView {
                    Label("Create a shooting schedule from your screenplay.", systemImage: "calendar")
                } description: {
                    Text("Choose a Production Source in Breakdown first. The schedule is built from the scenes of that screenplay draft.")
                } actions: {
                    Button("Open Breakdown") {
                        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.breakdown.rawValue)
                    }
                    .accessibilityIdentifier("schedule-open-breakdown")
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("schedule-empty")
    }

    private var board: some View {
        HStack(spacing: 0) {
            VStack(spacing: PageSpacing.small) {
                banners
                toolbar
                if model.mode != .calendar { ScheduleFilterBar(model: model) }
                switch model.mode {
                case .board: ScheduleBoardView(model: model)
                case .list: ScheduleListView(model: model).padding(.horizontal, 16)
                case .calendar: ScheduleCalendarView(model: model)
                }
                if let h = model.view?.history, !h.isEmpty, model.mode == .list {
                    Text("\(SchedFormat.plural(h.count, "scene")) \(h.count == 1 ? "was" : "were") removed from the schedule after script changes (kept in schedule history).")
                        .font(.caption).foregroundStyle(theme["muted"]).multilineTextAlignment(.center).padding(.horizontal, PageSpacing.medium)
                        .accessibilityIdentifier("schedule-history-note")
                }
            }
            .frame(maxWidth: .infinity, alignment: .top)
            drawer
        }
    }

    @ViewBuilder private var drawer: some View {
        if let d = model.drawer { ScheduleDrawerHost(model: model, drawer: d) }
    }

    private var banners: some View {
        VStack(spacing: 6) {
            BDBanner(.info, model.sourceLine).accessibilityIdentifier("schedule-source-line")
            if model.view?.scriptChanges != nil {
                BDBanner(tone: .warn, content: {
                    Text("\(model.changesTitle). Your schedule has not been changed.")
                }, actions: {
                    Button("Review changes") { model.sheet = .scriptChanges }
                        .controlSize(.small).buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("review-script-changes")
                })
                .accessibilityIdentifier("script-changes-banner")
            }
            if model.isFinalized {
                BDBanner(tone: .ok, content: {
                    Text("This schedule is finalized. Reopen it in settings to make changes.")
                }, actions: {
                    Button("Reopen\u{2026}") { model.sheet = .settings }.controlSize(.small)
                        .accessibilityIdentifier("reopen-schedule")
                })
                .accessibilityIdentifier("finalized-banner")
            }
        }
        .padding(.horizontal, 16).padding(.top, 10)
    }

    private var toolbar: some View {
        CenteredWrap(spacing: 10, lineSpacing: PageSpacing.small) {
            SchedTabs(
                items: SchedMode.allCases.map { .init(value: $0, title: $0.title, symbol: $0.symbol, id: "schedule-mode-\($0.rawValue)") },
                selection: $model.mode, label: "Schedule view", identifier: "schedule-modes")
            Button { model.sheet = .day(.create(offDay: false, afterDayId: nil)) } label: { Label("Create Shooting Day", systemImage: "plus") }
                .buttonStyle(.borderedProminent)
                .disabled(model.isFinalized)
                .keyboardShortcut("n", modifiers: [.command, .shift])
                .accessibilityIdentifier("create-shooting-day")
            Button { model.sheet = .day(.create(offDay: true, afterDayId: nil)) } label: { Label("Off Day", systemImage: "moon") }
                .disabled(model.isFinalized)
                .accessibilityIdentifier("add-off-day")
            Button { model.drawer = .suggestions } label: { Label("Suggest Grouping", systemImage: "sparkles") }
                .accessibilityIdentifier("suggest-grouping")
            Button { model.drawer = .warnings } label: {
                Label(model.warningsLabel, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(model.openWarningCount > 0 ? theme["red-t"] : theme["green-t"])
            }
            .accessibilityLabel(model.warningsLabel)
            .accessibilityIdentifier("warnings-button")
            if model.schedule?.strictValidation == true {
                SchedChip(text: "Strict validation", tone: .purple).accessibilityIdentifier("strict-chip")
            }
            Button { model.sheet = .settings } label: { Label("Settings", systemImage: "gearshape") }
                .help("Schedule settings")
                .accessibilityLabel("Schedule settings")
                .accessibilityIdentifier("schedule-settings")
            Button { model.sheet = .export(.schedule(currentDayId: nil)) } label: { Label("Export", systemImage: "square.and.arrow.up") }
                .accessibilityIdentifier("schedule-export")
        }
        .padding(.horizontal, PageSpacing.medium)
    }
}

// MARK: Filters

struct ScheduleFilterBar: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let opts = model.view?.filterOptions ?? SchedFilterOptions()
        let total = model.allStrips.count
        let shown = model.matches?.count ?? total
        CenteredWrap(spacing: 8, lineSpacing: PageSpacing.small) {
            menu("Filter by location", $model.filter.location, [("", "All locations")] + opts.locations.map { ($0.value, $0.label) }, "filter-location")
            menu("Filter by cast", $model.filter.cast, [("", "All cast")] + opts.cast.map { ($0.value, $0.label) }, "filter-cast")
            menu("Filter by day or night", $model.filter.dayNight, [("", "Day and night"), ("D", "Day scenes"), ("N", "Night scenes")], "filter-day-night")
            menu(
                "Filter by scene length",
                Binding(get: { model.filter.length.rawValue }, set: { model.filter.length = ScheduleFilter.Length(rawValue: $0) ?? .any }),
                [("any", "Any length"), ("short", "Up to 1 page"), ("mid", "1 to 3 pages"), ("long", "Over 3 pages")], "filter-length")
            menu(
                "Filter by scheduled state",
                Binding(get: { model.filter.scheduled.rawValue }, set: { model.filter.scheduled = ScheduleFilter.Scheduled(rawValue: $0) ?? .any }),
                [("any", "Scheduled and unscheduled"), ("scheduled", "Scheduled"), ("unscheduled", "Unscheduled")], "filter-scheduled")
            menu(
                "Filter by breakdown state",
                Binding(get: { model.filter.breakdown.rawValue }, set: { model.filter.breakdown = ScheduleFilter.Breakdown(rawValue: $0) ?? .any }),
                [("any", "Any breakdown"), ("complete", "Breakdown complete"), ("incomplete", "Breakdown incomplete")], "filter-breakdown")
            menu("Filter by shooting day", $model.filter.day, [("", "All days")] + opts.days.map { ($0.value, $0.label) }, "filter-day")
            menu(
                "Filter by script revision",
                Binding(get: { model.filter.revision.rawValue }, set: { model.filter.revision = ScheduleFilter.Revision(rawValue: $0) ?? .any }),
                [("any", "Any revision state"), ("changed", "Changed in the script"), ("current", "Unchanged")], "filter-revision")
            if model.filter.isActive {
                Text("Showing \(shown) of \(total) scenes").font(.callout).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("filter-count")
                Button { model.filter = ScheduleFilter() } label: { Label("Clear filters", systemImage: "line.3.horizontal.decrease.circle") }
                    .controlSize(.small)
                    .accessibilityIdentifier("clear-filters")
            }
        }
        .controlSize(.small)
        .padding(.horizontal, PageSpacing.medium)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Filter scenes")
        .accessibilityIdentifier("schedule-filters")
    }

    private func menu(_ label: String, _ value: Binding<String>, _ options: [(String, String)], _ id: String) -> some View {
        Picker(label, selection: value) {
            ForEach(options, id: \.0) { Text($0.1).tag($0.0) }
        }
        .labelsHidden()
        .fixedSize()
        .frame(maxWidth: 190)
        .accessibilityLabel(label)
        .accessibilityIdentifier(id)
    }
}

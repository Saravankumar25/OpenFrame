// The list and the calendar of the schedule (UX 3.29, mocks 131 and 132). The list is a table of
// every scene with its day and estimated start; the calendar shows one month with the shooting days
// on their dates and the days without a date underneath.

import SwiftUI
import OpenFrameCore

// MARK: List

struct ScheduleListRow: Identifiable, Hashable {
    let strip: ScheduleStripDto
    let day: ScheduleShootDayDto?
    var id: String { strip.id }
}

extension ScheduleModel {
    /// The rows of the list: every scene by scene number, only the ones that match the filter.
    var listRows: [ScheduleListRow] {
        var out: [ScheduleListRow] = []
        for d in days { for i in d.items { if let s = i.strip { out.append(ScheduleListRow(strip: s, day: d)) } } }
        for s in unscheduled { out.append(ScheduleListRow(strip: s, day: nil)) }
        out.sort { ($0.strip.number ?? 1e9) < ($1.strip.number ?? 1e9) }
        if let m = matches { out = out.filter { m.contains($0.strip.id) } }
        return out
    }

    /// The estimated start of a scene on its day, when the day has times.
    func startLabel(_ row: ScheduleListRow) -> String {
        row.day?.timeline.rows.first { $0.itemId == row.strip.id }?.startLabel ?? ""
    }
}

struct ScheduleListView: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme
    @State private var selection: Set<String> = []

    var body: some View {
        // The empty line is a sibling of the table, not an overlay: an overlay of a table is not
        // reachable for a screen reader.
        ZStack {
            table
            if model.listRows.isEmpty {
                Text("No matching scenes.").foregroundStyle(theme["muted"]).accessibilityIdentifier("list-empty")
            }
        }
    }

    private var table: some View {
        let rows = model.listRows
        return Table(rows, selection: $selection) {
            TableColumn("Sc") { r in
                Text(r.strip.number.map { String(Int($0)) } ?? "-").fontWeight(.semibold)
                    .accessibilityIdentifier("list-scene-\(r.strip.number.map { String(Int($0)) } ?? r.id)")
            }
            .width(min: 36, ideal: 44, max: 60)
            TableColumn("I/E \u{00B7} D/N") { r in Text(r.strip.ieLabel) }
                .width(min: 60, ideal: 70, max: 90)
            TableColumn("Location") { r in
                if let l = r.strip.locationName { Text(l) } else { Text("Missing").foregroundStyle(theme["red-t"]) }
            }
            TableColumn("Day") { r in
                if let d = r.day { Text(d.title) } else { Text("Unscheduled").foregroundStyle(theme["muted"]) }
            }
            TableColumn("Start") { r in Text(model.startLabel(r)).monospacedDigit() }
                .width(min: 60, ideal: 70, max: 100)
                .alignment(.numeric)
            TableColumn("Pages") { r in Text(SchedFormat.pages(r.strip.pageEighths) + (r.strip.pagesOverridden ? "*" : "")).monospacedDigit() }
                .width(min: 50, ideal: 60, max: 90)
                .alignment(.numeric)
            TableColumn("Est. time") { r in
                if let e = r.strip.estimatedMinutes { Text(SchedFormat.minutes(e)).monospacedDigit() } else { Text("Missing").foregroundStyle(theme["muted"]) }
            }
            .width(min: 60, ideal: 80, max: 110)
            .alignment(.numeric)
            TableColumn("Cast") { r in Text(r.strip.cast.map(\.character).joined(separator: ", ")).lineLimit(1) }
            TableColumn("Actions") { r in SchedStripMenu(model: model, strip: r.strip) }
                .width(min: 50, ideal: 56, max: 70)
        }
        .contextMenu(forSelectionType: String.self) { ids in
            if ids.count == 1, let id = ids.first, let s = model.strip(id) { SchedStripMenuItems(model: model, strip: s) }
        } primaryAction: { ids in
            if let id = ids.first { model.drawer = .strip(id) }
        }
        .accessibilityLabel("Scenes of the schedule")
        .accessibilityIdentifier("schedule-list")
    }
}

// MARK: Calendar

struct ScheduleCalendarView: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme

    private let columns = Array(repeating: GridItem(.flexible(), spacing: 4), count: 7)

    var body: some View {
        let cells = SchedCalendarGrid.monthGrid(year: model.calendarYear, month0: model.calendarMonth0)
        let byDate = model.daysByDate
        let undated = model.days.filter { $0.date == nil }
        let monthLabel = SchedCalendarGrid.label(year: model.calendarYear, month0: model.calendarMonth0)
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button { model.shiftMonth(-1) } label: { Label("Previous", systemImage: "chevron.left") }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Previous month")
                        .accessibilityIdentifier("calendar-previous")
                    Text(monthLabel).font(.headline).frame(minWidth: 150)
                        .accessibilityIdentifier("calendar-month")
                    Button { model.shiftMonth(1) } label: { Label("Next", systemImage: "chevron.right") }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Next month")
                        .accessibilityIdentifier("calendar-next")
                }
                .frame(maxWidth: .infinity, alignment: .center)
                LazyVGrid(columns: columns, spacing: 4) {
                    ForEach(SchedCalendarGrid.weekdays, id: \.self) { w in
                        Text(w).font(.caption.weight(.bold)).foregroundStyle(theme["muted"]).frame(maxWidth: .infinity)
                    }
                    ForEach(cells, id: \.iso) { c in cell(c, byDate[c.iso] ?? []) }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Shooting calendar, \(monthLabel)")
                .accessibilityIdentifier("schedule-calendar")
                if !undated.isEmpty {
                    VStack(spacing: 6) {
                        Text("Not yet dated").font(.headline)
                        HStack {
                            Spacer(minLength: 0)
                            ForEach(undated, id: \.id) { d in
                                Button("Set date of \(d.label)") { model.sheet = .day(.date(d)) }
                                    .controlSize(.small)
                                    .accessibilityIdentifier("calendar-set-date-\(d.number.map { String(Int($0)) } ?? d.id)")
                            }
                            Spacer(minLength: 0)
                        }
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity)
                    .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
            }
            .padding(.horizontal, PageSpacing.medium).padding(.bottom, PageSpacing.medium)
        }
    }

    private func cell(_ c: SchedCalendarCell, _ days: [ScheduleShootDayDto]) -> some View {
        let off = !days.isEmpty && days.allSatisfy(\.isOffDay)
        let shoot = days.contains { !$0.isOffDay }
        return VStack(alignment: .leading, spacing: 2) {
            Text("\(c.day)").font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
            ForEach(days, id: \.id) { d in
                Button { model.openDayId = d.id } label: {
                    VStack(alignment: .leading, spacing: 1) {
                        if d.isOffDay {
                            Text("OFF").font(.caption.weight(.bold))
                        } else {
                            Text(d.label).font(.caption.weight(.bold))
                            Text(SchedFormat.plural(Int(d.summary.sceneCount), "scene")).font(.caption2)
                            if let cs = d.callSheet {
                                Text("Call sheet: \(SchedFormat.callSheetWord(cs.status))").font(.caption2).foregroundStyle(theme["muted"])
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel(d.title)
                .accessibilityIdentifier("calendar-day-\(d.number.map { String(Int($0)) } ?? d.id)")
            }
            Spacer(minLength: 0)
        }
        .padding(5)
        .frame(maxWidth: .infinity, minHeight: 92, alignment: .topLeading)
        .background(shoot ? theme["accent-soft"] : off ? theme["surface3"] : theme["panel"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).strokeBorder(theme["line"]))
        .opacity(c.inMonth ? 1 : 0.45)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("calendar-cell-\(c.iso)")
    }
}

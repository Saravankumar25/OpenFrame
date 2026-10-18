// The stripboard (UX 3.29): the Unscheduled pool on the left and the shooting days on the right.
// Drag a scene into a day or onto another scene to put it there; drop it in the pool to take it
// off its day. Every scene and day also has a menu and Option+Up and Option+Down, so nothing needs
// a mouse. The filters only fade strips here (positions stay exact). Nothing moves on its own.

import SwiftUI
import OpenFrameCore

struct ScheduleBoardView: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme
    @State private var hot: String?

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            pool.frame(width: 320)
            days
        }
        .padding(.horizontal, 16).padding(.bottom, 12)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Stripboard")
        .accessibilityIdentifier("schedule-board")
    }

    // MARK: Pool

    private var pool: some View {
        let dim = model.matches
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Unscheduled").font(.headline)
                SchedChip(text: "\(model.unscheduled.count)")
                    .accessibilityLabel("\(model.unscheduled.count) unscheduled")
                    .accessibilityIdentifier("pool-count")
                Spacer()
            }
            ScrollView {
                LazyVStack(spacing: 5) {
                    ForEach(model.unscheduled, id: \.id) { s in
                        SchedStripRow(strip: s, model: model, dim: dim.map { !$0.contains(s.id) } ?? false, warn: model.warnStripIds.contains(s.id), compact: true) {
                            model.drawer = .strip(s.id)
                        }
                        .draggable(SchedDrag.strip(s.id))
                    }
                    if model.unscheduled.isEmpty {
                        Text("Every scene is scheduled.").font(.callout).foregroundStyle(theme["muted"]).padding(8)
                            .accessibilityIdentifier("pool-empty")
                    }
                }
            }
        }
        .padding(10)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(hot == "pool" ? theme["blue"] : theme["line"], lineWidth: hot == "pool" ? 2 : 1))
        .dropDestination(for: String.self) { items, _ in drop(items, .pool) } isTargeted: { hot = $0 ? "pool" : (hot == "pool" ? nil : hot) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Unscheduled")
        .accessibilityIdentifier("schedule-pool")
    }

    // MARK: Days

    private var days: some View {
        ScrollView {
            LazyVStack(spacing: 10) {
                if model.days.isEmpty {
                    Text("Create a shooting day, then drag scenes into it.")
                        .font(.callout).foregroundStyle(theme["muted"])
                        .frame(maxWidth: .infinity).padding(20)
                        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line2"], style: StrokeStyle(lineWidth: 1, dash: [4, 3])))
                        .accessibilityIdentifier("board-no-days")
                }
                ForEach(model.days, id: \.id) { d in
                    SchedDayColumn(model: model, day: d, hot: $hot) { drop($0, $1) }
                }
            }
        }
    }

    private func drop(_ items: [String], _ target: SchedDropTarget) -> Bool {
        guard let payload = items.first else { return false }
        if model.dropPlan(payload: payload, target: target) == .none { return false }
        Task { await model.drop(payload: payload, on: target) }
        return true
    }
}

/// One shooting day: its name, call sheet and warning chips, the total, its scenes and breaks.
struct SchedDayColumn: View {
    @Bindable var model: ScheduleModel
    let day: ScheduleShootDayDto
    @Binding var hot: String?
    let drop: ([String], SchedDropTarget) -> Bool
    @Environment(\.ofTheme) private var theme

    private var key: String { day.number.map { String(Int($0)) } ?? day.id }

    var body: some View {
        let dim = model.matches
        VStack(alignment: .leading, spacing: 5) {
            header
            ForEach(day.items.indices, id: \.self) { i in
                let item = day.items[i]
                if let s = item.strip {
                    SchedStripRow(strip: s, model: model, dim: dim.map { !$0.contains(s.id) } ?? false, warn: model.warnStripIds.contains(s.id)) {
                        model.drawer = .strip(s.id)
                    }
                    .draggable(SchedDrag.strip(s.id))
                    .dropDestination(for: String.self) { items, _ in drop(items, .day(day.id, before: s.id)) }
                } else if let m = item.marker {
                    SchedMarkerRow(marker: m, model: model) {
                        model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: m.markerType, marker: m))
                    }
                    .draggable(SchedDrag.marker(m.id))
                    .dropDestination(for: String.self) { items, _ in drop(items, .day(day.id, before: m.id)) }
                }
            }
            if day.items.isEmpty {
                Text(ScheduleModel.emptyDayText(day))
                    .font(.callout).foregroundStyle(theme["muted"])
                    .frame(maxWidth: .infinity).padding(8)
                    .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).strokeBorder(theme["line2"], style: StrokeStyle(lineWidth: 1, dash: [4, 3])))
                    .accessibilityIdentifier("day-\(key)-empty")
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(day.isOffDay ? theme["surface3"] : theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(hot == day.id ? theme["blue"] : theme["line"], lineWidth: hot == day.id ? 2 : 1))
        .dropDestination(for: String.self) { items, _ in drop(items, .day(day.id, before: nil)) } isTargeted: { on in
            if on { hot = day.id } else if hot == day.id { hot = nil }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(model.columnText(day))
        .accessibilityIdentifier("day-column-\(key)")
    }

    private var header: some View {
        HStack(spacing: 8) {
            Image(systemName: "calendar").imageScale(.small).accessibilityHidden(true)
            Button { model.openDayId = day.id } label: {
                Text(day.title).font(.headline)
            }
            .buttonStyle(.plain)
            .accessibilityLabel(day.title)
            .accessibilityHint("Opens the details of this day")
            .accessibilityIdentifier("day-open-\(key)")
            if let cs = day.callSheet {
                let word = SchedFormat.callSheetWord(cs.status)
                SchedChip(text: "Call sheet: \(word)", tone: word == "Needs refresh" ? .yellow : word == "Final" ? .green : .plain)
                    .accessibilityIdentifier("day-\(key)-call-sheet")
            }
            if day.openWarnings > 0 {
                SchedChip(text: "\(Int(day.openWarnings))", tone: .red, symbol: "exclamationmark.triangle")
                    .help("Open warnings on this day")
                    .accessibilityLabel("\(Int(day.openWarnings)) open warnings on this day")
            }
            Spacer(minLength: 4)
            Text(day.isOffDay ? "Off day" : day.summary.totalLabel + (day.summary.overTarget ? " \u{00B7} over target" : ""))
                .font(.callout).foregroundStyle(theme["muted"])
                .accessibilityIdentifier("day-\(key)-total")
            SchedDayMenu(model: model, day: day)
        }
    }
}

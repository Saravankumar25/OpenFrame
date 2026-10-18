// One shooting day (UX 3.29, spec section 26.1): "What exactly are we doing on this day?" Its scenes
// and breaks in order, what they add up to, the day times (crew call, shooting start, target wrap),
// the computed timeline, the day target and the day notes. Derived values are shown, not edited.

import SwiftUI
import OpenFrameCore

struct ScheduleDayView: View {
    @Bindable var model: ScheduleModel
    let day: ScheduleShootDayDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            HStack(alignment: .top, spacing: 14) {
                ScrollView { items.padding(16) }
                ScrollView { side.padding(14) }
                    .frame(width: 330)
                    .background(theme["panel"])
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(day.title)
        .accessibilityIdentifier("day-detail")
    }

    // MARK: Header

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Button { model.openDayId = nil } label: { Label("Back to schedule", systemImage: "chevron.left") }
                .keyboardShortcut("[", modifiers: .command)
                .accessibilityIdentifier("day-back")
            VStack(alignment: .leading, spacing: 1) {
                Text(day.title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("day-title")
                Text(day.isOffDay ? "Off day: no scenes, notes allowed." : "What exactly are we doing on this day?")
                    .font(.callout).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("day-subtitle")
            }
            Spacer()
            Button("Edit date") { model.sheet = .day(.date(day)) }.accessibilityIdentifier("day-edit-date")
            Menu("Add break") {
                Button("Meal Break") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Meal")) }
                    .accessibilityIdentifier("day-add-meal")
                Button("Travel") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Travel")) }
                    .accessibilityIdentifier("day-add-travel")
                Button("Company Move") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Company Move")) }
                    .accessibilityIdentifier("day-add-company-move")
                Button("Custom Note") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Custom")) }
                    .accessibilityIdentifier("day-add-custom")
            }
            .menuStyle(.button).fixedSize()
            .disabled(model.isFinalized)
            .accessibilityIdentifier("day-add-break")
            SchedDayMenu(model: model, day: day, detail: true)
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
    }

    // MARK: Scenes and breaks

    private var items: some View {
        let s = day.summary
        let warnings = (model.view?.warnings ?? []).filter { $0.dayId == day.id && $0.decision == nil }
        return VStack(alignment: .leading, spacing: 6) {
            ForEach(warnings, id: \.key) { w in
                BDBanner(.warn, w.message).accessibilityIdentifier("day-warning")
            }
            if day.items.isEmpty {
                VStack(spacing: 8) {
                    Text(day.isOffDay ? "Off day." : "No scenes scheduled.").font(.headline)
                    if !day.isOffDay {
                        HStack {
                            Button("Add Scene") { model.openDayId = nil }.accessibilityIdentifier("day-add-scene")
                            Button("Off Day") { Task { await model.setOffDay(day, true) } }
                                .disabled(model.isFinalized).accessibilityIdentifier("day-make-off")
                        }
                    }
                }
                .frame(maxWidth: .infinity).padding(24)
                .accessibilityIdentifier("day-empty")
            }
            ForEach(day.items.indices, id: \.self) { i in
                let item = day.items[i]
                if let st = item.strip {
                    SchedStripRow(strip: st, model: model, selected: model.drawer == .strip(st.id)) { model.drawer = .strip(st.id) }
                } else if let m = item.marker {
                    SchedMarkerRow(marker: m, model: model) {
                        model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: m.markerType, marker: m))
                    }
                }
            }
            HStack(spacing: 8) {
                SchedChip(text: "Total estimate: \(s.estimatedMinutes > 0 ? SchedFormat.minutes(s.estimatedMinutes) : "Missing")")
                if s.missingEstimates > 0 { SchedChip(text: "\(Int(s.missingEstimates)) without estimate", tone: .yellow) }
                SchedChip(text: "\(s.pagesLabel) pages", tone: .plain)
                Spacer()
                Button {
                    model.sheet = .export(.schedule(currentDayId: day.id))
                } label: { Label("Export", systemImage: "square.and.arrow.up") }
                    .accessibilityIdentifier("day-export")
                if !day.isOffDay {
                    Button(day.callSheet != nil ? "Open Call Sheet" : "Create Call Sheet") { Task { await model.openCallSheet(day) } }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("day-call-sheet")
                }
            }
            .padding(.top, 8)
        }
    }

    // MARK: Side panel

    private var side: some View {
        let s = day.summary
        return VStack(alignment: .leading, spacing: 10) {
            Text("Derived from the scenes").font(.headline)
            labelled("Locations") {
                if s.locations.isEmpty {
                    Text("Missing").foregroundStyle(theme["red-t"])
                } else {
                    Text(s.locations.map { $0.name + ($0.status.map { " (\($0))" } ?? "") }.joined(separator: " \u{00B7} "))
                }
            }
            labelled("Cast") {
                Text(s.cast.isEmpty ? "None" : s.cast.map { $0.actor ?? $0.character }.joined(separator: " \u{00B7} "))
            }
            labelled("Estimated duration") {
                HStack {
                    Text("\(SchedFormat.minutes(s.estimatedMinutes)) of \(SchedFormat.minutes(s.targetMinutes)) target")
                    if s.overTarget { SchedChip(text: "Over target", tone: .red).accessibilityIdentifier("day-over-target") }
                }
            }
            SchedCommitField(
                id: "day-target", label: "Day target length", value: day.plannedMinutes.map { SchedFormat.minutes($0) } ?? "",
                placeholder: SchedFormat.minutes(model.schedule?.dayDurationMinutes ?? 600),
                hint: "Empty = the schedule default.", disabled: model.isFinalized
            ) { text in
                _ = await model.setDayTarget(day, text)
                return nil
            }
            if !day.isOffDay { SchedDayTimesView(model: model, day: day) }
            if let cs = day.callSheet {
                labelled("Call sheet") { Text(SchedFormat.callSheetWord(cs.status)).accessibilityIdentifier("day-call-sheet-status") }
            }
            Divider()
            Text("Day notes").font(.headline)
            SchedNotesField(id: "day-notes", label: "Day notes", value: day.notes ?? "", disabled: model.isFinalized) { text in
                await model.setDayNotes(day, text)
            }
            Text("Derived values are viewed here, not edited. Change scenes in the schedule.")
                .font(.caption).foregroundStyle(theme["muted"])
        }
    }

    private func labelled<C: View>(_ title: String, @ViewBuilder _ content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title).font(.callout.weight(.semibold))
            content().font(.callout)
        }
    }
}

/// Crew call, shooting start and target wrap, and the timeline computed from them (spec 26.1).
struct SchedDayTimesView: View {
    @Bindable var model: ScheduleModel
    let day: ScheduleShootDayDto
    @Environment(\.ofTheme) private var theme
    @State private var crew = ""
    @State private var start = ""
    @State private var wrap = ""
    @State private var syncKey = ""
    @FocusState private var focus: Field?
    private enum Field { case crew, start, wrap }

    private var fromDay: [String] {
        [day.crewCall.map { SchedFormat.clock($0) } ?? "", day.shootingStart.map { SchedFormat.clock($0) } ?? "", day.targetWrap.map { SchedFormat.clock($0) } ?? ""]
    }

    var body: some View {
        let tl = day.timeline
        VStack(alignment: .leading, spacing: 6) {
            Text("Day times").font(.callout.weight(.semibold))
            HStack(spacing: 6) {
                timeField("Crew call", $crew, .crew, "day-crew-call")
                timeField("Shooting start", $start, .start, "day-shooting-start")
                timeField("Target wrap", $wrap, .wrap, "day-target-wrap")
            }
            if tl.anchored {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(tl.rows.indices, id: \.self) { i in row(tl.rows[i]) }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Timeline of \(day.label)")
                .accessibilityIdentifier("day-timeline")
            } else {
                Text("Enter a crew call or shooting start to see estimated start times and the wrap.")
                    .font(.caption).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("day-timeline-hint")
            }
            if tl.overTargetWrap, let w = tl.estimatedWrapLabel {
                SchedChip(text: "Estimated wrap \(w) is after the target wrap", tone: .red)
                    .accessibilityIdentifier("day-over-target-wrap")
            }
            Text("An estimate from the scene estimates and breaks. A time earlier than the crew call counts as after midnight.")
                .font(.caption).foregroundStyle(theme["muted"])
        }
        .onAppear { sync(force: true) }
        .onChange(of: fromDay) { _, _ in if focus == nil { sync(force: true) } }
        .onChange(of: focus) { old, new in
            if old != nil, new == nil { Task { await save() } }
        }
    }

    private func timeField(_ title: String, _ text: Binding<String>, _ f: Field, _ id: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title).font(.caption.weight(.semibold))
            TextField("HH:MM", text: text)
                .textFieldStyle(.roundedBorder)
                .monospacedDigit()
                .focused($focus, equals: f)
                .disabled(model.isFinalized)
                .onSubmit { Task { await save() } }
                .accessibilityLabel("\(title) for \(day.label)")
                .accessibilityIdentifier(id)
        }
    }

    private func row(_ r: ScheduleTimelineRow) -> some View {
        let anchor = r.kind != "strip" && r.kind != "marker"
        return HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(r.startLabel ?? "").font(.callout.weight(.bold).monospacedDigit()).frame(minWidth: 76, alignment: .leading)
            Text(r.label + ((r.durationMinutes ?? 0) > 0 ? ", \(SchedFormat.minutes(r.durationMinutes ?? 0))" : "")).font(.callout)
            if r.missingEstimate { SchedChip(text: "No estimate", tone: .yellow) }
            if r.overlapMinutes > 0 { SchedChip(text: "Overlaps by \(SchedFormat.minutes(r.overlapMinutes))", tone: .red) }
            Spacer(minLength: 0)
            if r.gapMinutes > 0 { Text("\(SchedFormat.minutes(r.gapMinutes)) free before").font(.caption).foregroundStyle(theme["muted"]) }
        }
        .foregroundStyle(anchor ? theme["muted"] : theme["ink"])
        .padding(.vertical, 3)
        .accessibilityElement(children: .combine)
    }

    private func sync(force: Bool) {
        let v = fromDay
        crew = v[0]; start = v[1]; wrap = v[2]
    }

    private func save() async {
        _ = await model.setDayTimes(day, crewCall: crew, shootingStart: start, targetWrap: wrap)
        sync(force: true)
    }
}

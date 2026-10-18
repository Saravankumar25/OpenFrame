// Day Out of Days (spec section 28): a report of cast members by shooting days, derived from the
// cast confirmed in Breakdown and the scenes on each day. The person picks the cast and the day
// range and can correct a single cell. Corrections are the only stored data; they never change the
// screenplay, the breakdown or the schedule.

import SwiftUI
import OpenFrameCore

struct SchedDoodScreen: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            if !model.doodLoaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let v = model.dood {
                if !v.hasSchedule || v.allDays.isEmpty { empty } else { report(v) }
            } else if let f = model.doodFailure {
                ContentUnavailableView {
                    Label("The Day Out of Days could not be loaded.", systemImage: "exclamationmark.triangle")
                } description: {
                    Text(f.message)
                } actions: {
                    Button("Try Again") { Task { await model.reloadDood() } }.accessibilityIdentifier("dood-retry")
                }
            }
        }
        .accessibilityIdentifier("dood-tab")
    }

    private var empty: some View {
        ContentUnavailableView {
            Label("Day Out of Days comes from your shooting schedule.", systemImage: "calendar.day.timeline.left")
        } description: {
            Text("Create shooting days and schedule scenes with confirmed cast. The report then shows who works on which day.")
        } actions: {
            Button("Open Schedule") { model.section = .schedule }.accessibilityIdentifier("dood-open-schedule")
        }
        .accessibilityIdentifier("dood-empty")
    }

    private func report(_ v: ScheduleDoodView) -> some View {
        let corrected = model.correctedCount
        return VStack(alignment: .leading, spacing: 0) {
            toolbar(v, corrected)
            if v.staleCorrections > 0 {
                BDBanner(.warn, "\(SchedFormat.plural(Int(v.staleCorrections), "correction")) \(v.staleCorrections == 1 ? "is" : "are") kept for a cast member or day that is no longer in the schedule and not shown.")
                    .padding(.horizontal, 16).padding(.bottom, 8)
                    .accessibilityIdentifier("dood-stale")
            }
            if v.rows.isEmpty {
                Text("No cast is assigned to scheduled scenes yet. Confirm cast in Breakdown and schedule the scenes.")
                    .foregroundStyle(theme["muted"]).padding(24).frame(maxWidth: .infinity)
                    .accessibilityIdentifier("dood-no-cast")
                Spacer()
            } else {
                ScrollView([.horizontal, .vertical]) {
                    VStack(alignment: .leading, spacing: 8) {
                        grid(v)
                        Text(v.legend.map { "\($0.code) \($0.meaning.lowercased())" }.joined(separator: " \u{00B7} ") + ". * corrected by hand. Built from the cast confirmed in Breakdown and the scenes on each day; corrections never change the schedule.")
                            .font(.caption).foregroundStyle(theme["muted"])
                            .frame(maxWidth: 760, alignment: .leading)
                    }
                    .padding(16)
                }
            }
        }
    }

    private func toolbar(_ v: ScheduleDoodView, _ corrected: Int) -> some View {
        HStack(spacing: 10) {
            picker("Cast to show", $model.doodCast, [("", "All cast")] + v.allCast.map { ($0.key, $0.label) }, "dood-cast")
            picker("First day", $model.doodFrom, [("", "From the first day")] + v.allDays.map { ($0.id, $0.title) }, "dood-from")
            picker("Last day", $model.doodTo, [("", "To the last day")] + v.allDays.map { ($0.id, $0.title) }, "dood-to")
            Picker("Code style", selection: Binding(get: { v.codeStyle }, set: { s in Task { await model.setDoodStyle(s) } })) {
                Text("Codes: SW, W, WF, H").tag("industry")
                Text("Codes: W and -").tag("simple")
            }
            .labelsHidden().fixedSize()
            .accessibilityLabel("Code style")
            .accessibilityIdentifier("dood-code-style")
            Spacer()
            if corrected > 0 { SchedChip(text: "\(corrected) corrected by hand", tone: .yellow).accessibilityIdentifier("dood-corrected") }
            Button { model.sheet = .export(.dood) } label: { Label("Export", systemImage: "square.and.arrow.up") }
                .accessibilityIdentifier("dood-export")
        }
        .padding(.horizontal, 16).padding(.vertical, 8)
    }

    private func picker(_ label: String, _ value: Binding<String>, _ options: [(String, String)], _ id: String) -> some View {
        Picker(label, selection: value) {
            ForEach(options, id: \.0) { Text($0.1).tag($0.0) }
        }
        .labelsHidden().fixedSize()
        .accessibilityLabel(label)
        .accessibilityIdentifier(id)
    }

    // MARK: Grid

    private func grid(_ v: ScheduleDoodView) -> some View {
        Grid(horizontalSpacing: 0, verticalSpacing: 0) {
            GridRow {
                head("Cast", left: true)
                ForEach(v.days, id: \.id) { d in
                    VStack(spacing: 0) {
                        Text(d.shortLabel).font(.caption.weight(.semibold))
                        if let l = d.dateLabel {
                            Text(l.split(separator: " ").prefix(3).joined(separator: " ")).font(.caption2).foregroundStyle(theme["muted"])
                        }
                    }
                    .frame(minWidth: 52).padding(.vertical, 4).padding(.horizontal, 4)
                    .background(d.isOffDay ? theme["line"] : Color.clear)
                    .border(theme["line"], width: 0.5)
                    .help(d.title)
                    .accessibilityLabel(d.title)
                }
                head("Work"); head("Hold"); head("Travel"); head("Start"); head("Finish")
            }
            ForEach(v.rows, id: \.key) { r in
                GridRow {
                    Text("\(Int(r.number)). \(r.label)").font(.callout).lineLimit(1)
                        .frame(minWidth: 180, alignment: .leading).padding(.horizontal, 6).padding(.vertical, 3)
                        .border(theme["line"], width: 0.5)
                    ForEach(r.cells.indices, id: \.self) { i in
                        if i < v.days.count { cell(v, r, r.cells[i], v.days[i], row: Int(r.number), column: i + 1) }
                    }
                    stat(String(Int(r.workDays))); stat(String(Int(r.holdDays))); stat(String(Int(r.travelDays)))
                    stat(r.startLabel ?? ""); stat(r.finishLabel ?? "")
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Day Out of Days")
        .accessibilityIdentifier("dood-table")
    }

    private func head(_ t: String, left: Bool = false) -> some View {
        Text(t).font(.caption.weight(.semibold))
            .frame(minWidth: left ? 180 : 44, alignment: left ? .leading : .center).padding(.vertical, 4).padding(.horizontal, 6)
            .border(theme["line"], width: 0.5)
    }

    private func stat(_ t: String) -> some View {
        Text(t).font(.callout.monospacedDigit()).frame(minWidth: 44).padding(.vertical, 3).padding(.horizontal, 6)
            .border(theme["line"], width: 0.5)
    }

    private func cell(_ v: ScheduleDoodView, _ r: DoodRow, _ c: DoodCell, _ d: DoodDay, row: Int, column: Int) -> some View {
        var label = "\(r.label), \(d.title): " + (c.code.isEmpty ? "blank" : c.code)
        if c.overridden {
            label += ", corrected by hand (schedule says " + (c.derivedCode.isEmpty ? "blank" : c.derivedCode) + ")"
        }
        return Menu {
            Section("\(r.label), \(d.title)") {
                ForEach(v.legend, id: \.code) { l in
                    Button("\(l.code)  \(l.meaning)") { Task { await model.setDoodCode(r, c, d, code: l.code) } }
                        .disabled(l.code == c.code || model.isFinalized)
                        .accessibilityIdentifier("dood-code-\(l.code)")
                }
            }
            Divider()
            Button(c.derivedCode.isEmpty ? "Use the schedule value (blank)" : "Use the schedule value (\(c.derivedCode))") {
                Task { await model.setDoodCode(r, c, d, code: nil) }
            }
            .disabled(!c.overridden)
            .accessibilityIdentifier("dood-use-schedule")
            Button(c.note != nil ? "Edit the note" : "Add a note") {
                model.sheet = .note(SchedDoodNoteTarget(row: r, cell: c, dayTitle: d.title))
            }
            .disabled(!c.overridden)
            .accessibilityIdentifier("dood-note")
        } label: {
            Text((c.code.isEmpty ? " " : c.code) + (c.overridden ? "*" : ""))
                .font(.callout.weight(.bold))
                .underline(c.overridden, pattern: .dot)
                .frame(minWidth: 36)
        }
        .menuStyle(.borderlessButton).menuIndicator(.hidden)
        .frame(minWidth: 52).padding(.vertical, 2)
        .background(d.isOffDay ? theme["line"] : (c.overridden ? theme["yellow-soft"] : Color.clear))
        .border(theme["line"], width: 0.5)
        .help(c.note ?? label)
        .accessibilityLabel(c.note.map { "\(label), note: \($0)" } ?? label)
        .accessibilityIdentifier("dood-cell-\(row)-\(column)")
    }
}

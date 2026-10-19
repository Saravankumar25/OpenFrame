// The sheets of the Schedule workspace (UX 3.29, mocks 129, 134 to 139): create the schedule, add a
// shooting or off day, edit a date, add or edit a break, settings, review script changes, Keep
// Anyway, group scenes, export, and the Day Out of Days correction note.

import AppKit
import SwiftUI
import OpenFrameCore

struct ScheduleSheetHost: View {
    @Bindable var model: ScheduleModel
    let sheet: SchedSheet

    var body: some View {
        switch sheet {
        case .createSchedule: SchedCreateSheet(model: model)
        case let .day(mode): SchedDaySheet(model: model, mode: mode)
        case let .marker(mode): SchedMarkerSheet(model: model, mode: mode)
        case .settings: SchedSettingsSheet(model: model)
        case .scriptChanges: SchedScriptChangesSheet(model: model)
        case let .export(kind): SchedExportSheet(model: model, kind: kind)
        case let .note(target): SchedDoodNoteSheet(model: model, target: target)
        case let .group(s): SchedGroupSheet(model: model, suggestion: s)
        }
    }
}

private struct CancelButton: View {
    let action: () -> Void
    var body: some View {
        Button("Cancel", action: action).keyboardShortcut(.cancelAction).accessibilityIdentifier("sheet-cancel")
    }
}

// MARK: Create schedule

struct SchedCreateSheet: View {
    @Bindable var model: ScheduleModel

    var body: some View {
        let src = model.view?.activeSource
        BDSheetFrame(title: "Create Shooting Schedule", width: 480) {
            if let src {
                BDField(label: "Screenplay source") {
                    Text("\(src.screenplayTitle), \(src.label) \u{00B7} \(Int(src.sceneCount)) scenes")
                        .padding(6).frame(maxWidth: .infinity, alignment: .leading)
                        .background(.quaternary, in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                        .accessibilityIdentifier("create-schedule-source")
                }
                BDBanner(.info, "All \(Int(src.sceneCount)) scenes will start in the Unscheduled pool. Scenes that need a location or cast will show it from your Breakdown once available.")
            }
        } footer: {
            CancelButton { model.sheet = nil }
            Button("Create Schedule") { Task { await model.createSchedule() } }
                .keyboardShortcut(.defaultAction)
                .disabled(model.busy || src == nil)
                .accessibilityIdentifier("create-schedule-confirm")
        }
    }
}

// MARK: Day

struct SchedDaySheet: View {
    @Bindable var model: ScheduleModel
    let mode: SchedDayDialogMode
    @State private var date: String
    @State private var notes = ""
    @State private var error: String?
    @State private var busy = false

    init(model: ScheduleModel, mode: SchedDayDialogMode) {
        self.model = model
        self.mode = mode
        if case let .date(d) = mode { _date = State(initialValue: d.date ?? "") } else { _date = State(initialValue: "") }
    }

    private var creating: Bool { if case .create = mode { true } else { false } }
    private var offDay: Bool { if case let .create(off, _) = mode { off } else { false } }

    private var title: String {
        switch mode {
        case let .create(off, _): off ? "Add Off Day" : "Create Shooting Day"
        case let .date(d): "Edit date of \(d.label)"
        }
    }

    var body: some View {
        BDSheetFrame(title: title, width: 420) {
            BDField(label: "Shooting date", hint: "Optional. A day can exist without a date. Write it as YYYY-MM-DD.") {
                HStack {
                    TextField("YYYY-MM-DD", text: $date)
                        .textFieldStyle(.roundedBorder)
                        .monospacedDigit()
                        .onSubmit { Task { await save() } }
                        .accessibilityLabel("Shooting date")
                        .accessibilityIdentifier("day-date")
                    DatePicker("Pick a date", selection: pickerBinding, displayedComponents: .date)
                        .labelsHidden()
                        .accessibilityLabel("Pick a date")
                        .accessibilityIdentifier("day-date-picker")
                }
            }
            if case let .date(d) = mode, d.callSheet != nil {
                Text("Changing the date marks this day's call sheet as needing a refresh. The call sheet itself is not rewritten.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            if creating {
                BDField(label: offDay ? "Note" : "Day notes") {
                    TextField("", text: $notes, axis: .vertical)
                        .lineLimit(3...6)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel(offDay ? "Note" : "Day notes")
                        .accessibilityIdentifier("day-new-notes")
                }
            }
            if let error {
                Text(error).font(.callout).foregroundStyle(.red).accessibilityIdentifier("day-sheet-error")
            }
        } footer: {
            CancelButton { model.sheet = nil }
            Button(creating ? (offDay ? "Add Off Day" : "Create Day") : "Save date") { Task { await save() } }
                .keyboardShortcut(.defaultAction)
                .disabled(busy)
                .accessibilityIdentifier("day-sheet-confirm")
        } leading: {
            if case let .date(d) = mode, d.date != nil {
                Button("Clear date") { date = "" }.accessibilityIdentifier("day-clear-date")
            }
        }
    }

    private var pickerBinding: Binding<Date> {
        Binding(
            get: { SchedDate.parse(date) ?? Date() },
            set: { date = SchedDate.iso($0) })
    }

    private func save() async {
        busy = true
        defer { busy = false }
        switch mode {
        case let .create(off, after): error = await model.createDay(date: date, notes: notes, offDay: off, afterDayId: after)
        case let .date(d): error = await model.setDayDate(d, date: date)
        }
    }
}

// MARK: Break

struct SchedMarkerSheet: View {
    @Bindable var model: ScheduleModel
    let mode: SchedMarkerDialogMode
    @State private var type: String
    @State private var label: String
    @State private var time: String
    @State private var duration: String
    @State private var notes: String
    @State private var error: String?

    static let types: [(value: String, label: String)] = [("Meal", "Meal Break"), ("Travel", "Travel"), ("Company Move", "Company Move"), ("Custom", "Custom Note")]

    init(model: ScheduleModel, mode: SchedMarkerDialogMode) {
        self.model = model
        self.mode = mode
        let m = mode.marker
        _type = State(initialValue: m?.markerType ?? mode.type)
        _label = State(initialValue: m?.label ?? "")
        _time = State(initialValue: m?.atTime ?? "")
        _duration = State(initialValue: (m?.durationMinutes ?? 0) > 0 ? SchedFormat.minutes(m?.durationMinutes ?? 0) : "")
        _notes = State(initialValue: m?.notes ?? "")
    }

    var body: some View {
        BDSheetFrame(title: mode.marker != nil ? "Edit break" : "Add to this day", width: 440) {
            BDField(label: "Type") {
                Picker("Type", selection: $type) {
                    ForEach(Self.types, id: \.value) { Text($0.label).tag($0.value) }
                }
                .labelsHidden()
                .accessibilityLabel("Type")
                .accessibilityIdentifier("marker-type")
            }
            BDField(label: type == "Custom" ? "Note" : "Label", hint: type == "Custom" ? nil : "Optional. Defaults to the type.") {
                TextField("", text: $label)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel(type == "Custom" ? "Note" : "Label")
                    .accessibilityIdentifier("marker-label")
            }
            HStack(alignment: .top) {
                BDField(label: "Time", hint: "Optional, e.g. 20:30") {
                    TextField("HH:MM", text: $time)
                        .textFieldStyle(.roundedBorder).monospacedDigit()
                        .accessibilityLabel("Time")
                        .accessibilityIdentifier("marker-time")
                }
                BDField(label: "Duration", hint: "Counts toward the day estimate") {
                    TextField("45m", text: $duration)
                        .textFieldStyle(.roundedBorder)
                        .onSubmit { Task { await save() } }
                        .accessibilityLabel("Duration")
                        .accessibilityIdentifier("marker-duration")
                }
            }
            BDField(label: "Notes") {
                TextField("", text: $notes, axis: .vertical)
                    .lineLimit(2...5)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Notes")
                    .accessibilityIdentifier("marker-notes")
            }
            if let error {
                Text(error).font(.callout).foregroundStyle(.red).accessibilityIdentifier("marker-error")
            }
        } footer: {
            CancelButton { model.sheet = nil }
            Button(mode.marker != nil ? "Save" : "Add") { Task { await save() } }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("marker-save")
        } leading: {
            if let m = mode.marker {
                Button("Delete", role: .destructive) { Task { await model.deleteMarker(m) } }
                    .accessibilityIdentifier("marker-delete")
            }
        }
    }

    private func save() async {
        error = await model.saveMarker(mode, type: type, label: label, time: time, duration: duration, notes: notes)
    }
}

// MARK: Settings

struct SchedSettingsSheet: View {
    @Bindable var model: ScheduleModel
    @State private var name = ""
    @State private var length = ""
    @State private var strict = false
    @State private var status = "Draft"
    @State private var error: String?
    @State private var loaded = false

    var body: some View {
        BDSheetFrame(title: "Schedule settings", width: 480) {
            BDField(label: "Name") {
                TextField("", text: $name).textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Name").accessibilityIdentifier("settings-name")
            }
            BDField(label: "Target day length", hint: "Days whose estimates exceed this show a warning. Each day can override it.") {
                TextField("10h", text: $length).textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Target day length").accessibilityIdentifier("settings-length")
            }
            BDField(label: "Status") {
                Picker("Schedule status", selection: $status) {
                    Text("Draft").tag("Draft")
                    Text("Active").tag("Active")
                    Text("Finalized (no further changes until reopened)").tag("Finalized")
                }
                .labelsHidden()
                .accessibilityLabel("Schedule status")
                .accessibilityIdentifier("settings-status")
            }
            Toggle("Strict validation: ask before keeping a change that creates a warning", isOn: $strict)
                .accessibilityIdentifier("settings-strict")
            Text("Warnings never block you by default. With strict validation, a change that creates a new warning needs an explicit \u{201C}Keep Anyway\u{201D}.")
                .font(.caption).foregroundStyle(.secondary)
            if let error {
                Text(error).font(.callout).foregroundStyle(.red).accessibilityIdentifier("settings-error")
            }
        } footer: {
            CancelButton { model.sheet = nil }
            Button("Save") { Task { await save() } }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("settings-save")
        }
        .onAppear {
            guard !loaded, let s = model.schedule else { return }
            loaded = true
            name = s.name
            length = SchedFormat.minutes(s.dayDurationMinutes)
            strict = s.strictValidation
            status = s.status
        }
    }

    private func save() async {
        error = await model.saveSettings(name: name, length: length, strict: strict, status: status, doodStyle: nil)
        if error == nil { model.sheet = nil }
    }
}

// MARK: Script changes

struct SchedScriptChangesSheet: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let c = model.view?.scriptChanges
        BDSheetFrame(
            title: model.changesTitle,
            sub: "The production source script changed. Your schedule has not been changed.", width: 640
        ) {
            if let c {
                if c.sourceSwitched, let n = c.newSource {
                    BDBanner(.info, "New production source: \(n.label). Scheduled scenes keep their days.")
                }
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        HStack {
                            Text("Sc").frame(width: 40, alignment: .leading)
                            Text("Heading").frame(maxWidth: .infinity, alignment: .leading)
                            Text("Change").frame(width: 200, alignment: .leading)
                            Text("Scheduled").frame(width: 120, alignment: .leading)
                        }
                        .font(.caption.weight(.semibold)).foregroundStyle(theme["muted"]).padding(.vertical, 4)
                        Divider()
                        ForEach(c.changed + c.removed, id: \.stripId) { r in
                            HStack(alignment: .top) {
                                Text(r.number.map { String(Int($0)) } ?? "-").frame(width: 40, alignment: .leading)
                                Text(r.heading).frame(maxWidth: .infinity, alignment: .leading)
                                VStack(alignment: .leading, spacing: 3) {
                                    ForEach(r.kinds, id: \.self) { k in SchedChip(text: k, tone: k.hasPrefix("Removed") ? .red : .yellow) }
                                }
                                .frame(width: 200, alignment: .leading)
                                Text(r.dayLabel ?? "Unscheduled").frame(width: 120, alignment: .leading)
                            }
                            .font(.callout).padding(.vertical, 5)
                            .accessibilityElement(children: .combine)
                            .accessibilityIdentifier("script-change-\(r.stripId)")
                            Divider()
                        }
                        if c.newScenes > 0 {
                            HStack {
                                Text("-").frame(width: 40, alignment: .leading)
                                Text(SchedFormat.plural(Int(c.newScenes), "new scene")).frame(maxWidth: .infinity, alignment: .leading)
                                SchedChip(text: "New scene", tone: .blue).frame(width: 200, alignment: .leading)
                                Text("Unscheduled").frame(width: 120, alignment: .leading)
                            }
                            .font(.callout).padding(.vertical, 5)
                        }
                    }
                }
                .frame(maxHeight: 280)
                Text("New scenes appear in Unscheduled. Removed scenes stay marked in the schedule history until you confirm removal. \u{201C}Update Schedule\u{201D} only flags changes; it never moves or deletes a scheduled scene.")
                    .font(.caption).foregroundStyle(theme["muted"])
            }
        } footer: {
            Button("Close") { model.sheet = nil }.keyboardShortcut(.cancelAction).accessibilityIdentifier("script-changes-close")
            Button("Update Schedule") { Task { await model.updateFromScript() } }
                .keyboardShortcut(.defaultAction)
                .disabled(model.busy)
                .accessibilityIdentifier("script-changes-update")
        }
    }
}

// MARK: Keep Anyway

struct SchedKeepAnywaySheet: View {
    @Bindable var model: ScheduleModel
    let pending: SchedPendingKeep

    var body: some View {
        BDSheetFrame(title: "This change creates a schedule warning", width: 460) {
            Text(pending.message).fixedSize(horizontal: false, vertical: true).accessibilityIdentifier("keep-anyway-message")
            Text("Strict validation is on for this schedule. Nothing was changed yet; the choice is yours.")
                .font(.caption).foregroundStyle(.secondary)
        } footer: {
            Button("Change the schedule instead") { model.cancelKeep() }
                .keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("keep-anyway-cancel")
            Button("Keep Anyway") { Task { await model.confirmKeep() } }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("keep-anyway-confirm")
        }
    }
}

// MARK: Group scenes

struct SchedGroupSheet: View {
    @Bindable var model: ScheduleModel
    let suggestion: ScheduleGroupingSuggestion
    @State private var target = ""

    var body: some View {
        let days = model.shootingDays
        BDSheetFrame(title: "Group these scenes?", width: 440) {
            Text("Scenes \(suggestion.sceneNumbers.map { String(Int($0)) }.joined(separator: ", ")) will be moved to the chosen day, after its current scenes. Nothing else changes and you can undo it.")
                .fixedSize(horizontal: false, vertical: true)
            BDField(label: "Shooting day") {
                Picker("Shooting day", selection: $target) {
                    ForEach(days, id: \.id) { Text($0.title).tag($0.id) }
                }
                .labelsHidden()
                .accessibilityLabel("Shooting day")
                .accessibilityIdentifier("group-day")
            }
        } footer: {
            CancelButton { model.sheet = nil }
            Button("Move scenes") { Task { await move() } }
                .keyboardShortcut(.defaultAction)
                .disabled(target.isEmpty)
                .accessibilityIdentifier("group-confirm")
        }
        .onAppear { if target.isEmpty { target = suggestion.suggestedDayId ?? days.first?.id ?? "" } }
    }

    private func move() async {
        guard let day = model.day(target) else { return }
        let ids = suggestion.stripIds.filter { id in !day.items.contains { $0.strip?.id == id } }
        model.sheet = nil
        if !ids.isEmpty { await model.moveStrips(ids, to: target, text: "Grouped scenes on \(day.label)") }
    }
}

// MARK: Day Out of Days note

struct SchedDoodNoteSheet: View {
    @Bindable var model: ScheduleModel
    let target: SchedDoodNoteTarget
    @State private var text: String

    init(model: ScheduleModel, target: SchedDoodNoteTarget) {
        self.model = model
        self.target = target
        _text = State(initialValue: target.cell.note ?? "")
    }

    var body: some View {
        BDSheetFrame(title: "Correction note", sub: "\(target.row.label), \(target.dayTitle): \(target.cell.code)", width: 440) {
            BDField(label: "Note", hint: "Optional. Up to 500 characters.") {
                TextField("", text: $text, axis: .vertical)
                    .lineLimit(3...6)
                    .textFieldStyle(.roundedBorder)
                    .onChange(of: text) { _, new in if new.count > 500 { text = String(new.prefix(500)) } }
                    .accessibilityLabel("Note")
                    .accessibilityIdentifier("dood-note-field")
            }
        } footer: {
            CancelButton { model.sheet = nil }
            Button("Save") { Task { _ = await model.saveDoodNote(target, text: text) } }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("dood-note-save")
        }
    }
}

// MARK: Export

struct SchedExportSheet: View {
    @Bindable var model: ScheduleModel
    let kind: SchedExportKind
    enum Scope: String { case all, day, days }
    @State private var scope: Scope = .all
    @State private var picked: Set<String> = []
    @State private var format = "pdf"
    @State private var includeUnscheduled = true
    @State private var includeNotes = false
    @State private var result: ExportResult?
    @Environment(\.ofTheme) private var theme
    @Environment(AppModel.self) private var app

    private var currentDay: ScheduleShootDayDto? {
        if case let .schedule(id) = kind { return model.day(id) }
        return nil
    }
    private var isSchedule: Bool { if case .schedule = kind { true } else { false } }
    private var title: String { isSchedule ? "Export Shooting Schedule" : "Export Day Out of Days" }
    private var documentName: String { isSchedule ? "Shooting Schedule" : "Day Out of Days" }
    private var projectTitle: String { if case let .project(p) = app.phase { p.title } else { "OpenFrame" } }
    private var suggestedName: String { "\(projectTitle) - \(documentName).\(format)" }
    private var scopeOk: Bool { scope != .days || !picked.isEmpty }

    var body: some View {
        if let result { done(result) } else { form }
    }

    private var form: some View {
        BDSheetFrame(title: title, width: 500) {
            if isSchedule {
                Text("Source: Shooting Schedule" + (model.schedule?.source.map { " \u{00B7} \($0.label)" } ?? ""))
                    .font(.callout).accessibilityIdentifier("export-source")
            } else {
                Text("Source: Day Out of Days \u{00B7} the cast and days shown").font(.callout).accessibilityIdentifier("export-source")
            }
            if isSchedule { scopes }
            GroupBox("Format") {
                HStack(spacing: 14) {
                    ForEach([("pdf", "PDF"), ("xlsx", "Spreadsheet (XLSX)"), ("csv", "CSV")], id: \.0) { f in
                        Button {
                            format = f.0
                        } label: {
                            Label(f.1, systemImage: format == f.0 ? "largecircle.fill.circle" : "circle")
                        }
                        .buttonStyle(.plain)
                        .accessibilityAddTraits(format == f.0 ? [.isButton, .isSelected] : .isButton)
                        .accessibilityIdentifier("export-format-\(f.0)")
                    }
                    Spacer()
                }
                .frame(maxWidth: .infinity, alignment: .leading).padding(4)
            }
            Text("Suggested file name: \(suggestedName)").font(.caption).foregroundStyle(theme["muted"])
                .accessibilityIdentifier("export-file-name")
            if isSchedule {
                GroupBox("Options") {
                    VStack(alignment: .leading, spacing: 6) {
                        Toggle("Include unscheduled scenes", isOn: $includeUnscheduled).accessibilityIdentifier("export-include-unscheduled")
                        Toggle("Include scene notes", isOn: $includeNotes).accessibilityIdentifier("export-include-notes")
                    }
                    .frame(maxWidth: .infinity, alignment: .leading).padding(4)
                }
            }
            BDBanner(.info, isSchedule
                ? "Private notes excluded. Export is a snapshot of the schedule; the schedule is not changed."
                : "Corrections are marked with *. Export is a snapshot; the schedule is not changed.")
        } footer: {
            CancelButton { model.sheet = nil }
            Button(model.busy ? "Exporting\u{2026}" : "Export\u{2026}") { Task { await run() } }
                .keyboardShortcut(.defaultAction)
                .disabled(model.busy || !scopeOk)
                .accessibilityIdentifier("export-confirm")
        }
        .onAppear { if currentDay != nil { scope = .day } }
    }

    private var scopes: some View {
        GroupBox("What are you exporting?") {
            VStack(alignment: .leading, spacing: 6) {
                radio(.all, "Entire schedule", "export-scope-all", enabled: true)
                radio(.day, currentDay.map { "Current day only (\($0.label))" } ?? "Current day only", "export-scope-day", enabled: currentDay != nil)
                radio(.days, "Selected days", "export-scope-days", enabled: true)
                if scope == .days {
                    if model.days.isEmpty {
                        Text("Create a shooting day first.").font(.caption).foregroundStyle(theme["muted"])
                    }
                    ForEach(model.days, id: \.id) { d in
                        Toggle(d.title, isOn: Binding(get: { picked.contains(d.id) }, set: { on in if on { picked.insert(d.id) } else { picked.remove(d.id) } }))
                            .padding(.leading, 20)
                            .accessibilityIdentifier("export-day-\(d.number.map { String(Int($0)) } ?? d.id)")
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading).padding(4)
        }
    }

    private func radio(_ s: Scope, _ text: String, _ id: String, enabled: Bool) -> some View {
        Button {
            scope = s
            picked = []
        } label: {
            Label(text, systemImage: scope == s ? "largecircle.fill.circle" : "circle")
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .accessibilityAddTraits(scope == s ? [.isButton, .isSelected] : .isButton)
        .accessibilityIdentifier(id)
    }

    private func run() async {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = suggestedName
        panel.message = "Choose where to save the \(documentName)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        if isSchedule {
            let ids: [String]? = switch scope {
            case .all: nil
            case .day: currentDay.map { [$0.id] }
            case .days: model.days.map(\.id).filter { picked.contains($0) }
            }
            result = await model.exportSchedule(format: format, path: url.path, dayIds: ids, includeUnscheduled: includeUnscheduled, includeNotes: includeNotes)
        } else {
            result = await model.exportDood(format: format, path: url.path)
        }
    }

    private func done(_ r: ExportResult) -> some View {
        BDSheetFrame(title: "Export complete", width: 500) {
            BDBanner(.ok, "\(r.fileName) was saved.")
            VStack(alignment: .leading, spacing: 4) {
                row("Document", r.document)
                row("Source", r.sourceLabel)
                row("Scope", r.scopeLabel)
                if !r.contentsLabel.isEmpty { row("Contents", r.contentsLabel) }
                row("Format", r.format.uppercased())
                if let p = r.pages { row("Pages", String(Int(p))) }
                row("Location", r.path)
            }
            .font(.callout)
            ForEach(r.warnings, id: \.self) { BDBanner(.warn, $0) }
            BDBanner(.info, "Private notes excluded. Export creates a snapshot; your project does not change.")
        } footer: {
            Button("Reveal in Finder") { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: r.path)]) }
                .accessibilityIdentifier("export-reveal")
            Button("Close") { model.sheet = nil }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("export-close")
        }
    }

    private func row(_ k: String, _ v: String) -> some View {
        HStack(alignment: .top) {
            Text(k).foregroundStyle(theme["muted"]).frame(width: 80, alignment: .leading)
            Text(v).textSelection(.enabled)
        }
    }
}

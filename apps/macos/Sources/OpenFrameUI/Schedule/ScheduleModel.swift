// The Schedule workspace's model: the shooting schedule as the core gives it (stripboard, list,
// calendar, shooting days), the Day Out of Days report, the filters, the open sheets and drawers,
// and every change. It mirrors apps/desktop/src/workspaces/production/tabs/schedule.tsx and
// dood.tsx and the shared call-sheets/shared/ views, and sends each change to a `schedule.*`
// operation. The core decides and does the moving; nothing here stores production data, and
// nothing here ever moves a scene on its own (suggestions are advice, moves are always asked for).

import Foundation
import Observation
import OpenFrameCore

/// The two parts of the workspace: Windows has them as the Production tabs Schedule and Day Out of Days.
enum SchedSection: String, CaseIterable, Identifiable {
    case schedule, dood
    var id: String { rawValue }
    var title: String { self == .schedule ? "Schedule" : "Day Out of Days" }
}

enum SchedMode: String, CaseIterable, Identifiable {
    case board, list, calendar
    var id: String { rawValue }
    var title: String {
        switch self {
        case .board: "Board"
        case .list: "List"
        case .calendar: "Calendar"
        }
    }
    var symbol: String {
        switch self {
        case .board: "rectangle.split.3x1"
        case .list: "list.bullet"
        case .calendar: "calendar"
        }
    }
}

/// The tables the schedule projections read (drives reloads).
enum SchedTables {
    static let all = [
        "shooting_schedule", "shooting_day", "schedule_strip", "schedule_marker", "schedule_warning_ack",
        "dood_override", "call_sheet", "production_source", "screenplay", "screenplay_draft", "screenplay_scene",
        "screenplay_element", "breakdown_element", "catalog_item", "location", "cast_member", "story_character", "project",
    ]
}

enum SchedDayDialogMode: Equatable {
    case create(offDay: Bool, afterDayId: String?)
    case date(ScheduleShootDayDto)
}

struct SchedMarkerDialogMode: Equatable {
    var dayId: String
    /// Meal / Travel / Company Move / Custom.
    var type: String
    var marker: ScheduleMarkerDto?
}

struct SchedDoodNoteTarget: Equatable {
    var row: DoodRow
    var cell: DoodCell
    var dayTitle: String
}

enum SchedExportKind: Equatable {
    case schedule(currentDayId: String?)
    case dood
}

enum SchedSheet: Identifiable, Equatable {
    case createSchedule
    case day(SchedDayDialogMode)
    case marker(SchedMarkerDialogMode)
    case settings
    case scriptChanges
    case export(SchedExportKind)
    case note(SchedDoodNoteTarget)
    case group(ScheduleGroupingSuggestion)

    var id: String {
        switch self {
        case .createSchedule: "create-schedule"
        case let .day(m):
            switch m {
            case let .create(off, after): "day-create-\(off)-\(after ?? "end")"
            case let .date(d): "day-date-\(d.id)"
            }
        case let .marker(m): "marker-\(m.marker?.id ?? m.dayId + m.type)"
        case .settings: "settings"
        case .scriptChanges: "script-changes"
        case let .export(k): "export-\(k)"
        case let .note(t): "note-\(t.row.key)-\(t.cell.dayId)"
        case let .group(s): "group-\(s.key)"
        }
    }
}

enum SchedDrawer: Equatable {
    case strip(String)
    case warnings
    case suggestions
}

/// A move that strict validation refused, waiting for "Keep Anyway".
struct SchedPendingKeep: Identifiable {
    enum Move {
        case strip(ScheduleStripMoveArgs)
        case strips(ScheduleStripsMoveArgs)
    }
    let id = UUID()
    var message: String
    var move: Move
    var doneText: String
}

/// What a drop on the board asks for. The board shows nothing until the core has done it.
enum SchedDropPlan: Equatable {
    case none
    case refuse(String)
    case moveStrip(id: String, dayId: String?, index: Int?, text: String)
    case moveMarker(id: String, dayId: String, index: Int?, text: String)
}

/// A place on the board where a scene or break can be dropped.
enum SchedDropTarget: Equatable {
    case pool
    /// A day column. `before` is the item the drop landed on (nil = after the last item).
    case day(String, before: String?)
}

/// The scene and break menus: the keyboard and menu alternative to dragging.
struct SchedStripMoveMenu {
    struct DayChoice: Identifiable {
        let day: ScheduleShootDayDto
        let disabled: Bool
        var id: String { day.id }
    }
    var days: [DayChoice]
    var canMoveUp: Bool
    var canMoveDown: Bool
    var isScheduled: Bool
}

enum SchedDrag {
    static let stripPrefix = "openframe-schedule-strip:"
    static let markerPrefix = "openframe-schedule-marker:"
    static func strip(_ id: String) -> String { stripPrefix + id }
    static func marker(_ id: String) -> String { markerPrefix + id }
}

extension Notification.Name {
    /// Posted after "Create Call Sheet" or "Open Call Sheet" on a day. The object is the call sheet id.
    static let openFrameScheduleOpenedCallSheet = Notification.Name("studio.openframe.schedule.openedCallSheet")
}

@MainActor
@Observable
final class ScheduleModel {
    let app: AppModel
    /// A scene chosen elsewhere ("Open in Schedule"): the drawer of its strip opens once.
    static var pendingScene: (sceneId: String?, lineageId: String?)?

    // What the core said.
    private(set) var view: ScheduleView?
    private(set) var loadFailure: CoreFailure?
    private(set) var loaded = false
    private(set) var suggestions: ScheduleGroupingSuggestions?
    private(set) var suggestionsLoading = false
    private(set) var dood: ScheduleDoodView?
    private(set) var doodFailure: CoreFailure?
    private(set) var doodLoaded = false
    private(set) var lastExport: ExportResult?

    // The person's view.
    var section: SchedSection = .schedule
    var mode: SchedMode = .board
    var openDayId: String?
    var filter = ScheduleFilter()
    var sheet: SchedSheet?
    var drawer: SchedDrawer?
    var pendingKeep: SchedPendingKeep?
    var dismissedSuggestions: Set<String> = []
    var doodCast = ""
    var doodFrom = ""
    var doodTo = ""
    private(set) var busy = false
    var calendarYear: Int
    var calendarMonth0: Int
    @ObservationIgnored private var linked: String?
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var doodGeneration = 0
    @ObservationIgnored private var calendarFollowsData = true

    init(app: AppModel) {
        self.app = app
        let c = Calendar(identifier: .gregorian).dateComponents([.year, .month], from: Date())
        calendarYear = c.year ?? 2027
        calendarMonth0 = (c.month ?? 1) - 1
    }

    var core: Core? { app.core }

    // MARK: Derived

    var schedule: ScheduleDto? { view?.schedule }
    var days: [ScheduleShootDayDto] { view?.days ?? [] }
    var shootingDays: [ScheduleShootDayDto] { days.filter { !$0.isOffDay } }
    var unscheduled: [ScheduleStripDto] { view?.unscheduled ?? [] }
    var counts: ScheduleCounts? { view?.counts }
    var openWarningCount: Int { Int(view?.openWarningCount ?? 0) }
    var allStrips: [ScheduleStripDto] { view?.allStrips ?? [] }
    var openDay: ScheduleShootDayDto? { openDayId.flatMap { id in days.first { $0.id == id } } }
    var isFinalized: Bool { schedule?.status == "Finalized" }

    /// Ids of the strips that match the filter, or nil when no filter is on.
    var matches: Set<String>? {
        guard view?.schedule != nil, filter.isActive else { return nil }
        return Set(allStrips.filter { filter.matches($0) }.map(\.id))
    }

    /// Strips with an open warning (the call sheet warning is shown on the day instead).
    var warnStripIds: Set<String> {
        var out = Set<String>()
        for w in view?.warnings ?? [] where w.decision == nil && w.kind != "call_sheet_stale" { out.formUnion(w.stripIds) }
        return out
    }

    func strip(_ id: String) -> ScheduleStripDto? {
        view?.unscheduled.first { $0.id == id }
            ?? days.lazy.flatMap { $0.items.compactMap(\.strip) }.first { $0.id == id }
            ?? view?.history.first { $0.id == id }
    }

    func day(_ id: String?) -> ScheduleShootDayDto? { id.flatMap { id in days.first { $0.id == id } } }

    /// The line above the board: the Production Source and the counts.
    var sourceLine: String {
        guard let s = schedule, let c = counts else { return "" }
        let src = s.source.map { "\($0.label) of \u{201C}\($0.screenplayTitle)\u{201D}" } ?? "missing"
        return "Production Source: \(src) \u{00B7} \(s.name) \u{00B7} \(Int(c.total)) scenes (\(Int(c.scheduled)) scheduled, \(Int(c.unscheduled)) unscheduled)"
            + (s.status == "Finalized" ? " \u{00B7} Finalized" : "")
    }

    /// The words the "Script changed" banner shows.
    var changesTitle: String {
        guard let c = view?.scriptChanges else { return "" }
        let affected = c.changed.count + c.removed.count
        if affected > 0 { return "Script changed in \(SchedFormat.plural(affected, "scene"))" }
        if c.newScenes > 0 { return "\(SchedFormat.plural(Int(c.newScenes), "new scene")) in the script" }
        return "The production source changed"
    }

    /// The warnings button: "3 warnings".
    var warningsLabel: String { SchedFormat.plural(openWarningCount, "warning") }

    /// What a day column shows, as one text (title, call sheet, warnings, total, scenes, breaks, empty hint).
    func columnText(_ d: ScheduleShootDayDto) -> String {
        var out = [d.title]
        if let cs = d.callSheet { out.append("Call sheet: \(SchedFormat.callSheetWord(cs.status))") }
        if d.openWarnings > 0 { out.append("\(Int(d.openWarnings)) warnings") }
        out.append(d.isOffDay ? "Off day" : d.summary.totalLabel + (d.summary.overTarget ? " \u{00B7} over target" : ""))
        if let wrap = d.timeline.estimatedWrapLabel, d.timeline.anchored { out.append("wrap \(wrap)") }
        for i in d.items {
            if let s = i.strip { out.append(SchedFormat.stripLabel(s)) }
            if let m = i.marker { out.append(SchedFormat.markerText(m)) }
        }
        if d.items.isEmpty { out.append(Self.emptyDayText(d)) }
        return out.joined(separator: "\n")
    }

    static func emptyDayText(_ d: ScheduleShootDayDto) -> String {
        d.isOffDay ? (d.notes ?? "Off day, no scenes.") : "No scenes scheduled. Drag scenes here."
    }

    // MARK: Loading

    func reload() async {
        guard let core else { return }
        generation += 1
        let gen = generation
        do {
            let v = try await core.scheduleGet()
            guard gen == generation else { return }
            view = v
            loadFailure = nil
            loaded = true
            if calendarFollowsData, let d = v.days.compactMap(\.date).first, let date = SchedDate.parse(d) {
                let c = Calendar(identifier: .gregorian).dateComponents(in: TimeZone(identifier: "UTC")!, from: date)
                calendarYear = c.year ?? calendarYear
                calendarMonth0 = (c.month ?? 1) - 1
            }
            openLinkedScene(v)
            // A day that no longer exists (deleted, or undone) takes the person back to the board.
            if let id = openDayId, !v.days.contains(where: { $0.id == id }) { openDayId = nil }
            if case let .strip(id)? = drawer, strip(id) == nil { drawer = nil }
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    /// The Screenplay and Breakdown can ask to open a scene's strip.
    private func openLinkedScene(_ v: ScheduleView) {
        guard let pending = Self.pendingScene else { return }
        let key = "\(pending.sceneId ?? "")|\(pending.lineageId ?? "")"
        guard linked != key, v.schedule != nil else { return }
        linked = key
        Self.pendingScene = nil
        let strips = v.allStrips
        let hit = strips.first { $0.sceneId == pending.sceneId }
            ?? strips.first { $0.lineageId == (pending.lineageId ?? pending.sceneId) }
        if let hit { drawer = .strip(hit.id) }
    }

    func reloadDood() async {
        guard let core else { return }
        doodGeneration += 1
        let gen = doodGeneration
        do {
            let v = try await core.scheduleDood(doodArgs)
            guard gen == doodGeneration else { return }
            dood = v
            doodFailure = nil
            doodLoaded = true
        } catch {
            guard gen == doodGeneration else { return }
            doodFailure = CoreFailure(asCore: error)
            doodLoaded = true
        }
    }

    var doodArgs: ScheduleDoodArgs {
        ScheduleDoodArgs(castKeys: doodCast.isEmpty ? nil : [doodCast], fromDayId: doodFrom.isEmpty ? nil : doodFrom, toDayId: doodTo.isEmpty ? nil : doodTo)
    }

    /// The Suggest Grouping panel asks for suggestions when it opens and when asked again.
    func loadSuggestions() async {
        guard let core else { return }
        suggestionsLoading = true
        defer { suggestionsLoading = false }
        do {
            suggestions = try await core.scheduleSuggestions()
        } catch {
            fail("Could not ask for grouping suggestions", error)
        }
    }

    var visibleSuggestions: [ScheduleGroupingSuggestion] {
        (suggestions?.suggestions ?? []).filter { !dismissedSuggestions.contains($0.key) }
    }

    // MARK: Plumbing

    func fail(_ title: String, _ error: Error) { app.show(title, error) }

    private func say(_ text: String, undoable: Bool = true) {
        if undoable { ToastCenter.shared.undoable(text, app: app) } else { ToastCenter.shared.show(text) }
    }

    func complain(_ text: String) { ToastCenter.shared.show(text, kind: .error) }

    /// Runs one change. On success says what happened (with Undo); on failure shows the core's words.
    @discardableResult
    private func command(_ done: String?, failTitle: String, _ body: (Core) async throws -> Void) async -> Bool {
        guard let core else { return false }
        do {
            try await body(core)
            if let done { say(done) }
            return true
        } catch {
            fail(failTitle, error)
            return false
        }
    }

    // MARK: Schedule

    @discardableResult
    func createSchedule() async -> Bool {
        busy = true
        defer { busy = false }
        let ok = await command("Created the shooting schedule", failTitle: "Could not create the shooting schedule") { _ = try await $0.scheduleCreate(ScheduleCreateArgs()) }
        if ok { sheet = nil }
        await reload()
        return ok
    }

    /// Name, target day length, strictness and status. Returns an error line for the dialog.
    func saveSettings(name: String, length: String, strict: Bool, status: String, doodStyle: String?) async -> String? {
        guard let s = schedule else { return nil }
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty { return "Write a name for the schedule." }
        let mins: Int
        switch SchedFormat.parseDuration(length) {
        case let .value(v) where v > 0: mins = v
        default: return "Enter the target day length, e.g. 10h or 11h 30m."
        }
        guard let core else { return nil }
        do {
            _ = try await core.scheduleUpdateSettings(ScheduleSettingsArgs(
                scheduleId: s.id, name: trimmed, strictValidation: strict, dayDurationMinutes: Double(mins),
                doodCodeStyle: doodStyle, expectedRev: s.rev))
            say("Changed schedule settings")
            if status != s.status {
                _ = try await core.scheduleSetStatus(ScheduleStatusArgs(scheduleId: s.id, status: status))
                say("Marked the schedule \(status)")
            }
            await reload()
            return nil
        } catch {
            return CoreFailure(asCore: error).message
        }
    }

    @discardableResult
    func reopenSchedule() async -> Bool {
        guard let s = schedule else { return false }
        let ok = await command("Marked the schedule Active", failTitle: "Could not reopen the schedule") {
            _ = try await $0.scheduleSetStatus(ScheduleStatusArgs(scheduleId: s.id, status: "Active"))
        }
        await reload()
        return ok
    }

    func updateFromScript() async -> Bool {
        guard let s = schedule, let core else { return false }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.scheduleReconcile(ScheduleIdArgs(scheduleId: s.id))
            say("Updated the schedule from the script changes")
            sheet = nil
            await reload()
            return true
        } catch {
            fail("Could not update the schedule from the script", error)
            return false
        }
    }

    // MARK: Days

    @discardableResult
    func createDay(date: String, notes: String, offDay: Bool, afterDayId: String?) async -> String? {
        guard let s = schedule, let core else { return "There is no schedule yet." }
        let d = date.trimmingCharacters(in: .whitespaces)
        if !d.isEmpty, SchedDate.parse(d) == nil { return "Enter the date as YYYY-MM-DD, for example 2027-06-14." }
        let n = notes.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            _ = try await core.scheduleCreateDay(ScheduleShootDayCreateArgs(
                scheduleId: s.id, date: d.isEmpty ? nil : d, notes: n.isEmpty ? nil : n, offDay: offDay, afterDayId: afterDayId))
            say(offDay ? "Added an off day" : "Created a shooting day")
            sheet = nil
            await reload()
            return nil
        } catch {
            return CoreFailure(asCore: error).message
        }
    }

    func setDayDate(_ day: ScheduleShootDayDto, date: String) async -> String? {
        let d = date.trimmingCharacters(in: .whitespaces)
        if !d.isEmpty, SchedDate.parse(d) == nil { return "Enter the date as YYYY-MM-DD, for example 2027-06-14." }
        guard let core else { return nil }
        do {
            _ = try await core.scheduleSetDayDate(ScheduleShootDateArgs(dayId: day.id, date: d.isEmpty ? nil : d))
            say("Rescheduled \(day.label)")
            sheet = nil
            await reload()
            return nil
        } catch {
            return CoreFailure(asCore: error).message
        }
    }

    func setOffDay(_ day: ScheduleShootDayDto, _ off: Bool) async {
        await command(off ? "Set \(day.label) as an off day" : "Changed the off day into a shooting day", failTitle: "Could not change the day") {
            _ = try await $0.scheduleSetOffDay(ScheduleShootOffDayArgs(dayId: day.id, offDay: off))
        }
        await reload()
    }

    func duplicateDay(_ day: ScheduleShootDayDto) async {
        await command("Duplicated \(day.label)", failTitle: "Could not duplicate the day") { _ = try await $0.scheduleDuplicateDay(ScheduleShootDayIdArgs(dayId: day.id)) }
        await reload()
    }

    /// Deleting a day returns its scenes to Unscheduled and can be undone.
    func deleteDay(_ day: ScheduleShootDayDto) async {
        if openDayId == day.id { openDayId = nil }
        await command("Deleted \(day.label); its scenes returned to Unscheduled", failTitle: "Could not delete the day") {
            _ = try await $0.scheduleDeleteDay(ScheduleShootDayIdArgs(dayId: day.id))
        }
        await reload()
    }

    /// Move a day earlier or later in the schedule.
    func moveDay(_ day: ScheduleShootDayDto, by delta: Int) async {
        guard let from = days.firstIndex(where: { $0.id == day.id }) else { return }
        let to = from + delta
        guard to >= 0, to < days.count else { return }
        await command("Moved \(day.label)", failTitle: "Could not move the day") {
            _ = try await $0.scheduleMoveDay(ScheduleShootDayMoveArgs(dayId: day.id, index: Double(to)))
        }
        await reload()
    }

    func setDayNotes(_ day: ScheduleShootDayDto, _ notes: String) async {
        let n = notes.trimmingCharacters(in: .whitespacesAndNewlines)
        if (day.notes ?? "") == n { return }
        await command(nil, failTitle: "Could not save the day notes") {
            _ = try await $0.scheduleSetDayNotes(ScheduleShootDayNotesArgs(dayId: day.id, notes: n.isEmpty ? nil : n))
        }
        await reload()
    }

    /// The day's target length ("10h"); empty = the schedule default.
    @discardableResult
    func setDayTarget(_ day: ScheduleShootDayDto, _ text: String) async -> Bool {
        let parsed = SchedFormat.parseDuration(text)
        if parsed.isInvalid {
            complain("Enter the target day length, e.g. 10h.")
            return false
        }
        let m = parsed.number
        if m.map(Double.init) == day.plannedMinutes { return true }
        let ok = await command("Changed the target length of \(day.label)", failTitle: "Could not change the target length") {
            _ = try await $0.scheduleSetDayTarget(ScheduleShootDayTargetArgs(dayId: day.id, minutes: m.map(Double.init)))
        }
        await reload()
        return ok
    }

    /// Crew call, shooting start and target wrap, as typed.
    @discardableResult
    func setDayTimes(_ day: ScheduleShootDayDto, crewCall: String, shootingStart: String, targetWrap: String) async -> Bool {
        let fields: [(String, String)] = [("crew call", crewCall), ("shooting start", shootingStart), ("target wrap", targetWrap)]
        var values: [Int?] = []
        for (name, text) in fields {
            let p = SchedFormat.parseClock(text)
            if p.isInvalid {
                complain("Enter the \(name) as a time such as 07:30.")
                return false
            }
            values.append(p.number)
        }
        func same(_ a: Int?, _ b: Double?) -> Bool { a.map(Double.init) == b }
        if same(values[0], day.crewCall), same(values[1], day.shootingStart), same(values[2], day.targetWrap) { return true }
        let ok = await command("Changed the call and wrap times of \(day.label)", failTitle: "Could not change the day times") {
            _ = try await $0.scheduleSetDayTimes(ScheduleDayTimesArgs(
                dayId: day.id, crewCall: values[0].map(Double.init), shootingStart: values[1].map(Double.init), targetWrap: values[2].map(Double.init)))
        }
        await reload()
        return ok
    }

    // MARK: Breaks

    /// Returns an error line for the dialog, or nil when it is saved.
    func saveMarker(_ mode: SchedMarkerDialogMode, type: String, label: String, time: String, duration: String, notes: String) async -> String? {
        let mins: Int?
        switch SchedFormat.parseDuration(duration) {
        case .invalid: return "Enter a duration such as 45m or 1h 30m."
        case let .value(v): mins = v
        case .empty: mins = nil
        }
        let t = time.trimmingCharacters(in: .whitespaces)
        if !t.isEmpty, case .invalid = SchedFormat.parseClock(t) { return "Enter the time like 20:30." }
        let l = label.trimmingCharacters(in: .whitespaces)
        if type == "Custom", l.isEmpty { return "Write the note for this marker." }
        let n = notes.trimmingCharacters(in: .whitespacesAndNewlines)
        let at: String? = t.isEmpty ? nil : (SchedFormat.parseClock(t).number.map { SchedFormat.clock($0) })
        guard let core else { return nil }
        do {
            if let m = mode.marker {
                _ = try await core.scheduleUpdateMarker(ScheduleMarkerUpdateArgs(
                    markerId: m.id, markerType: type, label: l.isEmpty ? nil : l, atTime: at, durationMinutes: mins.map(Double.init), notes: n.isEmpty ? nil : n))
                say("Edited a break")
            } else {
                _ = try await core.scheduleAddMarker(ScheduleMarkerAddArgs(
                    dayId: mode.dayId, markerType: type, label: l.isEmpty ? nil : l, atTime: at, durationMinutes: mins.map(Double.init), notes: n.isEmpty ? nil : n))
                say("Added a break")
            }
            sheet = nil
            await reload()
            return nil
        } catch {
            return CoreFailure(asCore: error).message
        }
    }

    func deleteMarker(_ marker: ScheduleMarkerDto) async {
        let ok = await command("Removed a break", failTitle: "Could not remove the break") { _ = try await $0.scheduleDeleteMarker(ScheduleMarkerIdArgs(markerId: marker.id)) }
        if ok { sheet = nil }
        await reload()
    }

    // MARK: Moves (drag and drop, and the menus)

    /// A move of one scene. Strict validation can ask for "Keep Anyway" first.
    @discardableResult
    func moveStrip(_ stripId: String, to dayId: String?, index: Int? = nil, text: String) async -> Bool {
        let args = ScheduleStripMoveArgs(stripId: stripId, dayId: dayId, index: index.map(Double.init), keepAnyway: false)
        return await runMove(.strip(args), text)
    }

    @discardableResult
    func moveStrips(_ stripIds: [String], to dayId: String, text: String) async -> Bool {
        await runMove(.strips(ScheduleStripsMoveArgs(stripIds: stripIds, dayId: dayId, keepAnyway: false)), text)
    }

    private func runMove(_ move: SchedPendingKeep.Move, _ text: String) async -> Bool {
        guard let core else { return false }
        do {
            switch move {
            case let .strip(a): _ = try await core.scheduleMoveStrip(a)
            case let .strips(a): _ = try await core.scheduleMoveStrips(a)
            }
            say(text)
            await reload()
            return true
        } catch {
            let f = CoreFailure(asCore: error)
            if f.code == "validation.schedule_warning" {
                pendingKeep = SchedPendingKeep(message: f.message, move: move, doneText: text)
            } else {
                fail("Could not move the scene", error)
            }
            return false
        }
    }

    /// "Keep Anyway": the same move again, with the new warning accepted.
    @discardableResult
    func confirmKeep() async -> Bool {
        guard let p = pendingKeep, let core else { return false }
        pendingKeep = nil
        do {
            switch p.move {
            case var .strip(a):
                a.keepAnyway = true
                _ = try await core.scheduleMoveStrip(a)
            case var .strips(a):
                a.keepAnyway = true
                _ = try await core.scheduleMoveStrips(a)
            }
            say(p.doneText)
            await reload()
            return true
        } catch {
            fail("Could not move the scene", error)
            return false
        }
    }

    func cancelKeep() { pendingKeep = nil }

    func moveMarker(_ markerId: String, to dayId: String, index: Int?, dayLabel: String) async {
        await command("Moved a break on \(dayLabel)", failTitle: "Could not move the break") {
            _ = try await $0.scheduleMoveMarker(ScheduleMarkerMoveArgs(markerId: markerId, dayId: dayId, index: index.map(Double.init)))
        }
        await reload()
    }

    /// What dropping the dragged payload on a target asks for (Board.tsx `onDragEnd`).
    func dropPlan(payload: String, target: SchedDropTarget) -> SchedDropPlan {
        let id: String
        let isStrip: Bool
        if payload.hasPrefix(SchedDrag.stripPrefix) {
            id = String(payload.dropFirst(SchedDrag.stripPrefix.count))
            isStrip = true
        } else if payload.hasPrefix(SchedDrag.markerPrefix) {
            id = String(payload.dropFirst(SchedDrag.markerPrefix.count))
            isStrip = false
        } else {
            return .none
        }
        var container: String?
        var label = "Break"
        if isStrip {
            guard let s = strip(id) else { return .none }
            container = s.dayId ?? "pool"
            label = SchedFormat.sceneName(s)
        } else {
            container = days.first { $0.items.contains { $0.marker?.id == id } }?.id
            if container == nil { return .none }
        }
        switch target {
        case .pool:
            if isStrip, container != "pool" { return .moveStrip(id: id, dayId: nil, index: nil, text: "Unscheduled \(label)") }
            return .none
        case let .day(dayId, before):
            guard let day = day(dayId) else { return .none }
            if day.isOffDay, isStrip { return .refuse("This is an off day. Change it back to a shooting day before adding scenes.") }
            let list = day.itemIds
            let overIndex = before.flatMap { list.firstIndex(of: $0) }
            if container == dayId {
                if overIndex == nil ? list.last == id : list.firstIndex(of: id) == overIndex { return .none }
            }
            if isStrip {
                let text = container == dayId ? "Reordered \(day.label)" : container == "pool" ? "Scheduled \(label) on \(day.label)" : "Moved \(label) to \(day.label)"
                return .moveStrip(id: id, dayId: dayId, index: overIndex, text: text)
            }
            return .moveMarker(id: id, dayId: dayId, index: overIndex, text: "")
        }
    }

    /// Does what a drop asks for. Returns true when something was done.
    @discardableResult
    func drop(payload: String, on target: SchedDropTarget) async -> Bool {
        if isFinalized {
            complain("This schedule is finalized. Reopen it in settings to make changes.")
            return false
        }
        switch dropPlan(payload: payload, target: target) {
        case .none: return false
        case let .refuse(text):
            complain(text)
            return false
        case let .moveStrip(id, dayId, index, text):
            return await moveStrip(id, to: dayId, index: index, text: text)
        case let .moveMarker(id, dayId, index, _):
            await moveMarker(id, to: dayId, index: index, dayLabel: day(dayId)?.label ?? "the day")
            return true
        }
    }

    /// The menu of a scene: "Move to" a day, Move Up, Move Down, Remove from Day.
    func moveMenu(for s: ScheduleStripDto) -> SchedStripMoveMenu {
        let choices = days.filter { !$0.isOffDay }.map { SchedStripMoveMenu.DayChoice(day: $0, disabled: $0.id == s.dayId) }
        guard let d = day(s.dayId) else { return SchedStripMoveMenu(days: choices, canMoveUp: false, canMoveDown: false, isScheduled: false) }
        let order = d.itemIds
        let idx = order.firstIndex(of: s.id)
        return SchedStripMoveMenu(
            days: choices, canMoveUp: (idx ?? 0) > 0, canMoveDown: idx.map { $0 < order.count - 1 } ?? false, isScheduled: true)
    }

    func moveStripToDay(_ s: ScheduleStripDto, _ d: ScheduleShootDayDto) async {
        await moveStrip(s.id, to: d.id, text: "Moved \(SchedFormat.sceneName(s)) to \(d.label)")
    }

    func moveStripBy(_ s: ScheduleStripDto, _ delta: Int) async {
        guard let d = day(s.dayId), let idx = d.itemIds.firstIndex(of: s.id) else { return }
        let to = idx + delta
        guard to >= 0, to < d.itemIds.count else { return }
        await moveStrip(s.id, to: d.id, index: to, text: "Reordered \(d.label)")
    }

    func removeFromDay(_ s: ScheduleStripDto) async {
        await moveStrip(s.id, to: nil, text: "Unscheduled \(SchedFormat.sceneName(s))")
    }

    // MARK: Scene details

    func setEstimate(_ s: ScheduleStripDto, _ text: String) async -> String? {
        let p = SchedFormat.parseDuration(text)
        if p.isInvalid { return "Enter the estimate as minutes or e.g. 1h 30m." }
        if p.number.map(Double.init) == s.estimatedMinutes { return nil }
        await command("Changed a time estimate", failTitle: "Could not change the estimate") {
            _ = try await $0.scheduleSetStripEstimate(ScheduleStripEstimateArgs(stripId: s.id, minutes: p.number.map(Double.init)))
        }
        await reload()
        return nil
    }

    func setStart(_ s: ScheduleStripDto, _ text: String) async -> String? {
        let p = SchedFormat.parseClock(text)
        if p.isInvalid { return "Enter the start time like 14:30, or leave it empty to follow the day's order." }
        if p.number.map(Double.init) == s.startMinutes { return nil }
        let n = p.number
        await command(n == nil ? "Let the scene follow the day's order" : "Set a scene start time", failTitle: "Could not set the start time") {
            _ = try await $0.scheduleSetStripStart(ScheduleStripStartArgs(stripId: s.id, minutes: n.map(Double.init)))
        }
        await reload()
        return nil
    }

    func setPages(_ s: ScheduleStripDto, _ text: String) async -> String? {
        let p = SchedFormat.parsePages(text)
        if p.isInvalid { return "Enter pages like 1 2/8 or 3/8." }
        if case .value(0) = p { return "Enter pages like 1 2/8 or 3/8." }
        let current: Int? = s.pagesOverridden ? Int(s.pageEighths) : nil
        if p.number == current { return nil }
        await command("Changed a page count", failTitle: "Could not change the page count") {
            _ = try await $0.scheduleSetStripPages(ScheduleStripPagesArgs(stripId: s.id, eighths: p.number.map(Double.init)))
        }
        await reload()
        return nil
    }

    func acknowledgeChange(_ s: ScheduleStripDto) async {
        await command("Marked the change reviewed", failTitle: "Could not mark the change reviewed") { _ = try await $0.scheduleAcknowledgeChange(ScheduleStripIdArgs(stripId: s.id)) }
        await reload()
    }

    func confirmRemoval(_ s: ScheduleStripDto) async {
        let ok = await command("Removed the scene from the schedule", failTitle: "Could not remove the scene") { _ = try await $0.scheduleConfirmRemoval(ScheduleStripIdArgs(stripId: s.id)) }
        if ok, drawer == .strip(s.id) { drawer = nil }
        await reload()
    }

    // MARK: Warnings

    /// "Kept" (Keep anyway), "Dismissed", or nil to show the warning again.
    func decide(_ w: ScheduleWarningDto, _ decision: String?) async {
        guard let s = schedule else { return }
        await command(decision == "Kept" ? "Kept the schedule as it is" : nil, failTitle: "Could not save your choice") {
            _ = try await $0.scheduleDecideWarning(ScheduleWarningDecisionArgs(scheduleId: s.id, key: w.key, decision: decision))
        }
        await reload()
    }

    // MARK: Call sheets

    /// "Create Call Sheet" / "Open Call Sheet" on a day: opens the Call Sheets workspace.
    func openCallSheet(_ day: ScheduleShootDayDto) async {
        var id = day.callSheet?.id
        if id == nil, let core {
            do {
                id = try await core.callsheetsCreate(CallSheetCreateArgs(dayId: day.id))
                say("Created a call sheet from the shooting day")
            } catch {
                let f = CoreFailure(asCore: error)
                if f.code == "conflict.call_sheet_exists", let existing = f.detail {
                    id = existing
                } else {
                    fail("Could not create the call sheet", error)
                    return
                }
            }
        }
        await reload()
        if let id {
            NotificationCenter.default.post(name: .openFrameScheduleOpenedCallSheet, object: id)
            // The Call Sheets workspace opens this sheet when it shows.
            CallSheetsRoute.shared.request(.sheet(id))
            NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.callSheets.rawValue)
        }
    }

    // MARK: Day Out of Days

    func setDoodCode(_ row: DoodRow, _ cell: DoodCell, _ day: DoodDay, code: String?) async {
        guard let v = dood, let sid = v.scheduleId else { return }
        // Changing the code of a correction keeps its note; clearing the correction removes both.
        let note = code != nil && cell.overridden ? cell.note : nil
        await command(
            code.map { "Set \(row.label) on \(day.title) to \($0)" } ?? "Cleared the correction for \(row.label) on \(day.title)",
            failTitle: "Could not change the Day Out of Days"
        ) {
            _ = try await $0.scheduleSetDoodOverride(ScheduleDoodOverrideArgs(scheduleId: sid, castKey: row.key, dayId: cell.dayId, code: code, note: note))
        }
        await reloadDood()
    }

    func saveDoodNote(_ t: SchedDoodNoteTarget, text: String) async -> Bool {
        guard let sid = dood?.scheduleId else { return false }
        let n = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let ok = await command("Changed the note for \(t.row.label) on \(t.dayTitle)", failTitle: "Could not save the note") {
            _ = try await $0.scheduleSetDoodOverride(ScheduleDoodOverrideArgs(scheduleId: sid, castKey: t.row.key, dayId: t.cell.dayId, code: t.cell.code, note: n.isEmpty ? nil : n))
        }
        if ok { sheet = nil }
        await reloadDood()
        return ok
    }

    func setDoodStyle(_ style: String) async {
        guard let s = schedule, style != dood?.codeStyle else { return }
        await command("Changed the Day Out of Days codes", failTitle: "Could not change the codes") {
            _ = try await $0.scheduleUpdateSettings(ScheduleSettingsArgs(
                scheduleId: s.id, name: s.name, strictValidation: s.strictValidation, dayDurationMinutes: s.dayDurationMinutes, doodCodeStyle: style))
        }
        await reload()
        await reloadDood()
    }

    var correctedCount: Int { (dood?.rows ?? []).reduce(0) { $0 + $1.cells.filter(\.overridden).count } }

    // MARK: Exports

    @discardableResult
    func exportSchedule(format: String, path: String, dayIds: [String]?, includeUnscheduled: Bool, includeNotes: Bool) async -> ExportResult? {
        guard let core else { return nil }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.scheduleExportSchedule(ExportScheduleArgs(
                format: format, path: path, dayIds: dayIds, includeUnscheduled: includeUnscheduled, includeNotes: includeNotes))
            lastExport = r
            ToastCenter.shared.show("Exported \(r.fileName)")
            return r
        } catch {
            fail("Could not export the shooting schedule", error)
            return nil
        }
    }

    @discardableResult
    func exportDood(format: String, path: String) async -> ExportResult? {
        guard let core else { return nil }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.scheduleExportDood(ExportDoodArgs(
                format: format, path: path, castKeys: doodCast.isEmpty ? nil : [doodCast],
                fromDayId: doodFrom.isEmpty ? nil : doodFrom, toDayId: doodTo.isEmpty ? nil : doodTo))
            lastExport = r
            ToastCenter.shared.show("Exported \(r.fileName)")
            return r
        } catch {
            fail("Could not export the Day Out of Days", error)
            return nil
        }
    }

    // MARK: Calendar

    func shiftMonth(_ delta: Int) {
        calendarFollowsData = false
        let n = SchedCalendarGrid.shift(year: calendarYear, month0: calendarMonth0, by: delta)
        calendarYear = n.year
        calendarMonth0 = n.month0
    }

    /// The days that fall on each date of the calendar.
    var daysByDate: [String: [ScheduleShootDayDto]] {
        var m: [String: [ScheduleShootDayDto]] = [:]
        for d in days { if let date = d.date { m[date, default: []].append(d) } }
        return m
    }
}

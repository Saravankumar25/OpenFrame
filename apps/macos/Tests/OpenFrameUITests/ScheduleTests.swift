// Mac phase M6, Schedule workspace. The shared end-to-end scenario 61-schedule.md, run at model level
// against a real core on a temporary folder: each test drives the Schedule model the way the screen
// does (the same calls, in the same order) and checks what the screen would show. Plus the pure
// helpers (minutes, clock times, pages, filters, the calendar grid), the edge cases (an empty
// project, deleting a day with scenes, undo, off days, finalized schedules, strict validation) and
// rendered snapshots of the main screens.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Helpers

private let nightShift = [
    "Title: Night Shift", "",
    "INT. CONTROL ROOM - NIGHT", "",
    "Kavya studies a flickering monitor and lifts a torch.", "",
    "KAVYA", "Someone is in the tunnel.", "",
    "EXT. TUNNEL MOUTH - NIGHT", "",
    "Ravi drags a heavy toolbox through the rain.", "",
    "RAVI", "Hold the gate.", "",
].joined(separator: "\n")

@MainActor
struct SchedRig {
    let root: URL
    let app: AppModel
    let model: ScheduleModel
    let projectPath: String

    static func make(title: String = "Signal Yard", source: Bool = false, elements: Bool = false) async throws -> SchedRig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        #expect(ok)
        guard case let .project(p) = app.phase else { throw CoreFailure(code: "test", message: "no project") }
        let rig = SchedRig(root: root, app: app, model: ScheduleModel(app: app), projectPath: p.path)
        if source { try await rig.importNightShift(); try await rig.chooseSource() }
        if elements { try await rig.confirmElements() }
        await rig.model.reload()
        return rig
    }

    var core: Core { app.core! }
    var quiet: Bool { app.alert == nil }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    func importNightShift(_ text: String = nightShift) async throws {
        let preview = try await core.screenplayImportPreview(InterchangeImportPreviewArgs(pastedText: text, format: "fountain"))
        _ = try await core.screenplayImportApply(InterchangeImportApplyArgs(previewId: preview.previewId, mode: "new_screenplay", keepSourceFile: false))
    }

    /// Breakdown: Select Source, click the draft, Set as Production Source.
    func chooseSource() async throws {
        let hub = BreakdownHub(app: app)
        let bd = hub.breakdown!
        await bd.reload()
        await bd.loadSourceOptions()
        let choice = try #require(bd.defaultSourceChoice)
        #expect(await bd.setSource(draftId: choice, reason: nil))
    }

    /// For each scene: Suggest Elements, then accept every suggestion (cast comes from this).
    func confirmElements() async throws {
        let hub = BreakdownHub(app: app)
        let bd = hub.breakdown!
        await bd.reload()
        for row in bd.rows {
            bd.selectScene(row.sceneId)
            await bd.reloadScene()
            await bd.suggest()
            await bd.reloadScene()
            await bd.acceptAllSafe()
            await bd.reloadScene()
            for s in bd.pendingSuggestions { await bd.accept(s); await bd.reloadScene() }
        }
    }

    func relaunch() async throws -> SchedRig {
        await app.closeProject()
        app.shutdown()
        let app2 = Fixture.model(root)
        await app2.start()
        await app2.openProject(path: projectPath)
        #expect(app2.alert == nil, "a recovery question showed")
        let rig = SchedRig(root: root, app: app2, model: ScheduleModel(app: app2), projectPath: projectPath)
        await rig.model.reload()
        return rig
    }

    // MARK: Things a person does

    @discardableResult
    func createDay(_ iso: String?, off: Bool = false, after: String? = nil) async -> Bool {
        let err = await model.createDay(date: iso ?? "", notes: "", offDay: off, afterDayId: after)
        await model.reload()
        return err == nil
    }

    func day(_ n: Int) -> ScheduleShootDayDto { model.days.filter { !$0.isOffDay }[n - 1] }
    func scene(_ n: Int) -> ScheduleStripDto { model.allStrips.first { Int($0.number ?? -1) == n }! }
    var dayOne: String { model.columnText(day(1)) }

    func dragScene(_ n: Int, into d: ScheduleShootDayDto, before: String? = nil) async {
        await model.drop(payload: SchedDrag.strip(scene(n).id), on: .day(d.id, before: before))
    }

    func undo() async { await app.undo(); await model.reload() }
    func redo() async { await app.redo(); await model.reload() }

    func addMeal(_ d: ScheduleShootDayDto, duration: String = "45m") async -> String? {
        await model.saveMarker(SchedMarkerDialogMode(dayId: d.id, type: "Meal"), type: "Meal", label: "", time: "", duration: duration, notes: "")
    }
}

// MARK: Scenario 61: Create the schedule

@Suite("Schedule scenario", .serialized)
@MainActor
struct ScheduleScenarioTests {
    @Test func createTheScheduleFromTheProductionSource() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        // Production tab Schedule: no schedule yet, but a source to build one from.
        #expect(r.model.loaded && r.model.schedule == nil)
        let src = try #require(r.model.view?.activeSource, "the screen offers Create Shooting Schedule")
        #expect(Int(src.sceneCount) == 2)
        r.model.sheet = .createSchedule
        #expect(await r.model.createSchedule())
        #expect(r.model.sheet == nil, "the dialog closes")
        #expect(Int(r.model.counts?.unscheduled ?? 0) == 2, "the workspace shows \"2 unscheduled\"")
        #expect(r.model.unscheduled.count == 2)
        #expect(r.model.sourceLine.contains("Production Source:"))
        #expect(r.model.sourceLine.contains("2 unscheduled"))
        #expect(r.quiet)
        // Every scene is accounted for, in scene order, with its heading.
        #expect(r.model.unscheduled.map { Int($0.number ?? 0) }.sorted() == [1, 2])
    }

    // MARK: Days, off day, drag, undo, menu

    @Test func daysOffDayDragUndoRedoAndMenuMove() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        #expect(await r.model.createSchedule())

        #expect(await r.createDay("2027-06-14"))
        #expect(await r.createDay("2027-06-15"))
        #expect(await r.createDay(nil, off: true))
        #expect(r.model.days.count == 3)
        #expect(r.model.days.map(\.title).contains("Shoot Day 2 \u{00B7} Tue 15 Jun 2027"), "titles: \(r.model.days.map(\.title))")
        #expect(r.model.days.last?.isOffDay == true)

        // Drag Scene 1 from the pool into the Day 1 column.
        await r.dragScene(1, into: r.day(1))
        #expect(r.dayOne.contains("1 scene"), "\(r.dayOne)")
        #expect(!r.dayOne.contains("2 scenes"))

        // Ctrl+Z puts it back in the pool; Ctrl+Y puts it in the day again.
        await r.undo()
        #expect(r.model.unscheduled.count == 2, "the pool shows 2 unscheduled")
        #expect(r.dayOne.contains("No scenes scheduled"), "\(r.dayOne)")
        await r.redo()
        #expect(r.dayOne.contains("1 scene"))

        // Scene 2 actions, then Shoot Day 1.
        let menu = r.model.moveMenu(for: r.scene(2))
        #expect(menu.days.map(\.day.id) == r.model.days.filter { !$0.isOffDay }.map(\.id), "the menu lists shooting days only")
        #expect(!menu.isScheduled)
        await r.model.moveStripToDay(r.scene(2), r.day(1))
        #expect(r.dayOne.contains("2 scenes"), "\(r.dayOne)")
        let off = try #require(r.model.days.last)
        #expect(r.model.columnText(off).contains("Off day"))
        #expect(r.quiet)
    }

    // MARK: Meal break, times, timeline

    @Test func mealBreakDayTimesAndTheComputedTimeline() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        #expect(await r.model.createSchedule())
        await r.createDay("2027-06-14")
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(1))

        // Shoot Day 1 actions, Meal Break, "45m", Add.
        r.model.sheet = .marker(SchedMarkerDialogMode(dayId: r.day(1).id, type: "Meal"))
        #expect(await r.addMeal(r.day(1)) == nil)
        #expect(r.model.sheet == nil)
        #expect(r.dayOne.contains("Meal Break"), "\(r.dayOne)")
        #expect(r.day(1).items.last?.marker?.durationMinutes == 45)

        // Open the day.
        r.model.openDayId = r.day(1).id
        let day = try #require(r.model.openDay)
        #expect(day.title == "Shoot Day 1 \u{00B7} Mon 14 Jun 2027")
        #expect(!day.timeline.anchored, "no times yet: the screen asks for a crew call")

        // Crew call 07:00, then shooting start 08:00.
        #expect(await r.model.setDayTimes(day, crewCall: "07:00", shootingStart: "", targetWrap: ""))
        #expect(await r.model.setDayTimes(try #require(r.model.openDay), crewCall: "07:00", shootingStart: "08:00", targetWrap: ""))
        let timed = try #require(r.model.openDay)
        let lines = timed.timeline.rows.map { "\($0.startLabel ?? "") \($0.label)" }.joined(separator: "\n")
        for want in ["07:00", "Crew call", "08:00", "Shooting starts", "Meal Break", "Estimated wrap"] {
            #expect(lines.contains(want), "the timeline shows \(want): \(lines)")
        }
        #expect(timed.timeline.anchored)

        // Day target length 30 (minutes): Over target.
        #expect(await r.model.setDayTarget(timed, "30"))
        #expect(try #require(r.model.openDay).summary.overTarget, "the workspace shows Over target")
        #expect(r.quiet)

        // Back to schedule.
        r.model.openDayId = nil
        #expect(r.model.openDay == nil)
    }

    // MARK: Conflicts are advice

    @Test func conflictsAreAdviceKeepAnywayAndGroupingMovesNothing() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await prepareDayOne(r)

        #expect(r.dayOne.contains("2 scenes"))
        #expect(r.model.openWarningCount > 0, "the warnings button does not read 0 warnings (\(r.model.warningsLabel))")
        #expect(r.model.warningsLabel != "0 warnings")

        r.model.drawer = .warnings
        let first = try #require(r.model.view?.warnings.first { $0.decision == nil && $0.kind != "call_sheet_stale" } ?? r.model.view?.warnings.first { $0.decision == nil })
        await r.model.decide(first, "Kept")
        let kept = try #require(r.model.view?.warnings.first { $0.key == first.key })
        #expect(kept.decision == "Kept", "the panel shows Kept anyway")
        r.model.drawer = nil

        let before = r.dayOne
        #expect(before.contains("2 scenes"))
        r.model.drawer = .suggestions
        await r.model.loadSuggestions()
        #expect(r.model.suggestions != nil, "the Grouping suggestions panel opens")
        r.model.drawer = nil
        #expect(r.dayOne == before, "grouping suggestions move nothing")
        #expect(r.quiet)

        // Show again undoes the decision; nothing else moves.
        await r.model.decide(kept, nil)
        #expect(r.model.view?.warnings.first { $0.key == first.key }?.decision == nil)
        #expect(r.dayOne.contains("2 scenes"))
    }

    // MARK: Filters

    @Test func filtersOnlyChangeWhatIsShown() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await prepareDayOne(r)
        let before = r.model.view

        r.model.mode = .list
        r.model.filter.scheduled = .unscheduled
        #expect(r.model.listRows.isEmpty, "the workspace shows \"No matching scenes.\"")
        #expect(r.model.matches?.count == 0)
        #expect(r.model.allStrips.count == 2, "Showing 0 of 2 scenes")
        r.model.filter = ScheduleFilter()
        #expect(r.model.listRows.count == 2)
        #expect(r.model.matches == nil)

        r.model.mode = .board
        #expect(r.dayOne.contains("2 scenes"))
        #expect(r.model.view == before, "filters never touch the schedule")
        #expect(r.quiet)
    }

    // MARK: Call sheet stale review

    @Test func theCallSheetStaleReviewFollowsTheDayTimes() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await prepareDayOne(r)

        // Create Call Sheet from the day menu.
        await r.model.openCallSheet(r.day(1))
        #expect(r.dayOne.contains("Call sheet: Draft"), "\(r.dayOne)")

        // Day detail: Target wrap 18:00.
        let day = r.day(1)
        #expect(await r.model.setDayTimes(day, crewCall: day.crewCall.map { SchedFormat.clock($0) } ?? "", shootingStart: day.shootingStart.map { SchedFormat.clock($0) } ?? "", targetWrap: "18:00"))
        #expect(r.dayOne.contains("Call sheet: Needs refresh"), "\(r.dayOne)")
        await r.undo()
        #expect(r.dayOne.contains("Call sheet: Draft"), "\(r.dayOne)")
        await r.redo()
        #expect(r.quiet)

        // Open Call Sheet on a day that has one does not create a second.
        let id = r.day(1).callSheet?.id
        await r.model.openCallSheet(r.day(1))
        #expect(r.day(1).callSheet?.id == id)
    }

    // MARK: Day Out of Days

    @Test func dayOutOfDaysDerivedCorrectedByHandAndUndone() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await prepareDayOne(r)
        await r.createDay("2027-06-15")
        await r.createDay(nil, off: true)

        r.model.section = .dood
        await r.model.reloadDood()
        let v = try #require(r.model.dood, "the Day Out of Days table shows")
        #expect(v.hasSchedule && !v.rows.isEmpty, "cast confirmed in Breakdown gives rows")
        let first = try #require(v.rows.first)
        #expect(["SWF", "SW", "W"].contains(first.cells[0].code), "the first cell for Shoot Day 1 shows a working code (\(first.cells[0].code))")

        // Click a cell for Shoot Day 2, choose T.
        let dayTwo = try #require(v.days.dropFirst().first)
        let cell2 = first.cells[1]
        #expect(cell2.dayId == dayTwo.id)
        await r.model.setDoodCode(first, cell2, dayTwo, code: "T")
        await r.model.reloadDood()
        #expect(r.model.correctedCount == 1, "the workspace shows \"1 corrected by hand\"")
        #expect(r.model.dood?.rows.first?.cells[1].code == "T")

        await r.undo()
        await r.model.reloadDood()
        #expect(r.model.correctedCount == 0)
        #expect(!(r.model.dood?.rows.flatMap(\.cells).contains { $0.code == "T" } ?? true), "the table no longer shows T*")
        await r.redo()
        await r.model.reloadDood()
        #expect(r.model.correctedCount == 1)

        // Click the cell again, Add a note.
        let row = try #require(r.model.dood?.rows.first)
        let cell = row.cells[1]
        let target = SchedDoodNoteTarget(row: row, cell: cell, dayTitle: dayTwo.title)
        r.model.sheet = .note(target)
        #expect(await r.model.saveDoodNote(target, text: "Flies in the night before"))
        #expect(r.model.sheet == nil)
        let noted = try #require(r.model.dood?.rows.first?.cells[1])
        #expect(noted.note == "Flies in the night before", "a cell is named with note: Flies in the night before")
        #expect(noted.overridden && noted.code == "T")
        #expect(r.quiet)

        // Back to the schedule value clears the correction and its note.
        await r.model.setDoodCode(row, noted, dayTwo, code: nil)
        #expect(r.model.correctedCount == 0)
    }

    // MARK: Exports

    @Test func exportsOpenAndTheFilesAreTheSameByteForByte() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await prepareDayOne(r)
        let out = r.root.appendingPathComponent("exports", isDirectory: true)
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

        r.model.sheet = .export(.dood)
        #expect(r.model.sheet?.id.hasPrefix("export") == true, "the screen shows Export Day Out of Days")
        r.model.sheet = nil
        r.model.sheet = .export(.schedule(currentDayId: nil))
        #expect(r.model.sheet != nil, "the screen shows Export Shooting Schedule")
        r.model.sheet = nil

        for format in ["pdf", "xlsx"] {
            let a = out.appendingPathComponent("schedule-a.\(format)").path
            let b = out.appendingPathComponent("schedule-b.\(format)").path
            let ra = await r.model.exportSchedule(format: format, path: a, dayIds: nil, includeUnscheduled: true, includeNotes: false)
            try await Task.sleep(for: .seconds(1))
            let rb = await r.model.exportSchedule(format: format, path: b, dayIds: nil, includeUnscheduled: true, includeNotes: false)
            #expect(ra != nil && rb != nil)
            let da = try Data(contentsOf: URL(fileURLWithPath: a)), db = try Data(contentsOf: URL(fileURLWithPath: b))
            #expect(!da.isEmpty && da == db, "the shooting schedule \(format) files are the same byte for byte")

            let c = out.appendingPathComponent("dood-a.\(format)").path
            let d = out.appendingPathComponent("dood-b.\(format)").path
            let rc = await r.model.exportDood(format: format, path: c)
            try await Task.sleep(for: .seconds(1))
            let rd = await r.model.exportDood(format: format, path: d)
            #expect(rc != nil && rd != nil)
            let dc = try Data(contentsOf: URL(fileURLWithPath: c)), dd = try Data(contentsOf: URL(fileURLWithPath: d))
            #expect(!dc.isEmpty && dc == dd, "the Day Out of Days \(format) files are the same byte for byte")
        }
        #expect(r.model.lastExport?.fileName.isEmpty == false)
        #expect(r.quiet)

        // Selected days and the current day only.
        let one = out.appendingPathComponent("one-day.csv").path
        let res = await r.model.exportSchedule(format: "csv", path: one, dayIds: [r.day(1).id], includeUnscheduled: false, includeNotes: false)
        #expect(res != nil)
    }

    // MARK: Relaunch

    @Test func afterARelaunchDaysScenesBreaksTimesAndCorrectionsAreKept() async throws {
        var r = try await SchedRig.make(source: true, elements: true)
        try await prepareDayOne(r, full: false)
        #expect(await r.addMeal(r.day(1)) == nil)
        let d = r.day(1)
        #expect(await r.model.setDayTimes(d, crewCall: "07:00", shootingStart: "08:00", targetWrap: ""))
        await r.createDay("2027-06-15")
        await r.model.reloadDood()
        let row = try #require(r.model.dood?.rows.first)
        let dayTwo = try #require(r.model.dood?.days[1])
        await r.model.setDoodCode(row, row.cells[1], dayTwo, code: "T")
        await r.model.reloadDood()
        let row2 = try #require(r.model.dood?.rows.first)
        #expect(await r.model.saveDoodNote(SchedDoodNoteTarget(row: row2, cell: row2.cells[1], dayTitle: dayTwo.title), text: "Flies in the night before"))

        r = try await r.relaunch()
        defer { r.close() }
        #expect(r.dayOne.contains("2 scenes"), "\(r.dayOne)")
        #expect(r.dayOne.contains("Meal Break"))
        #expect(r.dayOne.contains("wrap"), "\(r.dayOne)")
        await r.model.reloadDood()
        #expect(r.model.correctedCount == 1)
        #expect(r.model.dood?.rows.first?.cells[1].note == "Flies in the night before")
        #expect(r.quiet)
    }

    /// Two days' worth of setup that several scenarios share: schedule, Day 1 with both scenes.
    /// With `full`, also the state the earlier scenario steps leave behind: a 45m meal break, crew call
    /// 07:00, shooting start 08:00 and a 30 minute day target (which makes the day too long).
    private func prepareDayOne(_ r: SchedRig, full: Bool = true) async throws {
        #expect(await r.model.createSchedule())
        #expect(await r.createDay("2027-06-14"))
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(1))
        guard full else { return }
        #expect(await r.addMeal(r.day(1)) == nil)
        #expect(await r.model.setDayTimes(r.day(1), crewCall: "07:00", shootingStart: "08:00", targetWrap: ""))
        #expect(await r.model.setDayTarget(r.day(1), "30"))
    }
}

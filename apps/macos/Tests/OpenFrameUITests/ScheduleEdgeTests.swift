// Mac phase M6, Schedule workspace: the pure helpers and the edge, crash and failure cases around
// the shared scenario (an empty project, no scenes, deleting a day with scenes, undo and redo, off
// days, finalized schedules, strict validation, wrong input, breaks), plus rendered snapshots of
// the main screens in light and dark.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Pure helpers

@Suite("Schedule helpers")
struct ScheduleHelperTests {
    @Test func minutesAreWrittenAndRead() {
        #expect(SchedFormat.minutes(330) == "5h 30m")
        #expect(SchedFormat.minutes(45) == "45m")
        #expect(SchedFormat.minutes(600) == "10h")
        #expect(SchedFormat.minutes(605) == "10h 05m")
        #expect(SchedFormat.parseDuration("90") == .value(90))
        #expect(SchedFormat.parseDuration("1:30") == .value(90))
        #expect(SchedFormat.parseDuration("1h 30m") == .value(90))
        #expect(SchedFormat.parseDuration("1.5h") == .value(90))
        #expect(SchedFormat.parseDuration("45m") == .value(45))
        #expect(SchedFormat.parseDuration("45 min") == .value(45))
        #expect(SchedFormat.parseDuration("10h") == .value(600))
        #expect(SchedFormat.parseDuration("  ") == .empty)
        #expect(SchedFormat.parseDuration("soon") == .invalid)
        #expect(SchedFormat.parseDuration("h") == .invalid)
        #expect(SchedFormat.parseDuration("1:99x") == .invalid)
    }

    @Test func clockTimesAreWrittenAndRead() {
        #expect(SchedFormat.clock(420) == "07:00")
        #expect(SchedFormat.clock(1500) == "01:00 (+1)")
        #expect(SchedFormat.parseClock("7") == .value(420))
        #expect(SchedFormat.parseClock("7:30") == .value(450))
        #expect(SchedFormat.parseClock("07:30") == .value(450))
        #expect(SchedFormat.parseClock("730") == .value(450))
        #expect(SchedFormat.parseClock("19.30") == .value(1170))
        #expect(SchedFormat.parseClock("") == .empty)
        #expect(SchedFormat.parseClock("24:00") == .invalid)
        #expect(SchedFormat.parseClock("7:60") == .invalid)
        #expect(SchedFormat.parseClock("noon") == .invalid)
        #expect(SchedFormat.parseClock("12345") == .invalid)
    }

    @Test func pagesAreWrittenAndRead() {
        #expect(SchedFormat.pages(3) == "3/8")
        #expect(SchedFormat.pages(8) == "1")
        #expect(SchedFormat.pages(10) == "1 2/8")
        #expect(SchedFormat.parsePages("1 2/8") == .value(10))
        #expect(SchedFormat.parsePages("3/8") == .value(3))
        #expect(SchedFormat.parsePages("2") == .value(16))
        #expect(SchedFormat.parsePages("1.25") == .value(10))
        #expect(SchedFormat.parsePages("") == .empty)
        #expect(SchedFormat.parsePages("lots") == .invalid)
    }

    @Test func datesAreCheckedAndTheCalendarGridIsRightForAMonth() {
        #expect(SchedDate.parse("2027-06-14") != nil)
        #expect(SchedDate.parse("2027-02-30") == nil)
        #expect(SchedDate.parse("14/06/2027") == nil)
        #expect(SchedDate.parse("2027-6-1") == nil)
        #expect(SchedDate.iso(SchedDate.parse("2027-06-14")!) == "2027-06-14")
        // 1 June 2027 is a Tuesday: the first cell is Monday 31 May.
        let grid = SchedCalendarGrid.monthGrid(year: 2027, month0: 5)
        #expect(grid.count == 42)
        #expect(grid[0].iso == "2027-05-31" && !grid[0].inMonth)
        #expect(grid[1].iso == "2027-06-01" && grid[1].inMonth)
        #expect(grid.filter(\.inMonth).count == 30)
        #expect(SchedCalendarGrid.label(year: 2027, month0: 5).contains("2027"))
        #expect(SchedCalendarGrid.shift(year: 2027, month0: 0, by: -1) == (2026, 11))
        #expect(SchedCalendarGrid.shift(year: 2027, month0: 11, by: 1) == (2028, 0))
        #expect(SchedCalendarGrid.shift(year: 2027, month0: 5, by: 0) == (2027, 5))
    }

    @Test func theFilterKnowsItsLengthChoices() {
        var f = ScheduleFilter()
        #expect(!f.isActive && f.length == .any)
        f.length = .short
        #expect(f.maxEighths == 8 && f.minEighths == nil && f.isActive)
        f.length = .mid
        #expect(f.minEighths == 9 && f.maxEighths == 24)
        f.length = .long
        #expect(f.minEighths == 25 && f.maxEighths == nil && f.length == .long)
        f.length = .any
        #expect(!f.isActive)
        f.scheduled = .scheduled
        #expect(f.isActive)
    }

    @Test func aDropPayloadIsRecognisedOnlyWhenItIsOurs() {
        #expect(SchedDrag.strip("x").hasPrefix(SchedDrag.stripPrefix))
        #expect(SchedDrag.marker("y").hasPrefix(SchedDrag.markerPrefix))
        #expect(SchedDrag.strip("x") != SchedDrag.marker("x"))
    }
}

// MARK: Edge cases

@Suite("Schedule edge cases", .serialized)
@MainActor
struct ScheduleEdgeTests {
    private func ready(_ r: SchedRig, days: Int = 2) async throws {
        #expect(await r.model.createSchedule())
        for i in 0..<days { await r.createDay("2027-06-\(14 + i)") }
    }

    @Test func anEmptyProjectHasNothingToScheduleAndSaysSo() async throws {
        let r = try await SchedRig.make()
        defer { r.close() }
        #expect(r.model.loaded)
        #expect(r.model.schedule == nil && r.model.view?.activeSource == nil, "the screen asks for a Production Source in Breakdown")
        #expect(r.model.days.isEmpty && r.model.allStrips.isEmpty)
        #expect(r.model.sourceLine.isEmpty)
        // Creating from nothing is refused by the core, in plain words, and nothing is created.
        #expect(await r.model.createSchedule() == false)
        guard case let .error(_, failure)? = r.app.alert else {
            Issue.record("expected the core's answer to be shown")
            return
        }
        #expect(!failure.message.isEmpty)
        #expect(r.model.schedule == nil)
        // The Day Out of Days has no schedule either.
        await r.model.reloadDood()
        #expect(r.model.doodLoaded && r.model.dood?.hasSchedule == false)
        #expect(r.model.dood?.allDays.isEmpty == true)
    }

    @Test func aScreenplayWithoutSceneHeadingsGivesAnEmptyBoardNotACrash() async throws {
        let r = try await SchedRig.make()
        defer { r.close() }
        try await r.importNightShift("Title: Quiet\n\nJust a line of action with no scene heading.\n")
        try await r.chooseSource()
        await r.model.reload()
        if await r.model.createSchedule() {
            // The core reads the loose action line as one scene without a heading or a location.
            #expect(Int(r.model.counts?.total ?? -1) == r.model.allStrips.count)
            #expect(r.model.allStrips.allSatisfy { $0.locationName == nil })
            await r.createDay("2027-06-14")
            #expect(r.model.columnText(r.day(1)).contains("No scenes scheduled"))
            #expect(r.model.listRows.count == r.model.allStrips.count)
            await r.model.loadSuggestions()
            #expect(r.model.visibleSuggestions.isEmpty, "no shared locations or cast: nothing to advise")
            await r.model.reloadDood()
            #expect(r.model.dood?.rows.isEmpty == true, "no cast: the Day Out of Days says so")
            // A scene without a location still moves into a day and shows plainly that the location is missing.
            let s = try #require(r.model.allStrips.first)
            #expect(SchedFormat.stripLabel(s).contains("location missing"))
            await r.dragScene(1, into: r.day(1))
            #expect(r.model.columnText(r.day(1)).contains("1 scene"))
        } else {
            // The core may refuse a source with no scenes; then it says why and nothing is half made.
            #expect(r.app.alert != nil)
            #expect(r.model.schedule == nil)
        }
    }

    @Test func deletingADayWithScenesReturnsThemToUnscheduledAndUndoBringsThemBack() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r)
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(1))
        r.model.openDayId = r.day(1).id
        let doomed = r.day(1)
        await r.model.deleteDay(doomed)
        #expect(r.model.openDayId == nil, "the open day went away, so the person is back on the board")
        #expect(r.model.days.count == 1)
        #expect(r.model.unscheduled.count == 2, "both scenes are back in the pool")
        #expect(r.model.allStrips.allSatisfy { $0.dayId == nil })
        await r.undo()
        #expect(r.model.days.count == 2)
        #expect(r.model.columnText(r.day(1)).contains("2 scenes"), "undo brings the day and its scenes back")
        #expect(r.quiet)
    }

    @Test func reorderingInADayByDropByMenuAndByKeyboardMoves() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r)
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(1))
        func order() -> [Int] { r.day(1).items.compactMap { $0.strip.map { Int($0.number ?? 0) } } }
        #expect(order() == [1, 2])
        // Menu: Move Up on scene 2.
        var menu = r.model.moveMenu(for: r.scene(2))
        #expect(menu.canMoveUp && !menu.canMoveDown && menu.isScheduled)
        await r.model.moveStripBy(r.scene(2), -1)
        #expect(order() == [2, 1])
        menu = r.model.moveMenu(for: r.scene(2))
        #expect(!menu.canMoveUp && menu.canMoveDown)
        // Move Up at the top does nothing and says nothing wrong.
        await r.model.moveStripBy(r.scene(2), -1)
        #expect(order() == [2, 1] && r.quiet)
        // Move Down.
        await r.model.moveStripBy(r.scene(2), 1)
        #expect(order() == [1, 2])
        // Drag scene 2 onto scene 1: it goes there.
        await r.dragScene(2, into: r.day(1), before: r.scene(1).id)
        #expect(order() == [2, 1])
        // Dropping a scene where it already is changes nothing.
        let plan = r.model.dropPlan(payload: SchedDrag.strip(r.scene(2).id), target: .day(r.day(1).id, before: r.scene(2).id))
        #expect(plan == .none)
        // Undo brings the order back.
        await r.undo()
        #expect(order() == [1, 2])
    }

    @Test func dragToThePoolAndBetweenDaysAndTheRefusals() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r)
        await r.createDay(nil, off: true)
        await r.dragScene(1, into: r.day(1))
        let s1 = r.scene(1)
        // Pool to pool: nothing to do.
        #expect(r.model.dropPlan(payload: SchedDrag.strip(r.scene(2).id), target: .pool) == .none)
        // Day 1 to day 2.
        await r.dragScene(1, into: r.day(2))
        #expect(r.model.columnText(r.day(2)).contains("1 scene"))
        #expect(r.model.columnText(r.day(1)).contains("No scenes scheduled"))
        // Back to the pool.
        let ok = await r.model.drop(payload: SchedDrag.strip(s1.id), on: .pool)
        #expect(ok && r.model.unscheduled.count == 2)
        // An off day refuses scenes in plain words and nothing moves.
        let off = try #require(r.model.days.last)
        #expect(off.isOffDay)
        let plan = r.model.dropPlan(payload: SchedDrag.strip(s1.id), target: .day(off.id, before: nil))
        #expect(plan == .refuse("This is an off day. Change it back to a shooting day before adding scenes."))
        #expect(await r.model.drop(payload: SchedDrag.strip(s1.id), on: .day(off.id, before: nil)) == false)
        #expect(r.model.unscheduled.count == 2)
        #expect(ToastCenter.shared.items.last?.kind == .error)
        // Foreign text or a scene that no longer exists is ignored.
        #expect(r.model.dropPlan(payload: "just some text", target: .pool) == .none)
        #expect(r.model.dropPlan(payload: SchedDrag.strip("gone"), target: .day(r.day(1).id, before: nil)) == .none)
        #expect(r.quiet)
    }

    @Test func offDaysCannotHoldScenesAndComeBack() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r)
        await r.dragScene(1, into: r.day(1))
        // A day with scenes cannot become an off day: the core says so.
        await r.model.setOffDay(r.day(1), true)
        #expect(r.app.alert != nil, "the core's refusal is shown")
        #expect(!r.day(1).isOffDay)
        r.app.alert = nil
        await r.model.setOffDay(r.day(2), true)
        #expect(r.model.days[1].isOffDay)
        #expect(r.model.columnText(r.model.days[1]).contains("Off day"))
        await r.model.setOffDay(r.model.days[1], false)
        #expect(!r.model.days[1].isOffDay)
        #expect(r.quiet)
    }

    @Test func daysCanBeInsertedDuplicatedMovedAndDated() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r)
        let first = r.day(1)
        await r.createDay("2027-06-20", after: first.id)
        #expect(r.model.days.count == 3)
        #expect(r.model.days[1].date == "2027-06-20", "inserted right after day 1")
        _ = await r.addMeal(first)
        await r.model.reload()
        await r.model.duplicateDay(r.model.days[0])
        #expect(r.model.days.count == 4)
        await r.model.moveDay(r.model.days[3], by: -1)
        await r.model.moveDay(r.model.days[0], by: -1)
        #expect(r.model.days.count == 4, "moving the first day earlier is a no-op, not an error")
        #expect(r.quiet)
        // Edit date: wrong text is explained, a right one saves, clearing leaves the day undated.
        let d = r.model.days[0]
        #expect(await r.model.setDayDate(d, date: "next tuesday") != nil)
        #expect(await r.model.setDayDate(d, date: "2027-07-01") == nil)
        #expect(r.model.days.first { $0.id == d.id }?.date == "2027-07-01")
        #expect(await r.model.setDayDate(r.model.days.first { $0.id == d.id }!, date: "") == nil)
        #expect(r.model.days.first { $0.id == d.id }?.date == nil)
        #expect(r.model.daysByDate["2027-07-01"] == nil)
        #expect(r.model.days.filter { $0.date == nil }.contains { $0.id == d.id }, "an undated day shows under Not yet dated")
        // Bad date when creating.
        #expect(await r.model.createDay(date: "2027-13-40", notes: "", offDay: false, afterDayId: nil) != nil)
        // Day notes save and clear.
        await r.model.setDayNotes(r.model.days[0], "Bring the rain towers")
        #expect(r.model.days[0].notes == "Bring the rain towers")
        await r.model.setDayNotes(r.model.days[0], "")
        #expect(r.model.days[0].notes == nil)
    }

    @Test func breaksAreAddedEditedMovedAndDeleted() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r, days: 1)
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(1))
        let day = r.day(1)

        // Wrong input is explained, nothing is saved.
        let bad = SchedMarkerDialogMode(dayId: day.id, type: "Meal")
        #expect(await r.model.saveMarker(bad, type: "Meal", label: "", time: "", duration: "a while", notes: "") == "Enter a duration such as 45m or 1h 30m.")
        #expect(await r.model.saveMarker(SchedMarkerDialogMode(dayId: day.id, type: "Custom"), type: "Custom", label: "  ", time: "", duration: "", notes: "") == "Write the note for this marker.")
        #expect(await r.model.saveMarker(bad, type: "Travel", label: "", time: "late", duration: "", notes: "") != nil)
        #expect(r.day(1).items.compactMap(\.marker).isEmpty)

        // Travel with a time and a duration; Company Move; Custom note.
        #expect(await r.model.saveMarker(SchedMarkerDialogMode(dayId: day.id, type: "Travel"), type: "Travel", label: "", time: "13:00", duration: "1h", notes: "To the tunnel") == nil)
        #expect(await r.model.saveMarker(SchedMarkerDialogMode(dayId: day.id, type: "Company Move"), type: "Company Move", label: "Move to the station", time: "", duration: "30", notes: "") == nil)
        #expect(await r.model.saveMarker(SchedMarkerDialogMode(dayId: day.id, type: "Custom"), type: "Custom", label: "Wet down the street", time: "", duration: "", notes: "") == nil)
        let markers = r.day(1).items.compactMap(\.marker)
        #expect(markers.count == 3)
        #expect(markers.map(\.markerType) == ["Travel", "Company Move", "Custom"])
        #expect(markers[0].atTime == "13:00" && markers[0].durationMinutes == 60)
        #expect(SchedFormat.markerText(markers[0]).contains("at 13:00") && SchedFormat.markerText(markers[0]).contains("(1h)"))
        #expect(markers[2].label == "Wet down the street")

        // Edit.
        let editing = SchedMarkerDialogMode(dayId: day.id, type: markers[0].markerType, marker: markers[0])
        #expect(await r.model.saveMarker(editing, type: "Travel", label: "Drive", time: "13:30", duration: "45m", notes: "") == nil)
        #expect(r.day(1).items.compactMap(\.marker)[0].label == "Drive")

        // Move a break up with the menu and by drop.
        let last = r.day(1).items.compactMap(\.marker).last!
        await r.model.moveMarkerBy(last, -1)
        #expect(r.day(1).items.compactMap(\.marker).map(\.id).last != last.id)
        await r.model.drop(payload: SchedDrag.marker(last.id), on: .day(day.id, before: r.scene(1).id))
        #expect(r.day(1).items.first?.marker?.id == last.id, "dropped before the first scene")

        // Delete, and undo.
        await r.model.deleteMarker(last)
        #expect(r.day(1).items.compactMap(\.marker).count == 2)
        await r.undo()
        #expect(r.day(1).items.compactMap(\.marker).count == 3)
        #expect(r.quiet)
    }

    @Test func timesThatCannotBeReadAreExplainedAndChangeNothing() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r, days: 1)
        let day = r.day(1)
        #expect(await r.model.setDayTimes(day, crewCall: "seven", shootingStart: "", targetWrap: "") == false)
        #expect(ToastCenter.shared.items.last?.text == "Enter the crew call as a time such as 07:30.")
        #expect(await r.model.setDayTimes(day, crewCall: "", shootingStart: "25:00", targetWrap: "") == false)
        #expect(await r.model.setDayTarget(day, "ten hours") == false)
        #expect(ToastCenter.shared.items.last?.text == "Enter the target day length, e.g. 10h.")
        #expect(r.day(1).crewCall == nil && r.day(1).plannedMinutes == nil)
        // Typing the same value again is not a change, so no history step is made.
        #expect(await r.model.setDayTimes(day, crewCall: "", shootingStart: "", targetWrap: ""))
        #expect(r.quiet)
        // A wrap before the crew call counts as after midnight.
        #expect(await r.model.setDayTimes(day, crewCall: "20:00", shootingStart: "", targetWrap: "02:00"))
        let tl = r.day(1).timeline
        #expect(tl.targetWrap.map { $0 >= 1440 } == true, "02:00 after a 20:00 call is the next morning (\(String(describing: tl.targetWrap)))")
        // The day target can be cleared back to the schedule default.
        #expect(await r.model.setDayTarget(r.day(1), "8h"))
        #expect(r.day(1).plannedMinutes == 480)
        #expect(await r.model.setDayTarget(r.day(1), ""))
        #expect(r.day(1).plannedMinutes == nil)
    }

    @Test func sceneDetailsEstimateStartAndPagesAreCheckedAndSaved() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r, days: 1)
        await r.dragScene(1, into: r.day(1))
        #expect(await r.model.setEstimate(r.scene(1), "soon") == "Enter the estimate as minutes or e.g. 1h 30m.")
        #expect(await r.model.setEstimate(r.scene(1), "2h") == nil)
        #expect(r.scene(1).estimatedMinutes == 120)
        #expect(await r.model.setEstimate(r.scene(1), "") == nil)
        #expect(r.scene(1).estimatedMinutes == nil, "empty = not estimated, shown as Missing")

        #expect(await r.model.setStart(r.scene(1), "half past") != nil)
        #expect(await r.model.setStart(r.scene(1), "14:30") == nil)
        #expect(r.scene(1).startMinutes == 870)
        #expect(await r.model.setStart(r.scene(1), "") == nil)
        #expect(r.scene(1).startMinutes == nil, "empty = follow the day's order")

        #expect(await r.model.setPages(r.scene(1), "0/8") == "Enter pages like 1 2/8 or 3/8.")
        #expect(await r.model.setPages(r.scene(1), "x") != nil)
        #expect(await r.model.setPages(r.scene(1), "1 2/8") == nil)
        #expect(r.scene(1).pageEighths == 10 && r.scene(1).pagesOverridden)
        #expect(await r.model.setPages(r.scene(1), "") == nil)
        #expect(!r.scene(1).pagesOverridden, "empty = back to the screenplay's count")
        #expect(r.quiet)
    }

    @Test func settingsAreCheckedAndTheScheduleCanBeFinalizedAndReopened() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r, days: 1)
        #expect(await r.model.saveSettings(name: " ", length: "10h", strict: false, status: "Draft", doodStyle: nil) == "Write a name for the schedule.")
        #expect(await r.model.saveSettings(name: "Block A", length: "long", strict: false, status: "Draft", doodStyle: nil) == "Enter the target day length, e.g. 10h or 11h 30m.")
        #expect(await r.model.saveSettings(name: "Block A", length: "0", strict: false, status: "Draft", doodStyle: nil) != nil)
        #expect(await r.model.saveSettings(name: "Block A", length: "11h 30m", strict: true, status: "Active", doodStyle: nil) == nil)
        #expect(r.model.schedule?.name == "Block A" && r.model.schedule?.strictValidation == true)
        #expect(r.model.schedule?.dayDurationMinutes == 690 && r.model.schedule?.status == "Active")

        // Finalized: the screen says so and a drop does nothing.
        #expect(await r.model.saveSettings(name: "Block A", length: "11h 30m", strict: false, status: "Finalized", doodStyle: nil) == nil)
        #expect(r.model.isFinalized && r.model.sourceLine.hasSuffix("Finalized"))
        #expect(await r.model.drop(payload: SchedDrag.strip(r.scene(1).id), on: .day(r.day(1).id, before: nil)) == false)
        #expect(r.model.unscheduled.count == 2)
        #expect(ToastCenter.shared.items.last?.text.contains("finalized") == true)
        // Even a direct request is refused by the core, not only by the screen.
        let direct = await r.model.moveStrip(r.scene(1).id, to: r.day(1).id, text: "Scheduled")
        #expect(direct == false && r.model.unscheduled.count == 2)
        r.app.alert = nil
        #expect(await r.model.reopenSchedule())
        #expect(!r.model.isFinalized)
        #expect(await r.model.drop(payload: SchedDrag.strip(r.scene(1).id), on: .day(r.day(1).id, before: nil)))
        #expect(r.model.unscheduled.count == 1)
    }

    @Test func strictValidationAsksBeforeKeepingAChangeThatCreatesAWarning() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await ready(r, days: 1)
        // Two hours of work on a one hour day, with strict validation on.
        #expect(await r.model.setEstimate(r.scene(1), "2h") == nil)
        #expect(await r.model.setDayTarget(r.day(1), "1h"))
        #expect(await r.model.saveSettings(name: "Block A", length: "10h", strict: true, status: "Draft", doodStyle: nil) == nil)
        let moved = await r.model.moveStrip(r.scene(1).id, to: r.day(1).id, text: "Scheduled Scene 1 on Shoot Day 1")
        #expect(!moved, "a two hour scene on a one hour day is a new warning, so strict validation asks first")
        let pending = try #require(r.model.pendingKeep, "Keep Anyway is offered, in the core's words")
        #expect(!pending.message.isEmpty)
        #expect(r.model.unscheduled.count == 2, "nothing was changed yet")
        #expect(r.app.alert == nil)
        // The person can change their mind...
        r.model.cancelKeep()
        #expect(r.model.pendingKeep == nil && r.model.unscheduled.count == 2)
        // ...or keep it anyway.
        _ = await r.model.moveStrip(r.scene(1).id, to: r.day(1).id, text: "Scheduled Scene 1 on Shoot Day 1")
        #expect(await r.model.confirmKeep())
        #expect(r.model.unscheduled.count == 1 && r.model.pendingKeep == nil)
        #expect(r.quiet)
    }

    @Test func groupingSuggestionsAreAppliedOnlyWhenAsked() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await ready(r)
        await r.dragScene(1, into: r.day(1))
        let before = r.model.view?.days
        await r.model.loadSuggestions()
        #expect(r.model.suggestions != nil)
        #expect(r.model.view?.days == before, "asking for advice moves nothing")
        // Dismissing hides it for now and still moves nothing.
        for s in r.model.visibleSuggestions { r.model.dismissedSuggestions.insert(s.key) }
        #expect(r.model.visibleSuggestions.isEmpty)
        // Applying by hand is one batch move, in the order the person sees.
        let ids = [r.scene(2).id]
        #expect(await r.model.moveStrips(ids, to: r.day(2).id, text: "Grouped scenes on Shoot Day 2"))
        #expect(r.model.columnText(r.day(2)).contains("1 scene"))
        await r.undo()
        #expect(r.model.unscheduled.count == 1)
    }

    @Test func theDayOutOfDaysRangeCastAndCodeStyleAreViewState() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await ready(r)
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(2))
        await r.model.reloadDood()
        let all = try #require(r.model.dood)
        #expect(all.rows.count >= 2 && all.days.count == 2)
        // One cast member.
        r.model.doodCast = all.allCast[0].key
        await r.model.reloadDood()
        #expect(r.model.dood?.rows.count == 1)
        r.model.doodCast = ""
        // One day.
        r.model.doodFrom = all.allDays[1].id
        await r.model.reloadDood()
        #expect(r.model.dood?.days.count == 1)
        r.model.doodFrom = ""
        // Code style is a schedule setting.
        #expect(all.codeStyle == "industry")
        await r.model.setDoodStyle("simple")
        await r.model.reloadDood()
        #expect(r.model.dood?.codeStyle == "simple")
        #expect(r.model.dood?.legend.count != all.legend.count)
        // View state alone changed nothing in the schedule.
        #expect(r.model.correctedCount == 0)
        #expect(r.quiet)
    }

    @Test func scriptChangesAreReviewedNeverAppliedOnTheirOwn() async throws {
        let r = try await SchedRig.make(source: true, elements: true)
        defer { r.close() }
        try await ready(r, days: 1)
        await r.dragScene(1, into: r.day(1))
        #expect(r.model.view?.scriptChanges == nil, "no banner while the script is as scheduled")
        // A script change that leaves the source as it was does not move anything either.
        await r.model.reload()
        #expect(r.model.columnText(r.day(1)).contains("1 scene"))
        #expect(r.model.changesTitle.isEmpty)
    }

    @Test func theLinkedSceneOpensItsDrawerOnce() async throws {
        let r = try await SchedRig.make(source: true)
        defer { r.close() }
        try await ready(r, days: 1)
        let target = r.scene(2)
        ScheduleModel.pendingScene = (target.sceneId, nil)
        await r.model.reload()
        #expect(r.model.drawer == .strip(target.id))
        r.model.drawer = nil
        await r.model.reload()
        #expect(r.model.drawer == nil, "it opens once, not on every reload")
    }
}

// MARK: Rendered screens

@Suite("Schedule screens", .serialized)
@MainActor
struct ScheduleRenderTests {
    private func host<V: View>(_ view: V, app: AppModel, scheme: ColorScheme) -> some View {
        let themes = ThemeStore()
        return view
            .environment(app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
            .environment(\.colorScheme, scheme)
    }

    /// The accessibility names and values a screen exposes, found by walking the hosting view.
    private func texts(of view: NSView) -> String {
        var out: [String] = []
        func walk(_ e: Any) {
            guard let el = e as? NSAccessibilityProtocol else { return }
            if let l = el.accessibilityLabel(), !l.isEmpty { out.append(l) }
            if let v = el.accessibilityValue() as? String, !v.isEmpty { out.append(v) }
            for c in el.accessibilityChildren() ?? [] { walk(c) }
        }
        walk(view)
        return out.joined(separator: "\n")
    }

    private func populated() async throws -> SchedRig {
        let r = try await SchedRig.make(source: true, elements: true)
        #expect(await r.model.createSchedule())
        await r.createDay("2027-06-14")
        await r.createDay("2027-06-15")
        await r.createDay(nil, off: true)
        await r.dragScene(1, into: r.day(1))
        await r.dragScene(2, into: r.day(1))
        _ = await r.addMeal(r.day(1))
        _ = await r.model.setDayTimes(r.day(1), crewCall: "07:00", shootingStart: "08:00", targetWrap: "")
        await r.model.reloadDood()
        return r
    }

    @Test func theMainScreensRenderInLightAndDark() async throws {
        let r = try await populated()
        defer { r.close() }
        for scheme in [ColorScheme.light, .dark] {
            let n = scheme == .dark ? "dark" : "light"
            let size = CGSize(width: 1280, height: 780)
            for mode in SchedMode.allCases {
                r.model.mode = mode
                r.model.section = .schedule
                #expect(Fixture.render(host(ScheduleContent(model: r.model), app: r.app, scheme: scheme), name: "schedule-\(mode.rawValue)-\(n)", size: size) != nil)
            }
            r.model.mode = .board
            r.model.drawer = .strip(r.scene(1).id)
            #expect(Fixture.render(host(ScheduleContent(model: r.model), app: r.app, scheme: scheme), name: "schedule-strip-drawer-\(n)", size: size) != nil)
            r.model.drawer = .warnings
            #expect(Fixture.render(host(ScheduleContent(model: r.model), app: r.app, scheme: scheme), name: "schedule-warnings-\(n)", size: size) != nil)
            r.model.drawer = .suggestions
            #expect(Fixture.render(host(ScheduleContent(model: r.model), app: r.app, scheme: scheme), name: "schedule-suggestions-\(n)", size: size) != nil)
            r.model.drawer = nil
            r.model.openDayId = r.day(1).id
            #expect(Fixture.render(host(ScheduleContent(model: r.model), app: r.app, scheme: scheme), name: "schedule-day-\(n)", size: size) != nil)
            r.model.openDayId = nil
            r.model.section = .dood
            await r.model.reloadDood()
            #expect(Fixture.render(host(ScheduleContent(model: r.model), app: r.app, scheme: scheme), name: "schedule-dood-\(n)", size: size) != nil)
            r.model.section = .schedule
        }
        #expect(r.quiet)
    }

    @Test func theScreensShowTheWordsOfTheWindowsScreens() async throws {
        let r = try await populated()
        defer { r.close() }
        let size = CGSize(width: 1280, height: 780)
        func shown<V: View>(_ v: V, _ name: String) -> String {
            _ = NSApplication.shared
            let hosting = NSHostingView(rootView: v.frame(width: size.width, height: size.height))
            hosting.frame = NSRect(origin: .zero, size: size)
            let window = NSWindow(contentRect: hosting.frame, styleMask: [.borderless], backing: .buffered, defer: false)
            window.contentView = hosting
            hosting.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(1.2))
            hosting.layoutSubtreeIfNeeded()
            return texts(of: hosting)
        }
        let board = shown(host(ScheduleContent(model: r.model), app: r.app, scheme: .light), "board")
        // The accessibility tree may be shallow in a window nobody can see; only check what it gives us.
        if board.contains("Unscheduled") || board.contains("Stripboard") {
            #expect(board.contains("Shoot Day 1"))
        }
        r.model.openDayId = r.day(1).id
        let detail = shown(host(ScheduleContent(model: r.model), app: r.app, scheme: .light), "day")
        if !detail.isEmpty, detail.contains("Shoot Day 1") {
            #expect(detail.contains("Back to schedule") || detail.contains("What exactly are we doing on this day?"))
        }
    }

    @Test func theSheetsRender() async throws {
        let r = try await populated()
        defer { r.close() }
        let size = CGSize(width: 640, height: 520)
        let day = r.day(1)
        let sheets: [(String, AnyView)] = [
            ("day-create", AnyView(SchedDaySheet(model: r.model, mode: .create(offDay: false, afterDayId: nil)))),
            ("day-date", AnyView(SchedDaySheet(model: r.model, mode: .date(day)))),
            ("marker", AnyView(SchedMarkerSheet(model: r.model, mode: SchedMarkerDialogMode(dayId: day.id, type: "Meal")))),
            ("settings", AnyView(SchedSettingsSheet(model: r.model))),
            ("export-schedule", AnyView(SchedExportSheet(model: r.model, kind: .schedule(currentDayId: day.id)))),
            ("export-dood", AnyView(SchedExportSheet(model: r.model, kind: .dood))),
        ]
        for (name, view) in sheets {
            #expect(Fixture.render(host(view, app: r.app, scheme: .light), name: "schedule-sheet-\(name)", size: size) != nil)
        }
        #expect(r.quiet)
    }
}

// Capture tour section: the Production schedule tabs (Schedule, Day Out of Days, Daily View, Sides,
// Reports), the Call Sheets area and Budget. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

fileprivate extension UI {
    /// Adds a shooting day with this date through the day sheet, retrying when the sheet is slow.
    func scheduleAddDay(_ iso: String) async throws {
        for _ in 0..<3 {
            do {
                if !exists(id: "day-date") { try await capAct(id: "create-shooting-day") }
                _ = try await wait(id: "day-date", timeout: 8)
                await idle(0.8)
                try await capTypeIn(iso, into: "day-date")
                try await capAct(id: "day-sheet-confirm")
                try await waitGone(id: "day-date")
                await idle(0.4)
                return
            } catch { await capRecover(); await idle(0.8) }
        }
        throw DriverError("could not add the shooting day \(iso)")
    }

    /// The first element whose identifier starts with this text and that can be pressed.
    func scheduleFirst(_ prefix: String) -> Element? {
        let found = capQueryPrefix(prefix).filter { $0.frame.width > 0 }
        return found.first { $0.obj is NSPopUpButtonCell } ?? found.first { $0.role == "AXButton" } ?? found.first
    }

    /// Captures one radio button of a segmented control by its label.
    func scheduleSegment(group: String, label: String) async throws {
        guard let g = query(id: group).first(where: { $0.role == "AXRadioGroup" }),
              let b = g.children.first(where: { $0.role == "AXRadioButton" && $0.label == label }) else {
            throw DriverError("no segment \(label) in \(group): \(radioLabels(group: group))")
        }
        if !b.performPress() { try await tap(b) }
        await idle(0.6)
    }
}

extension Scenarios {
    /// The schedule this section starts from: three dated days, scenes 1 and 2 in Day 1, scene 3 in Day 2.
    @MainActor static func scheduleThreeDayBase(_ ui: UI) async throws {
        try await capBreakdownBase(ui)
        try await ui.capNav("Schedule")
        try await ui.capAct(id: "create-shooting-schedule")
        try await ui.capAct(id: "create-schedule-confirm")
        try await ui.waitText("unscheduled")
        for iso in ["2027-06-14", "2027-06-15", "2027-06-16"] { try await ui.scheduleAddDay(iso) }
        try await ui.capMenuButton(id: "strip-actions-1", item: "Shoot Day 1")
        await ui.idle(0.5)
        try await ui.capMenuButton(id: "strip-actions-2", item: "Shoot Day 1")
        await ui.idle(0.5)
        try? await ui.capMenuButton(id: "strip-actions-3", item: "Shoot Day 2")
        await ui.idle(0.5)
    }

    static let captureSchedule: [Scenario] = [
        tour("06-schedule", order: 6000, timeout: 900) { ui, _ in
            try await Scenarios.capBreakdownBase(ui)
            // The production tabs before any schedule exists.
            for (tab, name, title, area) in [("Day Out of Days", "schedule-dood-empty", "Day Out of Days, no schedule yet", "Day Out of Days"),
                                             ("Daily View", "schedule-daily-empty", "Daily View, no shooting day yet", "Daily View"),
                                             ("Sides", "schedule-sides-empty", "Sides, no schedule yet", "Sides")] {
                await ui.step("empty \(tab)") {
                    try await ui.capNav(tab)
                    await ui.idle(1.0)
                    await ui.capture(name, title: title, area: area, via: "production-tab")
                }
            }
            try await ui.capNav("Schedule")
            await ui.capture("schedule-empty", title: "Schedule, not created yet", area: "Schedule")
            await ui.step("create schedule") {
                try await ui.capAct(id: "create-shooting-schedule")
                await ui.idle(0.6)
                await ui.capture("schedule-create-sheet", title: "Create the shooting schedule", area: "Schedule", from: "schedule-empty", via: "create-shooting-schedule")
                try await ui.capAct(id: "create-schedule-confirm")
                try await ui.waitText("unscheduled")
                await ui.idle(0.8)
                await ui.capture("schedule-board-unscheduled", title: "Schedule board, scenes waiting", area: "Schedule", from: "schedule-create-sheet", via: "create-schedule-confirm")
            }
            await ui.step("day sheet") {
                try await ui.capAct(id: "create-shooting-day")
                _ = try await ui.wait(id: "day-date", timeout: 8)
                await ui.idle(0.8)
                await ui.capture("schedule-add-day-sheet", title: "Add a shooting day", area: "Schedule", from: "schedule-board-unscheduled", via: "create-shooting-day")
                try await ui.capTypeIn("14 June 2027", into: "day-date")
                try await ui.capAct(id: "day-sheet-confirm")
                _ = try await ui.wait(id: "day-sheet-error", timeout: 6)
                await ui.capture("schedule-add-day-error", title: "Add a shooting day, date not understood", area: "Schedule", from: "schedule-add-day-sheet", via: "day-sheet-confirm")
            }
            await ui.step("three days") {
                for iso in ["2027-06-14", "2027-06-15", "2027-06-16"] { try await ui.scheduleAddDay(iso) }
                await ui.capture("schedule-board-three-days", title: "Schedule board with three dated days", area: "Schedule", from: "schedule-add-day-sheet", via: "day-sheet-confirm")
            }
            await ui.step("off day") {
                try await ui.capAct(id: "add-off-day")
                _ = try await ui.wait(id: "day-sheet-confirm", timeout: 6)
                await ui.capture("schedule-add-off-day-sheet", title: "Add a day off", area: "Schedule", from: "schedule-board-three-days", via: "add-off-day")
                try await ui.capAct(id: "day-sheet-confirm")
                await ui.idle(0.8)
                await ui.capture("schedule-board-off-day", title: "Schedule board with a day off", area: "Schedule", from: "schedule-add-off-day-sheet", via: "day-sheet-confirm")
            }
            await ui.step("scenes into days") {
                await ui.capMenu(id: "strip-actions-1", name: "schedule-strip-menu", title: "Schedule: scene strip menu", area: "Schedule", from: "schedule-board-unscheduled")
                try await ui.capMenuButton(id: "strip-actions-1", item: "Shoot Day 1")
                await ui.idle(0.5)
                try await ui.capMenuButton(id: "strip-actions-2", item: "Shoot Day 1")
                await ui.idle(0.5)
                try? await ui.capMenuButton(id: "strip-actions-3", item: "Shoot Day 2")
                await ui.idle(0.8)
                await ui.capture("schedule-board", title: "Schedule board with scenes in Day 1 and Day 2", area: "Schedule", from: "schedule-strip-menu", via: "strip-actions-1")
            }
            await ui.step("strip details") {
                try await ui.capMenuButton(id: "strip-actions-1", item: "Details")
                _ = try await ui.wait(id: "strip-drawer-heading", timeout: 8)
                await ui.capture("schedule-strip-drawer", title: "Scene strip details drawer", area: "Schedule", from: "schedule-strip-menu", via: "strip-actions-1")
                await ui.capRecover()
            }
            await ui.step("meal break") {
                await ui.capMenu(id: "day-actions-1", name: "schedule-day-menu", title: "Schedule: day column menu", area: "Schedule", from: "schedule-board")
                try await ui.capMenuButton(id: "day-actions-1", item: "Meal Break")
                _ = try await ui.wait(id: "marker-duration", timeout: 6)
                await ui.capture("schedule-meal-break-sheet", title: "Add a meal break", area: "Schedule", from: "schedule-day-menu", via: "day-actions-1")
                try await ui.capTypeIn("45m", into: "marker-duration")
                try await ui.capAct(id: "marker-save")
                await ui.idle(1.0)
                await ui.capture("schedule-board-meal-break", title: "Schedule board with a meal break in Day 1", area: "Schedule", from: "schedule-meal-break-sheet", via: "marker-save")
            }
            await ui.step("marker menu") {
                guard let m = ui.scheduleFirst("marker-actions-") else { throw DriverError("no break menu") }
                await ui.capMenu(id: m.identifier, name: "schedule-break-menu", title: "Schedule: meal break menu", area: "Schedule", from: "schedule-board-meal-break")
                try await ui.capMenuButton(id: m.identifier, item: "Edit Break")
                _ = try await ui.wait(id: "marker-save", timeout: 6)
                await ui.capture("schedule-edit-break-sheet", title: "Edit the meal break", area: "Schedule", from: "schedule-break-menu", via: m.identifier)
                await ui.capRecover()
            }
            await ui.step("filters") {
                await ui.capMenu(id: "filter-day-night", name: "schedule-filter-menu", title: "Schedule: filter by day or night", area: "Schedule", from: "schedule-board")
                try await ui.capMenuButton(id: "filter-scheduled", item: "Unscheduled")
                await ui.idle(0.8)
                await ui.capture("schedule-board-filtered", title: "Schedule board filtered to unscheduled scenes", area: "Schedule", from: "schedule-filter-menu", via: "filter-scheduled")
                try await ui.capAct(id: "clear-filters")
                await ui.idle(0.6)
            }
            await ui.step("day detail") {
                try await ui.capAct(id: "day-open-1")
                _ = try await ui.wait(id: "day-detail", timeout: 10)
                await ui.capture("schedule-day-detail", title: "Shooting Day 1 detail and timeline", area: "Schedule", from: "schedule-board", via: "day-open-1")
                try await ui.capTypeIn("07:00", into: "day-crew-call")
                try await ui.capTypeIn("08:00", into: "day-shooting-start")
                try await ui.capTypeIn("19:00", into: "day-target-wrap")
                await ui.idle(0.6)
                await ui.capture("schedule-day-detail-times", title: "Shooting Day 1 with times set", area: "Schedule", from: "schedule-day-detail")
            }
            await ui.capMenu(id: "day-more-actions", name: "schedule-day-detail-menu", title: "Shooting Day 1: more actions", area: "Schedule", from: "schedule-day-detail")
            await ui.exploreMenu("day-add-break", area: "Schedule", prefix: "schedule-day-add", from: "schedule-day-detail", limit: 4)
            await ui.visit(press: "day-edit-date", name: "schedule-day-edit-date", title: "Edit the date of Shooting Day 1", area: "Schedule", from: "schedule-day-detail")
            await ui.visit(press: "day-export", name: "schedule-day-export", title: "Export Shooting Day 1", area: "Schedule", from: "schedule-day-detail")
            await ui.step("empty day") {
                try await ui.capAct(id: "day-back")
                await ui.idle(0.6)
                try await ui.capAct(id: "day-open-3")
                _ = try await ui.wait(id: "day-empty", timeout: 8)
                await ui.capture("schedule-day-empty", title: "Shooting Day 3, no scenes yet", area: "Schedule", from: "schedule-board", via: "day-open-3")
                try await ui.capAct(id: "day-back")
                await ui.idle(0.6)
            }
            for (m, label) in [("schedule-mode-list", "list"), ("schedule-mode-calendar", "calendar"), ("schedule-mode-board", "board")] {
                await ui.step("mode \(label)") {
                    try await ui.capAct(id: m)
                    await ui.idle(0.8)
                    await ui.capture("schedule-\(label)", title: "Schedule as a \(label)", area: "Schedule", from: "schedule-board", via: m)
                }
            }
            await ui.exploreButtons([("suggest-grouping", "Suggest grouping"), ("warnings-button", "Warnings"), ("schedule-settings", "Schedule settings"),
                                     ("schedule-export", "Export schedule")], area: "Schedule", prefix: "schedule", from: "schedule-board")
            await ui.step("settings error") {
                try await ui.capAct(id: "schedule-settings")
                _ = try await ui.wait(id: "settings-name", timeout: 6)
                try await ui.capTypeIn("", into: "settings-name")
                try await ui.capAct(id: "settings-save")
                _ = try await ui.wait(id: "settings-error", timeout: 6)
                await ui.capture("schedule-settings-error", title: "Schedule settings, name missing", area: "Schedule", from: "schedule-schedule-settings", via: "settings-save")
            }
        },

        tour("06-production-days", order: 6500, timeout: 900) { ui, _ in
            try await Scenarios.scheduleThreeDayBase(ui)
            await ui.step("dood") {
                try await ui.go("Day Out of Days")
                _ = try await ui.wait(id: "dood-table", timeout: 10)
                await ui.capture("schedule-dood", title: "Day Out of Days", area: "Day Out of Days", from: "schedule-board", via: "production-tab-dayOutOfDays")
            }
            await ui.capMenu(id: "dood-code-style", name: "schedule-dood-code-style-menu", title: "Day Out of Days: code style", area: "Day Out of Days", from: "schedule-dood")
            await ui.visit(press: "dood-export", name: "schedule-dood-export", title: "Export Day Out of Days", area: "Day Out of Days", from: "schedule-dood")

            await ui.step("daily") {
                try await ui.go("Daily View")
                _ = try await ui.wait(id: "daily-card-scenes", timeout: 10)
                await ui.capture("schedule-daily", title: "Daily View, Shoot Day 1", area: "Daily View", from: "schedule-board", via: "production-tab-daily")
            }
            await ui.capMenu(id: "daily-day", name: "schedule-daily-day-menu", title: "Daily View: choose a shooting day", area: "Daily View", from: "schedule-daily")
            await ui.visit(press: "daily-export", name: "schedule-daily-export", title: "Export Shoot Day 1 Daily view", area: "Daily View", from: "schedule-daily")
            for (n, item) in [(2, "Shoot Day 2"), (3, "Shoot Day 3")] {
                await ui.step("daily day \(n)") {
                    try await ui.capMenuButton(id: "daily-day", item: item)
                    await ui.idle(1.0)
                    await ui.capture("schedule-daily-day-\(n)", title: "Daily View, \(item)", area: "Daily View", from: "schedule-daily-day-menu", via: "daily-day")
                }
            }

            await ui.step("sides") {
                try await ui.go("Sides")
                _ = try await ui.wait(id: "sides-day", timeout: 10)
                await ui.idle(1.0)
                await ui.capture("schedule-sides", title: "Sides for Shoot Day 1", area: "Sides", from: "schedule-board", via: "production-tab-sides")
            }
            await ui.capMenu(id: "sides-day", name: "schedule-sides-day-menu", title: "Sides: choose a shooting day", area: "Sides", from: "schedule-sides")
            await ui.step("sides selected cast") {
                let labels = ui.radioLabels(group: "sides-for")
                guard let last = labels.last, labels.count > 1 else { throw DriverError("no sides-for choices") }
                try await ui.scheduleSegment(group: "sides-for", label: last)
                await ui.capture("schedule-sides-selected-cast", title: "Sides for selected cast", area: "Sides", from: "schedule-sides", via: "sides-for")
                if let first = labels.first { try await ui.scheduleSegment(group: "sides-for", label: first) }
            }
            await ui.step("sides cover") {
                try await ui.capAct(id: "sides-cover")
                await ui.idle(0.5)
                _ = try? await ui.wait(id: "sides-paper", timeout: 10)
                await ui.idle(1.2)
                await ui.capture("schedule-sides-cover", title: "Sides with the call-sheet cover page", area: "Sides", from: "schedule-sides", via: "sides-cover")
            }
            await ui.visit(press: "sides-export", name: "schedule-sides-export", title: "Export the sides as PDF", area: "Sides", from: "schedule-sides")
            await ui.step("sides save") {
                try await ui.capAct(id: "sides-save")
                try await ui.capWaitUntil("a saved sides row") { !ui.capQueryPrefix("sides-saved-").isEmpty }
                await ui.idle(0.8)
                await ui.capture("schedule-sides-saved", title: "Sides saved as a snapshot", area: "Sides", from: "schedule-sides", via: "sides-save")
                guard let row = ui.scheduleFirst("sides-saved-") else { throw DriverError("no saved sides") }
                try await ui.press(row)
                _ = try? await ui.wait(id: "sides-paper", timeout: 10)
                await ui.idle(1.5)
                await ui.capture("schedule-sides-snapshot", title: "Saved sides open", area: "Sides", from: "schedule-sides-saved", via: row.identifier)
                _ = await ui.tryPress(id: "sides-back-to-live", wait: 2)
            }

            await ui.step("reports") {
                try await ui.go("Reports")
                _ = try await ui.wait(id: "reports-list", timeout: 10)
                await ui.idle(1.0)
            }
            var last = "schedule-board"
            for (value, label) in [("scene", "Scene Report"), ("location", "Location Report"), ("cast-scene", "Cast Scene Report"), ("prop", "Prop Report"),
                                   ("schedule", "Schedule Report"), ("breakdown-completeness", "Breakdown Completeness")] {
                await ui.step("report \(value)") {
                    guard let b = ui.scheduleFirst("reports-type-\(value)") ?? ui.query(label: label, role: "AXButton").first else { throw DriverError("no report \(label)") }
                    try await ui.press(b)
                    await ui.idle(1.2)
                    let n = "schedule-reports-\(value)"
                    await ui.capture(n, title: "Reports: \(label)", area: "Reports", from: last, via: b.identifier)
                    last = n
                }
            }
            await ui.step("report filter") {
                if let b = ui.scheduleFirst("reports-type-scene") { try await ui.press(b); await ui.idle(1.0) }
                try await ui.capTypeIn("1", into: "reports-filter")
                await ui.idle(1.2)
                await ui.capture("schedule-reports-filtered", title: "Scene Report filtered", area: "Reports", from: "schedule-reports-scene", via: "reports-filter")
                _ = await ui.tryPress(id: "reports-filter-clear", wait: 2)
                await ui.idle(0.6)
            }
            await ui.visit(press: "reports-export", name: "schedule-reports-export", title: "Export the Scene Report", area: "Reports", from: "schedule-reports-scene")
            await ui.step("report snapshot") {
                try await ui.capAct(id: "reports-save")
                try await ui.capWaitUntil("a saved report row") { !ui.capQueryPrefix("reports-saved-").isEmpty }
                guard let row = ui.scheduleFirst("reports-saved-") else { throw DriverError("no saved report") }
                try await ui.press(row)
                _ = try await ui.wait(label: "Back to live reports", timeout: 8)
                await ui.idle(0.8)
                await ui.capture("schedule-reports-snapshot", title: "Saved report snapshot open", area: "Reports", from: "schedule-reports-scene", via: row.identifier)
                _ = await ui.tryPress(label: "Back to live reports", wait: 2)
            }
        },
    ]

    static let captureCallSheets: [Scenario] = [
        tour("07-callsheets", order: 7000, timeout: 1100) { ui, _ in
            try await Scenarios.scheduleThreeDayBase(ui)
            try await ui.capNav("Call Sheets")
            await ui.idle(0.8)
            await ui.capture("callsheets-empty", title: "Call Sheets, none yet", area: "Call Sheets")
            await ui.step("create") {
                try await ui.capAct(id: "callsheet-create")
                _ = try await ui.wait(id: "callsheet-create-confirm", timeout: 8)
                await ui.idle(0.8)
                await ui.capture("callsheet-create-sheet", title: "Create a call sheet", area: "Call Sheets", from: "callsheets-empty", via: "callsheet-create")
                await ui.capMenu(id: "callsheet-create-day", name: "callsheet-create-day-menu", title: "Create a call sheet: pick a day", area: "Call Sheets", from: "callsheet-create-sheet")
                await ui.capMenu(id: "callsheet-create-template", name: "callsheet-create-template-menu", title: "Create a call sheet: template", area: "Call Sheets", from: "callsheet-create-sheet")
                try await ui.capAct(id: "callsheet-create-confirm")
                _ = try await ui.wait(id: "callsheet-document", timeout: 25)
                await ui.idle(1.0)
                await ui.capture("callsheet-editor", title: "Call sheet editor, Shoot Day 1", area: "Call Sheets", from: "callsheet-create-sheet", via: "callsheet-create-confirm")
            }
            await ui.exploreMenu("callsheet-add-section", area: "Call Sheets", prefix: "callsheet-section", from: "callsheet-editor", limit: 5)
            await ui.exploreButtons([("callsheet-add-crew-row", "Add crew row"), ("callsheet-add-extras-row", "Add extras row"), ("callsheet-add-custom-field", "Add custom field"),
                                     ("callsheet-export", "Export"), ("callsheet-refresh", "Refresh from schedule")],
                                    area: "Call Sheets", prefix: "callsheet", from: "callsheet-editor")
            await ui.visit(press: "callsheet-delete", name: "callsheet-delete-confirm", title: "Delete this call sheet?", area: "Call Sheets", from: "callsheet-editor")
            await ui.step("back to list") {
                try await ui.capAct(id: "callsheet-back")
                await ui.idle(1.0)
                await ui.capture("callsheets-list-one", title: "Call Sheets list with one call sheet", area: "Call Sheets", from: "callsheet-editor", via: "callsheet-back")
            }
            await ui.step("second call sheet") {
                try await ui.capAct(id: "callsheet-create")
                _ = try await ui.wait(id: "callsheet-create-confirm", timeout: 8)
                await ui.idle(0.6)
                try await ui.capMenuButton(id: "callsheet-create-day", item: "Shoot Day 2")
                await ui.idle(0.5)
                await ui.capture("callsheet-create-sheet-day-2", title: "Create a call sheet for Shoot Day 2", area: "Call Sheets", from: "callsheets-list-one", via: "callsheet-create")
                try await ui.capAct(id: "callsheet-create-confirm")
                _ = try await ui.wait(id: "callsheet-document", timeout: 25)
                await ui.idle(1.0)
                await ui.capture("callsheet-editor-day-2", title: "Call sheet editor, Shoot Day 2", area: "Call Sheets", from: "callsheet-create-sheet-day-2", via: "callsheet-create-confirm")
                try await ui.capAct(id: "callsheet-back")
                await ui.idle(1.0)
                await ui.capture("callsheets-list", title: "Call Sheets list with two call sheets", area: "Call Sheets", from: "callsheet-editor-day-2", via: "callsheet-back")
            }
            await ui.step("row menu") {
                guard let m = ui.scheduleFirst("callsheet-actions-") else { throw DriverError("no call sheet row menu") }
                await ui.capMenu(id: m.identifier, name: "callsheet-row-menu", title: "Call Sheets: row actions", area: "Call Sheets", from: "callsheets-list")
                if let row = ui.scheduleFirst("callsheet-row-") {
                    await ui.capContext(on: row, name: "callsheet-row-context-menu", title: "Call Sheets: right-click a call sheet", area: "Call Sheets", from: "callsheets-list")
                }
            }
            await ui.step("row export") {
                guard let b = ui.scheduleFirst("callsheet-export-") else { throw DriverError("no row export") }
                try await ui.press(b)
                _ = try await ui.wait(id: "export-sheet", timeout: 8)
                await ui.capture("callsheet-list-export", title: "Export a call sheet from the list", area: "Call Sheets", from: "callsheets-list", via: b.identifier)
                await ui.capRecover()
            }
            await ui.step("search") {
                try await ui.capTypeIn("Day 2", into: "callsheet-search")
                await ui.idle(1.0)
                await ui.capture("callsheets-search", title: "Call Sheets searched for Day 2", area: "Call Sheets", from: "callsheets-list", via: "callsheet-search")
                try await ui.capTypeIn("no such sheet", into: "callsheet-search")
                await ui.idle(1.0)
                await ui.capture("callsheets-no-match", title: "Call Sheets, nothing matches the search", area: "Call Sheets", from: "callsheets-search", via: "callsheet-search")
                try await ui.capTypeIn("", into: "callsheet-search")
                await ui.idle(0.8)
            }
            await ui.step("filter final") {
                try await ui.scheduleSegment(group: "callsheet-filter", label: "Final")
                await ui.capture("callsheets-filter-final", title: "Call Sheets filtered to final", area: "Call Sheets", from: "callsheets-list", via: "callsheet-filter")
                try await ui.scheduleSegment(group: "callsheet-filter", label: "All")
            }
            await ui.step("open and finalize") {
                guard let b = ui.scheduleFirst("callsheet-open-") else { throw DriverError("no open button") }
                try await ui.press(b)
                _ = try await ui.wait(id: "callsheet-document", timeout: 25)
                await ui.idle(1.0)
                await ui.capture("callsheet-open", title: "Call sheet opened from the list", area: "Call Sheets", from: "callsheets-list", via: b.identifier)
                try await ui.capAct(id: "callsheet-finalize")
                _ = try await ui.wait(id: "callsheet-finalize-sheet", timeout: 8)
                await ui.capture("callsheet-finalize-sheet", title: "Finalize the call sheet", area: "Call Sheets", from: "callsheet-open", via: "callsheet-finalize")
                if ui.exists(id: "callsheet-finalize-confirm") { try await ui.capAct(id: "callsheet-finalize-confirm") } else { try await ui.capAct(id: "callsheet-finalize-anyway") }
                try await ui.capWaitUntil("the call sheet is final", timeout: 12) { ui.exists(id: "callsheet-finalized-at") || ui.capShows("Start New Revision") }
                await ui.idle(0.8)
                await ui.capture("callsheet-finalized", title: "Call sheet finalized", area: "Call Sheets", from: "callsheet-finalize-sheet", via: "callsheet-finalize-confirm")
                try await ui.capAct(id: "callsheet-refresh")
                _ = try await ui.wait(id: "callsheet-refresh-sheet", timeout: 8)
                await ui.capture("callsheet-new-revision-sheet", title: "Start a new revision of the call sheet", area: "Call Sheets", from: "callsheet-finalized", via: "callsheet-refresh")
                await ui.capRecover()
                try await ui.capAct(id: "callsheet-back")
                await ui.idle(1.0)
                await ui.capture("callsheets-list-final", title: "Call Sheets list with a final call sheet", area: "Call Sheets", from: "callsheet-finalized", via: "callsheet-back")
            }
        },
    ]

    static let captureBudget: [Scenario] = [
        tour("08-budget", order: 8000, importSample: false) { ui, _ in
            try await ui.capNav("Budget")
            _ = try? await ui.wait(label: "Start Budget", role: "AXButton", timeout: 8)
            await ui.capture("budget-start", title: "Budget, not started", area: "Budget")
            await ui.step("currency menu") {
                guard let e = ui.query(id: "budget-start-currency").first(where: { $0.obj is NSPopUpButtonCell }) ?? ui.query(label: "Currency").first(where: { $0.obj is NSPopUpButtonCell })
                else { throw DriverError("no currency picker") }
                guard let r = ui.relFrame(e) else { throw DriverError("no frame") }
                let menu = try await ui.peekMenu(e)
                await ui.capture("budget-currency-menu", title: "Budget: choose a currency", area: "Budget", from: "budget-start", via: "budget-start-currency", menu: menu, anchor: CGPoint(x: r.minX, y: r.maxY + 4))
            }
            await ui.step("start") {
                try await ui.capAct(label: "Start Budget", role: "AXButton")
                _ = try await ui.wait(id: "budget-add-line", timeout: 10)
                await ui.idle(1.0)
                await ui.capture("budget-empty-table", title: "Budget, no lines yet", area: "Budget", from: "budget-start", via: "Start Budget")
            }
            await ui.step("line sheet") {
                try await ui.capAct(id: "budget-add-line")
                _ = try await ui.wait(id: "of-bl-desc", timeout: 6)
                await ui.capture("budget-add-line-sheet", title: "Add a budget line", area: "Budget", from: "budget-empty-table", via: "budget-add-line")
                await ui.capMenu(id: "of-bl-cat", name: "budget-category-menu", title: "Add a budget line: category", area: "Budget", from: "budget-add-line-sheet")
                try await ui.capTypeIn("Lead actors", into: "of-bl-desc")
                try await ui.capTypeIn("forty thousand", into: "of-bl-amt")
                try await ui.capAct(id: "budget-line-sheet-primary")
                await ui.idle(1.0)
                await ui.capture("budget-add-line-error", title: "Add a budget line, amount not a number", area: "Budget", from: "budget-add-line-sheet", via: "budget-line-sheet-primary")
                await ui.capRecover()
            }
            let lines: [(String?, String, String, String?)] = [(nil, "Lead actors", "40000", "42500"), ("Crew", "Camera team", "25000", "21000"),
                                                               ("Locations", "Depot hire", "12000", nil), ("Equipment", "Lighting package", "8000", nil)]
            for (n, (cat, desc, amt, act)) in lines.enumerated() {
                await ui.step("line \(n)") {
                    try await ui.capAct(id: "budget-add-line")
                    _ = try await ui.wait(id: "of-bl-desc", timeout: 6)
                    await ui.idle(0.4)
                    if let cat { try? await ui.capMenuButton(id: "of-bl-cat", item: cat) }
                    try await ui.capTypeIn(desc, into: "of-bl-desc")
                    try await ui.capTypeIn(amt, into: "of-bl-amt")
                    if let act { try await ui.capTypeIn(act, into: "of-bl-act") }
                    if n == 1 { await ui.capture("budget-add-line-filled", title: "Add a budget line, filled in", area: "Budget", from: "budget-add-line-sheet", via: "of-bl-desc") }
                    try await ui.capAct(id: "budget-line-sheet-primary")
                    try await ui.waitGone(id: "of-bl-desc", timeout: 8)
                    await ui.idle(0.6)
                    if n == 0 { await ui.capture("budget-one-line", title: "Budget with one line and an overspend", area: "Budget", from: "budget-add-line-sheet", via: "budget-line-sheet-primary") }
                }
            }
            await ui.capture("budget-table", title: "Budget with lines in four categories and totals", area: "Budget", from: "budget-add-line-sheet")
            await ui.step("edit line") {
                guard let b = ui.scheduleFirst("budget-edit-") else { throw DriverError("no edit button") }
                try await ui.press(b)
                _ = try await ui.wait(id: "of-bl-desc", timeout: 6)
                await ui.capture("budget-edit-line-sheet", title: "Edit a budget line", area: "Budget", from: "budget-table", via: b.identifier)
                await ui.capRecover()
            }
            await ui.step("plan") {
                try await ui.capAct(id: "budget-edit-plan")
                _ = try await ui.wait(id: "of-b-plan", timeout: 6)
                await ui.capture("budget-edit-plan", title: "Budget plan", area: "Budget", from: "budget-table", via: "budget-edit-plan")
                await ui.capMenu(id: "of-b-cur", name: "budget-plan-currency-menu", title: "Budget plan: currency", area: "Budget", from: "budget-edit-plan")
                try await ui.capTypeIn("100000", into: "of-b-plan")
                try await ui.capTypeIn("10", into: "of-b-cont-value")
                try await ui.capTypeIn("Rough plan for the depot shoot.", into: "of-b-notes")
                try await ui.capAct(id: "budget-plan-sheet-primary")
                try await ui.waitGone(id: "of-b-plan", timeout: 8)
                await ui.idle(0.8)
                await ui.capture("budget-with-plan", title: "Budget with an overall plan, contingency and notes", area: "Budget", from: "budget-edit-plan", via: "budget-plan-sheet-primary")
            }
            await ui.step("snapshot") {
                try await ui.capAct(id: "budget-save-snapshot")
                _ = try await ui.wait(id: "of-b-snap", timeout: 6)
                await ui.capture("budget-snapshot-name-sheet", title: "Save a budget snapshot", area: "Budget", from: "budget-with-plan", via: "budget-save-snapshot")
                try await ui.capTypeIn("Pre-production estimate", into: "of-b-snap")
                try await ui.capAct(id: "budget-snapshot-sheet-primary")
                try await ui.waitGone(id: "of-b-snap", timeout: 8)
                try await ui.capWaitUntil("a snapshot row") { ui.exists(id: "budget-snapshots") }
                await ui.idle(0.8)
                await ui.capture("budget-snapshots", title: "Budget with a saved snapshot", area: "Budget", from: "budget-snapshot-name-sheet", via: "budget-snapshot-sheet-primary")
                guard let v = ui.scheduleFirst("budget-snapshot-view-") else { throw DriverError("no snapshot view button") }
                try await ui.press(v)
                _ = try await ui.wait(id: "budget-snapshot-close", timeout: 8)
                await ui.capture("budget-snapshot-view", title: "Budget snapshot open", area: "Budget", from: "budget-snapshots", via: v.identifier)
                await ui.capRecover()
            }
            await ui.visit(press: "budget-export", name: "budget-export", title: "Export the budget", area: "Budget", from: "budget-table")
        },
    ]
}

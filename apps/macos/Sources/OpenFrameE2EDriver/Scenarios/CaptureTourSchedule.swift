// Capture tour section: Production schedule, Call Sheets and Budget. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureSchedule: [Scenario] = [
        tour("06-schedule", order: 6000, timeout: 900) { ui, _ in
            try await Scenarios.capBreakdownBase(ui)
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
            await ui.step("days") {
                try await ui.capAct(id: "create-shooting-day")
                _ = try await ui.wait(id: "day-date", timeout: 6)
                await ui.capture("schedule-add-day-sheet", title: "Add a shooting day", area: "Schedule", from: "schedule-board-unscheduled", via: "create-shooting-day")
                for iso in ["2027-06-14", "2027-06-15"] {
                    if !ui.exists(id: "day-date") { try await ui.capAct(id: "create-shooting-day") }
                    try await ui.capTypeIn(iso, into: "day-date")
                    try await ui.capAct(id: "day-sheet-confirm")
                    try await ui.waitGone(id: "day-date")
                }
                try await ui.capAct(id: "add-off-day")
                await ui.capture("schedule-add-off-day-sheet", title: "Add a day off", area: "Schedule", from: "schedule-board-unscheduled", via: "add-off-day")
                try await ui.capAct(id: "day-sheet-confirm")
                await ui.idle(0.6)
            }
            await ui.step("scenes into day") {
                await ui.capMenu(id: "strip-actions-1", name: "schedule-strip-menu", title: "Schedule: scene strip menu", area: "Schedule", from: "schedule-board-unscheduled")
                try await ui.capMenuButton(id: "strip-actions-1", item: "Shoot Day 1")
                await ui.idle(0.5)
                try await ui.capMenuButton(id: "strip-actions-2", item: "Shoot Day 1")
                await ui.idle(0.8)
                await ui.capture("schedule-board", title: "Schedule board with scenes in Day 1", area: "Schedule", from: "schedule-board-unscheduled")
            }
            await ui.step("meal break") {
                await ui.capMenu(id: "day-actions-1", name: "schedule-day-menu", title: "Schedule: day column menu", area: "Schedule", from: "schedule-board")
                try await ui.capMenuButton(id: "day-actions-1", item: "Meal Break")
                _ = try await ui.wait(id: "marker-duration", timeout: 6)
                await ui.capture("schedule-meal-break-sheet", title: "Add a meal break", area: "Schedule", from: "schedule-day-menu", via: "day-actions-1")
                try await ui.capTypeIn("45m", into: "marker-duration")
                try await ui.capAct(id: "marker-save")
                await ui.idle(0.8)
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
                await ui.exploreButtons([("day-add-meal", "Add meal"), ("day-add-travel", "Add travel"), ("day-add-company-move", "Add company move"), ("day-add-custom", "Add custom"), ("day-export", "Export day")], area: "Schedule", prefix: "schedule-day", from: "schedule-day-detail")
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
            await ui.exploreButtons([("schedule-filters", "Filters"), ("suggest-grouping", "Suggest grouping"), ("warnings-button", "Warnings"), ("schedule-settings", "Schedule settings"),
                                     ("schedule-export", "Export schedule"), ("schedule-pool", "Unscheduled pool")], area: "Schedule", prefix: "schedule", from: "schedule-board")
            await ui.step("dood") {
                try await ui.go("Day Out of Days")
                _ = try await ui.wait(id: "dood-table", timeout: 10)
                await ui.capture("schedule-dood", title: "Day Out of Days", area: "Schedule", from: "schedule-board", via: "schedule-section-dood")
                await ui.exploreButtons([("dood-export", "Export Day Out of Days"), ("dood-note", "Day Out of Days note")], area: "Schedule", prefix: "schedule-dood", from: "schedule-dood")
                try await ui.go("Schedule")
            }
        },
    ]

    static let captureCallSheets: [Scenario] = [
        tour("07-callsheets", order: 7000, timeout: 1100) { ui, _ in
            try await Scenarios.capScheduleBase(ui)
            try await ui.capNav("Call Sheets")
            await ui.capture("callsheets-empty", title: "Call Sheets, none yet", area: "Call Sheets")
            await ui.step("create") {
                try await ui.capAct(id: "callsheet-create")
                await ui.idle(0.8)
                await ui.capture("callsheet-create-sheet", title: "Create a call sheet", area: "Call Sheets", from: "callsheets-empty", via: "callsheet-create")
                await ui.capMenu(id: "callsheet-create-day", name: "callsheet-create-day-menu", title: "Create a call sheet: pick a day", area: "Call Sheets", from: "callsheet-create-sheet")
                await ui.capMenu(id: "callsheet-create-template", name: "callsheet-create-template-menu", title: "Create a call sheet: template", area: "Call Sheets", from: "callsheet-create-sheet")
                try await ui.capAct(id: "callsheet-create-confirm")
                _ = try await ui.wait(id: "callsheet-document", timeout: 25)
                await ui.idle(1.0)
                await ui.capture("callsheet-editor", title: "Call sheet editor", area: "Call Sheets", from: "callsheet-create-sheet", via: "callsheet-create-confirm")
            }
            await ui.exploreMenu("callsheet-add-section", area: "Call Sheets", prefix: "callsheet-section", from: "callsheet-editor", limit: 4)
            await ui.exploreButtons([("callsheet-add-crew-row", "Add crew row"), ("callsheet-add-extras-row", "Add extras row"), ("callsheet-add-custom-field", "Add custom field"), ("callsheet-finalize", "Finalize"), ("callsheet-export", "Export"), ("callsheet-refresh", "Refresh from schedule")],
                                    area: "Call Sheets", prefix: "callsheet", from: "callsheet-editor")
            await ui.step("back to list") {
                try await ui.capAct(id: "callsheet-back")
                await ui.idle(0.8)
                await ui.capture("callsheets-list", title: "Call Sheets list", area: "Call Sheets", from: "callsheet-editor", via: "callsheet-back")
            }
            await ui.eachTab(group: "callsheets-tabs", area: "Call Sheets", prefix: "callsheets-tab", from: "callsheets-list")
        },
    ]

    static let captureBudget: [Scenario] = [
        tour("08-budget", order: 8000, importSample: false) { ui, _ in
            try await ui.capNav("Budget")
            await ui.capture("budget-start", title: "Budget, not started", area: "Budget")
            await ui.capMenu(id: "budget-start-currency", name: "budget-currency-menu", title: "Budget: choose a currency", area: "Budget", from: "budget-start")
            await ui.step("start") {
                try await ui.capAct(label: "Start Budget", role: "AXButton")
                await ui.idle(1.2)
                await ui.capture("budget-empty-table", title: "Budget, no lines yet", area: "Budget", from: "budget-start", via: "Start Budget")
            }
            for (n, (cat, desc, amt)) in [("Cast", "Lead actors", "40000"), ("Crew", "Camera team", "25000"), ("Locations", "Depot hire", "12000")].enumerated() {
                await ui.step("line \(n)") {
                    try await ui.capAct(id: "budget-add-line")
                    _ = try await ui.wait(id: "of-bl-desc", timeout: 6)
                    if n == 0 { await ui.capture("budget-add-line-sheet", title: "Add a budget line", area: "Budget", from: "budget-empty-table", via: "budget-add-line") }
                    try await ui.capTypeIn(desc, into: "of-bl-desc")
                    try await ui.capTypeIn(amt, into: "of-bl-amt")
                    if n == 0 { await ui.capture("budget-add-line-filled", title: "Add a budget line, filled in", area: "Budget", from: "budget-add-line-sheet", via: "of-bl-desc") }
                    try await ui.capAct(id: "budget-line-sheet-primary")
                    await ui.idle(0.8)
                    _ = cat
                }
            }
            await ui.capture("budget-table", title: "Budget with lines", area: "Budget", from: "budget-add-line-sheet")
            await ui.exploreButtons([("budget-edit-plan", "Edit plan"), ("budget-save-snapshot", "Save a snapshot"), ("budget-snapshots", "Snapshots"), ("budget-notes", "Notes"), ("budget-mark-reviewed", "Mark reviewed"), ("budget-export", "Export budget")],
                                    area: "Budget", prefix: "budget", from: "budget-table")
        },
    ]
}

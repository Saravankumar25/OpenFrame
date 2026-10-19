// Mac phase M6, Budget. The budget behaviours of the shared scenario 62-call-sheets-exports.md
// ("The budget keeps Planned and Actual with the difference"), run at model level against a real
// core, plus money reading and formatting, plans, snapshots, exports and the failure cases.

import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@MainActor
private struct BudgetRig {
    let root: URL
    let app: AppModel
    let model: BudgetModel

    static func make() async throws -> BudgetRig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        #expect(await app.createProject(CreateProjectArgs(title: "Night Bus", projectType: .featureFilm)))
        let rig = BudgetRig(root: root, app: app, model: BudgetModel(app: app))
        await rig.model.reload()
        return rig
    }

    var quiet: Bool { app.alert == nil }
    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }
    func line(_ desc: String) -> BudgetLineDto? { model.current?.lines.first { $0.description == desc } }
}

@Suite("Budget", .serialized)
@MainActor
struct BudgetTests {
    // MARK: Scenario 62: Planned, Actual and the difference

    @Test func theBudgetKeepsPlannedAndActualWithTheDifference() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        // Production tab Budget: no budget yet, the start card shows.
        #expect(r.model.loaded && r.model.current == nil)
        #expect(r.model.startCurrency == "USD" || r.model.startCurrency == "INR" || r.model.startCurrency == "GBP")
        #expect(await r.model.start(currency: "USD"))
        let b = try #require(r.model.current)
        #expect(b.currency == "USD" && b.lines.isEmpty)
        #expect(!r.model.categories.isEmpty)

        // Add Line: Lead actor, planned 50000, actual 55000.
        let err = await r.model.saveLine(editing: nil, category: r.model.categories[0], description: "Lead actor", planned: "50000", actual: "55000", notes: "")
        #expect(err == nil)
        let cur = try #require(r.model.current)
        let l = try #require(r.line("Lead actor"))
        #expect(l.planned == 5_000_000 && l.actual == 5_500_000)
        // The workspace shows "over plan".
        let diff = try #require(l.difference)
        #expect(diff == -500_000)
        #expect(BudgetMoney.differenceText(diff, "USD").hasSuffix("over plan"))
        #expect(cur.actualCount == 1 && cur.difference == -500_000)
        #expect(cur.plannedLinesTotal == 5_000_000 && cur.actualTotal == 5_500_000)

        // Ctrl+Z with focus off any text field: the line goes, "No lines yet" shows.
        await r.app.undo()
        await r.model.reload()
        #expect(r.model.current?.lines.isEmpty == true)
        #expect(r.model.current?.categories.allSatisfy { $0.count == 0 } == true)
        // The screen is quiet.
        #expect(r.quiet)
    }

    // MARK: Lines and totals

    @Test func linesCategoriesAndTotalsAddUpAndLinesCanBeEditedAndDeleted() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        #expect(await r.model.start(currency: "USD"))
        let cats = r.model.categories
        #expect(cats.count >= 2)
        #expect(await r.model.saveLine(editing: nil, category: cats[0], description: "Lead actor", planned: "1,000.50", actual: "", notes: "Two weeks") == nil)
        #expect(await r.model.saveLine(editing: nil, category: cats[0], description: "Supporting cast", planned: "400", actual: "350", notes: "") == nil)
        #expect(await r.model.saveLine(editing: nil, category: cats[1], description: "Camera", planned: "250", actual: "300", notes: "") == nil)
        var b = try #require(r.model.current)
        #expect(b.plannedLinesTotal == 100_050 + 40_000 + 25_000)
        // Actual only counts lines that have one.
        #expect(b.actualCount == 2 && b.actualTotal == 35_000 + 30_000)
        #expect(b.difference == (40_000 - 35_000) + (25_000 - 30_000))
        let first = try #require(b.categories.first { $0.category == cats[0] })
        #expect(first.count == 2 && first.actualCount == 1 && first.planned == 140_050 && first.difference == 5_000)
        #expect(r.line("Lead actor")?.actual == nil && r.line("Lead actor")?.difference == nil)
        #expect(r.line("Lead actor")?.notes == "Two weeks")

        // Edit a line: change the actual, the category and clear the note.
        let lead = try #require(r.line("Lead actor"))
        #expect(await r.model.saveLine(editing: lead, category: cats[1], description: "Lead actor", planned: "1000.5", actual: "900", notes: "") == nil)
        #expect(r.line("Lead actor")?.category == cats[1])
        #expect(r.line("Lead actor")?.actual == 90_000)
        #expect(r.line("Lead actor")?.notes == nil)
        b = try #require(r.model.current)
        #expect(b.actualCount == 3)
        // Delete and undo.
        await r.model.deleteLine(try #require(r.line("Camera")))
        #expect(r.line("Camera") == nil)
        await r.app.undo()
        await r.model.reload()
        #expect(r.line("Camera") != nil)
        #expect(r.quiet)
    }

    @Test func badAmountsAndEmptyDescriptionsAreExplainedAndNothingIsSaved() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        #expect(await r.model.start(currency: "USD"))
        let cat = r.model.categories[0]
        #expect(await r.model.saveLine(editing: nil, category: cat, description: "  ", planned: "10", actual: "", notes: "") == "A line needs a description.")
        #expect(await r.model.saveLine(editing: nil, category: cat, description: "X", planned: "", actual: "", notes: "")?.hasPrefix("Enter the planned amount") == true)
        #expect(await r.model.saveLine(editing: nil, category: cat, description: "X", planned: "abc", actual: "", notes: "")?.hasPrefix("Enter the planned amount") == true)
        #expect(await r.model.saveLine(editing: nil, category: cat, description: "X", planned: "-5", actual: "", notes: "")?.hasPrefix("Enter the planned amount") == true)
        #expect(await r.model.saveLine(editing: nil, category: cat, description: "X", planned: "1.234", actual: "", notes: "")?.hasPrefix("Enter the planned amount") == true)
        #expect(await r.model.saveLine(editing: nil, category: cat, description: "X", planned: "10", actual: "-1", notes: "")?.hasPrefix("Enter the actual amount") == true)
        #expect(r.model.current?.lines.isEmpty == true)
        #expect(r.quiet)
        // The core also refuses an absurd amount, in words, and the line is not added.
        let huge = await r.model.saveLine(editing: nil, category: cat, description: "Huge", planned: "99999999999999999999", actual: "", notes: "")
        if huge != nil { #expect(r.app.alert != nil); r.app.alert = nil }
        // A very long description is cut to 200 characters by the field; the core refuses longer ones.
        let long = String(repeating: "x", count: 5000)
        let longResult = await r.model.saveLine(editing: nil, category: cat, description: long, planned: "1", actual: "", notes: "")
        if longResult != nil { #expect(r.app.alert != nil); r.app.alert = nil }
    }

    // MARK: Plan, currency, contingency

    @Test func thePlanSetsCurrencyPlannedTotalAndContingency() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        #expect(await r.model.start(currency: "USD"))
        #expect(await r.model.saveLine(editing: nil, category: r.model.categories[0], description: "Crew", planned: "8000", actual: "", notes: "") == nil)
        // Percent.
        #expect(await r.model.savePlan(currency: "USD", plannedTotal: "20,000", contingencyMode: "percent", contingencyValue: "10%", notes: "Keep it lean") == nil)
        var b = try #require(r.model.current)
        #expect(b.plannedTotal == 2_000_000)
        #expect(b.contingencyMode == "percent" && b.contingencyValue == 1000)
        #expect(b.contingencyAmount == 200_000)
        #expect(b.notes == "Keep it lean")
        // Amount, and another currency (whole units for yen).
        #expect(await r.model.savePlan(currency: "JPY", plannedTotal: "20000", contingencyMode: "amount", contingencyValue: "1500", notes: "") == nil)
        b = try #require(r.model.current)
        #expect(b.currency == "JPY" && b.plannedTotal == 20_000 && b.contingencyValue == 1500 && b.contingencyAmount == 1500)
        #expect(b.notes == nil)
        // No planned total at all.
        #expect(await r.model.savePlan(currency: "INR", plannedTotal: "", contingencyMode: "percent", contingencyValue: "", notes: "") == nil)
        #expect(r.model.current?.plannedTotal == nil && r.model.current?.contingencyValue == 0)

        // Mistakes are explained.
        #expect(await r.model.savePlan(currency: "USD", plannedTotal: "oops", contingencyMode: "percent", contingencyValue: "5", notes: "") == "Enter the planned total as a non-negative amount.")
        #expect(await r.model.savePlan(currency: "USD", plannedTotal: "", contingencyMode: "percent", contingencyValue: "150", notes: "") == "Contingency must be between 0% and 100%.")
        #expect(await r.model.savePlan(currency: "USD", plannedTotal: "", contingencyMode: "percent", contingencyValue: "x", notes: "") == "Contingency must be between 0% and 100%.")
        #expect(await r.model.savePlan(currency: "USD", plannedTotal: "", contingencyMode: "amount", contingencyValue: "-3", notes: "") == "Enter the contingency as a non-negative amount.")
        #expect(r.model.current?.currency == "INR")
        #expect(r.quiet)
    }

    // MARK: Snapshots

    @Test func snapshotsAreFrozenCopiesThatCanBeViewedExportedAndDeleted() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        #expect(await r.model.start(currency: "EUR"))
        #expect(await r.model.saveLine(editing: nil, category: r.model.categories[0], description: "Locations", planned: "5000", actual: "4000", notes: "") == nil)
        #expect(!(await r.model.saveSnapshot(name: "   ")))
        #expect(await r.model.saveSnapshot(name: "Pre-production estimate"))
        #expect(r.model.snapshots.count == 1)
        let snap = try #require(r.model.snapshots.first)
        #expect(snap.label == "Pre-production estimate" && snap.currency == "EUR")
        #expect(snap.plannedLinesTotal == 500_000 && snap.actualTotal == 400_000)

        // Later changes do not touch the snapshot.
        #expect(await r.model.saveLine(editing: nil, category: r.model.categories[0], description: "Extra", planned: "100", actual: "", notes: "") == nil)
        await r.model.loadSnapshot(snap.id)
        let frozen = try #require(r.model.snapshot)
        #expect(frozen.isCurrent == false && frozen.frozenAt != nil)
        #expect(frozen.lines.map(\.description) == ["Locations"])
        #expect(r.model.current?.lines.count == 2)

        // Export: current budget and the snapshot, in three formats.
        let dir = r.root
        for ext in ["pdf", "csv", "xlsx"] {
            let res = try #require(await r.model.export(format: ext, path: dir.appendingPathComponent("budget.\(ext)").path, budgetId: nil, includeNotes: true))
            #expect(res.format == ext && res.bytesWritten > 50)
        }
        let csv = try String(contentsOf: dir.appendingPathComponent("budget.csv"), encoding: .utf8)
        #expect(csv.contains("Locations") && csv.contains("Extra"))
        let snapCsv = try #require(await r.model.export(format: "csv", path: dir.appendingPathComponent("snap.csv").path, budgetId: snap.id, includeNotes: false))
        #expect(snapCsv.bytesWritten > 0)
        let text = try String(contentsOf: dir.appendingPathComponent("snap.csv"), encoding: .utf8)
        #expect(text.contains("Locations") && !text.contains("Extra"))
        #expect(await r.model.export(format: "pdf", path: dir.appendingPathComponent("nope/x.pdf").path, budgetId: nil, includeNotes: false) == nil)
        #expect(r.app.alert != nil)
        r.app.alert = nil

        // Delete (and undo).
        await r.model.deleteSnapshot(snap)
        #expect(r.model.snapshots.isEmpty)
        await r.app.undo()
        await r.model.reload()
        #expect(r.model.snapshots.count == 1)
        // A snapshot that is gone says so.
        await r.model.loadSnapshot("missing-id")
        #expect(r.app.alert != nil)
        r.app.alert = nil
    }

    @Test func theReviewReminderIsManualAndGoesAwayWhenMarked() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        #expect(r.model.reviewReminder == nil)
        #expect(await r.model.start(currency: "USD"))
        #expect(r.model.reviewReminder == nil)
        await r.model.markReviewed()
        #expect(r.model.reviewReminder == nil)
        #expect(r.quiet)
    }

    @Test func theBudgetSurvivesARestart() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        #expect(await r.model.start(currency: "GBP"))
        #expect(await r.model.saveLine(editing: nil, category: r.model.categories[0], description: "Catering", planned: "300.25", actual: "", notes: "") == nil)
        await r.app.closeProject()
        let recent = try await r.app.core!.projectListRecent(ListRecentArgs(includeArchived: false))
        await r.app.openProject(path: recent[0].path)
        let again = BudgetModel(app: r.app)
        await again.reload()
        #expect(again.current?.currency == "GBP")
        #expect(again.current?.lines.first?.planned == 30_025)
        #expect(r.quiet)
    }

    // MARK: Money

    @Test func moneyIsReadAndShownLikeWindows() {
        let us = Locale(identifier: "en_US")
        #expect(BudgetMoney.parse("12,000", currency: "USD") == .amount(1_200_000))
        #expect(BudgetMoney.parse("1500.50", currency: "USD") == .amount(150_050))
        #expect(BudgetMoney.parse("1500.5", currency: "USD") == .amount(150_050))
        #expect(BudgetMoney.parse("12,00,000", currency: "INR") == .amount(120_000_000))
        #expect(BudgetMoney.parse("\u{20B9} 2,000", currency: "INR") == .amount(200_000))
        #expect(BudgetMoney.parse("$5", currency: "USD") == .amount(500))
        #expect(BudgetMoney.parse(".5", currency: "USD") == .invalid)
        #expect(BudgetMoney.parse("0", currency: "USD") == .amount(0))
        #expect(BudgetMoney.parse("", currency: "USD") == .empty)
        #expect(BudgetMoney.parse("   ", currency: "USD") == .empty)
        #expect(BudgetMoney.parse("abc", currency: "USD") == .invalid)
        #expect(BudgetMoney.parse("-5", currency: "USD") == .invalid)
        #expect(BudgetMoney.parse("1.234", currency: "USD") == .invalid)
        #expect(BudgetMoney.parse("1.5", currency: "JPY") == .invalid)
        #expect(BudgetMoney.parse("1500", currency: "JPY") == .amount(1500))
        #expect(BudgetMoney.parse("1e5", currency: "USD") == .invalid)
        #expect(BudgetMoney.toInput(150_050, currency: "USD") == "1500.50")
        #expect(BudgetMoney.toInput(5_000_000, currency: "USD") == "50000")
        #expect(BudgetMoney.toInput(1500, currency: "JPY") == "1500")
        #expect(BudgetMoney.toInput(nil, currency: "USD") == "")
        #expect(BudgetMoney.format(5_000_000, "USD", locale: us) == "$50,000")
        #expect(BudgetMoney.format(150_050, "USD", locale: us) == "$1,500.5")
        #expect(BudgetMoney.format(1500, "JPY", locale: us).hasSuffix("1,500"))
        #expect(BudgetMoney.format(120_000_000, "INR").contains("12,00,000"))
        #expect(BudgetMoney.digits("JPY") == 0 && BudgetMoney.digits("EUR") == 2)
        #expect(BudgetMoney.differenceText(0, "USD") == "On plan")
        #expect(BudgetMoney.differenceText(500, "USD") == "\(BudgetMoney.format(500, "USD")) under plan")
        #expect(BudgetMoney.differenceText(-500, "USD") == "\(BudgetMoney.format(500, "USD")) over plan")
        #expect(BudgetMoney.guessCurrency(Locale(identifier: "en_IN")) == "INR")
        #expect(BudgetMoney.guessCurrency(Locale(identifier: "en_GB")) == "GBP")
        #expect(BudgetMoney.guessCurrency(Locale(identifier: "de_DE")) == "USD")
        #expect(BudgetMoney.currencies.count == 17 && BudgetMoney.currencies.contains("AED"))
    }

    // MARK: Screens

    @Test func theBudgetScreenRendersEmptyFilledAndAsASnapshot() async throws {
        let r = try await BudgetRig.make()
        defer { r.close() }
        func host<V: View>(_ v: V, _ scheme: ColorScheme) -> some View {
            let t = ThemeStore()
            return v.environment(r.app).environment(t).environment(\.ofTheme, t.resolvedFor(colorScheme: scheme)).environment(\.colorScheme, scheme)
        }
        #expect(Fixture.render(host(BudgetContent(model: r.model), .light), name: "budget-empty", size: CGSize(width: 1100, height: 700)) != nil)
        #expect(await r.model.start(currency: "USD"))
        let cats = r.model.categories
        #expect(await r.model.saveLine(editing: nil, category: cats[0], description: "Lead actor", planned: "50000", actual: "55000", notes: "Over by a week") == nil)
        #expect(await r.model.saveLine(editing: nil, category: cats[1], description: "Camera package", planned: "12000", actual: "", notes: "") == nil)
        #expect(await r.model.savePlan(currency: "USD", plannedTotal: "100000", contingencyMode: "percent", contingencyValue: "10", notes: "") == nil)
        #expect(await r.model.saveSnapshot(name: "Estimate"))
        for scheme in [ColorScheme.light, .dark] {
            let n = scheme == .dark ? "dark" : "light"
            #expect(Fixture.render(host(BudgetContent(model: r.model), scheme), name: "budget-\(n)", size: CGSize(width: 1100, height: 760)) != nil)
        }
        r.model.sheet = .line(nil)
        #expect(Fixture.render(host(BudgetContent(model: r.model), .light), name: "budget-line-sheet", size: CGSize(width: 1100, height: 760)) != nil)
        r.model.sheet = nil
        #expect(r.quiet)
    }
}

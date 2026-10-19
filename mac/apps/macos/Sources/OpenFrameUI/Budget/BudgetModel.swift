// The Budget workspace's model and money helpers. Advisory only: categories, lines with a Planned
// and an Actual amount, category totals, the difference, a planned total and a contingency. Values
// change only when the person edits them, never automatically. Money is whole minor units (cents,
// paise); the core sums and compares, this file only reads typed text and formats amounts.
// Mirrors apps/desktop/src/workspaces/production/tabs/budget.tsx and api/schedule.ts (money).

import Foundation
import Observation
import OpenFrameCore

enum BudgetMoney {
    static let currencies = ["USD", "EUR", "GBP", "INR", "CAD", "AUD", "NZD", "JPY", "CHF", "SEK", "NOK", "DKK", "ZAR", "BRL", "MXN", "SGD", "AED"]
    static let zeroDigit: Set<String> = ["JPY", "KRW"]

    static func digits(_ currency: String) -> Int { zeroDigit.contains(currency) ? 0 : 2 }

    /// An amount typed by a person: nothing, a number in minor units, or something unreadable.
    enum Parsed: Equatable {
        case empty
        case amount(Double)
        case invalid
    }

    /// Format integer minor units (cents, paise) as money, for example "$50,000" or "\u{20B9}50,000".
    static func format(_ minor: Double, _ currency: String, locale: Locale? = nil) -> String {
        let d = digits(currency)
        let value = minor / pow(10, Double(d))
        let f = NumberFormatter()
        f.numberStyle = .currency
        f.currencyCode = currency
        f.locale = locale ?? (currency == "INR" ? Locale(identifier: "en_IN") : Locale.current)
        f.maximumFractionDigits = d
        f.minimumFractionDigits = 0
        f.currencyCode = currency
        return f.string(from: NSNumber(value: value)) ?? "\(currency) \(String(format: "%.\(d)f", value))"
    }

    /// Parse "12,00,000", "1500.50" or "\u{20B9} 2,000" into minor units. Empty is no amount;
    /// negative, malformed or too many decimals is invalid.
    static func parse(_ input: String, currency: String) -> Parsed {
        var s = input.filter { !$0.isWhitespace && $0 != "," }
        while let first = s.first, !(first.isNumber && first.isASCII) && first != "." && first != "-" { s.removeFirst() }
        if s.isEmpty { return input.trimmingCharacters(in: .whitespaces).isEmpty ? .empty : .invalid }
        guard s.range(of: #"^\d+(\.\d+)?$"#, options: .regularExpression) != nil else { return .invalid }
        let d = digits(currency)
        let parts = s.split(separator: ".", omittingEmptySubsequences: false)
        let whole = String(parts[0])
        let frac = parts.count > 1 ? String(parts[1]) : ""
        if frac.count > d { return .invalid }
        guard let w = Double(whole) else { return .invalid }
        let padded = (frac + String(repeating: "0", count: d)).prefix(d)
        let f = Double(String(padded)) ?? 0
        return .amount(w * pow(10, Double(d)) + f)
    }

    /// Minor units as the text of a field: 50000 cents is "500", 150050 is "1500.5".
    static func toInput(_ minor: Double?, currency: String) -> String {
        guard let minor else { return "" }
        let d = digits(currency)
        if d == 0 { return String(Int(minor)) }
        var s = String(format: "%.\(d)f", minor / pow(10, Double(d)))
        if s.hasSuffix(".00") { s.removeLast(3) }
        return s
    }

    /// "under plan" or "over plan" text for Planned minus Actual (never colour alone).
    static func differenceText(_ difference: Double, _ currency: String) -> String {
        if difference == 0 { return "On plan" }
        return difference > 0 ? "\(format(difference, currency)) under plan" : "\(format(-difference, currency)) over plan"
    }

    static func guessCurrency(_ locale: Locale = .current) -> String {
        switch locale.region?.identifier {
        case "IN": "INR"
        case "GB": "GBP"
        default: "USD"
        }
    }
}

enum BudgetSheet: Identifiable {
    case line(BudgetLineDto?)
    case plan
    case snapshotName
    case snapshot(String)
    case export(budgetId: String?)

    var id: String {
        switch self {
        case let .line(l): "line-\(l?.id ?? "new")"
        case .plan: "plan"
        case .snapshotName: "snapshot-name"
        case let .snapshot(id): "snapshot-\(id)"
        case let .export(id): "export-\(id ?? "current")"
        }
    }
}

@MainActor
@Observable
final class BudgetModel {
    let app: AppModel
    private(set) var view: BudgetView?
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    private(set) var snapshot: BudgetDto?
    private(set) var busy = false
    var sheet: BudgetSheet?
    var startCurrency = BudgetMoney.guessCurrency()
    @ObservationIgnored private var generation = 0

    init(app: AppModel) { self.app = app }

    var core: Core? { app.core }
    var current: BudgetDto? { view?.current }
    var categories: [String] { view?.categories ?? [] }
    var snapshots: [BudgetSnapshotRow] { view?.snapshots ?? [] }
    var reviewReminder: String? { current == nil ? nil : view?.reviewReminder }

    // MARK: Loading

    func reload() async {
        guard let core else { return }
        generation += 1
        let gen = generation
        do {
            let v = try await core.budgetGet()
            guard gen == generation else { return }
            view = v
            loadFailure = nil
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
        }
        loaded = true
    }

    func loadSnapshot(_ id: String) async {
        guard let core else { return }
        snapshot = nil
        do { snapshot = try await core.budgetGetSnapshot(BudgetIdArgs(id: id)) } catch {
            app.show("Could not open the snapshot", error)
            sheet = nil
        }
    }

    // MARK: Changes

    @discardableResult
    private func run<T>(_ title: String, done: String, _ body: (Core) async throws -> T) async -> T? {
        guard let core else { return nil }
        busy = true
        defer { busy = false }
        do {
            let r = try await body(core)
            ToastCenter.shared.undoable(done, app: app)
            return r
        } catch {
            app.show(title, error)
            return nil
        }
    }

    func start(currency: String) async -> Bool {
        let r = await run("Could not start the budget", done: "Started a budget snapshot") { try await $0.budgetCreate(BudgetCreateArgs(currency: currency)) }
        await reload()
        return r != nil
    }

    /// Add or change a line from the typed fields. Returns the message to show under the fields
    /// when something needs fixing, or nil when the line was saved.
    func saveLine(editing line: BudgetLineDto?, category: String, description: String, planned: String, actual: String, notes: String) async -> String? {
        guard let b = current else { return "There is no budget yet." }
        let planned = BudgetMoney.parse(planned, currency: b.currency)
        let actual = BudgetMoney.parse(actual, currency: b.currency)
        if description.trimmingCharacters(in: .whitespaces).isEmpty { return "A line needs a description." }
        guard case let .amount(plannedMinor) = planned else {
            return "Enter the planned amount as a non-negative number, for example 12,000 or 1500.50."
        }
        var actualMinor: Double?
        switch actual {
        case .empty: actualMinor = nil
        case let .amount(a): actualMinor = a
        case .invalid: return "Enter the actual amount as a non-negative number, or leave it empty."
        }
        let trimmedNotes = notes.trimmingCharacters(in: .whitespacesAndNewlines)
        let note = trimmedNotes.isEmpty ? nil : trimmedNotes
        let name = description.trimmingCharacters(in: .whitespaces)
        let ok: Bool
        if let line {
            ok = await run("Could not change the line", done: "Edited \u{201C}\(name)\u{201D}") {
                try await $0.budgetUpdateLine(BudgetLineUpdateArgs(
                    id: line.id, category: category, description: description, planned: plannedMinor, actual: actualMinor, notes: note, expectedRev: line.rev))
            } != nil
        } else {
            ok = await run("Could not add the line", done: "Added \u{201C}\(name)\u{201D}") {
                try await $0.budgetAddLine(BudgetLineAddArgs(
                    budgetId: b.id, category: category, description: description, planned: plannedMinor, actual: actualMinor, notes: note))
            } != nil
        }
        if ok {
            sheet = nil
            await reload()
            return nil
        }
        return ""
    }

    func deleteLine(_ line: BudgetLineDto) async {
        let r = await run("Could not delete the line", done: "Deleted \u{201C}\(line.description)\u{201D}") { try await $0.budgetDeleteLine(BudgetIdArgs(id: line.id)) }
        if r != nil { await reload() }
    }

    /// Change currency, planned total, contingency and notes. Returns a message when a field needs fixing.
    func savePlan(currency: String, plannedTotal: String, contingencyMode: String, contingencyValue: String, notes: String) async -> String? {
        guard let b = current else { return "There is no budget yet." }
        var planned: Double?
        switch BudgetMoney.parse(plannedTotal, currency: currency) {
        case .empty: planned = nil
        case let .amount(a): planned = a
        case .invalid: return "Enter the planned total as a non-negative amount."
        }
        var cv: Double
        if contingencyMode == "percent" {
            let text = contingencyValue.replacingOccurrences(of: "%", with: "").trimmingCharacters(in: .whitespaces)
            let pct = text.isEmpty ? 0 : (Double(text.replacingOccurrences(of: ",", with: ".")) ?? .nan)
            guard pct.isFinite, pct >= 0, pct <= 100 else { return "Contingency must be between 0% and 100%." }
            cv = MoodGeometry.round(pct * 100)
        } else {
            switch BudgetMoney.parse(contingencyValue, currency: currency) {
            case .empty: cv = 0
            case let .amount(a): cv = a
            case .invalid: return "Enter the contingency as a non-negative amount."
            }
        }
        let n = notes.trimmingCharacters(in: .whitespacesAndNewlines)
        let r = await run("Could not change the budget plan", done: "Changed the budget plan") {
            try await $0.budgetUpdate(BudgetUpdateArgs(
                id: b.id, currency: currency, plannedTotal: planned, contingencyMode: contingencyMode, contingencyValue: cv,
                notes: n.isEmpty ? nil : n, expectedRev: b.rev))
        }
        if r != nil {
            sheet = nil
            await reload()
            return nil
        }
        return ""
    }

    func saveSnapshot(name: String) async -> Bool {
        guard let b = current, !name.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        let label = name.trimmingCharacters(in: .whitespaces)
        let r = await run("Could not save the snapshot", done: "Saved budget snapshot \u{201C}\(label)\u{201D}") { try await $0.budgetSaveSnapshot(BudgetSnapshotArgs(id: b.id, label: name)) }
        if r != nil {
            sheet = nil
            await reload()
        }
        return r != nil
    }

    func deleteSnapshot(_ row: BudgetSnapshotRow) async {
        let r = await run("Could not delete the snapshot", done: "Deleted budget snapshot \u{201C}\(row.label ?? "Snapshot")\u{201D}") {
            try await $0.budgetDeleteSnapshot(BudgetIdArgs(id: row.id))
        }
        if r != nil { await reload() }
    }

    func markReviewed() async {
        guard let b = current, reviewReminder != nil else { return }
        let r = await run("Could not mark the budget as reviewed", done: "Marked the budget as reviewed") { try await $0.budgetMarkReviewed(BudgetIdArgs(id: b.id)) }
        if r != nil { await reload() }
    }

    // MARK: Export

    func export(format: String, path: String, budgetId: String?, includeNotes: Bool) async -> ExportResult? {
        guard let core else { return nil }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.budgetExportSummary(ExportBudgetArgs(format: format, path: path, budgetId: budgetId, includeNotes: includeNotes))
            ToastCenter.shared.show("Exported \(r.fileName)")
            return r
        } catch {
            app.show("Could not export the budget", error)
            return nil
        }
    }
}

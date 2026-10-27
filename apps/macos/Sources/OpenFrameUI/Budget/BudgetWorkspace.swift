// Budget: "What did we plan to spend, and what have we spent?" Advisory only: categories, lines with
// a Planned and an Actual amount, category totals, the difference, a planned total and a
// contingency. Values change only when the person edits them, never automatically. Mirrors
// apps/desktop/src/workspaces/production/tabs/budget.tsx.

import OpenFrameCore
import SwiftUI

struct BudgetWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var model: BudgetModel?

    var body: some View {
        Group {
            if let model {
                BudgetContent(model: model)
            } else {
                ProgressView()
            }
        }
        .fitsWindow()
        .task { if model == nil { model = BudgetModel(app: app) } }
    }
}

private enum BudgetTables {
    /// production_source and the draft tables drive the manual "Review budget" reminder only.
    static let all = ["budget_snapshot", "budget_line", "production_source", "screenplay_draft", "screenplay"]
}

struct BudgetContent: View {
    @Bindable var model: BudgetModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(spacing: PageSpacing.medium) {
                header
                VisualBanner(tone: .info, identifier: "budget-advisory", inset: 0) {
                    Text("Use a simple estimate for planning. Formal accounting is outside OpenFrame.")
                } actions: { EmptyView() }
                if !model.loaded {
                    ProgressView().frame(maxWidth: .infinity, minHeight: 120)
                } else if let f = model.loadFailure, model.view == nil {
                    VisualBanner(tone: .error, identifier: "budget-load-failed", inset: 0) {
                        Text(f.message)
                    } actions: {
                        Button("Try Again") { Task { await model.reload() } }.accessibilityIdentifier("budget-retry")
                    }
                } else if let b = model.current {
                    if let reminder = model.reviewReminder {
                        VisualBanner(tone: .warn, identifier: "budget-review-banner", inset: 0) {
                            Text("**Review budget.** \(reminder)")
                        } actions: {
                            Button("Mark reviewed") { Task { await model.markReviewed() } }.accessibilityIdentifier("budget-mark-reviewed")
                        }
                        }
                    tiles(b)
                    BudgetTable(budget: b, readOnly: false, onEdit: { model.sheet = .line($0) }, onDelete: { l in Task { await model.deleteLine(l) } })
                    if let notes = b.notes, !notes.isEmpty {
                        Text(notes).font(.callout).textSelection(.enabled).accessibilityIdentifier("budget-notes")
                    }
                    Text("Budget values never change automatically. If the script or schedule changes, review the budget yourself.")
                        .font(.caption).foregroundStyle(theme["muted"]).multilineTextAlignment(.center)
                } else {
                    startCard
                }
                if !model.snapshots.isEmpty { snapshots }
            }
            .pageColumn(maxWidth: PageSpacing.wideColumn)
            .padding(.bottom, PageSpacing.large)
        }
        .background(theme["bg"])
        .task(id: app.changes.key(BudgetTables.all)) { await model.reload() }
        .themedSheet(item: $model.sheet) { BudgetSheetHost(model: model, sheet: $0) }
        .accessibilityIdentifier("budget-workspace")
    }

    // MARK: Header

    private var header: some View {
        PageHeader("Budget", subtitle: "What did we plan to spend, and what have we spent?") {
            if model.current != nil {
                HStack(spacing: PageSpacing.small) {
                    Button { model.sheet = .export(budgetId: nil) } label: { Label("Export", systemImage: "square.and.arrow.up") }
                        .accessibilityIdentifier("budget-export")
                    Button("Save Snapshot") { model.sheet = .snapshotName }
                        .accessibilityIdentifier("budget-save-snapshot")
                    Button { model.sheet = .line(nil) } label: { Label("Add Line", systemImage: "plus") }
                        .buttonStyle(.ofProminent)
                        .keyboardShortcut("n", modifiers: [.command, .option])
                        .accessibilityIdentifier("budget-add-line")
                }
            }
        }
    }

    // MARK: Start card

    private var startCard: some View {
        VStack(spacing: PageSpacing.small) {
            Text("Start a budget").font(.headline)
            LabeledBlock(label: "Currency") {
                Picker("Currency", selection: $model.startCurrency) {
                    ForEach(BudgetMoney.currencies, id: \.self) { Text($0).tag($0) }
                }
                .labelsHidden().fixedSize()
                .accessibilityIdentifier("budget-start-currency")
            }
            Button("Start Budget") { Task { _ = await model.start(currency: model.startCurrency) } }
                .buttonStyle(.ofProminent)
                .disabled(model.busy)
                .accessibilityIdentifier("budget-start")
            Text("Optional. You can ignore the budget entirely.").font(.caption).foregroundStyle(theme["muted"])
        }
        .padding(PageSpacing.medium)
        .frame(maxWidth: 440)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .accessibilityIdentifier("budget-empty")
    }

    // MARK: Totals

    private func tiles(_ b: BudgetDto) -> some View {
        let planned = Int(b.contingencyValue)
        let contingency = b.contingencyMode == "percent"
            ? "\(planned % 100 == 0 ? String(planned / 100) : String(format: "%g", Double(planned) / 100))% contingency "
            : "Contingency "
        return HStack(alignment: .top, spacing: PageSpacing.medium) {
            tile("Planned", id: "budget-planned") {
                Text(BudgetMoney.format(b.plannedLinesTotal, b.currency)).font(.title.weight(.semibold))
                Text("\(b.plannedTotal.map { "Overall plan \(BudgetMoney.format($0, b.currency))" } ?? "Sum of the Planned amounts") \u{00B7} \(contingency)\(BudgetMoney.format(b.contingencyAmount, b.currency))")
                    .font(.caption).foregroundStyle(theme["muted"])
                Button("Edit Plan") { model.sheet = .plan }.controlSize(.small).accessibilityIdentifier("budget-edit-plan")
            }
            tile("Actual", id: "budget-actual") {
                Text(b.actualCount > 0 ? BudgetMoney.format(b.actualTotal, b.currency) : "None yet").font(.title.weight(.semibold))
                Text("\(Int(b.actualCount)) of \(b.lines.count) \(b.lines.count == 1 ? "line has" : "lines have") an Actual amount")
                    .font(.caption).foregroundStyle(theme["muted"])
            }
            tile("Difference", id: "budget-difference") {
                Group {
                    if b.actualCount > 0 {
                        BudgetDifference(value: b.difference, currency: b.currency)
                    } else {
                        Text("Not spent yet")
                    }
                }
                .font(.title3.weight(.semibold))
                Text("Planned minus Actual, over the lines with an Actual amount").font(.caption).foregroundStyle(theme["muted"])
            }
        }
    }

    private func tile<C: View>(_ title: String, id: String, @ViewBuilder content: () -> C) -> some View {
        VStack(spacing: 4) {
            Text(title.uppercased()).font(.caption2.weight(.semibold)).foregroundStyle(theme["muted"])
            content()
        }
        .multilineTextAlignment(.center)
        .padding(PageSpacing.medium)
        .frame(maxWidth: .infinity)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier(id)
    }

    // MARK: Snapshots

    private var snapshots: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Saved snapshots").font(.headline)
            ForEach(model.snapshots, id: \.id) { s in
                HStack {
                    Text(s.label ?? "Snapshot")
                    if let f = s.frozenAt { Text(VisualText.formatWhen(f)).font(.caption).foregroundStyle(theme["muted"]) }
                    Spacer()
                    Text("Planned \(BudgetMoney.format(s.plannedLinesTotal, s.currency)) \u{00B7} Actual \(BudgetMoney.format(s.actualTotal, s.currency))").font(.callout)
                    Button("View") { model.sheet = .snapshot(s.id) }.controlSize(.small)
                        .accessibilityIdentifier("budget-snapshot-view-\(s.id)")
                    Button { Task { await model.deleteSnapshot(s) } } label: { Label("Delete", systemImage: "trash") }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Delete snapshot \(s.label ?? "")")
                        .accessibilityIdentifier("budget-snapshot-delete-\(s.id)")
                }
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("budget-snapshot-\(s.id)")
                Divider()
            }
        }
        .padding(PageSpacing.medium)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .accessibilityIdentifier("budget-snapshots")
    }
}

// MARK: Difference text

struct BudgetDifference: View {
    let value: Double?
    let currency: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if let value {
            Text(BudgetMoney.differenceText(value, currency))
                .foregroundStyle(value < 0 ? theme["red-t"] : theme["ink"])
                .fontWeight(value < 0 ? .semibold : .regular)
        } else {
            Text("Not spent yet").foregroundStyle(theme["muted"])
        }
    }
}

// MARK: Table

struct BudgetTable: View {
    let budget: BudgetDto
    var readOnly: Bool
    var onEdit: ((BudgetLineDto) -> Void)?
    var onDelete: ((BudgetLineDto) -> Void)?
    @Environment(\.ofTheme) private var theme

    private var cats: [BudgetCategoryTotal] { budget.categories.filter { $0.count > 0 } }

    /// The table keeps its column widths. In a window too narrow for them it scrolls sideways
    /// instead of squeezing the line item column to nothing.
    var body: some View {
        ViewThatFits(in: .horizontal) {
            grid
            ScrollView(.horizontal) { grid.frame(width: 800) }
        }
    }

    private var grid: some View {
        VStack(spacing: 0) {
            head
            Divider()
            if cats.isEmpty {
                Text("No lines yet. Add a line for each rough cost you expect.")
                    .foregroundStyle(theme["muted"]).padding(PageSpacing.medium).frame(maxWidth: .infinity)
                    .accessibilityIdentifier("budget-no-lines")
            } else {
                ForEach(cats, id: \.category) { c in category(c) }
                row(cells: ["", "All lines", BudgetMoney.format(budget.plannedLinesTotal, budget.currency),
                            budget.actualCount > 0 ? BudgetMoney.format(budget.actualTotal, budget.currency) : "None yet"],
                    diff: budget.actualCount > 0 ? budget.difference : nil, showDiff: true, subtotal: true, id: "budget-total-all")
            }
        }
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Budget lines")
        .accessibilityIdentifier("budget-table")
    }

    private var head: some View {
        HStack(spacing: 8) {
            Text("Category").frame(width: 130, alignment: .leading)
            Text("Line item").frame(minWidth: 180, maxWidth: .infinity, alignment: .leading)
            Text("Planned").frame(width: 100, alignment: .trailing)
            Text("Actual").frame(width: 100, alignment: .trailing)
            Text("Difference").frame(width: 150, alignment: .trailing)
            if !readOnly { Spacer().frame(width: 64) }
        }
        .font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
        .padding(.horizontal, 12).padding(.vertical, 8)
    }

    @ViewBuilder private func category(_ total: BudgetCategoryTotal) -> some View {
        let lines = budget.lines.filter { $0.category == total.category }
        ForEach(Array(lines.enumerated()), id: \.element.id) { i, l in
            HStack(spacing: 8) {
                Text(i == 0 ? total.category : "").frame(width: 130, alignment: .leading)
                VStack(alignment: .leading, spacing: 1) {
                    Text(l.description)
                    if let n = l.notes, !n.isEmpty { Text(n).font(.caption).foregroundStyle(theme["muted"]) }
                }
                .frame(minWidth: 180, maxWidth: .infinity, alignment: .leading)
                Text(BudgetMoney.format(l.planned, budget.currency)).frame(width: 100, alignment: .trailing)
                Group {
                    if let a = l.actual { Text(BudgetMoney.format(a, budget.currency)) } else { Text("Not entered").foregroundStyle(theme["muted"]) }
                }
                .frame(width: 100, alignment: .trailing)
                BudgetDifference(value: l.difference, currency: budget.currency).frame(width: 150, alignment: .trailing)
                if !readOnly {
                    HStack(spacing: 4) {
                        Button { onEdit?(l) } label: { Label("Edit", systemImage: "pencil") }
                            .buttonStyle(.borderless)
                            .accessibilityLabel("Edit \(l.description)")
                            .accessibilityIdentifier("budget-edit-\(l.id)")
                        Button { onDelete?(l) } label: { Label("Delete", systemImage: "trash") }
                            .buttonStyle(.borderless)
                            .accessibilityLabel("Delete \(l.description)")
                            .accessibilityIdentifier("budget-delete-\(l.id)")
                    }
                    .frame(width: 64, alignment: .trailing)
                }
            }
            .padding(.horizontal, 12).padding(.vertical, 6)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("budget-line-\(l.id)")
            Divider()
        }
        row(cells: ["", "\(total.category) total", BudgetMoney.format(total.planned, budget.currency),
                    total.actualCount > 0 ? BudgetMoney.format(total.actual, budget.currency) : ""],
            diff: total.actualCount > 0 ? total.difference : nil, showDiff: total.actualCount > 0, subtotal: true, id: "budget-total-\(total.category)")
    }

    private func row(cells: [String], diff: Double?, showDiff: Bool, subtotal: Bool, id: String) -> some View {
        HStack(spacing: 8) {
            Text(cells[0]).frame(width: 130, alignment: .leading)
            Text(cells[1]).frame(minWidth: 180, maxWidth: .infinity, alignment: .leading)
            Text(cells[2]).frame(width: 100, alignment: .trailing)
            Text(cells[3]).frame(width: 100, alignment: .trailing)
            Group { if showDiff { BudgetDifference(value: diff, currency: budget.currency) } else { Text("") } }.frame(width: 150, alignment: .trailing)
            if !readOnly { Spacer().frame(width: 64) }
        }
        .fontWeight(.bold)
        .padding(.horizontal, 12).padding(.vertical, 6)
        .background(theme["surface2"])
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(id)
    }
}

// MARK: Sheets

private struct BudgetSheetHost: View {
    let model: BudgetModel
    let sheet: BudgetSheet

    var body: some View {
        switch sheet {
        case let .line(l): BudgetLineSheet(model: model, line: l)
        case .plan: BudgetPlanSheet(model: model)
        case .snapshotName: SnapshotNameSheet(model: model)
        case let .snapshot(id): SnapshotSheet(model: model, id: id)
        case let .export(id): BudgetExportSheet(model: model, budgetId: id)
        }
    }
}

private struct BudgetLineSheet: View {
    let model: BudgetModel
    let line: BudgetLineDto?
    @State private var category = ""
    @State private var description = ""
    @State private var planned = ""
    @State private var actual = ""
    @State private var notes = ""
    @State private var error: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let currency = model.current?.currency ?? "USD"
        VisualFormSheet(title: line == nil ? "Add Line" : "Edit line", primary: line == nil ? "Add Line" : "Save", busy: model.busy,
                        identifier: "budget-line-sheet", width: 420, onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Category") {
                Picker("Category", selection: $category) { ForEach(model.categories, id: \.self) { Text($0).tag($0) } }
                    .labelsHidden().accessibilityIdentifier("of-bl-cat")
            }
            LabeledBlock(label: "Line item") {
                TextField("Line item", text: $description).textFieldStyle(.roundedBorder)
                    .onChange(of: description) { _, v in if v.count > 200 { description = String(v.prefix(200)) } }
                    .accessibilityIdentifier("of-bl-desc")
            }
            LabeledBlock(label: "Planned (\(currency))") {
                TextField("Planned", text: $planned).textFieldStyle(.roundedBorder).onSubmit(submit)
                    .accessibilityIdentifier("of-bl-amt")
            }
            LabeledBlock(label: "Actual (\(currency))", hint: "Leave empty until the money is spent.", error: error) {
                TextField("Actual", text: $actual).textFieldStyle(.roundedBorder).onSubmit(submit)
                    .accessibilityIdentifier("of-bl-act")
            }
            LabeledBlock(label: "Notes") {
                TextEditor(text: $notes).frame(height: 52).padding(4)
                    .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line2"]))
                    .accessibilityLabel("Notes").accessibilityIdentifier("of-bl-notes")
            }
        }
        .onAppear {
            category = line?.category ?? model.categories.first ?? ""
            description = line?.description ?? ""
            planned = line.map { BudgetMoney.toInput($0.planned, currency: currency) } ?? ""
            actual = line?.actual.map { BudgetMoney.toInput($0, currency: currency) } ?? ""
            notes = line?.notes ?? ""
        }
    }

    private func submit() {
        Task { error = await model.saveLine(editing: line, category: category, description: description, planned: planned, actual: actual, notes: notes) }
    }
}

private struct BudgetPlanSheet: View {
    let model: BudgetModel
    @State private var currency = "USD"
    @State private var planned = ""
    @State private var mode = "percent"
    @State private var value = ""
    @State private var notes = ""
    @State private var error: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VisualFormSheet(title: "Budget plan", primary: "Save", busy: model.busy, identifier: "budget-plan-sheet", width: 420,
                        onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Currency") {
                Picker("Currency", selection: $currency) { ForEach(BudgetMoney.currencies, id: \.self) { Text($0).tag($0) } }
                    .labelsHidden().fixedSize().accessibilityIdentifier("of-b-cur")
            }
            LabeledBlock(label: "Planned total", hint: "Your overall estimate. Optional.") {
                TextField("Planned total", text: $planned).textFieldStyle(.roundedBorder).accessibilityIdentifier("of-b-plan")
            }
            LabeledBlock(label: "Contingency") {
                HStack {
                    Picker("Contingency type", selection: $mode) {
                        Text("Percent").tag("percent")
                        Text("Amount").tag("amount")
                    }
                    .pickerStyle(.segmented).labelsHidden().fixedSize()
                    .accessibilityLabel("Contingency type").accessibilityIdentifier("of-b-cont-mode")
                    TextField("Contingency value", text: $value).textFieldStyle(.roundedBorder).frame(maxWidth: 140)
                        .accessibilityIdentifier("of-b-cont-value")
                    if mode == "percent" { Text("%").foregroundStyle(theme["muted"]) }
                }
            }
            LabeledBlock(label: "Notes", error: error) {
                TextEditor(text: $notes).frame(height: 52).padding(4)
                    .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line2"]))
                    .accessibilityLabel("Notes").accessibilityIdentifier("of-b-notes")
            }
        }
        .onAppear {
            guard let b = model.current else { return }
            currency = b.currency
            planned = BudgetMoney.toInput(b.plannedTotal, currency: b.currency)
            mode = b.contingencyMode == "amount" ? "amount" : "percent"
            value = mode == "amount" ? BudgetMoney.toInput(b.contingencyValue, currency: b.currency) : String(format: "%g", b.contingencyValue / 100)
            notes = b.notes ?? ""
        }
    }

    private func submit() {
        Task { error = await model.savePlan(currency: currency, plannedTotal: planned, contingencyMode: mode, contingencyValue: value, notes: notes) }
    }
}

private struct SnapshotNameSheet: View {
    let model: BudgetModel
    @State private var name = ""

    var body: some View {
        VisualFormSheet(title: "Save budget snapshot", primary: "Save Snapshot", primaryEnabled: !name.trimmingCharacters(in: .whitespaces).isEmpty,
                        busy: model.busy, identifier: "budget-snapshot-sheet", width: 400, onCancel: { model.sheet = nil },
                        onPrimary: { Task { _ = await model.saveSnapshot(name: name) } }) {
            LabeledBlock(label: "Snapshot name", hint: "A frozen copy you can look back at later.") {
                TextField("Snapshot name", text: $name, prompt: Text("Pre-production estimate")).textFieldStyle(.roundedBorder)
                    .onSubmit { Task { _ = await model.saveSnapshot(name: name) } }
                    .accessibilityIdentifier("of-b-snap")
            }
        }
    }
}

private struct SnapshotSheet: View {
    let model: BudgetModel
    let id: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: PageSpacing.small) {
            if let s = model.snapshot {
                Text(s.label ?? "Budget snapshot").font(.title3.weight(.semibold))
                if let f = s.frozenAt { Text("Saved \(VisualText.formatWhen(f)) \u{00B7} read-only").font(.caption).foregroundStyle(theme["muted"]) }
                Text("Planned **\(BudgetMoney.format(s.plannedLinesTotal, s.currency))** \u{00B7} Actual **\(s.actualCount > 0 ? BudgetMoney.format(s.actualTotal, s.currency) : "none yet")**\(s.plannedTotal.map { " \u{00B7} Overall plan \(BudgetMoney.format($0, s.currency))" } ?? "")")
                    .font(.callout).multilineTextAlignment(.center).accessibilityIdentifier("budget-snapshot-summary")
                ScrollView { BudgetTable(budget: s, readOnly: true) }.frame(maxHeight: 360)
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 120)
            }
            HStack {
                Spacer()
                Button("Export\u{2026}") { model.sheet = .export(budgetId: id) }.accessibilityIdentifier("budget-snapshot-export")
                Button("Close") { model.sheet = nil }.keyboardShortcut(.cancelAction).accessibilityIdentifier("budget-snapshot-close")
            }
        }
        .padding(PageSpacing.large).frame(width: 760)
        .task { await model.loadSnapshot(id) }
        .accessibilityIdentifier("budget-snapshot-view")
    }
}

private struct BudgetExportSheet: View {
    let model: BudgetModel
    let budgetId: String?

    var body: some View {
        VisualExportSheet(
            title: "Export Budget", documentName: budgetId != nil ? "Budget Snapshot" : "Budget Summary", formats: [.pdf, .csv, .xlsx],
            options: [VisualExportOption(key: "includeNotes", label: "Include line notes", isOn: false)],
            identifier: "budget-export-sheet", onCancel: { model.sheet = nil }
        ) { req in
            await model.export(format: req.format, path: req.path, budgetId: budgetId, includeNotes: req.flags["includeNotes"] ?? false)
        }
    }
}

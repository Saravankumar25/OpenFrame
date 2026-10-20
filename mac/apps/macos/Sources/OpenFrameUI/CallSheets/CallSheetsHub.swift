// The root model of the Call Sheets workspace. It owns the models of its four tabs (Call Sheets,
// Daily View, Sides, Reports), the open call sheet, the export sheet and the way a call sheet is
// opened from a shooting day. It mirrors apps/desktop/src/workspaces/call-sheets (index.tsx,
// CallSheetList.tsx, shared/open-call-sheet.ts) and the Daily View, Sides and Reports tabs of the
// Production workspace, which the Mac shows here. It holds no production data of its own: every
// read and change is a core operation (ADR-0027).

import Foundation
import Observation
import OpenFrameCore

enum CSTab: String, CaseIterable, Identifiable {
    case callSheets, daily, sides, reports
    var id: String { rawValue }
    var title: String {
        switch self {
        case .callSheets: "Call Sheets"
        case .daily: "Daily View"
        case .sides: "Sides"
        case .reports: "Reports"
        }
    }
}

@MainActor
@Observable
final class CallSheetsHub {
    let app: AppModel
    var tab: CSTab = .callSheets
    /// The call sheet shown in the editor, when one is open.
    private(set) var openId: String?
    private(set) var editor: CallSheetEditorModel?
    /// The export sheet, when one is open.
    var exportModel: CSExportModel?
    /// A hint for the screens. The core still refuses what a role may not do.
    private(set) var canEdit = true

    @ObservationIgnored private(set) var list: CallSheetListModel!
    @ObservationIgnored private(set) var sides: CSSidesModel!
    @ObservationIgnored private(set) var reports: CSReportsModel!
    @ObservationIgnored private(set) var daily: CSDailyModel!

    init(app: AppModel) {
        self.app = app
        list = CallSheetListModel(hub: self)
        sides = CSSidesModel(hub: self)
        reports = CSReportsModel(hub: self)
        daily = CSDailyModel(hub: self)
    }

    var core: Core? { app.core }

    var projectTitle: String? {
        if case let .project(p) = app.phase { return p.title }
        return nil
    }

    func reloadMembers() async {
        guard let core else { return }
        if let members = try? await core.projectMembers() {
            let me = members.first { $0.isYou }
            canEdit = me == nil || me?.role == "Owner" || me?.role == "Editor"
        }
    }

    // MARK: Messages

    func say(_ text: String, undoable: Bool = false) {
        if undoable {
            ToastCenter.shared.undoable(text, app: app)
        } else {
            ToastCenter.shared.show(text)
        }
    }

    func fail(_ title: String, _ error: Error) {
        app.show(title, error)
    }

    // MARK: Opening call sheets

    /// Opens a call sheet in the editor. `action` is "refresh" or "finalize" from the list menu.
    func open(_ id: String, action: String? = nil) {
        tab = .callSheets
        if openId != id || editor == nil {
            openId = id
            editor = CallSheetEditorModel(hub: self, id: id)
        }
        if action == "refresh" { editor?.dialog = .refresh }
        if action == "finalize" { editor?.dialog = .finalize }
    }

    /// Back to the list. Saves first so no typed value is lost.
    func closeEditor() async {
        if let editor { _ = await editor.flush() }
        openId = nil
        editor = nil
    }

    /// "Create Call Sheet" from a shooting day. A day that already has one opens it
    /// (Windows `openCallSheetForDay`).
    func openForDay(_ dayId: String, existing: String? = nil, template: TemplateRefArgs? = nil) async {
        if let existing {
            open(existing)
            return
        }
        guard let core else { return }
        do {
            let id = try await core.callsheetsCreate(CallSheetCreateArgs(dayId: dayId, template: template))
            say("Created a call sheet from the shooting day", undoable: true)
            open(id)
        } catch {
            let f = CoreFailure(asCore: error)
            if f.code == "conflict.call_sheet_exists", let id = f.detail {
                open(id)
                return
            }
            fail("Could not create the call sheet", error)
        }
    }

    /// Takes a request another workspace left in `CallSheetsRoute`.
    func consumeRoute() async {
        guard let r = CallSheetsRoute.shared.take() else { return }
        switch r {
        case let .sheet(id): open(id)
        case let .day(dayId):
            let existing = try? await core?.callsheetsList().first { $0.dayId == dayId && !$0.replaced }?.id
            await openForDay(dayId, existing: existing)
        case let .sides(day):
            tab = .sides
            sides.focusDay(day)
        case let .daily(day):
            tab = .daily
            daily.pick(day)
        }
    }

    // MARK: Exports

    func exportCallSheet(id: String, title: String, status: String) {
        let word = CSStatus.word(status)
        let frozen = word == "Final"
        exportModel = CSExportModel(
            title: "Export Call Sheet",
            sub: "Creates the PDF snapshot to send.",
            source: "Source: \(title) \u{00B7} \(word)",
            documentName: title,
            note: frozen
                ? "This call sheet is final: the PDF is made from its frozen snapshot and never changes it. Private notes excluded."
                : "Private notes excluded. Export does not change the shooting schedule. Finalize the call sheet before sending it.",
            formats: [.pdf],
            options: [CSExportOption(
                key: "includeOptional",
                label: "Include the added sections (weather, extras, department notes, advance schedule, crew, contacts, attachments, reference images)",
                defaultValue: true)],
            projectTitle: projectTitle
        ) { [weak self] format, path, values in
            guard let core = self?.core else { throw CoreFailure(code: "no_project", message: "No project is open.") }
            return try await core.callsheetsExportPdf(ExportCallSheetArgs(
                id: id, format: format, path: path, includeOptional: values["includeOptional"] ?? true))
        }
    }

    func exportSides(args: ScheduleSidesArgs?, sideId: String?, name: String) {
        exportModel = CSExportModel(
            title: "Export Sides",
            sub: "Only the selected scenes, in screenplay format, with the draft name on every page.",
            source: "Source: \(name)",
            documentName: name,
            note: "Sides are a snapshot of the screenplay source. Editing the PDF never changes the screenplay. Private notes excluded.",
            formats: [.pdf],
            projectTitle: projectTitle
        ) { [weak self] format, path, _ in
            guard let core = self?.core else { throw CoreFailure(code: "no_project", message: "No project is open.") }
            if let sideId {
                return try await core.sidesExportPdf(ExportSidesArgs(format: format, path: path, sideId: sideId))
            }
            return try await core.sidesExportPdf(ExportSidesArgs(
                format: format, path: path, dayId: args?.dayId, stripIds: args?.stripIds, castKeys: args?.castKeys,
                includeCover: args?.includeCover, showDraftName: args?.showDraftName))
        }
    }

    /// The day shown on the Daily View, as a document of its own named after the day (owner
    /// request 2026-10-20): "Shoot Day 4 - Daily view".
    func exportDaily(dayId: String, label: String) {
        let name = "\(label) - Daily view"
        exportModel = CSExportModel(
            title: "Export Daily View",
            sub: "Everything this page shows for the day: scenes in order, location, cast and call times, notes, breakdown items.",
            source: "Source: \(label)",
            documentName: name,
            note: "Private notes excluded. Export creates a snapshot and does not change the schedule.",
            formats: CSExportFormat.tables,
            projectTitle: projectTitle,
            plainName: true
        ) { [weak self] format, path, _ in
            guard let core = self?.core else { throw CoreFailure(code: "no_project", message: "No project is open.") }
            return try await core.scheduleExportDaily(ExportDailyArgs(format: format, path: path, dayId: dayId))
        }
    }

    func exportReport(type: String?, filter: String?, reportId: String?, title: String) {
        let filtered = (filter?.isEmpty == false) ? " \u{00B7} filtered by \u{201C}\(filter ?? "")\u{201D}" : ""
        exportModel = CSExportModel(
            title: "Export \(title)",
            source: reportId != nil ? "Source: saved report \(title)" : "Source: \(title)\(filtered)",
            documentName: title,
            note: "Private notes excluded. Export creates a snapshot and does not change your project.",
            formats: CSExportFormat.tables,
            projectTitle: projectTitle
        ) { [weak self] format, path, _ in
            guard let core = self?.core else { throw CoreFailure(code: "no_project", message: "No project is open.") }
            if let reportId {
                return try await core.reportsExportReport(ExportReportArgs(format: format, path: path, reportId: reportId))
            }
            return try await core.reportsExportReport(ExportReportArgs(
                format: format, path: path, reportType: type, filter: (filter?.isEmpty == false) ? filter : nil))
        }
    }
}

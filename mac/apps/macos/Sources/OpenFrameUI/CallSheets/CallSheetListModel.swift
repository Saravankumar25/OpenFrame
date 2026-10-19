// The list of call sheets and the Create Call Sheet sheet (Windows CallSheetList.tsx). It reads
// `callsheets.list` and `schedule.get`, filters in the window, and sends deletes and creates to
// the core.

import Foundation
import Observation
import OpenFrameCore

enum CSFilter: String, CaseIterable, Identifiable {
    case all, draft, refresh, final
    var id: String { rawValue }
    var title: String {
        switch self {
        case .all: "All"
        case .draft: "Draft"
        case .refresh: "Needs refresh"
        case .final: "Final"
        }
    }

    func matches(_ status: String) -> Bool {
        switch self {
        case .all: true
        case .draft: CSStatus.word(status) == "Draft"
        case .refresh: CSStatus.word(status) == "Needs refresh"
        case .final: CSStatus.word(status) == "Final"
        }
    }
}

@MainActor
@Observable
final class CallSheetListModel {
    unowned let hub: CallSheetsHub
    private(set) var rows: [CallSheetListRow] = []
    private(set) var schedule: ScheduleView?
    private(set) var loaded = false
    private(set) var failure: CoreFailure?
    var filter: CSFilter = .all
    var text = ""
    var showOld = false
    var selection: String?
    var create: CreateCallSheetModel?

    init(hub: CallSheetsHub) { self.hub = hub }

    func reload() async {
        guard let core = hub.core else { return }
        do {
            rows = try await core.callsheetsList()
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error)
        }
        schedule = try? await core.scheduleGet()
        loaded = true
    }

    var isEmpty: Bool { rows.isEmpty }

    var filtered: [CallSheetListRow] {
        let t = text.trimmingCharacters(in: .whitespaces).lowercased()
        return rows.filter { r in
            (showOld || !r.replaced) && filter.matches(r.status)
                && (t.isEmpty || "\(r.title) \(r.dayLabel) \(r.dateLabel ?? "") \(r.status)".lowercased().contains(t))
        }
    }

    func beginCreate() {
        create = CreateCallSheetModel(hub: hub, view: schedule)
    }

    func delete(_ row: CallSheetListRow) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.callsheetsDelete(CallSheetIdArgs(id: row.id))
            hub.say("Deleted \(row.title)", undoable: true)
            await reload()
        } catch {
            hub.fail("Could not delete the call sheet", error)
        }
    }
}

/// The "Create Call Sheet" sheet: choose the shooting day (and a call sheet template, if any).
@MainActor
@Observable
final class CreateCallSheetModel {
    unowned let hub: CallSheetsHub
    let hasSchedule: Bool
    let candidates: [ScheduleShootDayDto]
    var dayId: String
    private(set) var templates: [TemplateDto] = []
    /// "none" or "scope:id".
    var templateKey = "none"
    private(set) var busy = false

    init(hub: CallSheetsHub, view: ScheduleView?) {
        self.hub = hub
        hasSchedule = view?.schedule != nil
        candidates = (view?.days ?? []).filter { !$0.isOffDay && $0.callSheet == nil }
        dayId = candidates.first?.id ?? ""
    }

    func loadTemplates() async {
        guard let core = hub.core else { return }
        if let l = try? await core.templatesList(TemplateListArgs(templateType: "call_sheet")) {
            templates = l.builtin + l.global + l.project
        }
    }

    static func key(_ t: TemplateDto) -> String { "\(t.scope.rawValue):\(t.id)" }

    var chosenTemplate: TemplateRefArgs? {
        guard templateKey != "none", let t = templates.first(where: { Self.key($0) == templateKey }) else { return nil }
        return TemplateRefArgs(scope: t.scope, id: t.id)
    }

    var canCreate: Bool { !dayId.isEmpty && !busy }

    /// Creates the sheet and opens it. Returns false when the day is missing.
    @discardableResult
    func confirm() async -> Bool {
        guard canCreate else { return false }
        busy = true
        await hub.openForDay(dayId, template: chosenTemplate)
        busy = false
        return true
    }
}

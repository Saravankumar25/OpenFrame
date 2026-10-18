// The Reports tab (Windows production/tabs/reports.tsx) and the Daily View tab (daily.tsx).
// Reports are read-only projections generated on demand; nothing persists unless the person saves
// a snapshot. The Daily View is a derived view of one shooting day and edits nothing.

import Foundation
import Observation
import OpenFrameCore

struct CSReportKind: Identifiable, Hashable {
    let value: String
    let label: String
    var id: String { value }

    static let all: [CSReportKind] = [
        CSReportKind(value: "scene", label: "Scene Report"),
        CSReportKind(value: "location", label: "Location Report"),
        CSReportKind(value: "cast_scene", label: "Cast Scene Report"),
        CSReportKind(value: "prop", label: "Prop Report"),
        CSReportKind(value: "schedule", label: "Schedule Report"),
        CSReportKind(value: "breakdown_completeness", label: "Breakdown Completeness"),
    ]
}

@MainActor
@Observable
final class CSReportsModel {
    unowned let hub: CallSheetsHub

    var type = "scene"
    var filter = ""
    private(set) var live: ReportData?
    private(set) var liveFailure: CoreFailure?
    private(set) var list: [ReportSavedRow] = []
    private(set) var savedId: String?
    private(set) var saved: ReportSaved?
    private(set) var savedFailure: CoreFailure?
    private(set) var loaded = false

    init(hub: CallSheetsHub) { self.hub = hub }

    var trimmedFilter: String { filter.trimmingCharacters(in: .whitespaces) }
    var title: String { CSReportKind.all.first { $0.value == type }?.label ?? "Report" }
    var key: String { "\(type)|\(trimmedFilter)|\(savedId ?? "")" }

    func choose(_ value: String) {
        type = value
        savedId = nil
    }

    func openSnapshot(_ id: String) {
        savedId = id
        saved = nil
        savedFailure = nil
    }

    func backToLive() { savedId = nil }

    func reload() async {
        guard let core = hub.core else { return }
        list = (try? await core.reportsList()) ?? list
        if let id = savedId {
            do {
                saved = try await core.reportsGet(ScheduleDocIdArgs(id: id))
                savedFailure = nil
            } catch {
                saved = nil
                savedFailure = CoreFailure(asCore: error)
            }
        } else {
            do {
                live = try await core.reportsGenerate(ReportArgs(reportType: type, filter: trimmedFilter.isEmpty ? nil : trimmedFilter))
                liveFailure = nil
            } catch {
                live = nil
                liveFailure = CoreFailure(asCore: error)
            }
        }
        loaded = true
    }

    func saveSnapshot() async {
        guard let core = hub.core else { return }
        do {
            let id = try await core.reportsSave(ReportArgs(reportType: type, filter: trimmedFilter.isEmpty ? nil : trimmedFilter))
            hub.say("Saved a report snapshot", undoable: true)
            list = (try? await core.reportsList()) ?? list
            openSnapshot(id)
        } catch {
            hub.fail("Could not save the report", error)
        }
    }

    func deleteSnapshot(_ id: String) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.reportsDelete(ScheduleDocIdArgs(id: id))
            hub.say("Deleted a saved report", undoable: true)
            list = (try? await core.reportsList()) ?? list
            if savedId == id { savedId = nil }
        } catch {
            hub.fail("Could not delete the saved report", error)
        }
    }

    var canExport: Bool { savedId != nil ? saved != nil : live != nil }

    func exportReport() {
        if let id = savedId {
            hub.exportReport(type: nil, filter: nil, reportId: id, title: saved?.title ?? "Saved report")
        } else {
            hub.exportReport(type: type, filter: trimmedFilter.isEmpty ? nil : trimmedFilter, reportId: nil, title: title)
        }
    }
}

@MainActor
@Observable
final class CSDailyModel {
    unowned let hub: CallSheetsHub
    private(set) var view: ScheduleDailyView?
    private(set) var failure: CoreFailure?
    private(set) var loaded = false
    /// nil shows the core's default day (today, or the first).
    private(set) var dayId: String?

    init(hub: CallSheetsHub) { self.hub = hub }

    func pick(_ id: String) { dayId = id }

    func reload() async {
        guard let core = hub.core else { return }
        do {
            view = try await core.scheduleDaily(ScheduleDailyArgs(dayId: dayId))
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error)
        }
        loaded = true
    }

    var day: ScheduleDailyDay? { view?.day }
    var openIssues: [ScheduleWarningDto] { day?.warnings.filter { $0.decision == nil } ?? [] }

    /// "Create Call Sheet" or "Open Call Sheet" for the shown day.
    func openCallSheet() async {
        guard let d = day else { return }
        await hub.openForDay(d.id, existing: d.callSheet?.id)
    }

    func callSheetNote(_ b: CallSheetBrief) -> String {
        if b.sourceChanged { return "Available. The schedule changed after it was created." }
        return CSStatus.word(b.status) == "Final" ? "Finalized and matches the schedule." : "Available and up to date with the schedule."
    }
}

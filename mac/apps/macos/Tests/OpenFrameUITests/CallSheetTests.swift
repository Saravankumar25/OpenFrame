// Mac phase M6, Call Sheets workspace. The shared end-to-end scenario 62-call-sheets-exports.md,
// run at model level against a real core on a temporary folder: each step drives the Call Sheets
// models the way the screen does (the same calls, in the same order) and checks what the screen
// would show. Steps the scenario marks "Through the core" call the core directly. Plus edge and
// failure cases (a missing crew call, a stale sheet, a final sheet, a day that already has a
// sheet, a refused export, a project with no schedule) and rendered snapshots of the screens.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Rig

@MainActor
struct CSRig {
    let root: URL
    let out: URL
    let app: AppModel
    let hub: CallSheetsHub
    let projectPath: String
    var scheduleId = ""
    var dayId = ""
    var day2Id = ""
    var scenes: [ScheduleStripDto] = []

    /// The footer line of every production PDF (report.rs BRAND_LINE).
    static let brand = "Created using OpenFrame"

    static func make(title: String = "Lost Property Unit") async throws -> CSRig {
        let root = try Fixture.root()
        let out = root.appendingPathComponent("exports", isDirectory: true)
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        #expect(ok)
        guard case let .project(p) = app.phase else { throw CoreFailure(code: "test", message: "no project") }
        return CSRig(root: root, out: out, app: app, hub: CallSheetsHub(app: app), projectPath: p.path)
    }

    var core: Core { app.core! }
    var quiet: Bool { app.alert == nil }
    var editor: CallSheetEditorModel { hub.editor! }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    func path(_ file: String) -> String { out.appendingPathComponent(file).path }

    /// "Open a sample script that ships with the app": Example Short, preview, import.
    @discardableResult
    func importSample() async throws -> InterchangeImportPreview {
        let samples = try await core.screenplaySamples()
        let sample = try #require(samples.first { $0.title.contains("Example Short") })
        let preview = try await core.screenplayImportPreview(InterchangeImportPreviewArgs(sample: sample.id))
        _ = try await core.screenplayImportApply(InterchangeImportApplyArgs(
            previewId: preview.previewId, mode: "new_screenplay", keepSourceFile: false))
        return preview
    }

    /// "Plan two shooting days from the Production Source", as in the Windows spec.
    mutating func plan() async throws {
        let sources = try await core.interchangeSources()
        let draft = try #require(sources.screenplays.first?.drafts.first).id
        _ = try await core.productionSetSource(ProductionSetSourceArgs(draftId: draft, reason: nil))
        let sched = try await core.scheduleCreate(ScheduleCreateArgs())
        scheduleId = sched.id
        dayId = try await core.scheduleCreateDay(ScheduleShootDayCreateArgs(scheduleId: sched.id, date: "2027-06-14", offDay: false))
        day2Id = try await core.scheduleCreateDay(ScheduleShootDayCreateArgs(scheduleId: sched.id, date: "2027-06-15", offDay: false))
        let view = try await core.scheduleGet()
        scenes = Array(view.unscheduled.prefix(3))
        for s in scenes.prefix(2) {
            _ = try await core.scheduleMoveStrip(ScheduleStripMoveArgs(stripId: s.id, dayId: dayId, keepAnyway: false))
        }
        // Break the two scenes down (suggest, then accept) so their cast reaches the call sheet.
        for s in scenes.prefix(2) {
            _ = try await core.breakdownSuggest(BreakdownSceneArgs(sceneId: s.sceneId))
            let detail = try await core.breakdownScene(BreakdownSceneArgs(sceneId: s.sceneId))
            let ids = detail.groups.flatMap(\.suggestions).map(\.id)
            if !ids.isEmpty { _ = try await core.breakdownAcceptMany(BreakdownIdsArgs(ids: ids)) }
        }
        let dood = try await core.scheduleDood(ScheduleDoodArgs())
        #expect(!dood.rows.isEmpty, "the planned scenes have no cast")
        _ = try await core.scheduleAddMarker(ScheduleMarkerAddArgs(dayId: dayId, markerType: "Meal", durationMinutes: 45, index: 1))
        try await setTimes(crew: 420)
    }

    func setTimes(crew: Double?) async throws {
        _ = try await core.scheduleSetDayTimes(ScheduleDayTimesArgs(dayId: dayId, crewCall: crew, shootingStart: 480, targetWrap: 1140))
    }

    /// "Click Call Sheets, Create Call Sheet, then Create Call Sheet in the dialog."
    @discardableResult
    func createSheet() async throws -> CallSheetEditorModel {
        await hub.list.reload()
        hub.list.beginCreate()
        let create = try #require(hub.list.create)
        #expect(create.hasSchedule)
        #expect(create.canCreate)
        #expect(await create.confirm())
        hub.list.create = nil
        let e = try #require(hub.editor)
        await e.reload()
        return e
    }

    /// The file is a PDF, with the brand line or (Sides, owner decision 2026-10-27) without it.
    func checkPDF(_ file: String, brand: Bool = true) throws {
        let p = path(file)
        #expect(FileManager.default.fileExists(atPath: p), "\(file) was not written")
        let data = try Data(contentsOf: URL(fileURLWithPath: p))
        #expect(String(decoding: data.prefix(5), as: UTF8.self) == "%PDF-", "\(file) is not a PDF")
        let has = String(decoding: data, as: UTF8.self).contains(Self.brand) || String(data: data, encoding: .isoLatin1)?.contains(Self.brand) == true
        #expect(has == brand, "\(file) \(brand ? "has no" : "carries the") OpenFrame brand line")
    }

    /// Drives an export sheet the way the Save panel does: the path is given, the core writes.
    @discardableResult
    func runExport(_ model: CSExportModel?, to file: String) async throws -> ExportResult {
        let m = try #require(model)
        #expect(await m.export(to: path(file)), "export failed: \(m.error ?? "")")
        return try #require(m.result)
    }

    func relaunch() async throws -> CSRig {
        await app.closeProject()
        app.shutdown()
        let app2 = Fixture.model(root)
        await app2.start()
        await app2.openProject(path: projectPath)
        #expect(app2.alert == nil, "a recovery question showed")
        var rig = CSRig(root: root, out: out, app: app2, hub: CallSheetsHub(app: app2), projectPath: projectPath)
        rig.scheduleId = scheduleId
        rig.dayId = dayId
        rig.day2Id = day2Id
        rig.scenes = scenes
        return rig
    }
}

// MARK: Scenario 62

@Suite("Call sheets scenario", .serialized)
@MainActor
struct CallSheetScenarioTests {
    @Test func theWholeCallSheetsAndExportsScenario() async throws {
        var r = try await CSRig.make()
        defer { r.close() }

        // Open a sample script that ships with the app: ten scenes, then the project has them.
        let preview = try await r.importSample()
        #expect(preview.sceneCount == 10)
        #expect(r.quiet)

        // Plan two shooting days from the Production Source.
        try await r.plan()
        let planned = try await r.core.scheduleGet()
        #expect(planned.days.first?.label.contains("Shoot Day 1") == true, "the schedule shows: Shoot Day 1")
        #expect(r.quiet)

        // Create a call sheet: six default sections, Needs input, Add Section.
        let editor = try await r.createSheet()
        let d = try #require(editor.detail)
        let sections = d.defaultSections.map { $0.uppercased() }
        for s in ["DAY AND TIMES", "PRODUCTION", "LOCATIONS", "SCENE SCHEDULE", "TALENT / CAST", "PRACTICAL / SAFETY INFORMATION"] {
            #expect(sections.contains(s), "default section \(s) is missing")
        }
        #expect(!editor.isShown("weather"), "an optional section shows before it is added")
        #expect(d.optionalSections.contains { $0.label == "Weather" })
        #expect(!d.missing.isEmpty, "no value reads Needs input")
        #expect(editor.doc?.crewCall == "07:00", "the crew call does not come from the day")
        let weather = try #require(d.optionalSections.first { $0.label == "Weather" })
        await editor.addSection(key: weather.key, label: weather.label)
        #expect(editor.isShown("weather"), "Weather was not added")
        editor.edit { $0.optional.weather = "Light rain after 18:00" }
        #expect(await editor.flush())
        await r.hub.list.reload()
        #expect(r.hub.list.rows.count == 1, "the project lists one call sheet")
        #expect(try await r.core.callsheetsGet(CallSheetIdArgs(id: editor.id)).document.optional.weather == "Light rain after 18:00")
        #expect(r.quiet)

        // A schedule change marks the sheet Needs refresh, and the preview lists what changed.
        let view = try await r.core.scheduleGet()
        let third = try #require(view.unscheduled.first)
        _ = try await r.core.scheduleMoveStrip(ScheduleStripMoveArgs(stripId: third.id, dayId: r.dayId, keepAnyway: false))
        await editor.reload()
        #expect(editor.word == "Needs refresh")
        #expect(editor.detail?.stale?.areas.isEmpty == false)
        await editor.showRefresh()
        #expect(editor.dialog == .refresh)
        await editor.loadPreview()
        let p = try #require(editor.preview)
        #expect(p.hasChanges && !p.createsRevision)
        #expect(p.summary.joined(separator: " ").lowercased().contains("scene"), "the dialog does not mention a scene")
        #expect(await editor.applyRefresh() == nil)
        #expect(editor.dialog == nil)
        #expect(editor.word != "Needs refresh", "the workspace still shows Needs refresh")
        #expect(editor.doc?.optional.weather == "Light rain after 18:00", "the refresh lost a typed value")
        #expect(r.quiet)

        // Finalize freezes a snapshot; a new revision follows the schedule.
        await editor.showFinalize()
        #expect(editor.dialog == .finalize)
        #expect(await editor.finalize(acknowledgeStale: false) == .done)
        #expect(editor.word == "Final")
        #expect(editor.detail?.finalizedAt != nil, "the workspace shows: Finalized and a date")
        #expect(!editor.editable, "the Crew call field is not disabled")
        let id1 = editor.id
        r.hub.exportCallSheet(id: id1, title: editor.detail?.title ?? "", status: "Final")
        try await r.runExport(r.hub.exportModel, to: "call-sheet.pdf")
        try r.checkPDF("call-sheet.pdf")
        r.hub.exportModel = nil
        try await r.setTimes(crew: 450)
        await editor.reload()
        #expect(editor.detail?.stale != nil && !editor.editable, "the workspace shows: Start New Revision")
        #expect(editor.doc?.crewCall == "07:00", "the final sheet changed with the schedule")
        await editor.showRefresh()
        await editor.loadPreview()
        #expect(editor.preview?.createsRevision == true, "the dialog offers Create New Revision")
        let newId = try #require(await editor.applyRefresh())
        #expect(r.hub.openId == newId, "the new revision is open")
        let e2 = try #require(r.hub.editor)
        await e2.reload()
        #expect(Int(e2.detail?.revision ?? 0) == 2, "the workspace shows: v2")
        #expect(e2.doc?.crewCall == "07:30")
        #expect(e2.detail?.revisions.count == 2)
        #expect(r.quiet)

        // The budget keeps Planned and Actual with the difference.
        let budget = try await r.core.budgetCreate(BudgetCreateArgs(currency: "INR"))
        let category = try #require(try await r.core.budgetGet().categories.first)
        _ = try await r.core.budgetAddLine(BudgetLineAddArgs(budgetId: budget, category: category, description: "Lead actor", planned: 50000, actual: 55000))
        let withLine = try #require(try await r.core.budgetGet().current)
        #expect(withLine.difference == 5000 || withLine.difference == -5000, "the workspace shows: over plan")
        #expect(withLine.lines.count == 1)
        await r.app.undo()
        let afterUndo = try #require(try await r.core.budgetGet().current)
        #expect(afterUndo.lines.isEmpty, "the workspace shows: No lines yet")
        #expect(r.quiet)

        // A call sheet from the Indie Call Sheet template is a copy of its own.
        let listing = try await r.core.templatesList(TemplateListArgs())
        let names = listing.builtin.map(\.name)
        for n in ["Feature Film Screenplay", "Short Film Screenplay", "Episodic / Series", "Simple Beat Board", "Indie Call Sheet", "Shot List", "Moodboard", "Lightweight Budget"] {
            #expect(names.contains(n), "built-in template \(n) is missing")
        }
        let day2 = try #require(try await r.core.scheduleGet().days.first { $0.callSheet == nil && !$0.isOffDay })
        #expect(day2.id == r.day2Id)
        _ = try await r.core.templatesApply(ApplyTemplateArgs(scope: .builtin, id: "builtin:call_sheet", dayId: day2.id))
        let sheets = try await r.core.callsheetsList()
        let made = try await r.core.callsheetsGet(CallSheetIdArgs(id: try #require(sheets.first { $0.dayId == day2.id }).id))
        #expect(made.document.optional.weather != nil, "the template's Weather section is missing")
        #expect(made.document.optional.crew != nil, "the template's Crew section is missing")
        #expect(r.quiet)

        // Sides are unbranded; a report and the visual planning exports carry the brand line.
        await r.hub.sides.reload()
        r.hub.sides.pickDay(r.dayId)
        r.hub.sides.cover = true
        #expect(r.hub.sides.ready)
        r.hub.sides.exportPDF()
        try await r.runExport(r.hub.exportModel, to: "sides.pdf")
        try r.checkPDF("sides.pdf", brand: false)
        r.hub.reports.choose("scene")
        r.hub.reports.exportReport()
        let rep = try await r.runExport(r.hub.exportModel, to: "scene-report.pdf")
        #expect(rep.format == "pdf")
        try r.checkPDF("scene-report.pdf")
        _ = try await r.core.scheduleExportSchedule(ExportScheduleArgs(format: "pdf", path: r.path("schedule.pdf")))
        try r.checkPDF("schedule.pdf")
        _ = try await r.core.scheduleExportDood(ExportDoodArgs(format: "pdf", path: r.path("dood.pdf")))
        try r.checkPDF("dood.pdf")
        _ = try await r.core.breakdownExportReport(ExportBreakdownArgs(format: "pdf", path: r.path("breakdown.pdf")))
        try r.checkPDF("breakdown.pdf")
        let vscenes = try await r.core.visualScenes()
        let sceneId = try #require(vscenes.scenes.first).sceneId
        _ = try await r.core.templatesApply(ApplyTemplateArgs(scope: .builtin, id: "builtin:shot_list", sceneId: sceneId))
        _ = try await r.core.shotExportList(ExportShotListArgs(format: "pdf", path: r.path("shots.pdf")))
        try r.checkPDF("shots.pdf")
        let board = try await r.core.moodboardCreate(CreateMoodboardArgs(name: "Look"))
        _ = try await r.core.moodboardAddNote(AddMoodboardNoteArgs(moodboardId: board.id, text: "Wet asphalt at night"))
        _ = try await r.core.moodboardExportPdf(ExportMoodboardArgs(moodboardId: board.id, format: "pdf", path: r.path("moodboard.pdf")))
        try r.checkPDF("moodboard.pdf")
        let sb = try await r.core.storyboardCreate(CreateStoryboardArgs(name: "Opening", sceneId: sceneId))
        _ = try await r.core.storyboardAddPanel(AddPanelArgs(storyboardId: sb.id, visual: "placeholder", description: "Rain on the platform roof"))
        _ = try await r.core.storyboardExportSheet(ExportStoryboardSheetArgs(storyboardIds: [sb.id], format: "pdf", path: r.path("storyboard.pdf")))
        try r.checkPDF("storyboard.pdf")
        let sources = try await r.core.interchangeSources()
        let draftId = try #require(sources.screenplays.first?.drafts.first).id
        _ = try await r.core.screenplayExport(InterchangeScreenplayExportArgs(draftId: draftId, format: "pdf", path: r.path("script.pdf")))
        let script = try Data(contentsOf: URL(fileURLWithPath: r.path("script.pdf")))
        #expect(String(data: script, encoding: .isoLatin1)?.contains(CSRig.brand) != true, "the screenplay PDF is branded")
        #expect(r.quiet)

        // Notes and tasks for the shoot.
        _ = try await r.core.notesCreate(CreateNoteArgs(title: "Rain cover", body: "Keep the tarps on the truck.", pinned: false))
        _ = try await r.core.tasksCreate(CreateTaskArgs(title: "Confirm the railway permit"))
        #expect(try await r.core.notesList(NoteListArgs()).contains { $0.title == "Rain cover" })
        #expect(try await r.core.tasksList(TaskListArgs()).contains { $0.title == "Confirm the railway permit" })
        #expect(r.quiet)

        // Quit, relaunch and find everything again.
        r = try await r.relaunch()
        let all = try await r.core.callsheetsList()
        #expect(all.contains { Int($0.revision) == 2 }, "the second revision is gone")
        #expect(all.contains { $0.status == "Final" }, "the final revision is gone")
        await r.hub.list.reload()
        #expect(r.hub.list.rows.contains { $0.dayLabel.contains("Shoot Day 1") }, "the workspace shows: Shoot Day 1")
        #expect(r.quiet)
    }
}

// MARK: Edge and failure cases

@Suite("Call sheets edge cases", .serialized)
@MainActor
struct CallSheetEdgeTests {
    private func planned() async throws -> CSRig {
        var r = try await CSRig.make()
        try await r.importSample()
        try await r.plan()
        return r
    }

    @Test func editsAreSavedAfterTheDelayAndOnFlush() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        e.edit { $0.header.producer = "Asha Rao" }
        #expect(e.hasUnsavedEdits)
        // The save goes out about half a second after the last edit, without any other action.
        try await Task.sleep(for: .milliseconds(1500))
        #expect(!e.hasUnsavedEdits)
        #expect(try await r.core.callsheetsGet(CallSheetIdArgs(id: e.id)).document.header.producer == "Asha Rao")
        e.edit { $0.header.director = "Imran" }
        e.edit { $0.header.director = "Imran Sheikh" }
        #expect(await e.flush())
        #expect(try await r.core.callsheetsGet(CallSheetIdArgs(id: e.id)).document.header.director == "Imran Sheikh")
        // A reload while an edit waits keeps the typed value.
        e.edit { $0.header.company = "Copper Films" }
        await e.reload()
        #expect(e.doc?.header.company == "Copper Films")
        #expect(await e.flush())
        #expect(r.quiet)
    }

    @Test func locationFieldsNobodyEnteredReadNeedsInput() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        let doc = try #require(e.doc)
        if let loc = doc.locations.first {
            #expect(loc.address.isEmpty && loc.hospital.isEmpty)
        }
        #expect(CSEdit.prompt(text: "", need: true, placeholder: nil) == "Needs input")
        #expect(CSEdit.prompt(text: "  ", need: true, placeholder: "x") == "Needs input")
        #expect(CSEdit.prompt(text: "Station Road", need: true, placeholder: nil) == "")
        #expect(CSEdit.prompt(text: "", need: false, placeholder: "SW, W, WF") == "SW, W, WF")
        #expect(e.detail?.missing.isEmpty == false)
        // Typing a value removes it from the list of missing values.
        let before = e.detail?.missing.count ?? 0
        e.edit { d in
            for i in d.locations.indices { d.locations[i].address = "Station Road"; d.locations[i].hospital = "Town Hospital" }
        }
        #expect(await e.flush())
        await e.reload()
        #expect((e.detail?.missing.count ?? before) < before)
    }

    @Test func theSceneScheduleHasStartTimesMealsAndEstimates() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        let doc = try #require(e.doc)
        let real = doc.scenes.filter { $0.kind.isEmpty }
        let rows = doc.scenes.filter { !$0.kind.isEmpty }
        #expect(real.count == 2)
        #expect(rows.contains { $0.kind == "Meal" && $0.minutes == 45 }, "the meal break row is missing")
        #expect(e.detail?.estimatedStarts.count == doc.scenes.count)
        e.edit { $0.scenes[0].start = "08:15" }
        #expect(await e.flush())
        #expect(try await r.core.callsheetsGet(CallSheetIdArgs(id: e.id)).document.scenes[0].start == "08:15")
    }

    @Test func finalizeWithoutACrewCallMarksTheField() async throws {
        var r = try await CSRig.make()
        defer { r.close() }
        try await r.importSample()
        try await r.plan()
        try await r.setTimes(crew: nil)
        let e = try await r.createSheet()
        #expect(e.doc?.crewCall == "")
        await e.showFinalize()
        let outcome = await e.finalize(acknowledgeStale: false)
        #expect(outcome == .missingCrewCall)
        #expect(e.crewInvalid)
        #expect(e.focusCrewToken == 1)
        #expect(e.dialog == nil)
        #expect(e.word != "Final")
        #expect(e.editable)
        // Typing the crew call clears the mark and finalizing then works.
        e.crewInvalid = false
        e.edit { $0.crewCall = "07:30" }
        #expect(await e.finalize(acknowledgeStale: false) == .done)
        #expect(e.word == "Final")
    }

    @Test func finalizeAfterAScheduleChangeAsksAndCanGoAheadAnyway() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        let view = try await r.core.scheduleGet()
        let next = try #require(view.unscheduled.first)
        _ = try await r.core.scheduleMoveStrip(ScheduleStripMoveArgs(stripId: next.id, dayId: r.dayId, keepAnyway: false))
        await e.reload()
        #expect(e.word == "Needs refresh")
        let first = await e.finalize(acknowledgeStale: false)
        #expect(first == .stale, "the dialog does not offer Finalize this version anyway")
        #expect(r.quiet, "a stale sheet is a question, not an error")
        #expect(await e.finalize(acknowledgeStale: true) == .done)
        #expect(e.word == "Final")
    }

    @Test func aFinalSheetCannotBeEditedAndKeepsItsValues() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        #expect(await e.finalize(acknowledgeStale: false) == .done)
        #expect(!e.editable)
        let before = e.doc
        e.edit { $0.crewCall = "09:99" }
        #expect(e.doc == before, "a final sheet took an edit")
        #expect(!e.hasUnsavedEdits)
        // The core refuses a direct update too.
        let doc = try #require(before)
        var changed = doc
        changed.crewCall = "10:00"
        await #expect(throws: (any Error).self) {
            _ = try await r.core.callsheetsUpdate(CallSheetUpdateArgs(id: e.id, document: changed))
        }
        // No new revision exists until the schedule moves.
        await e.showRefresh()
        await e.loadPreview()
        #expect(e.preview?.createsRevision == true)
        #expect(e.preview?.hasChanges == false)
    }

    @Test func aDayThatAlreadyHasASheetOpensIt() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        await r.hub.closeEditor()
        #expect(r.hub.editor == nil)
        await r.hub.openForDay(r.dayId)
        #expect(r.hub.openId == e.id, "the existing call sheet did not open")
        await r.hub.list.reload()
        #expect(r.hub.list.rows.count == 1)
        #expect(r.quiet)
        // The Create sheet no longer offers that day.
        r.hub.list.beginCreate()
        let create = try #require(r.hub.list.create)
        #expect(!create.candidates.contains { $0.id == r.dayId })
        #expect(create.candidates.contains { $0.id == r.day2Id })
    }

    @Test func createFromATemplateAddsItsSections() async throws {
        let r = try await planned()
        defer { r.close() }
        await r.hub.list.reload()
        r.hub.list.beginCreate()
        let create = try #require(r.hub.list.create)
        await create.loadTemplates()
        let indie = try #require(create.templates.first { $0.name == "Indie Call Sheet" })
        create.templateKey = CreateCallSheetModel.key(indie)
        create.dayId = r.day2Id
        #expect(await create.confirm())
        let e = try #require(r.hub.editor)
        await e.reload()
        #expect(e.doc?.optional.weather != nil)
        #expect(e.doc?.optional.crew != nil)
        #expect(e.doc?.optional.communication != nil)
        #expect(r.quiet)
    }

    @Test func addedSectionsCanBeRemovedAndEditedAndCustomFieldsAdded() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        let defs = try #require(e.detail?.optionalSections)
        for key in ["weather", "extras", "departments", "advance", "crew", "communication", "specialNotes"] {
            let def = try #require(defs.first { $0.key == key }, "\(key) is not offered")
            await e.addSection(key: def.key, label: def.label)
            #expect(e.isShown(key), "\(key) was not added")
        }
        e.edit { $0.optional.crew?.append(CallSheetCrewMember(key: "", name: "Imran", role: "DOP", department: "Camera", callTime: "06:30", contact: "")) }
        e.edit { $0.optional.departments?.append(CallSheetDepartmentNote(department: "Art", notes: "Wet down the street")) }
        e.edit { $0.optional.extras?.append(CallSheetExtra(key: "", description: "Passers-by", quantity: "6", callTime: "", onSet: "", notes: "")) }
        e.addCustomField()
        e.edit { $0.extraFields[0].label = "Parking permit"; $0.extraFields[0].value = "Pink slip" }
        #expect(await e.flush())
        let saved = try await r.core.callsheetsGet(CallSheetIdArgs(id: e.id)).document
        #expect(saved.optional.crew?.contains { $0.name == "Imran" } == true)
        #expect(saved.optional.departments?.contains { $0.department == "Art" } == true)
        #expect(saved.optional.extras?.contains { $0.description == "Passers-by" } == true)
        #expect(saved.extraFields.first?.label == "Parking permit")
        e.edit { $0.optional.weather = nil }
        #expect(!e.isShown("weather"))
        #expect(await e.flush())
        #expect(try await r.core.callsheetsGet(CallSheetIdArgs(id: e.id)).document.optional.weather == nil)
        #expect(r.quiet)
    }

    @Test func aNotePathAttachesAFileToTheSheet() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        let def = try #require(e.detail?.optionalSections.first { $0.key == "attachments" })
        await e.addSection(key: def.key, label: def.label)
        let file = r.out.appendingPathComponent("permit.txt")
        try "Railway permit".write(to: file, atomically: true, encoding: .utf8)
        await e.attach(section: "attachments", path: file.path)
        await e.reload()
        #expect(e.doc?.optional.attachments?.first?.name == "permit.txt")
        // A file that is not there is refused in plain words and the sheet is unchanged.
        let count = e.doc?.optional.attachments?.count
        await e.attach(section: "attachments", path: r.path("missing.pdf"))
        #expect(e.doc?.optional.attachments?.count == count)
        #expect(r.app.alert != nil, "the refusal was swallowed")
    }

    @Test func theListFiltersSearchesShowsReplacedAndDeletesWithUndo() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        _ = try await r.core.callsheetsCreate(CallSheetCreateArgs(dayId: r.day2Id))
        await r.hub.closeEditor()
        let list: CallSheetListModel = r.hub.list
        await list.reload()
        #expect(list.rows.count == 2)
        list.filter = .final
        #expect(list.filtered.isEmpty)
        list.filter = .draft
        #expect(list.filtered.count == 2)
        list.filter = .all
        list.text = "Day 2"
        #expect(list.filtered.count == 1 && list.filtered[0].dayLabel.contains("2"))
        list.text = "no such day"
        #expect(list.filtered.isEmpty)
        list.text = ""
        // Finalize day 1, move the schedule, start a new revision: the old one is replaced.
        await r.hub.open(e.id)
        let ed = r.editor
        await ed.reload()
        #expect(await ed.finalize(acknowledgeStale: false) == .done)
        try await r.setTimes(crew: 450)
        await ed.reload()
        await ed.showRefresh()
        await ed.loadPreview()
        _ = await ed.applyRefresh()
        await r.hub.closeEditor()
        await list.reload()
        #expect(list.rows.count == 3)
        #expect(list.filtered.count == 2, "the replaced revision shows without the switch")
        list.showOld = true
        #expect(list.filtered.count == 3)
        list.filter = .final
        #expect(list.filtered.count == 1 && list.filtered[0].replaced)
        list.filter = .all
        list.showOld = false
        // Delete day 2's sheet, then undo.
        let day2Row = try #require(list.rows.first { $0.dayId == r.day2Id })
        await list.delete(day2Row)
        #expect(!list.rows.contains { $0.id == day2Row.id })
        await r.app.undo()
        await list.reload()
        #expect(list.rows.contains { $0.id == day2Row.id }, "undo did not bring the call sheet back")
        #expect(r.quiet)
    }

    @Test func aProjectWithoutAScheduleSaysWhatToDoFirst() async throws {
        let r = try await CSRig.make(title: "Empty Film")
        defer { r.close() }
        await r.hub.list.reload()
        #expect(r.hub.list.isEmpty)
        r.hub.list.beginCreate()
        let create = try #require(r.hub.list.create)
        #expect(!create.hasSchedule && create.candidates.isEmpty && !create.canCreate)
        #expect(await !create.confirm())
        await r.hub.sides.reload()
        #expect(r.hub.sides.loaded && !r.hub.sides.hasSchedule)
        await r.hub.daily.reload()
        #expect(r.hub.daily.loaded)
        #expect(r.hub.daily.view?.hasSchedule != true)
        #expect(r.hub.daily.day == nil)
        #expect(r.quiet)
    }

    @Test func aMissingCallSheetShowsAPlainMessage() async throws {
        let r = try await planned()
        defer { r.close() }
        r.hub.open("does-not-exist")
        let e = try #require(r.hub.editor)
        await e.reload()
        #expect(e.loaded && e.detail == nil)
        #expect(e.failure?.message.isEmpty == false)
        #expect(!(e.failure?.message.contains("internal") ?? false))
        await r.hub.closeEditor()
        #expect(r.hub.editor == nil)
    }

    @Test func deletingTheOpenSheetGoesBackToTheList() async throws {
        let r = try await planned()
        defer { r.close() }
        let e = try await r.createSheet()
        e.edit { $0.header.producer = "Typed but unsaved" }
        #expect(await e.delete())
        #expect(r.hub.editor == nil && r.hub.openId == nil)
        await r.hub.list.reload()
        #expect(r.hub.list.isEmpty)
        await r.app.undo()
        await r.hub.list.reload()
        #expect(r.hub.list.rows.count == 1)
    }

    @Test func routeRequestsOpenTheRightThing() async throws {
        let r = try await planned()
        defer { r.close() }
        CallSheetsRoute.shared.request(.day(r.dayId))
        await r.hub.consumeRoute()
        let first = try #require(r.hub.editor)
        #expect(CallSheetsRoute.shared.pending == nil)
        // The same day again opens the call sheet it has, and creates no second one.
        await r.hub.closeEditor()
        CallSheetsRoute.shared.request(.day(r.dayId))
        await r.hub.consumeRoute()
        #expect(r.hub.openId == first.id)
        #expect(try await r.core.callsheetsList().count == 1)
        CallSheetsRoute.shared.request(.sides(day: r.day2Id))
        await r.hub.consumeRoute()
        #expect(r.hub.tab == .sides)
        await r.hub.sides.reload()
        #expect(r.hub.sides.dayId == r.day2Id)
        CallSheetsRoute.shared.request(.daily(day: r.dayId))
        await r.hub.consumeRoute()
        #expect(r.hub.tab == .daily)
        await r.hub.daily.reload()
        #expect(r.hub.daily.day?.id == r.dayId)
        CallSheetsRoute.shared.request(.sheet(first.id))
        await r.hub.consumeRoute()
        #expect(r.hub.tab == .callSheets && r.hub.openId == first.id)
    }
}

// MARK: Exports

@Suite("Call sheet exports", .serialized)
@MainActor
struct CallSheetExportTests {
    @Test func exportOptionsFormatsAndTheResultAreReported() async throws {
        var r = try await CSRig.make()
        defer { r.close() }
        try await r.importSample()
        try await r.plan()
        let e = try await r.createSheet()
        let weather = try #require(e.detail?.optionalSections.first { $0.key == "weather" })
        await e.addSection(key: weather.key, label: weather.label)

        r.hub.exportCallSheet(id: e.id, title: "Call Sheet: Day 1", status: "Draft")
        let m = try #require(r.hub.exportModel)
        #expect(m.formats.map(\.value) == ["pdf"])
        #expect(m.options.map(\.key) == ["includeOptional"] && m.values["includeOptional"] == true)
        #expect(m.note.contains("Finalize the call sheet before sending it"))
        #expect(m.suggestedName.hasSuffix(".pdf") && m.suggestedName.contains("Lost Property Unit") && m.suggestedName.contains("Call Sheet"))
        #expect(!m.suggestedName.contains(":"), "a colon is not allowed in a file name")
        let result = try await r.runExport(m, to: "draft.pdf")
        #expect(result.fileName == "draft.pdf" && result.format == "pdf")
        #expect(result.privateNotesExcluded)
        try r.checkPDF("draft.pdf")
        // Without the added sections the file is still written.
        r.hub.exportCallSheet(id: e.id, title: "Call Sheet: Day 1", status: "Draft")
        let m2 = try #require(r.hub.exportModel)
        m2.values["includeOptional"] = false
        try await r.runExport(m2, to: "plain.pdf")
        try r.checkPDF("plain.pdf")
        // A final sheet says it is made from the frozen snapshot.
        r.hub.exportCallSheet(id: e.id, title: "Call Sheet: Day 1", status: "Final")
        #expect(r.hub.exportModel?.note.contains("frozen snapshot") == true)

        // Reports export as PDF, CSV and a spreadsheet.
        r.hub.reports.choose("location")
        r.hub.reports.exportReport()
        let rm = try #require(r.hub.exportModel)
        #expect(rm.formats.map(\.value) == ["pdf", "csv", "xlsx"])
        for f in ["pdf", "csv", "xlsx"] {
            rm.format = f
            let res = try await r.runExport(rm, to: "locations.\(f)")
            #expect(res.format == f)
            #expect(FileManager.default.fileExists(atPath: r.path("locations.\(f)")))
        }
        let csv = try String(contentsOfFile: r.path("locations.csv"), encoding: .utf8)
        #expect(!csv.isEmpty)
        #expect(r.quiet)
    }

    @Test func aRefusedExportKeepsTheSheetOpenWithPlainWords() async throws {
        var r = try await CSRig.make()
        defer { r.close() }
        try await r.importSample()
        try await r.plan()
        let e = try await r.createSheet()
        r.hub.exportCallSheet(id: e.id, title: "Call Sheet: Day 1", status: "Draft")
        let m = try #require(r.hub.exportModel)
        let ok = await m.export(to: r.root.appendingPathComponent("no-such-folder/x/sheet.pdf").path)
        #expect(!ok)
        #expect(m.result == nil)
        let message = try #require(m.error)
        #expect(!message.isEmpty && !message.contains("internal"))
        #expect(r.quiet, "the refusal opened an alert as well")
        // Trying again to a good place works on the same sheet.
        #expect(await m.export(to: r.path("retry.pdf")))
        #expect(m.error == nil)
        try r.checkPDF("retry.pdf")
        // A second export while one runs is ignored.
        #expect(!m.busy)
    }

    @Test func fileNamesAreSafeOnBothSystems() {
        let d = Date(timeIntervalSince1970: 1_800_000_000)
        let n = CSFormat.exportFileName(project: "Lost: Property/Unit", document: "Sides - Shoot Day 1 \u{2013} Mon", format: "pdf", date: d)
        #expect(n.hasSuffix(".pdf"))
        for bad in [":", "/", "\\", "?", "*", "\"", "<", ">", "|"] { #expect(!n.contains(bad), "\(bad) in \(n)") }
        #expect(CSFormat.exportFileName(project: nil, document: "", format: "csv", date: d).hasPrefix("20"))
        let long = CSFormat.exportFileName(project: String(repeating: "A", count: 400), document: "B", format: "xlsx", date: d)
        #expect(long.count < 170)
    }
}

// MARK: Sides, reports, daily

@Suite("Sides reports and daily view", .serialized)
@MainActor
struct SidesReportsDailyTests {
    private func planned() async throws -> CSRig {
        var r = try await CSRig.make()
        try await r.importSample()
        try await r.plan()
        return r
    }

    @Test func sidesForADayPreviewSaveAndStayASnapshot() async throws {
        let r = try await planned()
        defer { r.close() }
        let s: CSSidesModel = r.hub.sides
        await s.reload()
        #expect(s.hasSchedule && s.loaded)
        #expect(s.days.count == 2)
        #expect(s.dayId == r.day2Id || s.dayId == r.dayId)
        s.pickDay(r.dayId)
        #expect(s.strips.count == 2)
        #expect(s.chosen.count == 2 && s.ready)
        await s.loadPreview()
        let live = try #require(s.preview)
        #expect(live.scenes.count == 2)
        #expect(live.cover == nil)
        #expect(live.showDraftName)
        // One scene only.
        s.setScene(s.strips[1].id, on: false)
        await s.loadPreview()
        #expect(s.preview?.scenes.count == 1)
        // The cover page carries the day, crew call and cast.
        s.cover = true
        s.setScene(s.strips[1].id, on: true)
        await s.loadPreview()
        let withCover = try #require(s.preview?.cover)
        #expect(!withCover.cast.isEmpty || !withCover.locations.isEmpty, "the cover page is empty")
        s.draftName = false
        await s.loadPreview()
        #expect(s.preview?.showDraftName == false)
        // Save: a snapshot that opens and keeps its scenes.
        await s.save()
        #expect(s.saved.count == 1)
        let id = try #require(s.openId)
        await s.loadPreview()
        let snap = try #require(s.openSaved)
        #expect(snap.id == id && snap.content.scenes.count == 2)
        s.backToLive()
        #expect(s.openId == nil)
        await s.deleteSaved(id)
        #expect(s.saved.isEmpty)
        #expect(r.quiet)
    }

    @Test func sidesNeedAtLeastOneSceneAndOneCastMemberWhenForSelectedCast() async throws {
        let r = try await planned()
        defer { r.close() }
        let s: CSSidesModel = r.hub.sides
        await s.reload()
        s.pickDay(r.dayId)
        for strip in s.strips { s.setScene(strip.id, on: false) }
        #expect(s.chosen.isEmpty && !s.ready)
        await s.loadPreview()
        #expect(s.preview == nil)
        for strip in s.strips { s.setScene(strip.id, on: true) }
        #expect(s.ready)
        s.forMode = .selected
        #expect(!s.ready, "selected cast needs a cast member")
        let cast = try #require(s.dayCast.first)
        s.setCast(cast.key, on: true)
        #expect(s.ready)
        await s.loadPreview()
        #expect(s.preview != nil)
        s.pickDay(r.day2Id)
        #expect(s.strips.isEmpty && !s.ready)
        #expect(r.quiet)
    }

    @Test func sidesWithNoScenesOnTheDayDoNotCallTheCore() async throws {
        let r = try await planned()
        defer { r.close() }
        let s: CSSidesModel = r.hub.sides
        await s.reload()
        s.pickDay(r.day2Id)
        #expect(!s.ready)
        await s.loadPreview()
        #expect(s.previewFailure == nil && s.preview == nil)
        #expect(r.quiet)
    }

    @Test func allSixReportsGenerateFilterSaveAndDelete() async throws {
        let r = try await planned()
        defer { r.close() }
        let rep: CSReportsModel = r.hub.reports
        #expect(CSReportKind.all.map(\.value) == ["scene", "location", "cast_scene", "prop", "schedule", "breakdown_completeness"])
        #expect(CSReportKind.all.map(\.label) == ["Scene Report", "Location Report", "Cast Scene Report", "Prop Report", "Schedule Report", "Breakdown Completeness"])
        for kind in CSReportKind.all {
            rep.choose(kind.value)
            await rep.reload()
            let d = try #require(rep.live, "\(kind.label) did not generate: \(rep.liveFailure?.message ?? "")")
            #expect(d.reportType == kind.value)
            #expect(!d.title.isEmpty)
            #expect(rep.liveFailure == nil)
            for row in d.rows { #expect(row.count == d.columns.count, "\(kind.label) has a ragged row") }
        }
        rep.choose("scene")
        await rep.reload()
        let all = try #require(rep.live).rows.count
        #expect(all >= 10)
        // Filter: rows narrow, then clear.
        rep.filter = "INT."
        await rep.reload()
        let narrowed = try #require(rep.live).rows.count
        #expect(narrowed <= all)
        rep.filter = "zzzz-no-such-row"
        await rep.reload()
        #expect(rep.live?.rows.isEmpty == true)
        rep.filter = ""
        // Save a snapshot; it does not change when the schedule does.
        await rep.reload()
        await rep.saveSnapshot()
        #expect(rep.list.count == 1)
        let id = try #require(rep.savedId)
        await rep.reload()
        let saved = try #require(rep.saved)
        let rowsThen = saved.data.rows
        _ = try await r.core.scheduleMoveStrip(ScheduleStripMoveArgs(stripId: try #require(r.scenes.last).id, dayId: r.dayId, keepAnyway: false))
        await rep.reload()
        #expect(rep.saved?.data.rows == rowsThen)
        #expect(rep.canExport)
        rep.backToLive()
        #expect(rep.savedId == nil)
        await rep.deleteSnapshot(id)
        #expect(rep.list.isEmpty)
        #expect(r.quiet)
    }

    @Test func aSavedSnapshotThatIsGoneShowsAMessage() async throws {
        let r = try await planned()
        defer { r.close() }
        let rep: CSReportsModel = r.hub.reports
        rep.openSnapshot("not-a-report")
        await rep.reload()
        #expect(rep.saved == nil)
        #expect(rep.savedFailure?.message.isEmpty == false)
        #expect(!rep.canExport)
    }

    @Test func theDailyViewShowsOneDayAndOpensItsCallSheet() async throws {
        let r = try await planned()
        defer { r.close() }
        let m: CSDailyModel = r.hub.daily
        await m.reload()
        #expect(m.view?.hasSchedule == true)
        #expect(m.view?.days.count == 2)
        m.pick(r.dayId)
        await m.reload()
        let d = try #require(m.day)
        #expect(d.id == r.dayId && !d.isOffDay)
        #expect(d.scenes.count == 2)
        #expect(!d.cast.isEmpty)
        #expect(d.callSheet == nil)
        // "Create Call Sheet" on a day without one makes it and opens it.
        await m.openCallSheet()
        #expect(r.hub.tab == .callSheets && r.hub.openId != nil)
        await m.reload()
        let brief = try #require(m.day?.callSheet)
        #expect(brief.id == r.hub.openId)
        #expect(m.callSheetNote(brief) == "Available and up to date with the schedule.")
        // Now the same button opens the existing sheet.
        await r.hub.closeEditor()
        await m.openCallSheet()
        #expect(r.hub.openId == brief.id)
        #expect(try await r.core.callsheetsList().count == 1)
        // A stale sheet is said so.
        await r.hub.closeEditor()
        try await r.setTimes(crew: 450)
        await m.reload()
        let stale = try #require(m.day?.callSheet)
        #expect(stale.sourceChanged)
        #expect(m.callSheetNote(stale) == "Available. The schedule changed after it was created.")
        // The empty day.
        m.pick(r.day2Id)
        await m.reload()
        #expect(m.day?.scenes.isEmpty == true)
        #expect(r.quiet)
    }

    /// Export on the Daily View writes that day as its own document named "Shoot Day N - Daily View"
    /// (Windows export.test.tsx), as PDF or plain text, with no schedule scopes.
    @Test func theDailyViewExportsTheShownDayAsItsOwnDocument() async throws {
        let r = try await planned()
        defer { r.close() }
        let m: CSDailyModel = r.hub.daily
        m.pick(r.dayId)
        await m.reload()
        let d = try #require(m.day)
        r.hub.exportDaily(dayId: d.id, label: d.label)
        let e = try #require(r.hub.exportModel)
        #expect(e.title == "Export Daily View")
        #expect(e.documentName == "\(d.label) - Daily View")
        #expect(e.source == "Source: \(d.label) - Daily View")
        #expect(e.suggestedName == "\(d.label) - Daily View.pdf")
        #expect(e.formats.map(\.value) == ["pdf", "txt"])
        #expect(e.formats.map(\.label) == ["PDF", "Plain text"])
        #expect(e.options.isEmpty, "the Daily View has no scope choices")
        #expect(e.note == "Private notes excluded. The document shows what the Daily View shows; the schedule is not changed.")
        #expect(await e.export(to: r.path("daily.pdf")), "export failed: \(e.error ?? "")")
        #expect(e.result?.document == "\(d.label) - Daily View")
        try r.checkPDF("daily.pdf")
        e.format = "txt"
        #expect(e.suggestedName == "\(d.label) - Daily View.txt")
        #expect(await e.export(to: r.path("daily.txt")), "export failed: \(e.error ?? "")")
        let text = try String(contentsOfFile: r.path("daily.txt"), encoding: .utf8)
        #expect(text.contains(d.label), "the text file does not name the day")
    }

    @Test func statusWordsFormattingAndMarkers() {
        #expect(CSStatus.word("Final") == "Final")
        #expect(CSStatus.word("Needs refresh") == "Needs refresh")
        #expect(CSStatus.word("Whatever") == "Draft")
        #expect(CSFormat.minutes(330) == "5h 30m" && CSFormat.minutes(45) == "45m" && CSFormat.minutes(600) == "10h")
        #expect(CSFormat.number(4) == "4" && CSFormat.number(nil) == "Not numbered" && CSFormat.number(4.5) == "4.5")
        let m = ScheduleMarkerDto(id: "m", dayId: "d", markerType: "Meal", label: "Lunch", atTime: "12:30", durationMinutes: 45, notes: nil, rev: 1)
        #expect(CSFormat.marker(m) == "LUNCH at 12:30 (45m)")
        #expect(csSlug("Actor for ARJUN") == "actor-for-arjun")
        #expect(csSlug("1st AD") == "1st-ad")
    }
}

// MARK: Rendered screens

@Suite("Call sheet screens", .serialized)
@MainActor
struct CallSheetRenderTests {
    private func host<V: View>(_ view: V, app: AppModel, scheme: ColorScheme) -> some View {
        let themes = ThemeStore()
        return view
            .environment(app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
            .environment(\.colorScheme, scheme)
    }

    @Test func everyTabRendersInLightAndDark() async throws {
        var r = try await CSRig.make()
        defer { r.close() }
        try await r.importSample()
        try await r.plan()
        let e = try await r.createSheet()
        for key in ["weather", "crew", "extras", "departments", "advance", "communication", "specialNotes", "attachments", "referenceImages"] {
            if let def = e.detail?.optionalSections.first(where: { $0.key == key }) { await e.addSection(key: def.key, label: def.label) }
        }
        e.addCustomField()
        await r.hub.list.reload()
        await r.hub.sides.reload()
        await r.hub.sides.loadPreview()
        await r.hub.reports.reload()
        await r.hub.daily.reload()
        _ = try await r.core.callsheetsCreate(CallSheetCreateArgs(dayId: r.day2Id))
        await r.hub.list.reload()
        for scheme in [ColorScheme.light, .dark] {
            let n = scheme == .dark ? "dark" : "light"
            for tab in CSTab.allCases {
                r.hub.tab = tab
                let image = Fixture.render(host(CallSheetsContent(hub: r.hub), app: r.app, scheme: scheme),
                                           name: "callsheets-\(tab.rawValue)-\(n)", size: CGSize(width: 1280, height: 860))
                #expect(image != nil)
            }
        }
        // A list with no editor open.
        await r.hub.closeEditor()
        r.hub.tab = .callSheets
        #expect(Fixture.render(host(CallSheetsContent(hub: r.hub), app: r.app, scheme: .light), name: "callsheets-list", size: CGSize(width: 1100, height: 600)) != nil)
        // Sheets.
        r.hub.list.beginCreate()
        let create = try #require(r.hub.list.create)
        await create.loadTemplates()
        #expect(Fixture.render(host(CSCreateSheet(model: create) {}, app: r.app, scheme: .light), name: "callsheets-create", size: CGSize(width: 560, height: 520)) != nil)
        r.hub.open(e.id)
        let ed = r.editor
        await ed.reload()
        _ = await ed.finalize(acknowledgeStale: false)
        try await r.setTimes(crew: 450)
        await ed.reload()
        await ed.loadPreview()
        #expect(Fixture.render(host(CSRefreshSheet(model: ed), app: r.app, scheme: .light), name: "callsheets-refresh", size: CGSize(width: 720, height: 560)) != nil)
        r.hub.exportCallSheet(id: ed.id, title: "Call Sheet: Day 1", status: "Final")
        let em = try #require(r.hub.exportModel)
        #expect(Fixture.render(host(CSExportSheet(model: em) {}, app: r.app, scheme: .dark), name: "callsheets-export", size: CGSize(width: 560, height: 420)) != nil)
        try await r.runExport(em, to: "render.pdf")
        #expect(Fixture.render(host(CSExportSheet(model: em) {}, app: r.app, scheme: .light), name: "callsheets-export-result", size: CGSize(width: 700, height: 760)) != nil)
        #expect(r.quiet)
        r = try await r.relaunch()
    }
}

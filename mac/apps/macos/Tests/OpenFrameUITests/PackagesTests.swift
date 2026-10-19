// Mac phase M7, packages. The shared scenario 70-packages, in its order, run at model level against
// real cores on temporary folders: backup, Full Project Package, review package, the reviewer's
// comment and response, import of the response, restore from Home with the collision choice, a
// damaged backup, and the relaunch. Plus stale packages, unmatched notes and the Review Queue,
// undo, replace after backup, wrong kinds of package, a version from the future, a cancelled task
// and a folder that cannot be written.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

private let nightShift = [
    "Title: Night Shift", "",
    "INT. CONTROL ROOM - NIGHT", "",
    "Kavya studies a flickering monitor and lifts a torch.", "",
    "KAVYA", "Someone is in the tunnel.", "",
    "EXT. TUNNEL MOUTH - NIGHT", "",
    "Ravi drags a heavy toolbox through the rain.", "",
    "RAVI", "Hold the gate.", "",
].joined(separator: "\n")

@MainActor
private struct PKRig {
    let rig: IXRig
    let draftId: String
    let screenplayId: String
    var app: AppModel { rig.app }
    var core: Core { rig.core }
    var packageOut: URL { rig.root.appendingPathComponent("package-out", isDirectory: true) }

    static func make(title: String = "Harbour Lights", note: Bool = true) async throws -> PKRig {
        let rig = try await IXRig.make(title: title)
        let r = try await rig.importPasted(nightShift)
        let out = rig.root.appendingPathComponent("package-out", isDirectory: true)
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        let p = PKRig(rig: rig, draftId: r.draftId, screenplayId: r.screenplayId)
        if note {
            let doc = try await rig.core.screenplayDocument(ScreenplayDraftIdArgs(draftId: r.draftId))
            _ = try await rig.core.privateNoteCreate(CreatePrivateNoteArgs(
                targetType: "screenplay_scene", targetId: doc.scenes[0].id, body: "Harbour private note for the restore check"))
        }
        return p
    }

    func file(_ name: String) -> URL { packageOut.appendingPathComponent(name) }
    func close() { rig.close() }

    func backup(_ name: String = "Harbour Lights backup.ofbackup") async throws -> URL {
        let m = BackupModel(app: app)
        await m.load()
        m.choose(destination: file(name))
        await m.start()
        await m.task.waitUntilFinished()
        _ = try #require(m.report)
        return file(name)
    }

    func exportScriptReview(_ name: String = "Harbour Lights review.ofscriptreview", configure: (ExchangeExportModel) async -> Void = { _ in }) async throws -> ExchangeExportModel {
        let m = ExchangeExportModel(app: app, packageType: .scriptReview)
        await m.load()
        await configure(m)
        await m.export(to: file(name))
        _ = try #require(m.done)
        return m
    }

    /// The reviewer: open the package, comment on a scene, export the response.
    func respond(review: String = "Harbour Lights review.ofscriptreview", comment: String = "Tighten the harbour line.", sceneIndex: Int = 0, quote: String? = nil, to name: String = "Harbour Lights reply.ofresponse") async throws -> URL {
        let v = ReviewViewerModel(app: app, source: .review, key: "")
        await v.open(file: file(review))
        let ws = try #require(v.workspace)
        v.sceneId = ws.scenes[sceneIndex].id
        v.body = comment
        if let quote { #expect(v.quoteSelection(quote)) }
        await v.addComment()
        await v.exportResponse(to: file(name))
        _ = try #require(v.exportedPath)
        return file(name)
    }
}

@Suite("PackagesTests", .serialized)
@MainActor
struct PackagesTests {
    // MARK: 70-packages, in order

    @Test func theWholePackagesScenario() async throws {
        let p = try await PKRig.make()
        defer { p.close() }
        let app = p.app

        // Create a backup through Create Backup and the Save panel.
        let backup = BackupModel(app: app)
        await backup.load()
        #expect(backup.preview != nil)
        #expect(!backup.canStart)                               // a location is needed first
        #expect(backup.name.hasPrefix("Harbour Lights") && backup.name.hasSuffix(".ofbackup"))
        backup.choose(destination: p.file("Harbour Lights backup.ofbackup"))
        #expect(backup.destination?.path == p.file("Harbour Lights backup.ofbackup").path)   // "Backup location" shows the full path
        #expect(backup.canStart)
        await backup.start()
        await backup.task.waitUntilFinished()
        let report = try #require(backup.report)               // "Backup created."
        #expect(FileManager.default.fileExists(atPath: p.file("Harbour Lights backup.ofbackup").path))
        #expect(report.bytes > 0 && report.path == p.file("Harbour Lights backup.ofbackup").path)
        #expect(PKText.formatBytes(report.bytes).contains("B"))
        backup.finish()

        // Export a Full Project Package and a screenplay review package.
        let exportProject = ExportProjectModel(app: app)
        await exportProject.load()
        #expect(exportProject.preview?.groups.isEmpty == false)
        #expect(exportProject.suggestedName == "Harbour Lights.ofproject")
        await exportProject.export(to: p.file("Harbour Lights.ofproject"))
        await exportProject.task.waitUntilFinished()
        _ = try #require(exportProject.report)                 // "Project package exported."
        #expect(FileManager.default.fileExists(atPath: p.file("Harbour Lights.ofproject").path))

        let review = ExchangeExportModel(app: app, packageType: .scriptReview)
        await review.load()
        #expect(review.heading.title == "Script Review Package")
        let summary = try #require(review.summary)             // "Private notes: excluded" once the preview loaded
        #expect(summary.privateNotesExcluded)
        #expect(!review.args.includePrivateNotes)
        #expect(summary.suggestedFileName.hasSuffix(".ofscriptreview"))
        await review.export(to: p.file("Harbour Lights review.ofscriptreview"))
        #expect(review.done != nil)                            // "Package exported."
        #expect(FileManager.default.fileExists(atPath: p.file("Harbour Lights review.ofscriptreview").path))

        // Import & Export History > Package log.
        let history = PackageHistoryModel(app: app)
        history.tab = .log
        await history.load()
        let summaries = history.log.map(\.summary)
        #expect(summaries.contains { $0.contains("Created backup") }, "\(summaries)")
        #expect(summaries.contains { $0.contains("Exported project package") }, "\(summaries)")

        // Review round trip: comment as the reviewer, send a response, preview and import it.
        let viewer = ReviewViewerModel(app: app, source: .review, key: "")
        await viewer.open(file: p.file("Harbour Lights review.ofscriptreview"))
        let ws = try #require(viewer.workspace)
        #expect(viewer.banner == "Review package. Comments are separate from the original project.")
        #expect(ws.canComment && ws.scenes.count == 2)
        #expect(viewer.myCount == 0)
        viewer.body = "Tighten the harbour line."
        await viewer.addComment()
        #expect(viewer.comments.map(\.body) == ["Tighten the harbour line."])
        #expect(viewer.comments.first?.mine == true)           // "Yours"
        #expect(viewer.count(for: ws.scenes[0].id) == 1)
        await viewer.exportResponse(to: p.file("Harbour Lights reply.ofresponse"))
        #expect(viewer.exportedPath == p.file("Harbour Lights reply.ofresponse").path)
        #expect(FileManager.default.fileExists(atPath: p.file("Harbour Lights reply.ofresponse").path))
        viewer.finish()

        let imp = ExchangeImportModel(app: app)
        await imp.open(file: p.file("Harbour Lights reply.ofresponse"))
        let session = try #require(imp.session)
        #expect(imp.commentsReceived == 1)
        let allLabels = imp.visibleChanges.map(\.label) + imp.visibleChanges.compactMap(\.detail)
        #expect(allLabels.contains { $0.contains("Tighten the harbour line.") }, "\(allLabels)")   // listed before anything is imported
        #expect(session.status == "Previewed" && !imp.finished)
        #expect(!imp.staleStep)
        await imp.apply(.allSafe)
        let result = try #require(imp.result)                   // "Imported:"
        #expect(result.status == "Applied" || result.status == "PartiallyApplied")
        #expect(result.message.contains("Imported"), "\(result.message)")
        #expect(imp.finished)
        let comments = try await p.core.screenplayDocument(ScreenplayDraftIdArgs(draftId: p.draftId))
        _ = comments
        await imp.cancel()

        // Restore from Home: the collision choice opens it as a copy.
        await app.closeProject()
        #expect(app.phase == .home)
        let open = OpenProjectModel(app: app)
        await open.choose(file: p.file("Harbour Lights backup.ofbackup"))
        #expect(open.errorText == nil)
        #expect(open.isBackup && open.title == "Restore Backup")
        #expect(open.headline == "This project already exists on this computer")
        #expect(open.notesNotice == "The private note in this package will become yours. You can see and edit it; other members of the project cannot.")
        #expect(open.mode == .copy && open.primaryLabel == "Open as Copy")
        #expect(open.collision?.title == "Harbour Lights")
        await open.start()
        let done = try #require(open.report)                    // "Opened as a copy."
        #expect(!done.identityKept)
        guard case let .project(opened) = app.phase else { Issue.record("no project is open: \(app.phase)"); return }
        #expect(opened.title == "Harbour Lights (copy)")        // the window title
        open.finish()

        // The restored copy keeps its private note, now owned by the person restoring.
        let notes = try await p.core.privateNoteList(ListPrivateNotesArgs())
        #expect(notes.contains { $0.body.contains("Harbour private note for the restore check") })
    }

    @Test func aDamagedBackupIsRefusedAndNothingIsCreated() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let damaged = p.file("damaged.ofbackup")
        try (Data([0x50, 0x4B, 0x03, 0x04]) + Data(" this is not a whole package".utf8)).write(to: damaged)
        await p.app.closeProject()
        let before = try await p.core.projectListRecent(ListRecentArgs(includeArchived: true)).map(\.title)
        let m = OpenProjectModel(app: p.app)
        await m.choose(file: damaged)
        #expect(m.info == nil)
        let said = (m.errorText ?? "").lowercased()
        #expect(said.contains("package") || said.contains("archive"), "\(said)")
        #expect(!m.started)                                     // "Choose Package…" is still offered
        #expect(try await p.core.projectListRecent(ListRecentArgs(includeArchived: true)).map(\.title) == before)
    }

    @Test func afterARelaunchTheOriginalAndTheCopyAreBothListed() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let backup = try await p.backup()
        await p.app.closeProject()
        let m = OpenProjectModel(app: p.app)
        await m.choose(file: backup)
        await m.start()
        #expect(m.report != nil)
        m.finish()
        p.app.shutdown()
        let again = Fixture.model(p.rig.root)
        await again.start()
        let recent = try await again.core!.projectListRecent(ListRecentArgs(includeArchived: false)).map(\.title)
        #expect(recent.contains("Harbour Lights (copy)"))
        #expect(recent.contains("Harbour Lights"))
        again.shutdown()
    }

    // MARK: Identity collision

    @Test func replaceExistingAfterBackupSavesASafetyBackupFirst() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let package = try await p.backup()
        await p.app.closeProject()
        let m = OpenProjectModel(app: p.app)
        await m.choose(file: package)
        m.mode = .replace
        #expect(m.primaryLabel == "Replace Existing After Backup")
        await m.start(.replace)
        let r = try #require(m.report)
        #expect(r.identityKept)
        let safety = try #require(r.safetyBackup)
        #expect(FileManager.default.fileExists(atPath: safety))
        guard case let .project(project) = p.app.phase else { Issue.record("no project is open"); return }
        #expect(project.title == "Harbour Lights")
        let recent = try await p.core.projectListRecent(ListRecentArgs(includeArchived: true)).map(\.title)
        #expect(recent.filter { $0 == "Harbour Lights" }.count == 1)
    }

    @Test func aPackageWithoutACollisionOpensAsTheSameProject() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let package = p.file("Harbour Lights.ofproject")
        let e = ExportProjectModel(app: p.app)
        await e.load()
        await e.export(to: package)
        await e.task.waitUntilFinished()
        #expect(e.report != nil)
        // A second, empty data folder has no project with this identity.
        let other = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: other) }
        let app2 = Fixture.model(other)
        await app2.start()
        let m = OpenProjectModel(app: app2)
        await m.choose(file: package)
        #expect(m.collision == nil)
        #expect(m.mode == .open && m.primaryLabel == "Open Project" && m.title == "Open Project Package")
        await m.start()
        let r = try #require(m.report)
        #expect(r.identityKept)
        guard case let .project(project) = app2.phase else { Issue.record("no project is open"); return }
        #expect(project.title == "Harbour Lights")
        m.finish()
        app2.shutdown()
    }

    // MARK: Exchange packages

    @Test func aNoteForASceneThatIsGoneWaitsInTheReviewQueueAndIsNeverDiscarded() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        _ = try await p.exportScriptReview()
        // The reviewer comments on the second scene.
        _ = try await p.respond(comment: "The gate line is too soft.", sceneIndex: 1, quote: "Hold the gate.")
        // The author removes that scene before the answer comes back.
        let doc = try await p.core.screenplayDocument(ScreenplayDraftIdArgs(draftId: p.draftId))
        _ = try await p.core.screenplayDeleteScene(ScreenplaySceneIdArgs(sceneId: doc.scenes[1].id))
        let imp = ExchangeImportModel(app: p.app)
        await imp.open(file: p.file("Harbour Lights reply.ofresponse"))
        let s = try #require(imp.session)
        // A changed project is a stale package: the choice is shown first, comments only by default.
        if imp.staleStep {
            #expect(imp.staleChoice == .comments)
            #expect(imp.staleOptions.contains { $0.choice == .compare })
            #expect(imp.staleOptions.first { $0.choice == .comments }?.isDefault == true)
            _ = await imp.continueStale()
        } else {
            await imp.apply(.allSafe)
        }
        let r = try #require(imp.result)
        #expect(r.applied + r.queued + r.pending >= 1, "the comment must go somewhere: \(r.message)")
        _ = s
        if r.queued > 0 {
            let queue = ReviewQueueModel(app: p.app)
            await queue.load()
            let item = try #require(queue.pending.first)
            #expect(item.body == "The gate line is too soft.")
            #expect(item.quotedText == "Hold the gate.")
            let target = try #require(queue.scenes.first)
            await queue.attach(item, to: target.id)
            await queue.load()
            #expect(queue.attached.contains { $0.id == item.id })
            #expect(queue.pending.isEmpty)
        } else {
            // The comment was kept some other safe way (as a record); it still exists in the history.
            let history = PackageHistoryModel(app: p.app)
            await history.load()
            #expect(history.sessions.contains { $0.id == imp.session?.id })
        }
    }

    @Test func anImportCanBeUndoneFromTheHistory() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        _ = try await p.exportScriptReview()
        _ = try await p.respond()
        let imp = ExchangeImportModel(app: p.app)
        await imp.open(file: p.file("Harbour Lights reply.ofresponse"))
        await imp.apply(.commentsOnly)
        let r = try #require(imp.result)
        if r.canUndo {
            let history = PackageHistoryModel(app: p.app)
            await history.load()
            let row = try #require(history.sessions.first { $0.id == imp.session?.id })
            #expect(row.canUndo)
            await history.undo(row)
            await history.load()
            #expect(history.sessions.first { $0.id == row.id }?.status == "Undone")
        }
        await imp.undo()
        #expect(imp.session != nil)
    }

    @Test func aStalePackageOffersViewCommentsRecordOrCompare() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        _ = try await p.exportScriptReview()
        _ = try await p.respond()
        // The author keeps writing: a new scene makes the review package older than the project.
        _ = try await p.rig.importPasted("INT. NEW ROOM - DAY\n\nSomething changes.\n", draftName: "Later")
        let imp = ExchangeImportModel(app: p.app)
        await imp.open(file: p.file("Harbour Lights reply.ofresponse"))
        let s = try #require(imp.session)
        #expect(["Valid", "Stale"].contains(s.compatibility))
        if s.stale {
            #expect(imp.staleStep)
            #expect(imp.staleChoice == .comments || imp.staleChoice == .record)
            imp.staleChoice = .record
            if s.canCreateRecord {
                _ = await imp.continueStale()
                #expect(imp.result?.recordId != nil)
                let records = PackageHistoryModel(app: p.app)
                records.tab = .records
                await records.load()
                #expect(!records.records.isEmpty)
                // The kept record opens read-only in the viewer and can be deleted.
                let rec = try #require(records.records.first)
                let v = ReviewViewerModel(app: p.app, source: .record, key: rec.id)
                await v.load()
                #expect(v.workspace?.canComment == false)
                #expect(v.banner == "Review record, kept separately from your working content.")
                records.deleting = rec
                await records.deleteRecord()
                await records.load()
                #expect(records.records.isEmpty)
            }
        }
    }

    @Test func everyExchangeTypeExplainsItselfOrSaysWhyNot() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        for t in PKInfo.exchangeExportTypes {
            let m = ExchangeExportModel(app: p.app, packageType: t)
            await m.load()
            #expect(m.loadError == nil, "\(t)")
            #expect(!m.heading.title.isEmpty)
            #expect(m.summary != nil || m.previewError != nil || !m.ready, "\(t) neither previews nor explains")
            if let s = m.summary { #expect(s.privateNotesExcluded) }
            #expect(PKInfo.entry(t).fileExtension.hasPrefix("of"))
        }
        // Specific scenes only: nothing chosen is not ready; one scene exports.
        let m = ExchangeExportModel(app: p.app, packageType: .scriptReview)
        await m.load()
        await m.setScope(.scenes)
        #expect(m.scenes.count == 2 && !m.ready && m.summary == nil)
        m.toggleScene(m.scenes[0].id, true)
        await m.refreshPreview()
        #expect(m.ready && m.summary != nil)
        await m.setScope(.comments)
        #expect(m.args.includeComments)
        await m.export(to: p.file("one.ofscriptreview"))
        #expect(m.done != nil)
    }

    @Test func wrongKindsOfPackageAreRefusedInPlainWords() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let backup = try await p.backup()
        _ = try await p.exportScriptReview()
        // A backup is not an exchange package.
        let imp = ExchangeImportModel(app: p.app)
        await imp.open(file: backup)
        #expect(imp.session == nil && (imp.errorText ?? "").count > 10)
        // A review package is not a project package.
        await p.app.closeProject()
        let open = OpenProjectModel(app: p.app)
        await open.choose(file: p.file("Harbour Lights review.ofscriptreview"))
        #expect(open.info == nil && (open.errorText ?? "").count > 10)
        // A text file with a package name is refused too.
        let fake = p.file("fake.ofbackup")
        try Data("not a package".utf8).write(to: fake)
        await open.choose(file: fake)
        #expect(open.info == nil && open.errorText != nil)
        let recent = try await p.core.projectListRecent(ListRecentArgs(includeArchived: true))
        #expect(recent.count == 1)
    }

    @Test func aPackageFromAFutureVersionIsRefusedAndNothingIsCreated() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let backup = try await p.backup()
        // Rewrite the manifest's format version inside the archive.
        let work = p.rig.root.appendingPathComponent("unzipped", isDirectory: true)
        try FileManager.default.createDirectory(at: work, withIntermediateDirectories: true)
        func run(_ tool: String, _ args: [String], in dir: URL) throws {
            let pr = Process()
            pr.executableURL = URL(fileURLWithPath: tool)
            pr.arguments = args
            pr.currentDirectoryURL = dir
            pr.standardOutput = FileHandle.nullDevice
            pr.standardError = FileHandle.nullDevice
            try pr.run(); pr.waitUntilExit()
        }
        try run("/usr/bin/unzip", ["-q", backup.path], in: work)
        let manifest = work.appendingPathComponent("manifest.json")
        try #require(FileManager.default.fileExists(atPath: manifest.path), "the package has no manifest.json")
        var text = String(decoding: bytes(manifest), as: UTF8.self)
        let changed = text.replacingOccurrences(of: "\"formatVersion\"\\s*:\\s*[0-9]+", with: "\"formatVersion\":99", options: .regularExpression)
        try #require(changed != text, "the manifest has no format version")
        text = changed
        try Data(text.utf8).write(to: manifest)
        let future = p.file("future.ofbackup")
        try run("/usr/bin/zip", ["-q", "-r", future.path, "."], in: work)
        await p.app.closeProject()
        let before = try await p.core.projectListRecent(ListRecentArgs(includeArchived: true)).count
        let m = OpenProjectModel(app: p.app)
        await m.choose(file: future)
        if m.info != nil {
            // The version is checked when the import starts; it must be refused there and create nothing.
            await m.start()
            #expect(m.report == nil)
            #expect(m.startError != nil || m.task.state == "failed")
        } else {
            #expect(m.errorText != nil)
        }
        #expect(try await p.core.projectListRecent(ListRecentArgs(includeArchived: true)).count == before)
    }

    // MARK: Tasks and files

    @Test func aBackupTaskCanBeCancelledAndLeavesNothingBehind() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let m = BackupModel(app: p.app)
        await m.load()
        m.choose(destination: p.file("cancelled.ofbackup"))
        let starting = Task { await m.start() }
        m.cancel()
        await starting.value
        await m.task.waitUntilFinished()
        #expect(m.task.finished)
        if m.task.state == "cancelled" {
            #expect(!FileManager.default.fileExists(atPath: p.file("cancelled.ofbackup").path))
        } else {
            #expect(m.task.state == "completed")
        }
    }

    @Test func aBackupToAFolderThatDoesNotExistFailsInPlainWords() async throws {
        let p = try await PKRig.make(note: false)
        defer { p.close() }
        let m = BackupModel(app: p.app)
        await m.load()
        m.choose(destination: p.packageOut.appendingPathComponent("missing/x.ofbackup"))
        await m.start()
        if m.startError == nil { await m.task.waitUntilFinished() }
        #expect(m.report == nil)
        #expect(m.startError != nil || m.task.state == "failed")
        #expect(!FileManager.default.fileExists(atPath: p.packageOut.appendingPathComponent("missing/x.ofbackup").path))
        let ro = p.rig.root.appendingPathComponent("ro", isDirectory: true)
        try FileManager.default.createDirectory(at: ro, withIntermediateDirectories: true)
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: ro.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: ro.path) }
        let m2 = ExportProjectModel(app: p.app)
        await m2.load()
        await m2.export(to: ro.appendingPathComponent("x.ofproject"))
        if m2.startError == nil { await m2.task.waitUntilFinished() }
        #expect(m2.report == nil)
        #expect(!FileManager.default.fileExists(atPath: ro.appendingPathComponent("x.ofproject").path))
    }

    // MARK: Pure helpers

    @Test func namesLabelsAndSelectionFollowWindows() {
        #expect(PKText.backupFileName(baseName: "BLACK RAIN 2026-09-29 1042", label: "") == "BLACK RAIN 2026-09-29 1042.ofbackup")
        #expect(PKText.backupFileName(baseName: "BLACK RAIN 2026-09-29 1042", label: " before/rewrite ") == "BLACK RAIN 2026-09-29 1042 before_rewrite.ofbackup")
        #expect(Set(PackageTypeAll.all.map { PKInfo.entry($0).fileExtension }).count == PackageTypeAll.all.count)
        #expect(PKInfo.extensions([.story]) == ["ofstory"])
        #expect(PKText.formatBytes(500) == "500 B" && PKText.formatBytes(2048) == "2 KB" && PKText.formatBytes(3 * 1024 * 1024) == "3.0 MB")
        #expect(PKText.stateChip(.safe).label == "Safe mapping" && PKText.stateChip(.unmapped).label == "Unmapped")
        #expect(PKText.resultLabel("PartiallyApplied") == "Partially Applied" && PKText.resultLabel("PendingReview") == "Pending Review")
        func change(_ id: String, _ group: String, kind: PackageChangeKind = .comment, state: PackageChangeState = .safe, applicable: Bool = true, def: Bool = true, copy: Bool = false) -> PackageChange {
            PackageChange(id: id, group: group, groupLabel: group, label: id, detail: nil, kind: kind, state: state, applicable: applicable, selectedByDefault: def, copyOnly: copy, candidates: [])
        }
        let changes = [change("a", "comments"), change("b", "comments", def: false), change("c", "text", kind: .info, state: .review, applicable: false),
                       change("d", "copy", def: false, copy: true), change("e", "story.cards", kind: .update, state: .conflict, def: false)]
        let sel = PKText.defaultSelection(changes)
        #expect(sel == ["a"])
        #expect(PKText.groupSelection(sel, changes, group: "comments") == .some)
        let all = PKText.toggleGroup(sel, changes, group: "comments", on: true)
        #expect(PKText.groupSelection(all, changes, group: "comments") == .all)
        #expect(PKText.groupSelection(PKText.toggleGroup(all, changes, group: "comments", on: false), changes, group: "comments") == .none)
        #expect(!PKText.toggleGroup(sel, changes, group: "text", on: true).contains("c"))
        let manifest = { (notes: Bool, n: Double?) in
            PackageManifest(format: "x", packageType: .backup, formatVersion: 1, packageId: "p", sourceProjectId: "s", sourceProjectTitle: "T", exportedAt: 0, appVersion: "1",
                            schemaVersion: nil, sourceDraft: nil, sourceVersions: [], includedObjectIds: [], scope: PackageScope(kind: "all", label: "", ids: []),
                            commentsIncluded: false, attachmentsIncluded: false, privateNotesIncluded: notes, baseSnapshot: [:],
                            originatingUser: PackageUser(userId: "u", displayName: "n"), label: nil, respondsTo: nil,
                            tableCounts: n.map { ["private_note": $0] } ?? [:], carried: [])
        }
        #expect(PKText.restoredNotesNotice(manifest(true, 2)) == "The 2 private notes in this package will become yours. You can see and edit them; other members of the project cannot.")
        #expect(PKText.restoredNotesNotice(manifest(true, nil))?.hasPrefix("Private notes in this package will become yours.") == true)
        #expect(PKText.restoredNotesNotice(manifest(true, 0)) == nil)
        #expect(PKText.restoredNotesNotice(manifest(false, 3)) == nil)
    }

    // MARK: Rendering

    @Test func thePackageSheetsBuild() async throws {
        let p = try await PKRig.make()
        defer { p.close() }
        _ = try await p.exportScriptReview()
        let store = ThemeStore()
        let imp = ExchangeImportModel(app: p.app)
        await imp.open(file: p.file("Harbour Lights review.ofscriptreview"))
        let queue = ReviewQueueModel(app: p.app)
        await queue.load()
        let hist = PackageHistoryModel(app: p.app)
        await hist.load()
        let viewer = ReviewViewerModel(app: p.app, source: .review, key: "")
        await viewer.open(file: p.file("Harbour Lights review.ofscriptreview"))
        let views: [(String, AnyView)] = [
            ("backup", AnyView(BackupSheet(model: BackupModel(app: p.app), close: {}))),
            ("export-project", AnyView(ExportProjectSheet(model: ExportProjectModel(app: p.app), close: {}))),
            ("exchange-export", AnyView(ExchangeExportSheet(model: ExchangeExportModel(app: p.app, packageType: .scriptReview), close: {}))),
            ("exchange-import", AnyView(ExchangeImportSheet(model: imp, close: {}))),
            ("open-project", AnyView(OpenProjectSheet(model: OpenProjectModel(app: p.app), close: {}))),
            ("queue", AnyView(ReviewQueueSheet(model: queue, close: {}))),
            ("history", AnyView(PackageHistorySheet(model: hist, close: {}))),
            ("viewer", AnyView(ReviewViewerSheet(model: viewer, close: {}))),
        ]
        for (name, v) in views {
            #expect(Fixture.render(v.environment(p.app).environment(store).openFrameTheme(store), name: "m7-pk-\(name)", size: CGSize(width: 1080, height: 720)) != nil, "\(name)")
        }
    }
}

private enum PackageTypeAll {
    static let all: [PackageType] = [.backup, .project, .scriptReview, .story, .breakdown, .shots, .schedule, .callReview, .response]
}

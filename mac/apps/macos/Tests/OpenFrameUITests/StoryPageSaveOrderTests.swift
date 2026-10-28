// The Story page never loses or undoes what was typed: saves after a project move, the order of
// failed and later saves, a page that left the screen meeting a newer revision, and a save the
// core refused. Run against a real core on a temporary folder.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("StoryPageTests save order", .serialized)
@MainActor
struct StoryPageSaveOrderTests {
    private func project(_ title: String) async throws -> (URL, AppModel, ProjectSummary) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        #expect(await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm)))
        guard case let .project(p) = app.phase else { throw CoreFailure(code: "no_project", message: "No project is open.") }
        return (root, app, p)
    }

    private func stored(_ app: AppModel) async throws -> StoryPageDto { try await app.core!.storyPageGet() }

    private func moveProject(_ app: AppModel, to dest: URL) -> StorageTaskModel {
        StorageTaskModel(
            app: app, title: "Move This Project", confirm: [], confirmLabel: "Move Project", relocates: true,
            start: { try await $0.projectMove(MoveToFolderArgs(destParent: dest.path)).taskId },
            done: { _ in "" })
    }

    @Test func textTypedBeforeAProjectMoveIsStoredAndLaterSavesStillWork() async throws {
        let (root, app, before) = try await project("Moving Day")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        // Typed just before the move: the 400 ms save has not run yet.
        m.changed(text: "Before the move", format: nil)
        let dest = root.appendingPathComponent("moved", isDirectory: true)
        try FileManager.default.createDirectory(at: dest, withIntermediateDirectories: true)
        let move = moveProject(app, to: dest)
        await move.start()
        #expect(move.startError == nil)
        await move.waitUntilSettled()
        guard case let .completed = move.stage else { Issue.record("the move did not complete: \(move.stage)"); return }
        guard case let .project(after) = app.phase else { Issue.record("no project after the move"); return }
        #expect(after.id == before.id && after.path != before.path)
        #expect(try await stored(app).text == "Before the move")
        // The page stays open (same project id): what is typed after the move is stored too.
        m.changed(text: "After the move", format: nil)
        #expect(await m.flush())
        #expect(m.status == .saved)
        #expect(try await stored(app).text == "After the move")
    }

    @Test func aFailedOlderSaveNeverComesBackOverANewerOne() async throws {
        let (root, app, project) = try await project("Order")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        // Save 1 fails (the project is closed while it runs); save 2 is queued behind it and works.
        _ = StoryPageModel.queue { _ = try? await app.core!.projectClose() }
        m.changed(text: "Older", format: nil)
        m.leave()
        _ = StoryPageModel.queue { await app.openProject(path: project.path) }
        m.changed(text: "Newer", format: nil)
        m.leave()
        await StoryPageModel.settledSaves()
        #expect(try await stored(app).text == "Newer")
        #expect(m.status == .saved)
        // Nothing older is waiting: closing sends nothing that undoes the newest text.
        await StoryPageModel.flushAll()
        #expect(try await stored(app).text == "Newer")
    }

    @Test func aPageThatLeftTheScreenNeverWritesOverANewerRevision() async throws {
        let (root, app, project) = try await project("Return")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let first = StoryPageModel(app: app)
        await first.load()
        first.changed(text: "Typed while the save failed", format: nil)
        _ = try await app.core!.projectClose()
        #expect(await first.flush() == false)
        // The writer leaves the page and comes back: a new page reads the stored text.
        first.close()
        await app.openProject(path: project.path)
        let second = StoryPageModel(app: app)
        await second.load()
        second.changed(text: "Newest edits", format: nil)
        #expect(await second.flush())
        // Closing the project stores every page: the old one must not overwrite the newest edits.
        await StoryPageModel.flushAll()
        #expect(try await stored(app).text == "Newest edits")
    }

    @Test func aRefusedSaveShowsTheCoresReasonAndIsNotSentAgainUntilTheTextChanges() async throws {
        let (root, app, project) = try await project("Too Long")
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = StoryPageModel(app: app)
        await m.load()
        m.changed(text: String(repeating: "a", count: 5 * 1024 * 1024 + 10), format: nil)
        #expect(await m.flush() == false)
        #expect(m.statusLabel == "Not saved yet")
        let reason = try #require(m.saveError)
        #expect(reason.contains("5 MB"))
        // The same refused text is not sent again: with the project closed, a new try would
        // replace the reason with a "no project" failure.
        _ = try await app.core!.projectClose()
        #expect(await m.flush() == false)
        #expect(m.saveError == reason)
        await app.openProject(path: project.path)
        // A change is sent, and a stored page clears the reason.
        m.changed(text: "Short again", format: nil)
        #expect(await m.flush())
        #expect(m.saveError == nil && m.status == .saved)
        #expect(try await stored(app).text == "Short again")
    }
}

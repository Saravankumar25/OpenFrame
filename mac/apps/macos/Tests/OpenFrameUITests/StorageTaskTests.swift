// A storage task whose end is never heard (its last event dropped, or the core stopped) does not
// keep the window in "moving" for ever: the move counter is given back, so a later close of the
// project goes Home. Run against a real core on a temporary folder.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Storage task end", .serialized)
@MainActor
struct StorageTaskTests {
    private func project() async throws -> (URL, AppModel) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        #expect(await app.createProject(CreateProjectArgs(title: "Lost Task", projectType: .featureFilm)))
        return (root, app)
    }

    /// A move whose task id never reports anything, like a task whose last event was missed.
    private func silentMove(_ app: AppModel, silence: Duration) -> StorageTaskModel {
        StorageTaskModel(
            app: app, title: "Move This Project", confirm: [], confirmLabel: "Move Project", relocates: true,
            start: { _ in "task-that-never-reports" }, done: { _ in "" }, silence: silence)
    }

    @Test func aMoveWhoseEndIsNeverHeardGivesTheCounterBackAfterALongSilence() async throws {
        let (root, app) = try await project()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let move = silentMove(app, silence: .milliseconds(300))
        await move.start()
        #expect(app.relocations == 1)
        await move.waitUntilSettled(seconds: 5)
        #expect(move.settled)
        #expect(move.stage == .lost)
        #expect(move.finished && !move.running)
        #expect(app.relocations == 0)
        // The core closes the project: the window goes Home again.
        _ = try await app.core!.projectClose()
        for _ in 0..<100 where app.phase != .home { try? await Task.sleep(for: .milliseconds(20)) }
        #expect(app.phase == .home)
    }

    @Test func aMoveGivesTheCounterBackWhenTheCoreStops() async throws {
        let (root, app) = try await project()
        defer { try? FileManager.default.removeItem(at: root) }
        let move = silentMove(app, silence: .seconds(600))
        await move.start()
        #expect(app.relocations == 1)
        app.shutdown()
        await move.waitUntilSettled(seconds: 5)
        #expect(move.settled)
        #expect(move.stage == .lost)
        #expect(app.relocations == 0)
    }
}

// Home lists the recent projects, so a project that opens or closes must wake it up: a project
// made and closed at once showed an empty list until something else changed.

import Foundation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

@Suite("Home reload on open and close", .serialized)
@MainActor
struct HomeReloadTests {
    @Test func openingAndClosingAProjectBumpsTheChangeCounter() async throws {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let before = app.changes.any
        app.handle(.projectOpened(projectId: "p1"))
        #expect(app.changes.any == before + 1)
        app.handle(.projectClosed(projectId: "p1"))
        #expect(app.changes.any == before + 2)
        #expect(app.changes.key(["sys_recent_project"], store: .global) >= 2)
    }
}

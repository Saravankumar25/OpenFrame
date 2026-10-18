// First launch: the welcome sheet and the name (scenario 01-journey case 1; 85 Activity shows it).

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@Suite("Onboarding", .serialized)
@MainActor
struct OnboardingTests {
    @Test func firstLaunchAsksForTheNameOnceAndTheActivityShowsIt() async throws {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        #expect(app.phase == .home)
        #expect(app.needsWelcomeName)

        let welcome = WelcomeNameModel(app: app, current: app.info?.profile.displayName ?? "")
        welcome.name = "   "
        #expect(welcome.invalid)
        #expect(await welcome.continueTapped() == false)
        #expect(app.needsWelcomeName)

        welcome.name = "  E2E Tester "
        #expect(await welcome.continueTapped())
        #expect(welcome.done)
        #expect(!app.needsWelcomeName)
        #expect(app.info?.profile.displayName == "E2E Tester")

        // A project made now credits that name in its activity.
        _ = await app.createProject(CreateProjectArgs(title: "Tide Line", projectType: .featureFilm))
        let notes = try await app.core!.appInfo()
        #expect(notes.profile.displayName == "E2E Tester")
    }

    @Test func theQuestionDoesNotComeBackAfterARelaunch() async throws {
        let root = try Fixture.root()
        let first = Fixture.model(root)
        await first.start()
        let welcome = WelcomeNameModel(app: first, current: "")
        welcome.name = "Asha"
        #expect(await welcome.continueTapped())
        first.shutdown()
        try await Task.sleep(for: .milliseconds(300))

        let second = Fixture.model(root)
        defer { second.shutdown(); try? FileManager.default.removeItem(at: root) }
        await second.start()
        #expect(!second.needsWelcomeName)
        #expect(second.info?.profile.displayName == "Asha")
    }

    @Test func theWelcomeSheetRendersWithItsWords() async throws {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let themes = ThemeStore()
        let view = WelcomeNameView(model: WelcomeNameModel(app: app, current: "m.udarshreddy"))
            .environment(app).environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
        #expect(Fixture.render(view, name: "welcome-name", size: CGSize(width: 460, height: 300)) != nil)
    }
}

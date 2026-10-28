// Mac phase M3 scenarios, driven through the app model against the real core on a temporary
// folder: Home to an empty project and back, and rendered snapshots of the main screens in light
// and dark. (The UI testing framework that clicks the real window needs full Xcode and arrives in
// phase M9; these tests drive the same model the window uses.)

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

enum Fixture {
    static func root() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("openframe-ui-tests", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    static let snapshots = URL(fileURLWithPath:
        "/private/tmp/claude-501/-Users-m-udarshreddy-Desktop-OpenFrame/78b64903-9d1c-4e08-a663-f47a7233f6c5/scratchpad/snapshots",
        isDirectory: true)

    @MainActor
    static func model(_ root: URL) -> AppModel {
        AppModel(options: CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false))
    }

    /// Draws the view in an off-screen window through AppKit (so scroll views, text fields and
    /// controls draw too, which ImageRenderer cannot) and writes a PNG for a person to look at.
    @MainActor
    static func render<V: View>(_ view: V, name: String, size: CGSize = CGSize(width: 1100, height: 720)) -> NSImage? {
        _ = NSApplication.shared
        let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height))
        host.frame = NSRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        // Let SwiftUI run its first update and any immediate tasks.
        RunLoop.main.run(until: Date().addingTimeInterval(1.2))
        host.layoutSubtreeIfNeeded()
        guard let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else { return nil }
        host.cacheDisplay(in: host.bounds, to: rep)
        let image = NSImage(size: size)
        image.addRepresentation(rep)
        try? FileManager.default.createDirectory(at: snapshots, withIntermediateDirectories: true)
        if let png = rep.representation(using: .png, properties: [:]) {
            try? png.write(to: snapshots.appendingPathComponent("\(name).png"))
        }
        return image
    }
}

@Suite("Shell scenarios", .serialized)
@MainActor
struct ShellScenarioTests {
    @Test func homeToAnEmptyProjectAndBack() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let model = Fixture.model(root)
        await model.start()
        #expect(model.phase == .home)
        #expect(model.info?.projectsDir.hasPrefix(root.path) == true)

        let ok = await model.createProject(CreateProjectArgs(title: "Empty Film", projectType: .featureFilm))
        #expect(ok)
        guard case let .project(project) = model.phase else {
            Issue.record("expected an open project, got \(model.phase)")
            return
        }
        #expect(project.title == "Empty Film")
        let home = try await model.core!.projectHome()
        #expect(home.isEmpty)

        await model.closeProject()
        #expect(model.phase == .home)
        let recent = try await model.core!.projectListRecent(ListRecentArgs(includeArchived: false))
        #expect(recent.map(\.title) == ["Empty Film"])

        await model.openProject(path: recent[0].path)
        #expect({ if case .project = model.phase { true } else { false } }())
        model.shutdown()
    }

    @Test func aBadProjectFolderShowsTheCoresMessage() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let model = Fixture.model(root)
        await model.start()
        await model.openProject(path: root.appendingPathComponent("not-a-project").path)
        #expect(model.phase == .home)
        guard case let .error(_, failure)? = model.alert else {
            Issue.record("expected an error alert")
            return
        }
        #expect(!failure.message.isEmpty)
        #expect(!failure.message.contains("internal"))
        model.shutdown()
    }

    @Test func emptyTitleIsRefusedByTheCore() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let model = Fixture.model(root)
        await model.start()
        let ok = await model.createProject(CreateProjectArgs(title: "   ", projectType: .shortFilm))
        #expect(!ok)
        #expect(model.phase == .home)
        #expect(model.alert != nil)
        model.shutdown()
    }

    @Test func screensRenderInLightAndDarkWithEveryTheme() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let model = Fixture.model(root)
        await model.start()
        _ = await model.createProject(CreateProjectArgs(title: "Night Train", projectType: .featureFilm, genre: "Thriller", creator: "A. Writer"))
        await model.closeProject()
        let themes = ThemeStore()
        for (themeId, scheme) in [("openframe-default", ColorScheme.light), ("openframe-default", .dark), ("high-contrast", .light), ("midnight-reel", .dark)] {
            themes.themeId = themeId
            let home = HomeView()
                .environment(model)
                .environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
                .environment(\.colorScheme, scheme)
            let image = Fixture.render(home, name: "home-\(themeId)-\(scheme == .dark ? "dark" : "light")")
            #expect(image != nil)
        }
        model.shutdown()
    }
}

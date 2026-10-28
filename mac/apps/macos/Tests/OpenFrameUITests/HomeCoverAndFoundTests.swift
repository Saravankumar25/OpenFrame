// Home: a cover photo on a project card with the generated cover as the fallback (Windows ddcf1317,
// AppHome.test.tsx), and "Found on this Mac" for projects in the projects folder that are not in the
// list (Windows FoundProjects, "Found on this PC").

import AppKit
import Foundation
import OpenFrameCore
import Testing
import UniformTypeIdentifiers
@testable import OpenFrameUI

@Suite("Home cover photo and found projects", .serialized)
@MainActor
struct HomeCoverAndFoundTests {
    @MainActor private struct Rig {
        let root: URL
        let app: AppModel
        func close() {
            app.shutdown()
            try? FileManager.default.removeItem(at: root)
        }
    }

    /// A started app with closed projects of these titles in its list.
    private func rig(_ titles: [String]) async throws -> Rig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        for t in titles {
            #expect(await app.createProject(CreateProjectArgs(title: t, projectType: .featureFilm)))
            await app.closeProject()
        }
        return Rig(root: root, app: app)
    }

    /// A real PNG picture of the given size.
    private func png(_ dir: URL, width: Int = 320, height: Int = 180) throws -> String {
        let rep = try #require(NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8, samplesPerPixel: 4,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
        NSColor.systemTeal.setFill()
        NSRect(x: 0, y: 0, width: width, height: height).fill()
        NSGraphicsContext.restoreGraphicsState()
        let data = try #require(rep.representation(using: .png, properties: [:]))
        let url = dir.appendingPathComponent("poster.png")
        try data.write(to: url)
        return url.path
    }

    @Test func showsTheChosenCoverPhotoAndTheGeneratedCoverWhenThereIsNone() async throws {
        let r = try await rig(["With Photo", "No Photo"])
        defer { r.close() }
        let home = HomeModel()
        await home.load(r.app.core)
        let with = try #require(home.recents.first { $0.title == "With Photo" })
        let without = try #require(home.recents.first { $0.title == "No Photo" })
        #expect(with.cover == nil && ProjectCoverView.source(with) == .generated(projectId: with.projectId))
        await home.setCover(r.app, with, path: try png(r.root))
        #expect(r.app.alert == nil, "the core refused a real PNG")
        let after = try #require(home.recents.first { $0.projectId == with.projectId })
        let cover = try #require(after.cover, "the card was not updated in place")
        #expect(ProjectCoverView.source(after) == .photo(path: cover.path, rev: cover.rev))
        // The picture is the app's own copy, readable for the card.
        #expect(cover.path != r.root.appendingPathComponent("poster.png").path)
        #expect(await ProjectCoverView.loadImage(path: cover.path) != nil)
        #expect(ProjectCoverView.source(without) == .generated(projectId: without.projectId))
        // A reload keeps it.
        await home.load(r.app.core)
        #expect(home.recents.first { $0.projectId == with.projectId }?.cover == cover)
        #expect(ToastCenter.shared.items.last?.text == "Cover photo changed")
    }

    @Test func offersChangeCoverPhotoAndRemoveCoverPhotoOnlyWhenOneIsSet() async throws {
        let r = try await rig(["With Photo", "No Photo"])
        defer { r.close() }
        let home = HomeModel()
        await home.load(r.app.core)
        let with = try #require(home.recents.first { $0.title == "With Photo" })
        await home.setCover(r.app, with, path: try png(r.root))
        let set = try #require(home.recents.first { $0.projectId == with.projectId })
        let none = try #require(home.recents.first { $0.title == "No Photo" })
        #expect(CoverMenuItem.items(set) == [.change, .remove])
        #expect(CoverMenuItem.items(none) == [.change])
        #expect(CoverMenuItem.change.title == "Change Cover Photo\u{2026}")
        #expect(CoverMenuItem.remove.title == "Remove Cover Photo")
    }

    @Test func thePickerTakesOnePictureAndCancelChangesNothing() async throws {
        let panel = HomeView.coverPanel()
        #expect(!panel.allowsMultipleSelection && !panel.canChooseDirectories && panel.canChooseFiles)
        #expect(Set(panel.allowedContentTypes) == [UTType.png, .jpeg, .webP])
        #expect(panel.title == "Choose a cover photo")
        // Cancel: the menu action returns before calling the core, so the list is unchanged.
        let r = try await rig(["No Photo"])
        defer { r.close() }
        let home = HomeModel()
        await home.load(r.app.core)
        let before = home.recents
        await home.load(r.app.core)
        #expect(home.recents == before)
    }

    @Test func removesTheCoverPhotoFromTheMenu() async throws {
        let r = try await rig(["With Photo"])
        defer { r.close() }
        let home = HomeModel()
        await home.load(r.app.core)
        let p = try #require(home.recents.first)
        await home.setCover(r.app, p, path: try png(r.root))
        let set = try #require(home.recents.first)
        let file = try #require(set.cover?.path)
        await home.clearCover(r.app, set)
        #expect(r.app.alert == nil)
        let cleared = try #require(home.recents.first)
        #expect(cleared.cover == nil)
        #expect(ProjectCoverView.source(cleared) == .generated(projectId: cleared.projectId))
        #expect(!FileManager.default.fileExists(atPath: file), "the old picture was kept")
        #expect(ToastCenter.shared.items.last?.text == "Cover photo removed")
    }

    @Test func aFileThatIsNotAPictureIsRefusedAndTheCardKeepsItsCover() async throws {
        let r = try await rig(["Bad Photo"])
        defer { r.close() }
        let home = HomeModel()
        await home.load(r.app.core)
        let p = try #require(home.recents.first)
        let fake = r.root.appendingPathComponent("not-a-picture.png")
        try Data("plain words".utf8).write(to: fake)
        await home.setCover(r.app, p, path: fake.path)
        #expect(r.app.alert != nil, "the refusal was not shown")
        #expect(home.recents.first?.cover == nil)
        // A picture that is too small is refused too (the core's limit, 64 pixels).
        r.app.alert = nil
        await home.setCover(r.app, p, path: try png(r.root, width: 20, height: 20))
        #expect(r.app.alert != nil)
        #expect(home.recents.first?.cover == nil)
    }

    @Test func foundOnThisMacListsProjectsMissingFromTheListAndOpensThem() async throws {
        let r = try await rig(["Kept", "Forgotten"])
        defer { r.close() }
        let home = HomeModel()
        await home.load(r.app.core)
        #expect(home.found.isEmpty, "a project in the list is also listed as found")
        let gone = try #require(home.recents.first { $0.title == "Forgotten" })
        _ = try await r.app.core!.projectRemoveRecent(ProjectIdArgs(projectId: gone.projectId))
        await home.load(r.app.core)
        #expect(!home.recents.contains { $0.projectId == gone.projectId })
        let found = try #require(home.found.first)
        #expect(home.found.count == 1)
        #expect(found.title == "Forgotten" && found.projectId == gone.projectId)
        #expect(FoundProjectsSection.title == "Found on this Mac")
        #expect(FoundProjectsSection.note == "These projects are in your projects folder but not in your list. Open one to add it back.")
        // Open adds it back to the list.
        await r.app.openProject(path: found.path)
        guard case .project = r.app.phase else {
            Issue.record("the found project did not open")
            return
        }
        await r.app.closeProject()
        await home.load(r.app.core)
        #expect(home.recents.contains { $0.projectId == gone.projectId })
        #expect(home.found.isEmpty)
    }
}

// Mac phase M3, project lifecycle, undo toasts, the command palette and About: the shared
// scenarios 10-home and 16-about, driven through the same models and actions the screens use,
// against the real core on a temporary folder.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Shell lifecycle", .serialized)
@MainActor
struct ShellLifecycleTests {
    /// A started app on its own data folder with one closed project, "Harbour Lights".
    @MainActor struct Session {
        let root: URL
        let app: AppModel
        var actions: ProjectActions { ProjectActions(app: app) }
        var core: Core { app.core! }

        func recents(archived: Bool = false) async throws -> [RecentProject] {
            try await core.projectListRecent(ListRecentArgs(includeArchived: archived))
        }
    }

    private func start(title: String = "Harbour Lights", close: Bool = true) async throws -> Session {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        #expect(app.phase == .home)
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        #expect(ok)
        if close { await app.closeProject() }
        ToastCenter.shared.dismissAll()
        return Session(root: root, app: app)
    }

    private func finish(_ s: Session) {
        s.app.shutdown()
        try? FileManager.default.removeItem(at: s.root)
        ToastCenter.shared.dismissAll()
    }

    // MARK: Rename, duplicate, archive

    @Test func renameShowsTheNewTitleAndSaysSo() async throws {
        let s = try await start()
        defer { finish(s) }
        let card = try #require(try await s.recents().first)
        try await s.actions.rename(projectId: card.projectId, to: "  Harbour Lights Redux ")
        let after = try await s.recents()
        #expect(after.map(\.title) == ["Harbour Lights Redux"])
        #expect(after[0].projectType == "Feature Film")
        #expect(after[0].status == "Idea")
        #expect(ToastCenter.shared.items.last?.text == "Renamed to \u{201C}Harbour Lights Redux\u{201D}.")
    }

    @Test func renameToNothingIsRefusedByTheCore() async throws {
        let s = try await start()
        defer { finish(s) }
        let card = try #require(try await s.recents().first)
        await #expect(throws: (any Error).self) { try await s.actions.rename(projectId: card.projectId, to: "   ") }
        #expect(try await s.recents().map(\.title) == ["Harbour Lights"])
    }

    @Test func duplicateMakesASeparateProject() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        await s.actions.duplicate(original)
        let all = try await s.recents()
        #expect(all.count == 2)
        #expect(Set(all.map(\.projectId)).count == 2)
        #expect(Set(all.map(\.path)).count == 2)
        let copy = try #require(all.first { $0.projectId != original.projectId })
        #expect(copy.projectType == "Feature Film")
        #expect(copy.title != original.title)
        #expect(ToastCenter.shared.items.last?.text == "Created \u{201C}\(copy.title)\u{201D}. It is a separate project with its own identity.")
    }

    @Test func archiveHidesFromTheDefaultListAndRestoreBringsItBack() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        await s.actions.duplicate(original)
        let copy = try #require(try await s.recents().first { $0.projectId != original.projectId })

        await s.actions.setArchived(copy, true)
        #expect(try await s.recents().map(\.projectId) == [original.projectId])
        let withArchived = try await s.recents(archived: true)
        #expect(withArchived.count == 2)
        #expect(withArchived.first { $0.projectId == copy.projectId }?.archived == true)
        #expect(ToastCenter.shared.items.last?.text.hasPrefix("\u{201C}\(copy.title)\u{201D} was archived.") == true)

        // The Home model shows an archived project only when asked to.
        let home = HomeModel()
        await home.load(s.app.core)
        #expect(home.pinned.count + home.others.count == 1 && home.archived.isEmpty)
        home.showArchived = true
        await home.load(s.app.core)
        #expect(home.archived.map(\.projectId) == [copy.projectId])

        await s.actions.setArchived(copy, false)
        #expect(try await s.recents().count == 2)
        #expect(ToastCenter.shared.items.last?.text == "\u{201C}\(copy.title)\u{201D} is back in your projects.")
    }

    @Test func searchProjectsFiltersByTitleTypeAndStatus() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        await s.actions.duplicate(original)
        let home = HomeModel()
        await home.load(s.app.core)
        #expect(home.others.count == 2)
        home.query = "Copy of"
        #expect(home.others.count == 1 && home.others[0].title.hasPrefix("Copy of"))
        home.query = "feature"
        #expect(home.others.count == 2)
        home.query = "zzz"
        #expect(!home.hasMatches)
        home.query = ""
        #expect(home.others.count == 2)
    }

    // MARK: Delete

    /// The test core moves a deleted folder into a `recycle-bin` folder of a test data root (the first
    /// root of this process), never the real Trash. Find it by the folder name.
    private func inTestBin(_ folderName: String) -> Bool {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("openframe-ui-tests", isDirectory: true)
        let roots = (try? FileManager.default.contentsOfDirectory(at: base, includingPropertiesForKeys: nil)) ?? []
        for root in roots {
            let bin = root.appendingPathComponent("recycle-bin", isDirectory: true)
            let names = (try? FileManager.default.contentsOfDirectory(atPath: bin.path)) ?? []
            if names.contains(where: { $0.hasSuffix("-\(folderName)") }) { return true }
        }
        return false
    }

    @Test func deleteMovesTheFolderOutAndHomeForgetsIt() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        await s.actions.duplicate(original)
        let copy = try #require(try await s.recents().first { $0.projectId != original.projectId })
        let folder = URL(fileURLWithPath: copy.path)
        #expect(FileManager.default.fileExists(atPath: folder.path))

        try await s.actions.delete(copy)

        #expect(!FileManager.default.fileExists(atPath: folder.path))
        #expect(inTestBin(folder.lastPathComponent))
        #expect(try await s.recents(archived: true).map(\.projectId) == [original.projectId])
        // The words are the Windows ones with the Trash for the Recycle Bin.
        #expect(ToastCenter.shared.items.last?.text == "\u{201C}\(copy.title)\u{201D} was moved to the Trash.")
        // The other project is untouched.
        #expect(FileManager.default.fileExists(atPath: original.path))
    }

    @Test func deleteOfAnOpenProjectIsRefusedAndNothingMoves() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        guard case let .project(open) = s.app.phase else { Issue.record("expected an open project"); return }
        let recent = try #require(try await s.recents().first { $0.projectId == open.id })
        await #expect(throws: (any Error).self) { try await s.actions.delete(recent) }
        #expect(FileManager.default.fileExists(atPath: open.path))
    }

    // MARK: Locate a moved project

    @Test func locateReconnectsAMovedFolderAndOpensIt() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        let moved = s.root.appendingPathComponent("elsewhere", isDirectory: true)
        try FileManager.default.createDirectory(at: moved, withIntermediateDirectories: true)
        let newFolder = moved.appendingPathComponent("Harbour Lights Moved", isDirectory: true)
        try FileManager.default.moveItem(atPath: original.path, toPath: newFolder.path)

        let missing = try #require(try await s.recents().first)
        #expect(!missing.available)

        let ok = await s.actions.locate(missing, at: newFolder.path)
        #expect(ok)
        guard case let .project(project) = s.app.phase else { Issue.record("expected an open project, got \(s.app.phase)"); return }
        #expect(project.id == original.projectId)
        #expect(URL(fileURLWithPath: project.path).standardizedFileURL.path.hasSuffix("Harbour Lights Moved"))
        await s.app.closeProject()
        let after = try #require(try await s.recents().first)
        #expect(after.available)
    }

    @Test func locateRefusesAFolderOfAnotherProject() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        await s.actions.duplicate(original)
        let all = try await s.recents()
        let copy = try #require(all.first { $0.projectId != original.projectId })
        let ok = await s.actions.locate(original, at: copy.path)
        #expect(!ok)
        #expect(s.app.phase == .home)
        guard case .error? = s.app.alert else { Issue.record("expected the core's message"); return }
    }

    // MARK: Undo toast

    @Test func statusChangeOffersUndoAndUndoRestoresTheStatus() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        await s.actions.setStatus(.writing)
        #expect(try await s.core.projectCurrent()?.status == "Writing")
        let toast = try #require(ToastCenter.shared.items.last)
        #expect(toast.text == "Changed project status to Writing")
        #expect(toast.actionLabel == "Undo")

        await ToastCenter.shared.perform(toast)
        #expect(try await s.core.projectCurrent()?.status == "Idea")
        // The toast is gone and a plain answer says what was undone.
        #expect(!ToastCenter.shared.items.contains { $0.id == toast.id })
        #expect(ToastCenter.shared.items.last?.text == "Undid: Changed project status to Writing")
        // Redo also works through the Edit menu path.
        await s.app.redo()
        #expect(try await s.core.projectCurrent()?.status == "Writing")
    }

    @Test func toastsGoAwayOnTheirOwnAndKeepOnlyThree() async throws {
        ToastCenter.shared.dismissAll()
        defer { ToastCenter.shared.dismissAll() }
        let first = ToastCenter.shared.show("one", seconds: 0.2)
        for n in 2...4 { ToastCenter.shared.show("t\(n)", seconds: 30) }
        #expect(ToastCenter.shared.items.count == 3)
        #expect(!ToastCenter.shared.items.contains { $0.id == first })
        let short = ToastCenter.shared.show("brief", seconds: 0.2)
        try await Task.sleep(for: .milliseconds(500))
        #expect(!ToastCenter.shared.items.contains { $0.id == short })
    }

    // MARK: Command palette and search

    @Test func searchFindsASceneHeadingAndAVaultNoteWithNavigableTargets() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        let created = try await s.core.screenplayCreate(CreateScreenplayArgs(title: "Harbour Script"))
        _ = try await s.core.screenplayCreateScene(ScreenplayCreateSceneArgs(draftId: created.draft.id, heading: "INT. LIGHTHOUSE KEEPER'S ROOM - NIGHT"))
        _ = try await s.core.vaultCreate(VaultCreateArgs(store: .project, itemType: .note, body: "Foghorn motif for the harbour sequence"))

        let palette = SearchPaletteModel(app: s.app)
        palette.query = "lighthouse"
        await palette.search()
        let scene = try #require(palette.hits.first { $0.entityType == "screenplay_scene" || $0.entityType == "screenplay_element" })
        let sceneTarget = SearchRouting.target(for: scene)
        #expect(sceneTarget.workspace == .screenplay)
        #expect(sceneTarget.screenplay["draftId"] == created.draft.id)
        #expect(sceneTarget.screenplay["screenplayId"] == created.screenplay.id)

        palette.query = "foghorn"
        await palette.search()
        let note = try #require(palette.hits.first { $0.entityType == "vault_item" })
        #expect(SearchRouting.target(for: note).workspace == .ideaVault)
        #expect(SearchRouting.label(for: note.entityType) == "Idea Vault")
    }

    @Test func jumpingFromAResultPostsTheWorkspaceAndTheScreenplayTarget() async throws {
        final class Seen: @unchecked Sendable { var workspace: String?; var info: [AnyHashable: Any]? }
        let seen = Seen()
        let center = NotificationCenter.default
        let t1 = center.addObserver(forName: .openFrameShowWorkspace, object: nil, queue: nil) { seen.workspace = $0.object as? String }
        let t2 = center.addObserver(forName: .openFrameShowScreenplayTarget, object: nil, queue: nil) { seen.info = $0.userInfo }
        defer { center.removeObserver(t1); center.removeObserver(t2) }
        SearchRouting.go(SearchTarget(workspace: .screenplay, screenplay: ["screenplayId": "sp1", "draftId": "d1", "sceneId": "sc1"]))
        #expect(seen.workspace == "screenplay")
        #expect(seen.info?["draftId"] as? String == "d1")
        #expect(seen.info?["sceneId"] as? String == "sc1")
        SearchRouting.go(SearchTarget(workspace: .story))
        #expect(seen.workspace == "story")
    }

    @Test func theCommandListHasTheProjectCommandsAndFiltersByText() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        let palette = SearchPaletteModel(app: s.app)
        let titles = palette.commands.map(\.title)
        for expected in ["New Project", "Close Project", "Go to Screenplay", "Go to Story", "Go to Idea Vault", "Settings", "About OpenFrame Studio"] {
            #expect(titles.contains(expected), "missing \(expected)")
        }
        palette.query = "story"
        let filtered = palette.rows.compactMap { row -> String? in if case let .command(c) = row { c.title } else { nil } }
        #expect(filtered.contains("Go to Story"))
        #expect(!filtered.contains("Close Project"))
        // Running Close Project closes the project.
        palette.query = ""
        let close = try #require(palette.commands.first { $0.id == "close-project" })
        await close.run()
        #expect(s.app.phase == .home)
    }

    @Test func aSearchWithNoMatchesAndTheHomePaletteStayQuiet() async throws {
        let s = try await start()
        defer { finish(s) }
        let palette = SearchPaletteModel(app: s.app)
        palette.query = "anything"
        await palette.search()
        #expect(palette.hits.isEmpty && palette.failure == nil)
        let titles = palette.commands.map(\.title)
        #expect(titles.contains("New Project") && titles.contains("Open Project\u{2026}") && !titles.contains("Close Project"))
    }

    // MARK: About

    @Test func aboutShowsTheVersionsAndSaysInternalBuildIsNotForSale() async throws {
        let s = try await start()
        defer { finish(s) }
        let about = await AboutInfo.load(app: s.app)
        #expect(about.appVersion == s.app.info?.appVersion)
        #expect(about.coreVersion == Core.version)
        #expect(about.coreVersion.split(separator: ".").count >= 3)
        #expect(about.channelName == "Internal team build")
        #expect(about.notice == "Internal build, not for sale.")
        let words = [about.appVersion, about.coreVersion, about.channelName, about.notice ?? ""].joined(separator: " ").lowercased()
        for banned in ["$", "\u{20AC}", "\u{00A3}", "\u{20B9}", "\u{00A5}", "price", "pay once", "subscription", "lifetime", "buy"] {
            #expect(!words.contains(banned), "found \(banned)")
        }
    }

    @Test func aStoreBuildShowsNoInternalNotice() {
        let store = AboutInfo.make(appVersion: "1.2.3", coreVersion: "1.2.3", channel: .macAppStore)
        #expect(store.notice == nil)
        #expect(store.channelName != "Internal team build")
        #expect(AboutInfo.make(appVersion: "1", coreVersion: "1", channel: .directInstaller).notice == AboutInfo.internalBuildNotice)
    }

    // MARK: Rendering

    @Test func screensRenderInLightAndDark() async throws {
        let s = try await start()
        defer { finish(s) }
        let original = try #require(try await s.recents().first)
        await s.actions.duplicate(original)
        let copy = try #require(try await s.recents().first { $0.projectId != original.projectId })
        await s.actions.setArchived(copy, true)
        await s.actions.duplicate(original)
        let themes = ThemeStore()
        await themes.load(from: s.app)
        ToastCenter.shared.undoable("Changed project status to Writing", app: s.app)
        for scheme in [ColorScheme.light, .dark] {
            let name = scheme == .dark ? "dark" : "light"
            let theme = themes.resolvedFor(colorScheme: scheme)
            let preset = HomeModel()
            preset.showArchived = true
            await preset.load(s.app.core)
            let home = HomeView(model: preset).overlay(alignment: .bottom) { ToastHost() }
            .environment(s.app).environment(themes).environment(\.ofTheme, theme).environment(\.colorScheme, scheme)
            #expect(Fixture.render(home, name: "m3-home-lifecycle-\(name)") != nil)

            let palette = SearchPaletteModel(app: s.app)
            palette.query = "light"
            let paletteView = SearchPaletteView(model: palette) {}
                .padding(.top, 40)
                .frame(maxHeight: .infinity, alignment: .top)
                .environment(s.app).environment(\.ofTheme, theme).environment(\.colorScheme, scheme)
                .background(theme["bg"])
            #expect(Fixture.render(paletteView, name: "m3-palette-\(name)", size: CGSize(width: 800, height: 520)) != nil)

            let about = AboutView(preset: await AboutInfo.load(app: s.app)).environment(s.app).environment(\.ofTheme, theme).environment(\.colorScheme, scheme)
            #expect(Fixture.render(about, name: "m3-about-\(name)", size: CGSize(width: 340, height: 300)) != nil)

            let rename = RenameProjectSheet(project: original, actions: s.actions) {}
                .environment(\.ofTheme, theme).environment(\.colorScheme, scheme)
            #expect(Fixture.render(rename, name: "m3-rename-\(name)", size: CGSize(width: 440, height: 220)) != nil)
            let delete = DeleteProjectSheet(project: original, actions: s.actions) {}
                .environment(\.ofTheme, theme).environment(\.colorScheme, scheme)
            #expect(Fixture.render(delete, name: "m3-delete-\(name)", size: CGSize(width: 500, height: 420)) != nil)
        }
    }

    // MARK: Project and library moves

    /// A move closes the project in the core and opens it again from the new folder. The close
    /// must not leave the window on Home for good, and the reopen must show the project again.
    @Test func aProjectTheCoreOpensAgainShowsAgain() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        guard case let .project(p) = s.app.phase else { Issue.record("no project open"); return }

        // During a move the close keeps the window on the project.
        s.app.beginRelocation()
        s.app.handle(.projectClosed(projectId: p.id))
        #expect(s.app.phase == .project(p))
        await s.app.endRelocation()
        #expect(s.app.relocations == 0)
        #expect(s.app.phase == .project(p))

        // Without one, the close goes Home and the core's reopen brings the project back.
        s.app.handle(.projectClosed(projectId: p.id))
        #expect(s.app.phase == .home)
        s.app.handle(.projectOpened(projectId: p.id))
        let deadline = Date().addingTimeInterval(5)
        while s.app.phase == .home && Date() < deadline { try await Task.sleep(for: .milliseconds(20)) }
        guard case let .project(again) = s.app.phase else { Issue.record("the reopened project did not show"); return }
        #expect(again.id == p.id)
    }

    @Test func aMoveThatEndsWithNothingOpenGoesHome() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        s.app.beginRelocation()
        _ = try await s.core.projectClose()
        if case .project = s.app.phase {} else { Issue.record("the close during a move left the project") }
        await s.app.endRelocation()
        #expect(s.app.phase == .home)
    }

    @Test func aCloseAfterAnOpenIsNeverUndoneByALateAnswer() async throws {
        let s = try await start(close: false)
        defer { finish(s) }
        guard case let .project(p) = s.app.phase else { return }
        s.app.handle(.projectOpened(projectId: p.id))
        await s.app.closeProject()
        try await Task.sleep(for: .milliseconds(300))
        #expect(s.app.phase == .home)
    }

}

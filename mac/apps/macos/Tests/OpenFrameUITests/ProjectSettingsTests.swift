// Project Settings: General form, Templates and the render of the screens.

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@Suite("Project settings", .serialized)
@MainActor
struct ProjectSettingsTests {
    private func open(_ title: String = "Night Bus") async throws -> (AppModel, URL) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        _ = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        return (app, root)
    }

    @Test func formLoadsEditsAndSaves() async throws {
        let (app, root) = try await open()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = ProjectSettingsModel(app: app)
        #expect(m.loading)
        await m.load()
        #expect(!m.loading)
        #expect(m.draft.title == "Night Bus")
        #expect(!m.dirty && !m.canSave)
        m.draft.title = "  Night Bus 2  "
        m.draft.genre = "Thriller"
        m.draft.creator = "A. Writer"
        m.draft.logline = "A bus, a night."
        m.draft.projectNotes = "Shoot in March"
        m.draft.status = .writing
        #expect(m.dirty && m.canSave)
        #expect(await m.save())
        let p = try await app.core!.projectGetSettings()
        #expect(p.project.title == "Night Bus 2")
        #expect(p.project.genre == "Thriller")
        #expect(p.project.creator == "A. Writer")
        #expect(p.project.logline == "A bus, a night.")
        #expect(p.project.status == "Writing")
        #expect(p.projectNotes == "Shoot in March")
        #expect(!m.dirty, "the form matches the stored project after saving")
    }

    @Test func emptyTitleCannotBeSavedAndTypeChangeSaysNothingIsDeleted() async throws {
        let (app, root) = try await open()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = ProjectSettingsModel(app: app)
        await m.load()
        m.draft.title = "   "
        #expect(m.titleMissing && !m.canSave)
        #expect(await m.save() == false)
        #expect(m.showTitleError)
        #expect(try await app.core!.projectGetSettings().project.title == "Night Bus")
        m.discard()
        #expect(!m.dirty)
        #expect(m.typeHint == nil)
        m.draft.projectType = .episodic
        #expect(m.typeHint == "Changing the type never deletes anything you have written.")
        m.draft.projectType = .shortFilm
        #expect(m.typeHint == nil)
    }

    @Test func newProjectsAreFilmsAndAnOldEpisodicProjectKeepsItsType() {
        #expect(ProjectSettingsDraft.offeredTypes == [.featureFilm, .shortFilm])
        #expect(ProjectSettingsDraft.typeChoices(current: .shortFilm) == [.featureFilm, .shortFilm])
        #expect(ProjectSettingsDraft.typeChoices(current: .series) == [.featureFilm, .shortFilm, .series])
    }

    @Test func clearingNotesAndFieldsStoresEmpty() async throws {
        let (app, root) = try await open()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = ProjectSettingsModel(app: app)
        await m.load()
        m.draft.projectNotes = "x"; m.draft.genre = "Drama"
        #expect(await m.save())
        m.draft.projectNotes = ""; m.draft.genre = ""
        #expect(await m.save())
        let p = try await app.core!.projectGetSettings()
        #expect((p.projectNotes ?? "").isEmpty)
        #expect((p.project.genre ?? "").isEmpty)
    }

    @Test func templatesListCreateEditCopyAndDelete() async throws {
        let (app, root) = try await open()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let t = TemplatesModel(app: app)
        await t.load()
        let listing = try #require(t.listing)
        #expect(!listing.builtin.isEmpty)
        #expect(listing.project.isEmpty && listing.global.isEmpty)

        #expect(await t.save(existing: nil, name: "  ", type: "call_sheet", scope: .project, description: "", lines: "") == false)
        #expect(await t.save(existing: nil, name: "My call sheet", type: "call_sheet", scope: .project, description: "One page", lines: "Weather\n\n Crew \n"))
        let mine = try #require(t.listing?.project.first)
        #expect(mine.name == "My call sheet")
        #expect(TemplateKinds.list(mine.content, key: "sections") == ["Weather", "Crew"])

        #expect(await t.save(existing: mine, name: "My call sheet 2", type: "call_sheet", scope: .project, description: "", lines: "Weather"))
        #expect(t.listing?.project.first?.name == "My call sheet 2")
        #expect(t.listing?.project.first?.content["description"] == nil)

        #expect(await t.keepForAll(mine))
        #expect(t.listing?.global.count == 1)
        let builtIn = try #require(t.listing?.builtin.first)
        #expect(await t.copyIntoProject(builtIn))
        #expect(t.listing?.project.count == 2)

        let globalOne = try #require(t.listing?.global.first)
        #expect(await t.remove(globalOne))
        #expect(t.listing?.global.isEmpty == true)
        #expect(await t.remove(mine))
        #expect(t.listing?.project.count == 1)
    }

    @Test func templateTextHelpersMatchWindows() {
        #expect(TemplateKinds.itemText(.string("Plain")) == "Plain")
        #expect(TemplateKinds.itemText(.object(["category": "Cast", "description": "Lead"])) == "Cast: Lead")
        #expect(TemplateKinds.itemText(.object(["description": "Wide", "size": "WS"])) == "Wide (WS)")
        #expect(TemplateKinds.itemText(.object(["heading": "INT. BUS - NIGHT"])) == "INT. BUS - NIGHT")
        let c = TemplateKinds.content(type: "shot_list", description: " d ", lines: "a\n\nb", base: ["keep": "me"])
        #expect(c["shots"] == .array(["a", "b"]))
        #expect(c["description"] == "d")
        #expect(c["keep"] == "me")
        #expect(TemplateKinds.content(type: "shot_list", description: " ", lines: "", base: ["description": "old"])["description"] == nil)
        #expect(TemplateKinds.createLabel("budget") == "Add budget lines")
        #expect(TemplateKinds.creates.contains("call_sheet") && !TemplateKinds.creates.contains("report"))
    }

    @Test func aTemplateCreatesItsDocument() async throws {
        let (app, root) = try await open()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let t = TemplatesModel(app: app)
        await t.load()
        let story = try #require(t.listing?.builtin.first { $0.templateType == "story_board" })
        let done = await t.apply(story, dayId: nil, sceneId: nil)
        #expect(done != nil)
        #expect(done?.summary.isEmpty == false)
        // Without a schedule there is no shooting day, and no scenes without a screenplay.
        #expect(await t.shootingDaysWithoutCallSheet().isEmpty)
        #expect(await t.scenes().isEmpty)
    }

    @Test func screensRender() async throws {
        let (app, root) = try await open()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let themes = ThemeStore()
        func shown<V: View>(_ v: V, _ name: String) -> NSImage? {
            Fixture.render(v.environment(app).environment(themes).environment(\.ofTheme, themes.resolvedFor(colorScheme: .light)),
                           name: name, size: CGSize(width: 920, height: 680))
        }
        #expect(shown(ProjectSettingsView(initialTab: .general), "project-settings-general") != nil)
        #expect(shown(ProjectSettingsView(initialTab: .templates), "project-settings-templates") != nil)
        #expect(shown(ProjectSettingsView(initialTab: .advanced), "project-settings-advanced") != nil)
        #expect(shown(ProjectSettingsView(initialTab: .shortcuts), "project-settings-shortcuts") != nil)
        #expect(shown(ProjectSettingsView(initialTab: .storage), "project-settings-storage") != nil)
        #expect(shown(ShortcutsSheet(app: app), "shortcuts-sheet") != nil)
    }

    /// Project Settings has a Storage page (Windows: Settings > Storage inside a project), and the
    /// General page links to it.
    @Test func storageIsAProjectSettingsPage() {
        #expect(ProjectSettingsTab.allCases.last == .storage)
        #expect(ProjectSettingsTab.storage.title == "Storage")
        #expect(ProjectSettingsTab(rawValue: "storage") == .storage)
    }

}

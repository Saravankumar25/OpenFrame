// Custom themes: paste, check, save, rename, apply, delete (scenario 91; Windows ThemeImportDialog).

import Foundation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

@Suite("Settings themes", .serialized)
@MainActor
struct SettingsThemesTests {
    private func start() async throws -> (AppModel, ThemeStore, URL) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        return (app, ThemeStore(), root)
    }

    /// A theme file built from a built-in theme's seeds, with a new name.
    private func themeText(_ app: AppModel, name: String) async throws -> String {
        let base = try #require(ThemeStore().catalog.theme(id: "studio-amber"))
        let object: [String: Any] = [
            "formatVersion": 1, "id": "my-" + name.lowercased().replacingOccurrences(of: " ", with: "-"),
            "name": name, "description": "Made for the test", "author": "Tester",
            "seeds": [
                "neutralHue": base.seeds.neutralHue, "neutralTint": base.seeds.neutralTint,
                "accentHue": base.seeds.accentHue, "accentChroma": base.seeds.accentChroma,
                "accentLightnessNudge": base.seeds.accentLightnessNudge,
            ],
        ]
        return String(decoding: try JSONSerialization.data(withJSONObject: object), as: UTF8.self)
    }

    @Test func pasteCheckSaveAndApply() async throws {
        let (app, themes, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let text = try await themeText(app, name: "Bus Depot")
        let model = ThemeImportModel(source: .text, app: app, themes: themes)
        #expect(!model.canCheck)
        model.text = text
        #expect(model.canCheck)
        await model.check()
        let result = try #require(model.result)
        #expect(result.problems.isEmpty)
        #expect(result.status != ThemeCheckStatus.rejected)
        #expect(model.usable)
        #expect(model.previewTheme?.name == "Bus Depot")
        #expect(!model.summary.isEmpty)
        // Nothing is stored before the person saves.
        #expect(try await app.core!.appearanceGet().myThemes.isEmpty)
        #expect(await model.save(apply: true))
        let state = try await app.core!.appearanceGet()
        #expect(state.myThemes.map(\.name) == ["Bus Depot"])
        #expect(state.themeId == state.myThemes.first?.id)
        #expect(themes.themeId == state.themeId)
        #expect(model.savedMessage == "Bus Depot is saved and applied.")
    }

    @Test func badTextIsRejectedAndNothingIsSaved() async throws {
        let (app, themes, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let model = ThemeImportModel(source: .text, app: app, themes: themes)
        model.text = "this is not a theme"
        await model.check()
        #expect(model.result?.status == ThemeCheckStatus.rejected || model.error != nil)
        #expect(!model.usable)
        #expect(await model.save(apply: false) == false)
        #expect(try await app.core!.appearanceGet().myThemes.isEmpty)
    }

    @Test func textOverTenKilobytesCannotBeChecked() async throws {
        let (app, themes, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let model = ThemeImportModel(source: .text, app: app, themes: themes)
        model.text = String(repeating: "x", count: ThemeImportModel.maxBytes + 1)
        #expect(model.tooLarge)
        #expect(!model.canCheck)
        model.text = "   \n"
        #expect(!model.canCheck, "blank text")
    }

    @Test func editTheTextGoesBackToThePasteField() async throws {
        let (app, themes, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let model = ThemeImportModel(source: .text, app: app, themes: themes)
        model.text = try await themeText(app, name: "Again")
        await model.check()
        #expect(model.result != nil)
        model.editText()
        #expect(model.result == nil)
    }

    @Test func renameApplyExportAndDelete() async throws {
        let (app, themes, root) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let imp = ThemeImportModel(source: .text, app: app, themes: themes)
        imp.text = try await themeText(app, name: "First Look")
        await imp.check()
        #expect(await imp.save(apply: false))

        let mine = MyThemesModel(app: app, themes: themes)
        await mine.reload()
        let theme = try #require(mine.mine.first)
        #expect(mine.activeId != theme.id)

        #expect(await mine.rename(theme, to: "  Second Look  "))
        #expect(mine.mine.first?.name == "Second Look")
        #expect(await mine.rename(theme, to: "   ") == false)
        #expect(await mine.rename(theme, to: String(repeating: "a", count: 61)) == false)

        await mine.apply(theme)
        #expect(mine.activeId == theme.id)
        #expect(themes.themeId == theme.id)

        let path = FileManager.default.temporaryDirectory.appendingPathComponent("exp-\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: path) }
        await mine.export(theme, to: path.path)
        #expect(FileManager.default.fileExists(atPath: path.path))

        await mine.delete(theme)
        #expect(mine.mine.isEmpty)
        #expect(mine.activeId == "openframe-default", "deleting the theme in use applies the default")
        #expect(themes.themeId == "openframe-default")
    }
}

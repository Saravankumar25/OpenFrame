// Download OpenFrame AI and the Offline AI settings, with a real core and no network: nothing is
// downloaded here. Covers the not-installed state before the click, remove, import of a bad file,
// the health card, Export AI report (scenarios 76 and 78 without a host).

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@Suite("AI download and settings", .serialized)
@MainActor
struct AIDownloadTests {
    private func start() async throws -> (URL, AppModel) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        return (root, app)
    }

    @Test func beforeTheClickTheCardShowsSizeAndSpaceAndAsksNothing() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        let d = try #require(m.download)
        #expect(d.phase == "Idle")
        #expect(d.canDownload)
        #expect(d.sizeBytes > 0 && d.neededBytes >= d.sizeBytes)
        #expect(!m.isRunning && !m.busy && m.error == nil)
        let line = AILogic.downloadLine(d)
        #expect(line.hasPrefix("A one-time download of "))
        #expect(line.contains("of free space"))
        // Still idle on a second read: reading never starts a download.
        await m.refresh()
        #expect(m.download?.phase == "Idle")
    }

    @Test func importingAFileThatIsNotAModelGivesAPlainMessage() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        let bogus = root.appendingPathComponent("not-a-model.gguf")
        try Data("hello".utf8).write(to: bogus)
        await m.importFile(at: bogus)
        let message = try #require(m.error)
        #expect(!message.isEmpty && !message.contains("internal"))
        #expect(!m.busy)
        // The file is copied, never moved or changed.
        #expect(try Data(contentsOf: bogus) == Data("hello".utf8))
        #expect(m.download?.phase != "Done")
    }

    @Test func importingAFileThatIsMissingGivesAPlainMessage() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        await m.importFile(at: root.appendingPathComponent("gone.gguf"))
        #expect(m.error?.isEmpty == false)
    }

    @Test func cancellingThePickerChangesNothing() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        m.pickFile = { nil }
        await m.importFile()
        #expect(m.error == nil && !m.busy)
    }

    @Test func removeWithNothingDownloadedIsSafeAndReloads() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        _ = await m.remove()
        #expect(!m.busy)
        #expect(m.download?.phase == "Idle")
    }

    @Test func pauseWhenNothingRunsDoesNotHangOrCrash() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        await m.pause()
        #expect(!m.busy)
        #expect(m.download != nil)
    }

    // MARK: Settings, health card, Export AI report

    @Test func settingsLoadsStatusDiagnosticsAndHealth() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let s = AISettingsModel(app: app)
        await s.load()
        #expect(s.error == nil)
        let status = try #require(s.status)
        #expect(status.localOnly)
        let diag = try #require(s.diagnostics)
        #expect(!diag.components.isEmpty)
        #expect(diag.components.allSatisfy { $0.sha256.count == 64 })
        #expect(s.health?.requests == 0)
        #expect(AILogic.stateText(status) == "Needs a reinstall" || AILogic.stateText(status) == "Not downloaded")
    }

    @Test func graphicsChoicePersists() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let s = AISettingsModel(app: app)
        await s.setGraphics(off: true)
        #expect(s.diagnostics?.graphicsOff == true)
        await s.setGraphics(off: false)
        #expect(s.diagnostics?.graphicsOff == false)
        #expect(s.error == nil)
    }

    @Test func exportAIReportWritesTheFileThePersonChose() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let s = AISettingsModel(app: app)
        let file = root.appendingPathComponent("report.json")
        s.chooseSaveLocation = { _, _ in file }
        let r = await s.exportReport()
        #expect(r != nil)
        #expect(FileManager.default.fileExists(atPath: file.path))
        #expect(s.lastExport?.hasPrefix("Saved ") == true)
        #expect(!s.busy)
        // Cancelling the panel does nothing.
        s.chooseSaveLocation = { _, _ in nil }
        #expect(await s.exportReport() == nil)
    }

    @Test func exportToAPlaceThatCannotBeWrittenSaysSo() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let s = AISettingsModel(app: app)
        s.chooseSaveLocation = { _, _ in URL(fileURLWithPath: "/nonexistent-folder/report.json") }
        #expect(await s.exportReport() == nil)
        #expect(s.error?.isEmpty == false)
        #expect(!s.busy)
    }

    @Test func removeFromSettingsReloadsTheState() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let s = AISettingsModel(app: app)
        s.removing = true
        await s.remove()
        #expect(!s.removing && !s.busy)
        #expect(s.status != nil)
    }

    @Test func settingsAndTheDownloadCardDraw() async throws {
        let (root, app) = try await start()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let themes = ThemeStore()
        for scheme in [ColorScheme.light, .dark] {
            let settings = AISettingsView()
                .environment(app).environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
                .environment(\.colorScheme, scheme)
            #expect(Fixture.render(settings, name: "ai-settings-\(scheme)", size: CGSize(width: 640, height: 700)) != nil)
        }
        let m = AIDownloadModel(app: app)
        await m.refresh()
        let card = AIDownloadCard(model: m, damaged: "An OpenFrame AI file is missing or damaged. Download it again.")
            .environment(app).environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
        #expect(Fixture.render(card, name: "ai-download-damaged", size: CGSize(width: 360, height: 420)) != nil)
    }
}

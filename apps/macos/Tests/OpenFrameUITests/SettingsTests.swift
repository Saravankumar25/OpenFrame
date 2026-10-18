// Settings: choices persist through the real core, and the Appearance tab draws.

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@MainActor
private func startedModel() async throws -> (AppModel, URL) {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("openframe-settings-tests", isDirectory: true)
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
    let core = try await Core.start(options)
    return (AppModel(core: core, options: options), root)
}

@Suite("Settings", .serialized)
@MainActor
struct SettingsTests {
    @Test func everyThemeChoicePersists() async throws {
        let (app, root) = try await startedModel()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let themes = ThemeStore()
        #expect(themes.themes.count >= 16)
        for item in themes.themes {
            try await themes.choose(themeId: item.id, core: app.core)
            let saved = try await app.core!.appearanceGet()
            #expect(saved.themeId == item.id, "theme \(item.id) did not persist")
            #expect(themes.themeId == item.id)
        }
    }

    @Test func modePersists() async throws {
        let (app, root) = try await startedModel()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let themes = ThemeStore()
        for mode in [AppearancePreference.dark, .light, .system] {
            try await themes.choose(mode: mode, core: app.core)
            let saved = try await app.core!.appearanceGet()
            #expect(saved.mode.rawValue == mode.rawValue)
        }
    }

    @Test func displayNameRoundTripsAndBadNameIsRefused() async throws {
        let (app, root) = try await startedModel()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let core = app.core!
        let profile = try await core.appSetDisplayName(DisplayNameArgs(displayName: "Asha Rao"))
        #expect(profile.displayName == "Asha Rao")
        #expect(try await core.appInfo().profile.displayName == "Asha Rao")
        do {
            _ = try await core.appSetDisplayName(DisplayNameArgs(displayName: "   "))
            Issue.record("an empty name was accepted")
        } catch {
            #expect(!CoreFailure(asCore: error).message.isEmpty)
        }
    }

    @Test func diagnosticsPreviewLoads() async throws {
        let (app, root) = try await startedModel()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let preview = try await app.core!.diagnosticsPreview()
        #expect(!preview.suggestedFileName.isEmpty)
        #expect(!preview.neverIncluded.isEmpty)
    }

    @Test func appearanceTabRenders() async throws {
        let (app, root) = try await startedModel()
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        let themes = ThemeStore()
        themes.apply(try await app.core!.appearanceGet())
        let resolved = themes.resolvedFor(colorScheme: .light)

        let view = AppearanceSettingsContent()
            .environment(app)
            .environment(themes)
            .environment(\.ofTheme, resolved)
            .frame(width: 640, height: 1100, alignment: .top)
            .background(resolved["bg"])
        let renderer = ImageRenderer(content: view)
        renderer.scale = 2
        let image = renderer.nsImage
        #expect(image != nil)
        #expect((image?.size.width ?? 0) > 300)

        if let image, let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
           let png = rep.representation(using: .png, properties: [:]) {
            let folder = URL(fileURLWithPath: "/private/tmp/claude-501/-Users-m-udarshreddy-Desktop-OpenFrame/78b64903-9d1c-4e08-a663-f47a7233f6c5/scratchpad/snapshots", isDirectory: true)
            try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try? png.write(to: folder.appendingPathComponent("settings-appearance.png"))
        }
    }
}

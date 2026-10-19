// The licence screens drawn in a real window, light and dark, to prove each one builds and draws
// something real, plus the app model wiring: the store build names its first screen at launch and
// the internal build never does. A picture is not text, so what the screens say is proved by the
// model tests in StoreLicenceTests; the pictures are written for a person to look at.

import AppKit
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@MainActor
private func drawn<V: View>(_ view: V, app: AppModel, scheme: ColorScheme, name: String, size: CGSize = CGSize(width: 640, height: 640)) async throws -> Int {
    let themes = ThemeStore()
    let hosted = view.environment(app).environment(themes)
        .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme)).environment(\.colorScheme, scheme)
    let host = NSHostingView(rootView: hosted.frame(width: size.width, height: size.height))
    host.frame = NSRect(origin: .zero, size: size)
    let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
    window.contentView = host
    window.appearance = NSAppearance(named: scheme == .dark ? .darkAqua : .aqua)
    host.layoutSubtreeIfNeeded()
    for _ in 0..<8 {
        try await Task.sleep(for: .milliseconds(50))
        host.layoutSubtreeIfNeeded()
    }
    guard let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else {
        Issue.record("could not draw \(name)")
        return 0
    }
    host.cacheDisplay(in: host.bounds, to: rep)
    try FileManager.default.createDirectory(at: Fixture.snapshots, withIntermediateDirectories: true)
    if let png = rep.representation(using: .png, properties: [:]) {
        try png.write(to: Fixture.snapshots.appendingPathComponent("\(name).png"))
    }
    var seen = Set<UInt32>()
    let step = max(1, min(rep.pixelsWide, rep.pixelsHigh) / 60)
    for x in stride(from: 0, to: rep.pixelsWide, by: step) {
        for y in stride(from: 0, to: rep.pixelsHigh, by: step) {
            if let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) {
                // The alpha counts too: a transparent corner and black text are both (0, 0, 0).
                seen.insert(UInt32(c.alphaComponent * 255) << 24 | UInt32(c.redComponent * 255) << 16 | UInt32(c.greenComponent * 255) << 8 | UInt32(c.blueComponent * 255))
            }
        }
    }
    return seen.count
}

@Suite("Store screens", .serialized)
@MainActor
struct StoreScreenTests {
    @Test func everyLicenceScreenDrawsInLightAndDark() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        let app = AppModel(core: rig.core, options: CoreStartOptions(appVersion: "0.1.0-test", writeLogs: false))
        let screens: [(String, LicenceScreen)] = [
            ("reminder", .reminder(daysLeft: 2)), ("data-note", .dataNote), ("tutorial", .tutorial),
            ("welcome", .welcome), ("mac-first-launch", .macFirstLaunch), ("locked", .locked(revoked: false)),
            ("locked-revoked", .locked(revoked: true)), ("check-needed", .checkNeeded),
        ]
        for (name, screen) in screens {
            for scheme in [ColorScheme.light, .dark] {
                let colours = try await drawn(
                    LicenceScreenView(screen: screen, licence: rig.licence), app: app, scheme: scheme,
                    name: "licence-\(name)-\(scheme == .dark ? "dark" : "light")"
                )
                #expect(colours > 3, "\(name) \(scheme) drew \(colours) colours")
            }
        }
    }

    @Test func theTrialCornerDrawsOnlyInAStoreBuild() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        let app = AppModel(core: rig.core, options: CoreStartOptions(appVersion: "0.1.0-test", writeLogs: false))
        let colours = try await drawn(TrialCorner(licence: rig.licence), app: app, scheme: .light, name: "licence-trial-corner", size: CGSize(width: 260, height: 40))
        #expect(colours > 1)
        #expect(rig.licence.chip != nil)
        let internalRig = try await StoreRig.start(store: false)
        defer { internalRig.finish() }
        #expect(internalRig.licence.chip == nil)
        #expect(!internalRig.licence.showsCheckPurchase)
    }

    @Test func theAppModelNamesTheFirstScreenOfTheStoreBuildAtLaunch() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        let app = AppModel(core: rig.core, options: CoreStartOptions(appVersion: "0.1.0-test", writeLogs: false))
        app.storeRuntime = rig.runtime
        await app.start()
        #expect(app.phase == .home)
        #expect(app.licence.screen == .macFirstLaunch)
        app.shutdown()
    }

    @Test func theAppModelOfTheInternalBuildHasNoLicenceScreenAndKeepsTheTeamGate() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let source = FakeStoreKitSource()
        let runtime = StoreRuntime(source: source, server: FakeLicenceServer())
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        runtime.install(into: &options)
        options.testTeamPasswordHash = Fixture.gateHash
        let app = AppModel(options: options)
        app.storeRuntime = runtime
        await app.start()
        guard case .teamGate = app.phase else {
            Issue.record("expected the team password screen, got \(app.phase)")
            return
        }
        #expect(app.licence.screen == nil)
        let answer = await app.unlock(password: Fixture.gatePassword)
        #expect(answer == nil)
        #expect(app.phase == .home)
        #expect(app.licence.screen == nil)
        #expect(!app.licence.isStore)
        #expect(source.calls == 0)
        app.shutdown()
    }
}

// The Settings buttons are on screen at the default window width (1360 points), with their
// names, and not in the toolbar's overflow menu.

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@MainActor
private func hostedWindow<V: View>(_ view: V, width: CGFloat = 1360, height: CGFloat = 860) -> NSWindow {
    _ = NSApplication.shared
    let controller = NSHostingController(rootView: view)
    let window = NSWindow(contentViewController: controller)
    window.styleMask = [.titled, .closable, .resizable, .miniaturizable]
    window.setContentSize(NSSize(width: width, height: height))
    window.toolbarStyle = .unified
    window.orderFrontRegardless()
    RunLoop.main.run(until: Date().addingTimeInterval(1.5))
    return window
}

/// The toolbar items the person can see: in the window and not folded into the overflow menu.
@MainActor
private func visibleToolbarLabels(_ window: NSWindow) -> [String] {
    (window.toolbar?.visibleItems ?? []).map(\.label)
}

@Suite("Settings toolbar", .serialized)
@MainActor
struct SettingsToolbarTests {
    private func env<V: View>(_ view: V, model: AppModel, themes: ThemeStore) -> some View {
        view
            .environment(model)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
    }

    @Test func homeShowsSettingsWithItsNameAtDefaultWidth() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let model = Fixture.model(root)
        await model.start()
        let themes = ThemeStore()
        let window = hostedWindow(env(HomeView(), model: model, themes: themes), width: 1360)
        defer { window.close(); model.shutdown() }

        let labels = visibleToolbarLabels(window)
        #expect(labels.contains("Settings"), "visible: \(labels)")
        let all = (window.toolbar?.items ?? []).map(\.label)
        #expect(Set(all).isSubset(of: Set(labels).union([""])), "folded into overflow: \(Set(all).subtracting(labels))")
        // Its own view sits inside the window and shows the word, not only the gear.
        let item = try #require(window.toolbar?.items.first { $0.label == "Settings" })
        let view = try #require(item.view)
        let frame = view.convert(view.bounds, to: nil)
        #expect(frame.maxX <= window.frame.width && frame.minX >= 0, "item frame \(frame) in window \(window.frame)")
        // Every button in the Home toolbar shows its name next to its icon (an icon alone is
        // about 30 points wide).
        for name in ["Global Idea Vault", "Settings"] {
            let v = try #require(window.toolbar?.items.first { $0.label == name }?.view, "\(name) in the toolbar")
            #expect(v.frame.width > 70, "\(name) is \(v.frame.width) wide: only its icon shows")
        }
    }

    /// A 1280-point window (a 1280 by 720 screen): every project button stays in the toolbar with
    /// its name, none folded into the overflow menu.
    @Test func projectToolbarKeepsEveryNamedButtonOnALaptopScreen() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let model = Fixture.model(root)
        await model.start()
        _ = await model.createProject(CreateProjectArgs(title: "Night Bus", projectType: .featureFilm))
        guard case let .project(project) = model.phase else {
            Issue.record("no project open")
            return
        }
        let themes = ThemeStore()
        let window = hostedWindow(env(ProjectView(project: project), model: model, themes: themes), width: 1280)
        defer { window.close(); model.shutdown() }

        let labels = visibleToolbarLabels(window)
        #expect(labels.contains("Settings"), "visible: \(labels)")
        #expect(labels.contains("Assistant"), "visible: \(labels)")
        let all = (window.toolbar?.items ?? []).map(\.label)
        #expect(Set(all).isSubset(of: Set(labels).union([""])), "folded into overflow: \(Set(all).subtracting(labels))")
        let item = try #require(window.toolbar?.items.first { $0.label == "Settings" })
        let view = try #require(item.view)
        let frame = view.convert(view.bounds, to: nil)
        #expect(frame.maxX <= window.frame.width && frame.minX >= 0, "item frame \(frame) in window \(window.frame)")
        #expect(view.frame.width > 70, "Settings is \(view.frame.width) wide: only its icon shows")
    }
}

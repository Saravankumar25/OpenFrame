// The app: one main window, the standard Mac menus, and Settings (Command+Comma).

import AppKit
import SwiftUI
import OpenFrameCore

/// Version shown in About and written into project files.
public enum AppVersion {
    public static var current: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? Core.version
    }
}

/// Start options from the process environment. `OPENFRAME_DATA_ROOT` keeps every file of a test
/// run in one absolute folder (ADR-0031), as on Windows; nothing else can be changed from outside.
enum LaunchOptions {
    static func make() -> CoreStartOptions {
        let env = ProcessInfo.processInfo.environment
        let root = env["OPENFRAME_DATA_ROOT"].flatMap { raw -> URL? in
            raw.hasPrefix("/") ? URL(fileURLWithPath: raw, isDirectory: true) : nil
        }
        if env["OPENFRAME_DATA_ROOT"] != nil && root == nil {
            // A test folder was asked for but is not absolute: never fall back to real data.
            FileHandle.standardError.write(Data("OPENFRAME_DATA_ROOT must be an absolute folder; OpenFrame Studio stops.\n".utf8))
            exit(2)
        }
        let ai = Bundle.main.resourceURL?.appendingPathComponent("ai", isDirectory: true)
        return CoreStartOptions(appVersion: AppVersion.current, isolatedDataRoot: root, aiBundleDir: ai, writeLogs: true)
    }
}

/// Clean shutdown and sleep notifications, which SwiftUI does not expose.
final class AppDelegate: NSObject, NSApplicationDelegate {
    @MainActor static var model: AppModel?
    private var observers: [NSObjectProtocol] = []

    func applicationDidFinishLaunching(_ notification: Notification) {
        let center = NSWorkspace.shared.notificationCenter
        observers.append(center.addObserver(forName: NSWorkspace.willSleepNotification, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { AppDelegate.model?.core?.powerEvent(sleeping: true) }
        })
        observers.append(center.addObserver(forName: NSWorkspace.didWakeNotification, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { AppDelegate.model?.core?.powerEvent(sleeping: false) }
        })
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    /// Command+Q, the Dock and logout: store what the editors still hold, then quit (the Mac
    /// counterpart of the Windows close gate). A store that does not answer in 5 seconds does not
    /// keep the app open forever; the core closes the project either way.
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        Task { @MainActor in
            let flush = Task { await ScreenplayWorkspaceModel.flushAll() }
            let timeout = Task { try? await Task.sleep(nanoseconds: 5_000_000_000) }
            _ = await (Task { await withTaskGroup(of: Void.self) { g in
                g.addTask { await flush.value }
                g.addTask { await timeout.value }
                await g.next()
                g.cancelAll()
            } }).value
            sender.reply(toApplicationShouldTerminate: true)
        }
        return .terminateLater
    }

    func applicationWillTerminate(_ notification: Notification) {
        MainActor.assumeIsolated {
            // Editors store their text on every change; this closes the project cleanly
            // (checkpoint, crash marker cleared) before the process ends.
            AppDelegate.model?.shutdown()
        }
    }
}

public struct OpenFrameStudioApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var model: AppModel
    @State private var themes: ThemeStore

    public init() {
        let model = AppModel(options: LaunchOptions.make())
        _model = State(initialValue: model)
        _themes = State(initialValue: ThemeStore())
        AppDelegate.model = model
    }

    public var body: some Scene {
        WindowGroup("OpenFrame Studio", id: "main") {
            RootView()
                .environment(model)
                .environment(themes)
                .openFrameTheme(themes)
                .frame(minWidth: 900, minHeight: 600)
                .task { await model.start() }
                .task(id: model.isRunning) { await themes.load(from: model) }
        }
        .windowToolbarStyle(.unified)
        .commands { AppCommands(model: model) }

        Window("About OpenFrame Studio", id: "about") {
            AboutView()
                .environment(model)
                .environment(themes)
                .openFrameTheme(themes)
        }
        .windowResizability(.contentSize)
        .windowToolbarStyle(.unifiedCompact)

        Settings {
            SettingsView()
                .environment(model)
                .environment(themes)
                .openFrameTheme(themes)
        }
    }
}

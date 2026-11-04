// The app: one main window, the standard Mac menus, and Settings (Command+Comma).

import AppKit
import SwiftUI
import StraftCore

/// Version shown in About and written into project files.
public enum AppVersion {
    public static var current: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? Core.version
    }
}

/// Start options from the process environment. `STRAFT_DATA_ROOT` keeps every file of a test
/// run in one absolute folder (ADR-0031), as on Windows. `STRAFT_LIBRARY_SETUP=1` and
/// `STRAFT_AI_DETERMINISTIC=1` are the Windows test switches; the core honours them only in a
/// debug build on such a test folder.
enum LaunchOptions {
    static func make() -> CoreStartOptions {
        let env = ProcessInfo.processInfo.environment
        let root = env["STRAFT_DATA_ROOT"].flatMap { raw -> URL? in
            raw.hasPrefix("/") ? URL(fileURLWithPath: raw, isDirectory: true) : nil
        }
        if env["STRAFT_DATA_ROOT"] != nil && root == nil {
            // A test folder was asked for but is not absolute: never fall back to real data.
            FileHandle.standardError.write(Data("STRAFT_DATA_ROOT must be an absolute folder; Straft Studio stops.\n".utf8))
            exit(2)
        }
        // The store build keeps the engine in a code location (Contents/Helpers/ai).
        let helpers = Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/ai", isDirectory: true)
        let ai = FileManager.default.fileExists(atPath: helpers.path)
            ? helpers : Bundle.main.resourceURL?.appendingPathComponent("ai", isDirectory: true)
        var options = CoreStartOptions(appVersion: AppVersion.current, isolatedDataRoot: root, aiBundleDir: ai, writeLogs: true)
        options.testLibrarySetup = env["STRAFT_LIBRARY_SETUP"] == "1"
        options.testAIDeterministic = env["STRAFT_AI_DETERMINISTIC"] == "1"
        options.testStoreChannel = env["STRAFT_TEST_STORE"] == "1"
        return options
    }

    /// The App Store's calls. In a debug build the store scenarios use a scripted stand-in, and only
    /// on a test folder.
    static func storeRuntime(_ options: CoreStartOptions) -> StoreRuntime {
        #if DEBUG
        if options.testStoreChannel, let root = options.isolatedDataRoot {
            let source = ScriptedStoreKitSource(file: root.appendingPathComponent("fake-store.json"))
            return StoreRuntime(
                source: source, keychain: FileGuardKeychain(file: root.appendingPathComponent("fake-keychain.json")),
                clock: { source.now() }
            )
        }
        #endif
        return StoreRuntime()
    }
}

/// Clean shutdown and sleep notifications, which SwiftUI does not expose.
final class AppDelegate: NSObject, NSApplicationDelegate {
    @MainActor static var model: AppModel?
    private var observers: [NSObjectProtocol] = []

    func applicationWillFinishLaunching(_ notification: Notification) {
        SidebarRestore.clearCollapsed()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let center = NSWorkspace.shared.notificationCenter
        observers.append(center.addObserver(forName: NSWorkspace.willSleepNotification, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { AppDelegate.model?.core?.powerEvent(sleeping: true) }
        })
        observers.append(center.addObserver(forName: NSWorkspace.didWakeNotification, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { AppDelegate.model?.core?.powerEvent(sleeping: false) }
        })
        // Quit from the Dock, logout and restart close the sheets first, like Command+Q.
        MainActor.assumeIsolated { QuitGate.installQuitEventHandler() }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    /// Command+Q, the Dock and logout: store what the editors still hold, then quit (the Mac
    /// counterpart of the Windows close gate). A store that does not answer in 5 seconds does not
    /// keep the app open forever; the core closes the project either way. AppKit asks this only
    /// once no sheet is open, which `QuitGate` sees to.
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        // Whichever comes first answers: the editors finished storing, or 5 seconds passed. A
        // task group cannot do this, because it waits for the slower child even after
        // cancelling it, so a store that never answers would keep the app open for ever.
        let reply = TerminateReply(sender)
        Task { @MainActor in
            await ScreenplayWorkspaceModel.flushAll()
            reply.answer()
        }
        // A timer, not the main queue: while AppKit waits for the answer it runs the event
        // loop in the modal panel mode, which timers added for all common modes still reach.
        let timer = Timer(timeInterval: 5, repeats: false) { _ in MainActor.assumeIsolated { reply.answer() } }
        RunLoop.main.add(timer, forMode: .common)
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

/// Answers AppKit's "may the app quit?" exactly once.
@MainActor
private final class TerminateReply {
    private let app: NSApplication
    private var answered = false
    init(_ app: NSApplication) { self.app = app }
    func answer() {
        guard !answered else { return }
        answered = true
        app.reply(toApplicationShouldTerminate: true)
    }
}

public struct StraftStudioApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var model: AppModel
    @State private var themes: ThemeStore

    public init() {
        // First, before any screen or @AppStorage reads a preference.
        LegacyDefaultsMigration.runAtLaunch()
        // The StoreKit bridge goes to the core; only the Mac App Store build uses it. The internal
        // disk image build ignores it.
        var options = LaunchOptions.make()
        let runtime = LaunchOptions.storeRuntime(options)
        runtime.install(into: &options)
        let model = AppModel(options: options)
        model.storeRuntime = runtime
        #if DEBUG
        if options.testStoreChannel, let root = options.isolatedDataRoot {
            // The scenarios read what would have opened in the browser.
            let opened = root.appendingPathComponent("opened-addresses.txt")
            model.licence.openAddress = { url in
                let before = (try? String(contentsOf: opened, encoding: .utf8)) ?? ""
                try? (before + url.absoluteString + "\n").write(to: opened, atomically: true, encoding: .utf8)
            }
        }
        #endif
        _model = State(initialValue: model)
        _themes = State(initialValue: ThemeStore())
        AppDelegate.model = model
    }

    public var body: some Scene {
        WindowGroup("Straft Studio", id: "main") {
            RootView()
                .settingsOpener()
                .environment(model)
                .environment(themes)
                .straftTheme(themes)
                .fitsScreen()
                .task { await model.start() }
                .task(id: model.isRunning) { await themes.load(from: model) }
        }
        // The first window is wide enough for every workspace toolbar; at the minimum width the
        // toolbar buttons move into the overflow menu. macOS keeps the size the person chooses;
        // on a smaller screen `fitsScreen` brings the window inside the screen.
        .defaultSize(width: 1360, height: 860)
        .windowToolbarStyle(.unified)
        .commands { AppCommands(model: model) }

        Window("About Straft Studio", id: "about") {
            AboutView()
                .environment(model)
                .environment(themes)
                .straftTheme(themes)
        }
        .windowResizability(.contentSize)
        .windowToolbarStyle(.unifiedCompact)

        Settings {
            SettingsView()
                .environment(model)
                .environment(themes)
                .straftTheme(themes)
        }
    }
}

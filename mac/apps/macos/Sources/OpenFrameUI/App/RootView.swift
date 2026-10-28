// What the main window shows for each app phase, plus the app-wide dialogs.

import SwiftUI
import OpenFrameCore

public struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.openWindow) private var openWindow
    @State private var palette: SearchPaletteModel?
    @State private var showShortcuts = false
    @State private var welcome: WelcomeNameModel?

    public init() {}

    public var body: some View {
        @Bindable var model = model
        content
            .overlay(alignment: .bottom) { ToastHost() }
            .overlay(alignment: .top) { paletteOverlay }
            .licenceScreens(model.licence)
            .trialCorner(model.licence)
            .task(id: showsProject) { await model.licence.setProjectOpen(showsProject) }
            .onReceive(NotificationCenter.default.publisher(for: .openFrameShowSearch)) { _ in openPalette() }
            .onReceive(NotificationCenter.default.publisher(for: .openFrameShowAbout)) { _ in openWindow(id: "about") }
            .onReceive(NotificationCenter.default.publisher(for: .openFrameShowShortcuts)) { _ in
                if showsHomeOrProject { showShortcuts = true }
            }
            .task(id: model.isRunning) { await ShortcutStore.shared.reload(core: model.core) }
            .themedSheet(isPresented: $showShortcuts) { ShortcutsSheet(app: model) }
            .task(id: welcomeKey) { updateWelcome() }
            .animation(reduceMotion ? .easeInOut(duration: 0.15) : .spring(response: 0.35, dampingFraction: 1), value: phaseKey)
            .alert(item: $model.alert) { alert in
                switch alert {
                case let .error(title, failure):
                    return Alert(title: Text(title), message: Text(failure.message), dismissButton: .default(Text("OK")))
                case let .recovery(_, offer):
                    let title = Text("OpenFrame closed unexpectedly last time")
                    let message = Text(RecoveryText.message(offer))
                    let keep = Alert.Button.default(Text("Keep the latest work")) { Task { await model.resolveRecovery("latest") } }
                    guard offer.hasCheckpoint else { return Alert(title: title, message: message, dismissButton: keep) }
                    return Alert(
                        title: title, message: message, primaryButton: keep,
                        secondaryButton: .default(Text("Open the last confirmed save")) { Task { await model.resolveRecovery("checkpoint") } }
                    )
                }
            }
    }

    private var showsHomeOrProject: Bool {
        switch model.phase {
        case .home, .project: !model.licence.blocksApp
        default: false
        }
    }

    /// First launch: on Application Home, until a name is stored.
    private var welcomeKey: String {
        "\(model.needsWelcomeName)-\(showsHome)-\(model.licence.blocksApp)"
    }

    private var showsHome: Bool {
        if case .home = model.phase { return true }
        return false
    }

    private func updateWelcome() {
        if model.needsWelcomeName && showsHome && !model.licence.blocksApp {
            if welcome == nil { welcome = WelcomeNameModel(app: model, current: model.info?.profile.displayName ?? "") }
        } else if welcome != nil, !model.needsWelcomeName {
            welcome = nil
        }
    }

    private var showsProject: Bool {
        if case .project = model.phase { return true }
        return false
    }

    private func openPalette() {
        // A licence screen that cannot be closed also holds back the search palette.
        guard !model.licence.blocksApp else { return }
        switch model.phase {
        case .home, .project: palette = SearchPaletteModel(app: model)
        default: break
        }
    }

    /// The Command+K palette, over a light scrim. A click outside or Escape closes it.
    @ViewBuilder private var paletteOverlay: some View {
        if let palette {
            ZStack(alignment: .top) {
                Color.black.opacity(0.18)
                    .ignoresSafeArea()
                    .onTapGesture { self.palette = nil }
                SearchPaletteView(model: palette) { self.palette = nil }
                    .padding(.top, 70)
            }
            .transition(.opacity)
        }
    }

    private var phaseKey: String {
        switch model.phase {
        case .starting: "starting"
        case .failed: "failed"
        case .librarySetup: "library"
        case .home: "home"
        case let .project(p): "project-\(p.id)"
        }
    }

    @ViewBuilder private var content: some View {
        switch model.phase {
        case .starting:
            ProgressView("Opening OpenFrame Studio")
                .controlSize(.large)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .accessibilityIdentifier("starting")
        case let .failed(failure):
            StartupFailureView(failure: failure)
        case let .librarySetup(status):
            LibrarySetupView(model: LibrarySetupModel(app: model, status: status, storeBuild: model.isStoreBuild))
        case .home:
            // The first-launch welcome fills the window, like the library setup, instead of being a
            // sheet: a sheet that needs a name before it closes would keep macOS from quitting.
            if let welcome {
                WelcomeNameView(model: welcome)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(theme["bg"])
            } else {
                HomeView()
            }
        case let .project(project):
            ProjectView(project: project)
                .id(project.id)
        }
    }
}

enum RecoveryText {
    static func message(_ offer: RecoveryOffer) -> String {
        var parts = ["Your latest work was kept."]
        if let at = offer.lastAutosaveAt {
            parts.append("Last saved \(Formatting.dateTime(ms: at)).")
        }
        if offer.hasCheckpoint, let cp = offer.checkpointAt {
            parts.append("You can also open the last confirmed save from \(Formatting.dateTime(ms: cp)).")
        }
        return parts.joined(separator: " ")
    }
}

struct StartupFailureView: View {
    let failure: CoreFailure

    var body: some View {
        ContentUnavailableView {
            Label("OpenFrame Studio could not start", systemImage: "exclamationmark.triangle")
        } description: {
            Text(failure.message)
        } actions: {
            Button("Quit") { NSApplication.shared.terminate(nil) }
                .keyboardShortcut(.defaultAction)
        }
        .accessibilityIdentifier("startup-failure")
    }
}

/// Dates and sizes in the person's own format.
enum Formatting {
    static func dateTime(ms: Double) -> String {
        Date(timeIntervalSince1970: ms / 1000).formatted(date: .abbreviated, time: .shortened)
    }

    static func relative(ms: Double) -> String {
        let date = Date(timeIntervalSince1970: ms / 1000)
        return date.formatted(.relative(presentation: .named))
    }
}

extension WelcomeNameModel: Identifiable {}

// What the main window shows for each app phase, plus the app-wide dialogs.

import SwiftUI
import OpenFrameCore

public struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.openWindow) private var openWindow
    @State private var palette: SearchPaletteModel?

    public init() {}

    public var body: some View {
        @Bindable var model = model
        content
            .overlay(alignment: .bottom) { ToastHost() }
            .overlay(alignment: .top) { paletteOverlay }
            .onReceive(NotificationCenter.default.publisher(for: .openFrameShowSearch)) { _ in openPalette() }
            .onReceive(NotificationCenter.default.publisher(for: .openFrameShowAbout)) { _ in openWindow(id: "about") }
            .tint(theme["accent"])
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

    private func openPalette() {
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
        case .teamGate: "gate"
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
        case let .teamGate(status):
            TeamGateView(status: status)
        case .home:
            HomeView()
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

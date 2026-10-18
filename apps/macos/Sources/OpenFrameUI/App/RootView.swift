// What the main window shows for each app phase, plus the app-wide dialogs.

import SwiftUI
import OpenFrameCore

public struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init() {}

    public var body: some View {
        @Bindable var model = model
        content
            .tint(theme["accent"])
            .animation(reduceMotion ? .easeInOut(duration: 0.15) : .spring(response: 0.35, dampingFraction: 1), value: phaseKey)
            .alert(item: $model.alert) { alert in
                switch alert {
                case let .error(title, failure):
                    return Alert(title: Text(title), message: Text(failure.message), dismissButton: .default(Text("OK")))
                case let .recovery(_, offer):
                    return Alert(
                        title: Text("OpenFrame closed unexpectedly last time"),
                        message: Text(RecoveryText.message(offer)),
                        primaryButton: .default(Text("Keep the latest work")) {
                            Task { await model.resolveRecovery("latest") }
                        },
                        secondaryButton: offer.hasCheckpoint
                            ? .default(Text("Open the last confirmed save")) { Task { await model.resolveRecovery("checkpoint") } }
                            : .cancel(Text("Keep the latest work")) { Task { await model.resolveRecovery("latest") } }
                    )
                }
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

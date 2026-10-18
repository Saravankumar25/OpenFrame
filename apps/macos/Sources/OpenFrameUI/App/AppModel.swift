// The app's view state. Rules live in the Rust core; this model only keeps what the screens show
// and forwards every user action to an operation (ADR-0027 decision 3).

import Foundation
import Observation
import OpenFrameCore

/// What the main window shows.
public enum AppPhase: Equatable {
    case starting
    /// The core could not start. The message is safe to show.
    case failed(CoreFailure)
    /// An internal build before the team password (ADR-0024).
    case teamGate(TeamGateStatus)
    case home
    case project(ProjectSummary)
}

/// A dialog the app asks for, shown by the root view.
public enum AppAlert: Identifiable, Equatable {
    case error(title: String, failure: CoreFailure)
    case recovery(project: ProjectSummary, offer: RecoveryOffer)

    public var id: String {
        switch self {
        case let .error(title, failure): return "error-\(title)-\(failure.code)"
        case let .recovery(project, _): return "recovery-\(project.id)"
        }
    }
}

@MainActor
@Observable
public final class AppModel {
    public private(set) var phase: AppPhase = .starting
    public private(set) var info: AppInfo?
    public var alert: AppAlert?
    public private(set) var saveState: SaveState?
    /// Set when the project was migrated from an older format on open.
    public private(set) var migratedFrom: Int?
    /// Bumped per table on every `dataChanged` event; screens reload what they read.
    public let changes = DataChanges()

    public private(set) var core: Core?
    private var eventTask: Task<Void, Never>?
    private let options: CoreStartOptions

    public init(options: CoreStartOptions) {
        self.options = options
    }

    /// For previews and tests that hand in a running core.
    public init(core: Core, options: CoreStartOptions) {
        self.options = options
        self.core = core
    }

    // MARK: Lifecycle

    public func start() async {
        do {
            let core: Core
            if let existing = self.core {
                core = existing
            } else {
                core = try await Core.start(options)
                self.core = core
            }
            listen(to: core)
            try await afterStart()
        } catch {
            phase = .failed(CoreFailure(asCore: error))
        }
    }

    private func afterStart() async throws {
        guard let core else { return }
        let gate = try await core.teamGateStatus()
        if gate.required && !gate.unlocked {
            phase = .teamGate(gate)
            return
        }
        try await enterApp()
    }

    private func enterApp() async throws {
        guard let core else { return }
        info = try await core.appInfo()
        if let current = try await core.projectCurrent() {
            phase = .project(current)
        } else {
            phase = .home
        }
    }

    /// One team password try. Returns the plain message to show when the try did not open.
    public func unlock(password: String) async -> String? {
        guard let core else { return nil }
        do {
            let answer = try await core.teamGateUnlock(TeamGateUnlockArgs(password: password))
            if answer.status.unlocked {
                try await enterApp()
                return nil
            }
            phase = .teamGate(answer.status)
            return answer.message ?? "That password is not right."
        } catch {
            return CoreFailure(asCore: error).message
        }
    }

    public func refreshGate() async {
        guard let core, case .teamGate = phase else { return }
        if let gate = try? await core.teamGateStatus() {
            phase = .teamGate(gate)
        }
    }

    /// Called from `applicationShouldTerminate`: close the project cleanly.
    public func shutdown() {
        eventTask?.cancel()
        core?.stop()
    }

    // MARK: Projects

    public func createProject(_ args: CreateProjectArgs) async -> Bool {
        guard let core else { return false }
        do {
            let result = try await core.projectCreate(args)
            migratedFrom = nil
            phase = .project(result.project)
            return true
        } catch {
            show("Could not create the project", error)
            return false
        }
    }

    public func openProject(path: String) async {
        guard let core else { return }
        do {
            let result = try await core.projectOpen(OpenProjectArgs(path: path))
            migratedFrom = result.migratedFrom.map { Int($0) }
            phase = .project(result.project)
            if let offer = result.recovery {
                alert = .recovery(project: result.project, offer: offer)
            }
        } catch {
            show("Could not open the project", error)
        }
    }

    public func closeProject() async {
        guard let core else { return }
        do {
            _ = try await core.projectClose()
        } catch {
            show("Could not close the project", error)
            return
        }
        phase = .home
    }

    public func resolveRecovery(_ choice: String) async {
        guard let core else { return }
        do {
            _ = try await core.projectResolveRecovery(RecoveryArgs(choice: choice))
        } catch {
            show("Could not finish the recovery", error)
        }
        alert = nil
    }

    // MARK: Errors

    public func show(_ title: String, _ error: Error) {
        alert = .error(title: title, failure: CoreFailure(asCore: error))
    }

    // MARK: Events

    private func listen(to core: Core) {
        eventTask?.cancel()
        let stream = core.events()
        eventTask = Task { [weak self] in
            for await event in stream {
                guard let self else { return }
                self.handle(event)
            }
        }
    }

    func handle(_ event: AppEvent) {
        switch event {
        case let .dataChanged(store, tables, _, _):
            changes.bump(store: store, tables: tables)
        case let .saveState(state):
            saveState = state
        case .projectClosed:
            if case .project = phase { phase = .home }
        default:
            break
        }
    }
}


extension CoreFailure {
    /// Any error from a call, as the coded failure the screens show.
    public init(asCore error: Error) {
        if let failure = error as? CoreFailure {
            self = failure
        } else if error is DecodingError {
            self.init(
                code: "internal.decode",
                message: "OpenFrame received an answer it could not read. Your project was not changed.",
                detail: String(describing: error)
            )
        } else {
            self.init(code: "internal.unexpected", message: "Something went wrong inside OpenFrame.", detail: String(describing: error))
        }
    }
}

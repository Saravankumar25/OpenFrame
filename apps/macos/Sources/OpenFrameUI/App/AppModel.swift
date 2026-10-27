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
    /// "Set up your OpenFrame library": a new Mac, or a remembered library that is missing. The
    /// core refuses everything else until a choice worked (doc 40 section 1).
    case librarySetup(LibraryStatus)
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
    /// The Mac App Store build's licence and trial screens. Inert in the internal build.
    public let licence = LicenceModel()
    /// The StoreKit bridge and the licensing server client handed to the core, set by the app.
    public var storeRuntime: StoreRuntime?
    private var eventTask: Task<Void, Never>?
    private let options: CoreStartOptions
    /// Project and library moves that are running. The core closes the open project and opens it
    /// again from its new folder; meanwhile the window keeps showing it.
    public private(set) var relocations = 0
    /// Moves on with every phase the person asked for, so a late answer about the current project
    /// never overrides a newer open or close.
    @ObservationIgnored private var phaseGeneration = 0

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
        let library = try await core.libraryStatus()
        guard library.state == .ready else {
            phase = .librarySetup(library)
            return
        }
        info = try await core.appInfo()
        await licence.attach(core: core, runtime: storeRuntime)
        if let current = try await core.projectCurrent() {
            phase = .project(current)
        } else {
            phase = .home
        }
    }

    /// The setup screen's choice worked: go on into the app. Any other answer keeps the screen.
    public func libraryReady(_ status: LibraryStatus) async {
        guard status.state == .ready else {
            phase = .librarySetup(status)
            return
        }
        do {
            try await enterApp()
        } catch {
            phase = .failed(CoreFailure(asCore: error))
        }
    }

    /// True in the Mac App Store build (see `StorageRules.isStoreBuild`).
    public var isStoreBuild: Bool {
        StorageRules.isStoreBuild(environment: ProcessInfo.processInfo.environment, channel: licence.channel)
    }

    /// Reads the app information again (after the person's name was stored on first launch).
    public func refreshInfo() async {
        guard let core else { return }
        if let fresh = try? await core.appInfo() { info = fresh }
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
        licence.detach()
        core?.stop()
    }

    // MARK: Projects

    public func createProject(_ args: CreateProjectArgs) async -> Bool {
        guard let core else { return false }
        do {
            let result = try await core.projectCreate(args)
            migratedFrom = nil
            phaseGeneration += 1
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
            phaseGeneration += 1
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
        // Text the editors still hold is stored before the project closes.
        await ScreenplayWorkspaceModel.flushAll()
        do {
            _ = try await core.projectClose()
        } catch {
            show("Could not close the project", error)
            return
        }
        phaseGeneration += 1
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

    // MARK: Moves

    /// A project or library move started: its close and reopen do not send the window Home.
    public func beginRelocation() { relocations += 1 }

    /// The move ended (done, failed or cancelled): show what the core has open now, and have
    /// every screen read again from the new place.
    public func endRelocation() async {
        relocations = max(0, relocations - 1)
        await showCurrentProject()
        changes.bumpAll()
    }

    /// Shows the project the core has open: after a move it is the same project in a new folder.
    /// Only on Home or in a project; never over the team gate or the setup screen.
    func showCurrentProject() async {
        guard let core else { return }
        switch phase {
        case .home, .project: break
        default: return
        }
        let mine = phaseGeneration
        guard let current = try? await core.projectCurrent() else {
            // Nothing open: a move that could not reopen the project ends on Home.
            if mine == phaseGeneration, relocations == 0, case .project = phase {
                phase = .home
            }
            return
        }
        guard mine == phaseGeneration else { return }
        switch phase {
        case .home, .project:
            if phase != .project(current) { phase = .project(current) }
        default:
            break
        }
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
        case let .module(module, name, _) where module == "entitlement" && name == "changed":
            // The store channel's check finished: read the licence state again.
            Task { await licence.refresh() }
        case .projectOpened:
            // Home lists the recent projects: a project that was just made or opened belongs in it.
            changes.bump(store: .global, tables: ["sys_recent_project"])
            // A project the core opened by itself (a move reopens it from its new folder) shows
            // again instead of leaving the window on Home.
            Task { await showCurrentProject() }
        case .projectClosed:
            changes.bump(store: .global, tables: ["sys_recent_project"])
            // During a move the core closes the project and opens it again from the new place.
            if relocations == 0, case .project = phase { phase = .home }
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

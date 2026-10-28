// "Set up your OpenFrame library" (Windows: features/library-setup/LibrarySetup.tsx): the first
// start on a new Mac, or a remembered library that went missing (doc 40 section 1). The core
// refuses every other operation until one of these choices worked, so this screen is not the only
// gate; it only asks for folders and shows what the core found. Nothing is created before the
// person confirms a choice.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class LibrarySetupModel {
    enum Choice: String, CaseIterable, Identifiable {
        case useDefault, newLocation, locate
        var id: String { rawValue }
        var label: String {
            switch self {
            case .useDefault: "Use the default location"
            case .newLocation: "Choose a new location"
            case .locate: "Locate existing OpenFrame data"
            }
        }
    }

    private(set) var status: LibraryStatus
    var choice: Choice
    /// The folder picked for a new library; the library is `<parent>/OpenFrame`.
    private(set) var parent: String?
    /// What the core found in the picked folder (Locate), or about the picked parent (new).
    private(set) var found: LibraryInspection?
    private(set) var error: String?
    private(set) var busy = false
    /// The Mac App Store build keeps the library in its container: no other folder is offered.
    let storeBuild: Bool

    @ObservationIgnored private weak var app: AppModel?
    @ObservationIgnored var picker: StoragePicking = PanelStoragePicker()

    init(app: AppModel?, status: LibraryStatus, storeBuild: Bool) {
        self.app = app
        self.status = status
        self.storeBuild = storeBuild
        let offerLocate = (!status.found.isEmpty || status.state == .missing) && StorageRules.canRelocate(storeBuild: storeBuild)
        choice = offerLocate ? .locate : .useDefault
    }

    /// The choices this build offers.
    var choices: [Choice] { StorageRules.canRelocate(storeBuild: storeBuild) ? Choice.allCases : [.useDefault] }

    var missing: Bool { status.state == .missing && status.root != nil }
    var newLibraryPath: String? { parent.map { StorageRules.joinPath($0, "OpenFrame") } }

    func choose(_ c: Choice) {
        guard choices.contains(c) else { return }
        choice = c
        parent = nil
        found = nil
        error = nil
    }

    // MARK: Folder panels

    func pickParent() async {
        guard StorageRules.canRelocate(storeBuild: storeBuild),
              let dir = picker.folder(title: "Choose the folder that will hold your OpenFrame library") else { return }
        error = nil
        parent = dir
        // Only for the synced-folder warning: a folder the core cannot read is refused at Create.
        found = try? await app?.core?.libraryInspect(LibraryDirArgs(dir: dir))
    }

    func pickExisting() async {
        guard StorageRules.canRelocate(storeBuild: storeBuild),
              let dir = picker.folder(title: "Choose the folder of your OpenFrame library") else { return }
        error = nil
        found = nil
        do {
            found = try await app?.core?.libraryInspect(LibraryDirArgs(dir: dir))
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    // MARK: Choices

    func useDefault() async { await run { try await $0.libraryUseDefault() } }

    func createLibrary() async {
        guard let parent else { return }
        await run { try await $0.libraryCreateAt(LibraryCreateArgs(parent: parent)) }
    }

    /// Use This Library, Connect Library and Try Again.
    func connect(_ dir: String) async {
        await run { try await $0.libraryConnect(LibraryDirArgs(dir: dir)) }
    }

    func tryAgain() async {
        guard let root = status.root else { return }
        await connect(root)
    }

    /// One choice. When the core says the library is ready, the app goes on; a failure stays on
    /// this screen with the core's message.
    private func run(_ op: (Core) async throws -> LibraryStatus) async {
        guard let app, let core = app.core, !busy else { return }
        busy = true
        error = nil
        defer { busy = false }
        do {
            let s = try await op(core)
            if s.state == .ready {
                await app.libraryReady(s)
            } else {
                status = s
            }
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

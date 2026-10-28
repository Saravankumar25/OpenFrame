// "Set up your OpenFrame library" against the real core started like a new Mac
// (`testLibrarySetup`, the Windows OPENFRAME_LIBRARY_SETUP=1 switch): every folder stays inside
// the test folder, with the suggested library at <root>/home/OpenFrame and an earlier one at
// <root>/Documents/OpenFrame (Windows: LibrarySetup.test.tsx and tests/platform/library_setup.rs).

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Library setup", .serialized)
@MainActor
struct LibrarySetupTests {
    private func root() throws -> URL { try Fixture.root() }

    private func newMac(_ root: URL) -> AppModel {
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        options.testLibrarySetup = true
        return AppModel(options: options)
    }

    private func setupStatus(_ app: AppModel) throws -> LibraryStatus {
        guard case let .librarySetup(status) = app.phase else {
            Issue.record("the setup screen did not show: \(app.phase)")
            throw CoreFailure(code: "test", message: "no setup screen")
        }
        return status
    }

    private func exists(_ path: String) -> Bool { FileManager.default.fileExists(atPath: path) }

    /// An earlier library (the Projects and vault folders) where an older version kept it.
    private func earlierLibrary(_ root: URL) throws -> URL {
        let lib = root.appendingPathComponent("Documents/OpenFrame", isDirectory: true)
        try FileManager.default.createDirectory(at: lib.appendingPathComponent("Projects"), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: lib.appendingPathComponent("Global Idea Vault"), withIntermediateDirectories: true)
        return lib
    }

    @Test func aNewMacShowsTheSetupScreenAndCreatesNothingBeforeAChoice() async throws {
        let root = try root()
        let app = newMac(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let status = try setupStatus(app)
        #expect(status.state == .needsSetup)
        #expect(status.found.isEmpty)
        #expect(status.suggested.hasSuffix("/home/OpenFrame"))
        #expect(!exists(status.suggested), "nothing is created before the person confirms")
        // The core refuses everything else until a choice worked.
        await #expect(throws: (any Error).self) { _ = try await app.core!.projectCurrent() }

        let model = LibrarySetupModel(app: app, status: status, storeBuild: false)
        #expect(model.choice == .useDefault)
        #expect(model.choices == [.useDefault, .newLocation, .locate])
        await model.useDefault()
        #expect(app.phase == .home)
        #expect(exists(status.suggested))
        #expect(model.error == nil)
    }

    @Test func librariesFoundAreOfferedFirstAndConnect() async throws {
        let root = try root()
        let lib = try earlierLibrary(root)
        let app = newMac(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let status = try setupStatus(app)
        #expect(status.found.map(\.libraryRoot) == [lib.path])
        let model = LibrarySetupModel(app: app, status: status, storeBuild: false)
        #expect(model.choice == .locate, "an existing library is offered before a new one")
        #expect(StorageRules.foundText(status.found[0]) == status.found[0].message)
        await model.connect(status.found[0].libraryRoot!)
        #expect(app.phase == .home)
        #expect(try await app.core!.storageInfo().libraryDir == lib.path)
        #expect(!exists(status.suggested), "no second library was made")
    }

    @Test func locateShowsThePreviewFindsTheParentAndRefusesWrongFolders() async throws {
        let root = try root()
        let lib = try earlierLibrary(root)
        let empty = root.appendingPathComponent("Empty", isDirectory: true)
        try FileManager.default.createDirectory(at: empty, withIntermediateDirectories: true)
        let app = newMac(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let model = LibrarySetupModel(app: app, status: try setupStatus(app), storeBuild: false)
        let picker = StubStoragePicker()
        model.picker = picker

        // An empty folder: the core's message, and no Connect.
        picker.folders = [empty.path]
        await model.pickExisting()
        let refused = try #require(model.found)
        #expect(refused.libraryRoot == nil)
        #expect(refused.kind == .empty)
        #expect(!refused.message.isEmpty)
        #expect(model.choice == .locate)
        if case .librarySetup = app.phase {} else { Issue.record("a refused folder left the setup screen") }

        // The folder above the library: the library inside it is found.
        picker.folders = [lib.deletingLastPathComponent().path]
        await model.pickExisting()
        let found = try #require(model.found)
        #expect(found.kind == .libraryParent)
        #expect(found.libraryRoot == lib.path)
        #expect(found.projectCount == 0)
        #expect(picker.asked.last == "Choose the folder of your OpenFrame library")

        // Cancel in the panel changes nothing.
        await model.pickExisting()
        #expect(model.found == found)

        await model.connect(found.libraryRoot!)
        #expect(app.phase == .home)
    }

    @Test func aNewLocationIsCreatedOnlyOnConfirm() async throws {
        let root = try root()
        let parent = root.appendingPathComponent("Work", isDirectory: true)
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
        let app = newMac(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let model = LibrarySetupModel(app: app, status: try setupStatus(app), storeBuild: false)
        let picker = StubStoragePicker()
        model.picker = picker
        model.choose(.newLocation)
        picker.folders = [parent.path]
        await model.pickParent()
        #expect(model.newLibraryPath == parent.appendingPathComponent("OpenFrame").path)
        #expect(!exists(parent.appendingPathComponent("OpenFrame").path), "choosing a folder creates nothing")
        await model.createLibrary()
        #expect(app.phase == .home)
        #expect(try await app.core!.storageInfo().libraryDir == parent.appendingPathComponent("OpenFrame").path)
    }

    @Test func aMissingLibraryOpensRecoveryAndIsNeverRecreatedEmpty() async throws {
        let root = try root()
        let first = newMac(root)
        await first.start()
        let status = try setupStatus(first)
        let model = LibrarySetupModel(app: first, status: status, storeBuild: false)
        await model.useDefault()
        #expect(first.phase == .home)
        first.shutdown()
        try FileManager.default.removeItem(atPath: status.suggested)

        let again = newMac(root)
        defer { again.shutdown(); try? FileManager.default.removeItem(at: root) }
        await again.start()
        let missing = try setupStatus(again)
        #expect(missing.state == .missing)
        #expect(missing.root == status.suggested)
        let recovery = LibrarySetupModel(app: again, status: missing, storeBuild: false)
        #expect(recovery.missing)
        #expect(recovery.choice == .locate)
        await recovery.tryAgain()
        #expect(recovery.error != nil, "Try Again with the folder still gone says why")
        if case .librarySetup = again.phase {} else { Issue.record("a missing library entered the app") }
        #expect(!exists(status.suggested), "the missing library was not made again, empty")
    }

    @Test func theStoreBuildOffersOnlyTheDefault() async throws {
        let root = try root()
        _ = try earlierLibrary(root)
        let app = newMac(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let model = LibrarySetupModel(app: app, status: try setupStatus(app), storeBuild: true)
        let picker = StubStoragePicker()
        model.picker = picker
        #expect(model.choices == [.useDefault])
        #expect(model.choice == .useDefault)
        model.choose(.locate)
        #expect(model.choice == .useDefault)
        await model.pickExisting()
        await model.pickParent()
        #expect(picker.asked.isEmpty, "no folder panel in the store build")
    }

    @Test func theSetupScreenDraws() async throws {
        let root = try root()
        let app = newMac(root)
        defer { app.shutdown(); try? FileManager.default.removeItem(at: root) }
        await app.start()
        let model = LibrarySetupModel(app: app, status: try setupStatus(app), storeBuild: false)
        let themes = ThemeStore()
        let view = LibrarySetupView(model: model).environment(app).environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
        #expect(Fixture.render(view, name: "library-setup") != nil)
    }
}

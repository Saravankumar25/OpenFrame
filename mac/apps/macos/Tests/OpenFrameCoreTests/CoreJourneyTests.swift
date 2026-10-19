// Mac phase M2 exit: a Swift program drives the Rust core end to end through the interface.
// Open a project, create a scene, undo, receive the events; bad requests come back as coded
// errors. Every test runs on its own temporary data folder (ADR-0031).

import Foundation
import Testing
@testable import OpenFrameCore

func temporaryRoot() throws -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("openframe-swift-tests", isDirectory: true)
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}

func startCore(_ root: URL) async throws -> Core {
    try await Core.start(CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false))
}

/// Collects events in the background for later checks.
actor EventLog {
    private(set) var events: [AppEvent] = []
    func add(_ e: AppEvent) { events.append(e) }

    func dataChangedTables() -> [String] {
        events.flatMap { event -> [String] in
            if case let .dataChanged(_, tables, _, _) = event { return tables }
            return []
        }
    }

    func hasProjectOpened() -> Bool {
        events.contains { if case .projectOpened = $0 { return true } else { return false } }
    }
}

@Suite("Core journey from Swift", .serialized)
struct CoreJourneyTests {
    @Test func projectSceneUndoAndEvents() async throws {
        let root = try temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let core = try await startCore(root)
        let log = EventLog()
        let stream = core.events()
        let listener = Task { for await e in stream { await log.add(e) } }

        let opened = try await core.projectCreate(CreateProjectArgs(title: "Swift Journey", projectType: .featureFilm))
        #expect(opened.project.title == "Swift Journey")

        let created = try await core.screenplayCreate(CreateScreenplayArgs())
        let draftId = created.draft.id
        let edit = try await core.screenplayCreateScene(
            ScreenplayCreateSceneArgs(draftId: draftId, heading: "INT. RAILWAY STATION - NIGHT"))
        #expect(!edit.createdIds.isEmpty) // the scene and its heading element

        var doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
        #expect(doc.scenes.contains { $0.heading.contains("RAILWAY STATION") })

        let undone = try await core.historyUndo(StoreArgs(store: .project))
        #expect(undone.label != nil)
        doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
        #expect(!doc.scenes.contains { $0.heading.contains("RAILWAY STATION") })

        _ = try await core.historyRedo(StoreArgs(store: .project))
        doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
        #expect(doc.scenes.contains { $0.heading.contains("RAILWAY STATION") })

        // Events are delivered on core threads; give the stream a moment to drain.
        for _ in 0..<50 where await log.dataChangedTables().isEmpty {
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        #expect(await log.hasProjectOpened())
        #expect(await log.dataChangedTables().contains { $0.hasPrefix("screenplay") })

        listener.cancel()
        core.stop()
    }

    @Test func badRequestsAreCodedErrors() async throws {
        let root = try temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let core = try await startCore(root)

        await #expect(throws: CoreFailure.self) { _ = try await core.invokeRaw("shell.run_anything", "{}") }
        do {
            _ = try await core.invokeRaw("shell.run_anything", "{}")
        } catch let failure as CoreFailure {
            #expect(failure.code == "security.unknown_operation")
            #expect(!failure.message.isEmpty)
        }
        do {
            _ = try await core.invokeRaw("project.list_recent", "{not json")
            Issue.record("malformed JSON was accepted")
        } catch let failure as CoreFailure {
            #expect(failure.code == "validation.invalid_input")
        }
        // No project open: the core answers with a code, never a crash.
        await #expect(throws: CoreFailure.self) { _ = try await core.screenplayCreate(CreateScreenplayArgs()) }
        // Paths never come from the shell.
        await #expect(throws: CoreFailure.self) { try await core.revealLocation("anywhere") }
        await #expect(throws: CoreFailure.self) { try await core.openURL("file:///etc/hosts") }
        core.stop()
    }

    @Test func projectReopensAfterRestart() async throws {
        let root = try temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        var core = try await startCore(root)
        _ = try await core.projectCreate(CreateProjectArgs(title: "Restart", projectType: .shortFilm))
        let draftId = try await core.screenplayCreate(CreateScreenplayArgs()).draft.id
        _ = try await core.screenplayCreateScene(ScreenplayCreateSceneArgs(draftId: draftId, heading: "EXT. HARBOUR - DAY"))
        core.stop()

        core = try await startCore(root)
        let recent = try await core.projectListRecent(ListRecentArgs(includeArchived: false))
        let path = try #require(recent.first?.path)
        _ = try await core.projectOpen(OpenProjectArgs(path: path))
        let doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
        #expect(doc.scenes.contains { $0.heading.contains("HARBOUR") })
        core.stop()
    }

    @Test func manyCallsInParallelNeverBlockOrCrossWires() async throws {
        let root = try temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let core = try await startCore(root)
        _ = try await core.projectCreate(CreateProjectArgs(title: "Parallel", projectType: .featureFilm))
        let draftId = try await core.screenplayCreate(CreateScreenplayArgs()).draft.id
        let before = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId)).scenes.count
        try await withThrowingTaskGroup(of: Void.self) { group in
            for i in 0..<20 {
                group.addTask {
                    _ = try await core.screenplayCreateScene(
                        ScreenplayCreateSceneArgs(draftId: draftId, heading: "INT. ROOM \(i) - DAY"))
                }
            }
            try await group.waitForAll()
        }
        let doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
        #expect(doc.scenes.count == before + 20)
        core.stop()
    }

    @Test func operationListMatchesTheGeneratedNames() async throws {
        let root = try temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let core = try await startCore(root)
        let names = Set(core.operationNames)
        #expect(names.contains(OperationName.projectCreate))
        #expect(names.contains(OperationName.screenplayCreateScene))
        #expect(names.count > 400)
        core.stop()
    }
}

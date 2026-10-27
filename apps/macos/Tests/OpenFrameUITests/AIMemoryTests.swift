// Settings > Offline AI > Assistant memory (Windows: workspaces/settings/AssistantMemory.test.tsx).
// The list and delete answers come from a stand-in for the core (AITestSupport.swift): entries are
// saved only by a real model turn, which these tests do not run. The real `ai.memory_list` of an
// open project is checked once, and the card without a project.

import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

private func entry(_ id: String, _ text: String, _ kind: AiMemoryKind) -> AiMemoryEntry {
    AiMemoryEntry(id: id, text: text, kind: kind, entityRefs: [], sourceRequestId: "r1", createdAt: Date().timeIntervalSince1970 * 1000)
}

@Suite("AI assistant memory", .serialized)
@MainActor
struct AIMemoryTests {
    private func project() async throws -> (URL, AppModel) {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        _ = await app.createProject(CreateProjectArgs(title: "Night Bus", projectType: .featureFilm))
        return (root, app)
    }

    private func close(_ root: URL, _ app: AppModel) {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    @Test func explainsWhatIsKeptWhenNothingIsRememberedYet() async throws {
        let (root, app) = try await project()
        defer { close(root, app) }
        // The real core: a new project remembers nothing.
        let m = AssistantMemoryModel(app: app)
        #expect(m.hasProject)
        await m.load()
        #expect(m.entries == [])
        #expect(m.error == nil)
        #expect(AssistantMemoryText.empty.hasPrefix("Nothing is remembered yet."))
        #expect(AssistantMemoryText.hint.hasPrefix("Saved on this Mac, in this project, for you only"))
        #expect(AssistantMemoryText.hint.contains("only when you state a fact or confirm an answer"))
    }

    @Test func listsEachEntryWithItsKindAndALabelledDeleteButton() async throws {
        let (root, app) = try await project()
        defer { close(root, app) }
        let stub = AIStubCore()
        let fact = entry("m1", "I'm the assistant director", .fact)
        let preference = entry("m2", "Always answer in short sentences", .preference)
        stub.reply("ai.memory_list", AiMemoryListDto(entries: [fact, preference]))
        let m = AssistantMemoryModel(app: app)
        m.invoker = stub
        await m.load()
        #expect(m.entries?.map(\.text) == ["I'm the assistant director", "Always answer in short sentences"])
        #expect(AssistantMemoryText.meta(preference).hasPrefix("Preference, saved Today "))
        #expect(AssistantMemoryText.meta(fact).hasPrefix("Fact, saved Today "))
        #expect(AssistantMemoryText.deleteLabel(fact) == "Delete: I'm the assistant director")
    }

    @Test func deleteForgetsThatEntryOnlyAndTheListUpdates() async throws {
        let (root, app) = try await project()
        defer { close(root, app) }
        let stub = AIStubCore()
        let director = entry("m1", "I'm the assistant director", .fact)
        stub.reply("ai.memory_list", AiMemoryListDto(entries: [director, entry("m2", "Call me Saravan", .preference)]))
        let m = AssistantMemoryModel(app: app)
        m.invoker = stub
        await m.load()
        stub.reply("ai.memory_delete", AiMemoryDeleteDto(deleted: true))
        // The core forgets the entry; the next list no longer has it.
        stub.reply("ai.memory_list", AiMemoryListDto(entries: [director]))
        await m.delete(try #require(m.entries?.last))
        #expect(stub.args("ai.memory_delete").count == 1)
        #expect(stub.args("ai.memory_delete").first?["id"] as? String == "m2")
        #expect(m.entries?.map(\.text) == ["I'm the assistant director"])
        #expect(ToastCenter.shared.items.last?.text == "Forgotten.")
        #expect(m.deleting == nil)
    }

    @Test func aFailedDeleteSaysWhyAndKeepsTheList() async throws {
        let (root, app) = try await project()
        defer { close(root, app) }
        let stub = AIStubCore()
        stub.reply("ai.memory_list", AiMemoryListDto(entries: [entry("m1", "Call me Saravan", .preference)]))
        stub.fail("ai.memory_delete", CoreFailure(code: "permission.denied", message: "You can't change this project."))
        let m = AssistantMemoryModel(app: app)
        m.invoker = stub
        await m.load()
        await m.delete(try #require(m.entries?.first))
        #expect(ToastCenter.shared.items.last?.text == "You can't change this project.")
        #expect(ToastCenter.shared.items.last?.kind == .error)
        #expect(m.entries?.count == 1)
    }

    @Test func withNoProjectOpenNothingIsAsked() async throws {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        defer { close(root, app) }
        let stub = AIStubCore()
        let m = AssistantMemoryModel(app: app)
        m.invoker = stub
        #expect(!m.hasProject)
        await m.load()
        #expect(m.entries == nil)
        #expect(stub.count("ai.memory_list") == 0)
        #expect(AssistantMemoryText.noProject.hasPrefix("Open a project"))
    }

    @Test func theCardDrawsInLightAndDark() async throws {
        let (root, app) = try await project()
        defer { close(root, app) }
        for scheme in [ColorScheme.light, .dark] {
            let themes = ThemeStore()
            let view = AssistantMemoryCard()
                .environment(app)
                .environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
                .environment(\.colorScheme, scheme)
            #expect(Fixture.render(view, name: "ai-memory-card-\(scheme)", size: CGSize(width: 480, height: 260)) != nil)
        }
    }
}

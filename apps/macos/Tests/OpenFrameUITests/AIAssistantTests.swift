// AI assistant: the model driven exactly as the view drives it, with a real core on a temporary
// folder. These tests have no engine and no model files, so they cover every state the core can
// report without a model (engine missing, nothing downloaded), that a failed request never loses
// the typed question, cancel, the scope line of each workspace (scenario 73), the change set
// review calls (scenario 74) and the report of an AI output. Questions that need a real model
// (scenarios 72, 74, 75 and 78 with an answer) are listed in the agent report.

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@MainActor
private struct Rig {
    let root: URL
    let app: AppModel
    let model: AIAssistantModel

    static func make(project: String? = "Night Bus") async throws -> Rig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        if let project { _ = await app.createProject(CreateProjectArgs(title: project, projectType: .featureFilm)) }
        return Rig(root: root, app: app, model: AIAssistantModel(app: app))
    }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }
}

private func exchange(_ text: String, set: AiChangeSetDto?) -> AiExchange {
    AiExchange(
        requestId: UUID().uuidString, conversationId: "c", requestText: text, scopeLabel: "Using: Whole Project",
        result: AiResultDto(
            id: UUID().uuidString, kind: set == nil ? .answer : .proposal, status: .informational, content: "ok", details: [], provenance: [],
            items: [], changeSet: set, steps: [], createdAt: 0),
        createdAt: 0)
}

private func set(_ id: String, state: String) -> AiChangeSetDto {
    AiChangeSetDto(
        id: id, title: "Create task", summary: "", state: state, validationState: "Valid", rows: [], exclusions: [], affectedModules: ["Notes & Tasks"],
        targets: [], operationCount: 1, createdAt: 0, appliedOperations: 1, partCount: 1, requiresConfirmation: false)
}

@Suite("AI assistant", .serialized)
@MainActor
struct AIAssistantTests {
    // MARK: Scenario 76: the engine is missing

    @Test func aMissingEngineTurnsAIOffAndSaysToReinstall() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        #expect(r.model.mode == .unavailable)
        await r.model.refreshStatus()
        let status = try #require(r.model.status)
        #expect(!status.available)
        #expect(status.mode == "Off")
        #expect(r.model.mode == .off)
        // The reinstall message, not the download offer, and no question box (the panel shows
        // the unavailable view for this mode, never the conversation).
        #expect(AILogic.offReason(status).contains("Reinstall OpenFrame Studio"))
        #expect(AILogic.damagedModels(status) == nil)
        #expect(!r.model.statusFailed)
    }

    @Test func everythingElseKeepsWorkingWhileAIIsOff() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        await r.model.refreshStatus()
        #expect(r.model.mode == .off)
        let task = try await r.app.core!.tasksCreate(CreateTaskArgs(title: "Confirm pier permit"))
        #expect(task.title == "Confirm pier permit")
        let open = try await r.app.core!.tasksList(TaskListArgs())
        #expect(open.map(\.title) == ["Confirm pier permit"])
    }

    // MARK: A failed request never loses the typed question

    @Test func aFailedRequestPutsTheQuestionBackAndSaysWhy() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.text = "  How many scenes are in the script?  "
        await r.model.sendAndWait()
        #expect(r.model.pending == nil)
        #expect(r.model.text == "How many scenes are in the script?")
        let message = try #require(r.model.error)
        #expect(message.contains("Reinstall OpenFrame Studio"))
        #expect(!message.contains("internal"))
        #expect(r.model.exchanges.isEmpty)
        // The person can send again straight away.
        #expect(r.model.canSend)
    }

    @Test func aSecondQuestionTypedWhileWaitingIsNotOverwritten() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.text = "First question"
        r.model.send()
        #expect(r.model.pending == "First question")
        #expect(r.model.text.isEmpty)
        // A second send while one runs does nothing.
        r.model.text = "Second question"
        r.model.send()
        #expect(r.model.pending == "First question")
        #expect(r.model.text == "Second question")
        #expect(!r.model.canSend)
        await r.model.sendAndWait()
        // The first failed; the person's newer text stays as it was.
        #expect(r.model.text == "Second question")
        #expect(r.model.error != nil)
    }

    @Test func emptyAndBlankQuestionsAreNotSent() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        for blank in ["", "   ", "\n\t"] {
            r.model.text = blank
            #expect(!r.model.canSend)
            r.model.send()
            #expect(r.model.pending == nil)
        }
    }

    @Test func cancelStopsWaitingAndKeepsTheQuestion() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.text = "Which scene happens at the railway station?"
        r.model.send()
        #expect(r.model.pending != nil)
        await r.model.cancel()
        #expect(r.model.pending == nil)
        #expect(r.model.text == "Which scene happens at the railway station?")
        #expect(r.model.error?.hasPrefix("Stopped.") == true)
        // The late failure of the stopped request does not overwrite the message or the box.
        try await Task.sleep(for: .milliseconds(400))
        #expect(r.model.error?.hasPrefix("Stopped.") == true)
        #expect(r.model.text == "Which scene happens at the railway station?")
        // Cancel with nothing running does nothing.
        await r.model.cancel()
        #expect(r.model.pending == nil)
    }

    // MARK: Conversation

    @Test func theConversationStartsEmptyAndNewClearsIt() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        await r.model.loadHistory()
        #expect(r.model.loadedHistory)
        #expect(r.model.exchanges.isEmpty)
        r.model.newConversation()
        await r.model.loadHistory()
        #expect(r.model.exchanges.isEmpty)
        #expect(r.model.error == nil)
    }

    @Test func stoppingTheEngineWhenNothingRunsIsHarmless() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        await r.model.stopRuntime()
        #expect(r.model.error == nil)
        #expect(r.model.status != nil)
        #expect(ToastCenter.shared.items.last?.text.hasPrefix("Offline AI stopped") == true)
    }

    // MARK: Scenario 73: the scope line of each workspace

    @Test func theScopeLineFollowsTheWorkspace() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        let expected: [(Workspace, String)] = [
            (.home, "Using: Whole Project"), (.ideaVault, "Using: Whole Project"), (.story, "Using: Story Board"),
            (.breakdown, "Using: Production Workspace"), (.production, "Using: Production Workspace"),
            (.callSheets, "Using: Production Workspace"), (.screenplay, "Using: Current Screenplay"), (.notes, "Using: Whole Project"),
        ]
        for (ws, line) in expected {
            r.model.setContext(AIScopeContext(workspace: ws))
            #expect(r.model.usingLabel == line, "\(ws)")
        }
        r.model.setContext(AIScopeContext(workspace: .screenplay, sceneId: "s1"))
        #expect(r.model.usingLabel == "Using: Current Scene")
    }

    @Test func aChosenScopeSticksUntilTheWorkspaceChanges() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.setContext(AIScopeContext(workspace: .screenplay))
        r.model.chosenScope = .wholeProject
        #expect(r.model.usingLabel == "Using: Whole Project")
        r.model.setContext(AIScopeContext(workspace: .screenplay, sceneId: "s1"))
        #expect(r.model.usingLabel == "Using: Whole Project")
        r.model.setContext(AIScopeContext(workspace: .story))
        #expect(r.model.usingLabel == "Using: Story Board")
        // A scope the place does not offer is dropped, not used.
        r.model.chosenScope = .callSheet
        #expect(r.model.effectiveScope == .storyBoard)
    }

    @Test func theScreenplayScopeUsesTheDraftTheCoreHas() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        let core = r.app.core!
        _ = try await core.screenplayCreate(CreateScreenplayArgs())
        let overview = try await core.screenplayOverview(ScreenplayOverviewArgs())
        let draft = try #require(overview.screenplay?.currentDraftId ?? overview.drafts.first?.id)
        r.model.setContext(AIScopeContext(workspace: .screenplay))
        let args = await r.model.resolvedScopeArgs()
        #expect(args.kind == .currentScreenplay)
        #expect(args.draftId == draft)
        // Outside the Screenplay workspace nothing is guessed.
        r.model.setContext(AIScopeContext(workspace: .home))
        r.model.chosenScope = .currentScreenplay
        #expect(await r.model.resolvedScopeArgs().draftId == nil)
    }

    @Test func workspacesHandTheirOpenSceneToTheAssistant() async throws {
        let hints = AIScopeHints.shared
        hints.sceneId = "scene-9"
        hints.draftId = "draft-2"
        defer { hints.sceneId = nil; hints.draftId = nil }
        let screenplay = hints.context(for: .screenplay)
        #expect(screenplay.sceneId == "scene-9" && screenplay.draftId == "draft-2")
        #expect(AILogic.defaultScope(screenplay) == .currentScene)
        // Another workspace does not inherit the screenplay's scene.
        let story = hints.context(for: .story)
        #expect(story.sceneId == nil && story.draftId == nil)
    }

    // MARK: Scenario 74: reviewing a change set

    @Test func actionsOnAnUnknownChangeSetSayWhatWentWrongAndChangeNothing() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        let ghost = set("does-not-exist", state: "Pending")
        #expect(await r.model.apply(ghost) == nil)
        let applyError = try #require(r.model.error)
        #expect(!applyError.isEmpty)
        #expect(r.model.busyChangeSet == nil)
        r.model.error = nil
        #expect(await r.model.reject(ghost) == nil)
        #expect(r.model.error?.isEmpty == false)
        r.model.error = nil
        #expect(await r.model.recheck(ghost) == nil)
        #expect(r.model.error?.isEmpty == false)
        #expect(r.model.busyChangeSet == nil)
        // Nothing was created.
        #expect(try await r.app.core!.tasksList(TaskListArgs()).isEmpty)
    }

    @Test func onlyTheNewestAppliedCardOffersUndo() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        #expect(r.model.latestAppliedChangeSetId == nil)
        r.model.exchanges = [
            exchange("a", set: set("cs-1", state: "Applied")),
            exchange("b", set: set("cs-2", state: "Applied")),
            exchange("c", set: set("cs-3", state: "Pending")),
            exchange("d", set: nil),
        ]
        #expect(r.model.latestAppliedChangeSetId == "cs-2")
        r.model.exchanges = [exchange("a", set: set("cs-1", state: "Rejected"))]
        #expect(r.model.latestAppliedChangeSetId == nil)
    }

    // MARK: Report AI output

    @Test func reportingAnOutputWritesAFileTheChosenPlaceAndSendsNothing() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        let file = r.root.appendingPathComponent("output.json")
        r.model.chooseSaveLocation = { _, _ in file }
        let x = exchange("How many scenes are in the script?", set: nil)
        let written = await r.model.reportOutput(x)
        #expect(written == file)
        let json = try JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any]
        #expect(json?["question"] as? String == "How many scenes are in the script?")
        #expect(r.model.error == nil)
    }

    @Test func cancellingTheSavePanelWritesNothing() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.chooseSaveLocation = { _, _ in nil }
        #expect(await r.model.reportOutput(exchange("q", set: nil)) == nil)
        #expect(r.model.error == nil)
    }

    @Test func aFileThatCannotBeSavedGivesAPlainMessage() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.chooseSaveLocation = { _, _ in URL(fileURLWithPath: "/nonexistent-folder/out.json") }
        #expect(await r.model.reportOutput(exchange("q", set: nil)) == nil)
        #expect(r.model.error?.contains("could not be saved") == true)
    }

    // MARK: The views draw

    @Test func thePanelDrawsInLightAndDarkWhenAIIsOff() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        for scheme in [ColorScheme.light, .dark] {
            let themes = ThemeStore()
            let view = AIPanelView(workspace: .home) {}
                .environment(r.app)
                .environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
                .environment(\.colorScheme, scheme)
            #expect(Fixture.render(view, name: "ai-panel-off-\(scheme)", size: CGSize(width: 360, height: 640)) != nil)
        }
    }

    @Test func theConversationAndChangeCardDraw() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.exchanges = [
            exchange("Create a task to scout the railway station.", set: set("cs-1", state: "Pending")),
            exchange("Mark the task as done.", set: set("cs-2", state: "Stale")),
            exchange("Which scene is at the bus stop?", set: set("cs-3", state: "Applied")),
        ]
        let themes = ThemeStore()
        let view = AIConversationView(model: r.model)
            .environment(r.app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
        #expect(Fixture.render(view, name: "ai-conversation", size: CGSize(width: 380, height: 760)) != nil)
    }
}

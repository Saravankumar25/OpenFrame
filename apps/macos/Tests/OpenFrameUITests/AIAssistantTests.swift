// AI assistant: the model driven exactly as the view drives it. Most tests use a real core on a
// temporary folder with no engine and no model files: every state the core reports without a model
// (engine missing, nothing downloaded), that a failed request never loses the typed question, the
// section picker (ADR-0041) and the change set review calls (scenario 74). The
// conversation tests (one id per conversation, progress, Cancel, Save conversation for review)
// answer the core calls with a stand-in (AITestSupport.swift), as the Windows Vitest files do.
// Questions that need a real model run in the Mac end-to-end scenarios (AIConversationScenarios).

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
        let model = AIAssistantModel(app: app)
        model.picks = [AISectionChoice(leaf: "screenplay/scenes")]
        return Rig(root: root, app: app, model: model)
    }

    /// The model with its core calls answered by `stub`, on an app with no core (no project needed).
    static func stubbed(_ stub: AIStubCore) throws -> Rig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        let model = AIAssistantModel(app: app)
        model.picks = [AISectionChoice(leaf: "screenplay/scenes")]
        model.invoker = stub
        stub.reply("ai.history", AiHistoryDto(conversationId: nil, exchanges: []))
        stub.reply("ai.status", AiStatusDto(available: true, mode: "AI Ready", bundleState: "Verified", problem: nil, runtimeState: "Ready", runtimeMessage: nil, localOnly: true))
        return Rig(root: root, app: app, model: model)
    }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }
}

private func exchange(_ text: String, set: AiChangeSetDto?) -> AiExchange {
    AIFixtures.exchange(text, conversation: "c", result: AIFixtures.result(kind: set == nil ? .answer : .proposal, content: "ok", changeSet: set))
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

    // MARK: The section picker (ADR-0041)

    private func answered(_ stub: AIStubCore, _ x: AiExchange) {
        stub.reply("ai.ask", x)
        stub.reply("ai.history", AiHistoryDto(conversationId: x.conversationId, exchanges: [x]))
    }

    private func labelled(_ label: String) -> AiExchange {
        var x = AIFixtures.exchange("What is the total?", conversation: "c1")
        x.scopeLabel = label
        return x
    }

    @Test func nothingIsPickedAtFirstAndSendWaitsForAPick() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.picks = []
        r.model.text = "How many scenes?"
        #expect(!r.model.canSend)
        #expect(!r.model.canAskNow)
        #expect(r.model.sendHint == "Pick a section")
        #expect(r.model.usingLabel == "Pick a section")
        await r.model.sendAndWait()
        #expect(r.model.pending == nil)
        #expect(r.model.text == "How many scenes?", "the question stays in the box")
        r.model.toggleSection("production/budget")
        #expect(r.model.canSend)
    }

    @Test func theWorkspaceNeverPicksASection() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        r.model.picks = []
        for ws in [Workspace.home, .story, .screenplay, .production, .callSheets] {
            r.model.workspace = ws
            #expect(r.model.picks.isEmpty, "\(ws)")
        }
    }

    @Test func theTreeComesFromTheCoreWithEverySection() async throws {
        let r = try await Rig.make()
        defer { r.close() }
        await r.model.loadSections()
        let sections = try #require(r.model.sections)
        #expect(sections.branches.first?.label == "Idea Vault")
        #expect(sections.maxPicks == 3)
        let leaves = sections.branches.flatMap(\.leaves)
        #expect(leaves.contains { $0.id == "production/cast-crew" && $0.title == "Cast & Crew" })
        #expect(leaves.first { $0.id == "story/episodes" }?.available == false, "a feature film has no episodes")
        #expect(leaves.first { $0.id == "screenplay/scenes" }?.picker == .screenplay)
    }

    @Test func thePicksGoWithEveryQuestionOfAConversationAndANewOneStartsEmpty() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        answered(stub, labelled("Using: Budget"))
        r.model.picks = [AISectionChoice(leaf: "production/budget")]
        r.model.text = "What is the total?"
        await r.model.sendAndWait()
        r.model.text = "And the camera?"
        await r.model.sendAndWait()
        let asks = stub.args("ai.ask")
        #expect(asks.count == 2)
        for a in asks {
            let sections = try #require(a["sections"] as? [[String: Any]])
            #expect(sections.map { $0["leaf"] as? String } == ["production/budget"])
            #expect(a["scope"] == nil, "the picks are all the core is sent")
        }
        #expect(r.model.picks.map(\.leaf) == ["production/budget"], "kept for the follow-up")
        r.model.newConversation()
        #expect(r.model.picks.isEmpty, "a new conversation starts empty")
        #expect(!r.model.canAskNow)
    }

    @Test func theAddButtonOfAPointerAddsTheSectionAndNeverTakesOneAway() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        r.model.picks = [AISectionChoice(leaf: "production/budget")]
        r.model.addPointedSection("production/cast-crew")
        #expect(r.model.picks.map(\.leaf) == ["production/budget", "production/cast-crew"])
        r.model.addPointedSection("production/cast-crew")
        #expect(r.model.picks.count == 2, "already picked: pressing it again does not remove it")
        r.model.addPointedSection("files/files")
        r.model.addPointedSection("notes/notes")
        #expect(r.model.picks.count == 3, "at three sections nothing is added")
        #expect(!r.model.picks.contains { $0.leaf == "notes/notes" })
    }

    @Test func aChosenInstanceGoesToTheCoreAndChangingTheDraftDropsTheScene() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        let tree = AiSections(
            branches: [], drafts: [], scenes: [AiSectionInstance(id: "old", label: "Scene 1")], days: [], callSheets: [], maxPicks: 3)
        stub.reply("ai.sections", tree)
        answered(stub, labelled("Using: Screenplay (Draft 1, Scene 1)"))
        r.model.picks = [AISectionChoice(leaf: "screenplay/scenes", sceneId: "gone")]
        await r.model.chooseDraft("d1", for: "screenplay/scenes")
        #expect(r.model.picks[0].sceneId == nil)
        #expect(r.model.draftScenes?.map(\.id) == ["old"])
        r.model.chooseScene("old", for: "screenplay/scenes")
        r.model.toggleSection("production/schedule")
        r.model.chooseDay("day3", for: "production/schedule")
        r.model.text = "What happens?"
        await r.model.sendAndWait()
        let sections = try #require(stub.args("ai.ask").first?["sections"] as? [[String: Any]])
        #expect(sections[0]["draftId"] as? String == "d1")
        #expect(sections[0]["sceneId"] as? String == "old")
        #expect(sections[1]["shootingDayId"] as? String == "day3")
        // A call sheet and a day of it are one choice.
        r.model.toggleSection("callsheets/callsheets")
        r.model.chooseDay("day3", for: "callsheets/callsheets")
        r.model.chooseCallSheet("cs", for: "callsheets/callsheets")
        let sheet = try #require(r.model.picks.first { $0.leaf == "callsheets/callsheets" })
        #expect(sheet.callSheetId == "cs" && sheet.shootingDayId == nil)
        r.model.removeSection("screenplay/scenes")
        #expect(r.model.draftScenes == nil, "the chosen draft's scenes go with its section")
    }

    @Test func theScenesOfAChosenScreenplayStayWhenAnotherSectionChanges() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        let drafts = [AiSectionDraft(id: "d2", screenplayId: "sp2", screenplayTitle: "Second", name: "Draft 1", isCurrent: true)]
        r.model.sections = AiSections(
            branches: [], drafts: drafts, scenes: [AiSectionInstance(id: "current", label: "Scene 1")], days: [], callSheets: [], maxPicks: 3)
        stub.reply("ai.sections", AiSections(
            branches: [], drafts: drafts, scenes: [AiSectionInstance(id: "second", label: "Scene 1")], days: [], callSheets: [], maxPicks: 3))
        r.model.picks = [AISectionChoice(leaf: "screenplay/scenes")]
        await r.model.chooseScreenplay("sp2", for: "screenplay/scenes")
        #expect(r.model.picks[0].draftId == nil, "a chosen screenplay has no draft id")
        #expect(r.model.draftScenes?.map(\.id) == ["second"])
        r.model.toggleSection("files/files")
        r.model.removeSection("files/files")
        #expect(r.model.draftScenes?.map(\.id) == ["second"], "the list of the chosen screenplay stays")
        r.model.removeSection("screenplay/scenes")
        #expect(r.model.draftScenes == nil)
    }

    @Test func theLineShowsTheStoredLabelUntilThePicksChange() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        answered(stub, labelled("Using: Budget (from the core)"))
        r.model.picks = [AISectionChoice(leaf: "production/budget")]
        r.model.text = "What is the total?"
        await r.model.sendAndWait()
        #expect(r.model.usingLabel == "Using: Budget (from the core)")
        r.model.toggleSection("files/files")
        #expect(r.model.usingLabel.hasPrefix("Using: production/budget"), "the picks changed: \(r.model.usingLabel)")
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

// MARK: - The conversation (Windows: AiProgress.test.tsx, SaveConversation.test.tsx)

@Suite("AI conversation", .serialized)
@MainActor
struct AIConversationTests {
    private func answered(_ stub: AIStubCore, _ x: AiExchange) {
        stub.reply("ai.ask", x)
        stub.reply("ai.history", AiHistoryDto(conversationId: x.conversationId, exchanges: [x]))
    }

    // MARK: One id per conversation

    @Test func theConversationIsNamedBeforeTheFirstAnswerAndKeptAfterIt() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        await r.model.loadHistory()
        answered(stub, AIFixtures.exchange("How many scenes?", conversation: "core-conv"))
        r.model.text = "How many scenes?"
        await r.model.sendAndWait()
        let first = try #require(stub.args("ai.ask").first?["conversationId"] as? String)
        #expect(first.count >= 8 && first.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" })
        // The next question reuses the id the core returned, not a new one.
        r.model.text = "And the drafts?"
        await r.model.sendAndWait()
        #expect(stub.args("ai.ask").last?["conversationId"] as? String == "core-conv")
        #expect(stub.args("ai.history").last?["conversationId"] as? String == "core-conv")
    }

    @Test func openingThePanelContinuesTheLatestConversation() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        let earlier = AIFixtures.exchange("How many scenes?", conversation: "latest")
        stub.reply("ai.history", AiHistoryDto(conversationId: "latest", exchanges: [earlier]))
        await r.model.loadHistory()
        // The earlier turns are loaded on open, and a question goes on in that conversation.
        #expect(r.model.exchanges.map(\.requestText) == ["How many scenes?"])
        #expect(r.model.conversationForAsk == "latest")
        // New starts another one, with its own id.
        r.model.newConversation()
        #expect(r.model.exchanges.isEmpty)
        let fresh = r.model.conversationForAsk
        #expect(fresh != "latest")
    }

    // MARK: Progress

    @Test func showsTheWaitingLineThenOnlyThisConversationsLatestLabel() async throws {
        let stub = AIStubCore()
        stub.hold("ai.ask")
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        var heard: [String] = []
        r.model.announce = { heard.append($0) }
        r.model.text = "Walk me through the story"
        r.model.send()
        await eventually { stub.count("ai.ask") == 1 }
        let id = try #require(r.model.running)
        #expect(stub.args("ai.ask").first?["conversationId"] as? String == id)
        #expect(r.model.workingLine == "Working on it on this computer\u{2026}")
        r.model.receive(AIFixtures.progress(id, "Reading Scenes 5 and 6", step: 1, total: 3))
        #expect(r.model.workingLine == "Reading Scenes 5 and 6")
        // Another conversation and other events change nothing.
        r.model.receive(AIFixtures.progress("someone-else", "Reading Scene 99"))
        r.model.receive(AIFixtures.progress(id, "Not mine", module: "other"))
        #expect(r.model.workingLine == "Reading Scenes 5 and 6")
        r.model.receive(AIFixtures.progress(id, "Writing the answer", step: 2, total: 3))
        r.model.receive(AIFixtures.progress(id, "Writing the answer", step: 2, total: 3))
        #expect(r.model.workingLine == "Writing the answer")
        // Each new line is read out once.
        #expect(heard == ["Reading Scenes 5 and 6", "Writing the answer"])
        answered(stub, AIFixtures.exchange("Walk me through the story", conversation: id))
        stub.release("ai.ask")
        await r.model.waitForAnswer()
        #expect(r.model.pending == nil && r.model.running == nil && r.model.progress == nil)
        // A late line after the answer is ignored.
        r.model.receive(AIFixtures.progress(id, "Late"))
        #expect(r.model.progress == nil)
    }

    // MARK: Cancel

    @Test func cancelAsksTheCoreOnceSaysStoppingAndTheAnswerThenArrives() async throws {
        let stub = AIStubCore()
        stub.hold("ai.ask")
        stub.reply("ai.cancel", AiCancelDto(stopping: true))
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        r.model.text = "Walk me through the story"
        r.model.send()
        await eventually { stub.count("ai.ask") == 1 }
        let id = try #require(r.model.running)
        await r.model.cancel()
        #expect(stub.args("ai.cancel").count == 1)
        #expect(stub.args("ai.cancel").first?["conversationId"] as? String == id)
        #expect(r.model.stopping)
        #expect(r.model.workingLine == "Stopping\u{2026}")
        // A second press while stopping does not ask again.
        await r.model.cancel()
        #expect(stub.count("ai.cancel") == 1)
        let stopped = AIFixtures.exchange("Walk me through the story", conversation: id, result: AIFixtures.result(content: "Stopped. Nothing was changed."))
        answered(stub, stopped)
        stub.release("ai.ask")
        await r.model.waitForAnswer()
        #expect(r.model.pending == nil)
        #expect(!r.model.stopping)
        #expect(r.model.exchanges.last?.result.content == "Stopped. Nothing was changed.")
        // The answer arrived as an ordinary answer; the box is free and empty.
        #expect(r.model.text.isEmpty && r.model.error == nil)
        // Cancel with nothing running does nothing.
        await r.model.cancel()
        #expect(stub.count("ai.cancel") == 1)
    }

    @Test func cancelCanBePressedAgainWhenNothingCouldBeStoppedYet() async throws {
        let stub = AIStubCore()
        stub.hold("ai.ask")
        stub.reply("ai.cancel", AiCancelDto(stopping: false))
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        r.model.text = "Summarise act two"
        r.model.send()
        await eventually { stub.count("ai.ask") == 1 }
        await r.model.cancel()
        #expect(!r.model.stopping)
        await r.model.cancel()
        #expect(stub.count("ai.cancel") == 2)
        // A failed stop says why and leaves Cancel usable.
        stub.fail("ai.cancel", CoreFailure(code: "ai.x", message: "The assistant could not be reached."))
        await r.model.cancel()
        #expect(!r.model.stopping)
        #expect(r.model.error == "The assistant could not be reached.")
        stub.fail("ai.ask", CoreFailure(code: "ai.x", message: "Stopped"))
        stub.release("ai.ask")
        await r.model.waitForAnswer()
        // A request that fails puts the question back.
        #expect(r.model.text == "Summarise act two")
    }

    // MARK: The end of a request (Windows clears pending and running together)

    @Test func theQuestionStaysPendingUntilTheAnswerIsOnScreen() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        answered(stub, AIFixtures.exchange("How many scenes?", conversation: "conv"))
        stub.hold("ai.history")
        r.model.text = "How many scenes?"
        r.model.send()
        await eventually { stub.count("ai.history") == 1 }
        // The answer is in but not yet shown: the request still runs, so no second one can start.
        #expect(r.model.pending == "How many scenes?")
        #expect(r.model.running != nil)
        r.model.text = "And the drafts?"
        r.model.send()
        #expect(stub.count("ai.ask") == 1)
        #expect(r.model.text == "And the drafts?")
        stub.release("ai.history")
        await r.model.waitForAnswer()
        #expect(r.model.pending == nil && r.model.running == nil)
        #expect(r.model.exchanges.map(\.requestText) == ["How many scenes?"])
    }

    @Test func anEarlierRequestEndingLateLeavesTheNewOneRunning() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        answered(stub, AIFixtures.exchange("How many scenes?", conversation: "conv"))
        // The first request finishes, then waits for the status it reads last.
        stub.hold("ai.status")
        r.model.text = "How many scenes?"
        r.model.send()
        await eventually { stub.count("ai.status") == 1 && r.model.pending == nil }
        // A second question in the same conversation starts and is held by the core.
        stub.hold("ai.ask")
        r.model.text = "And the drafts?"
        r.model.send()
        await eventually { stub.count("ai.ask") == 2 }
        let second = try #require(r.model.running)
        // The first request now ends: the second keeps its Working line, its progress and its Cancel.
        stub.release("ai.status")
        try await Task.sleep(for: .milliseconds(50))
        #expect(r.model.pending == "And the drafts?")
        #expect(r.model.running == second)
        r.model.receive(AIFixtures.progress(second, "Reading Scenes 5 and 6"))
        #expect(r.model.workingLine == "Reading Scenes 5 and 6")
        stub.reply("ai.cancel", AiCancelDto(stopping: true))
        await r.model.cancel()
        #expect(stub.args("ai.cancel").first?["conversationId"] as? String == second)
        stub.release("ai.ask")
        await r.model.waitForAnswer()
        #expect(r.model.pending == nil && r.model.running == nil && !r.model.stopping)
    }

    @Test func escapeElsewhereInTheWindowDoesNotCancelTheRequest() throws {
        // Windows binds no key to this Cancel. A window-wide Escape would stop the request from the
        // screenplay or the Story board, which use Escape for their own work.
        let view = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Sources/OpenFrameUI/AI/AIConversationView.swift")
        let source = try String(contentsOf: view, encoding: .utf8)
        #expect(source.contains("\"ai-cancel\""))
        #expect(!source.contains(".cancelAction"))
        #expect(!source.contains("(Esc)"))
    }

    // MARK: Save conversation for review

    @Test func saveIsOffWhileThereIsNoConversation() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        var asked = 0
        r.model.chooseSaveLocation = { _, _ in asked += 1; return nil }
        await r.model.loadHistory()
        #expect(r.model.conversationId == nil)
        #expect(await r.model.saveForReview() == nil)
        #expect(asked == 0)
        #expect(stub.count("ai.export_conversation") == 0)
    }

    @Test func saveAsksWhereAndWritesOnlyAfterThePersonChooses() async throws {
        let stub = AIStubCore()
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        let xs = [AIFixtures.exchange("How many scenes?"), AIFixtures.exchange("And the drafts?")]
        stub.reply("ai.history", AiHistoryDto(conversationId: "conv1", exchanges: xs))
        stub.reply("ai.export_conversation", AiReportResult(fileName: "openframe-ai-conversation.json", rows: 2))
        await r.model.loadHistory()
        // Showing the conversation writes nothing.
        #expect(stub.count("ai.export_conversation") == 0)
        var titles: [String] = []
        let file = r.root.appendingPathComponent("openframe-ai-conversation.json")
        r.model.chooseSaveLocation = { title, name in
            titles.append(title)
            #expect(name == "openframe-ai-conversation.json")
            return file
        }
        let saved = await r.model.saveForReview()
        #expect(saved?.rows == 2)
        #expect(titles == ["Save conversation for review"])
        let args = try #require(stub.args("ai.export_conversation").first)
        #expect(args["conversationId"] as? String == "conv1")
        #expect(args["path"] as? String == file.path)
        #expect(ToastCenter.shared.items.last?.text == "Saved openframe-ai-conversation.json (2 questions).")
        // Cancelling the Save panel writes nothing.
        r.model.chooseSaveLocation = { _, _ in nil }
        #expect(await r.model.saveForReview() == nil)
        #expect(stub.count("ai.export_conversation") == 1)
        // A file that cannot be written says why, from the core.
        stub.fail("ai.export_conversation", CoreFailure(code: "storage.write", message: "The file could not be saved."))
        r.model.chooseSaveLocation = { _, _ in file }
        #expect(await r.model.saveForReview() == nil)
        #expect(ToastCenter.shared.items.last?.text == "The file could not be saved.")
        #expect(ToastCenter.shared.items.last?.kind == .error)
    }

    @Test func theConversationWithSourcesAndProgressDraws() async throws {
        let stub = AIStubCore()
        stub.hold("ai.ask")
        let r = try Rig.stubbed(stub)
        defer { r.close() }
        r.model.exchanges = [
            AIFixtures.exchange("Who plays Anna?", result: AIFixtures.result(
                content: "Maya Rao plays Anna.\n\n- Scene 4\n- Scene 9\n\nIdea: open on her hands.", confidence: .inferred,
                provenance: [AiProvenance(kind: "Cast", label: "Maya Rao", nav: AiNavTarget(workspace: "production", sub: "cast-crew", params: [:]))])),
            AIFixtures.exchange("List the scenes", result: AIFixtures.result(
                content: "Two scenes.", items: [AiResultItem(label: "Scene 1"), AiResultItem(label: "Scene 2")], itemsAreAnswer: true)),
        ]
        r.model.text = "Walk me through the story"
        r.model.send()
        await eventually { stub.count("ai.ask") == 1 }
        r.model.receive(AIFixtures.progress(try #require(r.model.running), "Reading Scenes 5 and 6"))
        for scheme in [ColorScheme.light, .dark] {
            let themes = ThemeStore()
            let view = AIConversationView(model: r.model)
                .environment(r.app)
                .environment(themes)
                .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
                .environment(\.colorScheme, scheme)
            #expect(Fixture.render(view, name: "ai-conversation-progress-\(scheme)", size: CGSize(width: 380, height: 760)) != nil)
        }
        stub.fail("ai.ask", CoreFailure(code: "ai.x", message: "x"))
        stub.release("ai.ask")
        await r.model.waitForAnswer()
    }
}

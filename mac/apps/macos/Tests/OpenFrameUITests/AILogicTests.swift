// AI screens: the pure view logic (scopes per workspace, panel mode, texts, states). Scenarios 72,
// 73, 74, 76 and 78 are about these decisions; the core-backed ones are in AIAssistantTests and
// AIDownloadTests.

import Foundation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

private func status(
    available: Bool = false, mode: String = "Off", bundle: String = "NotInstalled", problem: (String, String)? = nil,
    runtime: String = "Unavailable"
) -> AiStatusDto {
    AiStatusDto(
        available: available, mode: mode, bundleState: bundle,
        problem: problem.map { AppError(code: $0.0, message: $0.1, retryable: false) },
        runtimeState: runtime, runtimeMessage: nil, localOnly: true)
}

private func changeSet(state: String, ops: Double = 1, applied: Double = 0, confirm: Bool = false, error: String? = nil) -> AiChangeSetDto {
    AiChangeSetDto(
        id: "cs1", title: "Create task", summary: "One task", state: state, validationState: "Valid", rows: [], exclusions: [],
        affectedModules: ["Notes & Tasks"], targets: [], operationCount: ops, staleReason: nil, errorMessage: error,
        createdAt: 0, appliedOperations: applied, partCount: 1, requiresConfirmation: confirm)
}

@Suite("AI logic")
struct AILogicTests {
    // MARK: Scenario 73: the scope of each workspace

    @Test func theAssistantTakesTheScopeOfEachWorkspace() {
        func scope(_ ws: Workspace, scene: String? = nil) -> AiScopeKind {
            AILogic.defaultScope(AIScopeContext(workspace: ws, sceneId: scene))
        }
        #expect(scope(.home) == .wholeProject)
        #expect(scope(.ideaVault) == .wholeProject)
        #expect(scope(.story) == .storyBoard)
        #expect(scope(.breakdown) == .production)
        #expect(scope(.production) == .production)
        #expect(scope(.callSheets) == .production)
        #expect(scope(.screenplay) == .currentScreenplay)
        #expect(scope(.screenplay, scene: "s1") == .currentScene)
        #expect(scope(.notes) == .wholeProject)
        #expect(scope(.files) == .wholeProject)
        #expect(AiScopeKind.wholeProject.label == "Whole Project")
        #expect(AiScopeKind.storyBoard.label == "Beat Board")
        #expect(AiScopeKind.production.label == "Production Workspace")
    }

    @Test func aScopeThatNeedsATargetIsOfferedOnlyWhenThereIsOne() {
        let plain = AILogic.scopeOptions(AIScopeContext(workspace: .home))
        #expect(plain == [.currentScreenplay, .storyBoard, .production, .wholeProject])
        let rich = AILogic.scopeOptions(
            AIScopeContext(workspace: .screenplay, draftId: "d", sceneId: "s", selectionIds: ["c1"], shootingDayId: "sd", callSheetId: "cs"))
        #expect(rich == [.currentSelection, .currentScene, .currentScreenplay, .specificDraft, .storyBoard, .production, .shootingDay, .callSheet, .wholeProject])
        let vault = AILogic.scopeOptions(AIScopeContext(workspace: .ideaVault, selectionIds: ["i1"]))
        #expect(vault.first == .ideaVaultSelection)
        #expect(AILogic.defaultScope(AIScopeContext(workspace: .ideaVault, selectionIds: ["i1"])) == .ideaVaultSelection)
        #expect(AILogic.defaultScope(AIScopeContext(workspace: .callSheets, callSheetId: "cs")) == .callSheet)
    }

    @Test func scopeArgumentsAreReferencesOnly() {
        let c = AIScopeContext(workspace: .screenplay, draftId: "d1", sceneId: "s1", selectionIds: ["a", "b"])
        let scene = AILogic.scopeArgs(.currentScene, c)
        #expect(scene.sceneId == "s1" && scene.draftId == nil && scene.selection.isEmpty)
        let sel = AILogic.scopeArgs(.currentSelection, c)
        #expect(sel.selection.map(\.id) == ["a", "b"])
        let play = AILogic.scopeArgs(.currentScreenplay, c)
        #expect(play.draftId == "d1")
        // The open draft is used only in the Screenplay workspace, never guessed elsewhere.
        let bare = AIScopeContext(workspace: .screenplay)
        #expect(AILogic.scopeArgs(.currentScreenplay, bare, fallbackDraftId: "open").draftId == "open")
        #expect(AILogic.scopeArgs(.currentScreenplay, AIScopeContext(workspace: .home), fallbackDraftId: "open").draftId == nil)
        #expect(AILogic.scopeArgs(.wholeProject, c).draftId == nil)
    }

    // MARK: Scenarios 76 and 78: what the panel shows for each state of the files

    @Test func panelModeFollowsTheFiles() {
        #expect(AILogic.panelMode(nil) == .unavailable)
        #expect(AILogic.panelMode(status(available: true, mode: "AI Ready", bundle: "Verified", runtime: "Ready")) == .ready)
        #expect(AILogic.panelMode(status(bundle: "NotInstalled")) == .download)
        let damaged = status(bundle: "Damaged", problem: ("ai.models_damaged", "An OpenFrame AI file is missing or damaged. Download it again."))
        #expect(AILogic.panelMode(damaged) == .download)
        #expect(AILogic.damagedModels(damaged)?.contains("missing or damaged") == true)
        let engine = status(bundle: "Damaged", problem: ("ai.runtime_damaged", "Reinstall OpenFrame Studio to repair it."))
        #expect(AILogic.panelMode(engine) == .off)
        #expect(AILogic.offReason(engine).contains("Reinstall OpenFrame Studio"))
        #expect(AILogic.damagedModels(engine) == nil)
        #expect(AILogic.offReason(status(bundle: "Unsupported")) == AIText.unavailable)
    }

    @Test func settingsStateTextMatchesWindows() {
        #expect(AILogic.stateText(status(bundle: "NotInstalled")) == "Not downloaded")
        #expect(AILogic.stateText(status(bundle: "Damaged", problem: ("ai.models_damaged", "x"))) == "Needs a new download")
        #expect(AILogic.stateText(status(bundle: "Missing", problem: ("ai.runtime_missing", "x"))) == "Needs a reinstall")
        #expect(AILogic.stateText(status(bundle: "Unsupported")) == "Not available on this computer")
        #expect(AILogic.stateText(status(available: true, bundle: "Verified", runtime: "Ready")) == "AI Ready")
        #expect(AILogic.stateText(status(available: true, bundle: "Verified", runtime: "Starting")).hasPrefix("Getting Offline AI ready"))
        #expect(AILogic.stateText(status(available: true, bundle: "Verified", runtime: "Failed")).hasPrefix("Stopped after a problem"))
        #expect(AILogic.fileCheckText("Damaged") == "changed or damaged")
        #expect(AILogic.fileCheckText("Missing").contains("reinstall"))
    }

    // MARK: Scenario 74: change set states

    @Test func changeSetActionsByState() {
        func a(_ s: String) -> AILogic.ChangeSetActions { AILogic.changeSetActions(changeSet(state: s)) }
        #expect(a("Pending") == .init(apply: true, reject: true, recheck: false))
        // A proposal whose target changed is never applied blindly: only Re-check and Reject remain.
        #expect(a("Stale") == .init(apply: false, reject: true, recheck: true))
        #expect(a("Conflict") == .init(apply: false, reject: true, recheck: true))
        for s in ["Applied", "Rejected", "Accepted", "Failed"] {
            #expect(a(s) == .init(apply: false, reject: false, recheck: false), "state \(s)")
        }
        #expect(AILogic.isStale(changeSet(state: "Stale")) && AILogic.isStale(changeSet(state: "Conflict")))
        #expect(!AILogic.isStale(changeSet(state: "Pending")))
    }

    @Test func changeSetTextsMatchTheScenario() {
        #expect(AILogic.changeSetStateText(changeSet(state: "Applied", applied: 1)) == "Applied 1 change. Use Undo to reverse it.")
        #expect(AILogic.changeSetStateText(changeSet(state: "Applied", applied: 3)) == "Applied 3 changes. Use Undo to reverse it.")
        #expect(AILogic.changeSetStateText(changeSet(state: "Rejected")) == "Rejected. Nothing was changed.")
        #expect(AILogic.changeSetStateText(changeSet(state: "Accepted")) == "Applying\u{2026}")
        #expect(AILogic.changeSetStateText(changeSet(state: "Failed")) == "This proposal couldn't be applied. Nothing was changed.")
        #expect(AILogic.changeSetStateText(changeSet(state: "Failed", error: "The task was removed.")) == "The task was removed.")
        #expect(AILogic.changeSetStateText(changeSet(state: "Pending")) == nil)
        #expect(AILogic.changeCount(1) == "1 change")
        #expect(AILogic.changeCount(4) == "4 changes")
    }

    // MARK: Answers

    @Test func sourcesAreTheProvenanceAsWindowsShowsIt() {
        let chips = AILogic.sourceChips([
            AiProvenance(kind: "Scope", label: "Using: Whole Project"),
            AiProvenance(kind: "Scene", label: "BUS STOP", nav: AiNavTarget(workspace: "screenplay", params: ["sceneId": "s1"])),
            AiProvenance(kind: "Cast", label: "Maya Rao", nav: AiNavTarget(workspace: "production", sub: "cast-crew", params: [:])),
            AiProvenance(kind: "Draft", label: "Draft 1", nav: AiNavTarget(workspace: "elsewhere", params: [:])),
        ])
        // A scope or basis shows its label alone; any other source names its kind.
        #expect(chips.map(\.text) == ["Using: Whole Project", "Scene: BUS STOP", "Cast: Maya Rao", "Draft: Draft 1"])
        #expect(Set(chips.map(\.id)).count == 4)
        #expect(chips[1].route?.workspace == .screenplay)
        #expect(chips[1].route?.params["sceneId"] == "s1")
        #expect(chips[2].route?.tab == .castCrew)
        // An unknown workspace is ignored, never guessed.
        #expect(chips[3].route == nil)
    }

    @Test func confidenceAndStepsWordsMatchWindows() {
        #expect(AILogic.confidenceLabel(.exact) == "Exact \u{00B7} from project data")
        #expect(AILogic.confidenceLabel(.inferred) == "Inferred \u{00B7} written by Offline AI")
        #expect(AILogic.confidenceLabel(.unavailable) == "Not available in project data")
        #expect(AILogic.confidenceLabel(nil) == nil)
        func step(_ label: String) -> AiStepDto { AiStepDto(index: 0, tool: "t", label: label, status: "Succeeded") }
        #expect(AILogic.stepsSummary([]) == nil)
        #expect(AILogic.stepsSummary([step("Count scenes")]) == nil)
        #expect(AILogic.stepsSummary([step("Find the scene"), step("Read it")]) == "2 steps: Find the scene \u{00B7} Read it")
    }

    // Windows ResultView.test.tsx: what sits under the answer and what waits behind See sources.

    @Test func anAnswerShowsItsTextWithTheMetadataBehindSeeSources() {
        let r = AIFixtures.result(
            content: "There are 130 scenes in the current screenplay.", details: ["From the screenplay rows."], confidence: .exact,
            provenance: [AiProvenance(kind: "Draft", label: "ENE")])
        let l = AILogic.layout(r)
        #expect(!l.reasonsVisible)
        #expect(l.hasSources)
        #expect(!l.itemsInline && !l.itemsAsSources)
    }

    @Test func theRowsOfACountStayUnderSeeSources() {
        let r = AIFixtures.result(items: [AiResultItem(label: "Scene 1: INT. KITCHEN - DAY")], itemsAreAnswer: false)
        let l = AILogic.layout(r)
        #expect(l.itemsAsSources && !l.itemsInline && l.hasSources)
    }

    @Test func aRequestedListStaysUnderTheAnswerWithNoEmptySeeSources() {
        let r = AIFixtures.result(items: [AiResultItem(label: "Scene 1: INT. KITCHEN - DAY")], itemsAreAnswer: true)
        let l = AILogic.layout(r)
        #expect(l.itemsInline && !l.itemsAsSources)
        #expect(!l.hasSources)
    }

    @Test func theReasonsOfARefusalStayInView() {
        let r = AIFixtures.result(kind: .denied, content: "I can't do that.", details: ["You are a Viewer on this project."])
        let l = AILogic.layout(r)
        #expect(l.reasonsVisible)
        // The reasons are in view, so they are not repeated behind See sources.
        #expect(!l.hasSources)
        #expect(AILogic.layout(AIFixtures.result(kind: .answer, details: ["How it was worked out."])).hasSources)
    }

    @Test func anAnswerNeverShowsACoverageLine() {
        // The "Used N of N scenes" line is gone on Windows; nothing here writes one.
        let r = AIFixtures.result(content: "Lead cast, planned: 40,00,000.", confidence: .exact)
        #expect(AIAnswerText.parse(r.content) == [.paragraph(["Lead cast, planned: 40,00,000."])])
        #expect(!AILogic.copyText(r).contains("Used "))
    }

    @Test func longListsShowAPreviewAndTheWholeListOnRequest() {
        let items = (1...30).map { AiResultItem(label: "Scene \($0)") }
        #expect(AILogic.visibleItems(items, all: false).count == AILogic.previewItems)
        #expect(AILogic.visibleItems(items, all: true).count == 30)
        #expect(AILogic.visibleItems(Array(items.prefix(3)), all: false).count == 3)
    }

    @Test func onlyRefusalsFailuresAndQuestionsShowTheirReasons() {
        for kind in [AiResultKind.denied, .failed, .unavailable, .clarify, .private] { #expect(AILogic.reasonsVisible(kind), "\(kind)") }
        for kind in [AiResultKind.answer, .navigate, .suggestion, .proposal] { #expect(!AILogic.reasonsVisible(kind), "\(kind)") }
    }

    // MARK: Navigation: every target the core sends

    @Test func everyWorkspaceTheCoreNamesOpensItsMacWorkspace() {
        let expected: [(String, Workspace)] = [
            ("home", .home), ("vault", .ideaVault), ("story", .story), ("screenplay", .screenplay), ("breakdown", .breakdown),
            ("production", .production), ("callsheets", .callSheets), ("files", .files), ("trash", .recentlyDeleted),
            ("activity", .activity), ("notes", .notes),
        ]
        for (id, ws) in expected {
            #expect(AIRoute.workspace(for: id) == ws, "\(id)")
            #expect(AIRoute(AiNavTarget(workspace: id, params: [:]))?.workspace == ws, "\(id)")
        }
        #expect(AIRoute(nil) == nil)
        // The product guide ("help") and anything unknown have no Mac place: ignored.
        #expect(AIRoute(AiNavTarget(workspace: "help", params: [:])) == nil)
        #expect(AIRoute(AiNavTarget(workspace: "elsewhere", params: [:])) == nil)
    }

    @Test func aProductionLinkOpensTheTabItsSubNames() {
        // The core sends the tab as `sub`, beside the parameters (never inside them).
        let tabs: [(String, ProductionTab)] = [
            ("budget", .budget), ("schedule", .schedule), ("cast-crew", .castCrew), ("locations", .locations), ("catalog", .catalog),
            ("shots", .shotLists), ("moodboards", .moodboards), ("storyboards", .storyboards), ("dood", .dayOutOfDays),
            ("sides", .sides), ("reports", .reports), ("daily", .daily), ("overview", .overview),
        ]
        for (sub, tab) in tabs {
            let r = AIRoute(AiNavTarget(workspace: "production", sub: sub, params: ["shotId": "x"]))
            #expect(r?.target == .place(ProjectPlace(.production, tab: tab)), "\(sub)")
            #expect(r?.params["shotId"] == "x")
        }
        #expect(AIRoute(AiNavTarget(workspace: "production", params: [:]))?.tab == .overview)
        #expect(AIRoute(AiNavTarget(workspace: "production", sub: "unknown", params: [:]))?.tab == .overview)
        #expect(AIRoute(AiNavTarget(workspace: "screenplay", params: [:]))?.tab == nil)
        // A parameter named "sub" is not the tab.
        #expect(AIRoute(AiNavTarget(workspace: "production", params: ["sub": "budget"]))?.tab == .overview)
    }

    @Test func storyAndSettingsLinksOpenTheirTabs() {
        #expect(AIRoute(AiNavTarget(workspace: "story", sub: "characters", params: ["characterId": "c1"]))?.target == .story(.characters))
        #expect(AIRoute(AiNavTarget(workspace: "story", sub: "episodes", params: [:]))?.target == .story(.episodes))
        #expect(AIRoute(AiNavTarget(workspace: "story", params: [:]))?.target == .story(.page))
        #expect(AIRoute.storyIntent(.characters) == "story.show_characters")
        #expect(AIRoute(AiNavTarget(workspace: "settings", params: [:]))?.target == .projectSettings(.general))
        #expect(AIRoute(AiNavTarget(workspace: "settings", sub: "templates", params: [:]))?.target == .projectSettings(.templates))
        #expect(AIRoute(AiNavTarget(workspace: "settings", sub: "offline-ai", params: [:]))?.target == .projectSettings(.offlineAI))
        #expect(AIRoute(AiNavTarget(workspace: "settings", sub: "storage", params: [:]))?.target == .projectSettings(.storage))
        #expect(AIRoute(AiNavTarget(workspace: "settings", sub: "unknown", params: [:]))?.target == .projectSettings(.general))
        #expect(AIRoute(AiNavTarget(workspace: "settings", params: [:]))?.workspace == nil)
    }

    @Test func aStoryLinkWithNoTabOpensTheTabItsIdsPointTo() {
        // Story search hits come as a link with a card id and no tab: Windows opens the Beat Board.
        #expect(AIRoute(AiNavTarget(workspace: "story", params: ["cardId": "c1"]))?.target == .story(.board))
        #expect(AIRoute(AiNavTarget(workspace: "story", params: ["beatId": "b1"]))?.target == .story(.board))
        #expect(AIRoute(AiNavTarget(workspace: "story", params: ["characterId": "k1"]))?.target == .story(.characters))
        #expect(AIRoute(AiNavTarget(workspace: "story", sub: "unknown", params: ["actId": "a1"]))?.target == .story(.board))
        #expect(AIRoute(AiNavTarget(workspace: "story", params: ["episodeId": "e1"]))?.target == .story(.page))
        // A named tab still wins over the ids.
        #expect(AIRoute(AiNavTarget(workspace: "story", sub: "outline", params: ["cardId": "c1"]))?.target == .story(.outline))
        #expect(AIRoute(AiNavTarget(workspace: "story", params: ["cardId": "c1"]))?.params["cardId"] == "c1")
    }

    @MainActor
    @Test func performingALinkMovesTheWindow() {
        final class Seen: @unchecked Sendable { var shown: [String] = []; var settings: [String?] = [] }
        let seen = Seen()
        let o = NotificationCenter.default.addObserver(forName: .openFrameShowWorkspace, object: nil, queue: nil) { n in
            if let s = n.object as? String { seen.shown.append(s) }
        }
        let p = NotificationCenter.default.addObserver(forName: .openFrameShowProjectSettings, object: nil, queue: nil) { n in
            seen.settings.append(n.object as? String)
        }
        defer { NotificationCenter.default.removeObserver(o); NotificationCenter.default.removeObserver(p) }
        AIRoute(AiNavTarget(workspace: "production", sub: "budget", params: [:]))?.perform()
        AIRoute(AiNavTarget(workspace: "callsheets", params: [:]))?.perform()
        AIRoute(AiNavTarget(workspace: "story", sub: "characters", params: [:]))?.perform()
        AIRoute(AiNavTarget(workspace: "settings", sub: "templates", params: [:]))?.perform()
        #expect(seen.shown == ["production/budget", "callSheets", "story"])
        #expect(seen.settings == ["templates"])
        #expect(ProjectIntents.shared.consume("story.show_characters"))
        // A story card link with no tab asks Story for the Beat Board.
        AIRoute(AiNavTarget(workspace: "story", params: ["cardId": "c1"]))?.perform()
        #expect(ProjectIntents.shared.consume("story.show_board"))
    }

    // MARK: Progress of the running conversation

    @Test func progressLinesOfOnlyTheRunningConversationCount() {
        let mine = AILogic.progress(AIFixtures.progress("c1", "Reading Scenes 5 and 6", step: 1, total: 3), conversation: "c1")
        #expect(mine == AIProgress(label: "Reading Scenes 5 and 6", step: 1, total: 3))
        #expect(AILogic.progress(AIFixtures.progress("someone-else", "Reading Scene 99"), conversation: "c1") == nil)
        #expect(AILogic.progress(AIFixtures.progress("c1", "Not mine", module: "other"), conversation: "c1") == nil)
        #expect(AILogic.progress(AIFixtures.progress("c1", "Not mine", name: "changed"), conversation: "c1") == nil)
        #expect(AILogic.progress(AIFixtures.progress("c1", ""), conversation: "c1") == nil)
        #expect(AILogic.progress(AIFixtures.progress("c1", "Reading"), conversation: nil) == nil)
        #expect(AILogic.progress(.dataChanged(store: .project, tables: ["ai_request"], ids: [], origin: "c1"), conversation: "c1") == nil)
    }

    @Test func aNewConversationIdIsAShortPlainToken() {
        let id = AILogic.newConversationId()
        #expect(id.count >= 8 && id.count <= 64)
        #expect(id.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" })
        #expect(AILogic.newConversationId() != id)
    }

    @Test func theSaveMessageCountsTheQuestions() {
        #expect(AILogic.savedMessage(AiReportResult(fileName: "a.json", rows: 2)) == "Saved a.json (2 questions).")
        #expect(AILogic.savedMessage(AiReportResult(fileName: "a.json", rows: 1)) == "Saved a.json (1 question).")
    }

    @Test func indexNoticeShowsOnlyWhileContextIsRebuilt() {
        func index(_ s: AiIndexState) -> AiIndexStatus {
            AiIndexStatus(
                state: s, preparing: false, semanticReady: false, documents: 0, chunks: 0, embeddedChunks: 0, graphNodes: 0,
                graphEdges: 0, pendingChanges: 0, pendingEmbeddings: 0)
        }
        #expect(AILogic.indexNotice(index(.rebuilding))?.preparing == "Preparing project context\u{2026}")
        #expect(AILogic.indexNotice(index(.stale)) != nil)
        for s in [AiIndexState.current, .updating, .failed, .unavailable] { #expect(AILogic.indexNotice(index(s)) == nil) }
        #expect(AILogic.indexNotice(nil) == nil)
        #expect(AILogic.runtimeLabel("LoadingModel") == "Getting Offline AI ready on this computer\u{2026}")
        #expect(AILogic.runtimeLabel("Busy") == "Working on a request\u{2026}")
        #expect(AILogic.runtimeLabel("Ready") == nil)
    }

    // MARK: Scenario 78: the download card

    private func download(_ phase: String, done: Double = 0, total: Double = 0, free: Double? = 50e9, problem: AppError? = nil) -> AiDownloadDto {
        AiDownloadDto(canDownload: true, phase: phase, sizeBytes: 5.5e9, neededBytes: 6e9, freeBytes: free, doneBytes: done, totalBytes: total, problem: problem)
    }

    @Test func theDownloadLineSaysSizeAndSpaceBeforeTheClick() {
        let line = AILogic.downloadLine(download("Idle"))
        #expect(line.hasPrefix("A one-time download of "))
        #expect(line.contains("It needs "))
        #expect(line.contains("of free space"))
        #expect(line.contains("free on this drive"))
        #expect(!AILogic.downloadLine(download("Idle", free: nil)).contains("free on this drive"))
        #expect(AILogic.downloadLine(download("Downloading", done: 1e9, total: 5e9)).hasPrefix("Downloading: "))
        #expect(AILogic.downloadLine(download("Paused", done: 1e9, total: 5e9)).hasPrefix("Paused at "))
        #expect(AILogic.downloadLine(download("Paused", done: 1e9, total: 5e9)).hasSuffix("It continues from there."))
        #expect(AILogic.downloadLine(download("Checking")) == "Getting ready to download\u{2026}")
    }

    @Test func progressAndSpaceRules() {
        #expect(AILogic.downloadFraction(download("Downloading", done: 1e9, total: 4e9)) == 0.25)
        #expect(AILogic.downloadFraction(download("Checking")) == nil)
        #expect(AILogic.downloadFraction(download("Downloading", done: 1e9, total: 0)) == nil)
        #expect(AILogic.downloadFraction(download("Downloading", done: 9e9, total: 4e9)) == 1)
        #expect(AILogic.tooLittleSpace(download("Idle", free: 1e9)))
        #expect(!AILogic.tooLittleSpace(download("Idle", free: 50e9)))
        #expect(!AILogic.tooLittleSpace(download("Idle", free: nil)))
        // A paused download already holds part of the files, so it needs less.
        #expect(!AILogic.tooLittleSpace(download("Paused", done: 5.5e9, free: 1e9)))
        // A damaged set frees its own space first.
        #expect(!AILogic.tooLittleSpace(download("Idle", free: 1e9), damaged: true))
        #expect(AILogic.phaseAnnouncement("Failed") == "The download stopped.")
        #expect(AILogic.phaseAnnouncement("Done") == "OpenFrame AI is ready.")
        #expect(AILogic.importMessage(AiImportResult(complete: true)) == "OpenFrame AI is ready.")
        #expect(AILogic.importMessage(AiImportResult(complete: false)).contains("Import the other model file"))
    }

    // MARK: Health card

    @Test func healthRowsMatchWindows() {
        let h = AiHealthDto(
            requests: 12, medianTotalMs: 4200, proposals: 5, applied: 3, rejected: 1, undone: 1, applyRatePct: 60, averagePasses: 1.25,
            errors: 2, weakestTool: WeakTool(name: "tasks.create", uses: 4, failurePct: 25), runsOn: "the processor")
        let rows = AIHealthText.rows(h)
        #expect(rows.map(\.label) == ["Requests counted", "Typical answer time", "Proposals applied", "Checks per answer", "Errors", "Runs on", "Weakest tool"])
        #expect(rows[0].value == "12")
        #expect(rows[1].value == "4.2 s")
        #expect(rows[2].value == "60% (3 of 5 decided)")
        #expect(rows[5].value == "the processor")
        #expect(rows[6].value == "tasks.create: 25% of 4 uses went wrong")
        let empty = AiHealthDto(requests: 0, proposals: 0, applied: 0, rejected: 0, undone: 0, errors: 0)
        let e = AIHealthText.rows(empty)
        #expect(e[1].value == "none yet" && e[5].value == "not known yet" && e[6].value == "none stands out")
        #expect(e[2].value == "0 of 0")
    }
}

@MainActor
struct AIStarterTests {
    /// The questions offered to start with are ones the core answers from project data.
    @Test func starterQuestionsArePlainAndPerPage() {
        for ws in Workspace.allCases {
            let qs = AIText.starterQuestions(for: ws)
            #expect(!qs.isEmpty && qs.count <= 4, "\(ws)")
            #expect(Set(qs).count == qs.count, "\(ws) repeats a question")
        }
        #expect(AIText.starterQuestions(for: .screenplay).first == "How many scenes does the screenplay have?")
    }

    @Test func copyTextHoldsTheAnswerAndItsList() {
        let r = AIFixtures.result(content: "Two characters speak.", items: [AiResultItem(label: "ARJUN"), AiResultItem(label: "MEERA", detail: "3 scenes")])
        #expect(AILogic.copyText(r) == "Two characters speak.\n- ARJUN\n- MEERA (3 scenes)")
    }
}

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

    @Test func answersListOnlyRealSourcesUnderSeeSources() {
        let chips = AILogic.answerSources([
            AiProvenance(kind: "Scope", label: "Using: Whole Project"),
            AiProvenance(kind: "Basis", label: "Exact"),
            AiProvenance(kind: "Scene", label: "BUS STOP", nav: AiNavTarget(workspace: "screenplay", params: ["sceneId": "s1"])),
            AiProvenance(kind: "Scene", label: "BUS STOP", nav: AiNavTarget(workspace: "screenplay", params: ["sceneId": "s1"])),
            AiProvenance(kind: "Draft", label: "Draft 1", nav: AiNavTarget(workspace: "elsewhere", params: [:])),
        ])
        // The scope is a setting of the question, not a source; a repeat is listed once.
        #expect(chips.map(\.text) == ["Scene: BUS STOP", "Draft: Draft 1"])
        #expect(chips[0].route?.workspace == .screenplay)
        #expect(chips[0].route?.params["sceneId"] == "s1")
        // An unknown workspace is ignored, never guessed.
        #expect(chips[1].route == nil)
        #expect(AILogic.answerSources([AiProvenance(kind: "Scope", label: "Using: Current Screenplay")]).isEmpty)
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

    @Test func navigationTargetsMapToMacWorkspaces() {
        let expected: [(String, Workspace)] = [
            ("home", .home), ("vault", .ideaVault), ("story", .story), ("screenplay", .screenplay), ("breakdown", .breakdown),
            ("production", .production), ("callsheets", .callSheets), ("files", .files), ("trash", .recentlyDeleted),
            ("activity", .activity), ("notes", .notes),
        ]
        for (id, ws) in expected { #expect(AIRoute.workspace(for: id) == ws, "\(id)") }
        #expect(AIRoute.workspace(for: "settings") == nil)
        #expect(AIRoute(nil) == nil)
        // A Production link opens its tab, Overview when it names none.
        #expect(AIRoute(AiNavTarget(workspace: "production", params: ["sub": "cast-crew"]))?.tab == .castCrew)
        #expect(AIRoute(AiNavTarget(workspace: "production", params: [:]))?.tab == .overview)
        #expect(AIRoute(AiNavTarget(workspace: "screenplay", params: [:]))?.tab == nil)
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

    // MARK: Report AI output

    @Test func reportedOutputHoldsWhatThePersonSawAndNothingElse() throws {
        let result = AiResultDto(
            id: "r", kind: .answer, status: .informational, content: "There are 3 scenes.", details: ["from the draft"], confidence: .exact,
            provenance: [AiProvenance(kind: "Draft", label: "Draft 1")], items: [], steps: [], createdAt: 1_700_000_000_000)
        let x = AiExchange(requestId: "q", conversationId: "c", requestText: "How many scenes?", scopeLabel: "Using: Whole Project", result: result, createdAt: 1)
        let json = try JSONSerialization.jsonObject(with: AIOutputReport.make(x)) as? [String: Any]
        #expect(json?["question"] as? String == "How many scenes?")
        #expect(json?["answer"] as? String == "There are 3 scenes.")
        #expect(json?["resultKind"] as? String == "Answer")
        #expect(json?["sources"] as? [String] == ["Draft: Draft 1"])
        #expect((json?["note"] as? String)?.contains("Nothing was sent") == true)
    }
}

// Pure view logic for the AI Assistant (Windows: apps/desktop/src/ai/model.ts). No project truth
// lives here: everything shown comes from `ai.*` operations. Same words as Windows.

import Foundation
import OpenFrameCore

enum AIText {
    static let unavailable = "AI is currently unavailable. OpenFrame's core workflows continue to work normally."
    static let preparing = "Preparing project context\u{2026}"
    static let degraded = "AI can still answer some questions while project context is being prepared."
    static let staleHint = "AI never applies a stale proposal blindly. It must be re-checked against the current project."
    static let promise = "OpenFrame AI runs on this computer after a one-time download. Your project data is not sent to OpenFrame or to an AI cloud service."
    static let gemmaNotice = "Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms."
    /// Questions to start with, for the page the person is on: plain questions about the
    /// project's own data, so a first try has something to read.
    static func starterQuestions(for workspace: Workspace) -> [String] {
        switch workspace {
        case .screenplay:
            ["How many scenes does the screenplay have?", "List the scene headings.", "How long is the screenplay?", "Are there unresolved comments?"]
        case .story:
            ["How many scene cards are on the beat board?", "Show the characters on the beat board.", "How many acts are there?", "How many scenes does the screenplay have?"]
        case .breakdown:
            ["How far along is the breakdown?", "Which draft is the production source?", "Which locations are used in the script?", "How many scenes does the screenplay have?"]
        case .production, .callSheets:
            ["How many scenes are unscheduled?", "How far along is the breakdown?", "Which locations are used in the script?", "Which draft is the production source?"]
        case .notes:
            ["What open tasks do I have?", "Summarise my project notes", "Are there unresolved comments?"]
        default:
            ["How many scenes does the screenplay have?", "How many scene cards are on the beat board?", "What open tasks do I have?", "How far along is the breakdown?"]
        }
    }

    static let emptyConversation = "Ask about your project, for example \u{201C}How many scenes are in the script?\u{201D}, \u{201C}Open Scene 12.\u{201D} or \u{201C}Create a task to scout the railway station.\u{201D} Answers come from your project and say what they are based on; changes are only ever prepared for you to review."
    static let working = "Working on it on this computer\u{2026}"
    static let stopping = "Stopping\u{2026}"
    static let saveConversation = "Save conversation for review"
}

extension AiScopeKind {
    var label: String {
        switch self {
        case .currentSelection: "Current Selection"
        case .currentScene: "Current Scene"
        case .currentScreenplay: "Current Screenplay"
        case .specificDraft: "Specific Draft"
        case .storyBoard: "Beat Board"
        case .ideaVaultSelection: "Idea Vault Selection"
        case .production: "Production Workspace"
        case .shootingDay: "Shooting Day"
        case .callSheet: "Call Sheet"
        case .wholeProject: "Whole Project"
        }
    }
}

/// Where the person is, for the scope list (Windows: the route). Workspaces fill the optional
/// ids when they know them (an open scene, selected cards); the assistant never guesses.
struct AIScopeContext: Equatable {
    var workspace: Workspace = .home
    var draftId: String?
    var sceneId: String?
    var selectionIds: [String] = []
    var shootingDayId: String?
    var callSheetId: String?
}

/// Shared place for workspaces to say what is open. The assistant reads it when it is asked.
@MainActor
@Observable
final class AIScopeHints {
    static let shared = AIScopeHints()
    var draftId: String?
    var sceneId: String?
    var selectionIds: [String] = []
    var shootingDayId: String?
    var callSheetId: String?

    func context(for workspace: Workspace) -> AIScopeContext {
        AIScopeContext(
            workspace: workspace,
            draftId: workspace == .screenplay ? draftId : nil,
            sceneId: workspace == .screenplay ? sceneId : nil,
            selectionIds: selectionIds,
            shootingDayId: workspace == .production || workspace == .callSheets ? shootingDayId : nil,
            callSheetId: workspace == .callSheets ? callSheetId : nil)
    }
}

enum AILogic {
    /// Scopes that make sense right now: a scope that needs a target is offered only when one exists.
    static func scopeOptions(_ c: AIScopeContext) -> [AiScopeKind] {
        var out: [AiScopeKind] = []
        if !c.selectionIds.isEmpty { out.append(c.workspace == .ideaVault ? .ideaVaultSelection : .currentSelection) }
        if c.sceneId != nil { out.append(.currentScene) }
        out.append(.currentScreenplay)
        if c.draftId != nil { out.append(.specificDraft) }
        out.append(.storyBoard)
        out.append(.production)
        if c.shootingDayId != nil { out.append(.shootingDay) }
        if c.callSheetId != nil { out.append(.callSheet) }
        out.append(.wholeProject)
        return out
    }

    /// Scope inherited from where the person is (AI spec section 7.2); Whole Project otherwise.
    static func defaultScope(_ c: AIScopeContext) -> AiScopeKind {
        let options = scopeOptions(c)
        if c.workspace == .screenplay, c.sceneId != nil { return .currentScene }
        if options.contains(.ideaVaultSelection) { return .ideaVaultSelection }
        if options.contains(.currentSelection), c.workspace != .screenplay { return .currentSelection }
        if c.workspace == .callSheets, c.callSheetId != nil { return .callSheet }
        switch c.workspace {
        case .screenplay: return .currentScreenplay
        case .story: return .storyBoard
        case .breakdown, .production, .callSheets: return .production
        default: return .wholeProject
        }
    }

    /// Arguments for `ai.ask`: references only; the core resolves and budgets the actual context.
    /// `fallbackDraftId` is the draft the Screenplay workspace has open when the hints name none.
    static func scopeArgs(_ kind: AiScopeKind, _ c: AIScopeContext, fallbackDraftId: String? = nil) -> AiScopeArgs {
        let selection: [AiSelectionRef] =
            kind == .currentSelection || kind == .ideaVaultSelection ? c.selectionIds.map { AiSelectionRef(id: $0) } : []
        let wantsDraft = kind == .specificDraft || kind == .currentScreenplay
        return AiScopeArgs(
            kind: kind,
            draftId: wantsDraft ? (c.draftId ?? (c.workspace == .screenplay ? fallbackDraftId : nil)) : nil,
            sceneId: kind == .currentScene ? c.sceneId : nil,
            selection: selection,
            shootingDayId: kind == .shootingDay ? c.shootingDayId : nil,
            callSheetId: kind == .callSheet ? c.callSheetId : nil)
    }

    // MARK: Panel mode

    enum PanelMode: Equatable { case unavailable, off, download, ready }

    /// Which top-level view the panel shows (ADR-0026). Ready once the models are downloaded; before
    /// that, and when a downloaded file is missing or damaged, the download; a missing or damaged
    /// engine needs a reinstall ("off", with the reason).
    static func panelMode(_ s: AiStatusDto?) -> PanelMode {
        guard let s else { return .unavailable }
        if s.available { return .ready }
        // Installed and not known to be broken, only not checked again since the engine was
        // stopped: the next question checks the files and starts the engine, so the Ask box stays.
        if s.problem == nil, ["NotChecked", "Checking", "Verified"].contains(s.bundleState) { return .ready }
        return s.bundleState == "NotInstalled" || s.problem?.code == "ai.models_damaged" ? .download : .off
    }

    static func damagedModels(_ s: AiStatusDto) -> String? {
        s.problem?.code == "ai.models_damaged" ? s.problem?.message : nil
    }

    static func offReason(_ s: AiStatusDto) -> String { s.problem?.message ?? AIText.unavailable }

    static func runtimeLabel(_ state: String) -> String? {
        switch state {
        case "Starting", "LoadingModel": "Getting Offline AI ready on this computer\u{2026}"
        case "Busy": "Working on a request\u{2026}"
        case "Stopping": "Stopping\u{2026}"
        default: nil
        }
    }

    /// Plain-language status line for the settings card.
    static func stateText(_ s: AiStatusDto) -> String {
        if s.bundleState == "NotInstalled" { return "Not downloaded" }
        if s.problem?.code == "ai.models_damaged" { return "Needs a new download" }
        if !s.available { return s.bundleState == "Unsupported" ? "Not available on this computer" : "Needs a reinstall" }
        if s.bundleState == "Checking" { return "Checking its files\u{2026}" }
        if s.runtimeState == "Failed" { return "Stopped after a problem. It restarts when you ask something" }
        return runtimeLabel(s.runtimeState) ?? "AI Ready"
    }

    static func fileCheckText(_ state: String) -> String {
        switch state {
        case "Verified": "checked"
        case "NotChecked": "checked before the first answer"
        case "Checking": "checking\u{2026}"
        case "NotInstalled": "models not downloaded"
        case "Missing": "engine missing, reinstall OpenFrame Studio"
        case "Damaged": "changed or damaged"
        default: "not available on this computer"
        }
    }

    // MARK: Answers

    /// The answer as plain text for the clipboard: the text, then every item of its list.
    static func copyText(_ r: AiResultDto) -> String {
        let items = r.items.map { "- " + $0.label + ($0.detail.map { " (\($0))" } ?? "") }
        return ([r.content] + items).joined(separator: "\n")
    }

    /// Items shown before "Show all": a long answer is a preview plus the full list, never "and N more".
    static let previewItems = 8

    static func visibleItems(_ items: [AiResultItem], all: Bool) -> [AiResultItem] {
        all ? items : Array(items.prefix(previewItems))
    }

    /// A refusal, a failure, a private item or a question back needs its reasons next to it. For
    /// an ordinary answer the details are how it was worked out and wait behind "See sources".
    static func reasonsVisible(_ kind: AiResultKind) -> Bool {
        [.denied, .failed, .unavailable, .clarify, .private].contains(kind)
    }

    /// Where the parts of a result go (Windows: ResultView). Rows the person asked for stay under
    /// the answer (the core says so with `itemsAreAnswer`); rows that only back the text, the
    /// sources, how sure the answer is and its steps wait behind a closed "See sources".
    struct ResultLayout: Equatable {
        var reasonsVisible: Bool
        var itemsInline: Bool
        var itemsAsSources: Bool
        var hasSources: Bool
    }

    static func layout(_ r: AiResultDto) -> ResultLayout {
        let reasons = reasonsVisible(r.kind)
        let hasItems = !r.items.isEmpty
        let asSources = hasItems && !r.itemsAreAnswer
        let hasSources = (!reasons && !r.details.isEmpty) || confidenceLabel(r.confidence) != nil
            || !r.provenance.isEmpty || stepsSummary(r.steps) != nil || asSources
        return ResultLayout(reasonsVisible: reasons, itemsInline: hasItems && r.itemsAreAnswer, itemsAsSources: asSources, hasSources: hasSources)
    }

    static func confidenceLabel(_ c: AiConfidence?) -> String? {
        switch c {
        case .exact: "Exact \u{00B7} from project data"
        case .inferred: "Inferred \u{00B7} written by Offline AI"
        case .unavailable: "Not available in project data"
        case nil: nil
        }
    }

    /// Short line naming the tool steps behind an answer (nil for a single step).
    static func stepsSummary(_ steps: [AiStepDto]) -> String? {
        steps.count < 2 ? nil : "\(steps.count) steps: " + steps.map(\.label).joined(separator: " \u{00B7} ")
    }

    struct SourceChip: Identifiable, Equatable {
        var id: String
        var text: String
        var route: AIRoute?
    }

    /// Provenance as chips under "Based on:"; a chip opens its source when it has a place.
    static func sourceChips(_ provenance: [AiProvenance]) -> [SourceChip] {
        provenance.enumerated().map { i, p in
            SourceChip(
                id: "\(i):\(p.kind):\(p.label)",
                text: p.kind == "Scope" || p.kind == "Basis" ? p.label : "\(p.kind): \(p.label)",
                route: AIRoute(p.nav))
        }
    }

    // MARK: Conversation

    /// A new conversation id made here, so a running request can be stopped before its answer
    /// exists. The core accepts letters, digits and hyphens (up to 64).
    static func newConversationId() -> String { UUID().uuidString.lowercased() }

    /// The latest progress line of the request running in `conversation`, from one app event.
    /// Lines of another conversation and other events are ignored.
    static func progress(_ event: AppEvent, conversation: String?) -> AIProgress? {
        guard let conversation, case let .module(module, name, payload) = event, module == "ai", name == "progress",
              payload["conversationId"]?.stringValue == conversation,
              let label = payload["label"]?.stringValue, !label.isEmpty
        else { return nil }
        return AIProgress(label: label, step: payload["step"]?.intValue ?? 0, total: payload["total"]?.intValue ?? 0)
    }

    /// The toast after "Save conversation for review".
    static func savedMessage(_ r: AiReportResult) -> String {
        "Saved \(r.fileName) (\(Int(r.rows)) \(Int(r.rows) == 1 ? "question" : "questions"))."
    }

    // MARK: Change Sets

    struct ChangeSetActions: Equatable { var apply: Bool; var reject: Bool; var recheck: Bool }

    static func changeSetActions(_ cs: AiChangeSetDto) -> ChangeSetActions {
        switch cs.state {
        case "Pending": ChangeSetActions(apply: true, reject: true, recheck: false)
        case "Stale", "Conflict": ChangeSetActions(apply: false, reject: true, recheck: true)
        default: ChangeSetActions(apply: false, reject: false, recheck: false)
        }
    }

    static func isStale(_ cs: AiChangeSetDto) -> Bool { cs.state == "Stale" || cs.state == "Conflict" }

    static func changeCount(_ n: Double) -> String { Int(n) == 1 ? "1 change" : "\(Int(n)) changes" }

    static func changeSetStateText(_ cs: AiChangeSetDto) -> String? {
        switch cs.state {
        case "Applied": "Applied \(Int(cs.appliedOperations) == 1 ? "1 change" : "\(Int(cs.appliedOperations)) changes"). Use Undo to reverse it."
        case "Rejected": "Rejected. Nothing was changed."
        case "Accepted": "Applying\u{2026}"
        case "Failed": cs.errorMessage ?? "This proposal couldn't be applied. Nothing was changed."
        default: nil
        }
    }

    // MARK: Index notice

    /// Shown only while the project context is rebuilt: the assistant still works, with fewer sources.
    static func indexNotice(_ s: AiIndexStatus?) -> (preparing: String, degraded: String)? {
        guard let s, s.state == .rebuilding || s.state == .stale else { return nil }
        return (AIText.preparing, AIText.degraded)
    }

    // MARK: Download

    static let running: Set<String> = ["Checking", "Downloading", "Installing"]

    static func downloadLine(_ d: AiDownloadDto) -> String {
        let progress = "\(formattedBytes(d.doneBytes)) of \(formattedBytes(d.totalBytes > 0 ? d.totalBytes : d.sizeBytes))"
        switch d.phase {
        case "Checking": return "Getting ready to download\u{2026}"
        case "Downloading": return "Downloading: \(progress)"
        case "Installing": return "Checking the files and getting OpenFrame AI ready\u{2026}"
        case "Paused": return "Paused at \(progress). It continues from there."
        default:
            var line = "A one-time download of \(formattedBytes(d.sizeBytes)). It needs \(formattedBytes(d.neededBytes)) of free space"
            if let free = d.freeBytes { line += " (\(formattedBytes(free)) free on this drive)" }
            return line + "."
        }
    }

    /// Not enough room for the rest of the download. A damaged set frees its own space when it is
    /// removed first, so that case is left to the core.
    static func tooLittleSpace(_ d: AiDownloadDto, damaged: Bool = false) -> Bool {
        let done = d.phase == "Paused" || d.phase == "Failed" ? d.doneBytes : 0
        guard !damaged, let free = d.freeBytes else { return false }
        return free < d.neededBytes - done
    }

    static func phaseAnnouncement(_ phase: String) -> String {
        switch phase {
        case "Checking": "Getting ready to download."
        case "Downloading": "Downloading OpenFrame AI."
        case "Installing": "Checking the files."
        case "Paused": "Download paused."
        case "Failed": "The download stopped."
        case "Done": "OpenFrame AI is ready."
        default: ""
        }
    }

    static func downloadFraction(_ d: AiDownloadDto) -> Double? {
        guard d.totalBytes > 0, d.phase == "Downloading" || d.phase == "Paused" else { return nil }
        return min(1, max(0, d.doneBytes / d.totalBytes))
    }

    static func importMessage(_ r: AiImportResult) -> String {
        r.complete ? "OpenFrame AI is ready." : "The file was added. Import the other model file to finish."
    }
}

// MARK: - Navigation

/// What a request is doing now, from the core's `ai.progress` event.
struct AIProgress: Equatable {
    var label: String
    var step: Int
    var total: Int
}

/// A place the assistant points at (Windows: model.ts toRoute). The core sends the Windows route:
/// a workspace id, an optional `sub` (a Production, Story or Settings tab) and parameters.
struct AIRoute: Equatable {
    enum Target: Equatable {
        /// A workspace, and for Production its tab.
        case place(ProjectPlace)
        /// The Story workspace on one of its tabs.
        case story(StoryTab)
        /// Project Settings on a page ("settings" with "templates", "offline-ai").
        case projectSettings(ProjectSettingsTab)
    }

    var target: Target
    var params: [String: String]

    init?(_ nav: AiNavTarget?) {
        guard let nav, let target = Self.target(workspace: nav.workspace, sub: nav.sub) else { return nil }
        self.target = target
        params = nav.params
    }

    /// The Mac workspace, when the target is one.
    var workspace: Workspace? {
        switch target {
        case .place(let p): p.workspace
        case .story: .story
        case .projectSettings: nil
        }
    }

    /// The Production tab named by the link, Overview when it names none.
    var tab: ProductionTab? {
        if case .place(let p) = target { return p.tab }
        return nil
    }

    /// The Windows route to the Mac place. Unknown workspaces are ignored, never guessed.
    static func target(workspace id: String, sub: String?) -> Target? {
        if id == "settings" {
            switch sub {
            case "offline-ai": return .projectSettings(.offlineAI)
            case let sub?: return .projectSettings(ProjectSettingsTab(rawValue: sub) ?? .general)
            case nil: return .projectSettings(.general)
            }
        }
        guard let ws = workspace(for: id) else { return nil }
        switch ws {
        case .production: return .place(ProjectPlace(.production, tab: sub.flatMap(ProductionTab.init(windowsId:)) ?? .overview))
        case .story: return sub.flatMap(StoryTab.init(rawValue:)).map(Target.story) ?? .place(ProjectPlace(.story))
        default: return .place(ProjectPlace(ws))
        }
    }

    /// Windows workspace ids to the Mac workspaces. Unknown ids are ignored.
    static func workspace(for id: String) -> Workspace? {
        switch id {
        case "home": .home
        case "vault": .ideaVault
        case "story": .story
        case "screenplay": .screenplay
        case "breakdown": .breakdown
        case "production": .production
        case "callsheets": .callSheets
        case "files": .files
        case "trash": .recentlyDeleted
        case "activity": .activity
        case "notes": .notes
        default: nil
        }
    }

    /// The request the Story workspace takes to show a tab (`onProjectIntents`).
    static func storyIntent(_ tab: StoryTab) -> String { "story.show_\(tab.rawValue)" }

    /// Open the place. Navigation is application control, not a project change (AI spec section 5.3).
    @MainActor func perform() {
        switch target {
        case .place(let place):
            ProjectNav.show(place)
            if place.workspace == .screenplay, params["sceneId"] != nil || params["draftId"] != nil {
                var info: [String: Any] = [:]
                for key in ["screenplayId", "draftId", "sceneId"] { if let v = params[key] { info[key] = v } }
                NotificationCenter.default.post(name: .openFrameShowScreenplayTarget, object: nil, userInfo: info)
            }
        case .story(let tab):
            ProjectNav.show(.story)
            ProjectIntents.shared.fire(Self.storyIntent(tab))
        case .projectSettings(let tab):
            ProjectSettingsRouter.open(tab: tab)
        }
    }
}

extension Notification.Name {
    /// Opens or closes the assistant panel (Command+J).
    static let openFrameToggleAssistant = Notification.Name("studio.openframe.toggleAssistant")
}

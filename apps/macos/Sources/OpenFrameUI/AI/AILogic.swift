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
    /// Questions to start with, for the page the person is on. Each is a phrasing the assistant
    /// answers from the project's own data (the direct routes in the core's commands.rs), so a
    /// first try never ends in "I couldn't find that".
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

    static let emptyConversation = "Ask about your project, for example \u{201C}How many scenes are in the script?\u{201D}, \u{201C}Open Scene 12.\u{201D} or \u{201C}Create a task to scout the railway station.\u{201D} The assistant can look things up in several steps; changes are only ever prepared for you to review."
    static let noProject = "The assistant answers from the project that is open, on this computer."
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
    /// an ordinary answer the details are how it was worked out and stay out of the conversation.
    static func reasonsVisible(_ kind: AiResultKind) -> Bool {
        [.denied, .failed, .unavailable, .clarify, .private].contains(kind)
    }

    struct SourceChip: Identifiable, Equatable {
        var id: String
        var text: String
        var route: AIRoute?
    }

    /// The sources listed under "See sources": the places the answer was read from. The scope and
    /// the basis ("Using: Whole Project") are settings of the question, not sources, so they are
    /// left out; repeats are listed once.
    static func answerSources(_ provenance: [AiProvenance]) -> [SourceChip] {
        var seen = Set<String>()
        return provenance.enumerated().compactMap { i, p in
            guard p.kind != "Scope", p.kind != "Basis" else { return nil }
            let text = "\(p.kind): \(p.label)"
            guard seen.insert(text).inserted else { return nil }
            return SourceChip(id: "\(i):\(text)", text: text, route: AIRoute(p.nav))
        }
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

/// A place the assistant points at: a workspace, and for the screenplay a scene or draft.
struct AIRoute: Equatable {
    var workspace: Workspace
    var params: [String: String]

    init?(_ nav: AiNavTarget?) {
        guard let nav, let ws = Self.workspace(for: nav.workspace) else { return nil }
        workspace = ws
        params = nav.params
    }

    /// The Production tab named by the link ("sub"), Overview when it names none.
    var tab: ProductionTab? {
        guard workspace == .production else { return nil }
        return params["sub"].flatMap(ProductionTab.init(windowsId:)) ?? .overview
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

    /// Open the place. Navigation is application control, not a project change (AI spec section 5.3).
    @MainActor func perform() {
        ProjectNav.show(workspace, tab: tab)
        if workspace == .screenplay, params["sceneId"] != nil || params["draftId"] != nil {
            var info: [String: Any] = [:]
            for key in ["screenplayId", "draftId", "sceneId"] { if let v = params[key] { info[key] = v } }
            NotificationCenter.default.post(name: .openFrameShowScreenplayTarget, object: nil, userInfo: info)
        }
    }
}

extension Notification.Name {
    /// Opens or closes the assistant panel (Command+J).
    static let openFrameToggleAssistant = Notification.Name("studio.openframe.toggleAssistant")
}

// MARK: - Report AI output

/// The file "Report AI output" writes: the question, what kind of answer it was, the answer the
/// person saw and the names of its sources and steps. It stays on this computer.
enum AIOutputReport {
    static func make(_ x: AiExchange) throws -> Data {
        let r = x.result
        let body: [String: Any] = [
            "note": "Saved by the person from OpenFrame Studio. Nothing was sent anywhere.",
            "question": x.requestText,
            "scope": x.scopeLabel,
            "resultKind": r.kind.rawValue,
            "status": r.status.rawValue,
            "answer": r.content,
            "details": r.details,
            "confidence": r.confidence?.rawValue ?? "",
            "sources": r.provenance.map { "\($0.kind): \($0.label)" },
            "steps": r.steps.map(\.label),
            "proposal": r.changeSet.map { ["title": $0.title, "state": $0.state, "changes": Int($0.operationCount)] as [String: Any] } ?? [:],
            "createdAt": ISO8601DateFormatter().string(from: Date(timeIntervalSince1970: r.createdAt / (r.createdAt > 10_000_000_000 ? 1000 : 1))),
        ]
        return try JSONSerialization.data(withJSONObject: body, options: [.prettyPrinted, .sortedKeys])
    }
}

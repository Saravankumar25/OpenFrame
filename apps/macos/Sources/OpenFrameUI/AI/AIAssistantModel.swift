// The assistant panel's model. It holds what the panel shows (status, the conversation, the typed
// question) and sends every request to an `ai.*` operation. The core decides what is allowed and
// what the answer is; nothing here judges an answer. The typed question is never lost: when a
// request fails or is cancelled it goes back into the box.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class AIAssistantModel {
    let app: AppModel

    /// What the core reports about OpenFrame AI. `nil` until the first answer.
    var status: AiStatusDto?
    var statusFailed = false
    var index: AiIndexStatus?
    var exchanges: [AiExchange] = []
    var context = AIScopeContext()
    var chosenScope: AiScopeKind?
    var text = ""
    /// The question being answered now.
    var pending: String?
    /// A plain message about the last request or action; cleared by the next one.
    var error: String?
    /// The change set an action is running on.
    var busyChangeSet: String?
    var loadedHistory = false
    /// Asks the person where to save a report. Tests set a fixed answer.
    var chooseSaveLocation: @MainActor (_ title: String, _ name: String) -> URL? = { AIFilePicker.chooseSaveLocation(title: $0, name: $1) }

    /// `nil`: the latest conversation. `.some(nil)`: a new one. `.some(id)`: this one.
    private var conversation: String??
    private var askTask: Task<Void, Never>?
    private var askGeneration = 0

    static let maxLength = 2000

    init(app: AppModel) {
        self.app = app
    }

    var core: Core? { app.core }

    // MARK: Derived

    var mode: AILogic.PanelMode { AILogic.panelMode(status) }
    var scopeOptions: [AiScopeKind] { AILogic.scopeOptions(context) }
    var effectiveScope: AiScopeKind {
        if let chosenScope, scopeOptions.contains(chosenScope) { return chosenScope }
        return AILogic.defaultScope(context)
    }
    var canSend: Bool { !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && pending == nil }

    /// "Using: Whole Project" (the line scenario 73 checks), or the label the last answer carried.
    var usingLabel: String {
        if let last = exchanges.last, last.scopeLabel == "Using: \(effectiveScope.label)" { return last.scopeLabel }
        return "Using: \(effectiveScope.label)"
    }

    /// The newest applied change set of this conversation: the only card that offers Undo, because
    /// Undo reverses the latest project change.
    var latestAppliedChangeSetId: String? {
        exchanges.last(where: { $0.result.changeSet?.state == "Applied" })?.result.changeSet?.id
    }

    var indexNotice: (preparing: String, degraded: String)? { AILogic.indexNotice(index) }

    // MARK: Where the person is

    /// The workspace changed (or the hints did). A scope the new place does not offer is dropped.
    func setContext(_ new: AIScopeContext) {
        let workspaceChanged = new.workspace != context.workspace
        context = new
        if workspaceChanged { chosenScope = nil }
    }

    // MARK: Reading

    func refreshStatus() async {
        guard let core else { return }
        do {
            status = try await core.aiStatus()
            statusFailed = false
        } catch {
            statusFailed = true
        }
    }

    func refreshIndex() async {
        guard let core, mode == .ready else { return }
        // The project context is a nicety: when it cannot be read the notice is simply absent.
        index = try? await core.aiIndexStatus()
    }

    func loadHistory() async {
        guard let core, conversation != .some(nil) else {
            exchanges = []
            loadedHistory = true
            return
        }
        do {
            let h = try await core.aiHistory(AiHistoryArgs(conversationId: conversation ?? nil, limit: 50))
            exchanges = h.exchanges
            if conversation == nil, let id = h.conversationId { conversation = .some(id) }
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        loadedHistory = true
    }

    // MARK: Asking

    /// Start a request. The box is emptied at once; it is filled again if the request fails or is cancelled.
    func send() {
        let question = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !question.isEmpty, pending == nil else { return }
        pending = question
        text = ""
        error = nil
        askGeneration += 1
        let generation = askGeneration
        askTask = Task { [weak self] in
            await self?.ask(question, generation: generation)
        }
    }

    /// Awaitable form for tests: the same path as `send`.
    func sendAndWait() async {
        send()
        await askTask?.value
    }

    private func ask(_ question: String, generation: Int) async {
        guard let core else {
            finishFailed(question, generation: generation, message: AIText.unavailable)
            return
        }
        let scope = await resolvedScopeArgs()
        do {
            let exchange = try await core.aiAsk(
                AiAskArgs(text: question, scope: scope, conversationId: (conversation ?? nil)))
            guard generation == askGeneration else { return }
            conversation = .some(exchange.conversationId)
            pending = nil
            await loadHistory()
            // Navigation is application control, not a project change: it runs directly (AI spec 5.3).
            if exchange.result.kind == .navigate, let route = AIRoute(exchange.result.nav) { route.perform() }
            await refreshStatus()
        } catch {
            finishFailed(question, generation: generation, message: CoreFailure(asCore: error).message)
        }
    }

    private func finishFailed(_ question: String, generation: Int, message: String) {
        guard generation == askGeneration else { return }
        // The question goes back into the box, unless the person already started typing another.
        if text.isEmpty { text = question }
        error = message
        pending = nil
        Task { await refreshStatus() }
    }

    /// Stop waiting for the answer. The engine is stopped so it does not keep working, the question
    /// goes back into the box, and the late answer (if any) is read from the history, not shown as new.
    func cancel() async {
        guard let question = pending else { return }
        askGeneration += 1
        askTask?.cancel()
        pending = nil
        if text.isEmpty { text = question }
        error = "Stopped. Your question is still in the box."
        _ = try? await core?.aiStopRuntime()
        await loadHistory()
        await refreshStatus()
    }

    func newConversation() {
        guard pending == nil else { return }
        conversation = .some(nil)
        exchanges = []
        error = nil
    }

    /// The draft the Screenplay workspace shows, when the hints name none (nothing is guessed:
    /// the core's own current draft of the oldest screenplay).
    func resolvedScopeArgs() async -> AiScopeArgs {
        let kind = effectiveScope
        var fallback: String?
        if (kind == .currentScreenplay || kind == .specificDraft), context.draftId == nil, context.workspace == .screenplay {
            if let o = try? await core?.screenplayOverview(ScreenplayOverviewArgs()) {
                fallback = o.screenplay?.currentDraftId ?? o.drafts.first?.id
            }
        }
        return AILogic.scopeArgs(kind, context, fallbackDraftId: fallback)
    }

    // MARK: Change Sets

    /// Apply a reviewed proposal. Only the person's click gets here; nothing applies by itself.
    @discardableResult
    func apply(_ cs: AiChangeSetDto, confirmDestructive: Bool = false) async -> AiChangeSetDto? {
        guard let core, busyChangeSet == nil else { return nil }
        busyChangeSet = cs.id
        defer { busyChangeSet = nil }
        error = nil
        do {
            let r = try await core.aiChangeSetAccept(AiAcceptArgs(id: cs.id, confirmDestructive: confirmDestructive ? true : nil))
            await loadHistory()
            if r.state == "Applied" { ToastCenter.shared.undoable("Applied: \(r.title)", app: app) }
            return r
        } catch {
            self.error = CoreFailure(asCore: error).message
            await loadHistory()
            return nil
        }
    }

    @discardableResult
    func reject(_ cs: AiChangeSetDto) async -> AiChangeSetDto? {
        await run(cs) { try await $0.aiChangeSetReject(AiChangeSetArgs(id: cs.id)) }
    }

    @discardableResult
    func recheck(_ cs: AiChangeSetDto) async -> AiChangeSetDto? {
        await run(cs) { try await $0.aiChangeSetRecheck(AiChangeSetArgs(id: cs.id)) }
    }

    private func run(_ cs: AiChangeSetDto, _ op: (Core) async throws -> AiChangeSetDto) async -> AiChangeSetDto? {
        guard let core, busyChangeSet == nil else { return nil }
        busyChangeSet = cs.id
        defer { busyChangeSet = nil }
        error = nil
        do {
            let r = try await op(core)
            await loadHistory()
            return r
        } catch {
            self.error = CoreFailure(asCore: error).message
            await loadHistory()
            return nil
        }
    }

    /// Undo an applied change set: one project Undo reverses all of it (scenario 74).
    func undo() async {
        await app.undoFromToast(store: .project)
        await loadHistory()
    }

    // MARK: Report AI output

    /// Save one question and answer to a file the person chooses, to share if they want to. Local
    /// only: nothing is sent. Returns the file written.
    @discardableResult
    func reportOutput(_ exchange: AiExchange) async -> URL? {
        guard let url = chooseSaveLocation("Report AI output", "openframe-ai-output.json") else { return nil }
        do {
            try AIOutputReport.make(exchange).write(to: url, options: .atomic)
            ToastCenter.shared.show("Saved \(url.lastPathComponent). Nothing was sent.")
            return url
        } catch {
            let message = "The file could not be saved. Choose another place and try again."
            self.error = message
            ToastCenter.shared.show(message, kind: .error)
            return nil
        }
    }

    // MARK: Engine

    func stopRuntime() async {
        guard let core else { return }
        do {
            _ = try await core.aiStopRuntime()
            ToastCenter.shared.show("Offline AI stopped. It starts again when you ask something.")
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        await refreshStatus()
    }
}

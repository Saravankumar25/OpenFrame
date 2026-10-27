// The assistant panel's model (Windows: ai/Conversation.tsx). It holds what the panel shows (status,
// the conversation, the typed question, the progress line) and sends every request to an `ai.*`
// operation. The core decides what is allowed and what the answer is; nothing here judges an
// answer. One conversation id is kept per conversation and sent with every question, so the core's
// focus, memory and "no re-ask" work. The typed question is never lost: when a request fails it
// goes back into the box.

import Foundation
import Observation
import OpenFrameCore
import SwiftUI

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
    /// The conversation the running request belongs to.
    private(set) var running: String?
    /// The running request itself. Two questions in one conversation share `running`; this tells
    /// them apart, so an earlier request that ends late never ends the newer one.
    @ObservationIgnored private var runningRequest: UUID?
    /// Cancel was pressed for the running request and the core is stopping it.
    private(set) var stopping = false
    /// What the running request is doing now (its latest progress line).
    private(set) var progress: AIProgress?
    /// A plain message about the last request or action; cleared by the next one.
    var error: String?
    /// The change set an action is running on.
    var busyChangeSet: String?
    var loadedHistory = false
    /// Asks the person where to save the conversation. Tests set a fixed answer.
    @ObservationIgnored var chooseSaveLocation: @MainActor (_ title: String, _ name: String) -> URL? = {
        AIFilePicker.chooseSaveLocation(title: $0, name: $1)
    }
    /// Says a new progress line to VoiceOver. Tests record the lines instead.
    @ObservationIgnored var announce: @MainActor (String) -> Void = { AccessibilityNotification.Announcement($0).post() }
    /// Tests answer the core calls themselves; the app uses the running core.
    @ObservationIgnored var invoker: (any CoreInvoking)?

    /// `nil`: the latest conversation. `.some(nil)`: a new one. `.some(id)`: this one.
    private var conversation: String??
    /// The id `ai.history` named for the latest conversation.
    private var historyConversation: String?
    @ObservationIgnored private var askTask: Task<Void, Never>?
    @ObservationIgnored private var progressTask: Task<Void, Never>?

    static let maxLength = 2000

    init(app: AppModel) {
        self.app = app
    }

    var core: (any CoreInvoking)? { invoker ?? app.core }

    // MARK: Derived

    var mode: AILogic.PanelMode { AILogic.panelMode(status) }
    var scopeOptions: [AiScopeKind] { AILogic.scopeOptions(context) }
    var effectiveScope: AiScopeKind {
        if let chosenScope, scopeOptions.contains(chosenScope) { return chosenScope }
        return AILogic.defaultScope(context)
    }
    var canSend: Bool { !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && pending == nil }
    /// A question can be sent now (a starter question fills the box itself).
    var canAskNow: Bool { pending == nil }

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
            historyConversation = h.conversationId
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        loadedHistory = true
    }

    // MARK: Asking

    /// The conversation the next question belongs to: the one shown, the latest one the history
    /// named, or a new id made here so Cancel can reach the request before its answer exists.
    var conversationForAsk: String {
        (conversation == .some(nil) ? nil : (conversation ?? nil) ?? historyConversation) ?? AILogic.newConversationId()
    }

    /// The conversation on screen, for "Save conversation for review" (none before the first answer).
    var conversationId: String? { exchanges.first?.conversationId }

    /// The line the running request shows.
    var workingLine: String { stopping ? AIText.stopping : (progress?.label ?? AIText.working) }

    /// Start a request. The box is emptied at once; it is filled again if the request fails.
    func send() {
        let question = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !question.isEmpty, pending == nil else { return }
        let id = conversationForAsk
        let request = UUID()
        pending = question
        running = id
        runningRequest = request
        stopping = false
        progress = nil
        text = ""
        error = nil
        listenForProgress()
        askTask = Task { [weak self] in
            await self?.ask(question, conversationId: id, request: request)
        }
    }

    /// Awaitable form for tests: the same path as `send`.
    func sendAndWait() async {
        send()
        await askTask?.value
    }

    /// Wait for the running request (tests).
    func waitForAnswer() async { await askTask?.value }

    private func ask(_ question: String, conversationId: String, request: UUID) async {
        defer { finishRunning(request) }
        guard let core else {
            finishFailed(question, message: AIText.unavailable)
            return
        }
        let scope = await resolvedScopeArgs()
        do {
            let exchange = try await core.aiAsk(AiAskArgs(text: question, scope: scope, conversationId: conversationId))
            conversation = .some(exchange.conversationId)
            await loadHistory()
            // The answer is on screen: the question and its Working line go in the same update.
            finishRunning(request)
            // Navigation is application control, not a project change: it runs directly (AI spec 5.3).
            if exchange.result.kind == .navigate, let route = AIRoute(exchange.result.nav) { route.perform() }
            await refreshStatus()
        } catch {
            finishFailed(question, message: CoreFailure(asCore: error).message)
        }
    }

    private func finishFailed(_ question: String, message: String) {
        // The question goes back into the box, unless the person already started typing another.
        if text.isEmpty { text = question }
        error = message
        Task { await refreshStatus() }
    }

    /// End `request`: the question, its Working line, its progress and its Cancel go together (as
    /// on Windows). A request that already ended, or was followed by a newer one, changes nothing.
    private func finishRunning(_ request: UUID) {
        guard runningRequest == request else { return }
        runningRequest = nil
        progressTask?.cancel()
        progressTask = nil
        pending = nil
        running = nil
        stopping = false
        progress = nil
    }

    /// Follow the core's progress lines while a request runs.
    private func listenForProgress() {
        progressTask?.cancel()
        guard let stream = app.core?.events() else { return }
        progressTask = Task { [weak self] in
            for await event in stream {
                guard let self, !Task.isCancelled else { return }
                self.receive(event)
            }
        }
    }

    /// One app event: a progress line of the running conversation replaces the line shown and is
    /// read out once.
    func receive(_ event: AppEvent) {
        guard let next = AILogic.progress(event, conversation: running) else { return }
        let changed = next.label != progress?.label
        progress = next
        if changed { announce(next.label) }
    }

    /// Ask the core to stop the running request. Nothing was changed by it; its answer says so and
    /// arrives as usual. When nothing could be stopped yet, Cancel can be pressed again.
    func cancel() async {
        guard let id = running, !stopping, let core else { return }
        stopping = true
        do {
            let r = try await core.aiCancel(AiCancelArgs(conversationId: id))
            if !r.stopping, running == id { stopping = false }
        } catch {
            if running == id { stopping = false }
            self.error = CoreFailure(asCore: error).message
        }
    }

    func newConversation() {
        guard pending == nil else { return }
        conversation = .some(nil)
        exchanges = []
        error = nil
    }

    // MARK: Save conversation for review

    /// Save this conversation to a file the person chooses. Only their click writes anything, and
    /// only where the Save panel says. Returns what the core wrote.
    @discardableResult
    func saveForReview() async -> AiReportResult? {
        guard let id = conversationId, let core else { return nil }
        guard let url = chooseSaveLocation(AIText.saveConversation, "openframe-ai-conversation.json") else { return nil }
        do {
            let r = try await core.aiExportConversation(AiConversationExportArgs(conversationId: id, path: url.path))
            ToastCenter.shared.show(AILogic.savedMessage(r))
            return r
        } catch {
            ToastCenter.shared.show(CoreFailure(asCore: error).message, kind: .error)
            return nil
        }
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

    private func run(_ cs: AiChangeSetDto, _ op: (any CoreInvoking) async throws -> AiChangeSetDto) async -> AiChangeSetDto? {
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

    // MARK: Engine

    func stopRuntime() async {
        guard let core else { return }
        // A running request is asked to stop first, so its answer arrives instead of a wait.
        if running != nil { await cancel() }
        do {
            _ = try await core.aiStopRuntime()
            ToastCenter.shared.show("Offline AI stopped. It starts again when you ask something.")
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        await refreshStatus()
    }
}

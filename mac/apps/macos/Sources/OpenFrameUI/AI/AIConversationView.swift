// The assistant conversation (Windows: ai/Conversation.tsx). One request runs a bounded,
// multi-step agent on this computer; the answer shows its sources as chips that open the place in
// OpenFrame. Exact facts are marked "Exact"; text the AI wrote is marked "Inferred". Project
// changes arrive only as a "Proposed Changes" card the person reviews.

import AppKit
import OpenFrameCore
import SwiftUI

struct AIConversationView: View {
    @Bindable var model: AIAssistantModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @FocusState private var inputFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            controls
            transcript
            Divider()
            input
        }
        .task(id: app.changes.key(["ai_request", "ai_result", "change_set"])) { await model.loadHistory() }
    }

    // MARK: Scope and state lines

    private var controls: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Scope").font(.caption).foregroundStyle(theme["muted"])
                Picker("Scope", selection: Binding(get: { model.effectiveScope }, set: { model.chosenScope = $0 })) {
                    ForEach(model.scopeOptions, id: \.self) { Text($0.label).tag($0) }
                }
                .labelsHidden()
                .accessibilityLabel("Scope")
                .accessibilityIdentifier("ai-scope")
                Button("New") { model.newConversation() }
                    .buttonStyle(.borderless)
                    .help("Start a new conversation")
                    .disabled(model.pending != nil)
                    .accessibilityIdentifier("ai-new-conversation")
            }
            Text(model.usingLabel).font(.caption).foregroundStyle(theme["muted"])
                .accessibilityIdentifier("ai-scope-line")
            if let status = model.status, status.runtimeState == "Failed" {
                BDBanner(.warn, "\(status.runtimeMessage ?? AIText.unavailable) Ask again to restart it.")
                    .accessibilityIdentifier("ai-runtime-failed")
            }
            if let label = model.status.flatMap({ AILogic.runtimeLabel($0.runtimeState) }) {
                Text(label).font(.caption).foregroundStyle(theme["muted"]).accessibilityIdentifier("ai-runtime-state")
            }
            if let notice = model.indexNotice {
                (Text(notice.preparing).bold() + Text(" " + notice.degraded))
                    .font(.caption)
                    .padding(8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme["note-bg"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .accessibilityIdentifier("ai-index-notice")
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    // MARK: Transcript

    private var transcript: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    if model.exchanges.isEmpty && model.pending == nil {
                        Text(AIText.emptyConversation).font(.caption).foregroundStyle(theme["muted"])
                            .accessibilityIdentifier("ai-empty")
                    }
                    ForEach(model.exchanges, id: \.requestId) { x in
                        AIExchangeView(model: model, exchange: x)
                    }
                    if let pending = model.pending {
                        AIBubble(text: pending, user: true).accessibilityIdentifier("ai-pending-question")
                        HStack(spacing: 8) {
                            ProgressView().controlSize(.small)
                            Text("Working on it on this computer\u{2026}").font(.callout).foregroundStyle(theme["muted"])
                            Spacer()
                            Button("Cancel") { Task { await model.cancel() } }
                                .keyboardShortcut(.cancelAction)
                                .accessibilityIdentifier("ai-cancel")
                        }
                        .accessibilityElement(children: .contain)
                        .accessibilityIdentifier("ai-working")
                    }
                    Color.clear.frame(height: 1).id("ai-end")
                }
                .padding(12)
            }
            .accessibilityIdentifier("ai-conversation")
            .onChange(of: model.exchanges.count) { _, _ in proxy.scrollTo("ai-end", anchor: .bottom) }
            .onChange(of: model.pending) { _, _ in proxy.scrollTo("ai-end", anchor: .bottom) }
        }
    }

    // MARK: Input

    private var input: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let error = model.error {
                BDBanner(.err, error).accessibilityIdentifier("ai-error")
            }
            HStack(alignment: .bottom, spacing: 8) {
                TextField("Ask or command\u{2026} e.g. \u{201C}Open Scene 42.\u{201D}", text: $model.text, axis: .vertical)
                    .lineLimit(2...6)
                    .textFieldStyle(.roundedBorder)
                    .focused($inputFocused)
                    .disabled(model.pending != nil)
                    .onSubmit { model.send() }
                    .onChange(of: model.text) { _, new in
                        if new.count > AIAssistantModel.maxLength { model.text = String(new.prefix(AIAssistantModel.maxLength)) }
                    }
                    .accessibilityLabel("Ask or command")
                    .accessibilityIdentifier("ai-input")
                Button("Send") { model.send() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!model.canSend)
                    .accessibilityIdentifier("ai-send")
            }
        }
        .padding(12)
    }
}

struct AIBubble: View {
    let text: String
    var user = false
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Text(text)
            .textSelection(.enabled)
            .font(.callout)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(user ? theme["accent-soft"] : theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .frame(maxWidth: .infinity, alignment: user ? .trailing : .leading)
    }
}

struct AIExchangeView: View {
    @Bindable var model: AIAssistantModel
    let exchange: AiExchange

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            AIBubble(text: exchange.requestText, user: true).accessibilityIdentifier("ai-question")
            AIResultView(model: model, exchange: exchange)
            if let cs = exchange.result.changeSet {
                AIProposedChangesView(model: model, cs: cs)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ai-exchange-\(exchange.requestId)")
    }
}

struct AIResultView: View {
    @Bindable var model: AIAssistantModel
    let exchange: AiExchange
    @Environment(\.ofTheme) private var theme

    private var r: AiResultDto { exchange.result }

    var body: some View {
        let conf = AILogic.confidence(r.confidence)
        let route = AIRoute(r.nav)
        let chips = AILogic.sourceChips(r.provenance)
        let isError = r.kind == .failed || r.kind == .unavailable
        VStack(alignment: .leading, spacing: 6) {
            Group {
                if [.denied, .answer, .proposal, .suggestion].contains(r.kind) {
                    Text(r.content).bold()
                } else {
                    Text(r.content)
                }
            }
            .textSelection(.enabled)
            .foregroundStyle(isError ? theme["red-t"] : theme["ink"])
            .accessibilityIdentifier("ai-answer-text")
            if !r.details.isEmpty {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(r.details.enumerated()), id: \.offset) { _, d in
                        Text(r.kind == .denied && !d.hasPrefix("Nothing") ? "\u{2022} \(d)" : d)
                    }
                }
                .font(.caption).foregroundStyle(theme["muted"])
            }
            if !r.items.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(r.items.prefix(25).enumerated()), id: \.offset) { i, item in
                        HStack(spacing: 6) {
                            if r.kind == .suggestion { BDChip(text: "Suggested", tone: .yellow) }
                            (Text(item.label) + Text(item.detail.map { " \u{00B7} \($0)" } ?? "").foregroundColor(theme["muted"]))
                                .font(.callout)
                            Spacer(minLength: 4)
                            if let to = AIRoute(item.nav) {
                                Button("Show") { to.perform() }
                                    .controlSize(.small)
                                    .accessibilityIdentifier("ai-item-show-\(i)")
                            }
                        }
                    }
                    if r.items.count > 25 {
                        Text("\u{2026}and \(r.items.count - 25) more").font(.caption).foregroundStyle(theme["muted"])
                    }
                }
                .accessibilityIdentifier("ai-items")
            }
            if conf != nil || route != nil {
                HStack(spacing: 6) {
                    if let conf {
                        BDChip(text: conf.text, tone: conf.tone == "green" ? .green : conf.tone == "blue" ? .blue : .yellow)
                            .accessibilityIdentifier("ai-confidence")
                    }
                    if let route {
                        Button(r.kind == .navigate ? "Open again" : "Open") { route.perform() }
                            .controlSize(.small)
                            .accessibilityIdentifier("ai-open")
                    }
                }
            }
            if !chips.isEmpty {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text("Based on:").font(.caption).foregroundStyle(theme["muted"])
                    BDFlowLayout(spacing: 4) {
                        ForEach(chips) { chip in
                            if let route = chip.route {
                                Button(chip.text) { route.perform() }
                                    .buttonStyle(.plain)
                                    .font(.caption.weight(.medium))
                                    .foregroundStyle(theme["blue-t"])
                                    .padding(.horizontal, 7).padding(.vertical, 2)
                                    .background(theme["blue-soft"], in: Capsule())
                                    .help("Open")
                                    .accessibilityIdentifier("ai-source")
                            } else {
                                BDChip(text: chip.text).accessibilityIdentifier("ai-source")
                            }
                        }
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Sources")
                .accessibilityIdentifier("ai-sources")
            }
            if let steps = AILogic.stepsSummary(r.steps) {
                Text(steps).font(.caption).foregroundStyle(theme["muted"]).accessibilityIdentifier("ai-steps")
            }
            HStack {
                Spacer()
                Button("Report AI output\u{2026}") { Task { await model.reportOutput(exchange) } }
                    .buttonStyle(.borderless)
                    .font(.caption)
                    .help("Save this question and answer to a file you choose. Nothing is sent.")
                    .accessibilityIdentifier("ai-report-output")
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ai-result-\(r.kind.rawValue.lowercased())")
    }
}

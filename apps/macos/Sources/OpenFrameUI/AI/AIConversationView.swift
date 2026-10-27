// The assistant conversation (Windows: ai/Conversation.tsx). One request runs a bounded,
// multi-step agent on this computer; the answer is plain conversational text. The places it was
// read from wait behind "See sources"; how it was worked out (steps, how sure) stays out of the
// conversation and goes into "Report AI output" and the AI log. Long lists show a preview and
// "Show all". Project changes arrive only as a "Proposed Changes" card the person reviews.

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
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityIdentifier("ai-empty")
                        // Questions to start with, for this page: one click asks them.
                        Text("Try asking").font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
                        VStack(alignment: .leading, spacing: 6) {
                            ForEach(AIText.starterQuestions(for: model.context.workspace), id: \.self) { q in
                                Button {
                                    model.text = q
                                    model.send()
                                } label: {
                                    Label(q, systemImage: "text.bubble")
                                        .font(.callout)
                                        .multilineTextAlignment(.leading)
                                        .fixedSize(horizontal: false, vertical: true)
                                }
                                .buttonStyle(.bordered)
                                .disabled(!model.canAskNow)
                                .accessibilityIdentifier("ai-starter")
                            }
                        }
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
                    .buttonStyle(.ofProminent)
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
    @State private var allItems = false
    @State private var sourcesOpen = false

    private var r: AiResultDto { exchange.result }

    var body: some View {
        let route = AIRoute(r.nav)
        let sources = AILogic.answerSources(r.provenance)
        let isError = r.kind == .failed || r.kind == .unavailable
        let items = AILogic.visibleItems(r.items, all: allItems)
        VStack(alignment: .leading, spacing: 6) {
            // The answer is plain text at normal weight, paragraphs kept (OpenFrameAI.docx 4.1).
            Text(r.content)
                .textSelection(.enabled)
                .lineSpacing(2)
                .fixedSize(horizontal: false, vertical: true)
                .foregroundStyle(isError ? theme["red-t"] : theme["ink"])
                .accessibilityIdentifier("ai-answer-text")
            // A refusal, failure or question needs its reasons in view. For an ordinary answer the
            // details are how it was worked out, which stays out of the conversation.
            if AILogic.reasonsVisible(r.kind) && !r.details.isEmpty {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(r.details.enumerated()), id: \.offset) { _, d in
                        Text(r.kind == .denied && !d.hasPrefix("Nothing") ? "\u{2022} \(d)" : d)
                    }
                }
                .font(.caption).foregroundStyle(theme["muted"])
                .fixedSize(horizontal: false, vertical: true)
            }
            if !r.items.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(items.enumerated()), id: \.offset) { i, item in
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            if r.kind == .suggestion { BDChip(text: "Suggested", tone: .yellow) }
                            (Text(item.label) + Text(item.detail.map { " \u{00B7} \($0)" } ?? "").foregroundColor(theme["muted"]))
                                .font(.callout)
                                .fixedSize(horizontal: false, vertical: true)
                            Spacer(minLength: 4)
                            if let to = AIRoute(item.nav) {
                                Button("Open") { to.perform() }
                                    .controlSize(.small)
                                    .help("Open \(item.label)")
                                    .accessibilityLabel("Open \(item.label)")
                                    .accessibilityIdentifier("ai-item-show-\(i)")
                            }
                        }
                    }
                    if r.items.count > AILogic.previewItems {
                        Button(allItems ? "Show fewer" : "Show all \(r.items.count)") { allItems.toggle() }
                            .buttonStyle(.borderless)
                            .font(.caption)
                            .accessibilityValue(allItems ? "expanded" : "collapsed")
                            .accessibilityIdentifier("ai-items-show-all")
                    }
                }
                .accessibilityIdentifier("ai-items")
            }
            if let route {
                Button(r.kind == .navigate ? "Open again" : "Open") { route.perform() }
                    .controlSize(.small)
                    .accessibilityIdentifier("ai-open")
            }
            if !sources.isEmpty {
                seeSources(sources)
            }
            HStack {
                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(AILogic.copyText(r), forType: .string)
                    ToastCenter.shared.show("Copied the answer")
                } label: {
                    Label("Copy", systemImage: "doc.on.doc")
                }
                .buttonStyle(.borderless)
                .font(.caption)
                .help("Copy this answer, with its list if it has one")
                .accessibilityIdentifier("ai-copy")
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

    /// The places the answer was read from, closed until asked for. Each one that has a place in
    /// OpenFrame opens it.
    private func seeSources(_ sources: [AILogic.SourceChip]) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Button {
                withAnimation(.easeOut(duration: 0.15)) { sourcesOpen.toggle() }
            } label: {
                Label("See sources", systemImage: sourcesOpen ? "chevron.down" : "chevron.right")
                    .font(.caption)
                    .foregroundStyle(theme["muted"])
            }
            .buttonStyle(.plain)
            .accessibilityValue(sourcesOpen ? "expanded" : "collapsed")
            .accessibilityIdentifier("ai-see-sources")
            if sourcesOpen {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(sources) { chip in
                        if let route = chip.route {
                            Button(chip.text) { route.perform() }
                                .buttonStyle(.plain)
                                .font(.caption.weight(.medium))
                                .foregroundStyle(theme["blue-t"])
                                .help("Open \(chip.text)")
                                .accessibilityIdentifier("ai-source")
                        } else {
                            Text(chip.text).font(.caption).foregroundStyle(theme["muted"])
                                .accessibilityIdentifier("ai-source")
                        }
                    }
                }
                .padding(.leading, 16)
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Sources")
                .accessibilityIdentifier("ai-sources")
            }
        }
    }
}

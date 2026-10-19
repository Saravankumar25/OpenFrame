// The "Proposed Changes" review card (Windows: ai/ProposedChanges.tsx). Every AI-prepared project
// change arrives here as ONE Change Set: summary, affected areas and an exact preview, then the
// person's explicit choice. Nothing is applied by the assistant itself. A stale or conflicting
// proposal can never be applied from this card; it must be re-checked first. A change that deletes
// things needs the product's usual confirmation.

import OpenFrameCore
import SwiftUI

struct AIProposedChangesView: View {
    @Bindable var model: AIAssistantModel
    let cs: AiChangeSetDto
    @Environment(\.ofTheme) private var theme
    @State private var confirming = false

    private var actions: AILogic.ChangeSetActions { AILogic.changeSetActions(cs) }
    private var stale: Bool { AILogic.isStale(cs) }
    private var busy: Bool { model.busyChangeSet != nil }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            card
            if let text = AILogic.changeSetStateText(cs) {
                Text(text)
                    .font(.caption)
                    .foregroundStyle(cs.state == "Failed" ? theme["red-t"] : theme["muted"])
                    .accessibilityIdentifier("ai-change-state")
            }
            buttons
            if stale {
                Text(AIText.staleHint).font(.caption).foregroundStyle(theme["muted"]).accessibilityIdentifier("ai-stale-hint")
            }
        }
        .alert("Apply changes that delete items?", isPresented: $confirming) {
            Button("Cancel", role: .cancel) {}
                .accessibilityIdentifier("ai-confirm-cancel")
            Button("Apply Changes", role: .destructive) { Task { await model.apply(cs, confirmDestructive: true) } }
                .accessibilityIdentifier("ai-confirm-apply")
        } message: {
            Text("Some of these changes delete or permanently change items. Deleted items go to Recently Deleted, and you can use Undo to reverse the whole change.")
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ai-change-set-\(cs.id)")
    }

    private var card: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(stale ? "Proposed Changes (out of date)" : "Proposed Changes")
                .font(.caption.weight(.semibold))
                .foregroundStyle(stale ? theme["yellow-t"] : theme["muted"])
                .padding(.horizontal, 10).padding(.vertical, 6)
                .accessibilityAddTraits(.isHeader)
                .accessibilityIdentifier("ai-change-title")
            Divider()
            VStack(alignment: .leading, spacing: 2) {
                Text(cs.title).bold()
                if !cs.summary.isEmpty { Text(cs.summary).font(.callout) }
                if stale {
                    Text(cs.staleReason ?? "The project changed after this suggestion was prepared. Review is required before applying it.")
                        .font(.callout)
                        .foregroundStyle(theme["yellow-t"])
                        .accessibilityIdentifier("ai-stale-reason")
                }
            }
            .padding(10)
            line("Affected areas", cs.affectedModules.isEmpty ? "None" : cs.affectedModules.joined(separator: ", "), bold: true)
            ForEach(Array(cs.rows.enumerated()), id: \.offset) { _, row in
                if row.tone == "section" {
                    HStack {
                        Text(row.label).bold()
                        Spacer()
                        Text(row.value).font(.caption).foregroundStyle(theme["muted"])
                    }
                    .padding(.horizontal, 10).padding(.vertical, 4)
                    .background(theme["surface2"])
                } else {
                    line(row.label, row.value, bold: true, color: row.tone == "destructive" ? theme["red-t"] : nil)
                }
            }
            ForEach(Array(cs.exclusions.enumerated()), id: \.offset) { _, row in
                line(row.label, row.value, bold: true, color: theme["muted"])
            }
            if !cs.targets.isEmpty {
                Text("Items: " + cs.targets.map(\.label).joined(separator: ", "))
                    .font(.caption).foregroundStyle(theme["muted"])
                    .padding(.horizontal, 10).padding(.vertical, 4)
            }
            if cs.state == "Pending" {
                Text("\(AILogic.changeCount(cs.operationCount))\(cs.partCount > 1 ? " in \(Int(cs.partCount)) parts" : ""), applied together as one step you can undo. If any part fails, nothing is changed.")
                    .font(.caption).foregroundStyle(theme["muted"])
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .accessibilityIdentifier("ai-change-count")
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["bg"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).stroke(stale ? theme["yellow-line"] : theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Proposed Changes")
    }

    private func line(_ label: String, _ value: String, bold: Bool, color: Color? = nil) -> some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label).font(.callout).foregroundStyle(color ?? theme["ink2"])
            Spacer(minLength: 8)
            Text(value).font(.callout.weight(bold ? .semibold : .regular)).foregroundStyle(color ?? theme["ink"])
                .multilineTextAlignment(.trailing)
        }
        .padding(.horizontal, 10).padding(.vertical, 4)
    }

    @ViewBuilder private var buttons: some View {
        if actions.apply || actions.reject || actions.recheck {
            HStack {
                Spacer()
                if actions.reject {
                    Button("Reject") { Task { await model.reject(cs) } }
                        .disabled(busy)
                        .accessibilityIdentifier("ai-reject")
                }
                if actions.recheck {
                    Button("Re-check & Review") { Task { await model.recheck(cs) } }
                        .buttonStyle(.borderedProminent)
                        .disabled(busy)
                        .accessibilityIdentifier("ai-recheck")
                }
                if actions.apply {
                    Button("Apply Changes") {
                        if cs.requiresConfirmation { confirming = true } else { Task { await model.apply(cs) } }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(busy)
                    .accessibilityIdentifier("ai-apply")
                }
            }
        } else if cs.state == "Applied", cs.id == model.latestAppliedChangeSetId {
            HStack {
                Spacer()
                Button("Undo") { Task { await model.undo() } }
                    .disabled(busy)
                    .help("Undo reverses all of this change in one step")
                    .accessibilityIdentifier("ai-undo")
            }
        }
    }
}

// The panels on the right of the board: a scene's details, Schedule assistance (warnings), and
// Grouping suggestions (UX 3.29, mocks 134 to 138). Warnings and suggestions are advice: nothing
// here blocks you and nothing moves a scene unless you press the button that says so.

import SwiftUI
import OpenFrameCore

/// The frame of a panel: a title, the kind of panel, a Close button, the content and a footer note.
struct SchedDrawerFrame<Content: View>: View {
    let title: String
    let typeLabel: String
    let identifier: String
    var footer: String?
    let onClose: () -> Void
    @ViewBuilder var content: () -> Content
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 1) {
                    Text(typeLabel).font(.caption).foregroundStyle(theme["muted"])
                    Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                }
                Spacer()
                Button("Close", action: onClose)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("drawer-close")
            }
            .padding(14)
            Divider()
            ScrollView { VStack(alignment: .leading, spacing: 10) { content() }.padding(14) }
            if let footer {
                Divider()
                Text(footer).font(.caption).foregroundStyle(theme["muted"]).padding(10)
            }
        }
        .frame(width: 350)
        .background(theme["panel"])
        .overlay(alignment: .leading) { Rectangle().fill(theme["line"]).frame(width: 1) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier(identifier)
    }
}

struct ScheduleDrawerHost: View {
    @Bindable var model: ScheduleModel
    let drawer: SchedDrawer

    var body: some View {
        switch drawer {
        case let .strip(id):
            if let s = model.strip(id) { SchedStripDrawer(model: model, strip: s) }
        case .warnings: SchedWarningsDrawer(model: model)
        case .suggestions: SchedSuggestionsDrawer(model: model)
        }
    }
}

// MARK: Scene details

struct SchedStripDrawer: View {
    @Bindable var model: ScheduleModel
    let strip: ScheduleStripDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let day = model.day(strip.dayId)
        let computed = day?.timeline.rows.first { $0.itemId == strip.id }?.startLabel
        let title = SchedFormat.sceneName(strip)
        let castText = castLine
        SchedDrawerFrame(title: title, typeLabel: "Scheduled scene", identifier: "strip-drawer", onClose: { model.drawer = nil }) {
            Text(strip.heading).font(.headline).accessibilityIdentifier("strip-drawer-heading")
            if !strip.synopsis.isEmpty { Text(strip.synopsis).font(.callout).foregroundStyle(theme["muted"]) }
            if strip.sourceState != "Current" {
                BDBanner(tone: strip.sourceState == "Removed" ? .err : .warn, content: {
                    Text(strip.changeKinds.isEmpty ? strip.sourceState : strip.changeKinds.joined(separator: " \u{00B7} "))
                }, actions: {
                    if strip.sourceState == "Removed" {
                        Button("Confirm removal") { Task { await model.confirmRemoval(strip) } }
                            .controlSize(.small).accessibilityIdentifier("strip-confirm-removal")
                    } else {
                        Button("Mark reviewed") { Task { await model.acknowledgeChange(strip) } }
                            .controlSize(.small).accessibilityIdentifier("strip-mark-reviewed")
                    }
                })
            }
            Divider()
            Text("**Scheduled:** \(day?.title ?? "Unscheduled")").font(.callout)
            if strip.locations.isEmpty {
                Text("**Location:** Missing").font(.callout).foregroundStyle(theme["red-t"])
            } else {
                Text("**Location:** " + strip.locations.map { $0.name + ($0.status.map { " (\($0))" } ?? ($0.fromBreakdown ? "" : " (from the heading)")) }.joined(separator: ", ")).font(.callout)
            }
            Text("**Cast:** " + castText).font(.callout)
            Divider()
            SchedCommitField(
                id: "strip-estimate", label: "Estimated shooting time", value: strip.estimatedMinutes.map { SchedFormat.minutes($0) } ?? "",
                placeholder: "Missing", hint: "Entered by you. Never guessed.", disabled: model.isFinalized
            ) { await model.setEstimate(strip, $0) }
            if strip.dayId != nil {
                SchedCommitField(
                    id: "strip-start", label: "Start time", value: strip.startMinutes.map { SchedFormat.clock($0) } ?? "",
                    placeholder: computed ?? "HH:MM",
                    hint: computed != nil && strip.startMinutes == nil
                        ? "Estimated from the day's order: \(computed ?? ""). Enter a time to fix it."
                        : "Leave empty to follow the day's order. The scene never moves.",
                    disabled: model.isFinalized
                ) { await model.setStart(strip, $0) }
            }
            SchedCommitField(
                id: "strip-pages", label: "Page count override", value: strip.pagesOverridden ? SchedFormat.pages(strip.pageEighths) : "",
                placeholder: SchedFormat.pages(strip.pageEighths),
                hint: "From the screenplay: \(strip.pagesOverridden ? "overridden" : SchedFormat.pages(strip.pageEighths)). Leave empty to use it.",
                disabled: model.isFinalized
            ) { await model.setPages(strip, $0) }
            VStack(alignment: .leading, spacing: 6) {
                if strip.dayId != nil {
                    Button("Remove from Day") { Task { await model.removeFromDay(strip) } }
                        .disabled(model.isFinalized).accessibilityIdentifier("strip-remove-from-day")
                }
                Button { open(.screenplay) } label: { Label("Open in Screenplay", systemImage: "arrow.right") }
                    .buttonStyle(.link).accessibilityIdentifier("strip-open-screenplay")
                Button { open(.breakdown) } label: { Label("Open in Breakdown", systemImage: "arrow.right") }
                    .buttonStyle(.link).accessibilityIdentifier("strip-open-breakdown")
            }
            Text("The heading and scene number come from the screenplay source and are edited there. Scheduling never changes the script.")
                .font(.caption).foregroundStyle(theme["muted"])
        }
    }

    private var castLine: String {
        if strip.cast.isEmpty { return "None confirmed" }
        var names: [String] = []
        for c in strip.cast {
            if let a = c.actor { names.append("\(a) (\(c.character))") } else { names.append(c.character) }
        }
        return names.joined(separator: ", ")
    }

    private func open(_ ws: Workspace) {
        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: ws.rawValue)
    }
}

// MARK: Warnings

struct SchedWarningsDrawer: View {
    @Bindable var model: ScheduleModel

    var body: some View {
        let all = model.view?.warnings ?? []
        let open = all.filter { $0.decision == nil }
        let decided = all.filter { $0.decision != nil }
        SchedDrawerFrame(
            title: "Schedule assistance", typeLabel: "Practical observations", identifier: "warnings-drawer",
            footer: "Warnings never block you and nothing is fixed automatically.", onClose: { model.drawer = nil }
        ) {
            if all.isEmpty {
                Text("No warnings. The schedule looks consistent.").font(.callout).foregroundStyle(.secondary)
                    .accessibilityIdentifier("warnings-empty")
            }
            ForEach(open, id: \.key) { SchedWarningCard(model: model, w: $0) }
            if !decided.isEmpty {
                Text("Kept or dismissed").font(.headline).padding(.top, 6)
                ForEach(decided, id: \.key) { SchedWarningCard(model: model, w: $0) }
            }
        }
    }
}

struct SchedWarningCard: View {
    @Bindable var model: ScheduleModel
    let w: ScheduleWarningDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(w.message).font(.callout)
            if let d = w.detail { Text(d).font(.caption).foregroundStyle(theme["muted"]) }
            HStack(spacing: 6) {
                if let dayId = w.dayId {
                    Button("Open affected item") { open(dayId) }.controlSize(.small)
                        .accessibilityIdentifier("warning-open-\(w.key)")
                    if w.kind != "call_sheet_stale" {
                        Button("Move scene (you decide)") { open(dayId) }.controlSize(.small)
                            .accessibilityIdentifier("warning-move-\(w.key)")
                    }
                }
            }
            HStack(spacing: 6) {
                if let decision = w.decision {
                    SchedChip(text: decision == "Kept" ? "Kept anyway" : "Dismissed", tone: decision == "Kept" ? .green : .plain, symbol: "checkmark")
                        .accessibilityIdentifier("warning-decision-\(w.key)")
                    Button("Show again") { Task { await model.decide(w, nil) } }
                        .buttonStyle(.link).controlSize(.small)
                        .accessibilityIdentifier("warning-show-again-\(w.key)")
                } else {
                    Button("Keep anyway") { Task { await model.decide(w, "Kept") } }.controlSize(.small)
                        .accessibilityIdentifier("warning-keep-\(w.key)")
                    Button("Dismiss") { Task { await model.decide(w, "Dismissed") } }
                        .buttonStyle(.link).controlSize(.small)
                        .accessibilityIdentifier("warning-dismiss-\(w.key)")
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(w.decision != nil ? theme["surface2"] : theme["panel"], in: RoundedRectangle(cornerRadius: 9, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(theme["line"]))
        .opacity(w.decision != nil ? 0.8 : 1)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("warning-\(w.key)")
    }

    private func open(_ dayId: String) {
        model.drawer = nil
        model.openDayId = dayId
    }
}

// MARK: Grouping suggestions

struct SchedSuggestionsDrawer: View {
    @Bindable var model: ScheduleModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let list = model.visibleSuggestions
        SchedDrawerFrame(
            title: "Grouping suggestions", typeLabel: "Schedule assistance", identifier: "suggestions-drawer",
            footer: "Suggestions are advice. OpenFrame never rearranges your schedule silently.", onClose: { model.drawer = nil }
        ) {
            Button { Task { await model.loadSuggestions() } } label: { Label("Ask for suggestion", systemImage: "sparkles") }
                .disabled(model.suggestionsLoading)
                .accessibilityIdentifier("suggestions-ask")
            if let s = model.suggestions, list.isEmpty {
                Text(s.message ?? "No obvious grouping improvement found.").font(.callout).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("suggestions-empty")
            }
            ForEach(list, id: \.key) { s in
                VStack(alignment: .leading, spacing: 5) {
                    Text(s.title).font(.callout.weight(.semibold))
                    SchedChip(text: s.subject, tone: s.kind == "location" ? .blue : .purple)
                    Text(s.advice).font(.callout)
                    if let l = s.suggestedDayLabel { Text("Most of them are already on \(l).").font(.caption).foregroundStyle(theme["muted"]) }
                    HStack {
                        Button("Apply manually") { model.sheet = .group(s) }
                            .buttonStyle(.borderedProminent).controlSize(.small)
                            .disabled(model.shootingDays.isEmpty || model.isFinalized)
                            .accessibilityIdentifier("suggestion-apply-\(s.key)")
                        Button("Dismiss") { model.dismissedSuggestions.insert(s.key) }
                            .buttonStyle(.link).controlSize(.small)
                            .accessibilityIdentifier("suggestion-dismiss-\(s.key)")
                    }
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["panel"], in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(theme["line"]))
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("suggestion-\(s.key)")
            }
        }
        .task { if model.suggestions == nil { await model.loadSuggestions() } }
    }
}

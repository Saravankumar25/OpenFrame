// The sheets of the Breakdown tab: choose the Production Source, review a source update, add an
// element (also "Tag as…" on selected words), edit or choose a match for a suggestion, review all
// suggestions, review a changed scene, and edit an element's notes (UX 3.22, mocks 103 to 111).

import SwiftUI
import OpenFrameCore

extension BreakdownModel {
    /// The draft the Select Source sheet starts on.
    var defaultSourceChoice: String? {
        guard let o = sourceOptions else { return nil }
        return o.activeDraftId ?? o.drafts.first { $0.status == "Locked" }?.id ?? o.drafts.first?.id
    }

    /// With a source already set, choosing another draft opens the update review instead of
    /// replacing it.
    func confirmSource(draftId: String, reason: String?) async {
        if let active = sourceOptions?.activeDraftId, active != draftId {
            sheet = .updateSource(draftId)
            return
        }
        await setSource(draftId: draftId, reason: reason)
    }
}

enum DraftText {
    static func summary(_ d: ProductionDraftInfo) -> String {
        let scenes = BDText.plural(d.sceneCount.asInt, "scene")
        if d.status == "Locked", let at = d.lockedAt {
            return "\(scenes) \u{00B7} locked \(Date(timeIntervalSince1970: at / 1000).formatted(date: .abbreviated, time: .omitted))"
        }
        return "Not locked \u{00B7} \(scenes)"
    }
}

// MARK: Select Source

struct SelectSourceSheet: View {
    @Bindable var model: BreakdownModel
    @State private var picked: String?
    @State private var reason = ""

    var body: some View {
        let drafts = model.sourceOptions?.drafts ?? []
        let active = model.sourceOptions?.activeDraftId
        let choice = picked ?? model.defaultSourceChoice
        let manyScreenplays = Set(drafts.map(\.screenplayId)).count > 1
        BDSheetFrame(title: "Handing the script over to production.", sub: "Which screenplay draft should become the production source?") {
            if model.sourceOptionsLoading && drafts.isEmpty {
                ProgressView().frame(maxWidth: .infinity)
            } else if drafts.isEmpty {
                BDBanner(.info, "There are no screenplay drafts yet. Write or import a screenplay first. Production is always based on a named draft.")
            } else {
                VStack(spacing: 8) {
                    ForEach(drafts, id: \.id) { d in
                        DraftRow(draft: d, selected: d.id == choice, current: d.id == active, showsTitle: manyScreenplays) { picked = d.id }
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Screenplay drafts")
                .accessibilityIdentifier("screenplay-drafts")
            }
            if active == nil, !drafts.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("OpenFrame will create or initialise").font(.callout.weight(.semibold))
                    Text("\u{2022} Breakdown scenes: one per screenplay scene, numbered automatically\n\u{2022} Production catalog\n\u{2022} The scene list used by shot lists, storyboards and the schedule")
                        .font(.callout).foregroundStyle(.secondary)
                    TextField("Why this draft? (optional)", text: $reason)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityIdentifier("source-reason")
                }
            }
            Text("You will not recreate any scene by hand. Your screenplay is not changed.").font(.caption).foregroundStyle(.secondary)
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button(active != nil ? "Review Production Update" : "Set as Production Source") {
                if let choice { Task { await model.confirmSource(draftId: choice, reason: reason) } }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(choice == nil || choice == active || model.busy)
            .accessibilityIdentifier("set-production-source")
        }
        .task { await model.loadSourceOptions() }
    }
}

private struct DraftRow: View {
    let draft: ProductionDraftInfo
    let selected: Bool
    let current: Bool
    let showsTitle: Bool
    let onPick: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Button(action: onPick) {
            HStack(spacing: 10) {
                Image(systemName: "doc.text").font(.title3).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text(draft.name + (showsTitle ? " \u{00B7} \(draft.screenplayTitle)" : "")).font(.callout.weight(.semibold))
                    Text(DraftText.summary(draft)).font(.caption).foregroundStyle(theme["muted"])
                }
                Spacer()
                if draft.status == "Locked" { BDChip(text: "Locked", tone: .green, symbol: "lock") }
                if current { BDChip(text: "Current source", tone: .accent) }
                if selected { Image(systemName: "checkmark.circle.fill").foregroundStyle(theme["accent"]).accessibilityHidden(true) }
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(selected ? theme["accent"] : theme["line"], lineWidth: selected ? 2 : 1))
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(draft.name)
        .accessibilityValue(DraftText.summary(draft))
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
        .accessibilityIdentifier("draft-\(draft.name)")
    }
}

// MARK: Update Source

struct UpdateSourceSheet: View {
    @Bindable var model: BreakdownModel
    let draftId: String
    @State private var showAll = false

    var body: some View {
        let p = model.updatePreview
        BDSheetFrame(
            title: p.map { "Update Production from \($0.to.name)" } ?? "Update Production Source",
            sub: p.map { "Production is currently based on \($0.from.name). \($0.to.createdAt > $0.from.createdAt ? "A newer revision is available." : "You chose a different draft.") Nothing changes until you confirm." },
            width: 600
        ) {
            if let p {
                let tiles: [(String, Int)] = [("Scenes added", p.added.asInt), ("Scenes removed", p.removed.asInt),
                                              ("Text changed", p.textChanged.asInt), ("Heading changed", p.headingChanged.asInt)]
                HStack(spacing: 10) {
                    ForEach(tiles, id: \.0) { t in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(t.0).font(.caption).foregroundStyle(.secondary)
                            Text("\(t.1)").font(.title2.weight(.semibold)).monospacedDigit()
                        }
                        .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        .accessibilityElement(children: .combine)
                    }
                }
                if p.moved > 0 || p.ambiguous > 0 {
                    Text((p.moved > 0 ? "\(p.moved.asInt) moved \u{00B7} " : "") + (p.ambiguous > 0 ? "\(p.ambiguous.asInt) need a manual match \u{00B7} " : "") + "\(p.unchanged.asInt) unchanged")
                        .font(.callout).foregroundStyle(.secondary)
                }
                if p.changes.isEmpty {
                    BDBanner(.ok, "No scene changes. Breakdown data carries over as it is.")
                } else {
                    let shown = showAll ? p.changes : Array(p.changes.prefix(8))
                    ScrollView {
                        VStack(spacing: 0) {
                            ForEach(Array(shown.enumerated()), id: \.offset) { _, c in
                                ChangeRow(change: c)
                                Divider()
                            }
                            if !showAll, p.changes.count > shown.count {
                                Text("\u{2026}and \(p.changes.count - shown.count) more.").font(.callout).foregroundStyle(.secondary).padding(8)
                            }
                        }
                    }
                    .frame(maxHeight: 260)
                }
                Text("Nothing is deleted: all \(p.elementsKept.asInt) breakdown elements are kept. Removed scenes keep their breakdown as history; changed scenes are flagged for review.")
                    .font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 80)
            }
        } footer: {
            Button("Dismiss") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            if let p {
                Button("Keep Production on \(p.from.name)") { model.sheet = nil }
                if p.changes.count > 8, !showAll { Button("Review Changes First") { showAll = true } }
            }
            Button("Update Production Baseline") { Task { await model.applyUpdate(draftId: draftId) } }
                .keyboardShortcut(.defaultAction)
                .disabled(p == nil || model.busy)
                .accessibilityIdentifier("update-production-baseline")
        }
        .task { await model.loadUpdatePreview(draftId: draftId) }
    }

    private struct ChangeRow: View {
        let change: ProductionSceneChange
        var body: some View {
            let (label, tone): (String, BDTone) = switch change.status {
            case "added": ("Needs Breakdown", .red)
            case "removed": ("Historical", .plain)
            default: ("Review", .yellow)
            }
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(change.newNumber ?? change.oldNumber ?? "").font(.callout.weight(.semibold)).frame(width: 34, alignment: .leading)
                VStack(alignment: .leading, spacing: 1) {
                    Text(change.heading.isEmpty ? "(no heading)" : change.heading).font(.callout.weight(.semibold))
                    Text(text).font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                BDChip(text: label, tone: tone)
            }
            .padding(.horizontal, 8).padding(.vertical, 6)
            .accessibilityElement(children: .combine)
        }

        private var text: String {
            switch change.status {
            case "added": return "New scene"
            case "removed":
                return change.elementCount > 0 ? "Removed. Its breakdown (\(change.elementCount.asInt)) is kept as history" : "Removed from the script"
            case "ambiguous": return "Scene identity is ambiguous (duplicated). The breakdown stays on the old scene for review"
            case "moved": return "Moved (was Scene \(change.oldNumber ?? ""))"
            default:
                var parts: [String] = []
                if change.headingChanged { parts.append("heading changed (was \(BDText.quoted(change.oldHeading ?? "")))") }
                if change.textChanged { parts.append("text changed") }
                if change.moved { parts.append("moved from Scene \(change.oldNumber ?? "")") }
                let joined = parts.joined(separator: " \u{00B7} ")
                return joined.prefix(1).uppercased() + joined.dropFirst()
            }
        }
    }
}

// MARK: Catalog picker

/// "Use existing: …" rows plus "Create new …" (mocks 108 and 109).
struct CatalogChoiceList: View {
    let name: String
    @Bindable var finder: CatalogMatchFinder
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if !name.trimmingCharacters(in: .whitespaces).isEmpty {
            let createNew = BreakdownCatalogChoice(mode: .new, catalogItemId: nil)
            VStack(spacing: 0) {
                ForEach(finder.matches, id: \.id) { m in
                    let choice = BreakdownCatalogChoice(mode: .existing, catalogItemId: m.id)
                    row(selected: sameChoice(finder.effective, choice), id: "use-existing-\(m.name)") { finder.choice = choice } label: {
                        VStack(alignment: .leading, spacing: 1) {
                            Text("Use existing: \(m.name)").font(.callout.weight(.semibold))
                            Text("\(BDCategory.short(m.category)) \u{00B7} \(m.status.rawValue) \u{00B7} \(m.usedInLabel == "Not used yet" ? "not used yet" : "used in \(BDText.usedIn(m.usedInLabel))")")
                                .font(.caption).foregroundStyle(theme["muted"])
                        }
                        Spacer()
                        BDChip(text: m.exact ? "Same item" : "Similar", tone: m.exact ? .green : .yellow)
                    }
                    Divider()
                }
                row(selected: sameChoice(finder.effective, createNew), id: "create-new") { finder.choice = createNew } label: {
                    VStack(alignment: .leading, spacing: 1) {
                        Text("Create new item \(BDText.quoted(name.trimmingCharacters(in: .whitespaces)))").font(.callout.weight(.semibold))
                        Text("Added to the production catalog").font(.caption).foregroundStyle(theme["muted"])
                    }
                    Spacer()
                }
                if finder.loading {
                    Divider()
                    Text("Looking for existing catalog items\u{2026}").font(.caption).foregroundStyle(theme["muted"])
                        .frame(maxWidth: .infinity, alignment: .leading).padding(8)
                        .accessibilityIdentifier("looking-for-matches")
                }
                if finder.needsChoice {
                    Divider()
                    Label("Several catalog items look similar. Choose one, or create a new item.", systemImage: "exclamationmark.triangle")
                        .font(.caption).foregroundStyle(theme["yellow-t"])
                        .frame(maxWidth: .infinity, alignment: .leading).padding(8)
                }
            }
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Catalog item")
        }
    }

    private func row<L: View>(selected: Bool, id: String, action: @escaping () -> Void, @ViewBuilder label: () -> L) -> some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: selected ? "largecircle.fill.circle" : "circle").foregroundStyle(selected ? theme["accent"] : theme["muted"]).accessibilityHidden(true)
                label()
            }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
        .accessibilityIdentifier(id)
    }
}

// MARK: Add Element

struct AddElementSheet: View {
    @Bindable var form: AddElementForm
    let sceneNumber: String
    @FocusState private var nameFocused: Bool

    var body: some View {
        BDSheetFrame(
            title: form.initial.span != nil ? "Tag \(BDText.quoted(form.initial.name ?? ""))" : "Add Element",
            sub: "What does Scene \(sceneNumber) need? Pick an existing catalog item or create a new one.",
            width: 560
        ) {
            VStack(alignment: .leading, spacing: 10) {
                BDField(label: "Category") {
                    Picker("Category", selection: Binding(get: { form.category }, set: { form.setCategory($0) })) {
                        ForEach(BDCategory.all, id: \.self) { Text($0.rawValue).tag($0) }
                    }
                    .labelsHidden()
                    .accessibilityLabel("Category")
                    .accessibilityIdentifier("add-category")
                }
                BDField(label: "Name") {
                    TextField("e.g. Red Folder", text: Binding(get: { form.name }, set: { form.setName($0) }))
                        .textFieldStyle(.roundedBorder)
                        .focused($nameFocused)
                        .onSubmit { Task { await form.submit() } }
                        .accessibilityLabel("Name")
                        .accessibilityIdentifier("add-name")
                }
                CatalogChoiceList(name: form.name, finder: form.finder)
                BDField(label: "Notes (optional)") {
                    TextField("e.g. must look worn", text: $form.notes, axis: .vertical)
                        .lineLimit(2...4)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Notes")
                        .accessibilityIdentifier("add-notes")
                }
                if form.initial.span != nil {
                    Text("The screenplay text is not changed; the tag marks the selected words in this scene.").font(.caption).foregroundStyle(.secondary)
                }
            }
        } footer: {
            Button("Cancel") { form.model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Add to Scene \(sceneNumber)") { Task { await form.submit() } }
                .keyboardShortcut(.defaultAction)
                .disabled(!form.canSubmit)
                .accessibilityIdentifier("add-to-scene")
        }
        .onAppear { nameFocused = form.initial.name == nil }
    }
}

// MARK: Edit or choose a match

struct EditSuggestionSheet: View {
    @Bindable var form: EditSuggestionForm
    let sceneNumber: String

    var body: some View {
        let e = form.element
        BDSheetFrame(
            title: form.chooser ? "Possible existing item: \(e.displayName)" : "Edit suggestion: \(e.displayName)",
            sub: form.chooser
                ? "You are adding \(BDText.quoted(e.displayName)) (\(e.category.rawValue)) to Scene \(sceneNumber). The catalog already has something similar."
                : "Change the category, name or catalog item before accepting.",
            width: 560
        ) {
            VStack(alignment: .leading, spacing: 10) {
                if let ev = e.evidence { Text("matched: \(BDText.quoted(ev))").font(.callout).foregroundStyle(.secondary) }
                BDField(label: "Category") {
                    Picker("Category", selection: Binding(get: { form.category }, set: { form.setCategory($0) })) {
                        ForEach(BDCategory.all, id: \.self) { Text($0.rawValue).tag($0) }
                    }
                    .labelsHidden()
                    .accessibilityLabel("Category")
                }
                BDField(label: "Name") {
                    TextField("Name", text: Binding(get: { form.name }, set: { form.setName($0) }))
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Name")
                }
                CatalogChoiceList(name: form.name, finder: form.finder)
                Text("If several items look similar you will see a list with distinguishing details and can choose one.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        } footer: {
            Button("Cancel") { form.model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button(form.finder.effective?.mode == .existing ? "Use Existing" : "Accept") { Task { await form.submit() } }
                .keyboardShortcut(.defaultAction)
                .disabled(!form.canSubmit)
                .accessibilityIdentifier("accept-edited")
        }
    }
}

// MARK: Review all suggestions

struct SuggestionReviewSheet: View {
    @Bindable var model: BreakdownModel
    let detail: BreakdownSceneDetail
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private func summary(_ list: [BreakdownElementDto]) -> String {
        var by: [(BreakdownCategory, Int)] = []
        for s in list {
            if let i = by.firstIndex(where: { $0.0 == s.category }) { by[i].1 += 1 } else { by.append((s.category, 1)) }
        }
        return by.map { "\($0.1) \($0.0.rawValue)" }.joined(separator: ", ")
    }

    private func matchNote(_ s: BreakdownElementDto) -> (String, BDTone) {
        switch s.matchKind {
        case "exact": ("Existing: \(s.matches.first?.name ?? "")", .green)
        case "ambiguous": ("Similar items, choose one", .yellow)
        default: ("New catalog item", .plain)
        }
    }

    var body: some View {
        let pending = model.pendingSuggestions
        let selected = model.selectedSuggestions
        let safe = model.safeSuggestions
        BDSheetFrame(title: "Suggested elements: Scene \(detail.number ?? "")", sub: "Grouped by category, with the text that triggered each suggestion.", width: 680) {
            BDBanner(.warn, "Suggestions are not production data yet. Accept, edit or reject each one.")
            if pending.isEmpty {
                Text("No pending suggestions for this scene. You can still tag text in the script or use Add Element.")
                    .font(.callout).foregroundStyle(.secondary).padding(.vertical, 10)
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 10) {
                        ForEach(detail.groups.filter { !$0.suggestions.isEmpty }, id: \.category) { g in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    CategoryMark(look: hub.look(g.category)).font(.callout.weight(.semibold))
                                    Spacer()
                                    Text("\(g.suggestions.count)").font(.caption).foregroundStyle(theme["muted"])
                                }
                                ForEach(g.suggestions, id: \.id) { s in row(s) }
                            }
                        }
                        Text("Existing catalog matches are suggested where obvious; ambiguous matches open a chooser.").font(.caption).foregroundStyle(.secondary)
                    }
                }
                .frame(maxHeight: 340)
            }
        } footer: {
            Button("Reject Selected (\(selected.count))") { Task { await model.rejectSelected() } }
                .disabled(model.busy || selected.isEmpty)
                .accessibilityIdentifier("reject-selected")
            Button("Accept Selected (\(selected.count))") { Task { await model.acceptSelected() } }
                .disabled(model.busy || selected.isEmpty)
                .help(summary(selected))
                .accessibilityIdentifier("accept-selected")
            Button("Accept All Safe (\(safe.count))") { Task { await model.acceptAllSafe() } }
                .keyboardShortcut(.defaultAction)
                .disabled(model.busy || safe.isEmpty)
                .help(summary(safe))
                .accessibilityIdentifier("accept-all-safe")
            Button("Close") { model.sheet = nil }.keyboardShortcut(.cancelAction)
        } leading: {
            Text("\(pending.count) suggested \u{00B7} \(selected.count) selected").font(.callout).foregroundStyle(.secondary)
        }
    }

    private func row(_ s: BreakdownElementDto) -> some View {
        let note = matchNote(s)
        return HStack(alignment: .top, spacing: 8) {
            Toggle(isOn: Binding(get: { !model.reviewDeselected.contains(s.id) }, set: { _ in model.toggleReview(s.id) })) {
                Text("Select \(s.displayName)")
            }
            .labelsHidden()
            .toggleStyle(.checkbox)
            .accessibilityLabel("Select \(s.displayName)")
            VStack(alignment: .leading, spacing: 1) {
                Text(s.displayName).font(.callout.weight(.semibold))
                Text("matched: \(BDText.quoted(s.evidence ?? "")) \u{00B7} \(s.confidence == "Likely" ? "Likely match" : "Possible match")")
                    .font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer(minLength: 4)
            BDChip(text: note.0, tone: note.1)
            Button("Accept") { Task { await model.accept(s) } }.accessibilityLabel("Accept \(s.name)")
            Button("Edit") { model.sheet = .edit(s, chooser: false) }.accessibilityLabel("Edit \(s.name)")
            Button("Reject") { Task { await model.reject([s.id]) } }.accessibilityLabel("Reject \(s.name)")
        }
        .controlSize(.small)
        .padding(.horizontal, 8).padding(.vertical, 5)
        .background(theme["suggest-bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
    }
}

// MARK: Scene changed

struct SceneChangesSheet: View {
    @Bindable var model: BreakdownModel
    let sceneId: String

    var body: some View {
        let d = model.sceneChanges
        BDSheetFrame(
            title: d.map { "Scene \($0.number ?? "") changed. Review breakdown differences." } ?? "Scene changed",
            sub: "The screenplay changed after this scene was broken down. Your production data has not been changed.",
            width: 600
        ) {
            if let d {
                if d.headingChanged {
                    BDBanner(.warn, "Heading changed from \(BDText.quoted(d.baselineHeading ?? "")) to \(BDText.quoted(d.heading)). The location/time description changed; existing associations stay until you review them.")
                }
                VStack(spacing: 0) {
                    ForEach(Array(d.removedCandidates.enumerated()), id: \.offset) { _, c in candidate(c, "Removed candidate", .red) }
                    ForEach(Array(d.addedCandidates.enumerated()), id: \.offset) { _, c in candidate(c, "Added candidate", .green) }
                    if d.removedCandidates.isEmpty && d.addedCandidates.isEmpty {
                        Text("No new or removed likely elements were found in the changed text.").font(.callout).foregroundStyle(.secondary).padding(8)
                    }
                }
                .background(.quaternary.opacity(0.3), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                Text("Impact").font(.callout.weight(.semibold))
                VStack(alignment: .leading, spacing: 3) {
                    Text("\u{2022} New elements suggested: \(d.newSuggestions.asInt)")
                    Text("\u{2022} Existing elements requiring review: \(d.elementsToReview.count)" + (d.elementsToReview.isEmpty ? "" : " (\(d.elementsToReview.map(\.name).joined(separator: ", ")))"))
                    ForEach(d.planningNotes, id: \.self) { Text("\u{2022} \($0)") }
                }
                .font(.callout)
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 80)
            }
        } footer: {
            Button("Review Manually") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Keep Production Data") { Task { await model.keepProductionData(sceneId: sceneId) } }
                .accessibilityIdentifier("keep-production-data")
            Button("Apply Suggested Update") { Task { await model.applySuggestedUpdate(sceneId: sceneId) } }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("apply-suggested-update")
        }
        .task { await model.loadChanges(sceneId: sceneId) }
    }

    private func candidate(_ c: BreakdownCandidateDto, _ label: String, _ tone: BDTone) -> some View {
        HStack(spacing: 8) {
            BDChip(text: label, tone: tone)
            (Text(c.name).bold() + Text(" (\(BDCategory.short(c.category))): \(BDText.quoted(c.evidence))").foregroundStyle(.secondary)).font(.callout)
            Spacer()
        }
        .padding(8)
        .accessibilityElement(children: .combine)
    }
}

// MARK: Notes

struct ElementNotesSheet: View {
    @Bindable var model: BreakdownModel
    let element: BreakdownElementDto
    @State private var notes: String

    init(model: BreakdownModel, element: BreakdownElementDto) {
        self.model = model
        self.element = element
        _notes = State(initialValue: element.notes ?? "")
    }

    var body: some View {
        BDSheetFrame(title: "Notes: \(element.name)", sub: "Scene-specific notes for this element. The catalog item and the screenplay are not changed.") {
            BDField(label: "Notes") {
                TextField("e.g. must look worn", text: $notes, axis: .vertical)
                    .lineLimit(4...10)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Notes")
                    .accessibilityIdentifier("element-notes")
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Save") { Task { await model.saveNotes(element, notes: notes) } }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("save-notes")
        }
    }
}

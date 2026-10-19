// The call sheet editor screen: banners for a stale or replaced sheet, the paper, the Source panel
// on the right (status, Add Section, Refresh, Finalize, Export, revisions) and the Refresh and
// Finalize sheets (Windows CallSheetEditor.tsx).

import SwiftUI
import OpenFrameCore

struct CallSheetEditorView: View {
    @Bindable var model: CallSheetEditorModel
    @Environment(AppModel.self) private var app
    @Environment(CallSheetsHub.self) private var hub
    @Environment(\.ofTheme) private var theme
    @State private var confirmDelete = false

    var body: some View {
        Group {
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let d = model.detail, model.doc != nil {
                loadedBody(d)
            } else {
                VStack(alignment: .leading, spacing: 10) {
                    CSBanner(tone: .err, text: model.failure?.message ?? "This call sheet could not be opened.") {
                        Button("Back to Call Sheets") { Task { await hub.closeEditor() } }
                            .controlSize(.small)
                            .accessibilityIdentifier("callsheet-back-error")
                    }
                    Spacer()
                }
                .padding(16)
                .accessibilityIdentifier("callsheet-editor-error")
            }
        }
        .task(id: app.changes.key(CSTables.callSheet)) { await model.reload() }
        .sheet(isPresented: Binding(get: { model.dialog == .refresh }, set: { if !$0 { model.dialog = nil } })) {
            CSRefreshSheet(model: model)
        }
        .sheet(isPresented: Binding(get: { model.dialog == .finalize }, set: { if !$0 { model.dialog = nil } })) {
            if let d = model.detail { CSFinalizeSheet(model: model, detail: d) }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("callsheet-editor")
    }

    private func loadedBody(_ d: CallSheetDetail) -> some View {
        VStack(spacing: PageSpacing.small) {
            editorHeader(d)
            banners(d)
            HStack(alignment: .top, spacing: PageSpacing.medium) {
                ScrollView([.vertical, .horizontal]) {
                    CallSheetPaper(model: model).padding(.bottom, PageSpacing.large)
                }
                .frame(minWidth: 0, maxWidth: .infinity)
                sourcePanel(d)
            }
        }
        .padding(PageSpacing.medium)
    }

    /// The title in the middle with the Back button on the left. In a narrow window the Back button
    /// takes its own line above the title.
    private func editorHeader(_ d: CallSheetDetail) -> some View {
        let title = VStack(spacing: PageSpacing.tight) {
            HStack(spacing: 8) {
                Text(d.title).font(.title2.weight(.semibold)).accessibilityAddTraits(.isHeader)
                CSChip(text: model.word, tone: CSStatus.tone(d.status))
            }
            Text("What does everyone need to know for this day?").font(.callout).foregroundStyle(theme["muted"])
        }
        .multilineTextAlignment(.center)
        let back = HStack {
            Button { Task { await hub.closeEditor() } } label: { Label("Call Sheets", systemImage: "chevron.left") }
                .keyboardShortcut("[", modifiers: .command)
                .accessibilityIdentifier("callsheet-back")
            Spacer(minLength: 0)
        }
        return ViewThatFits(in: .horizontal) {
            ZStack {
                title.padding(.horizontal, 160)
                back
            }
            VStack(spacing: PageSpacing.small) {
                back
                title
            }
        }
    }

    // MARK: Banners

    @ViewBuilder private func banners(_ d: CallSheetDetail) -> some View {
        if let newer = d.supersededBy {
            CSBanner(tone: .info, text: "A newer revision replaced this one. This final version stays unchanged as a record.") {
                Button("Open newer revision") { hub.open(newer) }
                    .controlSize(.small)
                    .accessibilityIdentifier("callsheet-open-newer")
            }
        } else if let stale = d.stale {
            if stale.dayDeleted {
                CSBanner(.warn, "The shooting day of this call sheet was deleted. The call sheet stays readable and unchanged.")
                    .accessibilityIdentifier("callsheet-day-deleted")
            } else if d.editable {
                CSBanner(
                    tone: .warn,
                    text: "Needs refresh. The schedule changed after this call sheet was prepared (\(CSFormat.dateTime(stale.preparedAt))). Changed: \(stale.areas.isEmpty ? "day details" : stale.areas.joined(separator: ", ")). Nothing changes until you review the update."
                ) {
                    Button("Review Update") { Task { await model.showRefresh() } }
                        .buttonStyle(.borderedProminent).controlSize(.small)
                        .accessibilityIdentifier("callsheet-review-update")
                }
                .accessibilityIdentifier("callsheet-stale-banner")
            } else {
                CSBanner(
                    tone: .info,
                    text: "The schedule changed after this call sheet was finalized (\(stale.areas.isEmpty ? "day details" : stale.areas.joined(separator: ", "))). The final version never changes; start a new revision to update it."
                ) {
                    Button("Start New Revision") { Task { await model.showRefresh() } }
                        .controlSize(.small)
                        .accessibilityIdentifier("callsheet-start-new-revision")
                }
                .accessibilityIdentifier("callsheet-final-stale-banner")
            }
        }
    }

    // MARK: Source panel

    private func sourcePanel(_ d: CallSheetDetail) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text("Source").font(.headline)
                Button(d.dayTitle) {
                    Task {
                        guard await model.flush() else { return }
                        csShowWorkspace(.schedule)
                    }
                }
                .buttonStyle(.link)
                .accessibilityIdentifier("callsheet-source-day")
                HStack(spacing: 6) {
                    CSChip(text: model.word, tone: CSStatus.tone(d.status))
                    Text("v\(Int(d.revision))").font(.caption).foregroundStyle(theme["muted"])
                        .accessibilityIdentifier("callsheet-revision")
                    if model.saving { Text("Saving\u{2026}").font(.caption).foregroundStyle(theme["muted"]) }
                }
                if !d.missing.isEmpty, d.editable {
                    Text("\(d.missing.count) \(d.missing.count == 1 ? "value reads" : "values read") \(csNeedsInput).")
                        .font(.caption).foregroundStyle(theme["red-t"])
                        .accessibilityIdentifier("callsheet-missing-count")
                }
                Text("Scenes, times, cast and locations come from the schedule. Editable fields use dashed highlights. Editing this sheet never changes the screenplay, breakdown or schedule.")
                    .font(.footnote).foregroundStyle(theme["muted"])
                Divider()
                if d.editable { addSectionMenu(d) }
                if d.supersededBy == nil {
                    Button(d.editable ? "Refresh from schedule" : "Start New Revision") { Task { await model.showRefresh() } }
                        .disabled(d.stale?.dayDeleted == true)
                        .keyboardShortcut("r", modifiers: [.command, .option])
                        .accessibilityIdentifier("callsheet-refresh")
                }
                if d.editable {
                    Button("Finalize\u{2026}") { Task { await model.showFinalize() } }
                        .buttonStyle(.borderedProminent)
                        .disabled(!hub.canEdit)
                        .accessibilityIdentifier("callsheet-finalize")
                }
                Button { hub.exportCallSheet(id: d.id, title: d.title, status: d.status) } label: {
                    Label("Export PDF", systemImage: "square.and.arrow.down")
                }
                .accessibilityIdentifier("callsheet-export")
                if let f = d.finalizedAt {
                    Text("Finalized \(CSFormat.dateTime(f)).").font(.caption).foregroundStyle(theme["muted"])
                        .accessibilityIdentifier("callsheet-finalized-at")
                }
                if d.revisions.count > 1 {
                    Divider()
                    Text("Revisions").font(.headline)
                    ForEach(d.revisions, id: \.id) { r in
                        Button { Task { await model.openRevision(r.id) } } label: {
                            HStack {
                                Text("v\(Int(r.revision))")
                                Spacer()
                                CSChip(text: CSStatus.word(r.status), tone: CSStatus.tone(r.status))
                            }
                            .padding(.horizontal, 6).padding(.vertical, 3)
                            .background(r.id == d.id ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityIdentifier("callsheet-revision-\(Int(r.revision))")
                    }
                }
                Divider()
                Button("Delete call sheet", role: .destructive) { confirmDelete = true }
                    .disabled(!hub.canEdit)
                    .accessibilityIdentifier("callsheet-delete")
            }
            .controlSize(.regular)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(PageSpacing.medium)
        }
        .frame(width: 290)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
        .confirmationDialog("Delete this call sheet?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) { Task { _ = await model.delete() } }
                .accessibilityIdentifier("callsheet-delete-confirm")
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("You can undo this right after.")
        }
    }

    private func addSectionMenu(_ d: CallSheetDetail) -> some View {
        Menu("Add Section") {
            ForEach(d.optionalSections, id: \.key) { s in
                Button(s.label) { Task { await model.addSection(key: s.key, label: s.label) } }
                    .disabled(model.isShown(s.key))
                    .accessibilityIdentifier("callsheet-add-\(csSlug(s.key))")
            }
            Divider()
            Button("Custom field") { model.addCustomField() }
                .accessibilityIdentifier("callsheet-add-custom-field")
        }
        .accessibilityIdentifier("callsheet-add-section")
    }
}

// MARK: Refresh sheet

struct CSRefreshSheet: View {
    @Bindable var model: CallSheetEditorModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let p = model.preview
        CSSheetFrame(title: "Update call sheet: review changes", sub: "Nothing is applied until you click Apply Update.", width: 680) {
            content(p)
        } footer: {
            Button("Cancel") { model.dialog = nil }
                .keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("callsheet-refresh-cancel")
            Button(p?.createsRevision == true ? "Create New Revision" : "Apply Update") {
                Task { await model.applyRefresh() }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(model.busy || p == nil || p?.dayDeleted == true)
            .accessibilityIdentifier("callsheet-refresh-apply")
        }
        .task { await model.loadPreview() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("callsheet-refresh-sheet")
    }

    @ViewBuilder private func content(_ p: CallSheetRefreshPreview?) -> some View {
        if let f = model.previewFailure {
            CSBanner(.err, f.message)
        } else if let p {
            if p.dayDeleted {
                CSBanner(.warn, "The shooting day of this call sheet was deleted, so there is nothing to update from. The call sheet stays readable.")
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 8) {
                        if !p.hasChanges {
                            CSBanner(.ok, "This call sheet already matches the schedule.")
                        } else {
                            VStack(alignment: .leading, spacing: 2) {
                                ForEach(p.summary, id: \.self) { Text("\u{2022} \($0)").font(.callout) }
                                if !p.other.isEmpty { Text("\u{2022} Also changes: \(p.other.joined(separator: ", "))").font(.callout) }
                            }
                            .accessibilityIdentifier("callsheet-refresh-summary")
                        }
                        changeList("Scenes", p.scenes)
                        changeList("Cast calls", p.cast)
                        changeList("Locations", p.locations)
                        Grid(alignment: .topLeading, horizontalSpacing: 12, verticalSpacing: 4) {
                            GridRow {
                                Text("Area").fontWeight(.semibold)
                                Text("Now in call sheet").fontWeight(.semibold)
                                Text("After update").fontWeight(.semibold)
                            }
                            Divider().gridCellUnsizedAxes(.horizontal)
                            ForEach(p.rows, id: \.area) { r in
                                GridRow {
                                    Text(r.area + (r.changed ? " (changes)" : "")).fontWeight(.semibold)
                                    Text(r.now).textSelection(.enabled)
                                    Text(r.after).textSelection(.enabled)
                                }
                                .background(r.changed ? theme["yellow-soft"] : Color.clear)
                            }
                        }
                        .font(.callout)
                        Text("Your call times, notes, contacts and added sections are kept where they still apply."
                            + (p.createsRevision ? " This call sheet is final, so the update becomes a new revision and the final version stays unchanged." : ""))
                            .font(.footnote).foregroundStyle(theme["muted"])
                    }
                }
                .frame(maxHeight: 380)
            }
        } else {
            ProgressView().frame(maxWidth: .infinity, minHeight: 80)
        }
    }

    @ViewBuilder private func changeList(_ title: String, _ c: CallSheetChanges) -> some View {
        let text = Self.changeText(c)
        if !text.isEmpty {
            (Text("\(title): ").fontWeight(.semibold) + Text(text)).font(.callout)
        }
    }

    static func changeText(_ c: CallSheetChanges) -> String {
        var parts: [String] = []
        if !c.added.isEmpty { parts.append("added \(c.added.joined(separator: ", ")).") }
        if !c.removed.isEmpty { parts.append("removed \(c.removed.joined(separator: ", ")).") }
        if !c.changed.isEmpty { parts.append("changed \(c.changed.joined(separator: ", ")).") }
        return parts.joined(separator: " ")
    }
}

// MARK: Finalize sheet

struct CSFinalizeSheet: View {
    @Bindable var model: CallSheetEditorModel
    let detail: CallSheetDetail
    @State private var stale = false
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let doc = model.doc ?? detail.document
        let scenes = doc.scenes.filter { $0.kind.isEmpty }.count
        CSSheetFrame(title: "Finalize call sheet") {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 6) {
                    CSChip(text: CSStatus.word(detail.status), tone: CSStatus.tone(detail.status))
                    Text("Is this the version you are sending?").fontWeight(.semibold)
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(doc.title) \u{00B7} \(doc.dayLabel) \u{00B7} \(detail.dateLong ?? "No date")").fontWeight(.semibold)
                    Text("Crew call \(doc.crewCall.isEmpty ? csNeedsInput : doc.crewCall) \u{00B7} \(scenes) \(scenes == 1 ? "scene" : "scenes") \u{00B7} \(doc.cast.count) cast")
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
                .accessibilityIdentifier("callsheet-finalize-summary")
                Text("Final freezes this document as a snapshot. Later schedule changes mark it for a new revision but never change the final version.")
                    .font(.callout)
                if !detail.missing.isEmpty {
                    Text("\(detail.missing.count) \(detail.missing.count == 1 ? "value still reads" : "values still read") \(csNeedsInput).")
                        .font(.footnote).foregroundStyle(theme["muted"])
                }
                if stale {
                    CSBanner(.warn, "The schedule changed after this call sheet was prepared. Refresh it first, or finalize this version anyway.")
                        .accessibilityIdentifier("callsheet-finalize-stale")
                }
            }
        } footer: {
            Button("Cancel") { model.dialog = nil }
                .keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("callsheet-finalize-cancel")
            if stale {
                Button("Finalize this version anyway") { Task { _ = await model.finalize(acknowledgeStale: true) } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(model.busy)
                    .accessibilityIdentifier("callsheet-finalize-anyway")
            } else {
                Button("Finalize") {
                    Task {
                        if await model.finalize(acknowledgeStale: false) == .stale { stale = true }
                    }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(model.busy)
                .accessibilityIdentifier("callsheet-finalize-confirm")
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("callsheet-finalize-sheet")
    }
}

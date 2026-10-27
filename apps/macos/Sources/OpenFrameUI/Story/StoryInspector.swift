// The inspector for the selected card, beat or sequence (the drawer on Windows). Simple edits
// save by themselves; closing returns to exactly the same place on the board.

import SwiftUI
import OpenFrameCore

struct StoryInspector: View {
    @Bindable var model: StoryModel
    let ref: StoryItemRef

    var body: some View {
        Group {
            switch ref.kind {
            case .card: CardInspector(model: model, id: ref.id)
            case .beat: BeatInspector(model: model, id: ref.id)
            case .sequence: SequenceInspector(model: model, id: ref.id)
            }
        }
        .id(ref.key)
        .onExitCommand { model.openDrawer(nil) }
    }
}

/// Header and frame shared by the three inspectors.
private struct InspectorFrame<Content: View, Footer: View>: View {
    let typeLabel: String
    let title: String
    let onClose: () -> Void
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(typeLabel.uppercased()).font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
                    Text(title).font(.headline).lineLimit(2)
                }
                Spacer()
                Button("Close", action: onClose)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("story-inspector-close")
            }
            .padding(14)
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 14) { content() }
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            Divider()
            footer().padding(10)
        }
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel(typeLabel)
    }
}

// MARK: Scene card

private struct CardInspector: View {
    @Bindable var model: StoryModel
    @State private var d: CardDrawerModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    init(model: StoryModel, id: String) {
        self.model = model
        _d = State(initialValue: CardDrawerModel(story: model, cardId: id))
    }

    var body: some View {
        Group {
            if d.missing {
                InspectorFrame(typeLabel: "Scene Card", title: "Scene Card", onClose: close) {
                    Text("This Scene Card is no longer on the Beat Board. It may have been deleted.").foregroundStyle(.secondary)
                } footer: { EmptyView() }
            } else if let detail = d.detail, let f = d.fields {
                let c = detail.card
                InspectorFrame(typeLabel: "Scene Card", title: d.title, onClose: close) {
                    Text(detail.location).font(.caption).foregroundStyle(theme["muted"])
                    SavedField(label: "Scene heading (optional now, required before it becomes a screenplay scene)", saver: f.heading,
                               placeholder: "EXT. OLD RAILWAY STATION - NIGHT", multiline: false)
                    SavedField(label: "Short description", saver: f.short, rows: 4, placeholder: "What happens in this scene?")
                    SavedField(label: "Long description", saver: f.long, rows: 8,
                               placeholder: "Develop the scene: what changes, what each character wants, images and sounds you already see.",
                               hint: "For scene-development material. Cards can show its first lines (Card Display). Up to \(CardDrawerModel.longDescriptionMax.formatted()) characters.",
                               limit: CardDrawerModel.longDescriptionMax)
                    SavedField(label: "Notes", saver: f.notes, rows: 4, placeholder: "Freeform scene notes")
                    LabelColorRow(current: c.color) { name in
                        Task { await setColor(name) }
                    }
                    StoryAttachmentsField(story: model, items: c.attachments, ownerType: "scene_card", ownerId: c.id)
                    if c.commentCount > 0 {
                        Text("Comments (\(Int(c.commentCount)) open)").font(.caption).foregroundStyle(theme["muted"])
                    }
                    if let s = detail.linkedScene {
                        VStack(alignment: .leading, spacing: 8) {
                            StoryBanner(tone: .info, text: "Used to create screenplay Scene \(Int(s.number)): \(s.heading) in \u{201C}\(s.draftName)\u{201D}. Changes here don't change the screenplay.")
                            Button("Open Screenplay") { openScreenplay(s) }.controlSize(.small)
                        }
                    }
                } footer: {
                    VStack(alignment: .leading, spacing: 8) {
                        StorySavedStatus(status: d.status)
                        FlowLayout(spacing: 6) {
                            Button("Duplicate") { Task { await model.duplicate([d.ref]) } }
                            if d.parked {
                                Button("Restore to Story") { Task { await model.restoreToStory([d.ref]) } }
                            } else {
                                Button("Send to Parking Lot") { Task { await model.park([d.ref]) } }
                            }
                            Button("Convert to Screenplay Scene") { model.sheet = .build(preselect: [c.id]) }.disabled(d.parked)
                            Button("Delete", role: .destructive) { Task { await model.requestDelete([d.ref]) } }
                        }
                        .controlSize(.small)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task(id: app.changes.key(StoryTables.board + StoryTables.screenplay)) { await d.reload() }
    }

    private func close() {
        Task {
            await d.flushAll()
            model.openDrawer(nil)
        }
    }

    private func setColor(_ name: String) async {
        guard let core = app.core else { return }
        d.status.set(.saving)
        do {
            _ = try await core.storyUpdateCard(StoryUpdateCardArgs(id: d.cardId, color: name))
            d.status.set(.saved)
        } catch {
            d.status.set(.error)
            model.fail("Could not save the scene card", error)
        }
    }

    private func openScreenplay(_ s: StoryLinkedScene) {
        NotificationCenter.default.post(
            name: .openFrameShowScreenplayTarget, object: nil,
            userInfo: ["screenplayId": s.screenplayId, "draftId": s.draftId, "sceneId": s.sceneId])
        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.screenplay.rawValue)
    }
}

/// The nine label colours as a row of swatches (None clears it).
private struct LabelColorRow: View {
    let current: String?
    let pick: (String) -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Colour (optional)").font(.callout.weight(.medium))
            HStack(spacing: 6) {
                Button("None") { pick("") }
                    .controlSize(.small)
                    .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(current == nil ? theme["ink"] : .clear, lineWidth: 2))
                    .accessibilityAddTraits(current == nil ? .isSelected : [])
                ForEach(StoryLogic.colorNames, id: \.self) { name in
                    Button { pick(name) } label: {
                        Circle().fill(theme.label(name)).frame(width: 18, height: 18)
                            .overlay(Circle().strokeBorder(current == name ? theme["ink"] : .clear, lineWidth: 2).padding(-3))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(name)
                    .accessibilityAddTraits(current == name ? .isSelected : [])
                }
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Colour")
        }
    }
}

// MARK: Beat

private struct BeatInspector: View {
    @Bindable var model: StoryModel
    @State private var d: BeatDrawerModel
    @Environment(\.ofTheme) private var theme

    init(model: StoryModel, id: String) {
        self.model = model
        _d = State(initialValue: BeatDrawerModel(story: model, beatId: id))
    }

    var body: some View {
        Group {
            if let b = d.beat {
                let ref = StoryItemRef(kind: .beat, id: b.id)
                let parked = d.container?.parentType == .parking
                let converted = b.state == "converted"
                InspectorFrame(typeLabel: "Beat Card", title: b.text.trimmingCharacters(in: .whitespaces).isEmpty ? "Beat" : b.text, onClose: close) {
                    if let c = d.container, let board = model.board {
                        Text(StoryLogic.containerLabel(board, c)).font(.caption).foregroundStyle(theme["muted"])
                    }
                    if converted {
                        VStack(alignment: .leading, spacing: 8) {
                            StoryBanner(tone: .ok, text: "Converted to a Scene Card. This beat stays as a reference.")
                            if let id = b.convertedSceneCardId {
                                Button("Open card") { model.openDrawer(StoryItemRef(kind: .card, id: id)) }.controlSize(.small)
                            }
                        }
                    }
                    SavedField(label: "Beat text", saver: d.text, placeholder: "")
                    SavedField(label: "Note (optional)", saver: d.note, placeholder: "Add a note\u{2026}")
                    LabelColorRow(current: b.color) { name in Task { await d.setColor(name) } }
                    StoryAttachmentsField(story: model, items: b.attachments, ownerType: "beat", ownerId: b.id)
                    Text("Convert to Scene creates a new Scene Card from this text. The beat stays behind marked Converted so no thought is lost.")
                        .font(.caption).foregroundStyle(theme["muted"])
                } footer: {
                    VStack(alignment: .leading, spacing: 8) {
                        StorySavedStatus(status: d.status)
                        FlowLayout(spacing: 6) {
                            Button("Convert to Scene") { Task { await model.convertBeat(b.id) } }
                                .buttonStyle(.ofProminent).disabled(converted)
                            Button("Duplicate") { Task { await model.duplicate([ref]) } }
                            if parked {
                                Button("Restore") { Task { await model.restoreToStory([ref]) } }
                            } else {
                                Button("Park") { Task { await model.park([ref]) } }
                            }
                            Button("Delete", role: .destructive) { Task { await model.requestDelete([ref]) } }
                        }
                        .controlSize(.small)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                InspectorFrame(typeLabel: "Beat Card", title: "Beat", onClose: { model.openDrawer(nil) }) {
                    Text("This beat is no longer on the Beat Board. It may have been deleted.").foregroundStyle(.secondary)
                } footer: { EmptyView() }
            }
        }
        .onAppear { d.sync() }
        .onChange(of: d.beat) { _, _ in d.sync() }
    }

    private func close() {
        Task {
            await d.flushAll()
            model.openDrawer(nil)
        }
    }
}

// MARK: Sequence

private struct SequenceInspector: View {
    @Bindable var model: StoryModel
    @State private var d: SequenceDrawerModel
    @Environment(\.ofTheme) private var theme

    init(model: StoryModel, id: String) {
        self.model = model
        _d = State(initialValue: SequenceDrawerModel(story: model, sequenceId: id))
    }

    var body: some View {
        Group {
            if let s = d.sequence {
                let cards: [StorySceneCardDto] = s.items.compactMap { if case let .card(c) = $0 { c } else { nil } }
                InspectorFrame(typeLabel: "Sequence", title: s.title, onClose: close) {
                    Text(d.actTitle).font(.caption).foregroundStyle(theme["muted"])
                    SavedField(label: "Sequence name", saver: d.title, multiline: false)
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Scenes (\(cards.count))").font(.callout.weight(.medium))
                        ForEach(cards, id: \.id) { c in
                            Button {
                                model.select([StoryItemRef(kind: .card, id: c.id).key])
                                model.openDrawer(StoryItemRef(kind: .card, id: c.id))
                            } label: {
                                Text(c.shortDescription.isEmpty ? (c.sceneHeading ?? "Blank scene card") : c.shortDescription)
                                    .lineLimit(2).frame(maxWidth: .infinity, alignment: .leading)
                                    .padding(8)
                                    .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
                            }
                            .buttonStyle(.plain)
                        }
                        if cards.isEmpty { Text("Drop scenes here from the board.").foregroundStyle(theme["muted"]) }
                    }
                    SavedField(label: "Note", saver: d.note)
                    StoryAttachmentsField(story: model, items: s.attachments, ownerType: "sequence", ownerId: s.id)
                } footer: {
                    HStack {
                        StorySavedStatus(status: d.status)
                        Spacer()
                        Button("Delete", role: .destructive) { Task { await model.requestDeleteSequence(s.id) } }.controlSize(.small)
                    }
                }
            } else {
                InspectorFrame(typeLabel: "Sequence", title: "Sequence", onClose: { model.openDrawer(nil) }) {
                    Text("This sequence is no longer on the Beat Board. It may have been deleted.").foregroundStyle(.secondary)
                } footer: { EmptyView() }
            }
        }
        .onAppear { d.sync() }
        .onChange(of: d.sequence) { _, _ in d.sync() }
    }

    private func close() {
        Task {
            await d.flushAll()
            model.openDrawer(nil)
        }
    }
}

// The decision sheets of the Beat Board: deleting an act or a sequence recommends moving the
// creative material out first; deleting a card linked to a screenplay scene explains that the
// scene stays; Move; the act note; Card Display Settings; Apply Board Order.

import SwiftUI
import OpenFrameCore

struct StorySheetHost: View {
    @Bindable var model: StoryModel
    let sheet: StorySheet

    var body: some View {
        switch sheet {
        case let .deleteAct(id): DeleteActSheet(model: model, actId: id)
        case let .deleteSequence(id): DeleteSequenceSheet(model: model, sequenceId: id)
        case let .deleteLinked(refs): DeleteLinkedSheet(model: model, refs: refs)
        case let .move(refs): MoveSheet(model: model, refs: refs)
        case let .actNote(id): ActNoteSheet(model: model, actId: id)
        case let .build(preselect): BuildScreenplaySheet(model: model, preselect: preselect)
        case .applyOrder: ApplyOrderSheet(model: model)
        case .cardDisplay: CardDisplaySheet(model: model)
        }
    }
}

/// One choice of a radio group, with a line of explanation.
struct StoryChoice: View {
    let title: String
    var tag: String?
    var detail: String?
    let selected: Bool
    var disabled = false
    let action: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Button(action: action) {
            HStack(alignment: .top, spacing: 8) {
                Image(systemName: selected ? "largecircle.fill.circle" : "circle")
                    .foregroundStyle(selected ? theme["accent"] : theme["muted"])
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(title).fontWeight(.semibold)
                        if let tag { Text(tag).font(.caption2.weight(.semibold)).padding(.horizontal, 6).padding(.vertical, 1).background(theme["green-soft"], in: Capsule()).foregroundStyle(theme["green-t"]) }
                    }
                    if let detail { Text(detail).font(.callout).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true) }
                }
                Spacer(minLength: 0)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(disabled)
        .opacity(disabled ? 0.55 : 1)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }
}

// MARK: Delete act

struct DeleteActSheet: View {
    @Bindable var model: StoryModel
    let actId: String
    @State private var mode: StoryDeleteMode = .moveContents
    @State private var dest = ""
    @State private var busy = false

    var body: some View {
        if let act = model.board?.acts.first(where: { $0.id == actId }) {
            let others = (model.board?.acts ?? []).filter { $0.id != actId }
            let seqs = act.items.filter { $0.kind == .sequence }.count
            let beats = act.items.reduce(0) { n, i in
                switch i {
                case .beat: n + 1
                case let .sequence(s): n + s.items.filter { $0.kind == .beat }.count
                case .card: n
                }
            }
            StorySheetFrame(title: "Delete \u{201C}\(act.title)\u{201D}?") {
                VStack(alignment: .leading, spacing: 10) {
                    Text("This Act contains \(StoryLogic.plural(seqs, "Sequence")) and \(StoryLogic.plural(Int(act.cardCount), "card"))\(beats > 0 ? " and \(StoryLogic.plural(beats, "beat"))" : ""). Your creative material is not deleted unless you choose that.")
                    StoryChoice(title: "Move its contents to another Act", tag: "Recommended", detail: "Then delete only the empty Act.", selected: mode == .moveContents) { mode = .moveContents }
                    if mode == .moveContents {
                        Picker("Move contents to", selection: $dest) {
                            ForEach(others, id: \.id) { Text($0.title).tag($0.id) }
                            Text("Unassigned (keep aside)").tag("unassigned")
                        }
                        .padding(.leading, 26)
                        .accessibilityIdentifier("story-delete-act-destination")
                    }
                    StoryChoice(title: "Delete the Act and everything inside it", detail: "The cards go to Recently Deleted and can be restored.", selected: mode == .deleteAll) { mode = .deleteAll }
                }
            } footer: {
                Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
                Button(mode == .moveContents ? "Move Contents and Delete Act" : "Delete Act and Contents", role: mode == .deleteAll ? .destructive : nil) {
                    busy = true
                    Task {
                        let ok = await model.deleteAct(actId, mode: mode, moveToActId: dest == "unassigned" || dest.isEmpty ? nil : dest)
                        busy = false
                        if ok { model.sheet = nil }
                    }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(busy)
            }
            .onAppear { if dest.isEmpty { dest = others.first?.id ?? "unassigned" } }
        }
    }
}

// MARK: Delete sequence

struct DeleteSequenceSheet: View {
    @Bindable var model: StoryModel
    let sequenceId: String
    @State private var mode: StoryDeleteMode = .moveContents
    @State private var dest = "here"
    @State private var busy = false

    var body: some View {
        if let board = model.board, let seq = StoryLogic.sequences(board).first(where: { $0.id == sequenceId }) {
            let actTitle = seq.actId.flatMap { id in board.acts.first { $0.id == id }?.title } ?? "Unassigned"
            StorySheetFrame(title: "Delete \u{201C}\(seq.title)\u{201D}?") {
                VStack(alignment: .leading, spacing: 10) {
                    Text("This Sequence contains \(StoryLogic.plural(seq.items.count, "item")). Your creative material is not deleted unless you choose that.")
                    StoryChoice(title: "Move its cards out first", tag: "Recommended", detail: "Then delete only the empty Sequence.", selected: mode == .moveContents) { mode = .moveContents }
                    if mode == .moveContents {
                        Picker("Move cards to", selection: $dest) {
                            Text("\(actTitle) (where the sequence is now)").tag("here")
                            ForEach(board.acts.filter { $0.id != seq.actId }, id: \.id) { Text($0.title).tag("act:\($0.id)") }
                            ForEach(StoryLogic.sequences(board).filter { $0.id != sequenceId }, id: \.id) { Text("Sequence: \($0.title)").tag("sequence:\($0.id)") }
                        }
                        .padding(.leading, 26)
                    }
                    StoryChoice(title: "Delete the Sequence and everything inside it", detail: "The cards go to Recently Deleted and can be restored.", selected: mode == .deleteAll) { mode = .deleteAll }
                }
            } footer: {
                Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
                Button(mode == .moveContents ? "Move Cards and Delete Sequence" : "Delete Sequence and Cards", role: mode == .deleteAll ? .destructive : nil) {
                    busy = true
                    Task {
                        var moveTo: StoryContainerRef?
                        if dest != "here", let colon = dest.firstIndex(of: ":") {
                            let kind = String(dest[..<colon])
                            moveTo = StoryContainerRef(parentType: kind == "act" ? .act : .sequence, parentId: String(dest[dest.index(after: colon)...]))
                        }
                        let ok = await model.deleteSequence(sequenceId, mode: mode, moveTo: moveTo)
                        busy = false
                        if ok { model.sheet = nil }
                    }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(busy)
            }
        }
    }
}

// MARK: Delete linked cards

/// Deleting cards linked to screenplay scenes: the scenes stay.
struct DeleteLinkedSheet: View {
    @Bindable var model: StoryModel
    let refs: [StoryItemRef]
    @State private var scene: StoryLinkedScene?

    static func title(count: Int) -> String { count == 1 ? "Delete this Scene Card?" : "Delete \(count) items?" }

    static func message(count: Int, scene: StoryLinkedScene?) -> String {
        let single = count == 1
        return "\(single ? "Its linked screenplay scene" : "Linked screenplay scenes")\(scene.map { " (Scene \(Int($0.number)): \($0.heading))" } ?? "") will remain. Screenplay scenes are never deleted from the Beat Board."
    }

    var body: some View {
        let single = refs.count == 1
        StorySheetFrame(title: Self.title(count: refs.count)) {
            VStack(alignment: .leading, spacing: 8) {
                Text(Self.message(count: refs.count, scene: scene))
                Text("\(single ? "The card moves" : "They move") to Recently Deleted. You can restore \(single ? "it" : "them") or use Undo.")
                    .foregroundStyle(.secondary)
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button(single ? "Delete Card" : "Delete", role: .destructive) {
                model.sheet = nil
                Task { await model.deleteNow(refs) }
            }
            .keyboardShortcut(.defaultAction)
            .accessibilityIdentifier("story-confirm-delete")
        }
        .task {
            guard let board = model.board, let core = model.app.core else { return }
            let linked = refs.first { r in
                if let loc = StoryLogic.locate(board, r), case let .card(c) = loc.item { return c.screenplaySceneId != nil }
                return false
            }
            if let linked { scene = try? await core.storyCard(StoryIdArgs(id: linked.id)).linkedScene }
        }
    }
}

// MARK: Move

struct MoveSheet: View {
    @Bindable var model: StoryModel
    let refs: [StoryItemRef]
    @State private var dest = ""

    var body: some View {
        let dests = model.board.map { StoryLogic.moveDestinations($0, kinds: Set(refs.map(\.kind))) } ?? []
        StorySheetFrame(title: refs.count == 1 ? "Move to\u{2026}" : "Move \(refs.count) items to\u{2026}", width: 400) {
            if dests.isEmpty {
                Text("Add an Act first.").foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 4) {
                    Picker("Destination", selection: $dest) {
                        ForEach(dests) { d in Text("\(d.depth > 0 ? "    \u{203A} " : "")\(d.label)").tag(d.id) }
                    }
                    Text("Items are added at the end and keep their order.").font(.caption).foregroundStyle(.secondary)
                }
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Move") {
                let chosen = dests.first { $0.id == dest }
                model.sheet = nil
                if let chosen { Task { await model.runMove(refs, to: StoryDropHint(target: chosen.container, before: nil)) } }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(!dests.contains { $0.id == dest })
        }
        .onAppear { if dest.isEmpty { dest = dests.first?.id ?? "" } }
    }
}

// MARK: Act note

struct ActNoteSheet: View {
    @Bindable var model: StoryModel
    let actId: String
    @State private var note = ""
    @FocusState private var focused: Bool

    var body: some View {
        if let act = model.board?.acts.first(where: { $0.id == actId }) {
            StorySheetFrame(title: "Note for \u{201C}\(act.title)\u{201D}", width: 400) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Note").font(.callout.weight(.medium))
                    TextEditor(text: $note)
                        .frame(height: 110)
                        .focused($focused)
                        .padding(4)
                        .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(.quaternary))
                        .accessibilityLabel("Note")
                }
            } footer: {
                Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Save Note") {
                    let n = note
                    model.sheet = nil
                    Task { await model.setActNote(actId, n) }
                }
                .keyboardShortcut(.defaultAction)
            }
            .onAppear {
                note = act.note ?? ""
                focused = true
            }
        }
    }
}

// MARK: Card Display

/// Card Display Settings: what a minimized Scene Card shows is the person's own view setting. It
/// never changes a card, a scene or the screenplay, and never enters undo.
struct CardDisplaySheet: View {
    @Bindable var model: StoryModel

    var body: some View {
        StorySheetFrame(title: "Card Display Settings",
                        sub: "Choose what minimized Scene Cards show. This changes only your view; the story and the screenplay stay exactly as they are.",
                        width: 540) {
            if let state = model.display {
                VStack(alignment: .leading, spacing: 12) {
                    Text("Preset").font(.headline)
                    ForEach(choices(state), id: \.preset) { c in
                        StoryChoice(title: c.label, detail: c.description, selected: state.current.preset == c.preset) {
                            Task { await model.setCardDisplay(c.preset, c.preset == .custom ? state.current.toggles : nil) }
                        }
                        .accessibilityIdentifier("story-preset-\(c.preset.rawValue)")
                    }
                    Text("Show on minimized cards").font(.headline).padding(.top, 4)
                    LazyVGrid(columns: [GridItem(.flexible(), alignment: .leading), GridItem(.flexible(), alignment: .leading)], spacing: 6) {
                        ForEach(StoryCardDisplayRules.toggles, id: \.label) { t in
                            Toggle(t.label, isOn: Binding(
                                get: { state.current.toggles[keyPath: t.key] },
                                set: { v in
                                    var next = state.current.toggles
                                    next[keyPath: t.key] = v
                                    Task { await model.setCardDisplay(.custom, next) }
                                }))
                            .toggleStyle(.checkbox)
                            .accessibilityIdentifier("story-toggle-\(t.label)")
                        }
                    }
                    Text("Ticking a box switches to Custom. Open a card to read and edit everything it holds.")
                        .font(.caption).foregroundStyle(.secondary)
                }
            } else {
                ProgressView()
            }
        } footer: {
            Button("Done") { model.sheet = nil }.keyboardShortcut(.defaultAction)
        }
    }

    private func choices(_ state: StoryCardDisplayState) -> [(preset: StoryCardPreset, label: String, description: String)] {
        state.presets.map { ($0.preset, $0.label, $0.description) } + [(.custom, "Custom", "Any combination of the boxes below.")]
    }
}

// MARK: Apply order

struct ApplyOrderSheet: View {
    @Bindable var model: StoryModel
    @State private var m: ApplyOrderModel?

    var body: some View {
        let p = m?.preview
        StorySheetFrame(
            title: "Apply this order to the written screenplay?",
            sub: p?.hasWrittenScenes == true
                ? "Reordering these cards will change the screenplay scene order. Some of these scenes have already been written."
                : "Reordering these cards will change the screenplay scene order.",
            width: 540
        ) {
            VStack(alignment: .leading, spacing: 10) {
                if let m, let p {
                    if p.draftId == nil {
                        StoryBanner(tone: .info, text: "There is no screenplay yet. Use Build Screenplay to create one from the Beat Board.")
                    } else {
                        if p.screenplays.count > 1 {
                            Picker("Screenplay to reorder", selection: Binding(
                                get: { p.draftId ?? "" }, set: { id in Task { await m.choose(id) } })) {
                                ForEach(p.screenplays.filter { $0.currentDraftId != nil }, id: \.id) { s in
                                    Text("\(s.title) (\(s.currentDraftName ?? "current draft"))").tag(s.currentDraftId ?? "")
                                }
                            }
                        }
                        if p.locked {
                            StoryBanner(tone: .warn, text: "\u{201C}\(p.draftName ?? "")\u{201D} is locked as the shooting draft. Start a revision in the Screenplay workspace to change scene order.")
                        } else if p.moves.isEmpty {
                            StoryBanner(tone: .ok, text: "\u{201C}\(p.draftName ?? "")\u{201D} already follows the Beat Board order\(p.linkedCount == 0 ? " (no cards are linked to its scenes)" : "").")
                        } else {
                            Text("In \u{201C}\(p.draftName ?? "")\u{201D}\(p.screenplayTitle.map { " of \($0)" } ?? ""):")
                            ScrollView {
                                VStack(alignment: .leading, spacing: 4) {
                                    ForEach(p.moves, id: \.sceneId) { mv in
                                        Text("Screenplay Scene \(Int(mv.fromNumber)) would become Scene \(Int(mv.toNumber)): \(mv.heading)")
                                    }
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                            }
                            .frame(maxHeight: 200)
                            Text("Production planning, if any, is not reordered automatically.").font(.caption).foregroundStyle(.secondary)
                        }
                    }
                } else if let f = m?.failure {
                    StoryBanner(tone: .err, text: f)
                } else {
                    ProgressView()
                }
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Apply") { if let m { Task { await m.apply() } } }
                .keyboardShortcut(.defaultAction)
                .disabled(!(m?.canApply ?? false))
        }
        .task {
            let made = ApplyOrderModel(story: model)
            m = made
            await made.load()
        }
    }
}

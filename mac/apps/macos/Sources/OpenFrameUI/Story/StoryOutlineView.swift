// The Outline view: the same objects as the Board, shown as an indented tree. Reorder by
// dragging a row or with Option and the Up or Down arrow; nothing here adds data or touches the
// screenplay.

import SwiftUI
import OpenFrameCore

struct StoryOutlineView: View {
    @Bindable var model: StoryModel
    let board: StoryBoardState
    @FocusState private var focus: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let rows = StoryLogic.outlineRows(board, collapsed: model.collapsed, filter: model.filter, includeEnds: true)
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
                ForEach(rows) { row in
                    OutlineRowView(model: model, board: board, row: row, rows: rows, focus: $focus)
                }
                if !model.addingAct {
                    Button { model.addingAct = true } label: { Label("Add Act", systemImage: "plus") }
                        .buttonStyle(.bordered).padding(.top, 10)
                        .accessibilityIdentifier("story-add-act")
                } else {
                    InlineField(initial: "", label: "Act title", placeholder: "Act title, e.g. Act 1: The Return",
                                onCommit: { v in Task { await model.createAct(v) } },
                                onCancel: { model.addingAct = false })
                        .padding(8).background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
                }
            }
            .padding(16)
        }
        .contentShape(Rectangle())
        .onTapGesture { model.clearSelection() }
        .onChange(of: model.focusRequest) { _, key in
            if let key { focus = key; model.focusRequest = nil }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Story outline")
        .accessibilityIdentifier("story-outline")
    }
}

private struct OutlineRowView: View {
    @Bindable var model: StoryModel
    let board: StoryBoardState
    let row: StoryOutlineRow
    let rows: [StoryOutlineRow]
    var focus: StoryFocus
    @Environment(\.ofTheme) private var theme
    @State private var size = CGSize(width: 600, height: 28)

    private var indent: CGFloat { [0, 24, 48][min(row.depth, 2)] }

    var body: some View {
        switch row.kind {
        case let .act(id):
            if let act = board.acts.first(where: { $0.id == id }) { actRow(act) }
        case let .item(ref):
            if let loc = StoryLogic.locate(board, ref) { itemRow(loc.item, ref) }
        case let .end(container):
            endRow(container)
        case let .header(container):
            headerRow(container)
        }
    }

    // MARK: Act

    private func actRow(_ act: StoryActDto) -> some View {
        let collapsed = model.collapsed.contains(act.id)
        let renaming = model.renamingId == act.id
        let hasMarker = model.actHint == .some(act.id)
        return VStack(spacing: 2) {
            if hasMarker { StoryInsertionMarker() }
            HStack(spacing: 6) {
                Image(systemName: "line.3.horizontal").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                Button { Task { await model.setCollapsed(act.id, !collapsed) } } label: {
                    Image(systemName: collapsed ? "chevron.right" : "chevron.down").frame(width: 14)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(collapsed ? "Expand \(act.title)" : "Collapse \(act.title)")
                if renaming {
                    InlineField(initial: act.title, label: "Act title",
                                onCommit: { v in Task { await model.renameAct(act.id, current: act.title, to: v) } },
                                onCancel: { model.cancelEditing() })
                } else {
                    Text(act.title).fontWeight(.bold).lineLimit(1)
                        .onTapGesture(count: 2) { model.renamingId = act.id }
                }
                Spacer(minLength: 4)
                Text(StoryLogic.actCountLabel(act)).font(.callout.weight(.semibold))
                Menu { StoryActMenuOutline(model: model, act: act) } label: { Image(systemName: "ellipsis") }
                    .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                    .accessibilityLabel("Act options for \(act.title)")
            }
            .padding(.horizontal, 8).padding(.vertical, 7)
            .background(model.hint?.target == .act(act.id) ? theme["accent-soft"] : theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .contentShape(Rectangle())
            .reading(size: $size)
            .onDrag {
                model.beginActDrag(act.id)
                return StoryDragType.provider()
            } preview: {
                Text(act.title).fontWeight(.semibold).padding(8).background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
            }
            .contextMenu { StoryActMenuOutline(model: model, act: act) }
            .storyDrop(model, size: size) {
                model.dragging?.actId != nil ? .act(act.id) : .into(.act(act.id), atStart: true)
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Act: \(act.title)")
            .accessibilityAddTraits(.isHeader)
            .accessibilityAction(named: "Move Up") { Task { await model.stepAct(act.id, direction: -1) } }
            .accessibilityAction(named: "Move Down") { Task { await model.stepAct(act.id, direction: 1) } }
            if model.addingSequenceIn == act.id {
                InlineField(initial: "", label: "Sequence name", placeholder: "Sequence name",
                            onCommit: { v in Task { await model.createSequence(in: act.id, title: v) } },
                            onCancel: { model.addingSequenceIn = nil })
                    .padding(.leading, 24)
            }
        }
        .padding(.top, 8)
    }

    // MARK: Item

    private func itemRow(_ item: StoryItem, _ ref: StoryItemRef) -> some View {
        let key = ref.key
        let selected = model.selected.contains(key)
        let editing = model.editingKey == key || (item.kind == .sequence && model.renamingId == ref.id)
        let collapsed = model.collapsed.contains(ref.id)
        let color: String? = {
            switch item {
            case let .card(c): c.color
            case let .beat(b): b.color
            case .sequence: nil
            }
        }()
        let converted: Bool = { if case let .beat(b) = item { b.state == "converted" } else { false } }()
        let marker: Bool = {
            guard let h = model.hint, let b = h.before, let c = row.container else { return false }
            return h.target == c && b == ref
        }()
        return VStack(spacing: 2) {
            if marker { StoryInsertionMarker().padding(.leading, indent) }
            HStack(spacing: 6) {
                Image(systemName: "line.3.horizontal").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                if item.kind == .sequence {
                    Button { Task { await model.setCollapsed(ref.id, !collapsed) } } label: {
                        Image(systemName: collapsed ? "chevron.right" : "chevron.down").frame(width: 14)
                    }
                    .buttonStyle(.borderless)
                    .accessibilityLabel(collapsed ? "Expand \(item.shownText)" : "Collapse \(item.shownText)")
                }
                if let color { Circle().fill(theme.label(color)).frame(width: 9, height: 9).accessibilityHidden(true) }
                if editing {
                    InlineField(initial: item.shownText, label: item.kind == .sequence ? "Sequence name" : item.kind == .beat ? "Beat text" : "Short description",
                                multiline: item.kind != .sequence,
                                onCommit: { v in Task { await model.commitText(ref, v) } },
                                onCancel: { model.cancelEditing() })
                } else {
                    (Text(row.prefix).foregroundStyle(theme["muted"]) + Text(row.text.isEmpty ? "blank" : row.text).italic(row.text.isEmpty))
                        .fontWeight(item.kind == .sequence ? .bold : .regular)
                        .foregroundStyle(converted ? theme["muted"] : theme["ink"])
                        .lineLimit(1)
                    if converted { Text("Converted").font(.caption).padding(.horizontal, 6).background(.quaternary, in: Capsule()) }
                }
                Spacer(minLength: 4)
                if case let .card(c) = item, !(c.sceneHeading ?? "").isEmpty {
                    Text("H").font(.caption.bold()).foregroundStyle(theme["muted"]).help("Has a scene heading")
                        .accessibilityLabel("Has a scene heading")
                }
                if case let .sequence(s) = item { Text("\(Int(s.cardCount))").foregroundStyle(theme["muted"]) }
            }
            .padding(.horizontal, 8).padding(.vertical, 5)
            .padding(.leading, indent)
            .background(selected ? theme["accent-soft"] : (model.hint?.target == .sequence(ref.id) && item.kind == .sequence ? theme["accent-soft"] : Color.clear),
                        in: RoundedRectangle(cornerRadius: 6, style: .continuous))
            .contentShape(Rectangle())
            .reading(size: $size)
            .focusable()
            .focused(focus, equals: key)
            .focusEffectDisabled()
            .onKeyPress(phases: [.down, .repeat]) { press in editing ? .ignored : model.handleKey(press, item: item) }
            .simultaneousGesture(TapGesture(count: 2).onEnded { model.openDrawer(ref) }, including: editing ? .subviews : .all)
            .simultaneousGesture(TapGesture().onEnded {
                let flags = NSEvent.modifierFlags
                model.click(ref, extend: flags.contains(.command) || flags.contains(.shift))
            }, including: editing ? .subviews : .all)
            .contextMenu { StoryItemMenu(model: model, item: item) }
            .onDrag {
                model.beginDrag(ref)
                return StoryDragType.provider()
            } preview: {
                Text(StoryLogic.label(item)).padding(8).background(theme["panel"], in: RoundedRectangle(cornerRadius: 6))
            }
            .storyDrop(model, size: size) { .beside(ref) }
            .accessibilityElement(children: editing ? .contain : .combine)
            .accessibilityLabel(row.label + (converted ? " (converted)" : ""))
            .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
            .accessibilityIdentifier("story-outline-\(ref.id)")
            .accessibilityAction(named: "Open") { model.openDrawer(ref) }
            .accessibilityAction(named: "Move Up") { Task { await model.step(ref, direction: -1) } }
            .accessibilityAction(named: "Move Down") { Task { await model.step(ref, direction: 1) } }
        }
    }

    // MARK: End of container and section headers

    private func endRow(_ container: StoryContainerRef) -> some View {
        let here = model.hint?.target == container && model.hint?.before == nil
        let empty = StoryLogic.children(board, of: container).isEmpty
        return VStack(alignment: .leading, spacing: 2) {
            if here { StoryInsertionMarker().padding(.leading, indent) }
            if empty {
                Text(container.parentType == .sequence ? "Drop scenes here." : container.parentType == .parking ? "Drag cards and beats here to keep them out of the story without deleting them." : "Add a sequence or scene.")
                    .font(.caption).foregroundStyle(theme["muted"]).padding(.leading, indent + 8)
            } else {
                Color.clear.frame(height: 4)
            }
        }
        .reading(size: $size)
        .storyDrop(model, size: size) { .into(container, atStart: false) }
    }

    private func headerRow(_ container: StoryContainerRef) -> some View {
        HStack(spacing: 6) {
            if container.parentType == .parking { Image(systemName: "parkingsign") }
            Text(row.text).fontWeight(.bold)
            Spacer()
        }
        .padding(.horizontal, 8).padding(.vertical, 7)
        .background(model.hint?.target == container ? theme["accent-soft"] : theme["note-bg"].opacity(0.5), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .padding(.top, 8)
        .reading(size: $size)
        .storyDrop(model, size: size) { .into(container, atStart: true) }
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
    }
}

/// Act menu in the Outline: Move Up and Move Down instead of Left and Right.
private struct StoryActMenuOutline: View {
    @Bindable var model: StoryModel
    let act: StoryActDto

    var body: some View {
        let ids = model.board?.acts.map(\.id) ?? []
        let i = ids.firstIndex(of: act.id) ?? 0
        Button("Rename") { model.renamingId = act.id }
        Button("Edit note\u{2026}") { model.sheet = .actNote(act.id) }
        Button("Add Sequence") { model.addingSequenceIn = act.id }
        Button("Add Scene") { Task { await model.newCard(in: .act(act.id)) } }
        Button("Add Beat") { Task { await model.newBeat(in: .act(act.id)) } }
        Button("Move Up") { Task { await model.stepAct(act.id, direction: -1) } }.disabled(i <= 0)
        Button("Move Down") { Task { await model.stepAct(act.id, direction: 1) } }.disabled(i >= ids.count - 1)
        Divider()
        Button("Delete", role: .destructive) { Task { await model.requestDeleteAct(act.id) } }
    }
}

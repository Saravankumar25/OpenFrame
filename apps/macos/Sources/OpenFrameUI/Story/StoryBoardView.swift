// The Board view: act columns side by side (scroll sideways), sequence groups, compact scene
// cards, dashed beats, the hatched Parking Lot and the Unassigned area. Drag a card to move it;
// every move calls the same `story.move_items` the Windows board calls.

import SwiftUI
import OpenFrameCore

struct StoryBoardView: View {
    @Bindable var model: StoryModel
    let board: StoryBoardState
    @AppStorage("of.story.zoom") private var zoomPercent = 100
    @AppStorage("of.story.parkingOpen") private var parkingOpen = true
    @FocusState private var focus: String?
    @Environment(\.ofTheme) private var theme

    private var zoom: Double { Double(zoomPercent) / 100 }

    var body: some View {
        ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: 14) {
                ForEach(board.acts, id: \.id) { act in
                    if model.actHint == .some(act.id) { ActInsertionBar() }
                    StoryActColumn(model: model, act: act, zoom: zoom, focus: $focus)
                }
                if model.actHint == .some(nil) { ActInsertionBar() }
                if model.addingAct {
                    NewActColumn(model: model, zoom: zoom)
                } else {
                    Button { model.addingAct = true } label: { Label("Add Act", systemImage: "plus") }
                        .buttonStyle(.bordered)
                        .controlSize(.large)
                        .accessibilityIdentifier("story-add-act")
                }
                if !board.unassigned.isEmpty { StoryUnassignedColumn(model: model, board: board, zoom: zoom, focus: $focus) }
                if parkingOpen { StoryParkingColumn(model: model, board: board, zoom: zoom, focus: $focus) }
            }
            .padding(16)
        }
        .scrollIndicators(.automatic)
        .contentShape(Rectangle())
        .onTapGesture { model.clearSelection() }
        .onChange(of: model.focusRequest) { _, key in
            if let key { focus = key; model.focusRequest = nil }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Story Board")
        .accessibilityIdentifier("story-board")
    }
}

struct ActInsertionBar: View {
    @Environment(\.ofTheme) private var theme
    var body: some View {
        Capsule().fill(theme["accent"]).frame(width: 3).frame(maxHeight: .infinity).accessibilityHidden(true)
    }
}

/// The inline field that adds an act.
struct NewActColumn: View {
    @Bindable var model: StoryModel
    let zoom: Double
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            InlineField(initial: "", label: "Act title", placeholder: "Act title, e.g. Act 1: The Return",
                        onCommit: { v in Task { await model.createAct(v) } },
                        onCancel: { model.addingAct = false })
                .font(.system(size: 14 * zoom, weight: .semibold))
            Text("Press Return to add the Act. You can add a note later.").font(.caption).foregroundStyle(theme["muted"])
        }
        .padding(12)
        .frame(width: 270 * zoom, alignment: .topLeading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["accent"]))
    }
}

struct StoryActColumn: View {
    @Bindable var model: StoryModel
    let act: StoryActDto
    let zoom: Double
    var focus: StoryFocus
    @Environment(\.ofTheme) private var theme
    @State private var size = CGSize(width: 270, height: 400)

    var body: some View {
        let collapsed = model.collapsed.contains(act.id)
        let renaming = model.renamingId == act.id
        let shape = RoundedRectangle(cornerRadius: 12, style: .continuous)
        let hinted = model.hint?.target == .act(act.id)

        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 6) {
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
                    Text(act.title).fontWeight(.semibold).lineLimit(1).truncationMode(.tail)
                        .help(act.note ?? act.title)
                        .onTapGesture(count: 2) { model.renamingId = act.id }
                }
                Spacer(minLength: 4)
                Text(StoryLogic.actCountLabel(act)).font(.caption).foregroundStyle(theme["muted"])
                Menu {
                    StoryActMenu(model: model, act: act)
                } label: {
                    Image(systemName: "ellipsis")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .accessibilityLabel("Act options for \(act.title)")
                .accessibilityIdentifier("story-act-options-\(act.id)")
            }
            .font(.system(size: 14 * zoom))
            .padding(.horizontal, 12).padding(.vertical, 10)
            .contentShape(Rectangle())
            .onDrag {
                model.beginActDrag(act.id)
                return StoryDragType.provider()
            } preview: {
                Text(act.title).fontWeight(.semibold).padding(10).background(theme["surface2"], in: shape)
            }
            .contextMenu { StoryActMenu(model: model, act: act) }
            .accessibilityAction(named: "Move Left") { Task { await model.stepAct(act.id, direction: -1) } }
            .accessibilityAction(named: "Move Right") { Task { await model.stepAct(act.id, direction: 1) } }

            if !collapsed {
                Divider()
                ScrollView(.vertical) {
                    LazyVStack(alignment: .leading, spacing: 6) {
                        StoryItemList(model: model, container: .act(act.id), items: act.items, zoom: zoom, focus: focus)
                        if model.addingSequenceIn == act.id {
                            InlineField(initial: "", label: "Sequence name", placeholder: "Sequence name, e.g. Hero Introduction",
                                        onCommit: { v in Task { await model.createSequence(in: act.id, title: v) } },
                                        onCancel: { model.addingSequenceIn = nil })
                                .padding(8)
                                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        }
                        if act.items.isEmpty && model.addingSequenceIn != act.id {
                            Text("Add a sequence or scene.").font(.caption).foregroundStyle(theme["muted"]).padding(.vertical, 4)
                        }
                        HStack(spacing: 6) {
                            Button { Task { await model.newCard(in: .act(act.id)) } } label: { Label("Scene", systemImage: "plus") }
                                .accessibilityLabel("Add Scene")
                            Button { model.addingSequenceIn = act.id } label: { Label("Sequence", systemImage: "plus") }
                                .accessibilityLabel("Add Sequence")
                            Button { Task { await model.newBeat(in: .act(act.id)) } } label: { Label("Beat", systemImage: "plus") }
                                .accessibilityLabel("Add Beat")
                        }
                        .buttonStyle(.borderless).font(.caption).padding(.top, 2)
                    }
                    .padding(10)
                }
                .frame(maxHeight: .infinity)
            }
        }
        .frame(width: 270 * zoom)
        .frame(maxHeight: collapsed ? nil : .infinity, alignment: .top)
        .background(theme["surface2"].opacity(0.55), in: shape)
        .overlay(shape.strokeBorder(hinted ? theme["accent"] : theme["line"], style: StrokeStyle(lineWidth: hinted ? 2 : 1, dash: hinted ? [6, 3] : [])))
        .reading(size: $size)
        .storyDrop(model, size: size, vertical: false) {
            model.dragging?.actId != nil ? .act(act.id) : .into(.act(act.id), atStart: false)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Act \(act.title)")
        .accessibilityIdentifier("story-act-\(act.id)")
    }
}

/// The items of an act's menu: also what a keyboard user reaches with the context menu key.
struct StoryActMenu: View {
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
        Button("Move Left") { Task { await model.stepAct(act.id, direction: -1) } }.disabled(i <= 0)
        Button("Move Right") { Task { await model.stepAct(act.id, direction: 1) } }.disabled(i >= ids.count - 1)
        Divider()
        Button("Delete", role: .destructive) { Task { await model.requestDeleteAct(act.id) } }
    }
}

struct StoryParkingColumn: View {
    @Bindable var model: StoryModel
    let board: StoryBoardState
    let zoom: Double
    var focus: StoryFocus
    @Environment(\.ofTheme) private var theme
    @State private var size = CGSize(width: 270, height: 400)

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 12, style: .continuous)
        let hinted = model.hint?.target == .parking
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Label("Parking Lot", systemImage: "parkingsign").fontWeight(.semibold)
                Spacer()
                Text("\(board.parking.count)").font(.caption).foregroundStyle(theme["muted"])
                    .accessibilityLabel("\(board.parking.count) items")
            }
            .font(.system(size: 14 * zoom))
            .padding(.horizontal, 12).padding(.vertical, 10)
            Divider()
            ScrollView(.vertical) {
                LazyVStack(alignment: .leading, spacing: 6) {
                    StoryItemList(model: model, container: .parking, items: board.parking, zoom: zoom, focus: focus)
                    if board.parking.isEmpty {
                        Text("Drag cards and beats here to keep them out of the story without deleting them.")
                            .font(.caption).foregroundStyle(theme["muted"]).padding(.vertical, 4)
                    }
                    HStack(spacing: 6) {
                        Button { Task { await model.newBeat(in: .parking) } } label: { Label("Beat", systemImage: "plus") }
                            .accessibilityLabel("Add Beat to the Parking Lot")
                        Button { Task { await model.newCard(in: .parking) } } label: { Label("Scene", systemImage: "plus") }
                            .accessibilityLabel("Add Scene to the Parking Lot")
                    }
                    .buttonStyle(.borderless).font(.caption)
                }
                .padding(10)
            }
        }
        .frame(width: 270 * zoom)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(theme["note-bg"].opacity(0.5), in: shape)
        .overlay(shape.strokeBorder(hinted ? theme["accent"] : theme["line2"], style: StrokeStyle(lineWidth: hinted ? 2 : 1, dash: [6, 4])))
        .reading(size: $size)
        .storyDrop(model, size: size) { .into(.parking, atStart: false) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Parking Lot")
        .accessibilityIdentifier("story-parking")
    }
}

struct StoryUnassignedColumn: View {
    @Bindable var model: StoryModel
    let board: StoryBoardState
    let zoom: Double
    var focus: StoryFocus
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 12, style: .continuous)
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Unassigned").fontWeight(.semibold)
                Spacer()
                Text("\(board.unassigned.count)").font(.caption).foregroundStyle(theme["muted"])
            }
            .font(.system(size: 14 * zoom))
            .padding(.horizontal, 12).padding(.vertical, 10)
            Text("Restored items whose Act or Sequence no longer exists. Drag them back into the story.")
                .font(.caption).foregroundStyle(theme["muted"]).padding(.horizontal, 12).padding(.bottom, 6)
            Divider()
            ScrollView(.vertical) {
                LazyVStack(alignment: .leading, spacing: 6) {
                    StoryItemList(model: model, container: .unassigned, items: board.unassigned, zoom: zoom, focus: focus)
                }
                .padding(10)
            }
        }
        .frame(width: 270 * zoom)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(theme["yellow-soft"].opacity(0.5), in: shape)
        .overlay(shape.strokeBorder(theme["yellow-line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Unassigned")
    }
}

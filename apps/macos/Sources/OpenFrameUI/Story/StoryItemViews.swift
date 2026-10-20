// The things on the Story Board: scene cards, beats, sequence blocks, their menus and the list
// that holds them. Board and Outline share the menu and the keyboard handling.

import AppKit
import SwiftUI
import OpenFrameCore

typealias StoryFocus = FocusState<String?>.Binding

// MARK: Menu

/// The context menu of a card, beat or sequence. The same items serve the keyboard: Move Up and
/// Move Down reorder among siblings, Move to Previous Act and Next Act move across the board.
struct StoryItemMenu: View {
    @Bindable var model: StoryModel
    let item: StoryItem

    var body: some View {
        let ref = item.ref
        let loc = model.board.flatMap { StoryLogic.locate($0, ref) }
        let parked = loc?.container.parentType == .parking
        switch item {
        case let .sequence(s):
            Button("Open") { model.openDrawer(ref) }
            Button("Rename") { model.renamingId = s.id }
            Button("Move\u{2026}") { model.sheet = .move([ref]) }
            moveItems(ref)
            Button("Add Scene") { Task { await model.newCard(in: .sequence(s.id)) } }
            Button("Add Beat") { Task { await model.newBeat(in: .sequence(s.id)) } }
            Divider()
            Button("Delete", role: .destructive) { Task { await model.requestDeleteSequence(s.id) } }
        case let .beat(b):
            Button("Open") { model.openDrawer(ref) }
            Button("Convert to Scene") { Task { await model.convertBeat(b.id) } }.disabled(b.state == "converted")
            common(ref, parked: parked)
            Divider()
            Button("Delete", role: .destructive) { Task { await model.requestDelete(model.targets(for: ref)) } }
        case .card:
            Button("Open") { model.openDrawer(ref) }
            common(ref, parked: parked)
            Button("Convert to Screenplay Scene") {
                model.sheet = .build(preselect: model.targets(for: ref).filter { $0.kind == .card }.map(\.id))
            }.disabled(parked)
            Button("Copy as Beat") { Task { await model.copyAsBeat(ref.id) } }
            Divider()
            Button("Delete", role: .destructive) { Task { await model.requestDelete(model.targets(for: ref)) } }
        }
    }

    @ViewBuilder private func common(_ ref: StoryItemRef, parked: Bool) -> some View {
        Button("Duplicate") { Task { await model.duplicate(model.targets(for: ref)) } }
        Button("Move\u{2026}") { model.sheet = .move(model.targets(for: ref)) }
        moveItems(ref)
        if parked {
            Button("Restore to Story") { Task { await model.restoreToStory(model.targets(for: ref)) } }
        } else {
            Button("Send to Parking Lot") { Task { await model.park(model.targets(for: ref)) } }
        }
    }

    @ViewBuilder private func moveItems(_ ref: StoryItemRef) -> some View {
        Button("Move Up") { Task { await model.step(ref, direction: -1) } }.disabled(!model.canStep(ref, direction: -1))
        Button("Move Down") { Task { await model.step(ref, direction: 1) } }.disabled(!model.canStep(ref, direction: 1))
        Button("Move to Previous Act") { Task { await model.moveToAdjacentAct(ref, direction: -1) } }
            .disabled(!model.canMoveToAdjacentAct(ref, direction: -1))
        Button("Move to Next Act") { Task { await model.moveToAdjacentAct(ref, direction: 1) } }
            .disabled(!model.canMoveToAdjacentAct(ref, direction: 1))
    }
}

// MARK: Keyboard

extension StoryModel {
    /// Keys on a focused card, beat or sequence (Return opens, Delete deletes, Space selects,
    /// Command-D duplicates, Option with Up or Down moves, the arrows move focus).
    func handleKey(_ press: KeyPress, item: StoryItem) -> KeyPress.Result {
        let ref = item.ref
        let mod = press.modifiers
        switch press.key {
        case .return:
            openDrawer(ref)
            return .handled
        case .delete, .deleteForward:
            Task {
                if case let .sequence(s) = item { await requestDeleteSequence(s.id) }
                else { await requestDelete(targets(for: ref)) }
            }
            return .handled
        case .space:
            toggle(ref.key)
            return .handled
        case .escape:
            clearSelection()
            return .handled
        case .upArrow, .downArrow:
            let down = press.key == .downArrow
            if mod.contains(.option) {
                Task { await step(ref, direction: down ? 1 : -1) }
            } else {
                navigate(from: ref.key, direction: down ? .down : .up, extend: mod.contains(.shift))
            }
            return .handled
        case .leftArrow, .rightArrow:
            navigate(from: ref.key, direction: press.key == .rightArrow ? .right : .left, extend: mod.contains(.shift))
            return .handled
        default:
            if mod.contains(.command), press.characters.lowercased() == "d", item.kind != .sequence {
                Task { await duplicate(targets(for: ref)) }
                return .handled
            }
            // With focus on a card (not in a text field) Command-Z undoes the project change.
            if mod.contains(.command), !mod.contains(.option), press.characters.lowercased() == "z" {
                Task { if mod.contains(.shift) { await app.redo() } else { await app.undo() } }
                return .handled
            }
            return .ignored
        }
    }
}

// MARK: Card and beat

struct StoryCardView: View {
    @Bindable var model: StoryModel
    let item: StoryItem
    let zoom: Double
    var focus: StoryFocus
    @Environment(\.ofTheme) private var theme
    @State private var size = CGSize(width: 240, height: 60)

    private var ref: StoryItemRef { item.ref }
    private var isBeat: Bool { item.kind == .beat }
    private var converted: Bool { if case let .beat(b) = item { b.state == "converted" } else { false } }
    private var labelColor: String? {
        switch item {
        case let .card(c): c.color
        case let .beat(b): b.color
        case .sequence: nil
        }
    }
    private var rich: Bool { if case .card = item { !StoryCardDisplayRules.isCompact(model.toggles) } else { false } }

    var body: some View {
        let key = ref.key
        let selected = model.selected.contains(key)
        let editing = model.editingKey == key
        let text = item.shownText
        let a11yLabel: String = "\(isBeat ? "Beat" : "Scene Card"): \(text.isEmpty ? "blank" : text)\(converted ? " (converted)" : "")"
        let traits: AccessibilityTraits = selected ? [.isButton, .isSelected] : [.isButton]
        let gestureMask: GestureMask = editing ? .subviews : .all

        chrome(selected: selected, editing: editing, text: text)
            .contentShape(shape)
            .reading(size: $size)
            .focusable()
            .focused(focus, equals: key)
            .focusEffectDisabled()
            .onKeyPress(phases: [.down, .repeat]) { press in editing ? .ignored : model.handleKey(press, item: item) }
            .simultaneousGesture(TapGesture(count: 2).onEnded { model.openDrawer(ref) }, including: gestureMask)
            .simultaneousGesture(TapGesture().onEnded {
                let flags = NSEvent.modifierFlags
                model.click(ref, extend: flags.contains(.command) || flags.contains(.shift))
                focus.wrappedValue = key
            }, including: gestureMask)
            .contextMenu { StoryItemMenu(model: model, item: item) }
            .onDrag {
                model.beginDrag(ref)
                return StoryDragType.provider()
            } preview: {
                dragPreview
            }
            .storyDrop(model, size: size) { .beside(ref) }
            .accessibilityElement(children: editing ? .contain : .combine)
            .accessibilityLabel(a11yLabel)
            .accessibilityValue(rich ? richFacts() : "")
            .accessibilityAddTraits(traits)
            .accessibilityHint("Press Return to open. Use the context menu or Option with the arrow keys to move.")
            .accessibilityIdentifier("story-card-\(ref.id)")
            .modifier(CardActions(model: model, ref: ref))
    }

    private var dragPreview: some View {
        Text(StoryLogic.label(item)).lineLimit(2).padding(8).frame(width: 200, alignment: .leading)
            .background(theme["panel"], in: shape).overlay(shape.strokeBorder(theme["accent"]))
    }

    /// The card itself: text or inline field, indicators, background, border and colour stripe.
    private func chrome(selected: Bool, editing: Bool, text: String) -> some View {
        let toggles = model.toggles
        let blank = text.trimmingCharacters(in: .whitespaces).isEmpty
        let background: Color = isBeat ? theme["beat-bg"] : (selected ? theme["accent-soft"] : theme["panel"])
        let border: Color = selected ? theme["accent"] : (isBeat ? theme["beat-line"] : theme["line"])
        let dash: [CGFloat] = isBeat ? [4, 3] : []
        return VStack(alignment: .leading, spacing: 3) {
            if editing {
                InlineField(initial: text, label: isBeat ? "Beat text" : "Short description",
                            placeholder: isBeat ? "Beat text" : "What happens in this scene?", multiline: true,
                            onCommit: { v in Task { await model.commitText(ref, v) } },
                            onCancel: { model.cancelEditing() })
            } else if rich, case let .card(c) = item {
                richFace(c, toggles)
            } else {
                Text(blank ? (isBeat ? "Blank beat" : "Blank scene card") : text)
                    .italic(blank)
                    .foregroundStyle(converted || blank ? theme["muted"] : theme["ink"])
                    .lineLimit(clamp)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                indicators(converted: converted)
            }
        }
        .font(.system(size: 13 * zoom))
        .padding(.vertical, 8)
        .padding(.leading, 11)
        .padding(.trailing, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(background, in: shape)
        .overlay(alignment: .leading) { stripe }
        .overlay { shape.strokeBorder(border, style: StrokeStyle(lineWidth: selected ? 2 : 1, dash: dash)) }
    }

    @ViewBuilder private var stripe: some View {
        if let color = labelColor {
            Rectangle().fill(theme.label(color)).frame(width: 4)
                .clipShape(UnevenRoundedRectangle(topLeadingRadius: 8, bottomLeadingRadius: 8))
        }
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: 8, style: .continuous) }

    /// A card shows its whole text (owner request 2026-10-20, as on Windows); only the overview
    /// zoom (70 percent and below) shortens it. The full text is always in the card drawer.
    private var clamp: Int? { zoom <= 0.7 ? 3 : nil }

    private func richFacts() -> String {
        guard case let .card(c) = item else { return "" }
        // The long description and note previews are on the card face, so a screen reader reads them too.
        let previews = [StoryCardDisplayRules.longPreview(c, model.toggles), StoryCardDisplayRules.notePreview(c, model.toggles)]
        return (previews + StoryCardDisplayRules.facts(c, model.toggles)).filter { !$0.isEmpty }.joined(separator: ", ")
    }

    @ViewBuilder private func richFace(_ c: StorySceneCardDto, _ t: StoryCardToggles) -> some View {
        if t.sceneHeading, let h = c.sceneHeading, !h.isEmpty {
            Text(h).font(.system(size: 12 * zoom, design: .monospaced).weight(.bold)).textCase(.uppercase)
                .lineLimit(clamp == nil ? nil : 1).truncationMode(.tail)
                .fixedSize(horizontal: false, vertical: true)
                .help(h)
        }
        if t.shortDescription {
            Text(c.shortDescription.trimmingCharacters(in: .whitespaces).isEmpty ? "Blank scene card" : c.shortDescription)
                .italic(c.shortDescription.trimmingCharacters(in: .whitespaces).isEmpty)
                .lineLimit(clamp)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        let long = StoryCardDisplayRules.longPreview(c, t)
        if !long.isEmpty {
            Text(long).font(.system(size: 12 * zoom)).foregroundStyle(theme["muted"]).lineLimit(clamp == nil ? nil : 2)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("story-long-preview")
        }
        let note = StoryCardDisplayRules.notePreview(c, t)
        if !note.isEmpty {
            Text("Note: \(note)").font(.system(size: 12 * zoom)).italic().foregroundStyle(theme["muted"])
                .lineLimit(clamp == nil ? nil : 1)
                .fixedSize(horizontal: false, vertical: true)
                .help(note)
        }
        let facts = StoryCardDisplayRules.facts(c, t)
        if !facts.isEmpty {
            Text(facts.joined(separator: " \u{00B7} ")).font(.system(size: 11.5 * zoom).weight(.semibold)).foregroundStyle(theme["ink2"])
        }
        HStack(spacing: 8) {
            if !t.sceneHeading, let h = c.sceneHeading, !h.isEmpty {
                Text("H").font(.caption2.bold()).help("Has a scene heading").accessibilityLabel("Has a scene heading")
            }
            if c.screenplaySceneId != nil {
                Image(systemName: "doc.text").font(.caption2).help("Used to create a screenplay scene")
                    .accessibilityLabel("Used to create a screenplay scene")
            }
            if t.attachments, !c.attachments.isEmpty {
                Label("\(c.attachments.count)", systemImage: "paperclip").font(.caption2)
                    .accessibilityLabel(StoryLogic.plural(c.attachments.count, "attachment"))
            }
            if t.characterCount, !c.characterIds.isEmpty {
                Label("\(c.characterIds.count)", systemImage: "person.2").font(.caption2)
                    .accessibilityLabel(StoryLogic.plural(c.characterIds.count, "character"))
            }
            if t.comments, c.commentCount > 0 {
                Label("\(Int(c.commentCount))", systemImage: "text.bubble").font(.caption2)
                    .accessibilityLabel("\(Int(c.commentCount)) open comment\(c.commentCount == 1 ? "" : "s")")
            }
        }
        .foregroundStyle(theme["muted"])
    }

    @ViewBuilder private func indicators(converted: Bool) -> some View {
        let hasHeading: Bool = { if case let .card(c) = item { !(c.sceneHeading ?? "").isEmpty } else { false } }()
        let linked: Bool = { if case let .card(c) = item { c.screenplaySceneId != nil } else { false } }()
        let comments: Int = {
            switch item {
            case let .card(c): Int(c.commentCount)
            case let .beat(b): Int(b.commentCount)
            case .sequence: 0
            }
        }()
        if converted || hasHeading || linked || comments > 0 {
            HStack(spacing: 8) {
                if converted { Text("Converted").font(.caption2.bold()) }
                if hasHeading { Text("H").font(.caption2.bold()).help("Has a scene heading").accessibilityLabel("Has a scene heading") }
                if linked {
                    Image(systemName: "doc.text").font(.caption2).help("Used to create a screenplay scene")
                        .accessibilityLabel("Used to create a screenplay scene")
                }
                if comments > 0 {
                    Label("\(comments)", systemImage: "text.bubble").font(.caption2)
                        .accessibilityLabel("\(comments) open comment\(comments == 1 ? "" : "s")")
                }
            }
            .foregroundStyle(theme["muted"])
        }
    }
}

// MARK: Sequence block

struct StorySequenceBlock: View {
    @Bindable var model: StoryModel
    let seq: StorySequenceDto
    let zoom: Double
    var focus: StoryFocus
    @Environment(\.ofTheme) private var theme
    @State private var headerSize = CGSize(width: 240, height: 30)
    @State private var bodySize = CGSize(width: 240, height: 60)

    var body: some View {
        let ref = StoryItemRef(kind: .sequence, id: seq.id)
        let key = ref.key
        let selected = model.selected.contains(key)
        let collapsed = model.collapsed.contains(seq.id)
        let renaming = model.renamingId == seq.id
        let item = StoryItem.sequence(seq)
        let shape = RoundedRectangle(cornerRadius: 10, style: .continuous)

        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: "line.3.horizontal").font(.caption).foregroundStyle(theme["muted"]).accessibilityHidden(true)
                Button { Task { await model.setCollapsed(seq.id, !collapsed) } } label: {
                    Image(systemName: collapsed ? "chevron.right" : "chevron.down").frame(width: 14)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(collapsed ? "Expand \(seq.title)" : "Collapse \(seq.title)")
                if renaming {
                    InlineField(initial: seq.title, label: "Sequence name",
                                onCommit: { v in Task { await model.commitText(ref, v) } },
                                onCancel: { model.cancelEditing() })
                } else {
                    Text(seq.title).fontWeight(.semibold).lineLimit(1)
                        .onTapGesture(count: 2) { model.renamingId = seq.id }
                }
                Spacer(minLength: 4)
                Text("\(Int(seq.cardCount))").font(.caption).foregroundStyle(theme["muted"])
            }
            .font(.system(size: 13 * zoom))
            .padding(.horizontal, 8).padding(.vertical, 6)
            .contentShape(Rectangle())
            .reading(size: $headerSize)
            .focusable()
            .focused(focus, equals: key)
            .focusEffectDisabled()
            .onKeyPress(phases: [.down, .repeat]) { press in renaming ? .ignored : model.handleKey(press, item: item) }
            .simultaneousGesture(TapGesture(count: 2).onEnded { model.openDrawer(ref) })
            .simultaneousGesture(TapGesture().onEnded {
                let flags = NSEvent.modifierFlags
                model.click(ref, extend: flags.contains(.command) || flags.contains(.shift))
            })
            .contextMenu { StoryItemMenu(model: model, item: item) }
            .onDrag {
                model.beginDrag(ref)
                return StoryDragType.provider()
            } preview: {
                Text(seq.title).padding(8).background(theme["panel"], in: shape)
            }
            .storyDrop(model, size: headerSize) { .beside(ref) }
            .accessibilityElement(children: .combine)
            .accessibilityLabel("Sequence \(seq.title), \(Int(seq.cardCount)) scenes")
            .accessibilityAddTraits(.isButton)
            .accessibilityIdentifier("story-sequence-\(seq.id)")
            .accessibilityAction(named: "Open") { model.openDrawer(ref) }
            .accessibilityAction(named: "Move Up") { Task { await model.step(ref, direction: -1) } }
            .accessibilityAction(named: "Move Down") { Task { await model.step(ref, direction: 1) } }

            if !collapsed {
                VStack(alignment: .leading, spacing: 6) {
                    StoryItemList(model: model, container: .sequence(seq.id), items: seq.items, zoom: zoom, focus: focus)
                    if seq.items.isEmpty {
                        Text("Drop scenes here.").font(.caption).foregroundStyle(theme["muted"])
                    }
                    Button { Task { await model.newCard(in: .sequence(seq.id)) } } label: { Label("Add scene", systemImage: "plus") }
                        .buttonStyle(.borderless).font(.caption)
                }
                .padding(.horizontal, 8).padding(.bottom, 8)
                .reading(size: $bodySize)
                .storyDrop(model, size: bodySize) { .into(.sequence(seq.id), atStart: false) }
            }
        }
        .background(selected ? theme["accent-soft"] : theme["surface2"], in: shape)
        .overlay(shape.strokeBorder(selected ? theme["accent"] : theme["line"], lineWidth: selected ? 2 : 1))
        .overlay {
            if let h = model.hint, h.target == .sequence(seq.id) {
                shape.strokeBorder(theme["accent"], style: StrokeStyle(lineWidth: 2, dash: [5, 3]))
            }
        }
    }
}

// MARK: List

/// The marker that shows where a drop will land.
struct StoryInsertionMarker: View {
    @Environment(\.ofTheme) private var theme
    var body: some View {
        Capsule().fill(theme["accent"]).frame(height: 3).accessibilityHidden(true)
    }
}

/// The items of one container with the insertion marker. Items that the filter hides are left out.
struct StoryItemList: View {
    @Bindable var model: StoryModel
    let container: StoryContainerRef
    let items: [StoryItem]
    let zoom: Double
    var focus: StoryFocus

    var body: some View {
        let hint = model.hint
        let here = hint?.target == container
        ForEach(items, id: \.ref) { item in
            if here, let b = hint?.before, b == item.ref { StoryInsertionMarker() }
            if StoryLogic.matchesFilter(item, model.filter) {
                switch item {
                case let .sequence(s): StorySequenceBlock(model: model, seq: s, zoom: zoom, focus: focus)
                default: StoryCardView(model: model, item: item, zoom: zoom, focus: focus)
                }
            }
        }
        if here, hint?.before == nil { StoryInsertionMarker() }
    }
}

/// VoiceOver actions of a card, beat or sequence: the same moves the menu and the keys offer.
struct CardActions: ViewModifier {
    let model: StoryModel
    let ref: StoryItemRef

    func body(content: Content) -> some View {
        content
            .accessibilityAction(named: "Open") { model.openDrawer(ref) }
            .accessibilityAction(named: "Move Up") { Task { await model.step(ref, direction: -1) } }
            .accessibilityAction(named: "Move Down") { Task { await model.step(ref, direction: 1) } }
            .accessibilityAction(named: "Move to Previous Act") { Task { await model.moveToAdjacentAct(ref, direction: -1) } }
            .accessibilityAction(named: "Move to Next Act") { Task { await model.moveToAdjacentAct(ref, direction: 1) } }
            .accessibilityAction(named: "Delete") { Task { await model.requestDelete(model.targets(for: ref)) } }
    }
}

// Moodboards: "What should this film feel like?" Named boards with a freeform canvas of image,
// note and link tiles. Tiles move (also as a multi-selection), resize and stack; the gesture is
// shown here while it runs and saved in the core as one undoable step. Arrow keys nudge the
// selection (8 points, 32 with Shift), which is the keyboard way to place a tile.
// Mirrors apps/desktop/src/workspaces/production/tabs/moodboards.tsx.

import AppKit
import OpenFrameCore
import SwiftUI

struct MoodboardsView: View {
    @Bindable var model: MoodboardsModel
    @Environment(AppModel.self) private var app
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            VisualPageHeader(title: "Moodboards", subtitle: VisualTab.moodboards.subtitle) {
                if let c = model.current {
                    Button { model.sheet = .export(c) } label: { Label("Export", systemImage: "square.and.arrow.up") }
                        .accessibilityIdentifier("moodboard-export")
                }
                Button { model.sheet = .newBoard } label: { Label("New Moodboard", systemImage: "plus") }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut("n", modifiers: [.command, .option])
                    .accessibilityIdentifier("moodboard-new")
            }
            content.frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task(id: app.changes.key(VisualTables.moodboard)) { await model.reload() }
        .sheet(item: $model.sheet) { MoodSheetHost(model: model, sheet: $0) }
        .inspector(isPresented: Binding(get: { model.inspectId != nil }, set: { if !$0 { model.inspectId = nil } })) {
            if let id = model.inspectId, let item = model.items.first(where: { $0.id == id }) {
                MoodItemInspector(model: model, item: item)
                    .id(item.id)
                    .inspectorColumnWidth(min: 300, ideal: 340, max: 460)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("moodboards-tab")
    }

    @ViewBuilder private var content: some View {
        if !model.loaded {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if model.boards.isEmpty, let f = model.loadFailure {
            ContentUnavailableView {
                Label("The moodboards could not be loaded.", systemImage: "exclamationmark.triangle")
            } description: { Text(f.message) } actions: {
                Button("Try Again") { Task { await model.reload() } }.accessibilityIdentifier("moodboard-retry")
            }
        } else if model.boards.isEmpty {
            ContentUnavailableView {
                Label("No moodboards yet", systemImage: "photo.on.rectangle")
            } description: {
                Text("Collect images, notes and links that show how the film should look and feel. Name a board anything, for example Overall Look, Cinematography or Costume.")
            } actions: {
                Button("New Moodboard") { model.sheet = .newBoard }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("moodboard-empty-new")
            }
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("moodboard-empty")
        } else {
            HStack(alignment: .top, spacing: 12) {
                MoodBoardList(model: model)
                MoodBoardCanvas(model: model).frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .padding(.horizontal, PageSpacing.medium).padding(.bottom, PageSpacing.medium)
        }
    }
}

// MARK: Board list

private struct MoodBoardList: View {
    @Bindable var model: MoodboardsModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                ForEach(Array(model.boards.enumerated()), id: \.element.id) { i, b in
                    let on = b.id == model.current?.id
                    Button { Task { await model.pick(b.id) } } label: {
                        HStack {
                            VStack(alignment: .leading, spacing: 1) {
                                Text(b.name).fontWeight(on ? .semibold : .regular).lineLimit(2)
                                if let n = b.sceneNumber { Text("Scene \(n) reference").font(.caption).foregroundStyle(theme["muted"]) }
                            }
                            Spacer()
                            Text("\(Int(b.itemCount))").font(.caption).foregroundStyle(theme["muted"])
                        }
                        .padding(.horizontal, 10).padding(.vertical, 8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(on ? theme["accent-soft"] : Color.clear)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Moodboard \(b.name), \(Int(b.itemCount)) items")
                    .accessibilityAddTraits(on ? .isSelected : [])
                    .accessibilityIdentifier("moodboard-row-\(b.id)")
                    .contextMenu {
                        Button("Rename\u{2026}") { model.sheet = .rename(b) }
                        Button("Board Notes\u{2026}") { model.sheet = .notes(b) }
                        Button("Move Up") { Task { await model.moveBoard(b, by: -1) } }.disabled(i == 0)
                        Button("Move Down") { Task { await model.moveBoard(b, by: 1) } }.disabled(i == model.boards.count - 1)
                        Divider()
                        Button("Delete\u{2026}", role: .destructive) { model.sheet = .deleteBoard(b) }
                    }
                    Divider()
                }
            }
        }
        .frame(width: 200)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Moodboards")
        .accessibilityIdentifier("moodboard-list")
    }
}

// MARK: Canvas

private struct MoodBoardCanvas: View {
    @Bindable var model: MoodboardsModel
    @Environment(\.ofTheme) private var theme
    @State private var dropTargeted = false
    @FocusState private var focused: Bool

    var body: some View {
        VStack(spacing: 8) {
            toolbar
            canvas
        }
    }

    private var toolbar: some View {
        CenteredWrap(spacing: 8, lineSpacing: PageSpacing.small) {
            Menu {
                Button("From File\u{2026}") { addFromFile() }
                    .accessibilityIdentifier("moodboard-add-image-file")
                Button("From Idea Vault\u{2026}") { model.sheet = .vault }
                    .accessibilityIdentifier("moodboard-add-image-vault")
            } label: { Label("Add Image", systemImage: "photo.badge.plus") }
                .menuStyle(.borderlessButton).fixedSize()
                .accessibilityIdentifier("moodboard-add-image")
            Button { model.sheet = .note } label: { Label("Add Note", systemImage: "note.text") }
                .accessibilityIdentifier("moodboard-add-note")
            Button { model.sheet = .link } label: { Label("Add Link", systemImage: "link") }
                .accessibilityIdentifier("moodboard-add-link")
            if let item = model.single {
                SelectionBar(model: model, item: item)
            } else if model.selected.count > 1 {
                Text("\(model.selected.count) items selected").font(.callout).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("moodboard-selection-count")
                Button { Task { await model.deleteSelected() } } label: { Label("Delete", systemImage: "trash") }
                    .accessibilityIdentifier("moodboard-delete-selected")
            } else {
                Text("Drag to move. Shift-click to select several. Arrow keys nudge. Paste images or links with Command V.")
                    .font(.caption).foregroundStyle(theme["muted"]).multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .controlSize(.small)
    }

    private func addFromFile() {
        let paths = VisualPanels.chooseImages(multiple: true, message: "Choose images for the moodboard")
        if !paths.isEmpty { Task { await model.addImages(paths: paths) } }
    }

    /// The canvas takes the room the page gives it; its scroll view would otherwise ask for the
    /// full height of the board and push the page out of the window.
    private var canvas: some View { canvasBody.fitsWindow() }

    private var canvasBody: some View {
        let extent = model.extent
        return ScrollView([.horizontal, .vertical]) {
            ZStack(alignment: .topLeading) {
                Color.clear
                    .contentShape(Rectangle())
                    .onTapGesture {
                        model.clearSelection()
                        focused = true
                    }
                if model.items.isEmpty {
                    Text("Drop images, notes and references here.")
                        .foregroundStyle(theme["muted"])
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .allowsHitTesting(false)
                }
                ForEach(model.items, id: \.id) { item in
                    if let box = model.box(item.id) {
                        MoodTile(model: model, item: item, box: box, selected: model.selected.contains(item.id))
                            .onTapGesture(count: 2) { Task { await open(item) } }
                            .onTapGesture {
                                let f = NSEvent.modifierFlags
                                model.click(item.id, additive: f.contains(.shift) || f.contains(.command))
                                focused = true
                            }
                    }
                }
            }
            .frame(width: max(extent.width, 600), height: max(extent.height, 400), alignment: .topLeading)
            .coordinateSpace(name: "mood-canvas")
            .dropDestination(for: URL.self) { urls, location in
                Task { await model.dropFiles(urls, at: CGPoint(x: location.x.rounded(), y: location.y.rounded())) }
                return !urls.isEmpty
            } isTargeted: { dropTargeted = $0 }
        }
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(dropTargeted ? theme["accent"] : theme["line"], style: StrokeStyle(lineWidth: dropTargeted ? 2 : 1, dash: dropTargeted ? [6] : [])))
        .focusable()
        .focused($focused)
        .focusEffectDisabled()
        .onKeyPress { press in handle(press) }
        .onPasteCommand(of: [.fileURL, .image, .png, .tiff, .url, .plainText]) { _ in
            Task { await model.paste() }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Moodboard canvas: \(model.current?.name ?? "")")
        .accessibilityIdentifier("moodboard-canvas")
    }

    private func open(_ item: MoodboardItemDto) async {
        if item.kind == "link", item.url != nil { await model.openLink(item) } else { model.inspectId = item.id }
    }

    private func handle(_ press: KeyPress) -> KeyPress.Result {
        guard !model.selected.isEmpty else { return .ignored }
        let step = press.modifiers.contains(.shift) ? 32.0 : 8.0
        switch press.key {
        case .leftArrow: Task { await model.nudge(dx: -step, dy: 0) }
        case .rightArrow: Task { await model.nudge(dx: step, dy: 0) }
        case .upArrow: Task { await model.nudge(dx: 0, dy: -step) }
        case .downArrow: Task { await model.nudge(dx: 0, dy: step) }
        case .delete, .deleteForward: Task { await model.deleteSelected() }
        case .escape: model.clearSelection()
        case .return:
            guard model.selected.count == 1, let id = model.selected.first else { return .ignored }
            model.inspectId = id
        default: return .ignored
        }
        return .handled
    }
}

// MARK: Selection bar

private struct SelectionBar: View {
    let model: MoodboardsModel
    let item: MoodboardItemDto

    var body: some View {
        HStack(spacing: 6) {
            CommitField(label: "Caption", value: item.caption ?? "", prompt: "Caption: why this matters", identifier: "moodboard-caption") { v in
                Task { await model.update(item, caption: v) }
            }
            .frame(width: 240)
            Button { Task { await model.arrange(item.id, placement: "front") } } label: { Image(systemName: "square.3.layers.3d.top.filled") }
                .accessibilityLabel("Bring to front").accessibilityIdentifier("moodboard-front")
            Button { Task { await model.arrange(item.id, placement: "back") } } label: { Image(systemName: "square.3.layers.3d.bottom.filled") }
                .accessibilityLabel("Send to back").accessibilityIdentifier("moodboard-back")
            Button { model.inspectId = item.id } label: { Image(systemName: "ellipsis.circle") }
                .accessibilityLabel("Edit item").accessibilityIdentifier("moodboard-edit-item")
            Button { Task { await model.deleteItems([item.id]) } } label: { Image(systemName: "trash") }
                .accessibilityLabel("Delete item").accessibilityIdentifier("moodboard-delete-item")
        }
    }
}

// MARK: Tile

private struct MoodTile: View {
    let model: MoodboardsModel
    let item: MoodboardItemDto
    let box: MoodBox
    let selected: Bool
    @Environment(\.ofTheme) private var theme

    var body: some View {
        tile
            .frame(width: box.w, alignment: .topLeading)
            .frame(minHeight: item.kind == "image" ? box.h : box.h, alignment: .topLeading)
            .background(background)
            .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(selected ? theme["accent"] : theme["line2"], lineWidth: selected ? 2 : 1))
            .clipShape(RoundedRectangle(cornerRadius: 4))
            .overlay(alignment: .bottomTrailing) { if selected { handle } }
            .shadow(color: .black.opacity(0.08), radius: 2, y: 1)
            .offset(x: box.x, y: box.y)
            .zIndex(item.z + (selected ? 0.5 : 0))
            .gesture(
                DragGesture(minimumDistance: 3, coordinateSpace: .named("mood-canvas"))
                    .onChanged { v in model.dragChanged(id: item.id, dx: v.translation.width, dy: v.translation.height) }
                    .onEnded { _ in Task { await model.dragEnded() } }
            )
            .contextMenu { menu }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(label)
            .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
            .accessibilityIdentifier("moodboard-tile-\(item.id)")
            .accessibilityAction(named: "Move right") { Task { await nudge(8, 0) } }
            .accessibilityAction(named: "Move left") { Task { await nudge(-8, 0) } }
            .accessibilityAction(named: "Move up") { Task { await nudge(0, -8) } }
            .accessibilityAction(named: "Move down") { Task { await nudge(0, 8) } }
            .accessibilityAction(named: "Bring to front") { Task { await model.arrange(item.id, placement: "front") } }
            .accessibilityAction(named: "Send to back") { Task { await model.arrange(item.id, placement: "back") } }
            .accessibilityAction(named: "Edit") { model.inspectId = item.id }
            .accessibilityAction(named: "Delete") { Task { await model.deleteItems([item.id]) } }
    }

    private func nudge(_ dx: Double, _ dy: Double) async {
        model.selected = [item.id]
        await model.nudge(dx: dx, dy: dy)
    }

    private var label: String {
        switch item.kind {
        case "image": "Image\(item.caption.map { ": \($0)" } ?? "")"
        case "note": "\(item.isPrivate ? "Internal note" : "Note"): \(item.body ?? "")"
        default: "Link: \(item.linkTitle ?? item.url ?? "")"
        }
    }

    private var background: Color {
        switch item.kind {
        case "note": item.isPrivate ? theme["blue-soft"] : theme["note-bg"]
        default: theme["panel"]
        }
    }

    @ViewBuilder private var tile: some View {
        switch item.kind {
        case "image":
            VStack(alignment: .leading, spacing: 0) {
                VisualAssetImage(asset: item.asset, fill: true, maxPixel: Int(max(box.w, box.h) * 2))
                    .frame(width: box.w, height: box.h)
                if let c = item.caption, !c.isEmpty {
                    Text(c).font(.caption).padding(6).frame(width: box.w, alignment: .leading)
                }
            }
        case "note":
            VStack(alignment: .leading, spacing: 4) {
                if item.isPrivate {
                    Label("Internal, left out of exports", systemImage: "lock").font(.caption2).foregroundStyle(theme["muted"])
                }
                Text(item.body ?? "").font(.callout)
                if let c = item.caption, !c.isEmpty { Text(c).font(.caption).foregroundStyle(theme["muted"]) }
            }
            .padding(9)
        default:
            VStack(alignment: .leading, spacing: 2) {
                Label(item.linkTitle ?? item.url ?? "", systemImage: "link").font(.callout.weight(.semibold)).lineLimit(1)
                Text("link \u{00B7} \(item.urlHost ?? "")").font(.caption).foregroundStyle(theme["muted"])
                if let c = item.caption, !c.isEmpty { Text(c).font(.caption) }
            }
            .padding(8)
        }
    }

    private var handle: some View {
        Rectangle()
            .fill(theme["accent"])
            .frame(width: 14, height: 14)
            .clipShape(RoundedRectangle(cornerRadius: 2))
            .contentShape(Rectangle().inset(by: -6))
            .gesture(
                DragGesture(minimumDistance: 1, coordinateSpace: .named("mood-canvas"))
                    .onChanged { v in
                        model.resizeChanged(id: item.id, dw: v.translation.width, dh: v.translation.height,
                                            keepAspect: item.kind == "image" && !NSEvent.modifierFlags.contains(.shift))
                    }
                    .onEnded { _ in Task { await model.resizeEnded(id: item.id) } }
            )
            .accessibilityLabel("Resize")
            .accessibilityIdentifier("moodboard-resize-\(item.id)")
    }

    @ViewBuilder private var menu: some View {
        let many = model.selected.count > 1 && model.selected.contains(item.id)
        Button("Edit\u{2026}") { model.inspectId = item.id }
        Button("Bring to Front") { Task { await model.arrange(item.id, placement: "front") } }
        Button("Send to Back") { Task { await model.arrange(item.id, placement: "back") } }
        if item.kind == "link", item.url != nil { Button("Open Link") { Task { await model.openLink(item) } } }
        Divider()
        Button(many ? "Delete \(model.selected.count) Items" : "Delete", role: .destructive) {
            Task { await model.deleteItems(many ? Array(model.selected) : [item.id]) }
        }
    }
}

// MARK: Item inspector

private struct MoodItemInspector: View {
    let model: MoodboardsModel
    let item: MoodboardItemDto
    @State private var bodyError: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    VStack(alignment: .leading) {
                        Text("MOODBOARD ITEM").font(.caption2).foregroundStyle(theme["muted"])
                        Text(kindLabel).font(.title3.weight(.semibold))
                    }
                    Spacer()
                    Button { model.inspectId = nil } label: { Image(systemName: "xmark") }
                        .buttonStyle(.borderless).accessibilityLabel("Close").accessibilityIdentifier("moodboard-inspector-close")
                }
                if item.kind == "image", let a = item.asset { Text(a.originalName).font(.caption).foregroundStyle(theme["muted"]) }
                if item.kind == "note" {
                    LabeledBlock(label: "Note", error: bodyError) {
                        CommitEditor(label: "Note", value: item.body ?? "", identifier: "moodboard-item-note", minHeight: 100) { v in
                            if v.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                                bodyError = "A note needs some text."
                            } else {
                                bodyError = nil
                                Task { await model.update(item, body: v) }
                            }
                        }
                    }
                    Toggle("Internal note, left out of exports unless selected", isOn: Binding(
                        get: { item.isPrivate }, set: { v in Task { await model.update(item, isPrivate: v) } }))
                        .accessibilityIdentifier("moodboard-item-private")
                }
                if item.kind == "link" {
                    LabeledBlock(label: "Web address") {
                        CommitField(label: "Web address", value: item.url ?? "", identifier: "moodboard-item-url") { v in
                            Task { await model.update(item, url: v) }
                        }
                        .textFieldStyle(.roundedBorder)
                    }
                    LabeledBlock(label: "Title") {
                        CommitField(label: "Title", value: item.linkTitle ?? "", identifier: "moodboard-item-title") { v in
                            Task { await model.update(item, linkTitle: v) }
                        }
                        .textFieldStyle(.roundedBorder)
                    }
                    if item.url != nil {
                        Button("Open Link") { Task { await model.openLink(item) } }.accessibilityIdentifier("moodboard-item-open-link")
                    }
                }
                LabeledBlock(label: "Caption", hint: "Optional. A short note on why this matters.") {
                    CommitField(label: "Caption", value: item.caption ?? "", identifier: "moodboard-item-caption") { v in
                        Task { await model.update(item, caption: String(v.prefix(300))) }
                    }
                    .textFieldStyle(.roundedBorder)
                }
                if item.sourceVaultItemId != nil {
                    Text("Copied from the Idea Vault. Changes here don't affect the vault.").font(.caption).foregroundStyle(theme["muted"])
                }
            }
            .padding(16)
        }
        .accessibilityIdentifier("moodboard-inspector")
    }

    private var kindLabel: String { item.kind == "image" ? "Image" : item.kind == "note" ? "Note" : "Link" }
}

// MARK: Sheets

private struct MoodSheetHost: View {
    let model: MoodboardsModel
    let sheet: MoodSheet
    @Environment(VisualHub.self) private var hub

    var body: some View {
        switch sheet {
        case .newBoard: NewMoodboardSheet(model: model)
        case let .rename(b): RenameMoodboardSheet(model: model, board: b)
        case let .notes(b): BoardNotesSheet(model: model, board: b)
        case let .deleteBoard(b): DeleteMoodboardSheet(model: model, board: b)
        case .note: AddNoteSheet(model: model)
        case .link: AddLinkSheet(model: model)
        case .vault: VaultImagesSheet(model: model)
        case let .export(b): MoodboardExportSheet(model: model, board: b)
        }
    }
}

private struct NewMoodboardSheet: View {
    let model: MoodboardsModel
    @Environment(VisualHub.self) private var hub
    @State private var name = ""
    @State private var sceneId = ""
    @State private var error: String?

    var body: some View {
        VisualFormSheet(title: "New Moodboard", primary: "Create Moodboard", identifier: "moodboard-new-sheet", onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Board name", error: error) {
                TextField("Board name", text: $name, prompt: Text("e.g. Overall Look"))
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(submit)
                    .onChange(of: name) { _, _ in error = nil }
                    .accessibilityIdentifier("mb-name")
            }
            if !model.suggestions.isEmpty {
                LabeledBlock(label: "Suggestions. Use any name you like") {
                    VisualFlowRow(items: model.suggestions) { s in
                        Button(s) { name = s }
                            .buttonStyle(.bordered).controlSize(.small)
                            .accessibilityIdentifier("moodboard-suggestion-\(s)")
                    }
                }
            }
            if let scenes = hub.scenes?.scenes, !scenes.isEmpty {
                LabeledBlock(label: "Reference for a scene", hint: "Optional.") {
                    Picker("Reference for a scene", selection: $sceneId) {
                        Text("No particular scene").tag("")
                        ForEach(scenes, id: \.lineageId) { Text(VisualText.sceneLabel($0)).tag($0.sceneId) }
                    }
                    .labelsHidden()
                    .accessibilityIdentifier("mb-scene")
                }
            }
        }
        .task { await model.loadSuggestions() }
    }

    private func submit() {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else {
            error = "Give the board a name."
            return
        }
        Task { if await model.createBoard(name: name, sceneId: sceneId.isEmpty ? nil : sceneId) { model.sheet = nil } }
    }
}

private struct RenameMoodboardSheet: View {
    let model: MoodboardsModel
    let board: MoodboardSummary
    @State private var name = ""

    var body: some View {
        VisualFormSheet(title: "Rename moodboard", primary: "Rename", primaryEnabled: !name.trimmingCharacters(in: .whitespaces).isEmpty,
                        identifier: "moodboard-rename-sheet", width: 380, onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Board name") {
                TextField("Board name", text: $name).textFieldStyle(.roundedBorder).onSubmit(submit)
                    .accessibilityIdentifier("mb-rename")
            }
        }
        .onAppear { name = board.name }
    }

    private func submit() {
        Task { if await model.rename(board, to: name) { model.sheet = nil } }
    }
}

private struct BoardNotesSheet: View {
    let model: MoodboardsModel
    let board: MoodboardSummary
    @State private var notes = ""
    @State private var edited = false
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VisualFormSheet(title: "Board notes: \(board.name)", primary: "Save Notes", primaryEnabled: edited, identifier: "moodboard-notes-sheet",
                        onCancel: { model.sheet = nil }, onPrimary: { Task { if await model.saveNotes(board, notes: notes) { model.sheet = nil } } }) {
            LabeledBlock(label: "Internal notes", hint: "For the team only. Left out of standard exports.") {
                TextEditor(text: Binding(get: { notes }, set: { notes = $0; edited = true }))
                    .frame(height: 140)
                    .padding(4)
                    .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line2"]))
                    .accessibilityLabel("Internal notes")
                    .accessibilityIdentifier("mb-notes")
            }
            Label("Internal", systemImage: "lock").font(.caption).foregroundStyle(theme["muted"])
        }
        .task { notes = await model.boardNotes(board) }
    }
}

private struct DeleteMoodboardSheet: View {
    let model: MoodboardsModel
    let board: MoodboardSummary

    var body: some View {
        let n = Int(board.itemCount)
        VisualFormSheet(title: "Delete moodboard \u{201C}\(board.name)\u{201D}?", primary: "Delete Moodboard", identifier: "moodboard-delete-sheet",
                        onCancel: { model.sheet = nil }, onPrimary: { Task { if await model.deleteBoard(board) { model.sheet = nil } } }) {
            Text("The board and its \(VisualText.plural(n, "item")) go to Recently Deleted and can be restored.")
        }
    }
}

private struct AddNoteSheet: View {
    let model: MoodboardsModel
    @State private var text = ""
    @State private var isPrivate = false
    @State private var error: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VisualFormSheet(title: "Add note", primary: "Add Note", identifier: "moodboard-note-sheet", onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Note", error: error) {
                TextEditor(text: $text)
                    .frame(height: 90)
                    .padding(4)
                    .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line2"]))
                    .onChange(of: text) { _, _ in error = nil }
                    .accessibilityLabel("Note")
                    .accessibilityIdentifier("new-note")
            }
            Toggle("Internal note, left out of exports unless selected", isOn: $isPrivate)
                .accessibilityIdentifier("new-note-private")
        }
    }

    private func submit() {
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            error = "Write the note first."
            return
        }
        Task { if await model.addNote(text: text, isPrivate: isPrivate) { model.sheet = nil } }
    }
}

private struct AddLinkSheet: View {
    let model: MoodboardsModel
    @State private var url = ""
    @State private var title = ""

    var body: some View {
        VisualFormSheet(title: "Add link", primary: "Add Link", primaryEnabled: !url.trimmingCharacters(in: .whitespaces).isEmpty,
                        identifier: "moodboard-link-sheet", onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Web address") {
                TextField("Web address", text: $url, prompt: Text("https://\u{2026}")).textFieldStyle(.roundedBorder)
                    .accessibilityIdentifier("new-link")
            }
            LabeledBlock(label: "Title", hint: "Optional, for example Reference: Memories of Murder") {
                TextField("Title", text: $title).textFieldStyle(.roundedBorder).onSubmit(submit)
                    .accessibilityIdentifier("new-link-title")
            }
        }
    }

    private func submit() {
        guard !url.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        Task { if await model.addLink(url: url, title: title) { model.sheet = nil } }
    }
}

private struct VaultImagesSheet: View {
    @Bindable var model: MoodboardsModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Add image from the Idea Vault").font(.title3.weight(.semibold))
            HStack {
                Picker("Idea Vault", selection: $model.vaultStore) {
                    Text("Project Idea Vault").tag(StoreSel.project)
                    Text("Global Idea Vault").tag(StoreSel.global)
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize()
                .accessibilityLabel("Idea Vault")
                .accessibilityIdentifier("mb-vault-store")
                Text("The image is copied onto the board; the vault item is not changed.").font(.caption).foregroundStyle(theme["muted"])
            }
            if model.vaultLoading {
                ProgressView().frame(maxWidth: .infinity, minHeight: 120)
            } else if model.vaultImages.isEmpty {
                Text("There are no images in this Idea Vault yet.").foregroundStyle(theme["muted"])
                    .frame(maxWidth: .infinity, minHeight: 120)
                    .accessibilityIdentifier("mb-vault-empty")
            } else {
                ScrollView {
                    LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 4), spacing: 10) {
                        ForEach(model.vaultImages, id: \.vaultItemId) { v in
                            Button {
                                Task { if await model.addVaultImage(v) { model.sheet = nil } }
                            } label: {
                                VStack(alignment: .leading, spacing: 4) {
                                    VisualAssetImage(asset: v.asset, maxPixel: 320).frame(height: 90).clipShape(RoundedRectangle(cornerRadius: 4))
                                    Text(v.title).font(.caption).lineLimit(2)
                                }
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel("Add \(v.title)")
                            .accessibilityIdentifier("mb-vault-\(v.vaultItemId)")
                        }
                    }
                }
                .frame(height: 280)
            }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { model.sheet = nil }
                    .keyboardShortcut(.cancelAction).accessibilityIdentifier("mb-vault-cancel")
            }
        }
        .padding(20).frame(width: 620)
        .task(id: model.vaultStore) { await model.loadVaultImages() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("moodboard-vault-sheet")
    }
}

private struct MoodboardExportSheet: View {
    let model: MoodboardsModel
    let board: MoodboardSummary

    var body: some View {
        VisualExportSheet(
            title: "Export moodboard: \(board.name)",
            note: "Private and internal notes are left out unless you tick Internal notes. Export creates a snapshot and does not change your project.",
            documentName: "Moodboard - \(board.name)", formats: [.pdf],
            options: [
                VisualExportOption(key: "includeTitle", label: "Project title and board name", isOn: true),
                VisualExportOption(key: "includeImages", label: "Images", isOn: true),
                VisualExportOption(key: "includeCaptions", label: "Captions", isOn: true),
                VisualExportOption(key: "includeInternalNotes", label: "Internal notes", isOn: false),
            ],
            identifier: "moodboard-export-sheet", onCancel: { model.sheet = nil }
        ) { req in
            await model.export(board, path: req.path, includeTitle: req.flags["includeTitle"] ?? true, includeImages: req.flags["includeImages"] ?? true,
                               includeCaptions: req.flags["includeCaptions"] ?? true, includeInternalNotes: req.flags["includeInternalNotes"] ?? false)
        }
    }
}

/// Buttons laid out left to right, wrapping to new lines.
struct VisualFlowRow<Item: Hashable, Content: View>: View {
    let items: [Item]
    @ViewBuilder var content: (Item) -> Content

    var body: some View {
        VisualFlowLayout(spacing: 6) {
            ForEach(items, id: \.self) { content($0) }
        }
    }
}

struct VisualFlowLayout: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 400
        var x: CGFloat = 0, y: CGFloat = 0, rowH: CGFloat = 0, maxX: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x + s.width > width, x > 0 { x = 0; y += rowH + spacing; rowH = 0 }
            x += s.width + spacing
            rowH = max(rowH, s.height)
            maxX = max(maxX, x - spacing)
        }
        return CGSize(width: maxX, height: y + rowH)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowH: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x + s.width > bounds.maxX, x > bounds.minX { x = bounds.minX; y += rowH + spacing; rowH = 0 }
            v.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(s))
            x += s.width + spacing
            rowH = max(rowH, s.height)
        }
    }
}

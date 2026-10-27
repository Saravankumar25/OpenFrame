// Storyboards: "What will the audience see?" Ordered panels for a scene (or a standalone board).
// Panel numbers come from order; a linked shot shows its generated label (12A). Drag a panel onto
// another panel's place to reorder it, or use Move Earlier and Move Later (menu, inspector or
// the accessibility actions): the linked shots follow in the core.
// Mirrors apps/desktop/src/workspaces/production/tabs/storyboards.tsx.

import AppKit
import OpenFrameCore
import SwiftUI

struct StoryboardsView: View {
    @Bindable var model: StoryboardsModel
    @Environment(AppModel.self) private var app
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private var dataKey: Int { app.changes.key(VisualTables.storyboard) &+ (hub.scenes?.scenes.count ?? 0) &* 7919 }

    var body: some View {
        VStack(spacing: 0) {
            header
            content
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .task(id: "\(dataKey)|\(hub.scenesLoaded)") { await model.reload() }
        .themedSheet(item: $model.sheet) { StoryboardSheetHost(model: model, sheet: $0) }
        .inspector(isPresented: Binding(get: { model.panel != nil }, set: { if !$0 { model.selectedPanelId = nil } })) {
            if let p = model.panel, let b = model.board {
                PanelInspector(model: model, panel: p, board: b.board).id(p.id)
                    .inspectorColumnWidth(min: 320, ideal: 380, max: 500)
            }
        }
        .onPasteCommand(of: [.image, .png, .tiff]) { _ in
            let pb = NSPasteboard.general
            if let data = pb.data(forType: .png) ?? pb.data(forType: .tiff) { Task { await model.addPasted(image: data) } }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("storyboards-tab")
    }

    // MARK: Header and toolbar

    private var header: some View {
        VStack(spacing: 0) {
            VisualPageHeader(title: "Storyboards", subtitle: VisualTab.storyboards.subtitle) {
                Button { model.sheet = .export } label: { Label("Export", systemImage: "square.and.arrow.up") }
                    .disabled(model.boards.isEmpty)
                    .accessibilityIdentifier("storyboard-export")
                Button("New Storyboard") { model.sheet = .newBoard }
                    .accessibilityIdentifier("storyboard-new")
                Menu {
                    addMenu
                } label: { Label("Add Panel", systemImage: "plus") }
                    .menuStyle(.borderedButton).fixedSize()
                    .disabled(!model.canAdd)
                    .accessibilityIdentifier("storyboard-add-panel")
            }
            if hasPicker { toolbar }
        }
    }

    private var hasPicker: Bool { hub.hasScenes || !model.boards.isEmpty }

    @ViewBuilder private var addMenu: some View {
        Button { Task { await model.importImagePanel() } } label: { Label("Import Image", systemImage: "photo.badge.plus") }
            .accessibilityIdentifier("storyboard-add-image")
        Button { model.sheet = .sketch(panelId: nil) } label: { Label("Draw / Sketch", systemImage: "pencil.tip") }
            .accessibilityIdentifier("storyboard-add-sketch")
        Button { Task { await model.addPanel(.placeholder) } } label: { Label("Empty Panel", systemImage: "square.dashed") }
            .accessibilityIdentifier("storyboard-add-empty")
    }

    private var toolbar: some View {
        let sel = model.selection
        return CenteredWrap(spacing: 10, lineSpacing: PageSpacing.small) {
            Picker("Scene or storyboard", selection: Binding(get: { sel.key ?? "" }, set: { k in Task { await model.choose(key: k) } })) {
                if let scenes = hub.scenes?.scenes, !scenes.isEmpty {
                    Section(hub.scenes?.sourceLabel.isEmpty == false ? hub.scenes!.sourceLabel : "Scenes") {
                        ForEach(scenes, id: \.lineageId) {
                            Text(VisualText.sceneLabel($0) + ($0.needsReview ? ", review" : "")).tag("scene:\($0.lineageId)")
                        }
                    }
                }
                if !model.removedBoards.isEmpty {
                    Section("Removed from script") { ForEach(model.removedBoards, id: \.id) { Text($0.name).tag("board:\($0.id)") } }
                }
                if !model.otherBoards.isEmpty {
                    Section("Other storyboards") { ForEach(model.otherBoards, id: \.id) { Text($0.name).tag("board:\($0.id)") } }
                }
            }
            .labelsHidden()
            .frame(maxWidth: 440)
            .accessibilityLabel("Scene or storyboard")
            .accessibilityIdentifier("storyboard-scene-picker")
            if model.sceneBoards.count > 1 {
                Picker("Storyboard for this scene", selection: Binding(get: { sel.boardId ?? "" }, set: { id in Task { await model.chooseSceneBoard(id) } })) {
                    ForEach(model.sceneBoards, id: \.id) { Text($0.name).tag($0.id) }
                }
                .labelsHidden().fixedSize()
                .accessibilityLabel("Storyboard for this scene")
                .accessibilityIdentifier("storyboard-board-picker")
            }
            Text(VisualText.plural(model.panels.count, "panel"))
                .font(.caption).padding(.horizontal, 8).padding(.vertical, 3)
                .background(theme["surface2"], in: Capsule())
                .accessibilityIdentifier("storyboard-panel-count")
            if let b = model.board?.board { BoardMenu(model: model, board: b) }
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, PageSpacing.medium).padding(.bottom, PageSpacing.small)
    }

    // MARK: Content

    @ViewBuilder private var content: some View {
        if !model.loaded || !hub.scenesLoaded {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if model.boards.isEmpty, let f = model.loadFailure {
            ContentUnavailableView {
                Label("The storyboards could not be loaded.", systemImage: "exclamationmark.triangle")
            } description: { Text(f.message) } actions: {
                Button("Try Again") { Task { await model.reload() } }.accessibilityIdentifier("storyboard-retry")
            }
        } else if !hub.hasScenes && model.boards.isEmpty {
            ContentUnavailableView {
                Label("No storyboards yet", systemImage: "rectangle.split.3x1")
            } description: {
                Text("Storyboards usually belong to a screenplay scene, so its heading comes from the script. You can also start a standalone storyboard now. The screenplay never needs shot directions.")
            } actions: {
                Button("New Storyboard") { model.sheet = .newBoard }
                    .buttonStyle(.ofProminent)
                    .accessibilityIdentifier("storyboard-empty-new")
            }
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("storyboard-empty")
        } else {
            ScrollView {
                VStack(spacing: PageSpacing.small) {
                    if let scene = model.selection.scene { SceneReviewBanner(hub: hub, scene: scene, what: "storyboard panels and shots").padding(.horizontal, -16) }
                    if let b = model.board?.board, b.sceneRemoved { RemovedSceneBanner(heading: b.sceneHeading ?? "", what: "storyboard panels").padding(.horizontal, -16) }
                    if model.panels.isEmpty {
                        emptyPanels
                    } else {
                        PanelGrid(model: model)
                    }
                }
                .pageColumn(maxWidth: PageSpacing.wideColumn)
                .padding(.bottom, PageSpacing.medium)
            }
            .frame(minHeight: 0)
        }
    }

    private var emptyPanels: some View {
        ContentUnavailableView {
            Label("Create panels for this scene.", systemImage: "rectangle.split.3x1")
        } description: {
            Text("A panel needs only a visual area and a short description. Link panels to shots whenever you like. Neither is required.")
        } actions: {
            if model.canAdd {
                HStack {
                    Button { Task { await model.importImagePanel() } } label: { Label("Import Image", systemImage: "photo.badge.plus") }
                        .accessibilityIdentifier("storyboard-empty-image")
                    Button { model.sheet = .sketch(panelId: nil) } label: { Label("Draw / Sketch", systemImage: "pencil.tip") }
                        .accessibilityIdentifier("storyboard-empty-sketch")
                    Button { Task { await model.addPanel(.placeholder) } } label: { Label("Empty Panel", systemImage: "square.dashed") }
                        .accessibilityIdentifier("storyboard-empty-panel")
                }
                .fixedSize()
            }
        }
        .frame(minHeight: 260)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("storyboard-no-panels")
    }
}

// MARK: Panel grid

private struct PanelGrid: View {
    @Bindable var model: StoryboardsModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(minimum: 160), spacing: 12), count: 3), spacing: 12) {
                ForEach(Array(model.panels.enumerated()), id: \.element.id) { i, p in
                    PanelCard(model: model, panel: p, number: i + 1, count: model.panels.count)
                }
                Menu {
                    Button("Import Image") { Task { await model.importImagePanel() } }.accessibilityIdentifier("storyboard-grid-add-image")
                    Button("Draw / Sketch") { model.sheet = .sketch(panelId: nil) }.accessibilityIdentifier("storyboard-grid-add-sketch")
                    Button("Empty Panel") { Task { await model.addPanel(.placeholder) } }.accessibilityIdentifier("storyboard-grid-add-empty")
                } label: {
                    VStack(spacing: 4) { Image(systemName: "plus"); Text("Add Panel").fontWeight(.semibold) }
                        .frame(maxWidth: .infinity, minHeight: 140)
                        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line2"], style: StrokeStyle(lineWidth: 1, dash: [5])))
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden)
                .accessibilityLabel("Add Panel")
                .accessibilityIdentifier("storyboard-grid-add")
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Storyboard panels")
            .accessibilityIdentifier("storyboard-panels")
            Text("Drag panels to change the order. Panel numbers\(model.board?.board.sceneLineageId != nil ? " and linked shot numbers" : "") follow the order automatically. Paste an image (Command V) to add it as a panel.")
                .font(.caption).foregroundStyle(theme["muted"])
        }
    }
}

private struct PanelCard: View {
    let model: StoryboardsModel
    let panel: PanelDto
    let number: Int
    let count: Int
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme
    @State private var targeted = false

    var body: some View {
        let selected = model.selectedPanelId == panel.id
        VStack(alignment: .leading, spacing: 0) {
            ZStack(alignment: .topLeading) {
                VisualAssetImage(asset: panel.asset, maxPixel: 480,
                                 missingText: "Image unavailable",
                                 emptyText: panel.visualKind == "placeholder" ? "Empty panel" : "Image unavailable")
                    .frame(height: 132)
                Text("\(number)").font(.caption.weight(.bold)).padding(.horizontal, 6).padding(.vertical, 2)
                    .background(theme["ink"], in: RoundedRectangle(cornerRadius: 4)).foregroundStyle(theme["panel"]).padding(6)
                if let label = panel.shotLabel, let shotId = panel.shotId {
                    HStack {
                        Spacer()
                        Button(label) { Task { await hub.openShot(shotId) } }
                            .buttonStyle(.plain).font(.caption.weight(.semibold))
                            .padding(.horizontal, 6).padding(.vertical, 2)
                            .background(theme["ink"], in: Capsule()).foregroundStyle(theme["panel"])
                            .help("Open shot \(label)")
                            .accessibilityLabel("Open shot \(label)")
                            .accessibilityIdentifier("panel-shot-chip-\(panel.id)")
                    }
                    .padding(6)
                }
            }
            Text(panel.description.isEmpty ? "Add a short description" : panel.description)
                .font(.callout).lineLimit(2)
                .foregroundStyle(panel.description.isEmpty ? theme["muted"] : theme["ink"])
                .padding(8).frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(selected ? theme["accent"] : (targeted ? theme["accent"] : theme["line"]), lineWidth: selected || targeted ? 2 : 1))
        .contentShape(RoundedRectangle(cornerRadius: 8))
        .onTapGesture { model.selectedPanelId = panel.id }
        .draggable(panel.id) {
            Text("Panel \(number)").padding(6).background(theme["panel"], in: RoundedRectangle(cornerRadius: 4))
        }
        .dropDestination(for: String.self) { ids, _ in
            guard let id = ids.first, model.panels.contains(where: { $0.id == id }), id != panel.id else { return false }
            Task { await model.reorder(id, to: number - 1) }
            return true
        } isTargeted: { targeted = $0 }
        .contextMenu {
            Button("Open") { model.selectedPanelId = panel.id }
            Button("Move Earlier") { Task { await model.nudge(panel.id, by: -1) } }.disabled(number == 1)
            Button("Move Later") { Task { await model.nudge(panel.id, by: 1) } }.disabled(number == count)
            Button("Move to Storyboard\u{2026}") { model.sheet = .movePanel(panel) }.disabled(model.boards.count < 2)
            Button("Link Shot\u{2026}") { model.selectedPanelId = panel.id }
            Divider()
            Button("Delete", role: .destructive) { Task { await model.deletePanels([panel.id]) } }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Panel \(number), \(panel.visualKind == "placeholder" ? "empty" : "picture")\(panel.shotLabel.map { ", shot \($0)" } ?? "")\(panel.description.isEmpty ? "" : ": \(panel.description)")")
        .accessibilityHint("Press to open. Use the actions to move it.")
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction(named: "Move earlier") { Task { await model.nudge(panel.id, by: -1) } }
        .accessibilityAction(named: "Move later") { Task { await model.nudge(panel.id, by: 1) } }
        .accessibilityAction(named: "Delete") { Task { await model.deletePanels([panel.id]) } }
        .accessibilityIdentifier("panel-\(panel.id)")
    }
}

// MARK: Board menu

private struct BoardMenu: View {
    let model: StoryboardsModel
    let board: StoryboardSummary
    @Environment(VisualHub.self) private var hub

    var body: some View {
        Menu {
            Button("Rename\u{2026}") { model.sheet = .rename(board) }
            Button(board.sceneLineageId != nil ? "Change Scene\u{2026}" : "Link to a Scene\u{2026}") { model.sheet = .linkScene(board) }
                .disabled(!hub.hasScenes)
            if board.sceneLineageId != nil {
                Button("Make Standalone") { Task { _ = await model.setScene(board, sceneId: nil) } }
            }
            if board.sceneLineageId != nil, !board.sceneRemoved {
                Button("Another Storyboard for This Scene\u{2026}") { model.sheet = .newBoard }
            }
            Divider()
            Button("Delete Storyboard\u{2026}", role: .destructive) { model.sheet = .deleteBoard(board) }
        } label: { Label("Actions", systemImage: "ellipsis.circle") }
            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
            .accessibilityLabel("Storyboard actions")
            .accessibilityIdentifier("storyboard-actions")
    }
}

// MARK: Panel inspector

private struct PanelInspector: View {
    let model: StoryboardsModel
    let panel: PanelDto
    let board: StoryboardSummary
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme
    @State private var durText = ""
    @State private var durError: String?
    @State private var shotScene = ""

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    VStack(alignment: .leading) {
                        Text("STORYBOARD PANEL").font(.caption2).foregroundStyle(theme["muted"])
                        Text("Panel \(Int(panel.number))\(panel.shotLabel.map { ", Shot \($0)" } ?? "")").font(.title3.weight(.semibold))
                    }
                    Spacer()
                    Button { model.selectedPanelId = nil } label: { Label("Close", systemImage: "xmark") }
                        .buttonStyle(.borderless).accessibilityLabel("Close").accessibilityIdentifier("panel-inspector-close")
                }
                VisualAssetImage(asset: panel.asset, fill: false, maxPixel: 720, emptyText: "Empty panel")
                    .frame(height: 170).clipShape(RoundedRectangle(cornerRadius: 6))
                LabeledBlock(label: "Description", hint: "A short description of what we see.") {
                    CommitEditor(label: "Description", value: panel.description, identifier: "panel-desc") { v in
                        Task { await model.update(panel, description: v) }
                    }
                }
                LabeledBlock(label: "Visual") {
                    HStack {
                        Button("Import Image\u{2026}") { Task { await model.chooseVisual(for: panel) } }
                            .accessibilityIdentifier("panel-import-image")
                        Button("Draw / Sketch\u{2026}") { model.sheet = .sketch(panelId: panel.id) }
                            .accessibilityIdentifier("panel-sketch")
                        if panel.visualKind != "placeholder" {
                            Button("Clear") { Task { _ = await model.setVisual(panel, .placeholder) } }
                                .accessibilityIdentifier("panel-clear-visual")
                        }
                    }
                    .controlSize(.small).disabled(model.busy)
                }
                HStack(alignment: .top, spacing: 8) {
                    LabeledBlock(label: "Framing") {
                        CommitField(label: "Framing", value: panel.framing ?? "", identifier: "panel-framing", suggestions: VisualText.shotSizes) { v in
                            Task { await model.update(panel, framing: v) }
                        }.textFieldStyle(.roundedBorder)
                    }
                    LabeledBlock(label: "Camera movement") {
                        CommitField(label: "Camera movement", value: panel.movement ?? "", identifier: "panel-movement", suggestions: VisualText.movements) { v in
                            Task { await model.update(panel, movement: v) }
                        }.textFieldStyle(.roundedBorder)
                    }
                }
                HStack(alignment: .top, spacing: 8) {
                    LabeledBlock(label: "Angle") {
                        CommitField(label: "Angle", value: panel.angle ?? "", identifier: "panel-angle", suggestions: VisualText.angles) { v in
                            Task { await model.update(panel, angle: v) }
                        }.textFieldStyle(.roundedBorder)
                    }
                    LabeledBlock(label: "Duration (seconds)", error: durError) {
                        CommitField(label: "Duration", value: VisualText.formatSeconds(panel.durationMs), prompt: "e.g. 3.5", identifier: "panel-duration") { v in
                            guard let ms = VisualText.parseSeconds(v) else {
                                durError = "Enter seconds between 0 and 3600."
                                return
                            }
                            durError = nil
                            if ms != (panel.durationMs ?? 0) { Task { await model.update(panel, durationMs: ms) } }
                        }.textFieldStyle(.roundedBorder)
                    }
                }
                LabeledBlock(label: "Dialogue / sound note") {
                    CommitField(label: "Dialogue / sound note", value: panel.soundNote ?? "", prompt: "e.g. Phone rings off screen", identifier: "panel-sound") { v in
                        Task { await model.update(panel, soundNote: v) }
                    }.textFieldStyle(.roundedBorder)
                }
                LabeledBlock(label: "Storyboard note") {
                    CommitEditor(label: "Storyboard note", value: panel.note ?? "", identifier: "panel-note") { v in
                        Task { await model.update(panel, note: v) }
                    }
                }
                shotBlock
                Divider()
                HStack {
                    Button("Move Earlier") { Task { await model.nudge(panel.id, by: -1) } }
                        .disabled(Int(panel.number) <= 1).accessibilityIdentifier("panel-move-earlier")
                    Button("Move Later") { Task { await model.nudge(panel.id, by: 1) } }
                        .disabled(Int(panel.number) >= model.panels.count).accessibilityIdentifier("panel-move-later")
                    Spacer()
                    Button("Delete", role: .destructive) { Task { await model.deletePanels([panel.id]) } }
                        .accessibilityIdentifier("panel-delete")
                }
                .controlSize(.small)
            }
            .padding(16)
        }
        .onAppear { shotScene = hub.scenes?.scenes.first?.sceneId ?? "" }
        .accessibilityIdentifier("panel-inspector")
    }

    @ViewBuilder private var shotBlock: some View {
        LabeledBlock(label: "Shot") {
            if let shotId = panel.shotId, let label = panel.shotLabel {
                HStack {
                    Button("Shot \(label)") { Task { await hub.openShot(shotId) } }
                        .accessibilityIdentifier("panel-open-shot")
                    Button("Unlink") { Task { await model.linkShot(panel, shotId: nil) } }
                        .accessibilityIdentifier("panel-unlink-shot")
                }
            } else if model.activeLineage != nil {
                HStack {
                    Menu("Link Shot\u{2026}") {
                        ForEach(model.sceneShots, id: \.id) { s in
                            Button("\(s.label): \(s.description)") { Task { await model.linkShot(panel, shotId: s.id) } }
                        }
                    }
                    .fixedSize().disabled(model.sceneShots.isEmpty)
                    .accessibilityIdentifier("panel-link-shot")
                    Button("Create Shot") { Task { await model.createShot(from: panel, sceneId: nil) } }
                        .accessibilityIdentifier("panel-create-shot")
                }
            } else if let scenes = hub.scenes?.scenes, !scenes.isEmpty {
                HStack {
                    Picker("Scene for the new shot", selection: $shotScene) {
                        ForEach(scenes, id: \.lineageId) { Text(VisualText.sceneLabel($0)).tag($0.sceneId) }
                    }
                    .labelsHidden().frame(maxWidth: 240)
                    .accessibilityLabel("Scene for the new shot")
                    .accessibilityIdentifier("panel-shot-scene")
                    Button("Create Shot") { Task { await model.createShot(from: panel, sceneId: shotScene.isEmpty ? nil : shotScene) } }
                        .accessibilityIdentifier("panel-create-shot")
                }
            } else {
                Text("Shots belong to screenplay scenes. Add a screenplay to plan shots.").font(.caption).foregroundStyle(theme["muted"])
            }
        }
    }
}

// MARK: Sheets

private struct StoryboardSheetHost: View {
    let model: StoryboardsModel
    let sheet: StoryboardSheet
    @Environment(VisualHub.self) private var hub

    var body: some View {
        switch sheet {
        case .newBoard: NewStoryboardSheet(model: model)
        case let .rename(b): RenameStoryboardSheet(model: model, board: b)
        case let .linkScene(b):
            PickSceneSheet(title: "Link storyboard to a scene", scenes: hub.scenes?.scenes ?? [], initial: b.sceneId, confirm: "Link Scene",
                           identifier: "storyboard-link-scene-sheet", onCancel: { model.sheet = nil }) { id in
                Task { if await model.setScene(b, sceneId: id) { model.sheet = nil } }
            }
        case let .deleteBoard(b):
            VisualFormSheet(title: "Delete storyboard \u{201C}\(b.name)\u{201D}?", primary: "Delete Storyboard", identifier: "storyboard-delete-sheet",
                            onCancel: { model.sheet = nil }, onPrimary: { Task { if await model.deleteBoard(b) { model.sheet = nil } } }) {
                Text("Its \(VisualText.plural(Int(b.panelCount), "panel")) go to Recently Deleted with it and can be restored. Shots are not affected.")
            }
        case let .movePanel(p): MovePanelSheet(model: model, panel: p)
        case let .sketch(panelId):
            SketchSheet(saving: model.busy, onSave: { png in
                Task {
                    if let panelId, let p = model.panels.first(where: { $0.id == panelId }) {
                        _ = await model.setVisual(p, .sketch, png: png)
                    } else {
                        await model.addPanel(.sketch, png: png)
                    }
                }
            }, onCancel: { model.sheet = nil })
        case .export: StoryboardExportSheet(model: model)
        }
    }
}

private struct NewStoryboardSheet: View {
    let model: StoryboardsModel
    @Environment(VisualHub.self) private var hub
    @State private var name = ""
    @State private var sceneId = ""
    @State private var error: String?

    var body: some View {
        VisualFormSheet(title: "New Storyboard", primary: "Create Storyboard", identifier: "storyboard-new-sheet", onCancel: { model.sheet = nil }, onPrimary: submit) {
            if let scenes = hub.scenes?.scenes, !scenes.isEmpty {
                LabeledBlock(label: "Scene", hint: "The scene heading and identity come from the screenplay.") {
                    Picker("Scene", selection: $sceneId) {
                        Text("No scene (standalone storyboard)").tag("")
                        ForEach(scenes, id: \.lineageId) { Text(VisualText.sceneLabel($0)).tag($0.sceneId) }
                    }
                    .labelsHidden()
                    .accessibilityIdentifier("new-sb-scene")
                }
            }
            LabeledBlock(label: sceneId.isEmpty ? "Name" : "Name (optional)", hint: sceneId.isEmpty ? nil : "Optional. Defaults to the scene heading.", error: error) {
                TextField("Name", text: $name, prompt: Text(sceneId.isEmpty ? "e.g. Title sequence" : "e.g. Alternative coverage"))
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(submit)
                    .onChange(of: name) { _, _ in error = nil }
                    .accessibilityIdentifier("new-sb-name")
            }
        }
        .onAppear { sceneId = model.selection.scene?.sceneId ?? "" }
    }

    private func submit() {
        if sceneId.isEmpty, name.trimmingCharacters(in: .whitespaces).isEmpty {
            error = "Give the storyboard a name, or choose a scene."
            return
        }
        Task { if await model.createBoard(name: name, sceneId: sceneId.isEmpty ? nil : sceneId) != nil { model.sheet = nil } }
    }
}

private struct RenameStoryboardSheet: View {
    let model: StoryboardsModel
    let board: StoryboardSummary
    @State private var name = ""

    var body: some View {
        VisualFormSheet(title: "Rename storyboard", primary: "Rename", primaryEnabled: !name.trimmingCharacters(in: .whitespaces).isEmpty,
                        identifier: "storyboard-rename-sheet", width: 380, onCancel: { model.sheet = nil }, onPrimary: submit) {
            LabeledBlock(label: "Name") {
                TextField("Name", text: $name).textFieldStyle(.roundedBorder).onSubmit(submit).accessibilityIdentifier("sb-rename")
            }
        }
        .onAppear { name = board.name }
    }

    private func submit() { Task { if await model.rename(board, to: name) { model.sheet = nil } } }
}

private struct MovePanelSheet: View {
    let model: StoryboardsModel
    let panel: PanelDto
    @State private var target = ""

    var body: some View {
        let others = model.boards.filter { $0.id != model.board?.board.id }
        VisualFormSheet(title: "Move panel \(Int(panel.number))", primary: "Move Panel", primaryEnabled: !target.isEmpty, identifier: "storyboard-move-sheet",
                        onCancel: { model.sheet = nil }, onPrimary: { Task { if await model.movePanel(panel, to: target) { model.sheet = nil } } }) {
            LabeledBlock(label: "Storyboard", hint: "The panel is added at the end. Its shot link is kept.") {
                Picker("Storyboard", selection: $target) {
                    ForEach(others, id: \.id) { Text("\($0.sceneNumber.map { "Scene \($0): " } ?? "")\($0.name)").tag($0.id) }
                }
                .labelsHidden()
                .accessibilityIdentifier("move-panel-board")
            }
        }
        .onAppear { target = others.first?.id ?? "" }
    }
}

private struct StoryboardExportSheet: View {
    let model: StoryboardsModel

    var body: some View {
        let current = model.board?.board
        var scopes: [VisualExportScope] = []
        if let c = current { scopes.append(VisualExportScope(id: "current", label: "Current storyboard (\(c.name))")) }
        scopes.append(VisualExportScope(id: "all", label: "All storyboards"))
        scopes.append(VisualExportScope(id: "selected", label: "Selected storyboards", picks: true))
        return VisualExportSheet(
            title: "Export Storyboard",
            note: "A clean storyboard sheet: panels with shot numbers, descriptions and framing.",
            documentName: current.map { "Storyboard - \($0.name)" } ?? "Storyboards", formats: [.pdf], scopes: scopes,
            pickItems: model.boards.map { ($0.id, $0.name) },
            options: [VisualExportOption(key: "includeNotes", label: "Include shot notes", isOn: false)],
            identifier: "storyboard-export-sheet", onCancel: { model.sheet = nil }
        ) { req in
            let ids: [String]? = req.scope == "all" ? nil : req.scope == "current" ? current.map { [$0.id] } : req.picked
            return await model.export(boardIds: ids, path: req.path, includeNotes: req.flags["includeNotes"] ?? false)
        }
    }
}

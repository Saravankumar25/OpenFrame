// Shot Lists: "How will I capture this scene?" Shots are planned scene by scene and ordered by
// drag and drop (or Move Up and Move Down). Labels (12A, 12B) are generated from order in the
// core and are never typed or renumbered by hand. Only a description is required.
// Mirrors apps/desktop/src/workspaces/production/tabs/shots.tsx.

import AppKit
import OpenFrameCore
import SwiftUI

struct ShotsView: View {
    @Bindable var model: ShotsModel
    @Environment(AppModel.self) private var app
    @Environment(VisualHub.self) private var hub
    /// List or grid (owner change list item 21, as on Windows); a view preference of this Mac.
    @AppStorage("openframe.shots.layout") private var layout = "list"
    @Environment(\.ofTheme) private var theme
    @State private var composer = ""
    @FocusState private var composerFocused: Bool

    private var dataKey: Int { app.changes.key(VisualTables.shot) &+ (hub.scenes?.scenes.count ?? 0) &* 7919 }

    var body: some View {
        VStack(spacing: 0) {
            header
            if hub.scenesLoaded, hub.scenes == nil || (hub.scenes?.sourceKind == "none" && hub.scenes?.removed.isEmpty == true) {
                VisualNoScriptState(what: "Shots")
            } else {
                if let label = hub.scenes?.sourceLabel, !label.isEmpty {
                    Text("Scenes from \(label)").font(.caption).foregroundStyle(theme["muted"])
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: .infinity).padding(.horizontal, PageSpacing.medium).padding(.bottom, PageSpacing.tight)
                }
                toolbar
                if let scene = model.scene { SceneReviewBanner(hub: hub, scene: scene, what: "shots and storyboards") }
                if let r = model.removed { RemovedSceneBanner(heading: r.heading, what: "shots") }
                list
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .task(id: "\(dataKey)|\(hub.scenesLoaded)|\(model.sceneKey ?? "")|\(model.showRemoved)") { await model.reload() }
        .themedSheet(item: $model.sheet) { ShotSheetHost(model: model, sheet: $0) }
        .inspector(isPresented: Binding(get: { model.selected != nil }, set: { if !$0 { model.selectedId = nil } })) {
            if let s = model.selected {
                ShotInspector(model: model, shot: s).id(s.id)
                    .inspectorColumnWidth(min: 320, ideal: 380, max: 500)
            }
        }
        .onProjectIntent("production.new_shot") { composerFocused = true }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("shots-tab")
    }

    // MARK: Header and toolbar

    private var header: some View {
        VisualPageHeader(title: "Shot Lists", subtitle: VisualTab.shots.subtitle) {
            Button { model.sheet = .export } label: { Label("Export", systemImage: "square.and.arrow.up") }
                .disabled(!hub.hasScenes)
                .accessibilityIdentifier("shots-export")
            Button {
                composerFocused = true
                if !composer.trimmingCharacters(in: .whitespaces).isEmpty { addShot() }
            } label: { Label("Add Shot", systemImage: "plus") }
                .buttonStyle(.ofProminent)
                .disabled(!model.canAdd)
                .help(model.canAdd ? "" : "Choose a scene first")
                .keyboardShortcut("n", modifiers: [.command, .option])
                .accessibilityIdentifier("shots-add")
        }
    }

    private var toolbar: some View {
        CenteredWrap(spacing: 10, lineSpacing: PageSpacing.small) {
            Picker("Scene", selection: Binding(get: { model.sceneKey ?? "" }, set: { k in Task { await model.chooseScene(k) } })) {
                Text("All scenes").tag("all")
                if let scenes = hub.scenes?.scenes, !scenes.isEmpty {
                    Section(hub.scenes?.sourceLabel.isEmpty == false ? hub.scenes!.sourceLabel : "Scenes") {
                        ForEach(scenes, id: \.lineageId) { Text(VisualText.sceneLabel($0) + ($0.needsReview ? ", review" : "")).tag($0.lineageId) }
                    }
                }
                if model.showRemoved, let removed = hub.scenes?.removed, !removed.isEmpty {
                    Section("Removed from script") {
                        ForEach(removed, id: \.lineageId) { Text("\($0.heading.isEmpty ? "Removed scene" : $0.heading) (\(Int($0.shotCount)))").tag($0.lineageId) }
                    }
                }
            }
            .labelsHidden().frame(maxWidth: 440)
            .accessibilityLabel("Scene")
            .accessibilityIdentifier("shots-scene-picker")
            Picker("Shot list view", selection: $layout) {
                Label("List", systemImage: "list.bullet").tag("list")
                Label("Grid", systemImage: "square.grid.2x2").tag("grid")
            }
            .pickerStyle(.segmented).fixedSize()
            .accessibilityLabel("Shot list view")
            .accessibilityIdentifier("shots-layout")
            Text(VisualText.plural(model.shots.count, "shot"))
                .font(.caption).padding(.horizontal, 8).padding(.vertical, 3)
                .background(theme["surface2"], in: Capsule())
                .accessibilityIdentifier("shots-count")
            if model.picked.count > 1 {
                Button("Delete \(model.picked.count) shots", role: .destructive) { Task { await model.delete(Array(model.picked)) } }
                    .accessibilityIdentifier("shots-delete-picked")
            }
            if let removed = hub.scenes?.removed, !removed.isEmpty {
                Toggle("Show removed scenes", isOn: Binding(get: { model.showRemoved }, set: { v in Task { await model.setShowRemoved(v) } }))
                    .accessibilityIdentifier("shots-show-removed")
            }
            if model.scene != nil, !model.shots.isEmpty {
                Menu {
                    Button("Copy Planning to Another Scene\u{2026}") { model.sheet = .copyPlanning }
                } label: { Label("More", systemImage: "ellipsis.circle") }
                    .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                    .accessibilityLabel("More scene actions")
                    .accessibilityIdentifier("shots-more")
            }
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, PageSpacing.medium).padding(.bottom, PageSpacing.small)
    }

    // MARK: List

    private var list: some View {
        ScrollView {
            VStack(alignment: .center, spacing: 8) {
                if !model.loaded {
                    ProgressView().frame(maxWidth: .infinity, minHeight: 120)
                } else if model.shots.isEmpty, let f = model.loadFailure {
                    ContentUnavailableView {
                        Label("The shots could not be loaded.", systemImage: "exclamationmark.triangle")
                    } description: { Text(f.message) } actions: {
                        Button("Try Again") { Task { await model.reload() } }.accessibilityIdentifier("shots-retry")
                    }
                } else if model.allScenes {
                    if model.groups.isEmpty {
                        emptyState(title: "No shots yet", text: "Choose a scene to start its shot list.", id: "shots-empty-all")
                    } else {
                        ForEach(model.groups) { g in
                            VStack(alignment: .leading, spacing: 6) {
                                HStack {
                                    Text(g.title).fontWeight(.semibold)
                                    Spacer()
                                    if g.needsReview { Text("Scene changed since planning").font(.caption).foregroundStyle(theme["yellow-t"]) }
                                    Button("Open Scene") { Task { await model.chooseScene(g.lineage) } }
                                        .controlSize(.small).accessibilityIdentifier("shots-open-scene-\(g.lineage)")
                                }
                                if layout == "grid" { ShotGrid(model: model, shots: g.shots) }
                                else { ShotRows(model: model, shots: g.shots, reorderable: false) }
                            }
                            .accessibilityElement(children: .contain)
                            .accessibilityLabel(g.title)
                        }
                    }
                } else {
                    if model.shots.isEmpty, model.scene != nil {
                        emptyState(title: "No shots in this scene yet", text: "Add only the shots you need. Camera details are optional.", id: "shots-empty")
                    } else if !model.shots.isEmpty {
                        // Reordering stays in the list, where the drag handles are.
                        if layout == "grid" { ShotGrid(model: model, shots: model.shown) }
                        else { ShotRows(model: model, shots: model.shown, reorderable: true) }
                    }
                    if model.scene != nil { composerRow }
                }
                Text(VisualText.shotHint).font(.caption).foregroundStyle(theme["muted"])
                    .multilineTextAlignment(.center).frame(maxWidth: .infinity)
            }
            .pageColumn(maxWidth: layout == "grid" ? PageSpacing.wideColumn : 780)
            .padding(.bottom, PageSpacing.medium)
        }
        .frame(minHeight: 0)
    }

    private func emptyState(title: String, text: String, id: String) -> some View {
        ContentUnavailableView(title, systemImage: "film", description: Text(text))
            .frame(minHeight: 180)
            .accessibilityIdentifier(id)
    }

    private var composerRow: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .top) {
                TextField("New shot description", text: $composer, prompt: Text("Describe the next shot, for example Wide: Arjun enters the station"))
                    .textFieldStyle(.roundedBorder)
                    .focused($composerFocused)
                    .onSubmit(addShot)
                    .onChange(of: composer) { _, _ in model.composerError = nil }
                    .accessibilityIdentifier("shots-composer")
                Button { addShot() } label: { Label("Add Shot", systemImage: "plus") }
                    .accessibilityIdentifier("shots-composer-add")
            }
            if let e = model.composerError {
                Text(e).font(.caption).foregroundStyle(theme["red-t"]).accessibilityIdentifier("shots-composer-error")
            }
        }
        .padding(.top, 6)
    }

    private func addShot() {
        Task {
            if await model.addShot(description: composer) != nil { composer = "" }
            composerFocused = true
        }
    }
}

// MARK: Grid

/// The shots as cards: the reference image (or the first storyboard panel) with the shot details
/// below, so a shot list also works as a visual reference (Windows: ShotCard in shots.tsx).
private struct ShotGrid: View {
    let model: ShotsModel
    let shots: [ShotDto]
    @Environment(\.ofTheme) private var theme

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 200, maximum: 280), spacing: 12, alignment: .top)], spacing: 12) {
            ForEach(shots, id: \.id) { s in card(s) }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Shots")
        .accessibilityIdentifier("shots-grid")
        .onDeleteCommand {
            if !model.picked.isEmpty { Task { await model.delete(Array(model.picked)) } }
        }
    }

    private func card(_ s: ShotDto) -> some View {
        let on = s.id == model.selectedId || model.picked.contains(s.id)
        let picture = s.referenceAsset ?? s.panels.first(where: { $0.asset != nil })?.asset
        let meta = [s.size, s.angle, s.movement, s.lens].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{00B7} ")
        return VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text("Scene \(s.sceneNumber ?? "?")").font(.caption).foregroundStyle(theme["muted"])
                Spacer()
                Text("Shot \(s.label)").font(.callout.weight(.bold))
            }
            ZStack {
                if picture != nil {
                    VisualAssetImage(asset: picture, maxPixel: 560)
                } else {
                    VStack(spacing: 4) {
                        Image(systemName: "photo").font(.title2)
                        Text("No reference image").font(.caption)
                    }
                    .foregroundStyle(theme["muted"])
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(theme["surface2"])
                }
            }
            .aspectRatio(16 / 9, contentMode: .fit)
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            if !meta.isEmpty {
                Text(meta).font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
                    .fixedSize(horizontal: false, vertical: true)
            }
            Text(s.description).font(.callout)
                .fixedSize(horizontal: false, vertical: true)
            if !s.characters.isEmpty {
                Text(s.characters.joined(separator: ", ")).font(.caption).foregroundStyle(theme["muted"])
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .background(on ? theme["accent-soft"] : theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(on ? theme["accent"] : theme["line"]))
        .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .onTapGesture {
            let f = NSEvent.modifierFlags
            model.click(s, command: f.contains(.command), shift: f.contains(.shift))
        }
        .contextMenu {
            Button("Open") { model.selectedId = s.id }
            Button("Duplicate") { Task { await model.duplicate(s) } }
            Divider()
            Button("Attach Storyboard\u{2026}") { model.sheet = .attach(s) }
            Divider()
            Button("Delete", role: .destructive) { Task { await model.delete([s.id]) } }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Shot \(s.label): \(s.description)")
        .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
        .accessibilityIdentifier("shot-card-\(s.id)")
    }
}

// MARK: Rows

private struct ShotRows: View {
    let model: ShotsModel
    let shots: [ShotDto]
    let reorderable: Bool
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 4) {
            ForEach(Array(shots.enumerated()), id: \.element.id) { i, s in
                ShotRow(model: model, shot: s, index: i, count: shots.count, reorderable: reorderable)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Shots")
        .accessibilityIdentifier("shots-list")
        .onDeleteCommand {
            if !model.picked.isEmpty { Task { await model.delete(Array(model.picked)) } }
        }
    }
}

private struct ShotRow: View {
    let model: ShotsModel
    let shot: ShotDto
    let index: Int
    let count: Int
    let reorderable: Bool
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme
    @State private var targeted = false

    var body: some View {
        let on = shot.id == model.selectedId || model.picked.contains(shot.id)
        HStack(spacing: 8) {
            if reorderable {
                Image(systemName: "line.3.horizontal").foregroundStyle(theme["muted"]).help("Drag to reorder")
                    .accessibilityHidden(true)
            }
            Text(shot.label).font(.callout.weight(.bold)).frame(minWidth: 34, alignment: .leading)
            if let size = shot.size, !size.isEmpty { chip(size, filled: true) }
            Text(shot.description).fontWeight(.semibold).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
            if let m = shot.movement, !m.isEmpty { chip(m) }
            if let a = shot.angle, !a.isEmpty { chip(a) }
            if let l = shot.lens, !l.isEmpty { chip(l) }
            if !shot.panels.isEmpty {
                Image(systemName: "rectangle.split.3x1").foregroundStyle(theme["muted"])
                    .help(VisualText.plural(shot.panels.count, "storyboard panel"))
                    .accessibilityLabel("Has storyboard panel")
            }
            if shot.referenceAsset != nil {
                Image(systemName: "photo").foregroundStyle(theme["muted"]).help("Has reference image")
                    .accessibilityLabel("Has reference image")
            }
        }
        .padding(.horizontal, 10).padding(.vertical, 8)
        .background(on ? theme["accent-soft"] : theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(targeted ? theme["accent"] : theme["line"], lineWidth: targeted ? 2 : 1))
        .contentShape(RoundedRectangle(cornerRadius: 8))
        .onTapGesture {
            let f = NSEvent.modifierFlags
            model.click(shot, command: f.contains(.command), shift: f.contains(.shift))
        }
        .modifier(ShotReorder(enabled: reorderable, model: model, shot: shot, index: index, targeted: $targeted))
        .contextMenu { menu }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Shot \(shot.label): \(shot.description)")
        .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction(named: "Move up") { Task { await model.nudge(shot.id, by: -1) } }
        .accessibilityAction(named: "Move down") { Task { await model.nudge(shot.id, by: 1) } }
        .accessibilityAction(named: "Delete") { Task { await model.delete([shot.id]) } }
        .accessibilityIdentifier("shot-\(shot.id)")
    }

    private func chip(_ text: String, filled: Bool = false) -> some View {
        Text(text).font(.caption)
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(filled ? theme["accent-soft"] : Color.clear, in: Capsule())
            .overlay(Capsule().strokeBorder(filled ? Color.clear : theme["line2"]))
    }

    @ViewBuilder private var menu: some View {
        let many = model.picked.count > 1 && model.picked.contains(shot.id)
        Button("Open") { model.selectedId = shot.id }
        Button("Duplicate") { Task { await model.duplicate(shot) } }
        Button("Move to Scene\u{2026}") { model.sheet = .move(shot) }.disabled((hub.scenes?.scenes.count ?? 0) < 2)
        if reorderable {
            Button("Move Up") { Task { await model.nudge(shot.id, by: -1) } }.disabled(index == 0)
            Button("Move Down") { Task { await model.nudge(shot.id, by: 1) } }.disabled(index == count - 1)
        }
        Divider()
        Button("Attach Storyboard\u{2026}") { model.sheet = .attach(shot) }
        Button("Create Storyboard Panel") { Task { await model.createPanel(for: shot) } }
            .disabled(shot.sceneRemoved && shot.panels.isEmpty)
        Divider()
        Button(many ? "Delete \(model.picked.count) Shots" : "Delete", role: .destructive) {
            Task { await model.delete(many ? Array(model.picked) : [shot.id]) }
        }
    }
}

/// Drag a shot onto another shot's place to reorder it (the list of one scene only).
private struct ShotReorder: ViewModifier {
    let enabled: Bool
    let model: ShotsModel
    let shot: ShotDto
    let index: Int
    @Binding var targeted: Bool

    func body(content: Content) -> some View {
        if enabled {
            content
                .draggable(shot.id) { Text("Shot \(shot.label)").padding(6).background(.regularMaterial, in: RoundedRectangle(cornerRadius: 4)) }
                .dropDestination(for: String.self) { ids, _ in
                    guard let id = ids.first, id != shot.id, model.shown.contains(where: { $0.id == id }) else { return false }
                    Task { await model.reorder(id, to: index) }
                    return true
                } isTargeted: { targeted = $0 }
        } else {
            content
        }
    }
}

// MARK: Inspector

private struct ShotInspector: View {
    let model: ShotsModel
    let shot: ShotDto
    @Environment(VisualHub.self) private var hub
    @Environment(\.ofTheme) private var theme
    @State private var descError: String?
    @State private var techOpen = false
    @State private var charText = ""

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    VStack(alignment: .leading) {
                        Text("SHOT").font(.caption2).foregroundStyle(theme["muted"])
                        Text([shot.label, shot.size, shot.description].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{00B7} "))
                            .font(.title3.weight(.semibold)).lineLimit(2)
                    }
                    Spacer()
                    Button { model.selectedId = nil } label: { Label("Close", systemImage: "xmark") }
                        .buttonStyle(.borderless).accessibilityLabel("Close").accessibilityIdentifier("shot-inspector-close")
                }
                if shot.needsReview, !shot.sceneRemoved {
                    Text("Scene \(shot.sceneNumber ?? "") changed since this shot was planned. Review it against the script.")
                        .font(.caption).foregroundStyle(theme["yellow-t"]).accessibilityIdentifier("shot-needs-review")
                }
                LabeledBlock(label: "Description", error: descError) {
                    CommitEditor(label: "Description", value: shot.description, identifier: "shot-desc") { v in
                        if v.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                            descError = "Description is required."
                        } else {
                            descError = nil
                            Task { await model.update(shot, description: v) }
                        }
                    }
                }
                DisclosureGroup(isExpanded: $techOpen) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(alignment: .top, spacing: 8) {
                            LabeledBlock(label: "Shot size") {
                                CommitField(label: "Shot size", value: shot.size ?? "", identifier: "shot-size", suggestions: VisualText.shotSizes) { v in
                                    Task { await model.update(shot, size: v) }
                                }.textFieldStyle(.roundedBorder)
                            }
                            LabeledBlock(label: "Movement") {
                                CommitField(label: "Movement", value: shot.movement ?? "", identifier: "shot-movement", suggestions: VisualText.movements) { v in
                                    Task { await model.update(shot, movement: v) }
                                }.textFieldStyle(.roundedBorder)
                            }
                        }
                        HStack(alignment: .top, spacing: 8) {
                            LabeledBlock(label: "Angle") {
                                CommitField(label: "Angle", value: shot.angle ?? "", identifier: "shot-angle", suggestions: VisualText.angles) { v in
                                    Task { await model.update(shot, angle: v) }
                                }.textFieldStyle(.roundedBorder)
                            }
                            LabeledBlock(label: "Lens") {
                                CommitField(label: "Lens", value: shot.lens ?? "", prompt: "e.g. 35mm", identifier: "shot-lens") { v in
                                    Task { await model.update(shot, lens: v) }
                                }.textFieldStyle(.roundedBorder)
                            }
                        }
                        LabeledBlock(label: "Camera notes") {
                            CommitField(label: "Camera notes", value: shot.cameraNotes ?? "", prompt: "e.g. follow focus to the folder", identifier: "shot-camera-notes") { v in
                                Task { await model.update(shot, cameraNotes: v) }
                            }.textFieldStyle(.roundedBorder)
                        }
                        charactersBlock
                        LabeledBlock(label: "Sound note") {
                            CommitField(label: "Sound note", value: shot.soundNote ?? "", prompt: "e.g. rain on window", identifier: "shot-sound") { v in
                                Task { await model.update(shot, soundNote: v) }
                            }.textFieldStyle(.roundedBorder)
                        }
                    }
                    .padding(.top, 6)
                } label: {
                    HStack {
                        Text("Technical details").fontWeight(.semibold)
                        Text("optional, hidden until you need them").font(.caption).foregroundStyle(theme["muted"])
                    }
                }
                .accessibilityIdentifier("shot-technical")
                referenceBlock
                storyboardBlock
                if let label = hub.scenes?.sourceLabel, !label.isEmpty {
                    Text("Scene \(shot.sceneNumber ?? "-") \u{00B7} \(shot.sceneHeading)").font(.caption).foregroundStyle(theme["muted"])
                }
                Divider()
                HStack {
                    Button("Delete", role: .destructive) { Task { await model.delete([shot.id]) } }
                        .accessibilityIdentifier("shot-delete")
                    Spacer()
                    Button("Duplicate") { Task { await model.duplicate(shot) } }
                        .accessibilityIdentifier("shot-duplicate")
                }
                .controlSize(.small)
            }
            .padding(16)
        }
        .onAppear {
            techOpen = shot.size != nil || shot.movement != nil || shot.angle != nil || shot.lens != nil || shot.cameraNotes != nil
                || !shot.characters.isEmpty || shot.soundNote != nil
        }
        .task(id: shot.sceneId) { await model.loadCharacters(for: shot) }
        .accessibilityIdentifier("shot-inspector")
    }

    private var charactersBlock: some View {
        let unused = model.characters.filter { c in !shot.characters.contains { $0.caseInsensitiveCompare(c) == .orderedSame } }
        return LabeledBlock(label: "Characters", hint: unused.isEmpty ? nil : "In this scene: \(unused.joined(separator: ", "))") {
            if !shot.characters.isEmpty {
                VisualFlowLayout(spacing: 4) {
                    ForEach(shot.characters, id: \.self) { c in
                        HStack(spacing: 3) {
                            Text(c).font(.caption)
                            Button { Task { await model.update(shot, characters: shot.characters.filter { $0 != c }) } } label: { Label("Remove", systemImage: "xmark").font(.caption2) }
                                .buttonStyle(.plain).accessibilityLabel("Remove \(c)")
                                .accessibilityIdentifier("shot-remove-character-\(c)")
                        }
                        .padding(.horizontal, 7).padding(.vertical, 2)
                        .overlay(Capsule().strokeBorder(theme["line2"]))
                    }
                }
            }
            HStack {
                TextField("Add a character", text: $charText, prompt: Text("Add a character and press Return"))
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(addCharacter)
                    .accessibilityIdentifier("shot-characters")
                if !unused.isEmpty {
                    Menu {
                        ForEach(unused, id: \.self) { c in Button(c) { charText = c; addCharacter() } }
                    } label: { Label("Characters", systemImage: "person.crop.circle.badge.plus") }
                        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                        .accessibilityLabel("Characters in this scene")
                        .accessibilityIdentifier("shot-character-suggestions")
                }
            }
        }
    }

    private func addCharacter() {
        let t = charText.trimmingCharacters(in: .whitespaces)
        guard !t.isEmpty else { return }
        if !shot.characters.contains(where: { $0.caseInsensitiveCompare(t) == .orderedSame }) {
            Task { await model.update(shot, characters: shot.characters + [t]) }
        }
        charText = ""
    }

    private var referenceBlock: some View {
        LabeledBlock(label: "Reference image", hint: "A visual aid for this shot. It is copied into the project.") {
            if shot.referenceAsset != nil {
                HStack(alignment: .top) {
                    VisualAssetImage(asset: shot.referenceAsset, maxPixel: 360)
                        .frame(width: 120, height: 72).clipShape(RoundedRectangle(cornerRadius: 6))
                        .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line2"]))
                        .accessibilityLabel("Shot reference")
                    VStack(alignment: .leading) {
                        Button("Replace\u{2026}") { Task { await model.chooseReference(for: shot) } }.accessibilityIdentifier("shot-reference-replace")
                        Button("Remove") { Task { await model.setReference(shot, path: nil) } }.accessibilityIdentifier("shot-reference-remove")
                    }
                    .controlSize(.small)
                }
            } else {
                Button { Task { await model.chooseReference(for: shot) } } label: { Label("Add reference image\u{2026}", systemImage: "photo") }
                    .controlSize(.small).accessibilityIdentifier("shot-reference-add")
            }
        }
    }

    private var storyboardBlock: some View {
        LabeledBlock(label: "Storyboard") {
            VisualFlowLayout(spacing: 6) {
                ForEach(shot.panels, id: \.panelId) { p in
                    HStack(spacing: 4) {
                        Button("Storyboard panel \(Int(p.number))") { Task { await hub.openPanel(storyboardId: p.storyboardId, panelId: p.panelId) } }
                            .buttonStyle(.plain).font(.caption)
                            .help(p.storyboardName)
                            .accessibilityIdentifier("shot-open-panel-\(p.panelId)")
                        Button { Task { await model.detach(shot, panelId: p.panelId) } } label: { Label("Detach", systemImage: "xmark").font(.caption2) }
                            .buttonStyle(.plain).accessibilityLabel("Detach storyboard panel \(Int(p.number))")
                            .accessibilityIdentifier("shot-detach-panel-\(p.panelId)")
                    }
                    .padding(.horizontal, 8).padding(.vertical, 3)
                    .background(theme["blue-soft"], in: Capsule())
                }
                Button("Attach\u{2026}") { model.sheet = .attach(shot) }.controlSize(.small).accessibilityIdentifier("shot-attach")
                Button("Create panel") { Task { await model.createPanel(for: shot) } }
                    .controlSize(.small)
                    .disabled(shot.sceneRemoved && shot.panels.isEmpty)
                    .accessibilityIdentifier("shot-create-panel")
            }
        }
    }
}

// MARK: Sheets

private struct ShotSheetHost: View {
    let model: ShotsModel
    let sheet: ShotSheet
    @Environment(VisualHub.self) private var hub

    var body: some View {
        switch sheet {
        case let .move(s):
            PickSceneSheet(title: "Move shot \(s.label)", scenes: hub.scenes?.scenes ?? [], excluding: s.sceneLineageId, confirm: "Move Shot",
                           identifier: "shot-move-sheet", onCancel: { model.sheet = nil }) { id in
                Task { if await model.move(s, toScene: id) { model.sheet = nil } }
            }
        case let .attach(s): AttachPanelSheet(model: model, shot: s)
        case .copyPlanning:
            PickSceneSheet(title: "Copy planning to another scene", scenes: hub.scenes?.scenes ?? [], excluding: model.scene?.lineageId, confirm: "Copy Planning",
                           identifier: "shot-copy-sheet", onCancel: { model.sheet = nil }) { id in
                Task { if await model.copyPlanning(toScene: id) { model.sheet = nil } }
            }
        case .export: ShotExportSheet(model: model)
        }
    }
}

private struct AttachPanelSheet: View {
    let model: ShotsModel
    let shot: ShotDto
    @State private var boardId = ""
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Attach storyboard panel to \(shot.label)").font(.title3.weight(.semibold))
            if model.attachBoards.isEmpty {
                Text("There are no storyboards yet. Use \u{201C}Create panel\u{201D} to start one from this shot.")
                    .foregroundStyle(theme["muted"]).accessibilityIdentifier("attach-empty")
            } else {
                LabeledBlock(label: "Storyboard") {
                    Picker("Storyboard", selection: $boardId) {
                        ForEach(model.attachBoards, id: \.id) {
                            Text("\($0.sceneNumber.map { "Scene \($0): " } ?? "")\($0.name) (\(Int($0.panelCount)))").tag($0.id)
                        }
                    }
                    .labelsHidden()
                    .accessibilityIdentifier("attach-board")
                }
                ScrollView {
                    LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 4), spacing: 10) {
                        ForEach(model.attachPanels, id: \.id) { p in
                            let already = p.shotId == shot.id
                            Button {
                                Task { if await model.attach(shot, panelId: p.id) { model.sheet = nil } }
                            } label: {
                                VStack(alignment: .leading, spacing: 4) {
                                    ZStack(alignment: .topLeading) {
                                        VisualAssetImage(asset: p.asset, maxPixel: 240, emptyText: "Empty panel").frame(height: 70)
                                        Text("\(Int(p.number))").font(.caption2.weight(.bold)).padding(3)
                                            .background(theme["ink"], in: RoundedRectangle(cornerRadius: 3)).foregroundStyle(theme["panel"]).padding(4)
                                    }
                                    Text(p.description.isEmpty ? "No description" : p.description).font(.caption).lineLimit(2)
                                }
                                .opacity(already ? 0.5 : 1)
                            }
                            .buttonStyle(.plain)
                            .disabled(already)
                            .accessibilityLabel("Panel \(Int(p.number))\(p.description.isEmpty ? "" : ": \(p.description)")\(already ? " (already attached)" : "")")
                            .accessibilityIdentifier("attach-panel-\(p.id)")
                        }
                    }
                }
                .frame(height: 240)
            }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { model.sheet = nil }.keyboardShortcut(.cancelAction).accessibilityIdentifier("attach-cancel")
            }
        }
        .padding(20).frame(width: 620)
        .task {
            await model.loadAttachChoices(for: shot, boardId: nil)
            boardId = model.attachBoards.first?.id ?? ""
        }
        .onChange(of: boardId) { _, id in
            guard !id.isEmpty else { return }
            Task { await model.loadAttachChoices(for: shot, boardId: id) }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("shot-attach-sheet")
    }
}

private struct ShotExportSheet: View {
    let model: ShotsModel
    @Environment(VisualHub.self) private var hub

    var body: some View {
        let scene = model.scene
        var scopes: [VisualExportScope] = []
        if let s = scene { scopes.append(VisualExportScope(id: "current", label: "Current scene (\(s.number) \u{00B7} \(s.heading))")) }
        scopes.append(VisualExportScope(id: "selected", label: "Selected scenes", picks: true))
        scopes.append(VisualExportScope(id: "project", label: "Whole project"))
        return VisualExportSheet(
            title: "Export Shot List", documentName: "Shot List", formats: [.pdf, .csv, .xlsx], scopes: scopes,
            pickItems: (hub.scenes?.scenes ?? []).map { ($0.sceneId, "\($0.number) \u{00B7} \($0.heading)") },
            options: [VisualExportOption(key: "includeRemoved", label: "Include shots of scenes removed from the script (whole project only)", isOn: false)],
            identifier: "shots-export-sheet", onCancel: { model.sheet = nil }
        ) { req in
            let ids: [String]? = req.scope == "project" ? nil : req.scope == "current" ? scene.map { [$0.sceneId] } : req.picked
            return await model.export(format: req.format, path: req.path, sceneIds: ids, includeRemoved: req.scope == "project" && (req.flags["includeRemoved"] ?? false))
        }
    }
}

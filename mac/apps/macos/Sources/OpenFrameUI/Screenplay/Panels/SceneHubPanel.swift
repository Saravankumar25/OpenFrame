// The inspector of the screenplay (Mac phase M4): the Scene Hub for the caret's scene, the
// comments, and the information of the open draft. The Scene Hub is navigation and context, plus
// the scene's own fields (synopsis, notes, story day, time note), which are saved through
// screenplay.update_scene. The heading itself is edited in the script.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class SceneHubModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?

    var sceneId: String?
    var hub: ScreenplaySceneHub?
    var story: ScreenplayStoryReference?
    var characters: [String] = []

    var synopsis = ""
    var notes = ""
    var storyDay = ""
    var timeNote = ""
    private var seededFor: SceneFields?

    struct SceneFields: Equatable {
        var id: String
        var synopsis: String
        var notes: String
        var storyDay: String
        var timeNote: String
    }

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel) {
        self.app = app
        self.workspace = workspace
    }

    private var core: Core? { app?.core }
    var canEdit: Bool { workspace?.canEdit == true && workspace?.overview?.permissions.edit == true }

    var scene: ScreenplaySceneDto? {
        guard let sceneId else { return nil }
        return workspace?.document?.scenes.first { $0.id == sceneId }
    }

    static func fields(_ s: ScreenplaySceneDto) -> SceneFields {
        SceneFields(id: s.id, synopsis: s.synopsis ?? "", notes: s.notes ?? "", storyDay: s.storyDay ?? "", timeNote: s.timeNote ?? "")
    }

    var isDirty: Bool {
        guard let seededFor else { return false }
        return seededFor != SceneFields(id: seededFor.id, synopsis: synopsis, notes: notes, storyDay: storyDay, timeNote: timeNote)
    }

    /// Fill the edit fields from the scene. A scene the person is editing is left alone.
    func seed(force: Bool = false) {
        guard let scene else { seededFor = nil; return }
        let f = Self.fields(scene)
        if !force, isDirty, seededFor?.id == f.id { return }
        seededFor = f
        synopsis = f.synopsis; notes = f.notes; storyDay = f.storyDay; timeNote = f.timeNote
    }

    func load(sceneId: String?) async {
        let changed = sceneId != self.sceneId
        self.sceneId = sceneId
        if changed { hub = nil; story = nil; characters = [] }
        seed(force: changed)
        guard let core, let sceneId, let draftId = workspace?.draft?.id else { return }
        hub = try? await core.screenplaySceneHub(ScreenplaySceneArgs(sceneId: sceneId))
        story = try? await core.screenplayStoryReference(ScreenplaySceneArgs(sceneId: sceneId))
        if let report = try? await core.screenplayCharacters(ScreenplayCharactersArgs(draftId: draftId)) {
            var names: [String] = []
            for cue in report.cues where cue.scenes.contains(where: { $0.sceneId == sceneId }) {
                let name = cue.characterName ?? cue.cueName
                if !names.contains(name) { names.append(name) }
            }
            characters = names
        } else {
            characters = localCharacters()
        }
    }

    /// The cue names written in the scene, when the report is not available.
    func localCharacters() -> [String] {
        var names: [String] = []
        for e in scene?.elements ?? [] where e.elementType == .character {
            let name = e.text.split(separator: "(").first.map { $0.trimmingCharacters(in: .whitespaces).uppercased() } ?? ""
            if !name.isEmpty, !names.contains(name) { names.append(name) }
        }
        return names
    }

    /// Save the changed fields. Returns the message to show when the core refuses.
    func save() async -> String? {
        guard let core, let scene, canEdit else { return nil }
        await workspace?.flush()
        func changed(_ new: String, _ old: String?) -> String? { new == (old ?? "") ? nil : new }
        let args = ScreenplayUpdateSceneArgs(
            sceneId: scene.id,
            notes: changed(notes, scene.notes), synopsis: changed(synopsis, scene.synopsis),
            storyDay: changed(storyDay, scene.storyDay), timeNote: changed(timeNote, scene.timeNote))
        do {
            _ = try await core.screenplayUpdateScene(args)
            seededFor = SceneFields(id: scene.id, synopsis: synopsis, notes: notes, storyDay: storyDay, timeNote: timeNote)
            if let id = workspace?.draft?.id { await workspace?.loadDocument(id) }
            await load(sceneId: scene.id)
            return nil
        } catch {
            return PanelText.message(error, fallback: "The scene could not be saved.")
        }
    }

    /// The workspace a hub row opens, or nil for the row that is the script itself.
    static func place(forRow key: String) -> ProjectPlace? {
        switch key {
        case "breakdown": ProjectPlace(.breakdown)
        case "story": ProjectPlace(.story)
        case "shots": ProjectPlace(.production, tab: .shotLists)
        case "storyboard": ProjectPlace(.production, tab: .storyboards)
        case "schedule": ProjectPlace(.production, tab: .schedule)
        default: nil
        }
    }
}

struct SceneHubPanel: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    let sceneId: String?
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = SceneHubModel()
    @State private var privateNotes = PrivateNotesModel()
    @State private var error: String?

    private static let tables = ["screenplay_scene", "screenplay_element", "comment", "story_scene_card"]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                if let scene = model.scene {
                    Text("Scene \(scene.label): \(scene.heading.isEmpty ? "Untitled scene" : scene.heading)")
                        .font(.headline)
                        .accessibilityAddTraits(.isHeader)
                        .accessibilityIdentifier("scene-hub-title")
                    Text("What exists around this scene?").font(.callout).foregroundStyle(theme["muted"])
                    rows
                    storyCard
                    charactersSection
                    fields
                    PrivateNotesSection(model: privateNotes)
                    sceneComments
                    Text("The hub is navigation and context, not a master edit screen. Each link opens its own workspace.")
                        .font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                } else {
                    Text("Place the caret in a scene to see what exists around it.").foregroundStyle(theme["muted"])
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .task(id: "\(sceneId ?? "-")-\(app.changes.key(Self.tables))") {
            model.attach(app, workspace)
            await model.load(sceneId: sceneId)
        }
        .task(id: "\(sceneId ?? "-")-\(app.changes.key(["private_note"]))") {
            privateNotes.attach(app)
            await privateNotes.load(sceneId: sceneId)
        }
        .accessibilityIdentifier("scene-hub")
    }

    @ViewBuilder private var rows: some View {
        if let hub = model.hub {
            VStack(spacing: 0) {
                ForEach(hub.rows, id: \.key) { r in
                    Button {
                        if r.key == "comments" { state.tab = .comments }
                        else if let place = SceneHubModel.place(forRow: r.key) {
                            ProjectNav.show(place)
                        }
                    } label: {
                        HStack {
                            Text(r.label).fontWeight(.semibold)
                            Spacer()
                            Text(r.detail).font(.callout).foregroundStyle(theme["muted"]).multilineTextAlignment(.trailing)
                        }
                        .padding(.vertical, 6).padding(.horizontal, 8)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("\(r.label): \(r.detail)")
                    Divider()
                }
            }
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
        } else {
            ProgressView().controlSize(.small)
        }
    }

    @ViewBuilder private var storyCard: some View {
        sectionTitle("Beat Board card")
        if let linked = model.story?.linked {
            VStack(alignment: .leading, spacing: 4) {
                Text(linked.sceneHeading.flatMap { $0.isEmpty ? nil : $0 } ?? "Scene card").fontWeight(.semibold)
                Text(linked.shortDescription.isEmpty ? "No description yet." : linked.shortDescription).textSelection(.enabled)
                if let n = linked.notes, !n.isEmpty { Text(n).font(.callout).foregroundStyle(theme["muted"]).textSelection(.enabled) }
            }
            .padding(8).frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("scene-story-card")
        } else {
            Text("No Beat Board card is linked to this scene.").foregroundStyle(theme["muted"])
        }
    }

    @ViewBuilder private var charactersSection: some View {
        sectionTitle("Characters in this scene")
        if model.characters.isEmpty {
            Text("Nobody speaks in this scene yet.").foregroundStyle(theme["muted"])
        } else {
            HStack(spacing: 6) {
                ForEach(model.characters, id: \.self) { PanelChip(text: $0) }
            }
            .accessibilityElement(children: .combine)
            .accessibilityLabel("Characters: \(model.characters.joined(separator: ", "))")
        }
    }

    @ViewBuilder private var fields: some View {
        sectionTitle("Scene details")
        detailFields
    }

    @ViewBuilder private var detailFields: some View {
        PanelField(label: "Synopsis") { PanelTextArea(label: "Synopsis", text: $model.synopsis, minHeight: 56) }
        HStack(alignment: .top, spacing: 8) {
            PanelField(label: "Story day") {
                TextField("Story day", text: $model.storyDay, prompt: Text("Day 3")).textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Story day")
            }
            PanelField(label: "Time note") {
                TextField("Time note", text: $model.timeNote, prompt: Text("Dusk")).textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Time note")
            }
        }
        PanelField(label: "Scene note", hint: "Not printed, and left out of exports by default.") {
            PanelTextArea(label: "Scene note", text: $model.notes, minHeight: 56)
        }
        if let error { PanelBanner(tone: .err, text: error) }
        HStack {
            Spacer()
            Button("Revert") { model.seed(force: true); error = nil }.disabled(!model.isDirty)
            Button("Save Scene Details") {
                Task { error = await model.save() }
            }
            .disabled(!model.isDirty || !model.canEdit)
            .accessibilityIdentifier("scene-details-save")
        }
        .controlSize(.small)
        .onChange(of: model.scene.map(SceneHubModel.fields)) { model.seed() }
    }

    @ViewBuilder private var sceneComments: some View {
        let threads = state.comments.threads(forScene: sceneId)
        sectionTitle("Comments")
        if threads.isEmpty {
            Text("No open comments on this scene.").foregroundStyle(theme["muted"])
        } else {
            ForEach(threads, id: \.comment.id) { t in
                Button { state.comments.reveal(t.comment); state.tab = .comments } label: {
                    HStack(alignment: .firstTextBaseline) {
                        Text(t.comment.body).lineLimit(2).frame(maxWidth: .infinity, alignment: .leading)
                        Text(t.comment.authorName).font(.caption).foregroundStyle(theme["muted"])
                    }
                }
                .buttonStyle(.plain)
            }
        }
        if state.comments.canComment, let sceneId {
            Button { state.addComment(sceneId: sceneId) } label: { Label("Comment on This Scene", systemImage: "text.bubble") }
                .controlSize(.small)
        }
    }

    private func sectionTitle(_ t: String) -> some View {
        Text(t).font(.subheadline.weight(.semibold)).foregroundStyle(theme["ink2"]).padding(.top, 4)
            .accessibilityAddTraits(.isHeader)
    }
}

// MARK: - Draft information

struct DraftInfoPanel: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                if let d = workspace.draft {
                    Grid(alignment: .topLeading, horizontalSpacing: 12, verticalSpacing: 6) {
                        ForEach(rows(d), id: \.0) { k, v in
                            GridRow {
                                Text(k).foregroundStyle(theme["muted"])
                                Text(v).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
                            }
                        }
                    }
                    .accessibilityElement(children: .contain)
                    HStack {
                        Button("Drafts\u{2026}") { state.sheet = .drafts }
                        Button("Page Setup\u{2026}") { state.sheet = .pageSetup }
                    }
                    .controlSize(.small)
                } else {
                    Text("No draft is open.").foregroundStyle(theme["muted"])
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityIdentifier("draft-info")
    }

    func rows(_ d: ScreenplayDraftDto) -> [(String, String)] {
        var rows: [(String, String)] = [
            ("Draft", d.name),
            ("Status", d.status.rawValue + (d.isCurrent ? " \u{00B7} Current" : "")),
            ("Lineage", d.lineage.map(\.name).joined(separator: " \u{2192} ")),
            ("Scenes", String(Int(d.sceneCount))),
            ("Open comments", String(Int(d.openComments))),
            ("Created", PanelText.date(d.createdAt)),
            ("Last modified", PanelText.when(d.lastModified)),
        ]
        if let n = d.note, !n.isEmpty { rows.append(("Note", n)) }
        if let at = d.lockedAt { rows.append(("Locked", PanelText.when(at) + (d.lockedByName.map { " by \($0)" } ?? ""))) }
        if let l = d.revisionLabel, !l.isEmpty { rows.append(("Revision", l + (d.revisionColor.map { " \u{00B7} \($0)" } ?? ""))) }
        if let r = d.revisionReason, !r.isEmpty { rows.append(("Reason", r)) }
        return rows
    }
}

// MARK: - The inspector

/// The right-hand pane: Scene, Comments and Draft.
struct ScreenplayInspector: View {
    let model: ScreenplayWorkspaceModel
    @Bindable var state: ScreenplayPanelState
    /// The paragraph index of the caret (the workspace already tracks it).
    let caretIndex: Int
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Picker("Inspector", selection: $state.tab) {
                    ForEach(ScreenplayInspectorTab.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .accessibilityLabel("Inspector")
                Button { state.inspectorShown = false } label: { Label("Close", systemImage: "xmark.circle.fill") }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                    .help("Close the inspector")
                    .accessibilityLabel("Close")
                    .accessibilityIdentifier("inspector-close")
            }
            switch state.tab {
            case .scene: SceneHubPanel(workspace: model, state: state, sceneId: currentSceneId)
            case .comments: CommentsPanel(workspace: model, state: state)
            case .draft: DraftInfoPanel(workspace: model, state: state)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Screenplay inspector")
        .accessibilityIdentifier("screenplay-inspector")
    }

    /// The scene asked for by a menu, else the scene the caret is in.
    private var currentSceneId: String? {
        if let pinned = state.pinnedSceneId { return pinned }
        guard let view = model.textView else { return model.document?.scenes.first?.id }
        return ScreenplayCaret.sceneId(in: view.paragraphs(), at: caretIndex) ?? model.document?.scenes.first?.id
    }
}

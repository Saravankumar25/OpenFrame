// The Screenplay workspace (Mac phase M4): screenplay and draft choice, the scene navigator, the
// native editor and its sync with the core. The core decides everything; this file only moves
// text and selections between the editor and the core.

import AppKit
import SwiftUI
import OpenFrameCore

/// The paragraphs the editor shows for a document: each scene's heading, then its elements.
/// Line breaks inside an element are line separators (U+2028) in the editor and "\n" in the core.
enum ScreenplayParagraphs {
    static let lineSeparator = "\u{2028}"

    /// Editor paragraphs of a snapshot, through the shared block mapping of the model port.
    @MainActor
    static func from(_ snapshot: ScreenplaySnapshot, rules: ElementRules) -> [EditorParagraph] {
        ScreenplayModel.blocks(from: snapshot, rules: rules).map { b in
            EditorParagraph(
                kind: b.kind,
                text: b.text.replacingOccurrences(of: "\n", with: lineSeparator),
                elementId: b.kind == .sceneHeading ? nil : b.id,
                customElementId: b.customElementId,
                sceneId: b.kind == .sceneHeading ? b.id : nil)
        }
    }

    static func blocks(_ paragraphs: [EditorParagraph]) -> [EditorBlock] {
        paragraphs.map { p in
            EditorBlock(
                id: p.kind == .sceneHeading ? (p.sceneId ?? p.elementId) : p.elementId,
                kind: p.kind,
                text: p.text.replacingOccurrences(of: lineSeparator, with: "\n"),
                customElementId: p.customElementId)
        }
    }

    /// Read the editor as a snapshot. New or repeated ids are made unique and written back, so the
    /// next read sees the same ids.
    @MainActor
    static func read(_ view: ScreenplayTextView) -> ScreenplaySnapshot {
        var blocks = blocks(view.paragraphs())
        let before = blocks.map(\.id)
        ScreenplayModel.assignIds(&blocks)
        let after = blocks.map(\.id)
        if before != after { view.assign(ids: after) }
        return ScreenplayModel.snapshot(fromBlocks: blocks)
    }
}

@MainActor
@Observable
final class ScreenplayWorkspaceModel {
    /// A screenplay and draft asked for while the workspace was not on screen (Build Screenplay
    /// posts its target before the workspace appears). Read once when the workspace loads.
    static var pendingTarget: (screenplayId: String?, draftId: String?)?

    var overview: ScreenplayOverview?
    var document: ScreenplayDocument?
    var failure: CoreFailure?
    var syncStatus: String = "saved"
    var currentScene: String?
    var showNewDraft = false

    @ObservationIgnored weak var textView: ScreenplayTextView?
    @ObservationIgnored var sync: ScreenplaySync?
    @ObservationIgnored let rules: SharedScreenplayRules
    @ObservationIgnored private weak var app: AppModel?
    @ObservationIgnored private var screenplayId: String?

    init(rules: SharedScreenplayRules) {
        self.rules = rules
    }

    var draft: ScreenplayDraftDto? { document?.draft }
    var canEdit: Bool { document?.canEdit ?? false }

    func attach(_ app: AppModel) { self.app = app }

    func loadOverview() async {
        guard let core = app?.core else { return }
        do {
            var o = try await core.screenplayOverview(ScreenplayOverviewArgs(screenplayId: screenplayId))
            if o.screenplays.isEmpty, o.permissions.edit {
                // A project starts without a screenplay; the first visit makes one, as on Windows.
                _ = try await core.screenplayCreate(CreateScreenplayArgs())
                o = try await core.screenplayOverview(ScreenplayOverviewArgs(screenplayId: screenplayId))
            }
            overview = o
            screenplayId = o.screenplay?.id
            failure = nil
            if let draftId = o.screenplay?.currentDraftId ?? o.drafts.first?.id {
                await loadDocument(draftId)
            }
        } catch {
            failure = CoreFailure(asCore: error)
        }
    }

    func loadDocument(_ draftId: String) async {
        guard let core = app?.core else { return }
        do {
            let doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
            if let sync, sync.isDirty, doc.draft.id == document?.draft.id {
                // Never replace text the person is typing; the sync sends it first.
                return
            }
            let switching = doc.draft.id != document?.draft.id
            document = doc
            if switching || sync == nil {
                makeSync(for: doc)
                textView?.load(ScreenplayParagraphs.from(ScreenplayModel.snapshot(fromScenes: doc.scenes), rules: rules.table))
            } else {
                sync?.receiveServer(seq: doc.seq, snapshot: ScreenplayModel.snapshot(fromScenes: doc.scenes))
            }
            textView?.isEditable = doc.canEdit
        } catch {
            failure = CoreFailure(asCore: error)
        }
    }

    private func makeSync(for doc: ScreenplayDocument) {
        sync?.dispose()
        let draftId = doc.draft.id
        let table = rules.table
        sync = ScreenplaySync(
            initial: ScreenplayModel.snapshot(fromScenes: doc.scenes),
            seq: doc.seq,
            deps: ScreenplaySync.Deps(
                send: { [weak self] ops in
                    guard let core = self?.app?.core else {
                        throw CoreFailure(code: "internal.no_core", message: "OpenFrame is not ready.")
                    }
                    return try await core.screenplayApplyEdits(ScreenplayApplyEditsArgs(draftId: draftId, ops: ops)).seq
                },
                read: { [weak self] in
                    guard let view = self?.textView else { return [] }
                    return ScreenplayParagraphs.read(view)
                },
                apply: { [weak self] snap in
                    self?.textView?.load(ScreenplayParagraphs.from(snap, rules: table))
                },
                onError: { [weak self] error in
                    self?.app?.show("Your last change could not be saved", error)
                    return .retry
                },
                onStatus: { [weak self] status in
                    self?.syncStatus = "\(status)"
                }))
    }

    func textChanged() { sync?.markDirty() }

    /// Show a screenplay and draft chosen elsewhere (Build Screenplay in Story).
    func show(screenplayId: String?, draftId: String?) async {
        await flush()
        if let screenplayId { self.screenplayId = screenplayId }
        await loadOverview()
        if let draftId, draftId != draft?.id { await loadDocument(draftId) }
    }

    /// Store everything typed before leaving (workspace switch, close, quit).
    func flush() async { await sync?.flush() }

    // MARK: Drafts

    func switchDraft(_ id: String) async {
        await flush()
        guard let core = app?.core else { return }
        do {
            _ = try await core.screenplaySetCurrentDraft(ScreenplayDraftArgs(draftId: id))
            await loadOverview()
        } catch { app?.show("Could not switch the draft", error) }
    }

    func newDraft(name: String, note: String?) async {
        await flush()
        guard let core = app?.core, let source = draft?.id else { return }
        do {
            _ = try await core.screenplayNewDraft(ScreenplayNewDraftArgs(sourceDraftId: source, name: name, note: note, makeCurrent: true))
            await loadOverview()
        } catch { app?.show("Could not create the draft", error) }
    }

    func renumber() async {
        await flush()
        guard let core = app?.core, let id = draft?.id else { return }
        do { _ = try await core.screenplayRenumberScenes(ScreenplayDraftIdArgs(draftId: id)) }
        catch { app?.show("Could not renumber the scenes", error) }
    }
}

// MARK: - Views

struct ScreenplayWorkspace: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = ScreenplayWorkspaceModel(rules: SharedScreenplayRules.shared)
    @State private var currentKind: ElementType = .action
    @State private var currentIndex = 0
    @State private var newDraftName = ""

    private static let contentTables = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element", "screenplay_custom_element", "comment"]

    var body: some View {
        HSplitView {
            SceneNavigator(model: model, currentIndex: currentIndex)
                .frame(minWidth: 200, idealWidth: 240, maxWidth: 340)
            ZStack {
                theme["well"]
                ScreenplayEditorHost(model: model, onMove: { p, i in currentKind = p.kind; currentIndex = i })
            }
            .frame(minWidth: 560)
        }
        .toolbar { toolbar }
        .overlay { if let f = model.failure, model.document == nil { ContentUnavailableView("Screenplay unavailable", systemImage: "doc.text", description: Text(f.message)) } }
        .task {
            model.attach(app)
            if let target = ScreenplayWorkspaceModel.pendingTarget {
                ScreenplayWorkspaceModel.pendingTarget = nil
                await model.show(screenplayId: target.screenplayId, draftId: target.draftId)
            } else {
                await model.loadOverview()
            }
        }
        .task(id: app.changes.key(Self.contentTables)) {
            if let id = model.draft?.id { await model.loadDocument(id) }
        }
        .onDisappear { Task { await model.flush() } }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowScreenplayTarget)) { note in
            let info = note.userInfo ?? [:]
            ScreenplayWorkspaceModel.pendingTarget = nil
            Task { await model.show(screenplayId: info["screenplayId"] as? String, draftId: info["draftId"] as? String) }
        }
        .sheet(isPresented: $model.showNewDraft) { newDraftSheet }
        .accessibilityIdentifier("screenplay-workspace")
    }

    @ToolbarContentBuilder private var toolbar: some ToolbarContent {
        ToolbarItem(placement: .principal) {
            Picker("Element", selection: Binding(get: { currentKind }, set: { k in currentKind = k; model.textView?.setKind(k) })) {
                ForEach(SharedScreenplayRules.shared.menuKinds, id: \.self) { k in
                    Text(SharedScreenplayRules.shared.label(for: k)).tag(k)
                }
            }
            .frame(width: 170)
            .disabled(!model.canEdit)
            .help("Element of the current line (Command+1 to Command+8, Tab to change)")
            .accessibilityIdentifier("element-picker")
        }
        ToolbarItem(placement: .automatic) {
            Menu {
                if let drafts = model.overview?.drafts {
                    ForEach(drafts, id: \.id) { d in
                        Button { Task { await model.switchDraft(d.id) } } label: {
                            if d.isCurrent { Label(d.name, systemImage: "checkmark") } else { Text(d.name) }
                        }
                    }
                }
                Divider()
                Button("New Draft…") { newDraftName = ""; model.showNewDraft = true }.disabled(!model.canEdit && model.draft?.lockedAt == nil)
                Button("Renumber Scenes") { Task { await model.renumber() } }.disabled(!model.canEdit)
            } label: {
                Label(model.draft?.name ?? "Draft", systemImage: "square.stack")
            }
            .help("Drafts")
            .accessibilityIdentifier("draft-menu")
        }
        ToolbarItem(placement: .automatic) {
            Text(model.syncStatus == "saved" ? "Saved" : model.syncStatus == "error" ? "Not saved" : "Saving")
                .font(.caption)
                .foregroundStyle(model.syncStatus == "error" ? theme["red-t"] : .secondary)
                .accessibilityIdentifier("editor-save-status")
        }
    }

    private var newDraftSheet: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("New Draft").font(.title3.weight(.semibold))
            Text("The new draft starts as a copy of \(model.draft?.name ?? "the current draft"). The current draft is kept as it is.")
                .foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            TextField("Name", text: $newDraftName, prompt: Text("Second draft"))
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { model.showNewDraft = false }.keyboardShortcut(.cancelAction)
                Button("Create") {
                    let name = newDraftName.trimmingCharacters(in: .whitespaces)
                    model.showNewDraft = false
                    Task { await model.newDraft(name: name, note: nil) }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(newDraftName.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(24).frame(width: 420)
    }
}

/// Scenes of the draft, numbered as the core numbers them. Clicking a scene moves the caret there.
struct SceneNavigator: View {
    let model: ScreenplayWorkspaceModel
    let currentIndex: Int
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let scenes = model.document?.scenes ?? []
        List {
            Section("Scenes (\(scenes.count))") {
                ForEach(Array(scenes.enumerated()), id: \.element.id) { i, scene in
                    Button {
                        model.textView?.reveal(paragraph: paragraphIndex(of: i, in: scenes))
                        model.textView?.window?.makeFirstResponder(model.textView)
                    } label: {
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text(scene.label).font(.caption.monospacedDigit()).foregroundStyle(.secondary).frame(minWidth: 22, alignment: .trailing)
                            Text(scene.heading.isEmpty ? "Untitled scene" : scene.heading.uppercased())
                                .font(.callout).lineLimit(2)
                        }
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Scene \(scene.label), \(scene.heading)")
                }
            }
        }
        .listStyle(.sidebar)
        .accessibilityIdentifier("scene-navigator")
    }

    private func paragraphIndex(of scene: Int, in scenes: [ScreenplaySceneDto]) -> Int {
        scenes.prefix(scene).reduce(0) { $0 + 1 + $1.elements.count }
    }
}

/// The text view inside a scroll view, centred on a page-coloured column.
struct ScreenplayEditorHost: NSViewRepresentable {
    let model: ScreenplayWorkspaceModel
    let onMove: (EditorParagraph, Int) -> Void
    @Environment(\.ofTheme) private var theme

    func makeCoordinator() -> Coordinator { Coordinator(model: model, onMove: onMove) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        let view = ScreenplayTextView.make()
        view.rules = model.rules
        view.screenplayDelegate = context.coordinator
        view.minSize = NSSize(width: 0, height: 0)
        view.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        view.autoresizingMask = [.width]
        scroll.documentView = view
        model.textView = view
        if let doc = model.document {
            view.load(ScreenplayParagraphs.from(ScreenplayModel.snapshot(fromScenes: doc.scenes), rules: model.rules.table))
            view.isEditable = doc.canEdit
        }
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let view = scroll.documentView as? ScreenplayTextView else { return }
        view.backgroundColor = NSColor(theme["panel"])
        view.insertionPointColor = NSColor(theme["ink"])
        context.coordinator.onMove = onMove
    }

    @MainActor
    final class Coordinator: NSObject, ScreenplayTextViewDelegate {
        let model: ScreenplayWorkspaceModel
        var onMove: (EditorParagraph, Int) -> Void
        init(model: ScreenplayWorkspaceModel, onMove: @escaping (EditorParagraph, Int) -> Void) {
            self.model = model
            self.onMove = onMove
        }
        func screenplayTextDidChange(_ view: ScreenplayTextView) { model.textChanged() }
        func screenplayTextView(_ view: ScreenplayTextView, didMoveInto paragraph: EditorParagraph, index: Int) {
            onMove(paragraph, index)
        }
    }
}

// The Idea Vault workspace: collect anything about the film without deciding what it is. A left
// panel of views, a canvas of cards or rows, an inspector for the open item, and sheets for the
// few things that need a question. It serves the project vault and the Global Idea Vault.

import AppKit
import OpenFrameCore
import SwiftUI
import UniformTypeIdentifiers

struct IdeaVaultWorkspace: View {
    let store: StoreSel
    @Environment(AppModel.self) private var app
    @State private var model: IdeaVaultModel?

    var body: some View {
        Group {
            if let model {
                IdeaVaultScreen(model: model)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .onAppear {
            if model == nil { model = IdeaVaultModel(app: app, store: store) }
        }
    }
}

/// The Global Idea Vault with no project open (shown from Home).
public struct GlobalIdeaVaultView: View {
    public init() {}
    public var body: some View {
        IdeaVaultWorkspace(store: .global)
            .accessibilityIdentifier("global-idea-vault")
    }
}

private struct ReloadKey: Hashable {
    var changes: Int
    var scope: StoreSel
    var view: String
    var mode: VaultDisplayMode
    var search: String
}

private enum ImportKind: Identifiable {
    case image, file, audio, video
    var id: Self { self }
    var types: [UTType] {
        switch self {
        case .image: [.image]
        case .file: [.item]
        case .audio: [.audio]
        case .video: [.movie, .video]
        }
    }
    var itemType: VaultItemType? {
        switch self {
        case .image: .image
        case .audio: .audio
        case .video: .video
        case .file: nil
        }
    }
}

struct IdeaVaultScreen: View {
    @Bindable var model: IdeaVaultModel
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage("vault.mode") private var storedMode = VaultDisplayMode.grid.rawValue
    @State private var importKind: ImportKind?
    @State private var importing = false
    @State private var dropTargeted = false

    private var reloadKey: ReloadKey {
        ReloadKey(
            changes: model.app.changes.key(VaultTables.all, store: model.storeKind),
            scope: model.scope, view: model.listView.key, mode: model.mode, search: model.search)
    }

    var body: some View {
        Group {
            if model.page == .deleted {
                VaultDeletedView(model: model)
            } else {
                vault
            }
        }
        .background(theme["bg"])
        .themedSheet(item: $model.dialog) { dialogView($0) }
        .task(id: reloadKey) { await model.reload() }
        .task(id: model.searchText) {
            try? await Task.sleep(for: .milliseconds(200))
            if !Task.isCancelled { model.applySearch() }
        }
        .onAppear { if let m = VaultDisplayMode(rawValue: storedMode) { model.mode = m } }
        .onChange(of: model.mode) { _, m in storedMode = m.rawValue }
        .overlay(alignment: .bottom) { noticeBar }
        .onProjectIntent("vault.new_note") { Task { await model.createText(.note) } }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameQuickCapture)) { _ in
            if model.page == .vault { model.dialog = .quick }
        }
        .accessibilityIdentifier("idea-vault")
    }

    // MARK: Vault page

    private var vault: some View {
        GeometryReader { geo in
            HStack(spacing: 0) {
                // A narrow window (or the AI Assistant beside the vault) leaves the items more room.
                VaultSidebar(model: model).frame(width: geo.size.width < 640 ? 170 : 214)
                Divider().overlay(theme["line"])
                main
            }
        }
        .inspector(isPresented: Binding(get: { model.openId != nil }, set: { if !$0 { model.closeDrawer() } })) {
            if let id = model.openId {
                VaultDrawer(model: model, id: id)
                    .inspectorColumnWidth(min: 320, ideal: 380, max: 520)
            }
        }
        .searchable(text: $model.searchText, placement: .toolbar, prompt: "Search ideas, tags, filenames…")
        .fileImporter(isPresented: $importing, allowedContentTypes: importKind?.types ?? [.item], allowsMultipleSelection: true) { result in
            handleImport(result)
        }
    }

    private var main: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            actionBar
            orphanBanner
            if model.mode == .folder && !model.searching {
                VaultCrumbs(path: VaultFolders.path(model.folders, to: model.folderCursor)) { model.openFolder($0) }
                    .padding(.horizontal, 24)
                    .padding(.bottom, 8)
            }
            if model.searching && model.loaded {
                Text("\(model.items.count) \(model.items.count == 1 ? "result" : "results") for “\(model.search)”")
                    .font(.callout).foregroundStyle(theme["muted"])
                    .padding(.horizontal, 24).padding(.bottom, 8)
                    .accessibilityAddTraits(.updatesFrequently)
            }
            ZStack(alignment: .bottom) {
                canvas
                if !model.selection.isEmpty { selectionBar.padding(.bottom, 14) }
                if dropTargeted { dropHint }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .dropDestination(for: URL.self) { urls, _ in
            let files = urls.filter(\.isFileURL)
            guard !files.isEmpty else { return false }
            Task { await addAccessing(files) }
            return true
        } isTargeted: { dropTargeted = $0 }
    }

    private var header: some View {
        VStack(spacing: PageSpacing.tight) {
            HStack(spacing: 10) {
                Text("Idea Vault").font(.system(size: 26, weight: .semibold)).tracking(-0.3)
                Text(model.isGlobal ? "Global" : (model.project?.title ?? "Project"))
                    .font(.caption.weight(.medium))
                    .padding(.horizontal, 8).padding(.vertical, 2)
                    .background(model.isGlobal ? theme["purple-soft"] : theme["accent-soft"], in: Capsule())
                    .foregroundStyle(model.isGlobal ? theme["purple-t"] : theme["accent-ink"])
                    .accessibilityLabel(model.isGlobal ? "Global Idea Vault" : "Project Idea Vault")
            }
            Text(model.isGlobal
                ? "Ideas across your whole filmmaking life. Copy them into a film when they are ready."
                : "Everything collected for this film. Nothing here is classified or synced to the screenplay.")
                .foregroundStyle(theme["muted"])
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 24)
        .padding(.top, 20)
        .padding(.bottom, 12)
    }

    /// Files in the vault folder that are in no item: offer to add them back.
    @ViewBuilder private var orphanBanner: some View {
        if let o = model.orphans, o.count > 0 {
            IXBanner(tone: .info, text: Text(StorageRules.orphanBanner(o.count))) {
                Button("Add Them Back") { Task { await model.recoverFiles() } }
                    .buttonStyle(.ofProminent)
                    .disabled(model.recovering)
                    .accessibilityIdentifier("vault-add-back")
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 8)
            .accessibilityIdentifier("vault-orphans")
        }
    }

    // MARK: Toolbar

    /// The Idea Vault's own controls in a row under the page title (Windows: the vault toolbar
    /// inside the page). With every button named they no longer fit the window toolbar, and macOS
    /// moved the scope picker, Record Note and Quick Capture into its overflow menu.
    private var actionBar: some View {
        BDFlowLayout(spacing: 8) {
            if model.hasProject {
                Picker("Which Idea Vault", selection: Binding(get: { model.requested }, set: { model.chooseStore($0) })) {
                    Text("Global").tag(StoreSel.global)
                    Text("This Project").tag(StoreSel.project)
                }
                .pickerStyle(.segmented)
                .help("Which Idea Vault")
                .accessibilityLabel("Which Idea Vault")
                .accessibilityIdentifier("vault-scope")
                .fixedSize()
            }
            Menu {
                Button("Note") { Task { await model.createText(.note) } }
                Button("Image…") { startImport(.image) }
                Button("URL…") { model.dialog = .url }
                Button("File…") { startImport(.file) }
                Button("Audio / Voice Note…") { model.dialog = .record }
                Button("Audio File…") { startImport(.audio) }
                Button("Video…") { startImport(.video) }
                Button("Quote") { Task { await model.createText(.quote) } }
                Divider()
                Button("Quick Capture") { model.dialog = .quick }
            } label: {
                Label("Add", systemImage: "plus").labelStyle(.titleAndIcon)
            }
            .help("Add to the Idea Vault")
            .accessibilityIdentifier("vault-add")
            Button { model.dialog = .record } label: { Label("Record Note", systemImage: "mic").labelStyle(.titleAndIcon) }
                .help("Record a voice note")
                .accessibilityIdentifier("vault-record")
            Button { model.dialog = .quick } label: { Label("Quick Capture", systemImage: "bolt").labelStyle(.titleAndIcon) }
                .keyboardShortcut("n", modifiers: [.command, .shift])
                .help("Quick Capture (Shift-Command-N)")
                .accessibilityIdentifier("vault-quick-capture")
            Picker("View", selection: Binding(get: { model.mode }, set: { model.setMode($0) })) {
                ForEach(VaultDisplayMode.allCases) { Label($0.title, systemImage: $0.symbol).labelStyle(.titleAndIcon).tag($0) }
            }
            .pickerStyle(.segmented)
            .help("Change how items are shown")
            .accessibilityLabel("View")
            .accessibilityIdentifier("vault-view-mode")
            .fixedSize()
            Button { model.page = .deleted } label: { Label("Recently Deleted", systemImage: "trash").labelStyle(.titleAndIcon) }
                .help("Recently Deleted")
                .accessibilityIdentifier("vault-recently-deleted")
            ControlGroup {
                undoButton
                redoButton
            }
            .fixedSize()
        }
        .controlSize(.small)
        .padding(.horizontal, 16).padding(.bottom, 8)
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("vault-action-bar")
    }

    // With no project open the Edit menu's project undo is off, so the Global vault answers the
    // same shortcut itself.
    @ViewBuilder private var undoButton: some View {
        let button = Button { Task { await model.undo() } } label: { Label("Undo", systemImage: "arrow.uturn.backward").labelStyle(.titleAndIcon) }
            .help("Undo the last change in the \(VaultText.scopeLabel(model.scope))")
            .accessibilityIdentifier("vault-undo")
        if model.hasProject { button } else { button.keyboardShortcut("z", modifiers: [.command, .option]) }
    }

    @ViewBuilder private var redoButton: some View {
        let button = Button { Task { await model.redo() } } label: { Label("Redo", systemImage: "arrow.uturn.forward").labelStyle(.titleAndIcon) }
            .help("Redo in the \(VaultText.scopeLabel(model.scope))")
            .accessibilityIdentifier("vault-redo")
        if model.hasProject { button } else { button.keyboardShortcut("z", modifiers: [.command, .option, .shift]) }
    }

    // MARK: Canvas

    private var columns: [GridItem] {
        switch model.mode {
        case .card: [GridItem(.adaptive(minimum: 380, maximum: 380), spacing: 14, alignment: .top)]
        default: [GridItem(.adaptive(minimum: 250, maximum: 250), spacing: 14, alignment: .top)]
        }
    }

    private var subfolders: [VaultFolderDto] {
        model.mode == .folder && !model.searching ? VaultFolders.children(model.folders, of: model.folderCursor) : []
    }

    private var canvas: some View {
        // A pane narrower than one card scrolls sideways instead of cutting the cards.
        GeometryReader { geo in
        ScrollView([.vertical, .horizontal]) {
            AtLeastWidth(width: geo.size.width) {
            LazyVStack(alignment: .leading, spacing: 14) {
                if let failure = model.loadFailure {
                    Text(failure.message).foregroundStyle(theme["red-t"]).accessibilityIdentifier("vault-load-error")
                }
                if !model.loaded {
                    ProgressView().frame(maxWidth: .infinity).padding(.top, 40)
                } else if model.items.isEmpty && subfolders.isEmpty {
                    emptyState.frame(maxWidth: .infinity)
                } else {
                    if !subfolders.isEmpty {
                        LazyVGrid(columns: columns, alignment: .center, spacing: 14) {
                            ForEach(subfolders, id: \.id) { f in
                                VaultFolderTile(folder: f, open: { model.openFolder(f.id) }, menu: { AnyView(folderMenu(f)) })
                            }
                        }
                        .frame(maxWidth: .infinity)
                    }
                    let sections = VaultSections.build(model.items, splitPinned: model.view == .all && !model.searching && model.mode != .folder,
                                                       header: !subfolders.isEmpty ? "Items in this folder" : nil)
                    ForEach(sections) { section in
                        if let h = section.header {
                            Text(h).font(.subheadline.weight(.semibold)).foregroundStyle(theme["ink2"]).padding(.top, 4)
                                .accessibilityAddTraits(.isHeader)
                        }
                        if model.mode == .list {
                            LazyVStack(spacing: 6) {
                                ForEach(section.items, id: \.id) { item in
                                    VaultItemRow(item: item, model: model, selected: model.selection.contains(item.id), actions: actions(for: item))
                                }
                            }
                        } else {
                            LazyVGrid(columns: columns, alignment: .center, spacing: 14) {
                                ForEach(section.items, id: \.id) { item in
                                    VaultItemCard(item: item, model: model, selected: model.selection.contains(item.id), full: model.mode == .card, actions: actions(for: item))
                                }
                            }
                            .frame(maxWidth: .infinity)
                        }
                    }
                }
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 80)
            .animation(reduceMotion ? nil : .spring(response: 0.3, dampingFraction: 1), value: model.items.map(\.id))
            }
        }
        .scrollBounceBehavior(.basedOnSize)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Idea Vault items")
        .accessibilityIdentifier("vault-items")
        .focusable()
        .focusEffectDisabled()
        .onKeyPress(keys: [.delete, .deleteForward], phases: .down) { _ in
            guard !model.selectedItems.isEmpty else { return .ignored }
            Task { await model.delete(model.selectedItems) }
            return .handled
        }
        .onKeyPress(.escape, phases: .down) { _ in
            guard !model.selection.isEmpty else { return .ignored }
            model.selection = []
            return .handled
        }
        .onKeyPress(.return, phases: .down) { _ in
            guard let first = model.selectedItems.first else { return .ignored }
            model.open(first.id)
            return .handled
        }
        .onKeyPress(characters: ["a"], phases: .down) { press in
            guard press.modifiers.contains(.command) else { return .ignored }
            model.selection = Set(model.items.map(\.id))
            return .handled
        }
        .onKeyPress(keys: [KeyEquivalent(Character(UnicodeScalar(NSF2FunctionKey)!))], phases: .down) { _ in
            guard let first = model.selectedItems.first else { return .ignored }
            model.open(first.id, focusTitle: true)
            return .handled
        }
    }

    @ViewBuilder private var emptyState: some View {
        if model.searching {
            ContentUnavailableView {
                Label("No results for “\(model.search)”", systemImage: "magnifyingglass")
            } description: {
                Text("Search looks at titles, note text, captions, filenames and tags.")
            }
        } else if model.total == 0 {
            ContentUnavailableView {
                Label(model.isGlobal ? "Drop any film idea, reference or file here." : "Start collecting anything about this film.", systemImage: "lightbulb")
            } description: {
                Text("This is where you throw anything about the film. Add a note, image, link, file, or voice note. You never have to decide what it is first.")
            } actions: {
                Button("Add Note") { Task { await model.createText(.note) } }
                Button("Record Note") { model.dialog = .record }
            }
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("vault-empty")
        } else {
            let text: String = {
                switch model.listView {
                case .pinned: return "Nothing pinned yet. Pin items to keep them at the top."
                case .folder: return "This folder is empty. Move items here or add new ones while it's open."
                case .collection: return "This collection is empty. Add items to it from their menu or the selection bar."
                case .tag: return "No items have this tag any more."
                default: return "Nothing here yet."
                }
            }()
            ContentUnavailableView(text, systemImage: "tray")
        }
    }

    // MARK: Selection bar, drop hint, notice

    private var selectionBar: some View {
        let xs = model.selectedItems
        return HStack(spacing: 10) {
            Text("\(xs.count) selected").fontWeight(.semibold)
            Divider().frame(height: 16)
            Button("Move to Folder") { model.dialog = .move(xs) }
            Button("Add to Collection") { model.dialog = .collection(xs) }
            Button("Tag") { model.dialog = .tag(xs) }
            Button(xs.allSatisfy(\.pinned) ? "Unpin" : "Pin") { Task { await model.setPinned(xs, !xs.allSatisfy(\.pinned)) } }
            if let label = model.copyLabel { Button(label) { model.dialog = .copy(xs) } }
            Button("Delete", role: .destructive) { Task { await model.delete(xs) } }
            Button { model.selection = [] } label: { Label("Clear", systemImage: "xmark").labelStyle(.titleAndIcon) }
                .buttonStyle(.borderless)
                .accessibilityLabel("Clear selection")
        }
        .controlSize(.small)
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
        .background(.regularMaterial, in: Capsule())
        .overlay(Capsule().strokeBorder(theme["line"]))
        .shadow(color: .black.opacity(0.12), radius: 8, y: 2)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Selected items")
    }

    private var dropHint: some View {
        VStack(spacing: 6) {
            Image(systemName: "tray.and.arrow.down").font(.largeTitle)
            Text("Drop to add to the \(model.isGlobal ? "Global Vault" : "Project Vault")").font(.headline)
            Text("No categories needed. They will be stored exactly as they are.").font(.callout)
        }
        .foregroundStyle(theme["accent-ink"])
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme["accent-soft"].opacity(0.92))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["accent"], style: StrokeStyle(lineWidth: 2, dash: [6])))
        .allowsHitTesting(false)
        .accessibilityElement(children: .combine)
    }

    @ViewBuilder private var noticeBar: some View {
        if let notice = model.notice {
            HStack(spacing: 12) {
                Text(notice.text).lineLimit(3).textSelection(.enabled)
                if let store = notice.undoStore {
                    Button("Undo") {
                        model.dismissNotice()
                        Task { await model.undo(in: store) }
                    }
                    .buttonStyle(.link)
                }
                if notice.action == .openStory {
                    Button("Open Beat Board") {
                        model.dismissNotice()
                        ProjectIntents.shared.fire(StoryTab.board.showIntent)
                        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.story.rawValue)
                    }
                    .buttonStyle(.link)
                }
                Button { model.dismissNotice() } label: { Label("Dismiss", systemImage: "xmark").labelStyle(.titleAndIcon) }
                    .buttonStyle(.borderless)
                    .accessibilityLabel("Dismiss")
            }
            .font(.callout)
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
            .background(.regularMaterial, in: Capsule())
            .overlay(Capsule().strokeBorder(theme["line"]))
            .shadow(color: .black.opacity(0.12), radius: 8, y: 2)
            .padding(.bottom, model.selection.isEmpty ? 16 : 64)
            .padding(.horizontal, 24)
            .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
            .accessibilityElement(children: .contain)
            .accessibilityAddTraits(.updatesFrequently)
            .accessibilityIdentifier("vault-notice")
            .task(id: notice.id) {
                try? await Task.sleep(for: .seconds(9))
                if !Task.isCancelled, model.notice?.id == notice.id { model.dismissNotice() }
            }
        }
    }

    // MARK: Actions

    private func actions(for item: VaultItemDto) -> VaultItemActions {
        VaultItemActions(
            open: { model.open(item.id) },
            rename: { model.open(item.id, focusTitle: true) },
            move: { model.dialog = .move(model.targets(for: item)) },
            pin: {
                let xs = model.targets(for: item)
                Task { await model.setPinned(xs, !xs.allSatisfy(\.pinned)) }
            },
            copy: model.copyLabel == nil ? nil : { model.dialog = .copy(model.targets(for: item)) },
            sendToStory: model.hasProject ? { model.dialog = .send(item) } : nil,
            addToCollection: { model.dialog = .collection(model.targets(for: item)) },
            tag: { model.dialog = .tag(model.targets(for: item)) },
            flag: { flag in
                let xs = model.targets(for: item)
                Task { await model.setResearchFlag(xs, flag) }
            },
            delete: {
                let xs = model.targets(for: item)
                Task { await model.delete(xs) }
            })
    }

    @ViewBuilder private func folderMenu(_ f: VaultFolderDto) -> some View {
        Button("Open") { model.openFolder(f.id) }
        Button("Rename…") { model.dialog = .renameFolder(f) }
        Button("New Folder Inside…") { model.dialog = .newFolder(parentId: f.id) }
        Divider()
        Button("Delete Folder", role: .destructive) { Task { await model.deleteFolder(f) } }
    }

    private func startImport(_ kind: ImportKind) {
        importKind = kind
        importing = true
    }

    private func handleImport(_ result: Result<[URL], Error>) {
        let kind = importKind
        switch result {
        case let .success(urls):
            Task { await addAccessing(urls, itemType: kind?.itemType) }
        case let .failure(error):
            model.app.show("Could not add the files", error)
        }
    }

    /// Picked and dropped files may be outside the app's own folders; hold access while the core reads them.
    private func addAccessing(_ urls: [URL], itemType: VaultItemType? = nil) async {
        let held = urls.filter { $0.startAccessingSecurityScopedResource() }
        defer { held.forEach { $0.stopAccessingSecurityScopedResource() } }
        await model.addFiles(urls.map(\.path), itemType: itemType)
    }

    // MARK: Sheets

    @ViewBuilder private func dialogView(_ dialog: VaultDialog) -> some View {
        let close = { model.dialog = nil }
        switch dialog {
        case .url:
            AddURLSheet(model: model, close: close)
        case .record:
            RecordNoteSheet(model: model, close: close, chooseFile: { startImport(.audio) })
        case .quick:
            QuickCaptureSheet(model: model, close: close)
        case let .send(item):
            SendToStorySheet(model: model, item: item, close: close)
        case let .copy(items):
            CopySheet(model: model, items: items, close: close)
        case let .move(items):
            MoveToFolderSheet(model: model, items: items, close: close)
        case let .collection(items):
            AddToCollectionSheet(model: model, items: items, close: close)
        case let .tag(items):
            TagSheet(model: model, items: items, close: close)
        case let .result(result):
            AddResultSheet(model: model, result: result, close: close)
        case .newCollection:
            NameSheet(title: "New Collection", label: "Name", confirmLabel: "Create", hint: VaultHints.collection, initial: "", close: close) { name in
                if let c = try await model.createCollection(name: name) { model.show(.collection(collectionId: c.id)) }
            }
        case let .newFolder(parentId):
            NameSheet(title: "New Folder", label: "Folder name", confirmLabel: "Create", hint: nil, initial: "", close: close) { name in
                try await model.createFolder(name: name, parentId: parentId)
            }
        case let .renameFolder(f):
            NameSheet(title: "Rename Folder", label: "Folder name", confirmLabel: "Rename", hint: nil, initial: f.name, close: close) { name in
                try await model.renameFolder(f, name: name)
            }
        case let .renameCollection(c):
            NameSheet(title: "Rename Collection", label: "Name", confirmLabel: "Rename", hint: nil, initial: c.name, close: close) { name in
                try await model.renameCollection(c, name: name)
            }
        }
    }
}

extension Notification.Name {
    /// Asks the open Idea Vault to show Quick Capture (from the app's quick-actions menu).
    static let openFrameQuickCapture = Notification.Name("studio.openframe.vault.quickCapture")
}

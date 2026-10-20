// Project Files (Windows: workspaces/files). "What files belong with this film?" A light file
// cabinet: folders on the left, the files in a list or a grid, a details drawer on the right.
// Files can be added with the system panel or by dragging them in from Finder, and moved by
// dragging them onto a folder or with Move to in the menu.

import AppKit
import OpenFrameCore
import SwiftUI
import UniformTypeIdentifiers

struct FilesWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var model: FilesModel?

    var body: some View {
        Group {
            if let model {
                FilesScreen(model: model)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .fitsWindow()
        .onAppear {
            if model == nil { model = FilesModel(app: app) }
        }
    }
}

struct FilesScreen: View {
    @Bindable var model: FilesModel
    @Environment(\.ofTheme) private var theme
    @AppStorage("files.view") private var storedView = FilesView.list.rawValue

    private var view: FilesView { FilesView(rawValue: storedView) ?? .list }

    var body: some View {
        ScrollingPage {
        VStack(alignment: .leading, spacing: 0) {
            ProjectPageHeader(title: "Project Files", subtitle: "What files belong with this film?", identifier: "files") {
                HStack {
                    Button { model.dialog = .newFolder(parentId: model.targetFolderId) } label: { Label("New Folder", systemImage: "folder.badge.plus") }
                        .accessibilityIdentifier("new-folder")
                    Menu {
                        Button("Copy into project\u{2026}") { startAdd(copy: true) }
                            .accessibilityIdentifier("add-file-copy")
                        Button("Link to file\u{2026}") { startAdd(copy: false) }
                            .accessibilityIdentifier("add-file-link")
                    } label: { Label("Add File", systemImage: "plus") }
                        .menuStyle(.borderedButton)
                        .accessibilityIdentifier("add-file")
                }
            }
            toolbar
            if let failure = model.loadFailure {
                Text(failure.message).foregroundStyle(theme["red-t"]).multilineTextAlignment(.center).frame(maxWidth: .infinity).padding(.horizontal, 24).padding(.bottom, 8)
                    .accessibilityIdentifier("files-load-error")
            }
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                HStack(alignment: .top, spacing: 14) {
                    FolderPanel(model: model)
                    content
                }
                .pageColumn(maxWidth: PageSpacing.wideColumn)
                .padding(.bottom, PageSpacing.medium)
                .frame(maxHeight: .infinity, alignment: .top)
            }
        }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["bg"])
        .overlay { if model.dropping { dropHint } }
        .dropDestination(for: URL.self) { urls, _ in
            let files = urls.filter(\.isFileURL)
            guard !files.isEmpty else { return false }
            Task { await model.addPicked(files, mode: "copy") }
            return true
        } isTargeted: { model.dropping = $0 }
        .inspector(isPresented: Binding(get: { model.current != nil }, set: { if !$0 { model.select(nil) } })) {
            if let f = model.current {
                FileDetailsView(files: model, id: f.id).id(f.id)
                    .inspectorColumnWidth(min: 320, ideal: 380, max: 520)
            }
        }
        .sheet(item: $model.dialog) { dialogView($0) }
        .task(id: model.app.changes.key(["project_file", "project_file_folder", "asset", "deleted_item"])) { await model.reload() }
        .onAppear { model.apply(PageNav.take()) }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowWorkspace)) { _ in model.apply(PageNav.take()) }
        .onProjectIntent("files.add") { startAdd(copy: true) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("files-workspace")
    }

    private var toolbar: some View {
        HStack(spacing: 10) {
            HStack(spacing: 4) {
                Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                TextField("Search files", text: $model.text)
                    .textFieldStyle(.plain)
                    .accessibilityLabel("Search files")
                    .accessibilityIdentifier("files-search")
            }
            .padding(.horizontal, 8).padding(.vertical, 5)
            .frame(width: 260)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(theme["line"]))
            Picker("View", selection: $storedView) {
                ForEach(FilesView.allCases) { Label($0.title, systemImage: $0.symbol).tag($0.rawValue) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 120)
            .accessibilityLabel("View")
            .accessibilityIdentifier("files-view")
            Text("Drag files here from Finder to add them.").font(.caption).foregroundStyle(theme["muted"])
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, PageSpacing.large)
        .padding(.bottom, PageSpacing.medium)
    }

    private var dropHint: some View {
        VStack(spacing: 6) {
            Image(systemName: "square.and.arrow.down").font(.system(size: 28))
            Text("Drop to add to \(model.targetFolderId.map { model.folderName($0) } ?? "this project")").fontWeight(.semibold)
            Text("A copy is stored inside the project.").foregroundStyle(theme["muted"])
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme["panel"].opacity(0.93))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).stroke(theme["accent"], style: StrokeStyle(lineWidth: 2, dash: [6])).padding(12))
        .allowsHitTesting(false)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("files-drop-hint")
    }

    // MARK: Content

    @ViewBuilder private var content: some View {
        if model.files.isEmpty {
            ContentUnavailableView {
                Label("Keep project documents and attachments here.", systemImage: "folder")
            } description: {
                Text("Contracts, permits, research, pitch decks: anything that belongs with this film. Drag files in, or add them.")
            } actions: {
                Button { startAdd(copy: true) } label: { Label("Add File", systemImage: "plus") }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("files-empty-add")
            }
            .frame(maxWidth: .infinity, minHeight: 260)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("files-empty")
        } else if let message = model.emptyMessage {
            Text(message)
                .foregroundStyle(theme["muted"])
                .frame(maxWidth: .infinity).padding(20)
                .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                .accessibilityIdentifier("files-none-shown")
        } else if view == .list {
            Group { listView }
                .focusable()
                .focusEffectDisabled()
                .onKeyPress(.delete) { deleteSelected() }
                .onKeyPress(.deleteForward) { deleteSelected() }
                .onKeyPress(KeyEquivalent(Character(UnicodeScalar(NSF2FunctionKey)!))) {
                    guard let f = model.current else { return .ignored }
                    model.askRename(f)
                    return .handled
                }
        } else {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 220, maximum: 340), spacing: 12, alignment: .top)], spacing: 12) {
                ForEach(model.shown, id: \.id) { f in FileCard(model: model, file: f) }
            }
            .accessibilityIdentifier("files-grid")
        }
    }

    private func deleteSelected() -> KeyPress.Result {
        guard let f = model.current else { return .ignored }
        Task { await model.delete(f) }
        return .handled
    }

    private var listView: some View {
        PageCard {
            HStack(spacing: 8) {
                Text("Name").frame(maxWidth: .infinity, alignment: .leading)
                Text("Type").frame(width: 90, alignment: .leading)
                Text("Where").frame(width: 190, alignment: .leading)
                Text("Modified").frame(width: 90, alignment: .leading)
                Text("Actions").frame(width: 130, alignment: .trailing).opacity(0).accessibilityHidden(true)
            }
            .font(.caption.weight(.semibold))
            .foregroundStyle(theme["muted"])
            .padding(.horizontal, 12).padding(.vertical, 8)
            .accessibilityHidden(true)
            Divider().overlay(theme["line"])
            LazyVStack(spacing: 0) {
                ForEach(model.shown, id: \.id) { f in
                    FileRow(model: model, file: f)
                    Divider().overlay(theme["line"])
                }
            }
            .accessibilityIdentifier("files-list")
        }
    }

    // MARK: Adding

    private func startAdd(copy: Bool) {
        let urls = PageFilePanels.chooseFiles(copy: copy)
        guard !urls.isEmpty else { return }
        Task { await model.addPicked(urls, mode: copy ? "copy" : "link") }
    }

    @ViewBuilder private func dialogView(_ dialog: FilesDialog) -> some View {
        switch dialog {
        case let .newFolder(parent):
            FolderNameSheet(
                title: "New Folder", subtitle: parent.map { "Inside \u{201C}\(model.folderPath($0))\u{201D}" },
                initial: "", confirm: "Create", close: { model.dialog = nil }
            ) { name in await model.createFolder(name: name, parentId: parent) != nil }
        case let .renameFolder(f):
            FolderNameSheet(title: "Rename Folder", subtitle: nil, initial: f.name, confirm: "Rename", close: { model.dialog = nil }) { name in
                await model.renameFolder(f, to: name)
            }
        case let .deleteFolder(f):
            DeleteFolderSheet(folder: f, close: { model.dialog = nil }) {
                await model.deleteFolder(f)
            }
        }
    }
}

// MARK: Menus shared by rows, cards and the details drawer

@MainActor
struct FileMenu: View {
    let model: FilesModel
    let file: ProjectFileDto

    var body: some View {
        let where_ = FilesModel.whereOf(file)
        if model.isUnavailableLink(file) {
            Button("Relink\u{2026}") { relink() }.accessibilityIdentifier("file-menu-relink")
        } else {
            Button("Open") { Task { await model.open(file) } }
                .disabled(where_ == .unavailable)
                .accessibilityIdentifier("file-menu-open")
        }
        Button("Details") { model.select(file.id) }.accessibilityIdentifier("file-menu-details")
        Button("Rename") { model.askRename(file) }.accessibilityIdentifier("file-menu-rename")
        Divider()
        Menu("Move to") {
            Button("Top level") { Task { await model.move(file, to: nil) } }
                .disabled(file.folderId == nil)
                .accessibilityIdentifier("file-move-top")
            ForEach(model.folderTree, id: \.folder.id) { entry in
                Button(model.folderPath(entry.folder.id)) { Task { await model.move(file, to: entry.folder.id) } }
                    .disabled(file.folderId == entry.folder.id)
                    .accessibilityIdentifier("file-move-\(entry.folder.id)")
            }
        }
        .accessibilityIdentifier("file-menu-move")
        Divider()
        Button("Reveal in Finder") { Task { await model.reveal(file) } }
            .disabled(where_ == .unavailable)
            .accessibilityIdentifier("file-menu-reveal")
        Button("Export Copy\u{2026}") { exportCopy() }
            .disabled(where_ == .unavailable)
            .accessibilityIdentifier("file-menu-export")
        if file.asset.storageMode == "external" && where_ != .unavailable {
            Button("Relink\u{2026}") { relink() }.accessibilityIdentifier("file-menu-relink")
        }
        Divider()
        Button("Delete", role: .destructive) { Task { await model.delete(file) } }
            .accessibilityIdentifier("file-menu-delete")
    }

    private func relink() {
        guard let url = PageFilePanels.chooseOne(title: "Locate \u{201C}\(file.asset.originalName)\u{201D}") else { return }
        Task { await model.relink(file, url: url) }
    }

    private func exportCopy() {
        guard let url = PageFilePanels.chooseCopyDestination(title: "Save a copy of \u{201C}\(file.displayName)\u{201D}", name: file.asset.originalName) else { return }
        Task { await model.exportCopy(file, to: url.path) }
    }
}

private struct WhereChips: View {
    let file: ProjectFileDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: 4) {
            switch FilesModel.whereOf(file) {
            case .stored: PageChip(text: "Stored in project", tone: .green)
            case .linked: PageChip(text: "Linked (external)", tone: .blue)
            case .unavailable: PageChip(text: "Unavailable", tone: .red)
            }
            if file.asset.storageMode == "external", let path = file.asset.path {
                Text(((path as NSString).deletingLastPathComponent as NSString).lastPathComponent)
                    .font(.caption).foregroundStyle(theme["muted"]).lineLimit(1).help(path)
            }
        }
    }
}

private func whereWord(_ f: ProjectFileDto) -> String {
    switch FilesModel.whereOf(f) {
    case .stored: "stored in project"
    case .linked: "linked"
    case .unavailable: "unavailable"
    }
}

private struct FileRow: View {
    let model: FilesModel
    let file: ProjectFileDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let selected = model.selectedId == file.id
        let unavailable = FilesModel.whereOf(file) == .unavailable
        let kind = PageText.fileKind(mediaType: file.asset.mediaType, name: file.asset.originalName)
        return HStack(spacing: 8) {
            VStack(alignment: .leading, spacing: 1) {
                Text(file.displayName).fontWeight(.semibold).lineLimit(1)
                if case .all = model.folder, let fid = file.folderId {
                    Label(model.folderPath(fid), systemImage: "folder").font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(kind).frame(width: 90, alignment: .leading)
            WhereChips(file: file).frame(width: 190, alignment: .leading)
            Text(PageText.dayWord(ms: file.updatedAt)).frame(width: 90, alignment: .leading)
            HStack(spacing: 4) {
                if model.isUnavailableLink(file) {
                    Button { relink() } label: { Label("Relink", systemImage: "link") }
                        .controlSize(.small)
                        .accessibilityIdentifier("file-relink-\(file.id)")
                } else {
                    Button("Open") { Task { await model.open(file) } }
                        .controlSize(.small)
                        .disabled(unavailable)
                        .accessibilityIdentifier("file-open-\(file.id)")
                }
                Menu { FileMenu(model: model, file: file) } label: { Label("More", systemImage: "ellipsis") }
                    .menuStyle(.borderlessButton)
                    .menuIndicator(.hidden)
                    .frame(width: 28)
                    .accessibilityLabel("More actions for \(file.displayName)")
                    .accessibilityIdentifier("file-more-\(file.id)")
            }
            .frame(width: 130, alignment: .trailing)
        }
        .font(.callout)
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(selected ? theme["accent-soft"] : Color.clear)
        .contentShape(Rectangle())
        .onTapGesture(count: 2) { if !unavailable { Task { await model.open(file) } } }
        .onTapGesture { model.select(file.id) }
        .draggable(FileDragPayload.encode(file.id))
        .contextMenu { FileMenu(model: model, file: file) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(file.displayName), \(kind), \(whereWord(file))")
        .accessibilityAddTraits(selected ? .isSelected : [])
        .accessibilityAction(named: "Show details") { model.select(file.id) }
        .accessibilityIdentifier("file-row-\(file.id)")
    }

    private func relink() {
        guard let url = PageFilePanels.chooseOne(title: "Locate \u{201C}\(file.asset.originalName)\u{201D}") else { return }
        Task { await model.relink(file, url: url) }
    }
}

private struct FileCard: View {
    let model: FilesModel
    let file: ProjectFileDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let selected = model.selectedId == file.id
        let unavailable = FilesModel.whereOf(file) == .unavailable
        return HStack(spacing: 10) {
            Text(PageText.fileBadge(mediaType: file.asset.mediaType, name: file.asset.originalName))
                .font(.caption.weight(.bold))
                .frame(width: 38, height: 44)
                .background(theme["blue-soft"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                .foregroundStyle(theme["blue-t"])
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 3) {
                Text(file.displayName).fontWeight(.semibold).lineLimit(1)
                WhereChips(file: file)
            }
            Spacer(minLength: 0)
        }
        .padding(10)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).stroke(selected ? theme["blue"] : theme["line"], lineWidth: selected ? 2 : 1))
        .contentShape(Rectangle())
        .onTapGesture(count: 2) { if !unavailable { Task { await model.open(file) } } }
        .onTapGesture { model.select(file.id) }
        .draggable(FileDragPayload.encode(file.id))
        .contextMenu { FileMenu(model: model, file: file) }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(file.displayName)
        .accessibilityValue(whereWord(file))
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction(named: "Show details") { model.select(file.id) }
        .accessibilityIdentifier("file-card-\(file.id)")
    }
}

// MARK: Folders

private struct FolderPanel: View {
    let model: FilesModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            item(id: "all", label: "All files", count: model.count(in: .all), depth: 0, sel: .all, menu: nil, dropTarget: nil)
            item(id: "top", label: "Top level", count: model.count(in: .top), depth: 0, sel: .top, menu: nil, dropTarget: .some(nil))
            ForEach(model.folderTree, id: \.folder.id) { entry in
                item(id: entry.folder.id, label: entry.folder.name, count: model.count(in: .folder(entry.folder.id)), depth: entry.depth + 1,
                     sel: .folder(entry.folder.id), menu: entry.folder, dropTarget: .some(entry.folder.id))
            }
            if case let .folder(id) = model.folder, let f = model.folders.first(where: { $0.id == id }) {
                HStack(spacing: 4) {
                    Button { model.dialog = .renameFolder(f) } label: { Label("Rename", systemImage: "pencil") }
                        .accessibilityIdentifier("folder-rename")
                    Button { model.dialog = .deleteFolder(f) } label: { Label("Delete", systemImage: "trash") }
                        .accessibilityIdentifier("folder-delete")
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
                .padding(.top, 8).padding(.horizontal, 4)
            }
        }
        .padding(6)
        .frame(width: 210, alignment: .topLeading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).stroke(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Folders")
        .accessibilityIdentifier("files-folders")
    }

    /// `dropTarget`: nil means the row takes no drops (All files); `.some(nil)` is the top level.
    private func item(id: String, label: String, count: Int, depth: Int, sel: FolderSel, menu: FolderDto?, dropTarget: String??) -> some View {
        let active = model.folder == sel
        let row = Button { model.folder = sel } label: {
            HStack(spacing: 6) {
                Image(systemName: "folder").accessibilityHidden(true)
                Text(label).lineLimit(1).fontWeight(active ? .bold : .medium)
                Spacer(minLength: 4)
                Text("\(count)").font(.caption).foregroundStyle(active ? theme["ink"] : theme["muted"])
            }
            .font(.callout)
            .padding(.vertical, 5).padding(.leading, CGFloat(8 + depth * 14)).padding(.trailing, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(active ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 7, style: .continuous))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(label), \(count) \(count == 1 ? "file" : "files")")
        .accessibilityAddTraits(active ? .isSelected : [])
        .accessibilityIdentifier("folder-\(id)")
        return Group {
            if let dropTarget {
                row.dropDestination(for: String.self) { texts, _ in
                    Task { _ = await model.drop(payloads: texts, onto: dropTarget) }
                    return texts.contains { FileDragPayload.decode($0) != nil }
                }
            } else {
                row
            }
        }
        .contextMenu {
            if let menu {
                Button("Rename Folder") { model.dialog = .renameFolder(menu) }.accessibilityIdentifier("folder-menu-rename")
                Divider()
                Button("Delete Folder\u{2026}", role: .destructive) { model.dialog = .deleteFolder(menu) }.accessibilityIdentifier("folder-menu-delete")
            }
        }
    }
}

private struct FolderNameSheet: View {
    let title: String
    let subtitle: String?
    let initial: String
    let confirm: String
    let close: () -> Void
    let save: (String) async -> Bool
    @State private var name: String
    @State private var busy = false
    @FocusState private var focused: Bool
    @Environment(\.ofTheme) private var theme

    init(title: String, subtitle: String?, initial: String, confirm: String, close: @escaping () -> Void, save: @escaping (String) async -> Bool) {
        self.title = title
        self.subtitle = subtitle
        self.initial = initial
        self.confirm = confirm
        self.close = close
        self.save = save
        _name = State(initialValue: initial)
    }

    private var invalid: Bool { name.trimmingCharacters(in: .whitespaces).isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title).font(.title3.weight(.semibold))
            if let subtitle { Text(subtitle).foregroundStyle(theme["muted"]) }
            VStack(alignment: .leading, spacing: 4) {
                Text("Folder name").font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
                TextField("e.g. Contracts", text: Binding(get: { name }, set: { name = String($0.prefix(120)) }))
                    .textFieldStyle(.roundedBorder)
                    .focused($focused)
                    .onSubmit { submit() }
                    .accessibilityLabel("Folder name")
                    .accessibilityIdentifier("folder-name")
            }
            HStack {
                Spacer()
                Button("Cancel", action: close)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("folder-cancel")
                Button(confirm) { submit() }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(invalid || busy)
                    .accessibilityIdentifier("folder-confirm")
            }
        }
        .padding(20)
        .frame(width: 380)
        .onAppear { focused = true }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("folder-sheet")
    }

    private func submit() {
        guard !invalid, !busy else { return }
        busy = true
        Task {
            if await save(name) { close() } else { busy = false }
        }
    }
}

private struct DeleteFolderSheet: View {
    let folder: FolderDto
    let close: () -> Void
    let confirm: () async -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Delete folder \u{201C}\(folder.name)\u{201D}?").font(.title3.weight(.semibold))
            Text("Files and folders inside it move to the top level. No files are deleted. The folder goes to Recently Deleted.")
                .foregroundStyle(theme["ink2"])
            HStack {
                Spacer()
                Button("Cancel", action: close)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("folder-delete-cancel")
                Button("Delete Folder", role: .destructive) {
                    close()
                    Task { await confirm() }
                }
                .buttonStyle(.borderedProminent)
                .tint(theme["red"])
                .accessibilityIdentifier("folder-delete-confirm")
            }
        }
        .padding(20)
        .frame(width: 400)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("folder-delete-sheet")
    }
}

// MARK: Details drawer

struct FileDetailsView: View {
    let files: FilesModel
    @State private var details: FileDetailsModel
    @FocusState private var focus: Focus?
    @Environment(\.ofTheme) private var theme

    private enum Focus { case name, notes }

    init(files: FilesModel, id: String) {
        self.files = files
        _details = State(initialValue: FileDetailsModel(files: files, id: id))
    }

    var body: some View {
        VStack(spacing: 0) {
            if let f = details.file {
                header(f)
                Divider().overlay(theme["line"])
                ScrollView {
                    VStack(alignment: .leading, spacing: 12) {
                        if FilesModel.whereOf(f) == .unavailable { unavailableBanner(f) }
                        nameField(f)
                        folderField(f)
                        whereSection(f)
                        notesField
                        Text("Original file name: \(f.asset.originalName)\nAdded \(PageText.dayAndTime(ms: f.createdAt)) \u{00B7} Changed \(PageText.dayAndTime(ms: f.updatedAt))")
                            .font(.caption).foregroundStyle(theme["muted"]).textSelection(.enabled)
                            .accessibilityIdentifier("file-dates")
                        actions(f)
                    }
                    .padding(16)
                }
                Divider().overlay(theme["line"])
                footer(f)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(theme["panel"])
        .onAppear { details.sync() }
        .onChange(of: details.file) { _, _ in details.sync() }
        .onChange(of: files.renameSignal) { _, _ in focus = .name }
        .onChange(of: focus) { old, new in
            details.nameFocused = new == .name
            details.notesFocused = new == .notes
            if old == .name, new != .name { Task { await details.commitName() } }
            if old == .notes, new != .notes { Task { await details.flushNotes() } }
        }
        .onDisappear { Task { await details.commitName(); await details.flushNotes() } }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("File details")
        .accessibilityIdentifier("file-drawer")
    }

    private func header(_ f: ProjectFileDto) -> some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                Text("File").font(.caption).foregroundStyle(theme["muted"])
                Text(f.displayName).font(.headline).lineLimit(2)
                    .accessibilityIdentifier("file-heading")
            }
            Spacer()
            Button { Task { await details.close() } } label: { Label("Close", systemImage: "xmark") }
                .buttonStyle(.borderless)
                .help("Close")
                .keyboardShortcut(.cancelAction)
                .accessibilityLabel("Close")
                .accessibilityIdentifier("file-close")
        }
        .padding(14)
    }

    private func unavailableBanner(_ f: ProjectFileDto) -> some View {
        Text(f.asset.storageMode == "external"
            ? "This linked file can't be found. It may be on a drive that isn't connected, or it was moved. Relink it. The reference is never deleted."
            : "This file is missing from the project folder. Restore the project folder from a backup, or delete this entry.")
            .font(.callout)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["yellow-soft"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .foregroundStyle(theme["yellow-t"])
            .accessibilityIdentifier("file-unavailable-banner")
    }

    private func nameField(_ f: ProjectFileDto) -> some View {
        labelled("Name", hint: "The name shown in OpenFrame. The file itself is not renamed.") {
            TextField("Name", text: Binding(get: { details.name }, set: { details.name = String($0.prefix(255)) }))
                .textFieldStyle(.roundedBorder)
                .focused($focus, equals: .name)
                .onSubmit { Task { await details.commitName() } }
                .onExitCommand { details.cancelName() }
                .accessibilityLabel("Name")
                .accessibilityIdentifier("file-name")
        }
    }

    private func folderField(_ f: ProjectFileDto) -> some View {
        labelled("Folder", hint: nil) {
            Picker("Folder", selection: Binding(get: { f.folderId ?? "" }, set: { v in Task { await files.move(f, to: v.isEmpty ? nil : v) } })) {
                Text("Top level").tag("")
                ForEach(files.folderTree.sorted { files.folderPath($0.folder.id) < files.folderPath($1.folder.id) }, id: \.folder.id) { entry in
                    Text(files.folderPath(entry.folder.id)).tag(entry.folder.id)
                }
            }
            .labelsHidden()
            .accessibilityLabel("Folder")
            .accessibilityIdentifier("file-folder")
        }
    }

    private func whereSection(_ f: ProjectFileDto) -> some View {
        labelled("Where", hint: f.asset.storageMode == "external"
            ? "Linked files stay where they are. Moving or renaming the original breaks the link until you relink it."
            : "A copy is stored inside the project folder and travels with the project.") {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    switch FilesModel.whereOf(f) {
                    case .stored: PageChip(text: "Stored in project", tone: .green)
                    case .linked: PageChip(text: "Linked (external)", tone: .blue)
                    case .unavailable: PageChip(text: "Unavailable", tone: .red)
                    }
                    Text(PageText.fileKind(mediaType: f.asset.mediaType, name: f.asset.originalName)
                        + (f.asset.byteSize.map { " \u{00B7} \(PageText.fileSize($0))" } ?? ""))
                        .font(.callout).foregroundStyle(theme["muted"])
                }
                if f.asset.storageMode == "external", let path = f.asset.path {
                    Text(path).font(.caption).foregroundStyle(theme["muted"]).textSelection(.enabled)
                        .accessibilityIdentifier("file-path")
                }
            }
        }
        .accessibilityIdentifier("file-where")
    }

    private var notesField: some View {
        labelled("Notes", hint: nil) {
            TextEditor(text: Binding(get: { details.notes }, set: { details.editNotes(String($0.prefix(20000))) }))
                .frame(minHeight: 100)
                .scrollContentBackground(.hidden)
                .padding(6)
                .background(theme["bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(theme["line"]))
                .focused($focus, equals: .notes)
                .overlay(alignment: .topLeading) {
                    if details.notes.isEmpty {
                        Text("What is this file, and why does it matter?").foregroundStyle(theme["muted"])
                            .padding(.horizontal, 11).padding(.vertical, 14).allowsHitTesting(false)
                    }
                }
                .accessibilityLabel("Notes")
                .accessibilityIdentifier("file-notes")
        }
    }

    private func actions(_ f: ProjectFileDto) -> some View {
        let unavailable = FilesModel.whereOf(f) == .unavailable
        return HStack {
            Button { Task { await files.reveal(f) } } label: { Label("Reveal in Finder", systemImage: "folder") }
                .disabled(unavailable)
                .accessibilityIdentifier("file-reveal")
            Button {
                if let url = PageFilePanels.chooseCopyDestination(title: "Save a copy of \u{201C}\(f.displayName)\u{201D}", name: f.asset.originalName) {
                    Task { await files.exportCopy(f, to: url.path) }
                }
            } label: { Label("Export Copy\u{2026}", systemImage: "square.and.arrow.down") }
                .disabled(unavailable)
                .accessibilityIdentifier("file-export")
            if f.asset.storageMode == "external" && !unavailable {
                Button { relink(f) } label: { Label("Relink\u{2026}", systemImage: "link") }
                    .accessibilityIdentifier("file-relink")
            }
        }
        .controlSize(.small)
    }

    private func footer(_ f: ProjectFileDto) -> some View {
        HStack {
            Button(role: .destructive) { Task { await files.delete(f) } } label: { Label("Delete", systemImage: "trash") }
                .accessibilityIdentifier("file-delete")
            Spacer()
            if files.isUnavailableLink(f) {
                Button { relink(f) } label: { Label("Relink\u{2026}", systemImage: "link") }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("file-relink-primary")
            } else {
                Button { Task { await files.open(f) } } label: { Label("Open", systemImage: "arrow.up.forward.square") }
                    .buttonStyle(.borderedProminent)
                    .disabled(FilesModel.whereOf(f) == .unavailable)
                    .accessibilityIdentifier("file-open")
            }
        }
        .padding(12)
    }

    private func relink(_ f: ProjectFileDto) {
        guard let url = PageFilePanels.chooseOne(title: "Locate \u{201C}\(f.asset.originalName)\u{201D}") else { return }
        Task { await files.relink(f, url: url) }
    }

    private func labelled<V: View>(_ text: String, hint: String?, @ViewBuilder _ content: () -> V) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(text).font(.caption.weight(.semibold)).foregroundStyle(theme["ink2"])
            content()
            if let hint { Text(hint).font(.caption).foregroundStyle(theme["muted"]) }
        }
    }
}

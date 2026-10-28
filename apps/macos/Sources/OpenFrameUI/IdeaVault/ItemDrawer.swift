// The open item: the note editor for text, and the detail view for pictures, links, files and
// voice notes. It sits in the window's inspector column. Text saves automatically; nothing forces
// a title before closing.

import AppKit
import AVKit
import OpenFrameCore
import SwiftUI
import UniformTypeIdentifiers

struct VaultDrawer: View {
    let model: IdeaVaultModel
    let id: String
    @State private var editor: ItemEditorModel?

    var body: some View {
        Group {
            if let editor {
                VaultDrawerContent(model: model, editor: editor)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task(id: id) {
            if let old = editor, old.id != id { await old.flush() }
            let next = ItemEditorModel(vault: model, id: id)
            editor = next
            await next.refresh()
        }
    }
}

private struct VaultDrawerContent: View {
    let model: IdeaVaultModel
    @Bindable var editor: ItemEditorModel
    @Environment(\.ofTheme) private var theme
    @FocusState private var focus: Field?
    @State private var tagText = ""
    @State private var relinking = false

    enum Field: Hashable { case title, body }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider().overlay(theme["line"])
            if editor.missing {
                missing
            } else if let item = editor.item {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        actionRow(item)
                        if !VaultText.isTextType(item.itemType), item.itemType != .url {
                            VaultPreview(model: model, item: item)
                            storageMarker(item)
                        }
                        fields(item)
                        statusLine(item)
                        researchFlag(item)
                        collectionsRow(item)
                        tagsRow(item)
                        folderRow(item)
                        if item.sourceGlobalItemId != nil {
                            Text("Copied from your Global Idea Vault. Editing this copy never changes the original.")
                                .font(.caption).foregroundStyle(theme["muted"])
                        }
                    }
                    .padding(16)
                }
                Divider().overlay(theme["line"])
                footer(item)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(theme["panel"])
        .onAppear {
            if model.focusTitle { focus = .title }
        }
        .onChange(of: editor.item?.id) { _, _ in
            if model.focusTitle { focus = .title } else if let i = editor.item, VaultText.isTextType(i.itemType), (i.body ?? "").isEmpty { focus = .body }
        }
        .task(id: model.app.changes.key(VaultTables.all, store: model.storeKind)) { await editor.refresh() }
        .fileImporter(isPresented: $relinking, allowedContentTypes: [.item]) { result in
            if case let .success(url) = result, let item = editor.item {
                Task { await model.relink(item, path: url.path) }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Item details")
        .accessibilityIdentifier("vault-drawer")
    }

    // MARK: Parts

    private var header: some View {
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                if let item = editor.item {
                    Text(VaultText.isExternal(item) ? "\(VaultText.typeLabel(item.itemType)) · external file" : VaultText.typeLabel(item.itemType))
                        .font(.caption).foregroundStyle(theme["muted"])
                }
                Text(editor.heading).font(.headline).lineLimit(2)
            }
            Spacer(minLength: 0)
            Button {
                Task {
                    await editor.flush()
                    model.closeDrawer()
                }
            } label: {
                Label("Close", systemImage: "xmark")
            }
            .buttonStyle(.borderless)
            .keyboardShortcut(.cancelAction)
            .help("Close")
            .accessibilityLabel("Close")
            .accessibilityIdentifier("drawer-close")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    private var missing: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Item not found").font(.headline)
            Text("This item is no longer in the vault. It may have been deleted. Look in Recently Deleted.")
                .foregroundStyle(theme["ink2"]).fixedSize(horizontal: false, vertical: true)
            Button("Close") { model.closeDrawer() }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func actionRow(_ item: VaultItemDto) -> some View {
        HStack(spacing: 6) {
            Button {
                Task { await model.setPinned([item], !item.pinned) }
            } label: { Label(item.pinned ? "Unpin" : "Pin", systemImage: item.pinned ? "pin.slash" : "pin") }
            if !VaultText.isTextType(item.itemType) {
                Button("Rename") { focus = .title }
            }
            Button("Move") { Task { await editor.flush(); model.dialog = .move([item]) } }
            if let label = model.copyLabel {
                Button(label) { Task { await editor.flush(); model.dialog = .copy([item]) } }
            }
        }
        .controlSize(.small)
    }

    @ViewBuilder private func fields(_ item: VaultItemDto) -> some View {
        let text = VaultText.isTextType(item.itemType)
        labeled(text ? "Title (optional)" : item.itemType == .voice ? "Name (optional)" : "Title") {
            TextField(text ? (item.itemType == .quote ? "Untitled quote" : "Untitled note") : "Leave blank to keep it untitled",
                      text: binding(.title))
                .textFieldStyle(.roundedBorder)
                .focused($focus, equals: .title)
                .onSubmit { Task { await editor.flush() } }
                .accessibilityLabel("Title")
                .accessibilityIdentifier("drawer-title")
        }
        if item.itemType == .url {
            labeled("Link") {
                HStack {
                    TextField("https://…", text: binding(.url))
                        .textFieldStyle(.roundedBorder)
                        .onSubmit { Task { await editor.flush() } }
                        .accessibilityLabel("Link")
                    Button { if let u = item.url { Task { await model.openLink(u) } } } label: { Label("Open", systemImage: "arrow.up.right.square") }
                        .disabled(item.url?.isEmpty ?? true)
                        .help("Open the link in your browser")
                }
                if let e = editor.urlError { SheetError(message: e) }
            }
        }
        if text || item.itemType == .url {
            labeled(item.itemType == .quote ? "Quote" : item.itemType == .url ? "Your note" : "Note") {
                bodyEditor(placeholder: item.itemType == .quote ? "The words, exactly as you heard or read them"
                    : item.itemType == .url ? "Why this link matters (optional)" : "Write anything…",
                    height: item.itemType == .url ? 90 : 240)
            }
        }
        if item.itemType == .quote {
            labeled("Source (optional)") {
                TextField("Who said it, where you found it", text: binding(.sourceText)).textFieldStyle(.roundedBorder)
                    .onSubmit { Task { await editor.flush() } }
            }
        }
        if !text, item.itemType != .url {
            labeled("Caption") {
                TextField("Optional", text: binding(.caption), axis: .vertical).lineLimit(2...5).textFieldStyle(.roundedBorder)
            }
            labeled("Source link (optional)") {
                TextField("https://…", text: binding(.url)).textFieldStyle(.roundedBorder)
                    .onSubmit { Task { await editor.flush() } }
                if let e = editor.urlError { SheetError(message: e) }
            }
        }
    }

    private func bodyEditor(placeholder: String, height: CGFloat) -> some View {
        ZStack(alignment: .topLeading) {
            TextEditor(text: binding(.body))
                .font(.body)
                .scrollContentBackground(.hidden)
                .padding(6)
                .frame(minHeight: height)
                .background(theme["bg"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line2"]))
                .focused($focus, equals: .body)
                .accessibilityLabel("Note text")
                .accessibilityIdentifier("drawer-body")
            if editor.draft.body.isEmpty {
                Text(placeholder).foregroundStyle(theme["placeholder"]).padding(.horizontal, 11).padding(.vertical, 14)
                    .allowsHitTesting(false).accessibilityHidden(true)
            }
        }
    }

    private func statusLine(_ item: VaultItemDto) -> some View {
        HStack {
            Group {
                if editor.status == .failed {
                    HStack(spacing: 6) {
                        Text(editor.statusText).foregroundStyle(theme["red-t"])
                        Button("Retry") { Task { await editor.flush() } }.controlSize(.small)
                    }
                } else {
                    Text(editor.statusText).foregroundStyle(theme["muted"])
                }
            }
            .accessibilityIdentifier("drawer-save-status")
            Spacer(minLength: 8)
            Text("Added \(VaultText.relativeDay(ms: item.createdAt)) · Modified \(VaultText.relativeDay(ms: item.updatedAt))")
                .foregroundStyle(theme["muted"])
        }
        .font(.caption)
    }

    private func researchFlag(_ item: VaultItemDto) -> some View {
        labeled("Research flag") {
            Picker("Research flag", selection: Binding(
                get: { item.researchFlag?.rawValue ?? "" },
                set: { raw in
                    let flag = VaultResearchFlag(rawValue: raw)
                    if flag != item.researchFlag { Task { await model.setResearchFlag([item], flag) } }
                })) {
                Text("No flag").tag("")
                ForEach(VaultText.researchFlags, id: \.value) { Text($0.label).tag($0.value.rawValue) }
            }
            .labelsHidden()
            .accessibilityLabel("Research flag")
            .accessibilityIdentifier("drawer-research-flag")
            Text(VaultHints.researchFlag)
                .font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
        }
    }

    private func collectionsRow(_ item: VaultItemDto) -> some View {
        let mine = model.collections.filter { item.collectionIds.contains($0.id) }
        return labeled("Collections") {
            VaultChipFlow(spacing: 6) {
                ForEach(mine, id: \.id) { c in
                    chip(c.name, removeLabel: "Remove from \(c.name)") { Task { await model.removeFromCollection(item, collectionId: c.id) } }
                }
                if mine.isEmpty { Text("Not in a collection").font(.caption).foregroundStyle(theme["muted"]) }
                Button("Add to Collection…") { Task { await editor.flush(); model.dialog = .collection([item]) } }
                    .buttonStyle(.link).font(.caption)
            }
        }
    }

    private func tagsRow(_ item: VaultItemDto) -> some View {
        labeled("Tags") {
            VaultChipFlow(spacing: 6) {
                ForEach(item.tags, id: \.self) { t in
                    chip(t, removeLabel: "Remove tag \(t)") { Task { await model.removeTag(item, tag: t) } }
                }
            }
            TextField("Add tag…", text: $tagText)
                .textFieldStyle(.roundedBorder)
                .onSubmit { addTags(item) }
                .accessibilityLabel("Add tag")
                .accessibilityIdentifier("drawer-add-tag")
        }
    }

    private func addTags(_ item: VaultItemDto) {
        let tags = VaultText.parseTags(tagText)
        guard !tags.isEmpty else { return }
        Task {
            do {
                try await model.addTags([item], tags, announce: false)
                tagText = ""
            } catch { model.app.show("Could not add the tag", error) }
        }
    }

    private func folderRow(_ item: VaultItemDto) -> some View {
        labeled("Folder") {
            HStack {
                Text(model.folders.first { $0.id == item.folderId }?.name ?? "No folder")
                Button("Move…") { Task { await editor.flush(); model.dialog = .move([item]) } }.buttonStyle(.link)
            }
        }
    }

    private func footer(_ item: VaultItemDto) -> some View {
        HStack {
            Button("Delete", role: .destructive) { Task { await editor.flush(); await model.delete([item]) } }
                .accessibilityIdentifier("drawer-delete")
            Spacer()
            if model.hasProject {
                Button("Send to Story") { Task { await editor.flush(); model.dialog = .send(item) } }
                    .accessibilityIdentifier("drawer-send-to-story")
            }
        }
        .controlSize(.regular)
        .padding(12)
    }

    // MARK: Storage

    @ViewBuilder private func storageMarker(_ item: VaultItemDto) -> some View {
        if let asset = item.asset {
            if VaultText.isUnavailable(item) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("File unavailable.").fontWeight(.semibold)
                    Text("The original file could not be found" + (asset.path.map { " at \($0)." } ?? "."))
                        .textSelection(.enabled)
                    Text("OpenFrame keeps this item and its notes. Nothing has been deleted.")
                    if VaultText.isExternal(item) {
                        HStack {
                            Button("Relink…") { relinking = true }
                            Button("Open Containing Folder") { Task { await model.openExternally(item, reveal: true) } }
                        }
                        Text("This item links to a file outside the project. Files stored inside the project show a different marker.")
                            .font(.caption).foregroundStyle(theme["muted"])
                    }
                }
                .font(.callout)
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["red-soft"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .foregroundStyle(theme["red-t"])
                .accessibilityIdentifier("file-unavailable")
            } else if VaultText.isExternal(item) {
                marker("Linked file", "Stays in its original location on this Mac.", tone: "blue")
            } else {
                marker("Stored in the vault", model.isGlobal ? "Kept in your Global Idea Vault folder." : "Kept inside this project.", tone: "green")
            }
        }
    }

    private func marker(_ title: String, _ detail: String, tone: String) -> some View {
        HStack(spacing: 8) {
            Text(title).font(.caption.weight(.medium)).padding(.horizontal, 7).padding(.vertical, 2)
                .background(theme["\(tone)-soft"], in: Capsule()).foregroundStyle(theme["\(tone)-t"])
            Text(detail).font(.caption).foregroundStyle(theme["muted"])
        }
    }

    // MARK: Helpers

    private func binding(_ field: VaultField) -> Binding<String> {
        Binding(get: { editor.draft[field] }, set: { editor.edit(field, to: $0) })
    }

    private func labeled<C: View>(_ title: String, @ViewBuilder content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(title).font(.caption.weight(.medium)).foregroundStyle(theme["ink2"])
            content()
        }
    }

    private func chip(_ text: String, removeLabel: String, remove: @escaping () -> Void) -> some View {
        HStack(spacing: 3) {
            Text(text).font(.caption)
            Button(action: remove) { Label("Remove", systemImage: "xmark").font(.caption2) }
                .buttonStyle(.plain)
                .accessibilityLabel(removeLabel)
        }
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .background(theme["accent-soft"], in: Capsule())
        .foregroundStyle(theme["accent-ink"])
    }
}

/// Wraps its children onto new lines, for chips.
struct VaultChipFlow: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        arrange(width: proposal.width ?? 320, subviews: subviews).size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = arrange(width: bounds.width, subviews: subviews)
        for (i, origin) in result.origins.enumerated() {
            subviews[i].place(at: CGPoint(x: bounds.minX + origin.x, y: bounds.minY + origin.y), proposal: .unspecified)
        }
    }

    private func arrange(width: CGFloat, subviews: Subviews) -> (size: CGSize, origins: [CGPoint]) {
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0, maxX: CGFloat = 0
        var origins: [CGPoint] = []
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > 0, x + size.width > width {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            origins.append(CGPoint(x: x, y: y))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
            maxX = max(maxX, x - spacing)
        }
        return (CGSize(width: maxX, height: y + rowHeight), origins)
    }
}

// MARK: Preview

/// The picture, player or file card at the top of a non-text item.
struct VaultPreview: View {
    let model: IdeaVaultModel
    let item: VaultItemDto
    @State private var player = AudioPlayerModel()
    @State private var videoPlayer: AVPlayer?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if let asset = item.asset {
            VStack(alignment: .leading, spacing: 8) {
                media(asset)
                HStack {
                    Text(facts(asset)).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1).truncationMode(.middle)
                    Spacer(minLength: 6)
                    if !VaultText.isUnavailable(item) {
                        Button { Task { await model.openExternally(item) } } label: { Label("Open", systemImage: "arrow.up.right.square") }
                            .controlSize(.small)
                            .help("Open with the app that handles this file")
                            .accessibilityIdentifier("drawer-open-externally")
                        Button { Task { await model.openExternally(item, reveal: true) } } label: { Label("Show in Finder", systemImage: "folder") }
                            .controlSize(.small)
                    }
                }
            }
            .task(id: asset.id) { await load(asset) }
            .onDisappear {
                player.stop()
                videoPlayer?.pause()
            }
        }
    }

    @ViewBuilder private func media(_ asset: AssetInfo) -> some View {
        if VaultText.isUnavailable(item) {
            fileCard(asset, note: "This file is not available right now.")
        } else if VaultText.isVisualType(item.itemType) {
            VaultThumbnailView(item: item, store: model.scope, height: 260, maxPixel: 1400, fill: false)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .accessibilityHidden(false)
                .accessibilityLabel(item.displayName)
        } else if VaultText.isAudioType(item.itemType) {
            audioPlayer
        } else if item.itemType == .video, let videoPlayer {
            VideoPlayer(player: videoPlayer)
                .frame(height: 220)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .accessibilityLabel("Play \(item.displayName)")
        } else {
            fileCard(asset, note: item.itemType == .pdf ? "PDFs open in your PDF app." : "Preview isn't available for this file type, but it is stored safely.")
        }
    }

    private var audioPlayer: some View {
        HStack(spacing: 10) {
            Button { player.toggle() } label: {
                Label(player.isPlaying ? "Pause" : "Play", systemImage: player.isPlaying ? "pause.fill" : "play.fill")
            }
            .buttonStyle(.ofProminent)
            .disabled(!player.loaded || player.failed)
            .accessibilityLabel(player.isPlaying ? "Pause \(item.displayName)" : "Play \(item.displayName)")
            .accessibilityIdentifier("audio-play")
            if player.failed {
                Text("This recording can't be played here, but it is stored safely.").font(.callout).foregroundStyle(theme["muted"])
            } else {
                Slider(value: Binding(get: { player.position }, set: { player.seek(to: $0) }), in: 0...max(player.duration, 0.01))
                    .disabled(player.duration <= 0)
                    .accessibilityLabel("Position")
                Text("\(VaultText.duration(ms: player.position * 1000) ?? "0:00") / \(VaultText.duration(ms: player.duration * 1000) ?? "0:00")")
                    .font(.caption.monospacedDigit()).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("audio-time")
            }
        }
        .padding(10)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        // A container keeps the play button's and the time's own identifiers; without it SwiftUI copies
        // "audio-player" over every child.
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("audio-player")
    }

    private func fileCard(_ asset: AssetInfo, note: String) -> some View {
        HStack(spacing: 10) {
            ZStack {
                RoundedRectangle(cornerRadius: 8, style: .continuous).fill(theme["accent-soft"])
                Text(item.itemType == .pdf ? "PDF" : VaultText.extLabel(asset.originalName)).font(.caption.weight(.bold)).foregroundStyle(theme["accent-ink"])
            }
            .frame(width: 44, height: 44)
            VStack(alignment: .leading, spacing: 2) {
                Text(asset.originalName).font(.headline).lineLimit(1)
                Text(note).font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer(minLength: 0)
        }
        .padding(10)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
    }

    private func facts(_ a: AssetInfo) -> String {
        [a.originalName, VaultText.bytes(a.byteSize), (a.width != nil && a.height != nil) ? "\(Int(a.width!))×\(Int(a.height!))" : nil,
         VaultText.duration(ms: a.durationMs)].compactMap { $0 }.joined(separator: " · ")
    }

    private func load(_ asset: AssetInfo) async {
        guard asset.available, let core = model.app.core else { return }
        guard VaultText.isAudioType(item.itemType) || item.itemType == .video else { return }
        let url = try? await core.assetFile(asset.id, global: model.scope == .global)
        if item.itemType == .video, let url { videoPlayer = AVPlayer(url: url) }
        if VaultText.isAudioType(item.itemType) {
            if let url { player.load(url) } else { player.load(URL(fileURLWithPath: "/nonexistent")) }
        }
    }
}

// The left panel of the Vault: smart views, collections, folders and tags with their counts.
// Items can be dragged onto a folder or a collection to file them there.

import OpenFrameCore
import SwiftUI

struct VaultSidebar: View {
    @Bindable var model: IdeaVaultModel
    @Environment(\.ofTheme) private var theme

    private var selection: Binding<VaultView?> {
        Binding(get: { model.listView }, set: { if let v = $0 { model.show(v) } })
    }

    var body: some View {
        List(selection: selection) {
            Section {
                row("All items", "square.grid.2x2", .all, count: model.overview?.total)
                row("Pinned", "pin", .pinned, count: model.overview?.pinned)
                row("Recently added", "clock", .recentlyAdded)
                row("Recently modified", "pencil", .recentlyModified)
            }
            Section("Collections") {
                ForEach(model.collections, id: \.id) { c in
                    row(c.name, "square.stack.3d.up", .collection(collectionId: c.id), count: c.itemCount)
                        .contextMenu {
                            Button("Rename…") { model.dialog = .renameCollection(c) }
                            Divider()
                            Button("Delete Collection", role: .destructive) { Task { await model.deleteCollection(c) } }
                        }
                        .dropDestination(for: String.self) { texts, _ in dropOnCollection(texts, c) }
                }
                Button { model.dialog = .newCollection } label: { Label("New Collection", systemImage: "plus") }
                    .buttonStyle(.plain)
                    .foregroundStyle(theme["accent"])
                    .accessibilityIdentifier("vault-new-collection")
            }
            Section("Folders") {
                ForEach(VaultFolders.tree(model.folders), id: \.folder.id) { entry in
                    row(entry.folder.name, "folder", .folder(folderId: entry.folder.id), count: entry.folder.itemCount)
                        .padding(.leading, CGFloat(entry.depth) * 12)
                        .contextMenu { folderMenu(entry.folder) }
                        .dropDestination(for: String.self) { texts, _ in dropOnFolder(texts, entry.folder) }
                }
                Button { model.dialog = .newFolder(parentId: currentFolder) } label: { Label("New Folder", systemImage: "plus") }
                    .buttonStyle(.plain)
                    .foregroundStyle(theme["accent"])
                    .accessibilityIdentifier("vault-new-folder")
            }
            if let tags = model.overview?.tags, !tags.isEmpty {
                Section("Tags") {
                    ForEach(tags, id: \.tag) { t in
                        row(t.tag, "number", .tag(tag: t.tag), count: t.count)
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .scrollContentBackground(.hidden)
        .background(theme["panel"])
        .accessibilityLabel("Idea Vault sections")
        .accessibilityIdentifier("vault-sidebar")
    }

    private var currentFolder: String? {
        if model.mode == .folder { return model.folderCursor }
        if case let .folder(id) = model.view { return id }
        return nil
    }

    private func row(_ title: String, _ symbol: String, _ view: VaultView, count: Double? = nil) -> some View {
        HStack {
            Label(title, systemImage: symbol).lineLimit(1)
            Spacer(minLength: 4)
            if let count {
                Text("\(Int(count))").font(.caption).monospacedDigit().foregroundStyle(theme["muted"])
                    .accessibilityLabel("\(Int(count)) items")
            }
        }
        .tag(view as VaultView?)
        .accessibilityIdentifier("vault-nav-\(title)")
    }

    @ViewBuilder private func folderMenu(_ f: VaultFolderDto) -> some View {
        Button("Open") { model.openFolder(f.id) }
        Button("Rename…") { model.dialog = .renameFolder(f) }
        Button("New Folder Inside…") { model.dialog = .newFolder(parentId: f.id) }
        Divider()
        Button("Delete Folder", role: .destructive) { Task { await model.deleteFolder(f) } }
    }

    private func vaultItems(_ ids: [String]) -> [VaultItemDto] {
        model.items.filter { ids.contains($0.id) }
    }

    private func dropOnFolder(_ texts: [String], _ folder: VaultFolderDto) -> Bool {
        guard let payload = texts.lazy.compactMap(VaultDragPayload.decode).first, payload.store == model.scope else { return false }
        let xs = vaultItems(payload.ids)
        guard !xs.isEmpty else { return false }
        Task {
            do { try await model.move(xs, to: folder.id) } catch { model.app.show("Could not move the items", error) }
        }
        return true
    }

    private func dropOnCollection(_ texts: [String], _ collection: VaultCollectionDto) -> Bool {
        guard let payload = texts.lazy.compactMap(VaultDragPayload.decode).first, payload.store == model.scope else { return false }
        let xs = vaultItems(payload.ids)
        guard !xs.isEmpty else { return false }
        Task {
            do { try await model.addToCollection(xs, collectionId: collection.id) } catch { model.app.show("Could not add the items", error) }
        }
        return true
    }
}

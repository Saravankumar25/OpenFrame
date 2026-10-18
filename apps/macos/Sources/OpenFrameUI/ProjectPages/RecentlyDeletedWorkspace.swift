// Recently Deleted (Windows: workspaces/trash). Deleted items stay here for 30 days, then the
// core removes them for good. Each row says how many days are left. Restore returns an item to
// its place; when that place no longer exists it goes to Unassigned or the top level and the
// person is told. Delete Permanently always asks a second, deliberate question.
//
// The Global Idea Vault has its own list, shown with the switch at the top. The Story and Idea
// Vault items of the project are in the project list, as on Windows.

import OpenFrameCore
import SwiftUI

@MainActor
@Observable
final class RecentlyDeletedModel {
    let app: AppModel
    var scope: StoreSel = .project
    var text = ""
    private(set) var items: [DeletedItemView] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    /// The id of the row being restored or deleted, so its buttons are off meanwhile.
    private(set) var busyId: String?
    var purging: DeletedItemView?
    @ObservationIgnored private var generation = 0

    init(app: AppModel) { self.app = app }

    var storeKind: StoreKind { scope == .global ? .global : .project }

    var shown: [DeletedItemView] {
        let q = text.trimmingCharacters(in: .whitespaces).lowercased()
        guard !q.isEmpty else { return items }
        return items.filter { [$0.title ?? "", $0.typeLabel, $0.wasIn].contains { $0.lowercased().contains(q) } }
    }

    static let subtitle = "Deleted items stay here for 30 days, then they are removed for good. Restore them before then, or delete them sooner."

    func choose(_ s: StoreSel) {
        guard s != scope else { return }
        scope = s
        items = []
        loaded = false
        purging = nil
    }

    func reload() async {
        guard let core = app.core else { return }
        generation += 1
        let mine = generation
        do {
            let rows = try await core.projectDeletedItems(StoreArgs(store: scope))
            guard mine == generation else { return }
            items = rows
            loaded = true
            loadFailure = nil
        } catch {
            guard mine == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    @discardableResult
    func restore(_ item: DeletedItemView) async -> Bool {
        guard let core = app.core else { return false }
        busyId = item.id
        defer { busyId = nil }
        let store = scope
        do {
            let row = try await core.trashRestore(TrashItemArgs(store: store, id: item.id))
            app.pageToast(PageText.restoredMessage(item, restored: row), store: store)
            await reload()
            return true
        } catch {
            app.show("Could not restore", error)
            await reload()
            return false
        }
    }

    @discardableResult
    func purge(_ item: DeletedItemView) async -> Bool {
        purging = nil
        guard let core = app.core else { return false }
        busyId = item.id
        defer { busyId = nil }
        do {
            _ = try await core.trashPurge(TrashItemArgs(store: scope, id: item.id))
            app.pageInfo("\(item.title.map { "\u{201C}\($0)\u{201D}" } ?? "The item") was deleted permanently.")
            await reload()
            return true
        } catch {
            app.show("Could not delete", error)
            await reload()
            return false
        }
    }
}

struct RecentlyDeletedWorkspace: View {
    @Environment(AppModel.self) private var app
    @State private var model: RecentlyDeletedModel?

    var body: some View {
        Group {
            if let model {
                RecentlyDeletedScreen(model: model)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .onAppear {
            if model == nil { model = RecentlyDeletedModel(app: app) }
        }
    }
}

struct RecentlyDeletedScreen: View {
    @Bindable var model: RecentlyDeletedModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ProjectPageHeader(title: "Recently Deleted", subtitle: RecentlyDeletedModel.subtitle, identifier: "trash") {
                Picker("Which deleted items", selection: Binding(get: { model.scope }, set: { model.choose($0) })) {
                    Text("This project").tag(StoreSel.project)
                    Text("Global Idea Vault").tag(StoreSel.global)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 280)
                .accessibilityLabel("Which deleted items")
                .accessibilityIdentifier("trash-scope")
            }
            if model.loaded && !model.items.isEmpty {
                HStack {
                    HStack(spacing: 4) {
                        Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                        TextField("Search deleted items", text: $model.text)
                            .textFieldStyle(.plain)
                            .accessibilityLabel("Search deleted items")
                            .accessibilityIdentifier("trash-search")
                    }
                    .padding(.horizontal, 8).padding(.vertical, 5)
                    .frame(width: 260)
                    .background(theme["panel"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).stroke(theme["line"]))
                    Spacer()
                    Text("\(model.items.count) item\(model.items.count == 1 ? "" : "s")").font(.caption).foregroundStyle(theme["muted"])
                        .accessibilityIdentifier("trash-count")
                }
                .padding(.horizontal, 24).padding(.bottom, 10)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    body_
                    Text("Delete Permanently asks for a second, deliberate confirmation. If the original place no longer exists, Restore puts the item in an Unassigned area and tells you.")
                        .font(.callout).fontWeight(.semibold)
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(theme["yellow-soft"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        .foregroundStyle(theme["yellow-t"])
                        .accessibilityIdentifier("trash-banner")
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 24)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["bg"])
        .task(id: model.app.changes.key(["deleted_item"], store: model.storeKind) &+ model.app.changes.any &+ (model.scope == .global ? 1 : 0)) {
            await model.reload()
        }
        .confirmationDialog(
            model.purging.map { "Delete \($0.title.map { "\u{201C}\($0)\u{201D}" } ?? "this \($0.typeLabel.lowercased())") permanently?" } ?? "",
            isPresented: Binding(get: { model.purging != nil }, set: { if !$0 { model.purging = nil } }),
            titleVisibility: .visible,
            presenting: model.purging
        ) { item in
            Button("Delete Permanently", role: .destructive) { Task { await model.purge(item) } }
                .accessibilityIdentifier("trash-purge-confirm")
            Button("Keep in Recently Deleted", role: .cancel) {}
                .accessibilityIdentifier("trash-purge-cancel")
        } message: { item in
            Text("This \(item.typeLabel.lowercased()) will be removed from the project for good. This can't be undone, and it can't be restored afterwards. Anything that was created from it elsewhere in the project is not affected.")
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("trash-workspace")
    }

    @ViewBuilder private var body_: some View {
        if let failure = model.loadFailure {
            ContentUnavailableView {
                Label("Recently Deleted couldn't be loaded.", systemImage: "exclamationmark.triangle")
            } description: {
                Text(failure.message)
            } actions: {
                Button("Try Again") { Task { await model.reload() } }.accessibilityIdentifier("trash-retry")
            }
            .accessibilityIdentifier("trash-load-error")
        } else if !model.loaded {
            ProgressView().frame(maxWidth: .infinity).padding(.top, 30)
        } else if model.items.isEmpty {
            PageCard {
                ContentUnavailableView {
                    Label("Nothing has been deleted.", systemImage: "trash")
                } description: {
                    Text("When you delete something it stays here for 30 days, and you can restore it until then.")
                }
                .padding(.vertical, 12)
            }
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("trash-empty")
        } else if model.shown.isEmpty {
            Text("No deleted items match \u{201C}\(model.text)\u{201D}.")
                .foregroundStyle(theme["muted"]).frame(maxWidth: .infinity).padding(20)
                .accessibilityIdentifier("trash-no-match")
        } else {
            PageCard {
                HStack(spacing: 8) {
                    Text("Item").frame(maxWidth: .infinity, alignment: .leading)
                    Text("Type").frame(width: 110, alignment: .leading)
                    Text("Deleted").frame(width: 110, alignment: .leading)
                    Text("Removed in").frame(width: 90, alignment: .leading)
                    Text("Was in").frame(width: 200, alignment: .leading)
                    Text("Actions").frame(width: 250, alignment: .trailing).opacity(0).accessibilityHidden(true)
                }
                .font(.caption.weight(.semibold))
                .foregroundStyle(theme["muted"])
                .padding(.horizontal, 12).padding(.vertical, 8)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("Columns: Item, Type, Deleted, Removed in, Was in")
                .accessibilityIdentifier("trash-columns")
                Divider().overlay(theme["line"])
                LazyVStack(spacing: 0) {
                    ForEach(model.shown, id: \.id) { item in
                        row(item)
                        Divider().overlay(theme["line"])
                    }
                }
                .accessibilityIdentifier("trash-list")
            }
        }
    }

    private func row(_ i: DeletedItemView) -> some View {
        let name = i.title ?? "Untitled \(i.typeLabel.lowercased())"
        let busy = model.busyId == i.id
        return HStack(alignment: .top, spacing: 8) {
            Text(name).fontWeight(.semibold).frame(maxWidth: .infinity, alignment: .leading)
            Text(i.typeLabel).frame(width: 110, alignment: .leading)
            VStack(alignment: .leading, spacing: 1) {
                Text(PageText.dayWord(ms: i.deletedAt))
                if let by = i.deletedByName { Text("by \(by)").font(.caption).foregroundStyle(theme["muted"]) }
            }
            .frame(width: 110, alignment: .leading)
            Text(PageText.daysLeft(i.daysLeft)).frame(width: 90, alignment: .leading)
            VStack(alignment: .leading, spacing: 1) {
                Text(i.wasIn)
                if i.parentMissing {
                    Text(PageText.parentMissingNote(objectType: i.objectType)).font(.caption).foregroundStyle(theme["muted"])
                }
            }
            .frame(width: 200, alignment: .leading)
            HStack(spacing: 6) {
                Button { Task { await model.restore(i) } } label: { Label("Restore", systemImage: "arrow.uturn.backward") }
                    .controlSize(.small)
                    .disabled(busy)
                    .accessibilityLabel("Restore \(i.title ?? i.typeLabel)")
                    .accessibilityIdentifier("restore-\(i.title ?? i.typeLabel)")
                Button("Delete Permanently", role: .destructive) { model.purging = i }
                    .controlSize(.small)
                    .disabled(busy)
                    .accessibilityLabel("Delete \(i.title ?? i.typeLabel) permanently")
                    .accessibilityIdentifier("purge-\(i.title ?? i.typeLabel)")
            }
            .frame(width: 250, alignment: .trailing)
        }
        .font(.callout)
        .padding(.horizontal, 12).padding(.vertical, 8)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(name), \(i.typeLabel), \(PageText.daysLeft(i.daysLeft)), was in \(i.wasIn)")
        .accessibilityIdentifier("trash-row-\(i.id)")
    }
}

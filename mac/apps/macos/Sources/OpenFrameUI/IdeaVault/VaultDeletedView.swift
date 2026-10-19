// Recently Deleted for the Vault: restore an item, or delete it for good as a separate, confirmed
// step. Items stay 30 days; each row says how many days are left.

import OpenFrameCore
import SwiftUI

struct VaultDeletedView: View {
    @Bindable var model: IdeaVaultModel
    @State private var purging: VaultTrashRow?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(spacing: PageSpacing.tight) {
                ZStack {
                    Text("Recently Deleted").font(.system(size: 26, weight: .semibold)).tracking(-0.3)
                    HStack {
                        Button {
                            model.page = .vault
                        } label: { Label("Back to Vault", systemImage: "chevron.left") }
                            .keyboardShortcut(.cancelAction)
                            .accessibilityIdentifier("vault-back")
                        Spacer()
                    }
                }
                Text("Items you deleted can be restored. · \(VaultText.scopeLabel(model.scope))").foregroundStyle(theme["muted"])
            }
            .padding(.horizontal, 24)
            .padding(.top, 20)
            .padding(.bottom, 12)

            Text("Deleted items stay here for 30 days, then they are removed for good. Restore them before then, or delete them sooner.")
                .font(.callout)
                .padding(10)
                .multilineTextAlignment(.center)
                .frame(maxWidth: .infinity, alignment: .center)
                .background(theme["blue-soft"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .foregroundStyle(theme["blue-t"])
                .padding(.horizontal, 24)

            Group {
                if !model.trashLoaded {
                    ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if model.trash.isEmpty {
                    ContentUnavailableView {
                        Label("Nothing deleted", systemImage: "trash")
                    } description: {
                        Text("Items you delete from the Idea Vault will appear here.")
                    }
                } else {
                    HStack(spacing: 12) {
                        Text("Item").frame(maxWidth: .infinity, alignment: .leading)
                        Text("Deleted").frame(width: 110, alignment: .trailing)
                        Text("Removed in").frame(width: 90, alignment: .trailing)
                        Text("Actions").frame(width: 210, alignment: .trailing).opacity(0).accessibilityHidden(true)
                    }
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(theme["muted"])
                    .padding(.horizontal, 28).padding(.top, 8)
                    List(model.trash, id: \.deleted.id) { row in
                        HStack(spacing: 12) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(row.deleted.title ?? "Untitled").fontWeight(.medium).lineLimit(1)
                                Text("\(row.typeLabel) · \(row.originalPlace)").font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                            }
                            Spacer(minLength: 8)
                            VStack(alignment: .trailing, spacing: 2) {
                                Text("Deleted \(VaultText.relativeDay(ms: row.deleted.deletedAt))").font(.caption).foregroundStyle(theme["muted"])
                                Text(VaultText.daysLeft(row.daysLeft)).font(.caption)
                            }
                            Button("Restore") { Task { await model.restore(row) } }
                                .accessibilityIdentifier("restore-\(row.deleted.title ?? "item")")
                            Button("Delete Permanently", role: .destructive) { purging = row }
                        }
                        .padding(.vertical, 4)
                        .accessibilityElement(children: .contain)
                        .accessibilityLabel("\(row.deleted.title ?? "Untitled"), \(row.typeLabel), \(VaultText.daysLeft(row.daysLeft))")
                    }
                    .scrollContentBackground(.hidden)
                }
            }
            .padding(.top, 8)
        }
        .background(theme["bg"])
        .task(id: model.app.changes.key(VaultTables.trash, store: model.storeKind)) { await model.reloadTrash() }
        .confirmationDialog(
            "Delete permanently?",
            isPresented: Binding(get: { purging != nil }, set: { if !$0 { purging = nil } }),
            titleVisibility: .visible,
            presenting: purging
        ) { row in
            Button("Delete Permanently", role: .destructive) { Task { await model.purge(row) } }
            Button("Keep in Recently Deleted", role: .cancel) {}
        } message: { row in
            Text("“\(row.deleted.title ?? "This item")” and any file stored with it will be removed from this Mac. This can't be undone.")
        }
        .accessibilityIdentifier("vault-deleted")
    }
}

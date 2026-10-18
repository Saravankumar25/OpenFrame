// How the items look: cards for Grid and Card view, rows for List view, folder tiles and the
// breadcrumb for Folder view. A card is one element for VoiceReader with the research flag spoken
// after the name; the flag is also written as words on the card.

import AppKit
import OpenFrameCore
import SwiftUI

/// Words for a research flag. Colour repeats the meaning, it never carries it.
struct ResearchFlagChip: View {
    let flag: VaultResearchFlag?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if let info = VaultText.researchFlag(flag) {
            Text(info.label)
                .font(.caption2.weight(.medium))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .foregroundStyle(foreground(info.tone))
                .background(background(info.tone), in: Capsule())
                .help("Research flag: \(info.label)")
                .accessibilityLabel("Research flag: \(info.label)")
                .accessibilityIdentifier("research-flag")
        }
    }

    private func foreground(_ tone: ResearchFlagInfo.Tone) -> Color {
        switch tone {
        case .plain: theme["ink2"]
        case .good: theme["green-t"]
        case .warn: theme["yellow-t"]
        }
    }

    private func background(_ tone: ResearchFlagInfo.Tone) -> Color {
        switch tone {
        case .plain: theme["surface2"]
        case .good: theme["green-soft"]
        case .warn: theme["yellow-soft"]
        }
    }
}

/// What a click on an item needs to know about the keys held.
@MainActor
func vaultModifierState() -> (command: Bool, shift: Bool) {
    let flags = NSEvent.modifierFlags
    return (flags.contains(.command), flags.contains(.shift))
}

/// The actions a card or row offers, shared by its menu and VoiceOver.
struct VaultItemActions {
    var open: () -> Void
    var rename: () -> Void
    var move: () -> Void
    var pin: () -> Void
    var copy: (() -> Void)?
    var sendToStory: (() -> Void)?
    var addToCollection: () -> Void
    var tag: () -> Void
    var flag: (VaultResearchFlag?) -> Void
    var delete: () -> Void
}

struct VaultItemMenu: View {
    let model: IdeaVaultModel
    let item: VaultItemDto
    let actions: VaultItemActions

    var body: some View {
        let xs = model.targets(for: item)
        let many = xs.count > 1
        Button("Open") { actions.open() }
        Button("Rename") { actions.rename() }.disabled(many)
        Button(many ? "Move \(xs.count) Items…" : "Move…") { actions.move() }
        Button(xs.allSatisfy(\.pinned) ? "Unpin" : "Pin") { actions.pin() }
        Button("Add to Collection…") { actions.addToCollection() }
        Button("Tag…") { actions.tag() }
        if let copy = actions.copy, let label = model.copyLabel {
            Button(label) { copy() }
        }
        if let send = actions.sendToStory, !many {
            Button("Send to Story") { send() }
        }
        Divider()
        Menu("Research flag") {
            ForEach(VaultText.researchFlags, id: \.value) { f in
                Button(xs.allSatisfy { $0.researchFlag == f.value } ? "\(f.label) (set)" : f.label) { actions.flag(f.value) }
            }
            Divider()
            Button("No flag") { actions.flag(nil) }.disabled(xs.allSatisfy { $0.researchFlag == nil })
        }
        Divider()
        Button(many ? "Delete \(xs.count) Items" : "Delete", role: .destructive) { actions.delete() }
    }
}

// MARK: Card

struct VaultItemCard: View {
    let item: VaultItemDto
    let model: IdeaVaultModel
    let selected: Bool
    let full: Bool
    let actions: VaultItemActions

    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ZStack(alignment: .topTrailing) {
                content
                if item.pinned {
                    Image(systemName: "pin.fill")
                        .font(.caption)
                        .foregroundStyle(theme["accent"])
                        .padding(6)
                        .accessibilityLabel("Pinned")
                }
            }
            Divider().overlay(theme["line"])
            HStack(spacing: 6) {
                if VaultText.isExternal(item) {
                    Image(systemName: "link").font(.caption2).accessibilityLabel("Linked file outside the vault")
                }
                Text(VaultText.metaLine(item, collections: model.collections))
                    .font(.caption)
                    .foregroundStyle(theme["muted"])
                    .lineLimit(1)
                ResearchFlagChip(flag: item.researchFlag)
                Spacer(minLength: 0)
                if VaultText.isUnavailable(item) {
                    Text("Unavailable").font(.caption2.weight(.semibold)).foregroundStyle(theme["red-t"])
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            if let reason = item.matchReason {
                Text(reason).font(.caption2).foregroundStyle(theme["muted"]).padding(.horizontal, 10).padding(.bottom, 6)
            }
        }
        .background(item.itemType == .note ? theme["note-bg"] : theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .strokeBorder(selected ? theme["accent"] : theme["line"], lineWidth: selected ? 2 : 1)
        )
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .modifier(VaultItemInteraction(item: item, model: model, selected: selected, actions: actions))
    }

    @ViewBuilder private var content: some View {
        let lines = full ? 12 : 5
        switch item.itemType {
        case .note:
            VStack(alignment: .leading, spacing: 4) {
                if let title = item.title, !title.isEmpty {
                    Text(title).font(.headline).lineLimit(2)
                }
                if let body = item.body, !body.isEmpty {
                    Text(body).font(.callout).foregroundStyle(theme["ink2"]).lineLimit(lines)
                } else if item.title?.isEmpty ?? true {
                    Text("Untitled note").foregroundStyle(theme["muted"])
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, minHeight: full ? 120 : 80, alignment: .topLeading)
        case .quote:
            VStack(alignment: .leading, spacing: 4) {
                Image(systemName: "quote.opening").font(.caption).foregroundStyle(theme["purple"]).accessibilityHidden(true)
                if let title = item.title, !title.isEmpty { Text(title).font(.headline).lineLimit(2) }
                Text(item.body.map { "“\($0)”" } ?? "Untitled quote").italic().font(.callout).lineLimit(lines)
                if let source = item.sourceText, !source.isEmpty {
                    Text("Source: \(source)").font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, minHeight: 80, alignment: .topLeading)
        case .url:
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 10) {
                    typeTile
                    VStack(alignment: .leading, spacing: 2) {
                        Text(item.title ?? VaultText.host(of: item.url)).font(.headline).lineLimit(1)
                        Text(VaultText.host(of: item.url)).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                    }
                }
                if let body = item.body, !body.isEmpty {
                    Text(body).font(.callout).foregroundStyle(theme["ink2"]).lineLimit(lines)
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        default:
            VStack(alignment: .leading, spacing: 8) {
                if VaultText.isVisualType(item.itemType) {
                    VaultThumbnailView(item: item, store: model.scope, height: full ? 190 : 120)
                } else if item.itemType == .video {
                    ZStack {
                        theme["surface2"]
                        Label(VaultText.duration(ms: item.asset?.durationMs) ?? "Video", systemImage: "play.fill")
                            .font(.caption).foregroundStyle(theme["ink2"])
                    }
                    .frame(height: full ? 120 : 70)
                    .accessibilityHidden(true)
                } else {
                    HStack(spacing: 10) {
                        typeTile
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.asset?.originalName ?? item.displayName).font(.headline).lineLimit(1)
                            if let d = VaultText.duration(ms: item.asset?.durationMs) {
                                Text(d).font(.caption).foregroundStyle(theme["muted"])
                            }
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.top, 12)
                }
                if (item.title?.isEmpty == false) || (item.caption?.isEmpty == false) {
                    VStack(alignment: .leading, spacing: 2) {
                        if let t = item.title, !t.isEmpty { Text(t).font(.headline).lineLimit(2) }
                        if let c = item.caption, !c.isEmpty { Text(c).font(.callout).foregroundStyle(theme["ink2"]).lineLimit(lines) }
                    }
                    .padding(.horizontal, 12)
                    .padding(.bottom, 10)
                } else if VaultText.isVisualType(item.itemType) || item.itemType == .video {
                    Text(item.displayName).font(.callout.weight(.medium)).lineLimit(1).padding(.horizontal, 12).padding(.bottom, 10)
                } else {
                    Spacer().frame(height: 2)
                }
            }
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
    }

    private var typeTile: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8, style: .continuous).fill(theme["accent-soft"])
            if item.itemType == .url || item.itemType == .audio || item.itemType == .voice || item.itemType == .video {
                Image(systemName: VaultText.symbol(item.itemType)).foregroundStyle(theme["accent-ink"])
            } else {
                Text(item.itemType == .pdf ? "PDF" : VaultText.extLabel(item.asset?.originalName))
                    .font(.caption2.weight(.bold)).foregroundStyle(theme["accent-ink"])
            }
        }
        .frame(width: 38, height: 38)
        .accessibilityHidden(true)
    }
}

/// Click, double click, menu, drag and VoiceOver actions that a card and a row share.
struct VaultItemInteraction: ViewModifier {
    let item: VaultItemDto
    let model: IdeaVaultModel
    let selected: Bool
    let actions: VaultItemActions

    func body(content: Content) -> some View {
        content
            .onTapGesture(count: 2) { actions.open() }
            .onTapGesture {
                let keys = vaultModifierState()
                model.select(item, command: keys.command, shift: keys.shift)
            }
            .contextMenu {
                VaultItemMenu(model: model, item: item, actions: actions)
            }
            .draggable(VaultDragPayload.encode(store: model.scope, ids: model.targets(for: item).map(\.id)))
            .accessibilityElement(children: .combine)
            .accessibilityLabel(VaultText.accessibilityLabel(item))
            .accessibilityValue(VaultText.researchFlag(item.researchFlag).map { "Research flag: \($0.label)" } ?? "")
            .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
            .accessibilityIdentifier("vault-card-\(item.displayName)")
            .accessibilityAction(named: "Open") { actions.open() }
            .accessibilityAction(named: item.pinned ? "Unpin" : "Pin") { actions.pin() }
            .accessibilityAction(named: "Delete") { actions.delete() }
    }
}

/// Dragging items onto a folder or collection in the sidebar. Plain text with a prefix, so no
/// custom type needs to be declared for the app.
enum VaultDragPayload {
    private static let prefix = "openframe-vault:"

    static func encode(store: StoreSel, ids: [String]) -> String {
        "\(prefix)\(store.rawValue):\(ids.joined(separator: ","))"
    }

    static func decode(_ text: String) -> (store: StoreSel, ids: [String])? {
        guard text.hasPrefix(prefix) else { return nil }
        let rest = text.dropFirst(prefix.count).split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard rest.count == 2, let store = StoreSel(rawValue: String(rest[0])) else { return nil }
        let ids = rest[1].split(separator: ",").map(String.init)
        return ids.isEmpty ? nil : (store, ids)
    }
}

// MARK: Row

struct VaultItemRow: View {
    let item: VaultItemDto
    let model: IdeaVaultModel
    let selected: Bool
    let actions: VaultItemActions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: VaultText.symbol(item.itemType))
                .frame(width: 22)
                .foregroundStyle(theme["accent"])
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 1) {
                Text(item.displayName).lineLimit(1)
                Text(VaultText.metaLine(item, collections: model.collections)).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
            }
            Spacer(minLength: 8)
            ResearchFlagChip(flag: item.researchFlag)
            if VaultText.isUnavailable(item) {
                Text("Unavailable").font(.caption2.weight(.semibold)).foregroundStyle(theme["red-t"])
            }
            if item.pinned {
                Image(systemName: "pin.fill").font(.caption).foregroundStyle(theme["accent"]).accessibilityLabel("Pinned")
            }
            Text(VaultText.relativeDay(ms: item.updatedAt)).font(.caption).foregroundStyle(theme["muted"]).frame(width: 90, alignment: .trailing)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(selected ? theme["accent-soft"] : theme["panel"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(selected ? theme["accent"] : theme["line"], lineWidth: selected ? 1.5 : 1))
        .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .modifier(VaultItemInteraction(item: item, model: model, selected: selected, actions: actions))
    }
}

// MARK: Folder tiles and breadcrumb

struct VaultFolderTile: View {
    let folder: VaultFolderDto
    let open: () -> Void
    let menu: () -> AnyView
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Button(action: open) {
            HStack(spacing: 10) {
                Image(systemName: "folder.fill").font(.title3).foregroundStyle(theme["accent"])
                VStack(alignment: .leading, spacing: 1) {
                    Text(folder.name).font(.headline).lineLimit(1)
                    Text("\(Int(folder.itemCount)) \(folder.itemCount == 1 ? "item" : "items")").font(.caption).foregroundStyle(theme["muted"])
                }
                Spacer(minLength: 0)
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
        .contextMenu { menu() }
        .accessibilityLabel("Folder \(folder.name), \(Int(folder.itemCount)) items")
        .accessibilityIdentifier("vault-folder-\(folder.name)")
    }
}

struct VaultCrumbs: View {
    let path: [VaultFolderDto]
    let go: (String?) -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: 4) {
            Button("All items") { go(nil) }.buttonStyle(.link)
            ForEach(path, id: \.id) { f in
                Image(systemName: "chevron.right").font(.caption2).foregroundStyle(theme["muted"]).accessibilityHidden(true)
                if f.id == path.last?.id {
                    Text(f.name).fontWeight(.semibold)
                } else {
                    Button(f.name) { go(f.id) }.buttonStyle(.link)
                }
            }
            Spacer(minLength: 0)
        }
        .font(.callout)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Folder path")
    }
}

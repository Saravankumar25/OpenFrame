// The scene's breakdown (right pane): categories with content, confirmed rows, visibly different
// suggestion rows with Accept, Edit and Reject, quick reuse of catalog items from nearby scenes,
// and the Add Element and Suggest Elements actions. Every category shows its colour with an icon,
// its text and a tooltip.

import SwiftUI
import OpenFrameCore

struct ElementsPanelView: View {
    @Bindable var model: BreakdownModel
    let detail: BreakdownSceneDetail
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private var n: String { detail.number ?? "" }
    private var canEdit: Bool { model.canEdit }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                header
                facts
                banners
                if detail.groups.isEmpty {
                    Text("Nothing broken down yet. Select words in the script to tag them, add an element, or ask for suggestions.")
                        .font(.callout).foregroundStyle(theme["muted"]).padding(.vertical, 8)
                        .accessibilityIdentifier("breakdown-empty")
                }
                ForEach(detail.groups, id: \.category) { g in group(g) }
                if detail.inSource, canEdit, !model.reuseOffers.isEmpty { reuse }
                if detail.inSource, canEdit { actions }
            }
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(theme["panel"])
        .onDeleteCommand {
            if let id = model.selectedElementId,
               let e = detail.groups.flatMap(\.confirmed).first(where: { $0.id == id }), canEdit {
                Task { await model.remove(e) }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(detail.number != nil ? "Scene \(n) breakdown" : "Removed scene")
        .accessibilityIdentifier("elements-panel")
    }

    // MARK: Header

    private var header: some View {
        HStack {
            Text(detail.number != nil ? "Scene \(n) breakdown" : "Removed scene").font(.headline)
            Spacer()
            if detail.inSource, canEdit {
                Button {
                    Task { await model.setComplete(!detail.complete) }
                } label: {
                    Label(detail.complete ? "Complete" : "Mark Complete", systemImage: detail.complete ? "checkmark.circle.fill" : "checkmark.circle")
                }
                .controlSize(.small)
                .disabled(model.busy)
                .help(detail.complete ? "Reopen this scene's breakdown" : "Mark this scene's breakdown complete")
                .accessibilityIdentifier("mark-complete")
            }
        }
    }

    @ViewBuilder private var facts: some View {
        if detail.facts.intExt != nil || detail.facts.timeOfDay != nil || detail.facts.setName != nil {
            HStack(spacing: 6) {
                if let v = detail.facts.intExt { BDChip(text: v) }
                if let v = detail.facts.timeOfDay { BDChip(text: v) }
                if let v = detail.facts.setName { Text(v).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1) }
            }
        }
    }

    @ViewBuilder private var banners: some View {
        if !detail.inSource {
            BDBanner(.info, "Scene removed from current source. Its breakdown is kept for reference.")
        }
        if let msg = detail.changeMessage {
            BDBanner(tone: .warn, content: { Text(msg) }, actions: {
                Button("Review") { model.sheet = .changes(detail.sceneId) }.controlSize(.small)
                    .accessibilityIdentifier("review-scene-changes")
            })
        }
        if detail.suggestionCount > 0 {
            BDBanner(tone: .warn, content: { Text("Suggested elements: \(Int(detail.suggestionCount))").bold() }, actions: {
                Button { model.openReview() } label: { Label("Review\u{2026}", systemImage: "eye") }
                    .controlSize(.small)
                    .keyboardShortcut("r", modifiers: [.command, .option])
                    .accessibilityIdentifier("review-suggestions")
            })
            .accessibilityIdentifier("suggested-banner")
        }
    }

    // MARK: Groups

    private func group(_ g: BreakdownCategoryGroup) -> some View {
        let collapsed = model.collapsed.contains(g.category)
        return VStack(alignment: .leading, spacing: 4) {
            Button { model.toggleCollapsed(g.category) } label: {
                HStack(spacing: 6) {
                    Image(systemName: collapsed ? "chevron.right" : "chevron.down").font(.caption2).frame(width: 10)
                    CategoryMark(look: hub.look(g.category))
                    Spacer()
                    Text("\(g.confirmed.filter { !$0.archived }.count)").font(.caption).foregroundStyle(theme["muted"])
                }
                .font(.callout.weight(.semibold))
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("\(g.category.rawValue), \(g.confirmed.filter { !$0.archived }.count) elements")
            .accessibilityValue(collapsed ? "Collapsed" : "Expanded")
            if !collapsed {
                ForEach(g.confirmed, id: \.id) { confirmedRow($0) }
                ForEach(g.suggestions, id: \.id) { suggestionRow($0) }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(g.category.rawValue)
        .accessibilityIdentifier("category-section-\(g.category.rawValue)")
    }

    private func confirmedRow(_ e: BreakdownElementDto) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Circle().fill(hub.look(e.category).color ?? theme["muted"]).frame(width: 8, height: 8).padding(.top, 5).accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 1) {
                Text(e.name).font(.callout.weight(.semibold))
                if let sub = e.notes ?? (e.name != e.displayName && !e.displayName.isEmpty ? "tagged as \(BDText.quoted(e.displayName))" : nil) {
                    Text(sub).font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 4)
            ElementStatus(e: e)
            Menu {
                elementMenu(e)
            } label: {
                Image(systemName: "ellipsis.circle")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .accessibilityLabel("Actions for \(e.name)")
            .accessibilityIdentifier("element-actions-\(e.name)")
        }
        .padding(.horizontal, 8).padding(.vertical, 5)
        .background(model.selectedElementId == e.id ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6, style: .continuous))
        .opacity(e.archived ? 0.55 : 1)
        .contentShape(Rectangle())
        .onTapGesture { model.selectedElementId = e.id }
        .contextMenu { elementMenu(e) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("element-\(e.name)")
    }

    @ViewBuilder private func elementMenu(_ e: BreakdownElementDto) -> some View {
        Button("Show in script") { Task { await model.showSource(e) } }.disabled(e.evidence == nil)
        Button("Show all scenes with this item") { if let id = e.catalogItemId { hub.openScenes(forCatalogItem: id) } }.disabled(e.catalogItemId == nil)
        Button("Open in Catalog") { if let id = e.catalogItemId { hub.openInCatalog(id) } }.disabled(e.catalogItemId == nil)
        Button("Edit notes\u{2026}") { model.sheet = .notes(e) }.disabled(!canEdit)
        Divider()
        Button("Remove from scene", role: .destructive) { Task { await model.remove(e) } }.disabled(!canEdit)
    }

    private func suggestionRow(_ s: BreakdownElementDto) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                BDChip(text: "Suggested", tone: .yellow)
                Text("\(s.displayName) (suggested)").font(.callout.weight(.semibold)).fixedSize(horizontal: false, vertical: true)
            }
            Text("matched: \(BDText.quoted(s.evidence ?? ""))").font(.caption).foregroundStyle(theme["muted"]).lineLimit(2)
                .help(s.evidence ?? "")
            if canEdit {
                HStack(spacing: 6) {
                    Button("Accept") { Task { await model.accept(s) } }
                        .accessibilityLabel("Accept \(s.name)")
                        .accessibilityIdentifier("accept-\(s.name)")
                    Button("Edit") { model.sheet = .edit(s, chooser: false) }
                        .accessibilityLabel("Edit \(s.name)")
                    Button("Reject") { Task { await model.reject([s.id]) } }
                        .accessibilityLabel("Reject \(s.name)")
                }
            }
        }
        .controlSize(.small)
        .padding(.horizontal, 8).padding(.vertical, 6)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["suggest-bg"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("suggestion-\(s.name)")
    }

    // MARK: Reuse and actions

    private var reuse: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Quick reuse from nearby scenes").font(.caption).foregroundStyle(theme["muted"])
            BDFlowLayout(spacing: 4) {
                ForEach(model.reuseOffers, id: \.catalogItemId) { i in
                    Button { Task { await model.reuse(i) } } label: {
                        CategoryMark(look: hub.look(i.category), labelOverride: i.name, size: 11)
                    }
                    .controlSize(.small)
                    .disabled(model.busy)
                    .help("\(i.category.rawValue). Used in \(BDText.usedIn(i.usedInLabel)).")
                    .accessibilityLabel("Reuse \(i.name) (\(i.category.rawValue)) in Scene \(n)")
                }
            }
        }
        .padding(.top, 6)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Quick reuse")
        .accessibilityIdentifier("quick-reuse")
    }

    private var actions: some View {
        HStack(spacing: 8) {
            Button { model.addByHand() } label: { Label("Add Element", systemImage: "plus") }
                .keyboardShortcut("n", modifiers: .command)
                .accessibilityIdentifier("add-element")
            Button { Task { await model.suggest() } } label: {
                Label(model.suggesting ? "Suggesting\u{2026}" : "Suggest Elements", systemImage: "sparkles")
            }
            .keyboardShortcut("s", modifiers: [.command, .option])
            .disabled(model.suggesting)
            .accessibilityIdentifier("suggest-elements")
            if model.selection != nil {
                Button { model.tagSelection() } label: { Label("Tag Selection", systemImage: "tag") }
                    .keyboardShortcut("t", modifiers: [.command, .shift])
                    .accessibilityIdentifier("tag-selection")
            }
        }
        .controlSize(.small)
        .padding(.top, 6)
    }
}

struct ElementStatus: View {
    let e: BreakdownElementDto
    var body: some View {
        if e.catalogRemoved {
            BDChip(text: "Catalog item removed", tone: .red)
                .help("The catalog item was deleted. Restore it from Recently Deleted or link another item.")
        } else if e.catalogArchived {
            BDChip(text: "Archived item").help("Archived in the Catalog and kept readable here.")
        } else if let s = e.catalogStatus {
            BDChip(text: s.rawValue, tone: s.tone)
        }
    }
}

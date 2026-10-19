// The Catalog tab: "What things does this film need? Reusable items, not one huge spreadsheet."
// (FSD 28, 98, 143 to 144; UX 3.23). A table of items with search and a category filter, an
// inspector for the selected item, and sheets to add, archive, replace and delete.

import AppKit
import SwiftUI
import OpenFrameCore

/// Header, scrolling body and footer of the inspector of a record.
struct BDInspectorFrame<Content: View, Footer: View>: View {
    let typeLabel: String
    let title: String
    let onClose: () -> Void
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(typeLabel.uppercased()).font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
                    Text(title).font(.headline).lineLimit(2)
                }
                Spacer()
                Button("Close", action: onClose)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("inspector-close")
            }
            .padding(14)
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 14) { content() }
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            Divider()
            HStack(spacing: 8) { footer() }.padding(10)
        }
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel(typeLabel)
    }
}

/// A search field, a segmented filter and the "Show archived" switch shared by the list tabs.
struct BDPageHeader<Actions: View>: View {
    let title: String
    let sub: String
    @ViewBuilder var actions: () -> Actions

    var body: some View {
        PageHeader(title, subtitle: sub) { actions() }
    }
}

struct CatalogRow: Identifiable {
    let item: CatalogItemDto
    var id: String { item.id }
}

struct CatalogView: View {
    @Bindable var model: CatalogModel
    @Environment(AppModel.self) private var app
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private static let mainFilters: [(BreakdownCategory?, String)] = [
        (nil, "All"), (.props, "Props"), (.wardrobe, "Wardrobe"), (.vehicles, "Vehicles"), (.cast, "Cast"), (.locationSet, "Locations"),
    ]
    private static var more: [BreakdownCategory] { BDCategory.all.filter { c in !mainFilters.contains { $0.0 == c } } }

    private var reloadKey: String {
        "\(app.changes.key(BDTables.production))|\(model.filter?.rawValue ?? "")|\(model.showArchived)|\(model.search)|\(model.selectedId ?? "")"
    }

    var body: some View {
        ScrollingPage {
        VStack(spacing: PageSpacing.medium) {
            BDPageHeader(title: "Catalog", sub: "What things does this film need? Reusable items, not one huge spreadsheet.") {
                if model.canEdit {
                    Button { model.sheet = .add(model.filter ?? .props) } label: { Label("Add Item", systemImage: "plus") }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("add-item")
                }
            }
            toolbar
            list
        }
        .pageColumn(maxWidth: PageSpacing.wideColumn)
        .padding(.bottom, PageSpacing.medium)
        }
        .task(id: reloadKey) {
            if !model.search.isEmpty { try? await Task.sleep(for: .milliseconds(150)) }
            guard !Task.isCancelled else { return }
            await model.reload()
        }
        .inspector(isPresented: Binding(get: { model.selected != nil }, set: { if !$0 { model.selectedId = nil } })) {
            if let item = model.selected {
                CatalogInspector(model: model, item: item).inspectorColumnWidth(min: 320, ideal: 380, max: 520)
            }
        }
        .sheet(item: $model.sheet) { CatalogSheetHost(model: model, sheet: $0) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("catalog-tab")
    }

    private var toolbar: some View {
        CenteredWrap(spacing: 10, lineSpacing: PageSpacing.small) {
            HStack(spacing: 4) {
                Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                TextField("Search catalog", text: $model.search)
                    .textFieldStyle(.plain)
                    .frame(width: 190)
                    .accessibilityLabel("Search catalog")
                    .accessibilityIdentifier("catalog-search")
            }
            .padding(.horizontal, 8).padding(.vertical, 5)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 7, style: .continuous))
            Picker("Category", selection: Binding(get: { Self.mainFilters.contains { $0.0 == model.filter } ? model.filter : nil }, set: { model.filter = $0 })) {
                ForEach(Self.mainFilters, id: \.1) { Text($0.1).tag($0.0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .accessibilityLabel("Category")
            Menu(Self.more.contains { $0 == model.filter } ? "\(model.filter?.rawValue ?? "") \u{25BE}" : "More \u{25BE}") {
                ForEach(Self.more, id: \.self) { c in Button(c.rawValue) { model.filter = c } }
            }
            .menuStyle(.borderlessButton)
            .fixedSize()
            .accessibilityLabel("More categories")
            Toggle("Show archived", isOn: $model.showArchived).toggleStyle(.checkbox)
                .accessibilityIdentifier("catalog-show-archived")
        }
        .frame(maxWidth: .infinity, alignment: .center)
    }

    @ViewBuilder private var list: some View {
        if !model.loaded {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let f = model.loadFailure, model.items.isEmpty {
            BDBanner(tone: .err, content: { Text(f.message) }, actions: { Button("Try Again") { Task { await model.reload() } } })
        } else if model.items.isEmpty {
            if model.isFiltered {
                Text("No catalog items match.").foregroundStyle(.secondary).padding(12)
            } else {
                ContentUnavailableView {
                    Label("The catalog is empty.", systemImage: "shippingbox")
                } description: {
                    Text("Confirmed breakdown items will appear here, or add one manually.")
                } actions: {
                    if model.canEdit { Button("Add Item") { model.sheet = .add(.props) } }
                }
            }
        } else {
            Table(model.items.map(CatalogRow.init), selection: $model.selectedId) {
                TableColumn("Name") { r in
                    let i = r.item
                    HStack(spacing: 6) {
                        Text(i.name).fontWeight(.semibold).lineLimit(1)
                        if i.archived { BDChip(text: "Archived") }
                    }
                    .opacity(i.archived ? 0.55 : 1)
                    .accessibilityIdentifier("catalog-row-\(i.name)")
                }
                TableColumn("Category") { r in let i = r.item; CategoryMark(look: hub.look(i.category), labelOverride: BDCategory.short(i.category)) }
                TableColumn("Status") { r in let i = r.item; BDChip(text: i.status.rawValue, tone: i.status.tone) }
                TableColumn("Used in") { r in let i = r.item; Text(i.usedInLabel).foregroundStyle(theme["muted"]).lineLimit(1) }
            }
            .contextMenu(forSelectionType: String.self) { ids in
                if let id = ids.first, let i = model.items.first(where: { $0.id == id }) {
                    Button("Open") { model.selectedId = i.id }
                    Button("Open Scenes") { hub.openScenes(forCatalogItem: i.id) }.disabled(i.usedIn.isEmpty)
                    Button(i.archived ? "Restore from Archive" : "Archive\u{2026}") {
                        if i.archived { Task { await model.setArchived(i, false) } } else { model.sheet = .archive(i) }
                    }.disabled(!model.canEdit)
                    Divider()
                    Button("Delete\u{2026}", role: .destructive) { model.sheet = .delete(i) }.disabled(!model.canEdit)
                }
            } primaryAction: { ids in
                if let id = ids.first { model.selectedId = id }
            }
            .onDeleteCommand {
                if let i = model.selected, model.canEdit { model.sheet = .delete(i) }
            }
            .frame(minHeight: 280)
            .accessibilityLabel("Catalog items")
            .accessibilityIdentifier("catalog-table")
        }
    }
}

// MARK: Inspector

struct CatalogInspector: View {
    @Bindable var model: CatalogModel
    let item: CatalogItemDto
    @Environment(BreakdownHub.self) private var hub
    @State private var alias = ""

    private var canEdit: Bool { model.canEdit }

    var body: some View {
        BDInspectorFrame(typeLabel: "Catalog item", title: item.name, onClose: { model.selectedId = nil }) {
            if item.archived { BDBanner(.info, "Archived: kept readable in its scenes, not offered for new selections.") }
            BufferedField(id: "ci-name", label: "Name", value: item.name, required: true, disabled: !canEdit) { v in Task { await model.update(item.id, name: v) } }
            BDField(label: "Category") { CategoryMark(look: hub.look(item.category)).font(.callout) }
            BDField(label: "Status") {
                Picker("Status", selection: Binding(get: { item.status }, set: { s in if canEdit { Task { await model.update(item.id, status: s) } } })) {
                    ForEach(CatalogStatus.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                .labelsHidden()
                .disabled(!canEdit)
                .accessibilityLabel("Status")
                .accessibilityIdentifier("ci-status")
            }
            BufferedField(id: "ci-desc", label: "Description", value: item.description, multiline: true, disabled: !canEdit) { v in Task { await model.update(item.id, description: v) } }
            BufferedField(id: "ci-notes", label: "Notes", value: item.notes, multiline: true, placeholder: "Add notes\u{2026}", disabled: !canEdit) { v in Task { await model.update(item.id, notes: v) } }
            ContactField(id: "ci-contact", label: "Contact / reference", value: item.contact, hidden: item.contactHidden, canEdit: canEdit) { v in Task { await model.update(item.id, contact: v) } }
            imageField
            aliasField
            BDField(label: "Used in scenes") { SceneChips(scenes: item.usedIn) }
            Text("No financial or procurement fields. This is a production directory.").font(.caption).foregroundStyle(.secondary)
        } footer: {
            if canEdit { Button("Delete") { model.sheet = .delete(item) }.accessibilityIdentifier("catalog-delete") }
            Spacer()
            if canEdit {
                Button(item.archived ? "Restore" : "Archive") {
                    if item.archived { Task { await model.setArchived(item, false) } } else { model.sheet = .archive(item) }
                }
                .accessibilityIdentifier("catalog-archive")
            }
            Button("Open Scenes") { hub.openScenes(forCatalogItem: item.id) }
                .disabled(item.usedIn.isEmpty)
                .accessibilityIdentifier("open-scenes")
        }
        .id(item.id)
    }

    private var imageField: some View {
        BDField(label: "Image") {
            if let img = item.image {
                ZStack(alignment: .topTrailing) {
                    AssetImageView(asset: img, maxPixel: 720)
                        .frame(maxWidth: .infinity).frame(height: 160)
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                    if canEdit {
                        Button { Task { await model.setImage(item.id, path: nil) } } label: { Image(systemName: "xmark.circle.fill") }
                            .buttonStyle(.borderless).padding(6)
                            .accessibilityLabel("Remove image")
                    }
                }
                if !img.available { BDChip(text: "Image file unavailable", tone: .red) }
            } else if canEdit {
                Button { if let p = ImagePicker.choose(multiple: false).first { Task { await model.setImage(item.id, path: p) } } } label: {
                    Label("Add Image", systemImage: "photo.badge.plus")
                }
                .controlSize(.small)
            } else {
                Text("No image").font(.callout).foregroundStyle(.secondary)
            }
        }
    }

    private var aliasField: some View {
        BDField(label: "Also known as", hint: "Other names used in the script. They help match this item in new scenes.") {
            VStack(alignment: .leading, spacing: 6) {
                if item.aliases.isEmpty {
                    Text("None").font(.caption).foregroundStyle(.secondary)
                } else {
                    BDFlowLayout(spacing: 4) {
                        ForEach(item.aliases, id: \.id) { a in
                            HStack(spacing: 3) {
                                BDChip(text: a.alias)
                                if canEdit {
                                    Button { Task { await model.removeAlias(a.id) } } label: { Image(systemName: "xmark.circle") }
                                        .buttonStyle(.borderless)
                                        .accessibilityLabel("Remove name \(a.alias)")
                                }
                            }
                        }
                    }
                }
                if canEdit {
                    HStack {
                        TextField("Add another name", text: $alias)
                            .textFieldStyle(.roundedBorder)
                            .onSubmit { add() }
                            .accessibilityLabel("Add another name")
                        Button("Add") { add() }.disabled(alias.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                }
            }
        }
    }

    private func add() {
        let a = alias
        Task { if await model.addAlias(item.id, a) { alias = "" } }
    }
}

// MARK: Sheets

private struct CatalogSheetHost: View {
    @Bindable var model: CatalogModel
    let sheet: CatalogSheet

    var body: some View {
        switch sheet {
        case let .add(c): AddItemSheet(model: model, initialCategory: c)
        case let .archive(i): ArchiveItemSheet(model: model, item: i)
        case let .replace(i): ReplaceItemSheet(model: model, item: i)
        case let .delete(i):
            BDConfirmSheet(title: "Delete \(BDText.quoted(i.name))?", confirmLabel: "Delete", danger: true,
                           onConfirm: { Task { await model.delete(i) } }, onCancel: { model.sheet = nil }) {
                VStack(alignment: .leading, spacing: 8) {
                    ReferencesNote(kind: .catalogItem, id: i.id)
                    if !i.usedIn.isEmpty { Text("Archiving is usually the better choice.").font(.callout).foregroundStyle(.secondary) }
                }
            }
        }
    }
}

private struct AddItemSheet: View {
    @Bindable var model: CatalogModel
    @State private var category: BreakdownCategory
    @State private var name = ""
    @State private var status: CatalogStatus = .required
    @State private var description = ""
    @State private var finder: CatalogMatchFinder?
    @FocusState private var focused: Bool

    init(model: CatalogModel, initialCategory: BreakdownCategory) {
        self.model = model
        _category = State(initialValue: initialCategory)
    }

    private var exact: CatalogMatchDto? { finder?.matches.first { $0.exact } }

    var body: some View {
        BDSheetFrame(title: "Add Item", sub: "A reusable production item. Scenes that need it are linked from the Breakdown.") {
            VStack(alignment: .leading, spacing: 10) {
                BDField(label: "Category") {
                    Picker("Category", selection: $category) { ForEach(BDCategory.all, id: \.self) { Text($0.rawValue).tag($0) } }
                        .labelsHidden().accessibilityLabel("Category").accessibilityIdentifier("ai-category")
                }
                BDField(label: "Name") {
                    TextField("Name", text: $name).textFieldStyle(.roundedBorder).focused($focused)
                        .onSubmit { submit() }
                        .accessibilityLabel("Name").accessibilityIdentifier("ai-name")
                }
                if let exact {
                    BDBanner(.warn, "The catalog already has \(BDText.quoted(exact.name)). Adding creates a separate item; to reuse it, link it from the Breakdown instead.")
                }
                BDField(label: "Status") {
                    Picker("Status", selection: $status) { ForEach(CatalogStatus.allCases, id: \.self) { Text($0.rawValue).tag($0) } }
                        .labelsHidden().accessibilityLabel("Status")
                }
                BDField(label: "Description (optional)") {
                    TextField("Description", text: $description, axis: .vertical).lineLimit(2...4).textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Description")
                }
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Add Item") { submit() }
                .keyboardShortcut(.defaultAction)
                .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
                .accessibilityIdentifier("ai-submit")
        }
        .onAppear {
            finder = CatalogMatchFinder(core: { [weak model] in model?.hub.core })
            focused = true
        }
        .onChange(of: name) { _, n in finder?.update(category: category, name: n) }
        .onChange(of: category) { _, c in finder?.update(category: c, name: name) }
    }

    private func submit() {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        Task { await model.create(category: category, name: name, status: status, description: description) }
    }
}

private struct ArchiveItemSheet: View {
    @Bindable var model: CatalogModel
    let item: CatalogItemDto

    var body: some View {
        BDConfirmSheet(title: "Archive \(BDText.quoted(item.name))?", confirmLabel: "Archive Item",
                       onConfirm: { Task { await model.setArchived(item, true) } }, onCancel: { model.sheet = nil }) {
            VStack(alignment: .leading, spacing: 10) {
                Text(item.usedIn.isEmpty
                     ? "The item will no longer be offered for new selections. You can restore it at any time."
                     : "This item is still used in \(BDText.plural(item.usedIn.count, "scene")) (\(item.usedIn.map { $0.number ?? "removed" }.joined(separator: ", "))). Archiving keeps those associations readable but the item will no longer be offered for new selections.")
                    .font(.callout).fixedSize(horizontal: false, vertical: true)
                if !item.usedIn.isEmpty, model.canEdit {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Instead").font(.callout.weight(.semibold))
                        Text("You can also replace it in selected scenes with another catalog item without deleting it.").font(.callout)
                        Button("Replace in Scenes\u{2026}") { model.sheet = .replace(item) }.controlSize(.small)
                    }
                    .padding(10)
                    .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
            }
        }
    }
}

private struct ReplaceItemSheet: View {
    @Bindable var model: CatalogModel
    let item: CatalogItemDto
    @State private var options: [CatalogItemDto] = []
    @State private var loaded = false
    @State private var target = ""
    @State private var scenes: Set<String>

    init(model: CatalogModel, item: CatalogItemDto) {
        self.model = model
        self.item = item
        _scenes = State(initialValue: Set(item.usedIn.filter(\.inSource).map(\.sceneId)))
    }

    var body: some View {
        let chosen = target.isEmpty ? (options.first?.id ?? "") : target
        BDSheetFrame(title: "Replace \(BDText.quoted(item.name)) in scenes",
                     sub: "This changes production associations only. The screenplay is not changed and the item is not deleted.") {
            VStack(alignment: .leading, spacing: 10) {
                if loaded, options.isEmpty {
                    BDBanner(.info, "There is no other \(BDCategory.short(item.category).lowercased()) item to use. Add one in the Catalog first.")
                } else {
                    BDField(label: "Replacement") {
                        Picker("Replacement", selection: Binding(get: { chosen }, set: { target = $0 })) {
                            ForEach(options, id: \.id) { Text("\($0.name) (\($0.status.rawValue))").tag($0.id) }
                        }
                        .labelsHidden().accessibilityLabel("Replacement")
                    }
                }
                BDField(label: "Apply to scenes") {
                    VStack(alignment: .leading, spacing: 4) {
                        ForEach(item.usedIn.filter(\.inSource), id: \.sceneId) { s in
                            Toggle("Scene \(s.number ?? ""): \(s.heading)", isOn: Binding(
                                get: { scenes.contains(s.sceneId) },
                                set: { if $0 { scenes.insert(s.sceneId) } else { scenes.remove(s.sceneId) } }))
                                .toggleStyle(.checkbox)
                        }
                    }
                }
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Replace in \(BDText.plural(scenes.count, "Scene"))") { Task { await model.replace(item, with: chosen, sceneIds: Array(scenes)) } }
                .keyboardShortcut(.defaultAction)
                .disabled(chosen.isEmpty || scenes.isEmpty)
        }
        .task {
            options = await model.replacementCandidates(for: item)
            loaded = true
        }
    }
}

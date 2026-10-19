// The Locations tab: "Which places can we use? A practical scouting notebook." (FSD 29, 99;
// UX 3.24). Cards with a photo, status and scene count; an inspector with the call-sheet fields
// (nearest hospital, emergency contact), dated availability, photos and practical notes.

import SwiftUI
import OpenFrameCore

private struct NoteField {
    let key: WritableKeyPath<ProductionLocationNotes, String?>
    let id: String
    let label: String
    let placeholder: String
}

private var noteFields: [NoteField] { [
    NoteField(key: \.parking, id: "parking", label: "Parking", placeholder: "Where can the unit park?"),
    NoteField(key: \.noise, id: "noise", label: "Noise", placeholder: "Traffic, trains, neighbours\u{2026}"),
    NoteField(key: \.permission, id: "permission", label: "Permission", placeholder: "Who approves, what is needed"),
    NoteField(key: \.access, id: "access", label: "Access", placeholder: "Gates, keys, stairs, load-in"),
    NoteField(key: \.power, id: "power", label: "Power", placeholder: "Sockets, generator needed?"),
    NoteField(key: \.toilets, id: "toilets", label: "Toilets", placeholder: "Available for the crew?"),
    NoteField(key: \.facilities, id: "facilities", label: "Nearby facilities", placeholder: "Food, shelter, shops\u{2026}"),
    NoteField(key: \.travel, id: "travel", label: "Travel", placeholder: "Distance and route from base"),
    NoteField(key: \.general, id: "general", label: "Other notes", placeholder: "Anything else"),
] }

struct LocationsView: View {
    @Bindable var model: LocationsModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    private var reloadKey: String {
        "\(app.changes.key(BDTables.people))|\(model.status?.rawValue ?? "")|\(model.showArchived)|\(model.search)|\(model.selectedId ?? "")"
    }

    var body: some View {
        ScrollingPage {
        VStack(spacing: PageSpacing.medium) {
            BDPageHeader(title: "Locations", sub: "Which places can we use? A practical scouting notebook.") {
                if model.canEdit {
                    Button { model.sheet = .add } label: { Label("Add Location", systemImage: "plus") }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("add-location")
                }
            }
            CenteredWrap(spacing: 10, lineSpacing: PageSpacing.small) {
                HStack(spacing: 4) {
                    Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                    TextField("Search locations", text: $model.search).textFieldStyle(.plain).frame(width: 190)
                        .accessibilityLabel("Search locations")
                }
                .padding(.horizontal, 8).padding(.vertical, 5)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 7, style: .continuous))
                Picker("Status", selection: $model.status) {
                    Text("All").tag(ProductionLocationStatus?.none)
                    ForEach(ProductionLocationStatus.allCases, id: \.self) { Text($0.rawValue).tag(ProductionLocationStatus?.some($0)) }
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize().accessibilityLabel("Status")
                Toggle("Show archived", isOn: $model.showArchived).toggleStyle(.checkbox)
            }
            .frame(maxWidth: .infinity, alignment: .center)
            content
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
            if let loc = model.selected {
                LocationInspector(model: model, loc: loc).inspectorColumnWidth(min: 340, ideal: 420, max: 560)
            }
        }
        .sheet(item: $model.sheet) { LocationSheetHost(model: model, sheet: $0) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("locations-tab")
    }

    @ViewBuilder private var content: some View {
        if !model.loaded {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let f = model.loadFailure, model.locations.isEmpty {
            BDBanner(tone: .err, content: { Text(f.message) }, actions: { Button("Try Again") { Task { await model.reload() } } })
        } else if model.locations.isEmpty {
            if model.isFiltered {
                Text("No locations match.").foregroundStyle(.secondary).padding(12)
            } else {
                ContentUnavailableView {
                    Label("No locations yet.", systemImage: "mappin.and.ellipse")
                } description: {
                    Text("Add a location you are considering for the film. Only the name is required.")
                } actions: {
                    if model.canEdit { Button("Add Location") { model.sheet = .add } }
                }
            }
        } else {
            CenteredWrap(itemWidth: 220) {
                ForEach(model.locations, id: \.id) { LocationCard(model: model, loc: $0) }
            }
            .frame(maxWidth: .infinity)
            .padding(.bottom, PageSpacing.medium)
        }
    }
}

private struct LocationCard: View {
    @Bindable var model: LocationsModel
    let loc: ProductionLocationDto
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Button { model.selectedId = loc.id } label: {
            VStack(alignment: .leading, spacing: 0) {
                AssetImageView(asset: loc.photos.first?.asset, maxPixel: 480, stand: "mappin.and.ellipse")
                    .frame(height: 84)
                VStack(alignment: .leading, spacing: 5) {
                    Text(loc.name).font(.callout.weight(.semibold)).lineLimit(2)
                    HStack(spacing: 6) {
                        BDChip(text: loc.status.rawValue, tone: loc.status.tone)
                        Text(BDText.plural(loc.sceneCount.asInt, "scene")).font(.caption).foregroundStyle(theme["muted"])
                        if loc.archived { BDChip(text: "Archived") }
                    }
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(loc.id == model.selectedId ? theme["accent"] : theme["line"], lineWidth: loc.id == model.selectedId ? 2 : 1))
            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .opacity(loc.archived ? 0.55 : 1)
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
        .contextMenu {
            Button("Open") { model.selectedId = loc.id }
            Button(loc.archived ? "Restore from Archive" : "Archive") { Task { await model.setArchived(loc, !loc.archived) } }.disabled(!model.canEdit)
            Divider()
            Button("Delete\u{2026}", role: .destructive) { model.sheet = .delete(loc) }.disabled(!model.canEdit)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(loc.name), \(loc.status.rawValue), \(BDText.plural(loc.sceneCount.asInt, "scene"))")
        .accessibilityIdentifier("location-card-\(loc.name)")
    }
}

// MARK: Inspector

struct LocationInspector: View {
    @Bindable var model: LocationsModel
    let loc: ProductionLocationDto
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    private var canEdit: Bool { model.canEdit }
    private var inSourceScenes: Int { loc.scenes.filter(\.inSource).count }

    var body: some View {
        BDInspectorFrame(typeLabel: "Location", title: loc.name, onClose: { model.selectedId = nil }) {
            VStack(alignment: .leading, spacing: 4) {
                Picker("Location status", selection: Binding(get: { loc.status }, set: { s in if canEdit { Task { await model.setStatus(loc, s) } } })) {
                    ForEach(ProductionLocationStatus.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented).labelsHidden().disabled(!canEdit)
                .accessibilityLabel("Location status")
                .accessibilityIdentifier("loc-status")
                Text("A status change never alters the screenplay.").font(.caption).foregroundStyle(theme["muted"])
            }
            if loc.status == .rejected, inSourceScenes > 0 {
                BDBanner(tone: .warn, content: { Text("Rejected, but still used in \(BDText.plural(inSourceScenes, "scene")).") }, actions: {
                    if canEdit { Button("Replace\u{2026}") { model.sheet = .replace(loc) }.controlSize(.small) }
                })
            }
            if let r = loc.replacement {
                (Text("Practical replacement: ").foregroundStyle(theme["muted"]) + Text(r.name).bold()).font(.callout)
            }
            photos
            BufferedField(id: "loc-name", label: "Name", value: loc.name, required: true, disabled: !canEdit) { v in Task { await model.update(loc.id, name: v) } }
            BufferedField(id: "loc-addr", label: "Address / area", value: loc.address, disabled: !canEdit) { v in Task { await model.update(loc.id, address: v) } }
            ContactField(id: "loc-contact", label: "Contact", value: loc.contact, hidden: loc.contactHidden, canEdit: canEdit) { v in Task { await model.update(loc.id, contact: v) } }
            Text("Safety").font(.callout.weight(.semibold))
            BufferedField(id: "loc-hospital", label: "Nearest hospital", value: loc.nearestHospital,
                          placeholder: "Name, address and phone of the nearest emergency room", disabled: !canEdit) { v in Task { await model.update(loc.id, nearestHospital: v) } }
            ContactField(id: "loc-emergency", label: "Emergency contact", value: loc.emergencyContact, hidden: loc.contactHidden, canEdit: canEdit) { v in Task { await model.update(loc.id, emergencyContact: v) } }
            AvailabilityEditor(owner: .location, ownerId: loc.id, windows: loc.availability, canEdit: canEdit, name: loc.name)
            Text("Practical notes").font(.callout.weight(.semibold))
            ForEach(noteFields, id: \.id) { f in
                BufferedField(id: "loc-note-\(f.id)", label: "\(f.label):", value: loc.notes[keyPath: f.key], placeholder: f.placeholder, disabled: !canEdit) { v in
                    var notes = loc.notes
                    notes[keyPath: f.key] = v.isEmpty ? nil : v
                    Task { await model.update(loc.id, notes: notes) }
                }
            }
            BDField(label: "Scenes") { SceneChips(scenes: loc.scenes, empty: "No scenes use this location yet. Tag it in the Breakdown (Location / Set).") }
        } footer: {
            if canEdit { Button("Delete") { model.sheet = .delete(loc) }.accessibilityIdentifier("location-delete") }
            Spacer()
            if canEdit { Button(loc.archived ? "Restore" : "Archive") { Task { await model.setArchived(loc, !loc.archived) } } }
            Button("Open Scenes") { if let s = loc.scenes.first { hub.openScene(s.sceneId) } }.disabled(loc.scenes.isEmpty)
        }
        .id(loc.id)
    }

    private var photos: some View {
        BDField(label: "Photos", hint: "The first photo is the thumbnail. Photos are stored in the project and work offline.") {
            VStack(alignment: .leading, spacing: 6) {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 96), spacing: 6)], spacing: 6) {
                    ForEach(Array(loc.photos.enumerated()), id: \.element.id) { i, p in
                        AssetImageView(asset: p.asset, maxPixel: 320, stand: "photo")
                            .frame(height: 72)
                            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                            .overlay(alignment: .topTrailing) {
                                if canEdit {
                                    HStack(spacing: 0) {
                                        Button { Task { await model.movePhoto(p.id, to: i - 1) } } label: { Image(systemName: "arrow.left") }
                                            .disabled(i == 0).accessibilityLabel("Move earlier")
                                        Button { Task { await model.movePhoto(p.id, to: i + 1) } } label: { Image(systemName: "arrow.right") }
                                            .disabled(i == loc.photos.count - 1).accessibilityLabel("Move later")
                                        Button { Task { await model.removePhoto(p.id) } } label: { Image(systemName: "trash") }
                                            .accessibilityLabel("Remove photo")
                                    }
                                    .buttonStyle(.borderless).controlSize(.mini).font(.caption2)
                                    .padding(2)
                                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 5, style: .continuous))
                                    .padding(2)
                                }
                            }
                            .accessibilityElement(children: .contain)
                            .accessibilityLabel("\(loc.name) photo \(i + 1)")
                    }
                }
                if canEdit {
                    Button {
                        let paths = ImagePicker.choose(multiple: true)
                        if !paths.isEmpty { Task { await model.addPhotos(loc.id, paths: paths) } }
                    } label: { Label("Add Photos", systemImage: "photo.badge.plus") }
                        .controlSize(.small)
                }
            }
        }
    }
}

// MARK: Sheets

private struct LocationSheetHost: View {
    @Bindable var model: LocationsModel
    let sheet: LocationSheet

    var body: some View {
        switch sheet {
        case .add: AddLocationSheet(model: model)
        case let .replace(l): ReplaceLocationSheet(model: model, loc: l)
        case let .delete(l):
            BDConfirmSheet(title: "Delete \(BDText.quoted(l.name))?", confirmLabel: "Delete", danger: true,
                           onConfirm: { Task { await model.delete(l) } }, onCancel: { model.sheet = nil }) {
                ReferencesNote(kind: .location, id: l.id)
            }
        }
    }
}

private struct AddLocationSheet: View {
    @Bindable var model: LocationsModel
    @State private var name = ""
    @State private var address = ""
    @State private var contact = ""
    @State private var hospital = ""
    @State private var status: ProductionLocationStatus = .idea
    @FocusState private var focused: Bool

    var body: some View {
        BDSheetFrame(title: "Add Location", sub: "Only the name is required. Confirming a location does not assign it to scenes.") {
            VStack(alignment: .leading, spacing: 10) {
                BDField(label: "Name") {
                    TextField("e.g. Old Railway Station", text: $name).textFieldStyle(.roundedBorder).focused($focused)
                        .onSubmit { submit() }
                        .accessibilityLabel("Location name").accessibilityIdentifier("nl-name")
                }
                BDField(label: "Address / area") {
                    TextField("Address / area", text: $address).textFieldStyle(.roundedBorder).accessibilityLabel("Address").accessibilityIdentifier("nl-addr")
                }
                BDField(label: "Contact") {
                    TextField("Contact", text: $contact).textFieldStyle(.roundedBorder).accessibilityLabel("Contact").accessibilityIdentifier("nl-contact")
                }
                BDField(label: "Nearest hospital") {
                    TextField("Nearest hospital", text: $hospital).textFieldStyle(.roundedBorder)
                        .onSubmit { submit() }
                        .accessibilityLabel("Nearest hospital").accessibilityIdentifier("nl-hospital")
                }
                BDField(label: "Status") {
                    Picker("Status", selection: $status) { ForEach(ProductionLocationStatus.allCases, id: \.self) { Text($0.rawValue).tag($0) } }
                        .labelsHidden().accessibilityLabel("Status")
                }
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Add Location") { submit() }
                .keyboardShortcut(.defaultAction)
                .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
                .accessibilityIdentifier("nl-submit")
        }
        .onAppear { focused = true }
    }

    private func submit() {
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        Task { await model.create(name: name, address: address, contact: contact, nearestHospital: hospital, status: status) }
    }
}

/// "Replace a rejected location" (mock 117).
private struct ReplaceLocationSheet: View {
    @Bindable var model: LocationsModel
    let loc: ProductionLocationDto
    @State private var options: [ProductionLocationDto] = []
    @State private var loaded = false
    @State private var target = ""
    @State private var scenes: Set<String>

    init(model: LocationsModel, loc: ProductionLocationDto) {
        self.model = model
        self.loc = loc
        _scenes = State(initialValue: Set(loc.scenes.filter(\.inSource).map(\.sceneId)))
    }

    var body: some View {
        let inSource = loc.scenes.filter(\.inSource)
        let chosen = target.isEmpty ? (options.first { $0.status == .confirmed }?.id ?? options.first?.id ?? "") : target
        let nums = inSource.compactMap(\.number).joined(separator: ", ")
        BDSheetFrame(title: "Replace a rejected location",
                     sub: "\(loc.name) is \(loc.status.rawValue). It is used in \(BDText.plural(inSource.count, "scene"))\(nums.isEmpty ? "" : " (\(nums))").") {
            VStack(alignment: .leading, spacing: 10) {
                if loaded, options.isEmpty {
                    BDBanner(.info, "Add the practical replacement as a location first (Idea, Shortlisted or Confirmed).")
                } else {
                    BDField(label: "Practical replacement") {
                        Picker("Practical replacement", selection: Binding(get: { chosen }, set: { target = $0 })) {
                            ForEach(options, id: \.id) { Text("\($0.name) (\($0.status.rawValue))").tag($0.id) }
                        }
                        .labelsHidden().accessibilityLabel("Practical replacement")
                    }
                }
                BDField(label: "Apply to scenes") {
                    VStack(alignment: .leading, spacing: 4) {
                        ForEach(inSource, id: \.sceneId) { s in
                            Toggle("Scene \(s.number ?? ""): \(s.heading)", isOn: Binding(
                                get: { scenes.contains(s.sceneId) },
                                set: { if $0 { scenes.insert(s.sceneId) } else { scenes.remove(s.sceneId) } }))
                                .toggleStyle(.checkbox)
                        }
                    }
                }
                Text("This updates production associations only. The screenplay text (which still says \(loc.name.uppercased())) is not changed; edit the script yourself if you want that.")
                    .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Apply Replacement") { Task { await model.replace(loc, with: chosen, sceneIds: Array(scenes)) } }
                .keyboardShortcut(.defaultAction)
                .disabled(chosen.isEmpty || scenes.isEmpty)
        }
        .task {
            options = await model.replacementCandidates(for: loc)
            loaded = true
        }
    }
}

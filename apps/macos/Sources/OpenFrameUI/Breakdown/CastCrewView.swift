// The Cast & Crew tab: "Who is involved and what role do they have? A small-team directory with no
// payroll and no contracts." (FSD 30, 100; UX 3.25). The Character is the story object; the cast
// member is the real person attached to it. Two lists (Cast, Crew), an inspector for the selected
// person, and sheets to assign a character, add crew and remove people.

import SwiftUI
import OpenFrameCore

struct CastCrewView: View {
    @Bindable var model: PeopleModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    private var reloadKey: String {
        "\(app.changes.key(BDTables.people))|\(model.showArchived)|\(model.selectedCast ?? "")|\(model.selectedCrew ?? "")"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            BDPageHeader(title: "Cast & Crew", sub: "Who is involved and what role do they have? A small-team directory with no payroll and no contracts.") {
                if model.canEdit {
                    Button {
                        model.sheet = model.view == .cast ? .assign(member: nil, character: nil) : .addCrew
                    } label: { Label("Add Person", systemImage: "plus") }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("add-person")
                }
            }
            HStack(spacing: 10) {
                Picker("Directory", selection: $model.view) {
                    ForEach(PeopleView.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize().accessibilityLabel("Directory")
                .accessibilityIdentifier("people-view")
                HStack(spacing: 4) {
                    Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                    TextField("Search", text: $model.search).textFieldStyle(.plain).frame(width: 190).accessibilityLabel("Search")
                }
                .padding(.horizontal, 8).padding(.vertical, 5)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 7, style: .continuous))
                Spacer()
                Toggle("Show archived", isOn: $model.showArchived).toggleStyle(.checkbox)
            }
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let f = model.loadFailure, model.directory == nil {
                BDBanner(tone: .err, content: { Text(f.message) }, actions: { Button("Try Again") { Task { await model.reload() } } })
            } else if model.view == .cast {
                castList
            } else {
                crewListView
            }
        }
        .padding(18)
        .task(id: reloadKey) { await model.reload() }
        .inspector(isPresented: Binding(get: { model.view == .cast ? model.castMember != nil : model.crewMember != nil },
                                        set: { if !$0 { model.selectedCast = nil; model.selectedCrew = nil } })) {
            if model.view == .cast, let m = model.castMember {
                CastInspector(model: model, member: m).inspectorColumnWidth(min: 340, ideal: 400, max: 540)
            } else if model.view == .crew, let c = model.crewMember {
                CrewInspector(model: model, member: c).inspectorColumnWidth(min: 340, ideal: 400, max: 540)
            }
        }
        .sheet(item: $model.sheet) { PeopleSheetHost(model: model, sheet: $0) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("cast-crew-tab")
    }

    // MARK: Cast

    @ViewBuilder private var castList: some View {
        let members = model.members
        let unassigned = model.unassigned
        if members.isEmpty && unassigned.isEmpty {
            ContentUnavailableView {
                Label("No cast yet.", systemImage: "person.2")
            } description: {
                Text("Characters from the Production Source appear here automatically. Add the actor who plays each one, with a contact and availability notes.")
            } actions: {
                if model.canEdit { Button("Add Person") { model.sheet = .assign(member: nil, character: nil) } }
            }
        } else {
            List(selection: $model.selectedCast) {
                Section {
                    ForEach(members, id: \.id) { m in
                        HStack(spacing: 10) {
                            Avatar(asset: m.photo).frame(width: 30, height: 30)
                            VStack(alignment: .leading, spacing: 1) {
                                HStack(spacing: 6) {
                                    Text(m.personName).fontWeight(.semibold)
                                    if !m.isPrimary { BDChip(text: "Alternate", tone: .purple) }
                                    if m.archived { BDChip(text: "Archived") }
                                }
                                Text(model.summary(of: m)).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                            }
                            Spacer()
                            Text(m.characterName ?? "None").foregroundStyle(theme["ink2"]).frame(width: 140, alignment: .leading)
                            Text(m.contactHidden ? "Hidden" : m.contact ?? "").foregroundStyle(theme["muted"]).lineLimit(1).frame(width: 140, alignment: .leading)
                            Text("\(m.scenes.filter(\.inSource).count)").monospacedDigit().frame(width: 40, alignment: .trailing)
                        }
                        .opacity(m.archived ? 0.55 : 1)
                        .tag(m.id)
                        .contextMenu {
                            Button("Open") { model.selectedCast = m.id }
                            Button("Assign Character\u{2026}") { model.sheet = .assign(member: m, character: m.characterName) }.disabled(!model.canEdit)
                            Button(m.archived ? "Restore from Archive" : "Archive") {
                                Task { await model.setArchived(kind: .cast, id: m.id, archived: !m.archived, name: m.personName) }
                            }.disabled(!model.canEdit)
                            Divider()
                            Button("Delete\u{2026}", role: .destructive) { model.sheet = .delete(kind: .cast, id: m.id, name: m.personName) }.disabled(!model.canEdit)
                        }
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("cast-row-\(m.personName)")
                    }
                } header: {
                    HStack {
                        Text("Person"); Spacer()
                        Text("Character").frame(width: 140, alignment: .leading)
                        Text("Contact").frame(width: 140, alignment: .leading)
                        Text("Scenes").frame(width: 40, alignment: .trailing)
                    }
                    .font(.caption.weight(.semibold))
                }
                if !unassigned.isEmpty {
                    Section("Needs casting") {
                        ForEach(unassigned, id: \.name) { c in
                            HStack(spacing: 10) {
                                Circle().fill(theme["surface2"]).frame(width: 30, height: 30).accessibilityHidden(true)
                                Text("Unassigned").fontWeight(.semibold).foregroundStyle(theme["muted"])
                                Spacer()
                                Text(c.name)
                                BDChip(text: "Needs casting", tone: .yellow)
                                if model.canEdit { Button("Assign") { model.sheet = .assign(member: nil, character: c.name) }.controlSize(.small).accessibilityLabel("Assign \(c.name)") }
                                Text("\(c.scenes.filter(\.inSource).count)").monospacedDigit().frame(width: 40, alignment: .trailing)
                            }
                            .accessibilityElement(children: .contain)
                        }
                    }
                }
            }
            .listStyle(.inset)
            .onDeleteCommand {
                if let m = model.castMember, model.canEdit { model.sheet = .delete(kind: .cast, id: m.id, name: m.personName) }
            }
            .accessibilityLabel("Cast")
            .accessibilityIdentifier("cast-list")
        }
    }

    // MARK: Crew

    @ViewBuilder private var crewListView: some View {
        let list = model.crewList
        if list.isEmpty {
            ContentUnavailableView {
                Label("No crew yet.", systemImage: "person.crop.rectangle.stack")
            } description: {
                Text("Add the people working on the film with their role and department. This is a production directory, not HR software.")
            } actions: {
                if model.canEdit { Button("Add Person") { model.sheet = .addCrew } }
            }
        } else {
            VStack(alignment: .leading, spacing: 8) {
                List(selection: $model.selectedCrew) {
                    ForEach(list, id: \.id) { c in
                        HStack(spacing: 10) {
                            HStack(spacing: 6) {
                                Text(c.personName).fontWeight(.semibold)
                                if c.archived { BDChip(text: "Archived") }
                            }
                            Spacer()
                            Text(c.role).frame(width: 170, alignment: .leading)
                            Text(c.department ?? "").foregroundStyle(theme["ink2"]).frame(width: 110, alignment: .leading)
                            Text(c.contactHidden ? "Hidden" : c.contact ?? "").foregroundStyle(theme["muted"]).lineLimit(1).frame(width: 150, alignment: .leading)
                        }
                        .opacity(c.archived ? 0.55 : 1)
                        .tag(c.id)
                        .contextMenu {
                            Button("Open") { model.selectedCrew = c.id }
                            Button(c.archived ? "Restore from Archive" : "Archive") {
                                Task { await model.setArchived(kind: .crew, id: c.id, archived: !c.archived, name: c.personName) }
                            }.disabled(!model.canEdit)
                            Divider()
                            Button("Delete\u{2026}", role: .destructive) { model.sheet = .delete(kind: .crew, id: c.id, name: c.personName) }.disabled(!model.canEdit)
                        }
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("crew-row-\(c.personName)")
                    }
                }
                .listStyle(.inset)
                .onDeleteCommand {
                    if let c = model.crewMember, model.canEdit { model.sheet = .delete(kind: .crew, id: c.id, name: c.personName) }
                }
                .accessibilityLabel("Crew")
                .accessibilityIdentifier("crew-list")
                Text("Departments: \(Departments.all.joined(separator: " \u{00B7} ")). This is a production directory, not HR software.")
                    .font(.caption).foregroundStyle(theme["muted"])
            }
        }
    }
}

struct Avatar: View {
    let asset: AssetInfo?
    var body: some View {
        AssetImageView(asset: asset, maxPixel: 160, stand: "person.fill").clipShape(Circle())
    }
}

// MARK: Inspectors

struct CastInspector: View {
    @Bindable var model: PeopleModel
    let member: ProductionCastMemberDto
    private var canEdit: Bool { model.canEdit }

    var body: some View {
        BDInspectorFrame(typeLabel: "Cast member", title: member.personName, onClose: { model.selectedCast = nil }) {
            HStack(spacing: 10) {
                Avatar(asset: member.photo).frame(width: 56, height: 56)
                if canEdit {
                    Button(member.photo == nil ? "Add Photo" : "Change Photo") {
                        if let p = ImagePicker.choose(multiple: false).first { Task { await model.setCastPhoto(member.id, path: p) } }
                    }.controlSize(.small)
                    if member.photo != nil {
                        Button { Task { await model.setCastPhoto(member.id, path: nil) } } label: { Image(systemName: "xmark.circle") }
                            .buttonStyle(.borderless).accessibilityLabel("Remove photo")
                    }
                }
            }
            BDField(label: "Character") {
                HStack {
                    Text(member.characterName ?? "None").fontWeight(.semibold)
                    BDChip(text: member.isPrimary ? "Primary" : "Alternate", tone: member.isPrimary ? .green : .purple)
                    Spacer()
                    if canEdit { Button("Assign Character\u{2026}") { model.sheet = .assign(member: member, character: member.characterName) }.controlSize(.small) }
                }
            }
            BufferedField(id: "cm-name", label: "Person name", value: member.personName, required: true, disabled: !canEdit) { v in Task { await model.updateCast(member.id, personName: v) } }
            ContactField(id: "cm-contact", label: "Contact", value: member.contact, hidden: member.contactHidden, canEdit: canEdit) { v in Task { await model.updateCast(member.id, contact: v) } }
            BufferedField(id: "cm-avail", label: "Availability notes", value: member.availabilityNotes, multiline: true,
                          placeholder: "e.g. Free Jun 9 to 20, weekends only", disabled: !canEdit, hint: "Anything the dates below cannot say.") { v in
                Task { await model.updateCast(member.id, availabilityNotes: v) }
            }
            AvailabilityEditor(owner: .castMember, ownerId: member.id, windows: member.availability, canEdit: canEdit, name: member.personName)
            BufferedField(id: "cm-notes", label: "Notes", value: member.notes, multiline: true, disabled: !canEdit) { v in Task { await model.updateCast(member.id, notes: v) } }
            BDField(label: "Scenes") { SceneChips(scenes: member.scenes, empty: "No scenes in the Production Source mention this character yet.") }
        } footer: {
            if canEdit { Button("Delete") { model.sheet = .delete(kind: .cast, id: member.id, name: member.personName) } }
            Spacer()
            if canEdit {
                Button(member.archived ? "Restore" : "Archive") {
                    Task { await model.setArchived(kind: .cast, id: member.id, archived: !member.archived, name: member.personName) }
                }
            }
        }
        .id(member.id)
    }
}

struct CrewInspector: View {
    @Bindable var model: PeopleModel
    let member: ProductionCrewMemberDto
    private var canEdit: Bool { model.canEdit }

    var body: some View {
        let depts = Array(Set(Departments.all + (member.department.map { [$0] } ?? []))).sorted { a, b in
            (Departments.all.firstIndex(of: a) ?? 99, a) < (Departments.all.firstIndex(of: b) ?? 99, b)
        }
        BDInspectorFrame(typeLabel: "Crew member", title: member.personName, onClose: { model.selectedCrew = nil }) {
            BufferedField(id: "cr-name", label: "Person name", value: member.personName, required: true, disabled: !canEdit) { v in Task { await model.updateCrew(member.id, personName: v) } }
            BufferedField(id: "cr-role", label: "Role", value: member.role, required: true, disabled: !canEdit) { v in Task { await model.updateCrew(member.id, role: v) } }
            BDField(label: "Department") {
                Picker("Department", selection: Binding(get: { member.department ?? "" }, set: { d in if canEdit { Task { await model.updateCrew(member.id, department: d) } } })) {
                    Text("No department").tag("")
                    ForEach(depts, id: \.self) { Text($0).tag($0) }
                }
                .labelsHidden().disabled(!canEdit).accessibilityLabel("Department")
            }
            ContactField(id: "cr-contact", label: "Contact", value: member.contact, hidden: member.contactHidden, canEdit: canEdit) { v in Task { await model.updateCrew(member.id, contact: v) } }
            BufferedField(id: "cr-avail", label: "Availability notes", value: member.availabilityNotes, multiline: true,
                          placeholder: "e.g. weekends only", disabled: !canEdit, hint: "Anything the dates below cannot say.") { v in
                Task { await model.updateCrew(member.id, availabilityNotes: v) }
            }
            AvailabilityEditor(owner: .crewMember, ownerId: member.id, windows: member.availability, canEdit: canEdit, name: member.personName)
            BufferedField(id: "cr-notes", label: "Notes", value: member.notes, multiline: true, disabled: !canEdit) { v in Task { await model.updateCrew(member.id, notes: v) } }
        } footer: {
            if canEdit { Button("Delete") { model.sheet = .delete(kind: .crew, id: member.id, name: member.personName) } }
            Spacer()
            if canEdit {
                Button(member.archived ? "Restore" : "Archive") {
                    Task { await model.setArchived(kind: .crew, id: member.id, archived: !member.archived, name: member.personName) }
                }
            }
        }
        .id(member.id)
    }
}

// MARK: Sheets

private struct PeopleSheetHost: View {
    @Bindable var model: PeopleModel
    let sheet: PeopleSheet

    var body: some View {
        switch sheet {
        case let .assign(member, character): AssignSheet(model: model, member: member, initialCharacter: character)
        case .addCrew: AddCrewSheet(model: model)
        case let .delete(kind, id, name):
            BDConfirmSheet(title: "Remove \(name) from the \(kind == .crew ? "crew" : "cast")?", confirmLabel: "Remove", danger: true,
                           onConfirm: { Task { await model.delete(kind: kind, id: id, name: name) } }, onCancel: { model.sheet = nil }) {
                VStack(alignment: .leading, spacing: 8) {
                    ReferencesNote(kind: kind == .crew ? .crewMember : .castMember, id: id)
                    Text("Archiving is the better choice if they may return.").font(.callout).foregroundStyle(.secondary)
                }
            }
        }
    }
}

/// "Assign character" (mock 119).
private struct AssignSheet: View {
    @Bindable var model: PeopleModel
    let member: ProductionCastMemberDto?
    @State private var person: String
    @State private var character: String?
    @State private var newCharacter = ""
    @State private var role: String?
    @Environment(\.ofTheme) private var theme
    @FocusState private var focused: Bool

    init(model: PeopleModel, member: ProductionCastMemberDto?, initialCharacter: String?) {
        self.model = model
        self.member = member
        _person = State(initialValue: member?.personName ?? "")
        _character = State(initialValue: initialCharacter)
    }

    private var characters: [ProductionCastCharacterDto] { model.directory?.characters ?? [] }
    private var chosen: String? {
        let n = newCharacter.trimmingCharacters(in: .whitespaces)
        return n.isEmpty ? character : n
    }

    var body: some View {
        let current = characters.first { $0.name == chosen }
        let hasOtherPrimary = current?.primaryCastId != nil && current?.primaryCastId != member?.id
        let effectiveRole = role ?? (hasOtherPrimary ? "alternate" : "primary")
        let byId = Dictionary(uniqueKeysWithValues: (model.directory?.members ?? []).map { ($0.id, $0.personName) })
        BDSheetFrame(title: member.map { "Assign a character to \($0.personName)" } ?? "Add Person", sub: "Link the real person to the character they play.", width: 540) {
            VStack(alignment: .leading, spacing: 10) {
                BDField(label: "Person") {
                    TextField("Actor's name", text: $person).textFieldStyle(.roundedBorder).focused($focused)
                        .disabled(member != nil)
                        .accessibilityLabel("Person").accessibilityIdentifier("as-person")
                }
                BDField(label: "Choose a character") {
                    if characters.isEmpty {
                        Text("No characters found in the Production Source yet. Create one below.").font(.callout).foregroundStyle(.secondary)
                    } else {
                        ScrollView {
                            VStack(spacing: 0) {
                                ForEach(characters, id: \.name) { c in
                                    let on = newCharacter.trimmingCharacters(in: .whitespaces).isEmpty && character == c.name
                                    Button { character = c.name; newCharacter = ""; role = nil } label: {
                                        HStack {
                                            Image(systemName: on ? "largecircle.fill.circle" : "circle").foregroundStyle(on ? theme["accent"] : theme["muted"]).accessibilityHidden(true)
                                            Text(c.name).fontWeight(.semibold)
                                            Spacer()
                                            let n = c.scenes.filter(\.inSource).count
                                            Text("\(BDText.plural(n, "scene")) \u{00B7} \(c.primaryCastId.map { "played by \(byId[$0] ?? "someone")" } ?? "no actor")")
                                                .font(.caption).foregroundStyle(theme["muted"])
                                        }
                                        .padding(8).contentShape(Rectangle())
                                    }
                                    .buttonStyle(.plain)
                                    .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
                                    .accessibilityIdentifier("character-\(c.name)")
                                    Divider()
                                }
                            }
                        }
                        .frame(maxHeight: 200)
                        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                    }
                }
                BDField(label: "Create a new character", hint: "For a character that isn't in the script yet.") {
                    TextField("e.g. THE STATION MASTER", text: $newCharacter).textFieldStyle(.roundedBorder)
                        .onChange(of: newCharacter) { _, _ in role = nil }
                        .accessibilityLabel("Create a new character").accessibilityIdentifier("as-new")
                }
                BDField(label: "Casting") {
                    Picker("Primary or alternate", selection: Binding(get: { effectiveRole }, set: { role = $0 })) {
                        Text("Primary actor").tag("primary")
                        Text("Alternate / double").tag("alternate")
                    }
                    .pickerStyle(.segmented).labelsHidden().accessibilityLabel("Primary or alternate")
                }
                if effectiveRole == "primary", hasOtherPrimary {
                    BDBanner(.info, "\(byId[current?.primaryCastId ?? ""] ?? "The current actor") becomes an alternate for \(chosen ?? "").")
                }
                Text("One character has one primary actor by default. Alternates or double-casting can be added when needed.").font(.caption).foregroundStyle(.secondary)
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Assign") { submit(role: effectiveRole, current: current) }
                .keyboardShortcut(.defaultAction)
                .disabled(chosen == nil || person.trimmingCharacters(in: .whitespaces).isEmpty)
                .accessibilityIdentifier("assign-submit")
        }
        .onAppear { focused = member == nil }
    }

    private func submit(role: String, current: ProductionCastCharacterDto?) {
        guard let chosen else { return }
        let storyId = current?.storyCharacterId
        Task {
            if let member {
                await model.assign(member, characterName: chosen, characterId: storyId, primary: role == "primary")
            } else {
                await model.createCast(person: person, characterName: chosen, characterId: storyId, primary: role == "primary")
            }
        }
    }
}

private struct AddCrewSheet: View {
    @Bindable var model: PeopleModel
    @State private var person = ""
    @State private var role = ""
    @State private var department = ""
    @State private var contact = ""
    @State private var notes = ""
    @FocusState private var focused: Bool

    var body: some View {
        BDSheetFrame(title: "Add Person", sub: "A crew member with their role. No payroll, no contracts.") {
            VStack(alignment: .leading, spacing: 10) {
                BDField(label: "Person name") {
                    TextField("Person name", text: $person).textFieldStyle(.roundedBorder).focused($focused)
                        .accessibilityLabel("Person name").accessibilityIdentifier("nc-name")
                }
                BDField(label: "Role") {
                    TextField("e.g. Director of Photography", text: $role).textFieldStyle(.roundedBorder).onSubmit { submit() }
                        .accessibilityLabel("Role").accessibilityIdentifier("nc-role")
                }
                BDField(label: "Department") {
                    Picker("Department", selection: $department) {
                        Text("No department").tag("")
                        ForEach(Departments.all, id: \.self) { Text($0).tag($0) }
                    }
                    .labelsHidden().accessibilityLabel("Department")
                }
                BDField(label: "Contact") {
                    TextField("Contact", text: $contact).textFieldStyle(.roundedBorder).accessibilityLabel("Contact")
                }
                BDField(label: "Notes") {
                    TextField("Notes", text: $notes, axis: .vertical).lineLimit(2...4).textFieldStyle(.roundedBorder).accessibilityLabel("Notes")
                }
            }
        } footer: {
            Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Add to Crew") { submit() }
                .keyboardShortcut(.defaultAction)
                .disabled(person.trimmingCharacters(in: .whitespaces).isEmpty || role.trimmingCharacters(in: .whitespaces).isEmpty)
                .accessibilityIdentifier("nc-submit")
        }
        .onAppear { focused = true }
    }

    private func submit() {
        guard !person.trimmingCharacters(in: .whitespaces).isEmpty, !role.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        Task { await model.createCrew(person: person, role: role, department: department.isEmpty ? nil : department, contact: contact, notes: notes) }
    }
}

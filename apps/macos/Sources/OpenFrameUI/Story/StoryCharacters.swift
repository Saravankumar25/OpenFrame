// Characters: a lightweight directory whose most useful part is the list of screenplay scenes
// each character appears in (derived by the core from character cues), plus typed relationships
// and optional reference links to Scene Cards. Mirrors workspaces/story/Characters.tsx.

import AppKit
import SwiftUI
import UniformTypeIdentifiers
import OpenFrameCore

enum CharacterSheet: Identifiable {
    case new
    case edit(StoryCharacterDto)
    case addRelationship(from: String)
    case delete(StoryCharacterDto, StoryCharacterUsage)

    var id: String {
        switch self {
        case .new: "new"
        case let .edit(c): "edit-\(c.id)"
        case let .addRelationship(f): "rel-\(f)"
        case let .delete(c, _): "delete-\(c.id)"
        }
    }
}

@MainActor
@Observable
final class CharactersModel {
    let story: StoryModel
    var showArchived = false
    var selectedId: String?
    var mapOpen = false
    var sheet: CharacterSheet?
    private(set) var list: [StoryCharacterDto] = []
    private(set) var loaded = false
    private(set) var detail: StoryCharacterDetail?
    private(set) var relationships: [StoryRelationshipDto] = []

    init(story: StoryModel) { self.story = story }

    var current: String? {
        if let s = selectedId, list.contains(where: { $0.id == s }) { return s }
        return list.first?.id
    }

    var activeCount: Int { list.filter { !$0.archived }.count }

    func reload() async {
        guard let core = story.app.core else { return }
        do {
            list = try await core.storyCharacters(StoryCharacterListArgs(episodeId: story.episodeId, includeArchived: showArchived))
            relationships = try await core.storyRelationships(StoryCharacterListArgs(includeArchived: false))
            if let id = current {
                detail = try await core.storyCharacter(StoryCharacterArgs(id: id, episodeId: story.episodeId))
            } else {
                detail = nil
            }
            loaded = true
        } catch {
            loaded = true
            story.fail("Could not load the characters", error)
        }
    }

    /// Characters whose name matches (the "possible match" warning of the New Character sheet).
    func match(for name: String) -> StoryCharacterDto? {
        let n = name.trimmingCharacters(in: .whitespaces).lowercased()
        guard !n.isEmpty else { return nil }
        return list.first { $0.name.trimmingCharacters(in: .whitespaces).lowercased() == n }
    }

    @discardableResult
    func create(name: String, role: String, description: String, imagePath: String?) async -> String? {
        guard let core = story.app.core else { return nil }
        let n = name.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            let r = try await core.storyCreateCharacter(StoryCreateCharacterArgs(
                name: n, roleLabel: role.isEmpty ? nil : role, description: description.isEmpty ? nil : description,
                episodeId: nil, imagePath: imagePath))
            story.say("Added character \(n)", undoable: true)
            selectedId = r.id
            return r.id
        } catch { story.fail("Could not add the character", error); return nil }
    }

    @discardableResult
    func update(_ c: StoryCharacterDto, name: String, role: String, description: String) async -> Bool {
        guard let core = story.app.core else { return false }
        do {
            _ = try await core.storyUpdateCharacter(StoryUpdateCharacterArgs(id: c.id, name: name, roleLabel: role, description: description))
            return true
        } catch { story.fail("Could not save the character", error); return false }
    }

    func saveNotes(_ id: String, _ notes: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storyUpdateCharacter(StoryUpdateCharacterArgs(id: id, notes: notes)) }
        catch { story.fail("Could not save the notes", error) }
    }

    func setArchived(_ c: StoryCharacterDto, _ archived: Bool) async {
        guard let core = story.app.core else { return }
        do {
            _ = try await core.storySetCharacterArchived(StoryArchiveArgs(id: c.id, archived: archived))
            story.say("\(archived ? "Archived" : "Unarchived") \(c.name)", undoable: true)
        } catch { story.fail("Could not change the character", error) }
    }

    func setImage(_ id: String, path: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storySetCharacterImage(StoryImageArgs(id: id, path: path)) }
        catch { story.fail("Could not set the image", error) }
    }

    func clearImage(_ id: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storyClearCharacterImage(StoryIdArgs(id: id)) }
        catch { story.fail("Could not remove the image", error) }
    }

    /// Delete asks the core where the character is used. In use: archive instead is offered.
    func askDelete(_ c: StoryCharacterDto) async {
        guard let core = story.app.core else { return }
        do { sheet = .delete(c, try await core.storyCharacterUsage(StoryIdArgs(id: c.id))) }
        catch { story.fail("Could not check where the character is used", error) }
    }

    @discardableResult
    func delete(_ c: StoryCharacterDto) async -> Bool {
        guard let core = story.app.core else { return false }
        do {
            _ = try await core.storyDeleteCharacter(StoryIdArgs(id: c.id))
            story.say("Deleted character \(c.name)", undoable: true)
            if selectedId == c.id { selectedId = nil }
            return true
        } catch { story.fail("Could not delete the character", error); return false }
    }

    @discardableResult
    func addRelationship(from: String, to: String, type: String, note: String) async -> Bool {
        guard let core = story.app.core else { return false }
        do {
            _ = try await core.storyCreateRelationship(StoryCreateRelationshipArgs(
                fromCharacterId: from, toCharacterId: to, relationshipType: type, note: note.isEmpty ? nil : note))
            story.say("Added relationship", undoable: true)
            return true
        } catch { story.fail("Could not add the relationship", error); return false }
    }

    func removeRelationship(_ id: String) async {
        guard let core = story.app.core else { return }
        do {
            _ = try await core.storyDeleteRelationship(StoryIdArgs(id: id))
            story.say("Removed relationship", undoable: true)
        } catch { story.fail("Could not remove the relationship", error) }
    }

    func link(_ characterId: String, card: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storyLinkCharacterCard(StoryCardLinkArgs(characterId: characterId, cardId: card)) }
        catch { story.fail("Could not link the card", error) }
    }

    func unlink(_ characterId: String, card: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storyUnlinkCharacterCard(StoryCardLinkArgs(characterId: characterId, cardId: card)) }
        catch { story.fail("Could not unlink the card", error) }
    }

    /// Cards that can still be linked to the character: every card of the board, not yet linked.
    func linkableCards(for detail: StoryCharacterDetail) -> [(id: String, label: String)] {
        guard let board = story.board else { return [] }
        let linked = Set(detail.linkedCards.map(\.cardId))
        return StoryLogic.allCards(board).filter { !linked.contains($0.id) }.map {
            ($0.id, String(($0.shortDescription.isEmpty ? ($0.sceneHeading ?? "Blank scene card") : $0.shortDescription).prefix(80)))
        }
    }
}

let characterRelationSuggestions = [
    "father of", "mother of", "son of", "daughter of", "sibling of", "partner of", "friend of",
    "colleague of", "mentor of", "rival of", "enemy of",
]

// MARK: Views

struct CharactersView: View {
    @Bindable var story: StoryModel
    @State private var m: CharactersModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    init(story: StoryModel) {
        self.story = story
        _m = State(initialValue: CharactersModel(story: story))
    }

    var body: some View {
        HStack(alignment: .top, spacing: 14) {
            if m.mapOpen {
                RelationshipMapView(m: m)
            } else {
                list
                detailPane
            }
        }
        .padding(16)
        .task(id: app.changes.key(StoryTables.characters) &+ story.viewRevision &+ (m.showArchived ? 1 : 0) &+ (story.episodeId?.hashValue ?? 0) &+ (m.selectedId?.hashValue ?? 0)) {
            await m.reload()
        }
        .sheet(item: $m.sheet) { sheet in
            switch sheet {
            case .new: NewCharacterSheet(m: m)
            case let .edit(c): EditCharacterSheet(m: m, c: c)
            case let .addRelationship(from): AddRelationshipSheet(m: m, from: from)
            case let .delete(c, usage): DeleteCharacterSheet(m: m, c: c, usage: usage)
            }
        }
        .accessibilityIdentifier("story-characters")
    }

    private var list: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Characters").fontWeight(.semibold)
                Text("\(m.activeCount)").font(.caption).padding(.horizontal, 6).background(.quaternary, in: Capsule())
                Spacer()
                Button { m.sheet = .new } label: { Label("Add Character", systemImage: "plus") }
                    .buttonStyle(.borderedProminent).controlSize(.small)
                    .accessibilityIdentifier("story-add-character")
            }
            .padding(10)
            Divider()
            if m.list.isEmpty && m.loaded {
                ContentUnavailableView {
                    Label("No characters yet.", systemImage: "person.2")
                } description: {
                    Text("A lightweight directory. Its most useful feature is the list of scenes each character appears in.")
                }
            } else {
                List(selection: Binding(get: { m.current }, set: { m.selectedId = $0 })) {
                    ForEach(m.list, id: \.id) { c in
                        HStack(spacing: 10) {
                            AvatarView(name: c.name, image: c.image, size: 34)
                            VStack(alignment: .leading, spacing: 1) {
                                Text(c.name).fontWeight(.semibold).textCase(.uppercase).lineLimit(1)
                                Text(c.roleLabel ?? c.description ?? "").font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                            }
                            Spacer(minLength: 4)
                            if c.archived { Text("Archived").font(.caption2).padding(.horizontal, 6).background(.quaternary, in: Capsule()) }
                            Text(StoryLogic.plural(Int(c.sceneCount), "scene")).font(.caption)
                                .padding(.horizontal, 6).padding(.vertical, 2)
                                .background(theme["blue-soft"], in: Capsule()).foregroundStyle(theme["blue-t"])
                        }
                        .tag(c.id)
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("story-character-\(c.id)")
                    }
                }
                .listStyle(.inset)
            }
            Divider()
            Toggle("Show archived characters", isOn: $m.showArchived).toggleStyle(.checkbox).padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(width: 360)
        .frame(maxHeight: .infinity)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
    }

    private var detailPane: some View {
        Group {
            if let d = m.detail, d.character.id == m.current {
                CharacterDetailView(m: m, d: d)
            } else if m.current == nil && m.loaded {
                ContentUnavailableView("Create your first character.", systemImage: "person.crop.circle.badge.plus",
                                       description: Text("Name is all you need. Role, image and notes are optional."))
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Character details")
    }
}

struct AvatarView: View {
    let name: String
    let image: AssetInfo?
    var size: CGFloat = 34
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            if let path = image?.path, image?.available == true, let ns = NSImage(contentsOfFile: path) {
                Image(nsImage: ns).resizable().scaledToFill()
            } else {
                Text(initials).font(.system(size: size * 0.38, weight: .bold)).foregroundStyle(theme["accent-ink"])
                    .frame(maxWidth: .infinity, maxHeight: .infinity).background(theme["accent"])
            }
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
        .accessibilityHidden(true)
    }

    private var initials: String {
        let s = name.split(whereSeparator: \.isWhitespace).prefix(2).compactMap { $0.first.map { String($0).uppercased() } }.joined()
        return s.isEmpty ? "?" : s
    }
}

private struct CharacterDetailView: View {
    @Bindable var m: CharactersModel
    let d: StoryCharacterDetail
    @State private var picking = false
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let c = d.character
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                HStack(alignment: .top, spacing: 12) {
                    AvatarView(name: c.name, image: c.image, size: 56)
                    VStack(alignment: .leading, spacing: 3) {
                        Text(c.name).font(.title2.weight(.bold)).textCase(.uppercase).accessibilityAddTraits(.isHeader)
                        Text([c.roleLabel, c.description].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{00B7} ").nilIfEmpty ?? "No role yet")
                            .foregroundStyle(theme["muted"])
                            .accessibilityIdentifier("story-character-role")
                        if c.archived { Text("Archived").font(.caption2).padding(.horizontal, 6).background(.quaternary, in: Capsule()) }
                    }
                    Spacer()
                    Button("Edit") { m.sheet = .edit(c) }.accessibilityIdentifier("story-character-edit")
                    Button { m.mapOpen = true } label: { Label("Open Relationship Map", systemImage: "point.3.connected.trianglepath.dotted") }
                    Menu {
                        Button(c.image == nil ? "Add image\u{2026}" : "Change image\u{2026}") { picking = true }
                        Button("Remove image") { Task { await m.clearImage(c.id) } }.disabled(c.image == nil)
                        Button(c.archived ? "Unarchive" : "Archive") { Task { await m.setArchived(c, !c.archived) } }
                        Divider()
                        Button("Delete\u{2026}", role: .destructive) { Task { await m.askDelete(c) } }
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                    .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                    .accessibilityLabel("More actions for \(c.name)")
                    .accessibilityIdentifier("story-character-more")
                }

                section("Scenes this character appears in") {
                    if d.scenes.isEmpty {
                        Text("No screenplay scenes mention \(c.name) yet. Scenes appear here when the current draft has dialogue cues with this name.")
                            .font(.callout).foregroundStyle(theme["muted"])
                    } else {
                        FlowLayout(spacing: 6) {
                            ForEach(d.scenes, id: \.sceneId) { s in
                                Button("Sc \(Int(s.number))") {
                                    NotificationCenter.default.post(name: .openFrameShowScreenplayTarget, object: nil, userInfo: ["sceneId": s.sceneId])
                                    NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.screenplay.rawValue)
                                }
                                .controlSize(.small).help(s.heading)
                                .accessibilityLabel("Scene \(Int(s.number)): \(s.heading)")
                            }
                        }
                    }
                }

                HStack {
                    Text("Relationships").font(.headline)
                    Spacer()
                    Button { m.sheet = .addRelationship(from: c.id) } label: { Label("Add Relationship", systemImage: "plus") }
                        .controlSize(.small).disabled(m.list.count < 2)
                }
                RelationshipList(m: m, rels: d.relationships)

                linkedCards(c)

                VStack(alignment: .leading, spacing: 4) {
                    Text("Notes").font(.callout.weight(.medium))
                    CharacterNotes(m: m, id: c.id, value: c.notes ?? "")
                }
            }
            .padding(16)
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.image]) { result in
            if case let .success(url) = result {
                let scoped = url.startAccessingSecurityScopedResource()
                Task {
                    await m.setImage(c.id, path: url.path)
                    if scoped { url.stopAccessingSecurityScopedResource() }
                }
            }
        }
    }

    private func section<C: View>(_ title: String, @ViewBuilder _ content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.headline)
            content()
        }
    }

    @ViewBuilder private func linkedCards(_ c: StoryCharacterDto) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Linked Scene Cards (reference only)").font(.headline)
            if d.linkedCards.isEmpty { Text("Optional. Scene Cards never need characters.").font(.callout).foregroundStyle(theme["muted"]) }
            ForEach(d.linkedCards, id: \.cardId) { l in
                HStack {
                    Text(l.shortDescription.isEmpty ? "Blank scene card" : l.shortDescription).lineLimit(1)
                    Spacer()
                    Text(l.location).font(.caption).foregroundStyle(theme["muted"]).lineLimit(1)
                    Button { Task { await m.unlink(c.id, card: l.cardId) } } label: { Image(systemName: "xmark.circle") }
                        .buttonStyle(.borderless).accessibilityLabel("Unlink card")
                }
            }
            LinkCardRow(m: m, characterId: c.id, options: m.linkableCards(for: d))
        }
    }
}

private struct LinkCardRow: View {
    @Bindable var m: CharactersModel
    let characterId: String
    let options: [(id: String, label: String)]
    @State private var pick = ""

    var body: some View {
        if !options.isEmpty {
            HStack {
                Picker("Scene Card to link", selection: $pick) {
                    Text("Link a Scene Card\u{2026}").tag("")
                    ForEach(options, id: \.id) { Text($0.label).tag($0.id) }
                }
                .labelsHidden()
                Button("Link") {
                    let id = pick
                    pick = ""
                    Task { await m.link(characterId, card: id) }
                }
                .controlSize(.small).disabled(pick.isEmpty)
            }
        }
    }
}

private struct CharacterNotes: View {
    @Bindable var m: CharactersModel
    let id: String
    let value: String
    @State private var text = ""
    @State private var last = ""
    @FocusState private var focused: Bool
    @Environment(\.ofTheme) private var theme

    var body: some View {
        TextEditor(text: $text)
            .scrollContentBackground(.hidden)
            .frame(minHeight: 100)
            .padding(4)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(focused ? theme["accent"] : theme["line"]))
            .focused($focused)
            .accessibilityLabel("Notes")
            .onChange(of: focused) { _, now in
                if !now, text != last {
                    last = text
                    Task { await m.saveNotes(id, text) }
                }
            }
            .onChange(of: value) { _, v in if !focused { text = v; last = v } }
            .onAppear { text = value; last = value }
            .onDisappear {
                if text != last { let t = text; last = t; Task { await m.saveNotes(id, t) } }
            }
    }
}

struct RelationshipList: View {
    @Bindable var m: CharactersModel
    let rels: [StoryRelationshipDto]
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if rels.isEmpty {
            Text("No relationships yet.").font(.callout).foregroundStyle(theme["muted"])
        } else {
            ForEach(rels, id: \.id) { r in
                HStack(spacing: 8) {
                    Text(r.fromName).fontWeight(.semibold).textCase(.uppercase)
                    Text("\u{2500}\u{2500} \(r.relationshipType) \u{2500}\u{2500}").foregroundStyle(theme["muted"])
                    Text(r.toName).fontWeight(.semibold).textCase(.uppercase)
                    if let n = r.note, !n.isEmpty { Text("(\(n))").font(.caption).foregroundStyle(theme["muted"]) }
                    Spacer()
                    Button { Task { await m.removeRelationship(r.id) } } label: { Image(systemName: "xmark.circle") }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Remove relationship \(r.fromName) \(r.relationshipType) \(r.toName)")
                }
                .accessibilityElement(children: .combine)
            }
        }
    }
}

// MARK: Sheets

private struct NewCharacterSheet: View {
    @Bindable var m: CharactersModel
    @State private var name = ""
    @State private var role = ""
    @State private var desc = ""
    @State private var image: URL?
    @State private var picking = false
    @State private var busy = false
    @FocusState private var nameFocus: Bool

    var body: some View {
        let match = m.match(for: name)
        StorySheetFrame(title: "New Character") {
            VStack(alignment: .leading, spacing: 10) {
                field("Name") { TextField("Name", text: $name).focused($nameFocus).accessibilityLabel("Character name").accessibilityIdentifier("story-new-character-name") }
                if let match {
                    StoryBanner(tone: .warn, text: "Possible match: a character named \(match.name.uppercased())\(match.description.map { " (\($0))" } ?? "") already exists.")
                }
                field("Role / title") { TextField("Protagonist", text: $role).accessibilityLabel("Role").accessibilityIdentifier("story-new-character-role") }
                field("Image") {
                    HStack {
                        Button(image == nil ? "Choose image\u{2026}" : "Change image\u{2026}") { picking = true }.controlSize(.small)
                        if let image { Text(image.lastPathComponent).font(.caption).foregroundStyle(.secondary).lineLimit(1) }
                    }
                }
                field("Short description") { TextField("Add later", text: $desc).accessibilityLabel("Short description") }
            }
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            if let match {
                Button("Create Another Character Anyway") { create() }.disabled(busy)
                Button("Use Existing") {
                    m.selectedId = match.id
                    m.sheet = nil
                }
                .keyboardShortcut(.defaultAction)
            } else {
                Button("Create Character") { create() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(busy || name.trimmingCharacters(in: .whitespaces).isEmpty)
                    .accessibilityIdentifier("story-create-character")
            }
        }
        .onAppear { nameFocus = true }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.image]) { r in if case let .success(u) = r { image = u } }
    }

    private func field<C: View>(_ label: String, @ViewBuilder _ c: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(label).font(.callout.weight(.medium))
            c().textFieldStyle(.roundedBorder)
        }
    }

    private func create() {
        busy = true
        Task {
            let scoped = image?.startAccessingSecurityScopedResource() ?? false
            let id = await m.create(name: name, role: role, description: desc, imagePath: image?.path)
            if scoped { image?.stopAccessingSecurityScopedResource() }
            busy = false
            if id != nil { m.sheet = nil }
        }
    }
}

private struct EditCharacterSheet: View {
    @Bindable var m: CharactersModel
    let c: StoryCharacterDto
    @State private var name = ""
    @State private var role = ""
    @State private var desc = ""
    @FocusState private var nameFocus: Bool

    var body: some View {
        StorySheetFrame(title: "Edit \(c.name)") {
            VStack(alignment: .leading, spacing: 10) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Name").font(.callout.weight(.medium))
                    TextField("Name", text: $name).textFieldStyle(.roundedBorder).focused($nameFocus).accessibilityLabel("Character name")
                    Text("Renaming doesn't change screenplay text; use Find & Replace in the screenplay for that.").font(.caption).foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 3) {
                    Text("Role / title").font(.callout.weight(.medium))
                    TextField("Role", text: $role).textFieldStyle(.roundedBorder).accessibilityLabel("Role")
                }
                VStack(alignment: .leading, spacing: 3) {
                    Text("Short description").font(.callout.weight(.medium))
                    TextEditor(text: $desc).frame(height: 70).padding(3)
                        .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(.quaternary))
                        .accessibilityLabel("Short description")
                }
            }
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Save") {
                Task { if await m.update(c, name: name, role: role, description: desc) { m.sheet = nil } }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
            .accessibilityIdentifier("story-save-character")
        }
        .onAppear {
            name = c.name
            role = c.roleLabel ?? ""
            desc = c.description ?? ""
            nameFocus = true
        }
    }
}

private struct AddRelationshipSheet: View {
    @Bindable var m: CharactersModel
    let from: String
    @State private var fromId = ""
    @State private var to = ""
    @State private var type = ""
    @State private var note = ""

    var body: some View {
        let chars = m.list.filter { !$0.archived }
        StorySheetFrame(title: "Add Relationship", width: 520) {
            VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .bottom, spacing: 8) {
                    Picker("Character", selection: $fromId) { ForEach(chars, id: \.id) { Text($0.name.uppercased()).tag($0.id) } }
                    VStack(alignment: .leading, spacing: 3) {
                        Text("is").font(.callout.weight(.medium))
                        HStack {
                            TextField("father of", text: $type).textFieldStyle(.roundedBorder).accessibilityLabel("Relationship")
                            Menu("Suggestions") {
                                ForEach(characterRelationSuggestions, id: \.self) { s in Button(s) { type = s } }
                            }
                            .fixedSize()
                        }
                    }
                    Picker("Character", selection: $to) { ForEach(chars, id: \.id) { Text($0.name.uppercased()).tag($0.id) } }
                }
                VStack(alignment: .leading, spacing: 3) {
                    Text("Note (optional)").font(.callout.weight(.medium))
                    TextField("Note", text: $note).textFieldStyle(.roundedBorder).accessibilityLabel("Note")
                }
            }
        } footer: {
            Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Add Relationship") {
                Task { if await m.addRelationship(from: fromId, to: to, type: type, note: note) { m.sheet = nil } }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(type.trimmingCharacters(in: .whitespaces).isEmpty || to.isEmpty || to == fromId)
        }
        .onAppear {
            fromId = from
            to = chars.first { $0.id != from }?.id ?? ""
        }
    }
}

private struct DeleteCharacterSheet: View {
    @Bindable var m: CharactersModel
    let c: StoryCharacterDto
    let usage: StoryCharacterUsage

    var body: some View {
        if !usage.inUse {
            StorySheetFrame(title: "Delete \(c.name)?", width: 420) {
                Text("The character moves to Recently Deleted. You can restore it or use Undo.")
            } footer: {
                Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Delete Character", role: .destructive) {
                    Task { if await m.delete(c) { m.sheet = nil } }
                }
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("story-confirm-delete-character")
            }
        } else {
            let uses = [
                usage.screenplayScenes > 0 ? StoryLogic.plural(Int(usage.screenplayScenes), "screenplay scene") : nil,
                usage.castLinks > 0 ? "Cast & Crew" : nil,
                usage.catalogLinks > 0 ? "the Production Catalog" : nil,
            ].compactMap { $0 }
            StorySheetFrame(title: "\(c.name) is in use", width: 480) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("\(c.name) appears in \(uses.joined(separator: " and ")). Archiving removes the character from this list without breaking those links.")
                    Text("Screenplay text is never changed. If you delete, the character goes to Recently Deleted and can be restored.").foregroundStyle(.secondary)
                }
            } footer: {
                Button("Cancel") { m.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Delete Anyway", role: .destructive) { Task { if await m.delete(c) { m.sheet = nil } } }
                Button("Archive Instead") {
                    m.sheet = nil
                    Task { await m.setArchived(c, true) }
                }
                .keyboardShortcut(.defaultAction)
            }
        }
    }
}

// MARK: Relationship map

/// An optional visual helper (not an analysis tool): characters on a circle, a line per relationship.
struct RelationshipMapView: View {
    @Bindable var m: CharactersModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let chars = m.list.filter { !$0.archived }
        let rels = m.relationships
        let nodes = chars.filter { c in rels.contains { $0.fromCharacterId == c.id || $0.toCharacterId == c.id } }
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Button { m.mapOpen = false } label: { Label("Back to Characters", systemImage: "chevron.left") }.buttonStyle(.borderless)
                Text("Character relationships").font(.headline)
                Spacer()
                if chars.count >= 2 {
                    Button { m.sheet = .addRelationship(from: chars[0].id) } label: { Label("Add Relationship", systemImage: "plus") }
                        .buttonStyle(.borderedProminent).controlSize(.small)
                }
            }
            Text("Optional visual helper, not a character-analysis system.").font(.caption).foregroundStyle(theme["muted"])
            if rels.isEmpty {
                ContentUnavailableView("No relationships yet.", systemImage: "point.3.connected.trianglepath.dotted",
                                       description: Text("Add a relationship such as \u{201C}father of\u{201D} or \u{201C}colleague of\u{201D}."))
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 12) {
                        Canvas { ctx, size in
                            let layout = RelationshipMapLayout.layout(
                                ids: nodes.map(\.id),
                                rels: rels.map { .init(id: $0.id, from: $0.fromCharacterId, to: $0.toCharacterId) },
                                size: size)
                            for rel in rels {
                                guard let a = layout.positions[rel.fromCharacterId], let b = layout.positions[rel.toCharacterId],
                                      let e = layout.edges[rel.id] else { continue }
                                var p = Path(); p.move(to: a); p.addQuadCurve(to: b, control: e.control)
                                ctx.stroke(p, with: .color(theme["line2"]), lineWidth: 2)
                            }
                            // Labels after every line, on a pill, so no line runs through a word.
                            for rel in rels {
                                guard let e = layout.edges[rel.id] else { continue }
                                let label = ctx.resolve(Text(rel.relationshipType).font(.system(size: 11)).foregroundColor(theme["ink2"]))
                                let sz = label.measure(in: CGSize(width: 160, height: 40))
                                let pill = Path(roundedRect: CGRect(x: e.label.x - sz.width / 2 - 5, y: e.label.y - sz.height / 2 - 2,
                                                                    width: sz.width + 10, height: sz.height + 4), cornerRadius: 6)
                                ctx.fill(pill, with: .color(theme["panel"]))
                                ctx.draw(label, at: e.label)
                            }
                            for n in nodes {
                                guard let p = layout.positions[n.id] else { continue }
                                let circle = Path(ellipseIn: CGRect(x: p.x - 22, y: p.y - 22, width: 44, height: 44))
                                ctx.fill(circle, with: .color(theme["accent-soft"]))
                                ctx.stroke(circle, with: .color(theme["accent"]), lineWidth: 1)
                                ctx.draw(Text(RelationshipMapLayout.initials(n.name)).font(.system(size: 12, weight: .bold)).foregroundColor(theme["ink"]), at: p)
                                ctx.draw(Text(n.name).font(.system(size: 11, weight: .semibold)).foregroundColor(theme["ink"]),
                                         at: CGPoint(x: p.x, y: p.y + 34))
                            }
                        }
                        .frame(height: RelationshipMapLayout.height(for: nodes.count))
                        .accessibilityElement()
                        .accessibilityLabel("Relationship map")
                        .accessibilityValue(rels.map { "\($0.fromName) \($0.relationshipType) \($0.toName)" }.joined(separator: ", "))
                        RelationshipList(m: m, rels: rels)
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityLabel("Character relationships")
    }
}

fileprivate extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}

/// Where the relationship map puts characters, lines and labels (Windows: layoutRelationshipMap
/// in Characters.tsx, owner change list item 3). Characters sit on a circle in the order with the
/// fewest crossing lines; several relationships between one pair bend apart; each label is moved
/// along its line until it no longer sits on a name or another label.
enum RelationshipMapLayout {
    struct Rel { let id: String; let from: String; let to: String }
    struct Edge { let control: CGPoint; let label: CGPoint }
    struct Result { let positions: [String: CGPoint]; let edges: [String: Edge] }

    static func height(for count: Int) -> CGFloat { CGFloat(max(380, min(560, 260 + count * 34))) }

    static func initials(_ name: String) -> String {
        let parts = name.split(separator: " ")
        let s = (parts.first.map { String($0.prefix(1)) } ?? "") + (parts.count > 1 ? String(parts.last!.prefix(1)) : "")
        return s.uppercased()
    }

    static func layout(ids: [String], rels: [Rel], size: CGSize) -> Result {
        let n = ids.count
        let r = max(40, min(size.width, size.height) / 2 - 62)
        let order = n <= 8 ? fewestCrossings(ids, rels) : ids
        let step = 2 * Double.pi / Double(max(n, 1))
        var positions: [String: CGPoint] = [:]
        for (i, id) in order.enumerated() {
            let a = step * Double(i) - Double.pi / 2
            positions[id] = CGPoint(x: size.width / 2 + r * cos(a), y: size.height / 2 + r * sin(a))
        }
        // Names sit under each circle; labels keep clear of them.
        var placed: [CGPoint] = positions.values.map { CGPoint(x: $0.x, y: $0.y + 34) } + Array(positions.values)
        var pairCount: [String: Int] = [:]
        var edges: [String: Edge] = [:]
        for rel in rels {
            guard let a = positions[rel.from], let b = positions[rel.to] else { continue }
            let key = [rel.from, rel.to].sorted().joined(separator: "|")
            let k = pairCount[key, default: 0]
            pairCount[key] = k + 1
            let dir: Double = rel.from < rel.to ? 1 : -1
            let bend: Double = k == 0 ? 0 : Double((k + 1) / 2) * 46 * (k % 2 == 1 ? 1 : -1) * dir
            let dx = b.x - a.x, dy = b.y - a.y
            let len = max(hypot(dx, dy), 1)
            let control = CGPoint(x: (a.x + b.x) / 2 + (-dy / len) * bend, y: (a.y + b.y) / 2 + (dx / len) * bend)
            func at(_ t: Double) -> CGPoint {
                CGPoint(x: (1 - t) * (1 - t) * a.x + 2 * (1 - t) * t * control.x + t * t * b.x,
                        y: (1 - t) * (1 - t) * a.y + 2 * (1 - t) * t * control.y + t * t * b.y)
            }
            var label = at(0.5)
            for t in [0.5, 0.38, 0.62, 0.28, 0.72, 0.44, 0.56] {
                let p = at(t)
                if placed.allSatisfy({ abs($0.x - p.x) > 70 || abs($0.y - p.y) > 18 }) { label = p; break }
            }
            placed.append(label)
            edges[rel.id] = Edge(control: control, label: label)
        }
        return Result(positions: positions, edges: edges)
    }

    /// The order of characters round the circle with the fewest crossing lines (up to 8, every
    /// order tried; the first character stays in place because rotations are the same picture).
    static func fewestCrossings(_ ids: [String], _ rels: [Rel]) -> [String] {
        guard ids.count >= 4 else { return ids }
        let index = Dictionary(uniqueKeysWithValues: ids.enumerated().map { ($1, $0) })
        let pairs = rels.compactMap { r -> (Int, Int)? in
            guard let a = index[r.from], let b = index[r.to], a != b else { return nil }
            return (a, b)
        }
        var perm = Array(ids.indices)
        var slot = Array(repeating: 0, count: ids.count)
        var best = perm
        var bestScore = Int.max
        func crossings() -> Int {
            for (i, p) in perm.enumerated() { slot[p] = i }
            var c = 0
            for i in pairs.indices {
                let lo = min(slot[pairs[i].0], slot[pairs[i].1]), hi = max(slot[pairs[i].0], slot[pairs[i].1])
                for j in (i + 1)..<pairs.count {
                    let p = slot[pairs[j].0], q = slot[pairs[j].1]
                    if p == lo || p == hi || q == lo || q == hi { continue }
                    if (p > lo && p < hi) != (q > lo && q < hi) { c += 1 }
                }
            }
            return c
        }
        func visit(_ k: Int) {
            if bestScore == 0 { return }
            if k == perm.count {
                let c = crossings()
                if c < bestScore { bestScore = c; best = perm }
                return
            }
            for i in k..<perm.count {
                perm.swapAt(k, i)
                visit(k + 1)
                perm.swapAt(k, i)
            }
        }
        visit(1)
        return best.map { ids[$0] }
    }
}

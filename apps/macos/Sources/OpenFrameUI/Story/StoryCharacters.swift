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
    /// nil: the character list. A focus of nil: the whole-film map. A character id: that
    /// character's own map (Windows Characters.tsx).
    var map: RelationshipMapFocus?
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
            if let focus = m.map {
                RelationshipMapView(m: m, focus: focus.id)
            } else {
                list
                detailPane
            }
        }
        .padding(16)
        .task(id: app.changes.key(StoryTables.characters) &+ story.viewRevision &+ (m.showArchived ? 1 : 0) &+ (story.episodeId?.hashValue ?? 0) &+ (m.selectedId?.hashValue ?? 0)) {
            await m.reload()
        }
        .themedSheet(item: $m.sheet) { sheet in
            switch sheet {
            case .new: NewCharacterSheet(m: m)
            case let .edit(c): EditCharacterSheet(m: m, c: c)
            case let .addRelationship(from): AddRelationshipSheet(m: m, from: from)
            case let .delete(c, usage): DeleteCharacterSheet(m: m, c: c, usage: usage)
            }
        }
        .onProjectIntent("story.characters.new") { m.sheet = .new }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("story-characters")
    }

    private var list: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Characters").fontWeight(.semibold)
                Text("\(m.activeCount)").font(.caption).padding(.horizontal, 6).background(.quaternary, in: Capsule())
                Spacer()
                if m.list.count > 1 {
                    Button { m.map = RelationshipMapFocus(id: nil) } label: { Label("Whole film map", systemImage: "point.3.connected.trianglepath.dotted") }
                        .controlSize(.small)
                        .help("Every character with a relationship, the most connected first")
                        .accessibilityIdentifier("story-whole-film-map")
                }
                Button { m.sheet = .new } label: { Label("Add Character", systemImage: "plus") }
                    .buttonStyle(.ofProminent).controlSize(.small)
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
                    Button { m.map = RelationshipMapFocus(id: c.id) } label: { Label("Relationship Map", systemImage: "point.3.connected.trianglepath.dotted") }
                        .help("Only this character\u{2019}s direct relationships")
                        .accessibilityIdentifier("story-character-map")
                    Menu {
                        Button(c.image == nil ? "Add image\u{2026}" : "Change image\u{2026}") { picking = true }
                        Button("Remove image") { Task { await m.clearImage(c.id) } }.disabled(c.image == nil)
                        Button(c.archived ? "Unarchive" : "Archive") { Task { await m.setArchived(c, !c.archived) } }
                        Divider()
                        Button("Delete\u{2026}", role: .destructive) { Task { await m.askDelete(c) } }
                    } label: {
                        Label("More", systemImage: "ellipsis.circle")
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
                    Button { Task { await m.unlink(c.id, card: l.cardId) } } label: { Label("Unlink", systemImage: "xmark.circle") }
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
                    Button { Task { await m.removeRelationship(r.id) } } label: { Label("Remove", systemImage: "xmark.circle") }
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

/// Which relationship map is open: one character's (`id`) or the whole film's (nil).
struct RelationshipMapFocus: Equatable { var id: String? }

/// Relationship maps (Windows Characters.tsx, mock 069): an optional visual helper, not an
/// analysis system. With a focus character it shows only that character's direct relationships;
/// without one, the whole film. Names open that character's map. The list under the picture says
/// the same in words, so the map is also readable without sight.
struct RelationshipMapView: View {
    @Bindable var m: CharactersModel
    let focus: String?
    @Environment(\.ofTheme) private var theme

    /// What one map shows: the focus character (if any), its relationships, the layout and the
    /// words the screen and VoiceOver use.
    struct Content {
        var person: StoryCharacterDto?
        var list: [StoryRelationshipDto]
        var layout: RelationshipMapLayout.Result
        var title: String
        var pictureLabel: String
        /// The characters whose name opens their own map (every box but the centre).
        var links: [String]

        init(chars: [StoryCharacterDto], relationships all: [StoryRelationshipDto], focus: String?) {
            let person = focus.flatMap { id in chars.first { $0.id == id } }
            let list = person.map { p in all.filter { $0.fromCharacterId == p.id || $0.toCharacterId == p.id } } ?? all
            let live = chars.filter { !$0.archived || $0.id == focus }
            self.person = person
            self.list = list
            layout = RelationshipMapLayout.layout(
                live.map { .init(id: $0.id, name: $0.name) },
                list.map { .init(id: $0.id, from: $0.fromCharacterId, to: $0.toCharacterId, label: $0.relationshipType) },
                focus: person?.id)
            title = person.map { "\($0.name.uppercased()): relationships" } ?? "Character relationships"
            let n = list.count
            pictureLabel = "\(title). \(n) relationship\(n == 1 ? "" : "s"), listed below."
            links = layout.boxes.keys.filter { $0 != person?.id }.sorted()
        }
    }

    var body: some View {
        let chars = m.list
        let content = Content(chars: chars, relationships: m.relationships, focus: focus)
        let person = content.person
        let list = content.list
        let map = content.layout
        let title = content.title
        let addFrom = person ?? chars.first
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Button { m.map = nil } label: { Label("Back to Characters", systemImage: "chevron.left") }
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("story-map-back")
                Text(title).font(.headline).accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("story-map-title")
                Spacer()
                if person != nil {
                    Button { m.map = RelationshipMapFocus(id: nil) } label: { Label("Whole film map", systemImage: "point.3.connected.trianglepath.dotted") }
                        .controlSize(.small)
                        .accessibilityIdentifier("story-whole-film-map")
                }
                if chars.count >= 2, let from = addFrom {
                    Button { m.sheet = .addRelationship(from: from.id) } label: { Label("Add Relationship", systemImage: "plus") }
                        .buttonStyle(.ofProminent).controlSize(.small)
                        .accessibilityIdentifier("story-map-add-relationship")
                }
            }
            Text(person != nil
                 ? "Only this character\u{2019}s direct relationships. Choose a name to see theirs."
                 : "Every character with a relationship, the most connected first. Choose a name to see just their relationships.")
                .font(.caption).foregroundStyle(theme["muted"])
            if list.isEmpty {
                ContentUnavailableView(person.map { "\($0.name.uppercased()) has no relationships yet." } ?? "No relationships yet.",
                                       systemImage: "point.3.connected.trianglepath.dotted",
                                       description: Text("Add a relationship such as \u{201C}father of\u{201D} or \u{201C}colleague of\u{201D}."))
            } else {
                ScrollView([.horizontal, .vertical]) {
                    VStack(alignment: .leading, spacing: 12) {
                        picture(map, list: list, label: content.pictureLabel, centre: person?.id)
                        RelationshipList(m: m, rels: list)
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(person.map { "Relationships of \($0.name)" } ?? "Character relationships")
        .accessibilityIdentifier("story-relationship-map")
    }

    private func picture(_ map: RelationshipMapLayout.Result, list: [StoryRelationshipDto], label: String, centre: String?) -> some View {
        ZStack(alignment: .topLeading) {
            Canvas { ctx, _ in
                let line = theme["muted"]
                for r in list {
                    guard let e = map.edges[r.id] else { continue }
                    var p = Path()
                    p.move(to: e.path.start)
                    p.addQuadCurve(to: e.path.end, control: e.path.control)
                    ctx.stroke(p, with: .color(line), lineWidth: 1.6)
                    ctx.fill(Self.arrow(e.path), with: .color(line))
                }
                // Labels after every line, on the panel colour, so no line runs through a word.
                for r in list {
                    guard let e = map.edges[r.id] else { continue }
                    let box = CGRect(x: e.label.x - e.label.w / 2, y: e.label.y - e.label.h / 2, width: e.label.w, height: e.label.h)
                    ctx.fill(Path(roundedRect: box, cornerRadius: 4), with: .color(theme["panel"]))
                    ctx.draw(Text(e.label.text).font(.system(size: 11)).foregroundColor(theme["ink"]), at: CGPoint(x: e.label.x, y: e.label.y))
                }
            }
            .frame(width: map.W, height: map.H)
            .accessibilityElement()
            .accessibilityLabel(label)
            .accessibilityIdentifier("story-relationship-picture")
            ForEach(map.boxes.keys.sorted(), id: \.self) { id in
                if let b = map.boxes[id], let c = m.list.first(where: { $0.id == id }) {
                    nameBox(b, character: c, isCentre: id == centre)
                }
            }
        }
        .frame(width: map.W, height: map.H, alignment: .topLeading)
    }

    @ViewBuilder private func nameBox(_ b: RelationshipMapLayout.Box, character c: StoryCharacterDto, isCentre: Bool) -> some View {
        let shape = Capsule()
        let label = Text(b.text).font(.system(size: 12, weight: .bold))
            .foregroundStyle(isCentre ? theme["accent-ink"] : theme["ink"])
            .frame(width: b.w, height: b.h)
            .background(isCentre ? theme["accent"] : theme["accent-soft"], in: shape)
            .overlay(shape.strokeBorder(theme["accent"], lineWidth: 1.5))
            .help(c.name.uppercased())
        if isCentre {
            label.position(x: b.x, y: b.y).accessibilityLabel(c.name)
        } else {
            Button { m.map = RelationshipMapFocus(id: c.id) } label: { label }
                .buttonStyle(.plain)
                .position(x: b.x, y: b.y)
                .accessibilityLabel("Show relationships of \(c.name)")
                .accessibilityIdentifier("story-map-name-\(c.id)")
        }
    }

    /// An arrow head at the end of the curve, pointing along it.
    static func arrow(_ c: RelationshipMapLayout.Curve) -> Path {
        let dx = c.end.x - c.control.x, dy = c.end.y - c.control.y
        let len = max(hypot(dx, dy), 0.001)
        let ux = dx / len, uy = dy / len
        let size: CGFloat = 8
        let base = CGPoint(x: c.end.x - ux * size, y: c.end.y - uy * size)
        var p = Path()
        p.move(to: c.end)
        p.addLine(to: CGPoint(x: base.x - uy * size / 2, y: base.y + ux * size / 2))
        p.addLine(to: CGPoint(x: base.x + uy * size / 2, y: base.y - ux * size / 2))
        p.closeSubpath()
        return p
    }
}

fileprivate extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}

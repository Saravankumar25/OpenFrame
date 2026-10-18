// The Vault's sheets: Quick Capture, Add URL, Record Note, Send to Story, Copy, Move, Add to
// Collection, Tag, the add-files result and the name sheet for folders and collections. A sheet
// that cannot finish keeps what was typed and shows the core's plain message inside itself.

import AppKit
import OpenFrameCore
import SwiftUI

/// Title, optional line under it, content, and Cancel / action buttons.
struct VaultSheetFrame<Content: View, Footer: View>: View {
    let title: String
    var subtitle: String?
    var width: CGFloat = 440
    @ViewBuilder var content: Content
    @ViewBuilder var footer: Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.title3.weight(.semibold))
                if let subtitle {
                    Text(subtitle).font(.callout).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                }
            }
            content
            HStack(spacing: 8) { footer }
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
        .padding(20)
        .frame(width: width)
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
    }
}

struct SheetError: View {
    let message: String?
    @Environment(\.ofTheme) private var theme
    var body: some View {
        if let message {
            Text(message).font(.callout).foregroundStyle(theme["red-t"]).fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("sheet-error")
        }
    }
}

// MARK: Quick Capture

struct QuickCaptureSheet: View {
    let model: IdeaVaultModel
    let close: () -> Void
    @State private var text = ""
    @State private var error: String?
    @State private var busy = false
    @FocusState private var focused: Bool
    @Environment(\.ofTheme) private var theme

    private var context: String {
        model.isGlobal ? "Saving to your Global Idea Vault" : "Saving to \(model.project?.title ?? "this project") · \(VaultText.scopeLabel(model.scope))"
    }

    var body: some View {
        VaultSheetFrame(title: "Quick Capture", subtitle: context, width: 460) {
            VStack(alignment: .leading, spacing: 6) {
                ZStack(alignment: .topLeading) {
                    TextEditor(text: $text)
                        .font(.body)
                        .scrollContentBackground(.hidden)
                        .padding(6)
                        .background(theme["bg"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line2"]))
                        .frame(height: 120)
                        .focused($focused)
                        .accessibilityLabel("Idea")
                        .accessibilityIdentifier("quick-capture-idea")
                        .onKeyPress(.return, phases: .down) { press in
                            // Return saves; Shift+Return starts a new line.
                            if press.modifiers.contains(.shift) { return .ignored }
                            Task { await save() }
                            return .handled
                        }
                        .onChange(of: text) { _, _ in error = nil }
                    if text.isEmpty {
                        Text("Type the idea before it's gone…").foregroundStyle(theme["placeholder"]).padding(.horizontal, 11).padding(.vertical, 14)
                            .allowsHitTesting(false).accessibilityHidden(true)
                    }
                }
                SheetError(message: error)
                Text("Return to save · Shift+Return for a new line · Esc to cancel").font(.caption).foregroundStyle(theme["muted"])
            }
        } footer: {
            Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
            Button("Save") { Task { await save() } }
                .keyboardShortcut(.defaultAction)
                .disabled(busy || text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
        .onAppear { focused = true }
    }

    private func save() async {
        let t = text
        guard !t.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            try await model.quickCapture(t)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

// MARK: Add URL

struct AddURLSheet: View {
    let model: IdeaVaultModel
    let close: () -> Void
    @State private var url = ""
    @State private var note = ""
    @State private var error: String?
    @State private var busy = false
    @FocusState private var focused: Bool

    var body: some View {
        VaultSheetFrame(title: "Add URL", subtitle: "Paste a link. The link itself is what matters; a preview is just a bonus.") {
            VStack(alignment: .leading, spacing: 10) {
                LabeledContent("Link") {
                    TextField("https://…", text: $url)
                        .textFieldStyle(.roundedBorder)
                        .focused($focused)
                        .onSubmit { Task { await add() } }
                        .accessibilityIdentifier("add-url-link")
                }
                LabeledContent("Your note") {
                    TextField("Optional", text: $note, axis: .vertical)
                        .lineLimit(3...6)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityIdentifier("add-url-note")
                }
                SheetError(message: error)
                Text("OpenFrame stores the link as you entered it. Nothing is downloaded.").font(.caption).foregroundStyle(.secondary)
            }
        } footer: {
            Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
            Button("Add URL") { Task { await add() } }
                .keyboardShortcut(.defaultAction)
                .disabled(busy || url.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .onAppear { focused = true }
        .onChange(of: url) { _, _ in error = nil }
    }

    private func add() async {
        guard !busy, !url.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        busy = true
        defer { busy = false }
        do {
            _ = try await model.addURL(url, note: note)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

// MARK: Record Note

struct RecordNoteSheet: View {
    let model: IdeaVaultModel
    let close: () -> Void
    let chooseFile: () -> Void
    @State private var recorder = VoiceNoteModel()
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VaultSheetFrame(title: "Record Note", width: 420) {
            VStack(alignment: .leading, spacing: 12) {
                if case let .blocked(message, canOpenSettings) = recorder.phase {
                    Text(message).foregroundStyle(theme["red-t"]).fixedSize(horizontal: false, vertical: true)
                        .accessibilityIdentifier("record-blocked")
                    HStack {
                        if canOpenSettings {
                            Button("Open System Settings") { openMicrophoneSettings() }
                        }
                        Button("Choose an audio file…") {
                            close()
                            chooseFile()
                        }
                    }
                } else {
                    HStack(spacing: 12) {
                        Image(systemName: recorder.phase == .recording ? "mic.fill" : "mic")
                            .font(.title2)
                            .foregroundStyle(recorder.phase == .recording ? theme["red"] : theme["muted"])
                            .accessibilityHidden(true)
                        Text(recorder.timeText).font(.system(size: 28, weight: .medium).monospacedDigit())
                        Text(recorder.statusText).foregroundStyle(theme["muted"])
                        Spacer(minLength: 0)
                    }
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("record-status")
                    LabeledContent("Name (optional)") {
                        TextField("Leave blank to keep it untitled", text: $recorder.name)
                            .textFieldStyle(.roundedBorder)
                            .accessibilityIdentifier("record-name")
                    }
                    if case let .failed(message) = recorder.phase { SheetError(message: message) }
                    SheetError(message: recorder.error)
                    Text("The recording is always kept exactly as recorded.").font(.caption).foregroundStyle(theme["muted"])
                }
            }
        } footer: {
            Button("Cancel", role: .cancel) {
                recorder.discard()
                close()
            }.keyboardShortcut(.cancelAction)
            if recorder.canStart {
                Button("Start Recording") { Task { await recorder.start() } }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("record-start")
            }
            Button(recorder.phase == .stopped ? "Save" : "Stop & Save") {
                Task {
                    if await recorder.save(into: model) != nil { close() }
                }
            }
            .keyboardShortcut(recorder.canStart ? nil : .defaultAction)
            .disabled(!recorder.canSave)
            .accessibilityIdentifier("record-save")
        }
        .onDisappear { recorder.discard() }
    }

    private func openMicrophoneSettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone") {
            NSWorkspace.shared.open(url)
        }
    }
}

// MARK: Send to Story

struct SendToStorySheet: View {
    let model: IdeaVaultModel
    let item: VaultItemDto
    let close: () -> Void
    @State private var target: VaultStoryTarget = .sceneCard
    @State private var place = "parking"
    @State private var busy = false
    @State private var error: String?
    @Environment(\.ofTheme) private var theme

    private static let options: [(VaultStoryTarget, String, String?)] = [
        (.beat, "Beat", "a small story event"),
        (.sceneCard, "Scene Card", "a compact scene reminder"),
        (.sequence, "Sequence idea", "a group of scenes"),
        (.character, "Character note", nil),
    ]

    private var acts: [VaultStoryAct] { model.storyTargets?.acts ?? [] }

    private var placeOptions: [(value: String, label: String)] {
        if target == .character { return [] }
        var out: [(String, String)] = []
        if target != .sequence { out.append(("parking", "Parking Lot")) }
        for a in acts {
            out.append(("act:\(a.id)", a.title))
            if target != .sequence { for s in a.sequences { out.append(("seq:\(s.id)", "\(a.title) ▸ \(s.title)")) } }
        }
        return out
    }

    private var needsAct: Bool { target == .sequence && acts.isEmpty }

    var body: some View {
        VaultSheetFrame(title: "Send to Story Board", subtitle: "What should this become? This creates a copy. The original stays in your Idea Vault.", width: 480) {
            VStack(alignment: .leading, spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Idea").font(.caption).foregroundStyle(theme["muted"])
                    Text((item.title.map { "\($0). " } ?? "") + (item.body ?? item.caption ?? item.url ?? item.displayName))
                        .lineLimit(5)
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(item.itemType == .note ? theme["note-bg"] : theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))

                Picker("What should this become?", selection: $target) {
                    ForEach(Self.options, id: \.0) { o in
                        Text(o.2.map { "\(o.1) (\($0))" } ?? o.1).tag(o.0)
                    }
                }
                .pickerStyle(.radioGroup)
                .accessibilityIdentifier("send-target")

                if target != .character {
                    if needsAct {
                        Text("There are no acts yet. Add an act on the Story Board first, or choose Beat or Scene Card.")
                            .font(.callout).foregroundStyle(theme["muted"])
                    } else {
                        Picker("Place it in", selection: $place) {
                            ForEach(placeOptions, id: \.value) { Text($0.label).tag($0.value) }
                        }
                        .accessibilityIdentifier("send-place")
                        Text(target == .sequence ? "A sequence idea goes into an act." : "Or leave it in the Parking Lot.")
                            .font(.caption).foregroundStyle(theme["muted"])
                    }
                }
                SheetError(message: error)
            }
        } footer: {
            Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
            Button("Create Copy in Story Board") { Task { await submit() } }
                .keyboardShortcut(.defaultAction)
                .disabled(busy || needsAct)
        }
        .task { await model.loadStoryTargets() }
        .onChange(of: target) { _, _ in fixPlace() }
        .onChange(of: model.storyTargets) { _, _ in fixPlace() }
    }

    private func fixPlace() {
        if let first = placeOptions.first, !placeOptions.contains(where: { $0.value == place }) { place = first.value }
    }

    private func submit() async {
        busy = true
        defer { busy = false }
        let parts = place.split(separator: ":", maxSplits: 1).map(String.init)
        let kind = parts.first ?? "parking"
        let parentType: String? = target == .character ? nil : (kind == "act" ? "act" : kind == "seq" ? "sequence" : "parking")
        let parentId = parts.count > 1 ? parts[1] : nil
        do {
            try await model.sendToStory(item, target: target, parentType: parentType, parentId: parentId)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

// MARK: Copy between vaults

struct CopySheet: View {
    let model: IdeaVaultModel
    let items: [VaultItemDto]
    let close: () -> Void
    @State private var collections: [VaultCollectionDto] = []
    @State private var collectionId = ""
    @State private var busy = false
    @State private var error: String?
    @Environment(\.ofTheme) private var theme

    private var to: StoreSel { model.scope == .global ? .project : .global }
    private var title: String { to == .project ? "Copy to Project" : "Copy to Global Vault" }

    var body: some View {
        VaultSheetFrame(title: title, width: 460) {
            VStack(alignment: .leading, spacing: 12) {
                Text(to == .project
                    ? "The copy is independent. Editing it will not change the Global original, and the Global original stays where it is."
                    : "The copy is independent. Editing it will not change this project's item, and the project item stays where it is.")
                    .fixedSize(horizontal: false, vertical: true)
                VStack(alignment: .leading, spacing: 2) {
                    if items.count == 1 {
                        Text(items[0].displayName).font(.headline).lineLimit(1)
                        Text("\(model.isGlobal ? "Global" : "Project") \(items[0].itemType == .note ? "note" : "item")").font(.caption).foregroundStyle(theme["muted"])
                    } else {
                        Text("\(items.count) items").font(.headline)
                    }
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                LabeledContent("Copy into") { Text(to == .project ? model.projectLabel : "Global Idea Vault") }
                Picker("Collection (optional)", selection: $collectionId) {
                    Text("None").tag("")
                    ForEach(collections, id: \.id) { Text($0.name).tag($0.id) }
                }
                SheetError(message: error)
            }
        } footer: {
            Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
            Button(title) { Task { await submit() } }.keyboardShortcut(.defaultAction).disabled(busy)
        }
        .task { collections = await model.destinationCollections() }
    }

    private func submit() async {
        busy = true
        defer { busy = false }
        do {
            try await model.copy(items, collectionId: collectionId.isEmpty ? nil : collectionId)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

// MARK: Move to folder

struct MoveToFolderSheet: View {
    let model: IdeaVaultModel
    let items: [VaultItemDto]
    let close: () -> Void
    @State private var target = ""
    @State private var creating = false
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        if creating {
            NameSheet(title: "New Folder", label: "Folder name", confirmLabel: "Create and Move", hint: nil, initial: "", close: { creating = false }) { name in
                try await model.createFolderAndMove(items, name: name)
                close()
            }
        } else {
            VaultSheetFrame(title: "Move to Folder", subtitle: "Folders only organise. Nothing is copied or changed.", width: 400) {
                VStack(alignment: .leading, spacing: 8) {
                    Picker("Folder", selection: $target) {
                        Text("No folder (All items)").tag("")
                        ForEach(VaultFolders.tree(model.folders), id: \.folder.id) { e in
                            Text(String(repeating: "    ", count: e.depth) + e.folder.name).tag(e.folder.id)
                        }
                    }
                    .pickerStyle(.radioGroup)
                    .accessibilityIdentifier("move-folder")
                    SheetError(message: error)
                }
            } footer: {
                Button("New Folder…") { creating = true }
                Spacer()
                Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
                Button("Move") { Task { await submit() } }.keyboardShortcut(.defaultAction).disabled(busy)
            }
            .onAppear { if items.count == 1 { target = items[0].folderId ?? "" } }
        }
    }

    private func submit() async {
        busy = true
        defer { busy = false }
        do {
            try await model.move(items, to: target.isEmpty ? nil : target)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

// MARK: Add to collection

struct AddToCollectionSheet: View {
    let model: IdeaVaultModel
    let items: [VaultItemDto]
    let close: () -> Void
    @State private var target = ""
    @State private var creating = false
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        if creating || model.collections.isEmpty {
            NameSheet(title: "New Collection", label: "Name", confirmLabel: "Create", hint: VaultHints.collection, initial: "", close: {
                if model.collections.isEmpty { close() } else { creating = false }
            }) { name in
                try await model.createCollectionAndAdd(items, name: name)
                close()
            }
        } else {
            VaultSheetFrame(title: "Add to Collection", subtitle: "Collections are references. The items stay where they are.", width: 400) {
                VStack(alignment: .leading, spacing: 8) {
                    Picker("Collection", selection: $target) {
                        ForEach(model.collections, id: \.id) { Text("\($0.name)  (\(Int($0.itemCount)))").tag($0.id) }
                    }
                    .pickerStyle(.radioGroup)
                    SheetError(message: error)
                }
            } footer: {
                Button("New Collection…") { creating = true }
                Spacer()
                Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
                Button("Add") { Task { await submit() } }.keyboardShortcut(.defaultAction).disabled(busy || target.isEmpty)
            }
            .onAppear { if target.isEmpty { target = model.collections.first?.id ?? "" } }
        }
    }

    private func submit() async {
        busy = true
        defer { busy = false }
        do {
            try await model.addToCollection(items, collectionId: target)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

enum VaultHints {
    static let researchFlag = "Verification flags are yours to set. OpenFrame never marks research as true just because you imported it."
    static let collection = "Examples: Ending ideas · Visual References · Crazy scenes · Locations · Research · Music · Things to remember. OpenFrame never prescribes names."
}

// MARK: Tag

struct TagSheet: View {
    let model: IdeaVaultModel
    let items: [VaultItemDto]
    let close: () -> Void
    @State private var text = ""
    @State private var error: String?
    @FocusState private var focused: Bool

    var body: some View {
        VaultSheetFrame(title: "Tag", subtitle: "Add tags to \(VaultText.itemsPhrase(items)). Tags help you find things; they never classify.", width: 420) {
            VStack(alignment: .leading, spacing: 6) {
                TextField("rain, night, locations", text: $text)
                    .textFieldStyle(.roundedBorder)
                    .focused($focused)
                    .onSubmit { Task { await submit() } }
                    .accessibilityLabel("Tags")
                Text("Separate tags with commas.").font(.caption).foregroundStyle(.secondary)
                SheetError(message: error)
            }
        } footer: {
            Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
            Button("Add Tags") { Task { await submit() } }.keyboardShortcut(.defaultAction)
        }
        .onAppear { focused = true }
        .onChange(of: text) { _, _ in error = nil }
    }

    private func submit() async {
        let tags = VaultText.parseTags(text)
        guard !tags.isEmpty else {
            error = "Type at least one tag."
            return
        }
        do {
            try await model.addTags(items, tags)
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

// MARK: Add files result

struct AddResultSheet: View {
    let model: IdeaVaultModel
    let result: VaultAddFilesResult
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let added = result.added.count, failed = result.failed.count
        VaultSheetFrame(
            title: "\(added) \(added == 1 ? "item" : "items") added, \(failed) could not be added",
            subtitle: "Successful items were kept. Only the failed item needs attention.", width: 500
        ) {
            ScrollView {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(result.added, id: \.id) { i in
                        HStack { Text(i.displayName).lineLimit(1); Spacer(); Text("Added").foregroundStyle(theme["green-t"]) }
                    }
                    ForEach(result.failed, id: \.path) { f in
                        VStack(alignment: .leading, spacing: 1) {
                            HStack { Text(f.name).lineLimit(1); Spacer(); Text("Not added").foregroundStyle(theme["red-t"]) }
                            Text(f.reason).font(.caption).foregroundStyle(theme["muted"])
                        }
                    }
                }
            }
            .frame(maxHeight: 220)
        } footer: {
            Button(failed == 1 ? "Try Failed Item Again" : "Try Failed Items Again") {
                let paths = result.failed.map(\.path)
                close()
                Task { await model.addFiles(paths) }
            }
            Button("Done", action: close).keyboardShortcut(.defaultAction)
        }
    }
}

// MARK: Name sheet

/// New Collection, New Folder and the rename sheets.
struct NameSheet: View {
    let title: String
    let label: String
    let confirmLabel: String
    let hint: String?
    let initial: String
    let close: () -> Void
    let submit: (String) async throws -> Void
    @State private var name = ""
    @State private var error: String?
    @State private var busy = false
    @FocusState private var focused: Bool

    var body: some View {
        VaultSheetFrame(title: title, width: 400) {
            VStack(alignment: .leading, spacing: 6) {
                TextField(label, text: $name)
                    .textFieldStyle(.roundedBorder)
                    .focused($focused)
                    .onSubmit { Task { await go() } }
                    .accessibilityLabel(label)
                    .accessibilityIdentifier("name-sheet-field")
                if let hint { Text(hint).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
                SheetError(message: error)
            }
        } footer: {
            Button("Cancel", role: .cancel, action: close).keyboardShortcut(.cancelAction)
            Button(confirmLabel) { Task { await go() } }.keyboardShortcut(.defaultAction).disabled(busy)
        }
        .onAppear {
            name = initial
            focused = true
        }
        .onChange(of: name) { _, _ in error = nil }
    }

    private func go() async {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else {
            error = "\(label) is required."
            return
        }
        busy = true
        defer { busy = false }
        do {
            try await submit(String(trimmed.prefix(120)))
            close()
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

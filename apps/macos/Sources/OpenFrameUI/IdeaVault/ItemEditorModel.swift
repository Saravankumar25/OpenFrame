// The open item in the drawer: its text fields with automatic saving. Typing waits a moment and
// saves in the background; closing the drawer or opening another item saves at once, so typing
// is never lost. A value that changed for another reason (undo, another window) replaces only a
// field the person is not editing, so the caret never jumps.

import Foundation
import Observation
import OpenFrameCore

struct VaultFields: Equatable {
    var title = ""
    var body = ""
    var caption = ""
    var url = ""
    var sourceText = ""

    init() {}
    init(_ i: VaultItemDto) {
        title = i.title ?? ""
        body = i.body ?? ""
        caption = i.caption ?? ""
        url = i.url ?? ""
        sourceText = i.sourceText ?? ""
    }
}

enum VaultField: CaseIterable { case title, body, caption, url, sourceText }

extension VaultFields {
    subscript(field: VaultField) -> String {
        get {
            switch field {
            case .title: title
            case .body: body
            case .caption: caption
            case .url: url
            case .sourceText: sourceText
            }
        }
        set {
            switch field {
            case .title: title = newValue
            case .body: body = newValue
            case .caption: caption = newValue
            case .url: url = newValue
            case .sourceText: sourceText = newValue
            }
        }
    }
}

@MainActor
@Observable
final class ItemEditorModel {
    enum Status: Equatable { case saved, saving, failed }

    let vault: IdeaVaultModel
    let id: String
    let scope: StoreSel

    private(set) var item: VaultItemDto?
    /// True when the item can no longer be read (deleted, for example).
    private(set) var missing = false
    var draft = VaultFields()
    private(set) var status: Status = .saved
    private(set) var urlError: String?

    @ObservationIgnored private var baseline = VaultFields()
    @ObservationIgnored private var dirty = Set<VaultField>()
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private var inflight: Task<Void, Never>?
    /// How long typing rests before it is saved.
    @ObservationIgnored var delay: Duration = .milliseconds(600)

    init(vault: IdeaVaultModel, id: String) {
        self.vault = vault
        self.id = id
        self.scope = vault.scope
    }

    /// Reads the item and merges it into fields the person is not editing.
    func refresh() async {
        guard let core = vault.app.core else { return }
        do {
            let fresh = try await core.vaultGet(VaultGetArgs(store: scope, id: id))
            missing = false
            if item == nil {
                draft = VaultFields(fresh)
                baseline = draft
            } else {
                let server = VaultFields(fresh)
                for f in VaultField.allCases where server[f] != baseline[f] && !dirty.contains(f) {
                    baseline[f] = server[f]
                    draft[f] = server[f]
                }
            }
            item = fresh
        } catch {
            if item == nil || !(error is CancellationError) { missing = true }
        }
    }

    /// A field changed by typing.
    func edit(_ field: VaultField, to value: String) {
        guard draft[field] != value else { return }
        draft[field] = value
        dirty.insert(field)
        status = .saving
        timer?.cancel()
        let wait = delay
        timer = Task { [weak self] in
            try? await Task.sleep(for: wait)
            guard !Task.isCancelled else { return }
            await self?.flush()
        }
    }

    /// Saves what is waiting. Returns true when nothing is left unsaved.
    @discardableResult
    func flush() async -> Bool {
        timer?.cancel()
        timer = nil
        if let inflight { await inflight.value }
        let keys = dirty
        guard !keys.isEmpty, let core = vault.app.core else { return dirty.isEmpty }
        let snapshot = draft
        dirty = []
        status = .saving
        let store = scope, itemId = id
        let run = Task { [weak self] in
            guard let self else { return }
            do {
                var args = VaultUpdateArgs(store: store, id: itemId)
                if keys.contains(.title) { args.title = snapshot.title }
                if keys.contains(.body) { args.body = snapshot.body }
                if keys.contains(.caption) { args.caption = snapshot.caption }
                if keys.contains(.url) { args.url = snapshot.url }
                if keys.contains(.sourceText) { args.sourceText = snapshot.sourceText }
                let saved = try await core.vaultUpdate(args)
                let s = VaultFields(saved)
                for k in keys { baseline[k] = s[k] }
                item = saved
                if keys.contains(.url) { urlError = nil }
                status = dirty.isEmpty ? .saved : .saving
            } catch {
                for k in keys { dirty.insert(k) }
                status = .failed
                let failure = CoreFailure(asCore: error)
                if keys.contains(.url), failure.code.hasPrefix("validation") {
                    urlError = failure.message
                } else {
                    vault.app.show("Could not save", error)
                }
            }
        }
        inflight = run
        await run.value
        inflight = nil
        await vault.reload()
        return dirty.isEmpty
    }

    var statusText: String {
        switch status {
        case .saved: "Saved automatically"
        case .saving: "Saving…"
        case .failed: "Not saved yet."
        }
    }

    /// The title in the drawer header.
    var heading: String {
        guard let item else { return "Loading…" }
        let typed = draft.title.trimmingCharacters(in: .whitespaces)
        if !typed.isEmpty { return typed }
        if item.itemType == .note { return "Untitled note" }
        if item.itemType == .quote { return "Untitled quote" }
        return item.title ?? item.displayName
    }
}

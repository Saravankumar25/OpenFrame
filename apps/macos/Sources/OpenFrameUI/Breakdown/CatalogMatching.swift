// "Existing catalog items" for a name (FSD 28.5, 143), shared by the Add Element sheet, the Edit
// Suggestion sheet and the Catalog's Add Item sheet. Renaming runs the matching again after a
// short pause. The rule for the default choice is the one on Windows (dialogs.tsx): nothing
// similar means create a new item, exactly one same item means use it, anything else means the
// person must choose.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class CatalogMatchFinder {
    private let core: () -> Core?
    private(set) var matches: [CatalogMatchDto] = []
    private(set) var failed = false
    private var requestedKey = ""
    private var answeredKey = ""
    private(set) var fetching = false
    /// The person's own pick; nil until they click a row.
    var choice: BreakdownCatalogChoice?
    @ObservationIgnored private var task: Task<Void, Never>?
    @ObservationIgnored private var name = ""
    private let pause: Duration

    init(core: @escaping () -> Core?, pause: Duration = .milliseconds(150)) {
        self.core = core
        self.pause = pause
    }

    /// The name has no text, so there is nothing to look for.
    var isBlank: Bool { name.isEmpty }

    /// "Looking for existing catalog items…" shows while this is true.
    var loading: Bool { !name.isEmpty && (fetching || answeredKey != requestedKey) }

    var defaultChoice: BreakdownCatalogChoice? {
        guard !name.isEmpty, !loading, !failed else { return nil }
        if matches.isEmpty { return BreakdownCatalogChoice(mode: .new, catalogItemId: nil) }
        if matches.count == 1, matches[0].exact { return BreakdownCatalogChoice(mode: .existing, catalogItemId: matches[0].id) }
        return nil
    }

    var effective: BreakdownCatalogChoice? { choice ?? defaultChoice }

    /// Several items look similar and nothing is chosen yet.
    var needsChoice: Bool { !loading && !name.isEmpty && effective == nil && !matches.isEmpty }

    func update(category: BreakdownCategory, name rawName: String, excludeId: String? = nil) {
        let trimmed = rawName.trimmingCharacters(in: .whitespacesAndNewlines)
        name = trimmed
        choice = nil
        task?.cancel()
        requestedKey = "\(category.rawValue)|\(trimmed)"
        guard !trimmed.isEmpty else {
            matches = []
            answeredKey = requestedKey
            fetching = false
            failed = false
            return
        }
        let key = requestedKey
        task = Task { [weak self] in
            guard let self else { return }
            try? await Task.sleep(for: self.pause)
            guard !Task.isCancelled, let core = self.core() else { return }
            self.fetching = true
            do {
                let found = try await core.catalogFindMatches(CatalogMatchArgs(category: category.rawValue, name: trimmed, excludeId: excludeId))
                guard !Task.isCancelled, key == self.requestedKey else { return }
                self.matches = found
                self.failed = false
            } catch {
                guard !Task.isCancelled, key == self.requestedKey else { return }
                self.matches = []
                self.failed = true
            }
            self.answeredKey = key
            self.fetching = false
        }
    }

    /// Waits until the latest lookup has finished (for tests).
    func settle() async {
        await task?.value
    }
}

func sameChoice(_ a: BreakdownCatalogChoice?, _ b: BreakdownCatalogChoice) -> Bool {
    guard let a else { return false }
    return a.mode == b.mode && (a.mode != .existing || a.catalogItemId == b.catalogItemId)
}

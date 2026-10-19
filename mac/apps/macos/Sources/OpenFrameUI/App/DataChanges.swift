// Exact reloads from the core's `dataChanged` events, as on Windows: every committed change lists
// the tables it touched, and each screen declares the tables it reads (events.rs). A screen
// reloads when the version of any of its tables moves; nothing polls.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
public final class DataChanges {
    /// Version per "store:table". Missing means 0.
    private var versions: [String: Int] = [:]
    /// Bumped on every change, for screens that read many tables.
    public private(set) var any: Int = 0

    public init() {}

    func bump(store: StoreKind, tables: [String]) {
        let prefix = store == .global ? "global:" : "project:"
        for table in tables {
            versions[prefix + table, default: 0] += 1
        }
        any += 1
    }

    /// A key that changes whenever any of these project tables change. Use it as a `.task(id:)`.
    public func key(_ tables: [String], store: StoreKind = .project) -> Int {
        let prefix = store == .global ? "global:" : "project:"
        return tables.reduce(0) { $0 &+ (versions[prefix + $1] ?? 0) }
    }

    /// A key for tables whose names start with any of these prefixes (for example "story_").
    public func key(prefixes: [String], store: StoreKind = .project) -> Int {
        let prefix = store == .global ? "global:" : "project:"
        return versions.reduce(0) { sum, entry in
            let name = entry.key.dropFirst(prefix.count)
            guard entry.key.hasPrefix(prefix), prefixes.contains(where: { name.hasPrefix($0) }) else { return sum }
            return sum &+ entry.value
        }
    }
}

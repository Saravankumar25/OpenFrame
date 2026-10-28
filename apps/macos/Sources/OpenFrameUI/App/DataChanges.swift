// Exact reloads from the core's `dataChanged` events, as on Windows: every committed change lists
// the tables it touched, and each screen declares the tables it reads (events.rs). A screen
// reloads when the version of any of its tables moves; nothing polls.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
public final class DataChanges {
    /// Version per "store:table", one observable box each: a screen that reads the key of its own
    /// tables is redrawn only when one of those tables changes. With one dictionary for all tables,
    /// every change anywhere redrew every screen that read a key, and laid its page out again.
    @ObservationIgnored private var versions: [String: TableVersion] = [:]
    /// Moves when a table changes for the first time, for keys that match table names by prefix.
    private var tablesSeen = 0
    /// Bumped on every change, for screens that read many tables.
    public private(set) var any: Int = 0

    public init() {}

    func bump(store: StoreKind, tables: [String]) {
        let prefix = store == .global ? "global:" : "project:"
        for table in tables {
            let box = version(prefix + table)
            if box.value == 0 { tablesSeen += 1 }
            box.value += 1
        }
        any += 1
    }

    /// Moves on when every screen must read again; part of every key.
    private var epoch = 0

    /// Every screen reads again (a project or the vault moved to another folder).
    func bumpAll() {
        epoch += 1
        any += 1
    }

    /// The box of one table, made the first time anyone asks (reading it observes only that table).
    private func version(_ name: String) -> TableVersion {
        if let box = versions[name] { return box }
        let box = TableVersion()
        versions[name] = box
        return box
    }

    /// A key that changes whenever any of these project tables change. Use it as a `.task(id:)`.
    public func key(_ tables: [String], store: StoreKind = .project) -> Int {
        let prefix = store == .global ? "global:" : "project:"
        return tables.reduce(epoch) { $0 &+ version(prefix + $1).value }
    }

    /// A key for tables whose names start with any of these prefixes (for example "story_").
    public func key(prefixes: [String], store: StoreKind = .project) -> Int {
        let prefix = store == .global ? "global:" : "project:"
        _ = tablesSeen
        return versions.reduce(epoch) { sum, entry in
            let name = entry.key.dropFirst(prefix.count)
            guard entry.key.hasPrefix(prefix), prefixes.contains(where: { name.hasPrefix($0) }) else { return sum }
            return sum &+ entry.value.value
        }
    }
}

/// The version of one table.
@MainActor
@Observable
final class TableVersion {
    var value = 0
}

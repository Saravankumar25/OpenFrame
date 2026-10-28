// A screen redraws only when one of its own tables changes. When every key read one shared
// dictionary, each change anywhere (the background indexer writes many) redrew every open screen,
// and a project page with an expensive layout kept the main thread busy for tens of seconds.

import Foundation
import Observation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

@MainActor
private final class Flag: @unchecked Sendable { var fired = false }

/// Whether `change` invalidates what `read` observed.
@MainActor
private func invalidates(_ read: () -> Void, by change: () -> Void) -> Bool {
    let flag = Flag()
    withObservationTracking { read() } onChange: { MainActor.assumeIsolated { flag.fired = true } }
    change()
    return flag.fired
}

@Suite("DataChangesTests")
@MainActor
struct DataChangesTests {
    @Test func aKeyMovesWithItsOwnTablesOnly() {
        let c = DataChanges()
        let before = c.key(["task", "project_note"])
        c.bump(store: .project, tables: ["search_doc"])
        #expect(c.key(["task", "project_note"]) == before)
        c.bump(store: .project, tables: ["task"])
        #expect(c.key(["task", "project_note"]) != before)
        // The same table in the other store is another table.
        let global = c.key(["task"], store: .global)
        c.bump(store: .project, tables: ["task"])
        #expect(c.key(["task"], store: .global) == global)
    }

    @Test func aScreenIsNotRedrawnForAnotherTablesChange() {
        let c = DataChanges()
        #expect(!invalidates({ _ = c.key(["task", "project_note"]) }, by: { c.bump(store: .project, tables: ["search_doc"]) }))
        #expect(invalidates({ _ = c.key(["task", "project_note"]) }, by: { c.bump(store: .project, tables: ["task"]) }))
        // A table nobody changed yet is watched too: its first change reaches the screen.
        #expect(invalidates({ _ = c.key(["deleted_item"]) }, by: { c.bump(store: .project, tables: ["deleted_item"]) }))
        #expect(invalidates({ _ = c.key(["task"]) }, by: { c.bumpAll() }))
    }

    @Test func aPrefixKeySeesNewAndChangedTablesWithThatPrefix() {
        let c = DataChanges()
        let before = c.key(prefixes: ["story_"])
        #expect(invalidates({ _ = c.key(prefixes: ["story_"]) }, by: { c.bump(store: .project, tables: ["story_card"]) }))
        #expect(c.key(prefixes: ["story_"]) != before)
        #expect(invalidates({ _ = c.key(prefixes: ["story_"]) }, by: { c.bump(store: .project, tables: ["story_card"]) }))
        let now = c.key(prefixes: ["story_"])
        c.bump(store: .project, tables: ["task"])
        #expect(c.key(prefixes: ["story_"]) == now)
    }

    @Test func everyChangeStillMovesTheCounterForScreensThatReadAll() {
        let c = DataChanges()
        let before = c.any
        #expect(invalidates({ _ = c.any }, by: { c.bump(store: .project, tables: ["search_doc"]) }))
        #expect(c.any == before + 1)
    }
}

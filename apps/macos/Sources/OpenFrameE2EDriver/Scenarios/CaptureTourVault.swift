// Capture tour section: Idea Vault. Primitives and helpers are in CaptureTour.swift.
// Three scenarios, each from an empty data folder: adding and opening items (a), organising them
// with pins, tags, folders, collections and search (b), deleting, the trash and the Global vault (c).

import AppKit
import Foundation

private let vaultArea = "Idea Vault"
private let vaultIdeas = ["A bus that never reaches its stop", "Rain on the depot roof, a single lamp", "What if the driver is the ticket inspector?"]

fileprivate extension UI {
    /// The card or row of an item whose name contains this text.
    func vaultCard(_ text: String) -> Element? {
        capQueryPrefix("vault-card-").first { $0.frame.width > 0 && ($0.identifier.contains(text) || $0.label.contains(text)) }
    }

    func vaultCardOrThrow(_ text: String) async throws -> Element {
        try await capWaitUntil("the card \(text)", timeout: 8) { vaultCard(text) != nil }
        guard let c = vaultCard(text) else { throw DriverError("no card \(text)") }
        return c
    }

    /// Clears the notice bar so it does not cover the next picture.
    func vaultQuiet() async {
        if let d = query(label: "Dismiss").first(where: { $0.role == "AXButton" && $0.frame.width > 0 }) { _ = d.performPress(); await idle(0.4) }
    }

    func vaultQuickCapture(_ text: String) async throws {
        try await capAct(id: "vault-quick-capture")
        _ = try await wait(id: "quick-capture-idea", timeout: 6)
        try await fill(nil, text, id: "quick-capture-idea")
        try await vaultSheetPress("Save")
        try await waitGone(id: "quick-capture-idea", timeout: 6)
        await idle(0.5)
    }

    /// The three ideas every scenario starts from.
    func vaultSeedIdeas() async throws {
        for t in vaultIdeas { try await vaultQuickCapture(t) }
        await vaultQuiet()
    }

    /// Presses a button of the open sheet by its label.
    func vaultSheetPress(_ label: String) async throws {
        try await capWaitUntil("a sheet", timeout: 6) { hasOverlay }
        guard let sh = NSApp.windows.last(where: { $0.isVisible && $0.sheetParent != nil }) else { throw DriverError("no sheet") }
        guard let b = windowElements(sh).first(where: { $0.role == "AXButton" && ($0.label == label || $0.title == label) }) else {
            throw DriverError("no button \(label) in the sheet")
        }
        guard b.isEnabled else { throw DriverError("button \(label) is disabled") }
        if !b.performPress() { try await tap(b) }
        await idle(0.6)
    }

    func vaultOpen(_ text: String) async throws {
        let c = try await vaultCardOrThrow(text)
        try await capCustomAction(c, "Open")
        _ = try await wait(id: "vault-drawer", timeout: 6)
        await idle(0.6)
    }

    func vaultCloseDrawer() async {
        _ = await tryPress(id: "drawer-close", wait: 1.5)
        await idle(0.4)
    }

    /// Shows a view of the vault's left panel (All items, Pinned, a folder, a collection, a tag).
    func vaultNav(_ title: String) async throws {
        let e = try await wait(id: "vault-nav-\(title)", timeout: 6)
        try await tap(e)
        await idle(0.6)
        // A click right after a sheet closes can be lost: select the row in the list itself too.
        if let outline = query(id: "vault-sidebar").first?.obj as? NSOutlineView, let w = outline.window {
            let p = outline.convert(w.convertPoint(fromScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)), from: nil)
            let row = outline.row(at: p)
            if row >= 0 {
                // The list can believe a row is chosen while the vault shows another view: go through a
                // neighbour first so the change is seen.
                if outline.selectedRow == row {
                    outline.selectRowIndexes(IndexSet(integer: row == 1 ? 2 : 1), byExtendingSelection: false)
                    await idle(0.5)
                }
                outline.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
            }
        }
        await idle(0.6)
    }

    /// Chooses an item of an element's right-click menu (submenus included).
    func vaultContextPick(_ e: Element, _ title: String) async throws {
        guard let m = capContextMenuItems(at: NSPoint(x: e.frame.midX, y: e.frame.midY)) else { throw DriverError("no context menu on \(e)") }
        func find(_ m: NSMenu) -> (NSMenu, Int)? {
            for (i, it) in m.items.enumerated() where !it.isSeparatorItem && it.title.hasPrefix(title) { return (m, i) }
            for it in m.items { if let sub = it.submenu, let r = find(sub) { return r } }
            return nil
        }
        guard let (menu, i) = find(m) else { throw DriverError("no item \(title) in \(m.items.map(\.title))") }
        guard menu.items[i].isEnabled else { throw DriverError("item \(title) is disabled") }
        menu.performActionForItem(at: i)
        await idle(0.8)
    }

    /// The search field the vault puts in the window toolbar (a button until it is expanded).
    func vaultSearchField() -> Element? {
        all().first { $0.subrole == "AXSearchField" && $0.role == "AXTextField" && $0.frame.width > 0 }
    }

    func vaultSearch(_ text: String) async throws {
        if vaultSearchField() == nil {
            let b = all().last { $0.subrole == "AXSearchField" && $0.frame.width > 0 } ?? query(label: "Search", role: "AXButton").last { $0.frame.width > 0 && $0.identifier.isEmpty }
            guard let b else { throw DriverError("no search button") }
            if !b.performPress() { capPostClick(atScreen: NSPoint(x: b.frame.midX, y: b.frame.midY)) }
            try await capWaitUntil("the search field", timeout: 4) { vaultSearchField() != nil }
        }
        guard let e = vaultSearchField() else { throw DriverError("no search field") }
        guard let tf = (e.obj as? NSTextField) ?? ((e.obj as? NSCell)?.controlView as? NSTextField), let w = tf.window else {
            throw DriverError("the search field is not a text field: \(e.description) \(Swift.type(of: e.obj))")
        }
        var target: NSTextView?
        for _ in 0..<10 where target == nil {
            w.makeKeyAndOrderFront(nil)
            w.makeFirstResponder(tf)
            await idle(0.2)
            target = w.firstResponder as? NSTextView
        }
        guard let tv = target else { throw DriverError("the search field took no focus") }
        tv.selectAll(nil)
        tv.insertText(text, replacementRange: tv.selectedRange())
        await idle(1.2)
    }

    /// Captures a sheet opened by `open`, then cancels it.
    func vaultSheet(_ name: String, _ title: String, from: String?, via: String, _ open: () async throws -> Void) async {
        await step("sheet \(name)") {
            try await open()
            try await capWaitUntil("the sheet \(name)", timeout: 6) { hasOverlay }
            await capture(name, title: title, area: vaultArea, from: from, via: via)
            await capRecover()
        }
    }
}

extension Scenarios {
    static let captureVault: [Scenario] = [
        // Adding items, the add menu and its sheets, the view modes, the item inspector and its sheets.
        tour("04-vault-a", order: 4000, importSample: false) { ui, _ in
            try await ui.capNav("Idea Vault")
            try await ui.wait(id: "vault-items", timeout: 15)
            await ui.capture("vault-empty", title: "Idea Vault, empty", area: vaultArea)
            await ui.step("trash empty") {
                try await ui.capAct(id: "vault-recently-deleted")
                _ = try await ui.wait(label: "Back to Vault", role: "AXButton", timeout: 6)
                await ui.capture("vault-recently-deleted-empty", title: "Recently Deleted, nothing deleted", area: vaultArea, from: "vault-empty", via: "vault-recently-deleted")
                try await ui.capAct(label: "Back to Vault", role: "AXButton")
                await ui.idle(0.6)
            }
            await ui.exploreMenu("vault-add", area: vaultArea, prefix: "vault-add", from: "vault-empty")
            await ui.vaultQuiet()
            await ui.step("quick capture typed") {
                try await ui.capAct(id: "vault-quick-capture")
                _ = try await ui.wait(id: "quick-capture-idea", timeout: 6)
                try await ui.fill(nil, vaultIdeas[0], id: "quick-capture-idea")
                await ui.capture("vault-quick-capture-typed", title: "Quick Capture with an idea typed", area: vaultArea, from: "vault-add-quick-capture", via: "quick-capture-idea")
                try await ui.vaultSheetPress("Save")
                await ui.idle(0.6)
                await ui.capture("vault-saved-notice", title: "Idea Vault, saved notice after Quick Capture", area: vaultArea, from: "vault-quick-capture-typed", via: "Save")
            }
            for t in vaultIdeas.dropFirst() { await ui.step("quick capture \(t)") { try await ui.vaultQuickCapture(t) } }
            await ui.vaultQuiet()
            await ui.step("add url") {
                try await ui.capMenuButton(id: "vault-add", item: "URL")
                _ = try await ui.wait(id: "add-url-link", timeout: 6)
                try await ui.fill(nil, "night bus timetable", id: "add-url-link")
                try await ui.vaultSheetPress("Add URL")
                if ui.exists(id: "sheet-error") {
                    await ui.capture("vault-add-url-error", title: "Add URL, the link is not valid", area: vaultArea, from: "vault-add-url", via: "Add URL")
                }
                try await ui.fill(nil, "https://example.com/night-bus-routes", id: "add-url-link")
                try await ui.fill(nil, "The real timetable for the 3 a.m. route.", id: "add-url-note")
                await ui.capture("vault-add-url-filled", title: "Add URL, link and note filled", area: vaultArea, from: "vault-add-url", via: "add-url-link")
                try await ui.vaultSheetPress("Add URL")
                try await ui.waitGone(id: "add-url-link", timeout: 6)
            }
            await ui.step("add quote") {
                try await ui.capMenuButton(id: "vault-add", item: "Quote")
                _ = try await ui.wait(id: "drawer-body", timeout: 6)
                try await ui.fill(nil, "The last bus waits for no one.", id: "drawer-body")
                await ui.idle(1.0)
                await ui.capture("vault-drawer-quote", title: "Idea Vault, a quote open in the inspector", area: vaultArea, from: "vault-add-menu", via: "Quote")
                await ui.vaultCloseDrawer()
            }
            await ui.vaultQuiet()
            await ui.capture("vault-filled", title: "Idea Vault with ideas, a link and a quote", area: vaultArea, from: "vault-add-menu")
            await ui.eachTab(group: "vault-view-mode", area: vaultArea, prefix: "vault-view", from: "vault-filled")
            await ui.step("note drawer") {
                try await ui.vaultOpen(vaultIdeas[0])
                await ui.capture("vault-drawer-note", title: "Idea Vault, a note open in the inspector", area: vaultArea, from: "vault-filled", via: "vault-card")
            }
            await ui.capMenu(id: "drawer-research-flag", name: "vault-drawer-flag-menu", title: "Research flag choices", area: vaultArea, from: "vault-drawer-note")
            await ui.step("flag and tag") {
                if !ui.exists(id: "vault-drawer") { try await ui.vaultOpen(vaultIdeas[0]) }
                try await ui.choose("drawer-research-flag", "Needs Expert Check")
                await ui.idle(0.6)
                try await ui.fill(nil, "night", id: "drawer-add-tag", submit: true)
                await ui.idle(0.8)
                await ui.vaultQuiet()
                await ui.capture("vault-drawer-flag-and-tag", title: "Inspector with a research flag and a tag", area: vaultArea, from: "vault-drawer-note", via: "drawer-research-flag")
            }
            for (label, name, title) in [("Move", "vault-move-sheet", "Move to Folder"), ("Copy to Global Vault", "vault-copy-to-global-sheet", "Copy to Global Vault"),
                                         ("Send to Story", "vault-send-to-story-sheet", "Send to Beat Board")] {
                await ui.vaultSheet(name, title, from: "vault-drawer-note", via: label) {
                    if !ui.exists(id: "vault-drawer") { try await ui.vaultOpen(vaultIdeas[0]) }
                    guard let b = ui.query(label: label).first(where: { $0.role == "AXButton" && $0.frame.width > 0 }) ?? ui.query(label: label).first(where: { $0.role == "AXButton" }) else { throw DriverError("no button \(label)") }
                    try await ui.press(b)
                }
            }
            await ui.vaultCloseDrawer()
            await ui.step("url drawer") {
                try await ui.vaultOpen("example.com")
                await ui.capture("vault-drawer-url", title: "Idea Vault, a link open in the inspector", area: vaultArea, from: "vault-filled", via: "vault-card")
                await ui.vaultCloseDrawer()
            }
            await ui.step("select one") {
                let c = try await ui.vaultCardOrThrow(vaultIdeas[1])
                try await ui.tap(c)
                try await ui.capWaitUntil("the selection bar") { ui.exists(label: "Selected items") }
                await ui.capture("vault-item-selected", title: "Idea Vault, one item selected", area: vaultArea, from: "vault-filled", via: "vault-card")
            }
            await ui.vaultSheet("vault-tag-sheet", "Tag items", from: "vault-item-selected", via: "Tag") {
                guard let bar = ui.query(label: "Selected items").first, let b = bar.children.first(where: { $0.label == "Tag" || $0.title == "Tag" }) ?? ui.query(label: "Tag", role: "AXButton").first else { throw DriverError("no Tag button") }
                try await ui.press(b)
            }
            _ = await ui.tryPress(label: "Clear selection", wait: 1.5)
            await ui.step("card menu") {
                let c = try await ui.vaultCardOrThrow(vaultIdeas[2])
                await ui.capContext(on: c, name: "vault-card-context-menu", title: "Right-click menu of an item", area: vaultArea, from: "vault-filled")
            }
            await ui.step("flag submenu") {
                let c = try await ui.vaultCardOrThrow(vaultIdeas[2])
                guard let m = ui.capContextMenuItems(at: NSPoint(x: c.frame.midX, y: c.frame.midY)),
                      let sub = m.items.first(where: { $0.title == "Research flag" })?.submenu, let r = ui.relFrame(c) else { throw DriverError("no Research flag submenu") }
                await ui.capture("vault-card-flag-submenu", title: "Research flag choices in the right-click menu", area: vaultArea, from: "vault-card-context-menu",
                                 via: "Research flag", menu: sub.copy() as? NSMenu ?? sub, anchor: CGPoint(x: max(20, r.minX - 40), y: r.midY))
            }
        },

        // Pins, tags, folders, collections, the smart views and search.
        tour("04-vault-b", order: 4200, importSample: false) { ui, _ in
            try await ui.capNav("Idea Vault")
            try await ui.wait(id: "vault-items", timeout: 15)
            try await ui.vaultSeedIdeas()
            await ui.step("pinned empty") {
                try await ui.vaultNav("Pinned")
                await ui.vaultQuiet()
                await ui.capture("vault-pinned-empty", title: "Pinned, nothing pinned yet", area: vaultArea, from: "vault-filled", via: "vault-nav-Pinned")
                try await ui.vaultNav("All items")
            }
            await ui.step("pin") {
                try await ui.capCustomAction(try await ui.vaultCardOrThrow(vaultIdeas[1]), "Pin")
                await ui.idle(0.8)
                await ui.capture("vault-pinned-notice", title: "Idea Vault, a pinned item at the top", area: vaultArea, from: "vault-card-context-menu", via: "Pin")
                try await ui.vaultNav("Pinned")
                await ui.vaultQuiet()
                await ui.capture("vault-pinned", title: "Pinned items", area: vaultArea, from: "vault-pinned-notice", via: "vault-nav-Pinned")
                try await ui.vaultNav("All items")
            }
            await ui.step("tag") {
                try await ui.vaultContextPick(try await ui.vaultCardOrThrow(vaultIdeas[1]), "Tag")
                _ = try await ui.wait(label: "Tags", timeout: 6)
                try await ui.fill("Tags", "rain, depot")
                await ui.capture("vault-tag-sheet-filled", title: "Tag items, two tags typed", area: vaultArea, from: "vault-card-context-menu", via: "Tag")
                try await ui.vaultSheetPress("Add Tags")
                await ui.vaultQuiet()
                try await ui.vaultNav("rain")
                await ui.capture("vault-tag-view", title: "Items with the tag rain", area: vaultArea, from: "vault-tag-sheet-filled", via: "vault-nav-rain")
                try await ui.vaultNav("All items")
            }
            await ui.step("new folder") {
                try await ui.capAct(id: "vault-new-folder")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                await ui.capture("vault-new-folder-sheet", title: "New folder", area: vaultArea, from: "vault-filled", via: "vault-new-folder")
                try? await ui.vaultSheetPress("Create")
                if ui.exists(id: "sheet-error") {
                    await ui.capture("vault-new-folder-error", title: "New folder, the name is missing", area: vaultArea, from: "vault-new-folder-sheet", via: "Create")
                }
                if ui.exists(id: "name-sheet-field") { try await ui.fill(nil, "Night scenes", id: "name-sheet-field", submit: true) }
                try await ui.waitGone(id: "name-sheet-field", timeout: 6)
                await ui.vaultQuiet()
            }
            await ui.step("folder empty") {
                try await ui.vaultNav("Night scenes")
                await ui.capture("vault-folder-empty", title: "A folder with nothing in it", area: vaultArea, from: "vault-new-folder-sheet", via: "vault-nav-Night scenes")
                try await ui.vaultNav("All items")
            }
            await ui.step("move to folder") {
                try await ui.vaultContextPick(try await ui.vaultCardOrThrow(vaultIdeas[0]), "Move")
                _ = try await ui.wait(id: "move-folder", timeout: 6)
                try await ui.capAct(label: "Night scenes", role: "AXRadioButton")
                await ui.capture("vault-move-sheet-chosen", title: "Move to Folder, a folder chosen", area: vaultArea, from: "vault-card-context-menu", via: "Move")
                try await ui.vaultSheetPress("Move")
                await ui.vaultQuiet()
                try await ui.vaultNav("Night scenes")
                await ui.capture("vault-folder-with-item", title: "A folder with an item", area: vaultArea, from: "vault-move-sheet-chosen", via: "vault-nav-Night scenes")
            }
            await ui.step("folder menu") {
                let row = try await ui.wait(id: "vault-nav-Night scenes", timeout: 6)
                await ui.capContext(on: row, name: "vault-folder-context-menu", title: "Right-click menu of a folder", area: vaultArea, from: "vault-folder-with-item")
            }
            await ui.vaultSheet("vault-rename-folder-sheet", "Rename folder", from: "vault-folder-context-menu", via: "Rename") {
                try await ui.vaultContextPick(try await ui.wait(id: "vault-nav-Night scenes", timeout: 6), "Rename")
            }
            await ui.step("subfolder") {
                try await ui.vaultContextPick(try await ui.wait(id: "vault-nav-Night scenes", timeout: 6), "New Folder Inside")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                try await ui.fill(nil, "Depot", id: "name-sheet-field")
                await ui.capture("vault-new-subfolder-sheet", title: "New folder inside a folder", area: vaultArea, from: "vault-folder-context-menu", via: "New Folder Inside")
                try await ui.vaultSheetPress("Create")
                await ui.vaultQuiet()
                try await ui.vaultNav("All items")
            }
            await ui.step("folder mode") {
                try await ui.capTab("Folder")
                await ui.capture("vault-folder-mode-top", title: "Folder view, the top level", area: vaultArea, from: "vault-view-folder", via: "Folder")
                let tile = try await ui.wait(id: "vault-folder-Night scenes", timeout: 6)
                try await ui.press(tile)
                await ui.idle(0.8)
                await ui.capture("vault-folder-mode-inside", title: "Folder view, inside a folder with a subfolder", area: vaultArea, from: "vault-folder-mode-top", via: "vault-folder-Night scenes")
                try await ui.capTab("Grid")
                try await ui.vaultNav("All items")
            }
            for (row, name) in [("Recently added", "vault-recently-added"), ("Recently modified", "vault-recently-modified")] {
                await ui.step(name) {
                    try await ui.vaultNav(row)
                    await ui.capture(name, title: "Idea Vault: \(row)", area: vaultArea, from: "vault-filled", via: "vault-nav-\(row)")
                }
            }
            await ui.step("search") {
                try await ui.vaultNav("All items")
                try await ui.vaultSearch("bus")
                await ui.capture("vault-search-results", title: "Idea Vault search with results", area: vaultArea, from: "vault-filled", via: "search")
                try await ui.vaultSearch("zebra crossing")
                await ui.capture("vault-search-none", title: "Idea Vault search with no results", area: vaultArea, from: "vault-search-results", via: "search")
                try await ui.vaultSearch("")
            }
            await ui.step("new collection") {
                try await ui.capAct(id: "vault-new-collection")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                try await ui.fill(nil, "Look and feel", id: "name-sheet-field")
                await ui.capture("vault-new-collection-sheet", title: "New collection", area: vaultArea, from: "vault-filled", via: "vault-new-collection")
                try await ui.vaultSheetPress("Create")
                await ui.idle(0.8)
                await ui.vaultQuiet()
                await ui.capture("vault-collection-empty", title: "A collection with nothing in it", area: vaultArea, from: "vault-new-collection-sheet", via: "Create")
            }
            await ui.step("collection item") {
                // Quick Capture adds to the collection that is open.
                try await ui.vaultQuickCapture("Sodium lamps, orange on wet tarmac")
                await ui.vaultQuiet()
                await ui.capture("vault-collection-with-item", title: "A collection with an item", area: vaultArea, from: "vault-collection-empty", via: "vault-quick-capture")
                try await ui.vaultContextPick(try await ui.vaultCardOrThrow("Sodium lamps"), "Add to Collection")
                try await ui.capWaitUntil("the sheet") { ui.hasOverlay }
                await ui.capture("vault-add-to-collection-sheet", title: "Add to Collection", area: vaultArea, from: "vault-card-context-menu", via: "Add to Collection")
                await ui.capRecover()
                let row = try await ui.wait(id: "vault-nav-Look and feel", timeout: 6)
                await ui.capContext(on: row, name: "vault-collection-context-menu", title: "Right-click menu of a collection", area: vaultArea, from: "vault-collection-with-item")
            }
            await ui.vaultSheet("vault-rename-collection-sheet", "Rename collection", from: "vault-collection-context-menu", via: "Rename") {
                try await ui.vaultContextPick(try await ui.wait(id: "vault-nav-Look and feel", timeout: 6), "Rename")
            }
        },

        // Deleting, Recently Deleted, restoring, and the Global Idea Vault.
        tour("04-vault-c", order: 4400, importSample: false) { ui, _ in
            try await ui.capNav("Idea Vault")
            try await ui.wait(id: "vault-items", timeout: 15)
            try await ui.vaultSeedIdeas()
            await ui.step("delete") {
                try await ui.capCustomAction(try await ui.vaultCardOrThrow(vaultIdeas[2]), "Delete")
                try await ui.capWaitUntil("the notice", timeout: 6) { ui.exists(label: "Dismiss") }
                await ui.capture("vault-deleted-notice", title: "Idea Vault, an item deleted with Undo offered", area: vaultArea, from: "vault-card-context-menu", via: "Delete")
                await ui.vaultQuiet()
            }
            await ui.step("trash list") {
                try await ui.capAct(id: "vault-recently-deleted")
                _ = try await ui.wait(label: "Back to Vault", role: "AXButton", timeout: 6)
                await ui.idle(0.6)
                await ui.capture("vault-recently-deleted", title: "Recently Deleted with an item", area: vaultArea, from: "vault-deleted-notice", via: "vault-recently-deleted")
                guard let b = ui.query(label: "Delete Permanently").first(where: { $0.role == "AXButton" && $0.frame.width > 0 }) else { throw DriverError("no Delete Permanently") }
                try await ui.press(b)
                try await ui.capWaitUntil("the confirmation", timeout: 6) { ui.hasOverlay || ui.exists(label: "Keep in Recently Deleted") }
                await ui.capture("vault-purge-confirm", title: "Delete permanently, confirmation", area: vaultArea, from: "vault-recently-deleted", via: "Delete Permanently")
                _ = await ui.tryPress(label: "Keep in Recently Deleted", wait: 2)
                await ui.capRecover()
                let restore = try await ui.wait(label: "Restore", role: "AXButton", timeout: 6)
                try await ui.press(restore)
                await ui.idle(0.8)
                try? await ui.capAct(label: "Back to Vault", role: "AXButton")
                await ui.idle(0.6)
                await ui.capture("vault-restored", title: "Idea Vault after restoring an item", area: vaultArea, from: "vault-recently-deleted", via: "Restore")
                await ui.vaultQuiet()
            }
            await ui.step("global empty") {
                try await ui.capTab("Global")
                try await ui.wait(id: "vault-items", timeout: 10)
                await ui.idle(0.8)
                await ui.capture("vault-global-empty", title: "Global Idea Vault, empty", area: vaultArea, from: "vault-filled", via: "Global")
                try await ui.capTab("This Project")
                await ui.idle(0.8)
            }
            await ui.step("copy to global") {
                try await ui.vaultContextPick(try await ui.vaultCardOrThrow(vaultIdeas[0]), "Copy to Global")
                try await ui.vaultSheetPress("Copy to Global Vault")
                await ui.idle(0.6)
                await ui.vaultQuiet()
                try await ui.capTab("Global")
                await ui.idle(1.0)
                await ui.capture("vault-global-with-copy", title: "Global Idea Vault with a copied idea", area: vaultArea, from: "vault-copy-to-global-sheet", via: "Copy to Global Vault", settle: 2.0)
                try await ui.vaultOpen(vaultIdeas[0])
                await ui.capture("vault-global-drawer", title: "Global Idea Vault, an idea open in the inspector", area: vaultArea, from: "vault-global-with-copy", via: "vault-card")
                await ui.vaultCloseDrawer()
                if let c = ui.vaultCard(vaultIdeas[0]) {
                    await ui.capContext(on: c, name: "vault-global-card-context-menu", title: "Right-click menu of a Global idea", area: vaultArea, from: "vault-global-with-copy")
                }
                try await ui.capTab("This Project")
            }
        },
    ]
}

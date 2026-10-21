// Capture tour section: Story (Story page, Beat Board, Outline, Characters, Timeline). Primitives and helpers are in CaptureTour.swift.
// Two scenarios: 03-story covers the Story page, the Beat Board and the Outline on a project without a screenplay;
// 03-story-people covers Characters and the Timeline on a project with the example screenplay.

import AppKit
import Foundation

fileprivate let storyArea = "Story"

fileprivate extension UI {
    /// The largest element whose identifier starts with `prefix` (and whose text holds `text`).
    func storyEl(_ prefix: String, _ text: String? = nil, exclude: [String] = []) -> Element? {
        let c = all().filter { e in
            e.identifier.hasPrefix(prefix) && !exclude.contains(where: { e.identifier.hasPrefix($0) }) && e.frame.width > 0
                && (text == nil || e.text.localizedCaseInsensitiveContains(text!))
        }
        return c.max { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height }
    }

    /// The act columns on the board, left to right.
    func storyActs() -> [Element] {
        var best: [String: Element] = [:]
        for e in all() where e.identifier.hasPrefix("story-act-") && !e.identifier.hasPrefix("story-act-options-") && e.identifier != "story-add-act" && e.frame.width > 0 {
            if let b = best[e.identifier], b.frame.width * b.frame.height >= e.frame.width * e.frame.height { continue }
            best[e.identifier] = e
        }
        return best.values.sorted { $0.frame.minX < $1.frame.minX }
    }

    /// A point on an act column's header (screen coordinates).
    func storyActHeader(_ a: Element) -> NSPoint { NSPoint(x: a.frame.minX + 90, y: a.frame.maxY - 20) }

    func storyContext(_ e: Element, at p: NSPoint? = nil, name: String, title: String, from: String?) async {
        await step("context menu \(name)") {
            let f = e.frame
            let pt = p ?? NSPoint(x: f.midX, y: f.midY)
            guard let menu = capContextMenuItems(at: pt), let w = capturePrimary() else { throw DriverError("no context menu on \(e)") }
            let anchor = CGPoint(x: pt.x - w.frame.minX, y: w.frame.maxY - pt.y)
            await capture(name, title: title, area: storyArea, from: from, via: e.identifier, menu: menu.copy() as? NSMenu ?? menu, anchor: anchor)
        }
    }

    /// Chooses an item of an element's context menu, as a right click and a choice would.
    func storyContextChoose(_ e: Element, at p: NSPoint? = nil, _ item: String) async throws {
        let f = e.frame
        let pt = p ?? NSPoint(x: f.midX, y: f.midY)
        guard let menu = capContextMenuItems(at: pt) else { throw DriverError("no context menu on \(e)") }
        guard let i = menu.items.firstIndex(where: { !$0.isSeparatorItem && $0.title.hasPrefix(item) }) else {
            throw DriverError("no item \(item) in \(menu.items.map(\.title))")
        }
        guard menu.items[i].isEnabled else { throw DriverError("menu item is disabled: \(item)") }
        menu.performActionForItem(at: i)
        await idle(0.7)
    }

    /// Chooses an item of the n-th pop-up button (left to right) of the open sheet.
    func storyPick(index: Int, item: String) async throws {
        guard let sheet = NSApp.windows.last(where: { $0.isVisible && $0.sheetParent != nil }) else { throw DriverError("no sheet") }
        var seen = Set<ObjectIdentifier>()
        let cells = windowElements(sheet).filter { $0.obj is NSPopUpButtonCell && seen.insert(ObjectIdentifier($0.obj)).inserted }
            .sorted { $0.frame.minX < $1.frame.minX }
        guard index < cells.count, let cell = cells[index].obj as? NSPopUpButtonCell, let pb = cell.controlView as? NSPopUpButton else {
            throw DriverError("no pop-up \(index) in the sheet (\(cells.count) found)")
        }
        guard let i = pb.itemTitles.firstIndex(where: { $0.hasPrefix(item) }) else { throw DriverError("no item \(item) in \(pb.itemTitles)") }
        pb.selectItem(at: i)
        _ = pb.sendAction(pb.action, to: pb.target)
        await idle(0.4)
    }

    /// Captures the menu of the n-th pop-up button (left to right) of the open sheet, drawn open.
    func storySheetMenu(index: Int, name: String, title: String, from: String?) async {
        await step("sheet menu \(name)") {
            guard let sheet = NSApp.windows.last(where: { $0.isVisible && $0.sheetParent != nil }) else { throw DriverError("no sheet") }
            var seen = Set<ObjectIdentifier>()
            let cells = windowElements(sheet).filter { $0.obj is NSPopUpButtonCell && seen.insert(ObjectIdentifier($0.obj)).inserted }
                .sorted { $0.frame.minX < $1.frame.minX }
            guard index < cells.count, let r = relFrame(cells[index]) else { throw DriverError("no pop-up \(index) in the sheet") }
            let menu = try await peekMenu(cells[index])
            await capture(name, title: title, area: storyArea, from: from, via: cells[index].label, menu: menu, anchor: CGPoint(x: r.minX, y: r.maxY + 4))
        }
    }

    /// The More menu of the open character (its label is hidden by the detail pane's own label).
    func storyMorePopup() throws -> Element {
        guard let w = capturePrimary() else { throw DriverError("no window") }
        let mid = w.frame.minX + w.frame.width * 0.45
        guard let e = all().filter({ $0.obj is NSPopUpButtonCell && $0.frame.minX > mid }).max(by: { $0.frame.maxY < $1.frame.maxY }) else { throw DriverError("no More menu") }
        return e
    }

    func storyMoreChoose(_ item: String) async throws {
        let e = try storyMorePopup()
        guard let cell = e.obj as? NSPopUpButtonCell else { throw DriverError("not a menu") }
        capPostClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY))
        var done = false
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline, !done {
            if let m = cell.menu, let i = m.items.firstIndex(where: { $0.title.hasPrefix(item) }) { m.performActionForItem(at: i); done = true }
            await idle(0.1)
        }
        cell.menu?.cancelTracking()
        await idle(0.5)
        for w in NSApp.windows where w.isVisible && String(describing: Swift.type(of: w)).contains("PopupMenu") { w.orderOut(nil) }
        if !done { throw DriverError("no item \(item) in the More menu") }
    }

    /// The map's two buttons (Back, Add Relationship), left to right; the map's own label hides theirs.
    func storyMapButtons() -> [Element] {
        var seen = Set<ObjectIdentifier>()
        return all().filter { $0.role == "AXButton" && ($0.label == "Character relationships" || $0.title == "Back to Characters" || $0.title == "Add Relationship") && seen.insert(ObjectIdentifier($0.obj)).inserted }
            .sorted { $0.frame.minX < $1.frame.minX }
    }

    func storyMapBack() async throws {
        guard let b = storyMapButtons().first else { throw DriverError("no Back to Characters") }
        try await press(b)
        await idle(0.6)
    }

    func storyAssignDay() async throws {
        guard let b = all().first(where: { $0.role == "AXButton" && $0.label.hasPrefix("Assign Story Day") }) else { throw DriverError("no Assign Story Day button") }
        try await press(b)
    }

    func storyOpen(_ e: Element?) async throws {
        guard let e else { throw DriverError("nothing to open") }
        try await capCustomAction(e, "Open")
        try await wait(id: "story-inspector-close", timeout: 6)
        await idle(0.4)
    }

    func storyCloseInspector() async {
        _ = await tryPress(id: "story-inspector-close", wait: 1.5)
        await idle(0.4)
    }

    func storyCharacter(_ name: String) async throws {
        let rows = all().filter { $0.text.localizedCaseInsensitiveContains(name) && $0.frame.width > 150 && $0.frame.width < 420 && $0.frame.height < 80 }
        guard let e = rows.max(by: { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height }) else {
            throw DriverError("no character row \(name)")
        }
        try await tap(e)
        await idle(0.8)
    }

    func storyNewCharacter(_ name: String, _ role: String) async throws {
        try await capAct(label: "Add Character", role: "AXButton")
        try await fill("Character name", name)
        try await fill("Role", role)
        try await capAct(label: "Create Character", role: "AXButton")
        await idle(0.8)
    }

    /// Fills the open Add Relationship sheet and adds it.
    func storyRelationship(from: String, to: String, type: String) async throws {
        try await storyPick(index: 0, item: from.uppercased())
        try await storyPick(index: 2, item: to.uppercased())
        try await fill("Relationship", type)
        try await capAct(label: "Add Relationship", role: "AXButton")
        await idle(0.8)
    }

    func storySetZoom(_ percent: Int) async {
        for _ in 0..<6 {
            guard let z = all().first(where: { $0.label.hasPrefix("Zoom ") && $0.label.hasSuffix(" percent") }) else { return }
            let now = Int(z.label.dropFirst(5).dropLast(8)) ?? percent
            if now == percent { return }
            _ = await tryPress(label: now > percent ? "Zoom out" : "Zoom in", wait: 1)
            await idle(0.3)
        }
    }
}

extension Scenarios {
    static let captureStory: [Scenario] = [
        // The Story page, the Beat Board and the Outline, on a project with no screenplay yet.
        Scenario("zz-capture/03-story", timeout: 600) { ui, _, ctx in
            let A = storyArea
            try await ui.capSeed(ctx, order: 3000, importSample: false)
            try await ui.capNav("Story")
            try await ui.wait(id: "story-tabs", timeout: 20)
            await ui.storySetZoom(100)
            await ui.capture("story-board-empty", title: "Beat Board, nothing on it yet", area: A)
            await ui.capMenu(id: "story-add-menu", name: "story-add-menu", title: "Beat Board, the Add menu", area: A, from: "story-board-empty")
            await ui.step("timeline empty") {
                try await ui.capTab("Timeline")
                await ui.capture("story-timeline-empty", title: "Timeline, no screenplay scenes yet", area: A, from: "story-board-empty", via: "story-tabs")
            }

            // The free Story page, left of the Beat Board.
            await ui.step("story page") {
                try await ui.capTab("Story")
                try await ui.wait(id: "story-page-text", timeout: 10)
                await ui.capture("story-page-empty", title: "Story page, empty", area: A, from: "story-board-empty", via: "story-tabs")
                try await ui.capTypeIn("Meera works the night shift at the city hospital. Every night she takes the last bus home.\n\nOne night the bus does not come, and the driver of the next one will not wait. She walks, and on the way she finds a lost bag with a ticket for a bus that left ten years ago.\n\nBy dawn at the depot she knows who it belongs to.", into: "story-page-text")
                await ui.idle(2.5)
                await ui.capture("story-page-written", title: "Story page with text, saved", area: A, from: "story-page-empty")
            }
            await ui.step("story page export") {
                try await ui.capAct(id: "story-page-export")
                try await ui.wait(id: "export-sheet", timeout: 6)
                await ui.capture("story-page-export-sheet", title: "Export the Story page", area: A, from: "story-page-written", via: "story-page-export")
                try await ui.capAct(label: "Word (DOCX)", role: "AXRadioButton")
                await ui.capture("story-page-export-docx", title: "Export the Story page as a Word document", area: A, from: "story-page-export-sheet", via: "export-format")
                _ = await ui.tryPress(id: "export-cancel", wait: 1.5)
                await ui.capRecover()
            }

            // The Beat Board: acts, a sequence, scene cards, beats and a parked card.
            try await ui.capTab("Beat Board")
            await ui.step("add act") {
                try await ui.capAct(label: "Add Act", role: "AXButton")
                await ui.capture("story-add-act-inline", title: "Beat Board, typing the name of a new act", area: A, from: "story-board-empty", via: "story-add-act")
                try await ui.fill("Act title", "Act One: The last bus", submit: true)
                await ui.idle(0.8)
                await ui.capture("story-board-one-act", title: "Beat Board with one empty act", area: A, from: "story-add-act-inline")
            }
            await ui.step("add sequence") {
                try await ui.capAct(label: "Add Sequence", role: "AXButton")
                await ui.capture("story-add-sequence-inline", title: "Beat Board, typing the name of a new sequence", area: A, from: "story-board-one-act", via: "Add Sequence")
                try await ui.fill("Sequence name", "Missing the bus", submit: true)
                await ui.idle(0.8)
                await ui.capture("story-sequence-empty", title: "Beat Board, an act with an empty sequence", area: A, from: "story-add-sequence-inline")
            }
            let cards = ["Meera misses the last bus", "The driver will not wait", "A lost bag on the back seat", "The ticket inspector boards"]
            for (n, c) in cards.enumerated() {
                await ui.step("add card \(n)") {
                    if n == 0 { try await ui.capAct(label: "Add scene", role: "AXButton") } else { try await ui.capMenuButton(id: "story-add-menu", item: "Scene Card") }
                    if n == 0 { await ui.capture("story-add-card-inline", title: "Beat Board, writing a new scene card", area: A, from: "story-sequence-empty", via: "Add scene") }
                    try await ui.fill("Short description", c, submit: true)
                    await ui.idle(0.6)
                }
            }
            let beats = ["Meera decides to walk", "She opens the bag"]
            for (n, b) in beats.enumerated() {
                await ui.step("add beat \(n)") {
                    try await ui.capMenuButton(id: "story-add-menu", item: "Beat")
                    if n == 0 { await ui.capture("story-add-beat-inline", title: "Beat Board, writing a new beat", area: A, from: "story-board-one-act", via: "story-add-menu") }
                    try await ui.fill("Beat text", b, submit: true)
                    await ui.idle(0.6)
                }
            }
            await ui.step("second act") {
                try await ui.capAct(label: "Add Act", role: "AXButton")
                try await ui.fill("Act title", "Act Two: The depot", submit: true)
                await ui.idle(0.8)
                guard let add = ui.query(label: "Add Scene", role: "AXButton").max(by: { $0.frame.minX < $1.frame.minX }) else { throw DriverError("no Add Scene in Act Two") }
                try await ui.press(add)
                try await ui.fill("Short description", "Dawn at the depot", submit: true)
                await ui.idle(0.6)
            }
            await ui.step("parked card") {
                try await ui.capAct(label: "Add Scene to the Parking Lot")
                try await ui.fill("Short description", "A dream about the sea", submit: true)
                await ui.idle(0.8)
            }
            await ui.step("clear selection") { try await ui.escape(); await ui.idle(0.4) }
            await ui.capture("story-board-filled", title: "Beat Board with acts, a sequence, scene cards, beats and a parked card", area: A, from: "story-board-one-act")

            // Selection.
            await ui.step("select card") {
                guard let c = ui.capCard(cards[1]) else { throw DriverError("no card") }
                try await ui.tap(c)
                await ui.capture("story-card-selected", title: "Beat Board, a scene card selected", area: A, from: "story-board-filled", via: "story-card")
                try await ui.key("\u{F701}", code: 125, modifiers: [.shift])
                await ui.idle(0.6)
                if !ui.exists(label: "Selection actions") {
                    try await ui.key("\u{F701}", code: 125, modifiers: [.shift])
                    await ui.idle(0.6)
                }
                try await ui.wait(label: "Selection actions", timeout: 3)
                await ui.capture("story-multi-select", title: "Beat Board, two cards selected with the selection bar", area: A, from: "story-card-selected", via: "Shift Down Arrow")
                try await ui.capAct(label: "Move\u{2026}", role: "AXButton")
                try await ui.capWaitUntil("move sheet") { ui.hasOverlay }
                await ui.capture("story-move-sheet", title: "Move the selected cards", area: A, from: "story-multi-select", via: "Move")
                await ui.capRecover()
                _ = await ui.tryPress(label: "Clear selection", wait: 1.5)
            }

            // Context menus.
            if let c = ui.capCard(cards[0]) { await ui.storyContext(c, name: "story-card-context-menu", title: "Scene card, the context menu", from: "story-board-filled") }
            if let b = ui.capCard(beats[0]) { await ui.storyContext(b, name: "story-beat-context-menu", title: "Beat, the context menu", from: "story-board-filled") }
            if let s = ui.storyEl("story-sequence-") { await ui.storyContext(s, name: "story-sequence-context-menu", title: "Sequence, the context menu", from: "story-board-filled") }
            if let p = ui.capCard("A dream about the sea") { await ui.storyContext(p, name: "story-parked-card-context-menu", title: "A card in the Parking Lot, the context menu", from: "story-board-filled") }
            if let a = ui.storyActs().first { await ui.storyContext(a, at: ui.storyActHeader(a), name: "story-act-context-menu", title: "Act, the context menu", from: "story-board-filled") }
            if let o = ui.storyEl("story-act-options-") {
                await ui.capMenu(id: o.identifier, name: "story-act-options-menu", title: "Act, the Options menu", area: A, from: "story-board-filled")
            }

            // Inspectors.
            await ui.step("card inspector") {
                try await ui.storyOpen(ui.capCard(cards[0]))
                await ui.capture("story-card-inspector", title: "Scene card open in the inspector", area: A, from: "story-board-filled", via: "story-card")
                try await ui.fill("Scene heading (optional now, required before it becomes a screenplay scene)", "EXT. BUS STOP - NIGHT")
                try await ui.fill("Long description", "The shelter light flickers. Meera checks the timetable twice.")
                try await ui.capAct(label: "blue", role: "AXButton")
                await ui.idle(1.5)
                await ui.capture("story-card-inspector-filled", title: "Scene card inspector with a heading, a long description and a colour", area: A, from: "story-card-inspector")
                await ui.storyCloseInspector()
                await ui.capture("story-board-card-details", title: "Beat Board, a card with a colour and a scene heading", area: A, from: "story-card-inspector-filled")
            }
            await ui.step("beat inspector") {
                try await ui.storyOpen(ui.capCard(beats[0]))
                await ui.capture("story-beat-inspector", title: "Beat open in the inspector", area: A, from: "story-board-filled", via: "story-card")
                await ui.storyCloseInspector()
            }
            await ui.step("sequence inspector") {
                try await ui.storyOpen(ui.storyEl("story-sequence-"))
                await ui.capture("story-sequence-inspector", title: "Sequence open in the inspector", area: A, from: "story-board-filled", via: "story-sequence")
                await ui.storyCloseInspector()
            }
            await ui.step("parked card inspector") {
                try await ui.storyOpen(ui.capCard("A dream about the sea"))
                await ui.capture("story-parked-card-inspector", title: "A parked card open in the inspector", area: A, from: "story-board-filled", via: "story-card")
                await ui.storyCloseInspector()
            }

            // Renaming in place.
            await ui.step("rename act") {
                guard let o = ui.storyEl("story-act-options-") else { throw DriverError("no act") }
                try await ui.capMenuButton(id: o.identifier, item: "Rename")
                try await ui.wait(label: "Act title", timeout: 4)
                await ui.capture("story-act-rename-inline", title: "Beat Board, renaming an act", area: A, from: "story-act-options-menu", via: "Rename")
                try await ui.escape()
                await ui.idle(0.4)
            }
            await ui.step("rename sequence") {
                guard let s = ui.storyEl("story-sequence-") else { throw DriverError("no sequence") }
                try await ui.storyContextChoose(s, "Rename")
                try await ui.wait(label: "Sequence name", timeout: 4)
                await ui.capture("story-sequence-rename-inline", title: "Beat Board, renaming a sequence", area: A, from: "story-sequence-context-menu", via: "Rename")
                try await ui.escape()
                await ui.idle(0.4)
            }

            // Sheets from the act menu and the card menu.
            await ui.step("act note") {
                guard let o = ui.storyEl("story-act-options-") else { throw DriverError("no act options") }
                try await ui.capMenuButton(id: o.identifier, item: "Edit note")
                try await ui.capWaitUntil("note sheet") { ui.hasOverlay }
                await ui.capture("story-act-note-sheet", title: "Edit the note of an act", area: A, from: "story-act-options-menu", via: "Edit note")
                await ui.capRecover()
            }
            await ui.step("delete act sheet") {
                guard let o = ui.storyEl("story-act-options-") else { throw DriverError("no act options") }
                try await ui.capMenuButton(id: o.identifier, item: "Delete")
                try await ui.capWaitUntil("delete act sheet") { ui.hasOverlay }
                await ui.capture("story-delete-act-sheet", title: "Delete an act that holds cards", area: A, from: "story-act-options-menu", via: "Delete")
                await ui.capRecover()
            }
            await ui.step("card move sheet") {
                guard let c = ui.capCard(cards[2]) else { throw DriverError("no card") }
                try await ui.storyContextChoose(c, "Move")
                try await ui.capWaitUntil("move sheet") { ui.hasOverlay }
                await ui.storySheetMenu(index: 0, name: "story-move-destination-menu", title: "Move a card, the Destination menu", from: "story-card-context-menu")
                await ui.capRecover()
            }
            await ui.step("build preselected") {
                guard let c = ui.capCard(cards[0]) else { throw DriverError("no card") }
                try await ui.storyContextChoose(c, "Convert to Screenplay Scene")
                try await ui.capWaitUntil("build sheet") { ui.hasOverlay }
                await ui.capture("story-build-sheet-one-card", title: "Build Screenplay with one card chosen", area: A, from: "story-card-context-menu", via: "Convert to Screenplay Scene")
                await ui.capRecover()
            }
            await ui.step("deleted notice") {
                guard let b = ui.capCard(beats[1]) else { throw DriverError("no beat") }
                try await ui.capCustomAction(b, "Delete")
                try await ui.wait(id: "story-notice", timeout: 4)
                await ui.capture("story-deleted-notice", title: "Beat Board, a beat deleted with Undo offered", area: A, from: "story-beat-context-menu", via: "Delete")
                try await ui.capAct(id: "story-notice-undo")
                await ui.idle(0.8)
            }

            // Collapsing, filtering, zoom and the Parking Lot.
            await ui.step("collapse sequence") {
                try await ui.capAct(label: "Collapse Missing the bus", role: "AXButton")
                await ui.capture("story-sequence-collapsed", title: "Beat Board, a sequence collapsed", area: A, from: "story-board-filled", via: "Collapse")
                try await ui.capAct(label: "Expand Missing the bus", role: "AXButton")
            }
            await ui.step("collapse act") {
                try await ui.capAct(label: "Collapse Act Two: The depot", role: "AXButton")
                await ui.capture("story-act-collapsed", title: "Beat Board, an act collapsed", area: A, from: "story-board-filled", via: "Collapse")
                try await ui.capAct(label: "Expand Act Two: The depot", role: "AXButton")
            }
            await ui.step("filter") {
                try await ui.fill("Search / filter cards", "bus")
                await ui.idle(0.8)
                await ui.capture("story-board-filtered", title: "Beat Board filtered to cards that mention bus", area: A, from: "story-board-filled", via: "Search / filter cards")
                try await ui.fill("Search / filter cards", "helicopter")
                await ui.idle(0.8)
                await ui.capture("story-board-filter-no-match", title: "Beat Board filtered with no matching card", area: A, from: "story-board-filtered", via: "Search / filter cards")
                _ = await ui.tryPress(label: "Clear filter", wait: 1.5)
                await ui.idle(0.4)
            }
            await ui.step("zoom") {
                await ui.storySetZoom(70)
                await ui.capture("story-board-zoom-out", title: "Beat Board zoomed out to 70 percent", area: A, from: "story-board-filled", via: "Zoom out")
                await ui.storySetZoom(150)
                await ui.capture("story-board-zoom-in", title: "Beat Board zoomed in to 150 percent", area: A, from: "story-board-filled", via: "Zoom in")
                await ui.storySetZoom(100)
            }
            await ui.step("parking hidden") {
                try await ui.capAct(id: "story-parking-toggle")
                await ui.idle(0.5)
                await ui.capture("story-board-parking-hidden", title: "Beat Board with the Parking Lot hidden", area: A, from: "story-board-filled", via: "story-parking-toggle")
                try await ui.capAct(id: "story-parking-toggle")
                await ui.idle(0.4)
            }

            // View options and the board-wide sheets.
            await ui.capMenu(id: "story-more", name: "story-more-menu", title: "Beat Board, the More menu", area: A, from: "story-board-filled")
            await ui.step("card display") {
                try await ui.capMenuButton(id: "story-more", item: "Card Display Settings")
                try await ui.capWaitUntil("card display sheet") { ui.hasOverlay }
                await ui.capture("story-card-display-sheet", title: "Card Display Settings", area: A, from: "story-more-menu", via: "story-more")
                try await ui.capAct(id: "story-preset-production")
                await ui.idle(1.0)
                await ui.capture("story-card-display-production", title: "Card Display Settings with the Production preset", area: A, from: "story-card-display-sheet", via: "story-preset-production")
                await ui.capRecover()
                await ui.capture("story-board-production-cards", title: "Beat Board with cards showing the Production preset", area: A, from: "story-card-display-production")
                try await ui.capMenuButton(id: "story-more", item: "Card Display Settings")
                try await ui.capAct(id: "story-preset-minimal")
                await ui.idle(0.8)
                await ui.capRecover()
            }
            await ui.step("apply order") {
                try await ui.capMenuButton(id: "story-more", item: "Apply Board Order")
                try await ui.capWaitUntil("apply order sheet") { ui.hasOverlay }
                await ui.capture("story-apply-order-sheet", title: "Apply Board Order to Screenplay, no screenplay yet", area: A, from: "story-more-menu", via: "story-more")
                await ui.capRecover()
            }
            await ui.visit(press: "story-build", name: "story-build-sheet", title: "Build Screenplay from the Beat Board", area: A, from: "story-board-filled", close: nil)

            // Outline.
            await ui.step("outline") {
                try await ui.capTab("Outline")
                try await ui.wait(id: "story-outline", timeout: 6)
                await ui.capture("story-outline", title: "Outline of the story", area: A, from: "story-board-filled", via: "story-tabs")
            }
            await ui.capMenu(label: "Act options for Act One: The last bus", name: "story-outline-act-menu", title: "Outline, the act options menu", area: A, from: "story-outline")
            if let r = ui.storyEl("story-outline-", cards[2], exclude: ["story-outline-act"]) {
                await ui.storyContext(r, name: "story-outline-row-context-menu", title: "Outline, the context menu of a card row", from: "story-outline")
                await ui.step("outline row selected") {
                    try await ui.tap(r)
                    await ui.capture("story-outline-row-selected", title: "Outline with a card row selected", area: A, from: "story-outline", via: "story-outline-row")
                }
                await ui.step("outline inspector") {
                    try await ui.storyOpen(ui.storyEl("story-outline-", cards[2], exclude: ["story-outline-act"]))
                    await ui.capture("story-outline-inspector", title: "Outline with a card open in the inspector", area: A, from: "story-outline-row-selected", via: "story-outline-row")
                    await ui.storyCloseInspector()
                }
            }
            await ui.step("outline collapse act") {
                try await ui.capAct(label: "Collapse Act One: The last bus", role: "AXButton")
                await ui.capture("story-outline-act-collapsed", title: "Outline with an act collapsed", area: A, from: "story-outline", via: "Collapse")
                try await ui.capAct(label: "Expand Act One: The last bus", role: "AXButton")
            }
            await ui.step("outline filter") {
                try await ui.fill("Search / filter cards", "depot")
                await ui.idle(0.8)
                await ui.capture("story-outline-filtered", title: "Outline filtered to cards that mention depot", area: A, from: "story-outline", via: "Search / filter cards")
                _ = await ui.tryPress(label: "Clear filter", wait: 1.5)
            }

            // Converting a beat leaves it marked Converted and opens the new card.
            await ui.step("convert beat") {
                try await ui.capTab("Beat Board")
                guard let b = ui.capCard(beats[0]) else { throw DriverError("no beat") }
                try await ui.storyContextChoose(b, "Convert to Scene")
                try await ui.wait(id: "story-inspector-close", timeout: 6)
                await ui.capture("story-beat-converted", title: "Beat Board, a beat converted to a scene card", area: A, from: "story-beat-context-menu", via: "Convert to Scene")
                await ui.storyCloseInspector()
            }
        },

        // Characters and the Timeline, on a project with the example screenplay (its scenes have the cues KIRAN and DEVI).
        Scenario("zz-capture/03-story-people", timeout: 500) { ui, _, ctx in
            let A = storyArea
            try await ui.capSeed(ctx, order: 3500)
            try await ui.capNav("Story")
            try await ui.wait(id: "story-tabs", timeout: 20)

            // A card on the board with no act, to link to a character later.
            await ui.step("unassigned card") {
                try await ui.capAct(label: "Add Scene", role: "AXButton")
                try await ui.fill("Short description", "Kiran waits at the stop", submit: true)
                await ui.idle(0.8)
                try await ui.escape()
                await ui.capture("story-board-unassigned", title: "Beat Board with a card outside any act", area: A, from: "story-board-empty", via: "story-empty-add-scene")
            }

            await ui.step("characters empty") {
                try await ui.capTab("Characters")
                try await ui.wait(label: "Add Character", role: "AXButton", timeout: 6)
                await ui.capture("story-characters-empty", title: "Characters, none yet", area: A, from: "story-board-empty", via: "story-tabs")
            }
            await ui.step("new character") {
                try await ui.capAct(label: "Add Character", role: "AXButton")
                try await ui.wait(label: "Character name", timeout: 5)
                await ui.capture("story-add-character-sheet", title: "New Character", area: A, from: "story-characters-empty", via: "story-add-character")
                try await ui.fill("Character name", "Kiran")
                try await ui.fill("Role", "A ticket inspector on the night route")
                try await ui.fill("Short description", "Knows every regular by name")
                await ui.capture("story-add-character-filled", title: "New Character, filled in", area: A, from: "story-add-character-sheet")
                try await ui.capAct(label: "Create Character", role: "AXButton")
                await ui.idle(1.0)
                await ui.capture("story-character-one", title: "Characters, one character with the scenes it appears in", area: A, from: "story-add-character-filled", via: "story-create-character")
            }
            await ui.step("possible match") {
                try await ui.capAct(label: "Add Character", role: "AXButton")
                try await ui.fill("Character name", "Kiran")
                await ui.idle(0.6)
                await ui.capture("story-add-character-match", title: "New Character warns about a possible match", area: A, from: "story-character-one", via: "story-add-character")
                await ui.capRecover()
            }
            for (name, role) in [("Devi", "A nurse coming off shift"), ("Meera", "A student who sleeps on the bus"), ("Tomas", "The depot mechanic")] {
                await ui.step("add \(name)") { try await ui.storyNewCharacter(name, role) }
            }
            await ui.capture("story-characters", title: "Characters, four in the list", area: A, from: "story-character-one")
            await ui.step("character without scenes") {
                try await ui.storyCharacter("Meera")
                await ui.capture("story-character-no-scenes", title: "A character no screenplay scene mentions yet", area: A, from: "story-characters", via: "story-character")
            }
            await ui.step("character with scenes") {
                try await ui.storyCharacter("Devi")
                await ui.capture("story-character-detail", title: "A character with the screenplay scenes it appears in", area: A, from: "story-characters", via: "story-character")
            }
            await ui.visit(label: "Edit", role: "AXButton", name: "story-character-edit-sheet", title: "Edit a character", area: A, from: "story-character-detail")
            await ui.step("character more menu") {
                let e = try ui.storyMorePopup()
                guard let r = ui.relFrame(e) else { throw DriverError("no frame") }
                let menu = try await ui.peekMenu(e)
                await ui.capture("story-character-more-menu", title: "Character, the More menu", area: A, from: "story-character-detail", via: "story-character-more", menu: menu, anchor: CGPoint(x: r.minX - 180, y: r.maxY + 4))
            }

            // Relationships.
            await ui.step("map empty") {
                try await ui.capAct(label: "Open Relationship Map", role: "AXButton")
                await ui.idle(1.0)
                await ui.capture("story-relationship-map-empty", title: "Relationship map, no relationships yet", area: A, from: "story-character-detail", via: "Open Relationship Map")
                try await ui.storyMapBack()
            }
            await ui.step("add relationship") {
                try await ui.storyCharacter("Kiran")
                try await ui.capAct(label: "Add Relationship", role: "AXButton")
                try await ui.capWaitUntil("relationship sheet") { ui.hasOverlay }
                await ui.capture("story-add-relationship-sheet", title: "Add Relationship", area: A, from: "story-character-detail", via: "Add Relationship")
                await ui.capMenu(label: "Suggestions", name: "story-relationship-suggestions", title: "Add Relationship, the Suggestions menu", area: A, from: "story-add-relationship-sheet")
                try await ui.storyPick(index: 2, item: "DEVI")
                try await ui.fill("Relationship", "sibling of")
                try await ui.fill("Note", "Older by four years")
                await ui.capture("story-add-relationship-filled", title: "Add Relationship, filled in", area: A, from: "story-add-relationship-sheet")
                try await ui.capAct(label: "Add Relationship", role: "AXButton")
                await ui.idle(1.0)
                await ui.capRecover()
                await ui.capture("story-character-relationships", title: "A character with a relationship", area: A, from: "story-add-relationship-filled")
            }
            await ui.step("more relationships") {
                try await ui.capAct(label: "Open Relationship Map", role: "AXButton")
                await ui.idle(0.6)
                for (f, t, r) in [("Meera", "Kiran", "friend of"), ("Tomas", "Meera", "rival of"), ("Devi", "Tomas", "colleague of")] {
                    guard let add = ui.storyMapButtons().last else { throw DriverError("no Add Relationship on the map") }
                    try await ui.press(add)
                    try await ui.capWaitUntil("relationship sheet") { ui.hasOverlay }
                    try await ui.storyRelationship(from: f, to: t, type: r)
                    await ui.capRecover()
                }
                await ui.idle(0.8)
                await ui.capture("story-relationship-map", title: "Relationship map with four characters", area: A, from: "story-character-relationships", via: "Open Relationship Map")
                try await ui.storyMapBack()
            }

            // Linking a card, deleting and archiving.
            await ui.step("link card") {
                try await ui.storyCharacter("Kiran")
                try await ui.capMenuButton(label: "Scene Card to link", item: "Kiran waits")
                try await ui.capAct(label: "Link", role: "AXButton")
                await ui.idle(1.0)
                await ui.capture("story-character-linked-card", title: "A character with a linked scene card", area: A, from: "story-character-relationships", via: "Link")
            }
            await ui.step("delete in use") {
                try await ui.storyMoreChoose("Delete")
                try await ui.capWaitUntil("delete sheet") { ui.hasOverlay }
                await ui.capture("story-delete-character-in-use", title: "Delete a character that is in use", area: A, from: "story-character-more-menu", via: "Delete")
                await ui.capRecover()
            }
            await ui.step("delete unused") {
                try await ui.storyCharacter("Tomas")
                try await ui.storyMoreChoose("Delete")
                try await ui.capWaitUntil("delete sheet") { ui.hasOverlay }
                await ui.capture("story-delete-character-sheet", title: "Delete a character", area: A, from: "story-character-more-menu", via: "Delete")
                await ui.capRecover()
            }
            await ui.step("archive") {
                try await ui.storyMoreChoose("Archive")
                await ui.idle(1.0)
                await ui.capture("story-characters-archived-hidden", title: "Characters after archiving one", area: A, from: "story-character-more-menu", via: "Archive")
                try await ui.capAct(label: "Show archived characters")
                await ui.idle(1.0)
                await ui.capture("story-characters-show-archived", title: "Characters with archived ones shown", area: A, from: "story-characters-archived-hidden", via: "Show archived characters")
                try await ui.capAct(label: "Show archived characters")
            }

            // Timeline.
            await ui.step("timeline") {
                try await ui.capTab("Timeline")
                try await ui.wait(label: "Select scene 1", timeout: 8)
                await ui.idle(0.8)
                await ui.capture("story-timeline", title: "Timeline, every scene without a story day", area: A, from: "story-characters", via: "story-tabs")
                for n in 1...3 { try await ui.capAct(label: "Select scene \(n)") }
                await ui.capture("story-timeline-picked", title: "Timeline with three scenes picked", area: A, from: "story-timeline", via: "Select scene")
                try await ui.storyAssignDay()
                try await ui.capWaitUntil("assign sheet") { ui.hasOverlay }
                await ui.capture("story-assign-day-sheet", title: "Assign Story Day", area: A, from: "story-timeline-picked", via: "story-assign-day")
                try await ui.fill("Story Day", "1")
                try await ui.capAct(label: "Assign", role: "AXButton")
                await ui.idle(1.0)
                await ui.capRecover()
                for n in 4...5 { try await ui.capAct(label: "Select scene \(n)") }
                try await ui.storyAssignDay()
                try await ui.capWaitUntil("assign sheet") { ui.hasOverlay }
                try await ui.fill("Story Day", "Flashback")
                try await ui.capAct(label: "Assign", role: "AXButton")
                await ui.idle(1.0)
                await ui.capRecover()
                try await ui.fill("Time of day note for scene 1", "11:40 pm")
                try await ui.enter()
                await ui.idle(0.8)
                await ui.capture("story-timeline-days", title: "Timeline grouped by story day, with a time note", area: A, from: "story-assign-day-sheet", via: "Assign")
            }
        },
    ]
}

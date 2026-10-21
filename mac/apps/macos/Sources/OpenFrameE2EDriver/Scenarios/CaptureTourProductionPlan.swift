// Capture tour section: Production plan (Overview, Catalog, Locations, Cast and Crew, Moodboards,
// Storyboards, Shot Lists). Primitives and helpers are in CaptureTour.swift.
// Seven scenarios: every tab of an empty project (09-plan-a-empty), then Night Bus for the
// Catalog (b), Locations (c), Cast and Crew (d), Moodboards (e), Storyboards (f) and Shot Lists (g).
// Adding images needs the system Open panel, so no image is added anywhere here.

import AppKit
import Foundation

private let planArea = "Production Plan"

fileprivate extension UI {
    func planShot(_ name: String, _ title: String, from: String? = nil, via: String? = nil, settle: Double = 0.8) async {
        await capture(name, title: title, area: planArea, from: from, via: via, settle: settle)
    }

    func planMenu(id: String? = nil, label: String? = nil, _ name: String, _ title: String, from: String?) async {
        await capMenu(id: id, label: label, name: name, title: title, area: planArea, from: from)
    }

    /// A pop-up button's menu drawn open, found by its label among pop-up buttons only. A failure
    /// is logged but leaves the sheet open, so the step that called it goes on.
    func planPopupShot(label: String, _ name: String, _ title: String, from: String?) async {
        guard let e = all().first(where: { $0.obj is NSPopUpButtonCell && ($0.label == label || $0.title == label) && $0.frame.width > 0 }),
              let r = relFrame(e), let menu = try? await peekMenu(e) else { log("no pop-up button \(label)"); return }
        let w = capturePrimary()?.frame.width ?? 1200
        await capture(name, title: title, area: planArea, from: from, via: label, menu: menu, anchor: CGPoint(x: min(r.minX, w - 260), y: r.maxY + 4))
    }

    /// Opens a Production tab by its title.
    func planTab(_ title: String) async throws {
        await capRecover()
        try await capNav(title)
        await idle(0.8)
    }

    /// Elements whose identifier starts with this prefix and that are on screen.
    func planRows(_ prefix: String) -> [Element] {
        capQueryPrefix(prefix).filter { $0.frame.width > 0 && $0.frame.height > 0 }
    }

    /// The point a click or right click aims at: the middle, or just inside the top left corner
    /// (the visible part of a tile that others overlap).
    func planPoint(_ e: Element, corner: Bool) -> NSPoint {
        let f = e.frame
        return corner ? NSPoint(x: f.minX + 14, y: f.maxY - 14) : NSPoint(x: f.midX, y: f.midY)
    }

    func planTileClick(_ e: Element, modifiers: NSEvent.ModifierFlags = []) async throws {
        let c = planPoint(e, corner: true)
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(c) }) else { throw DriverError("no window under \(e)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: c)
        for t in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            if let ev = NSEvent.mouseEvent(with: t, location: local, modifierFlags: modifiers, timestamp: ProcessInfo.processInfo.systemUptime,
                                           windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: t == .leftMouseDown ? 1 : 0) {
                NSApp.postEvent(ev, atStart: false)
            }
        }
        await idle(0.7)
    }

    /// The right-click menu at a tile's top left corner, drawn open.
    func planContextShot(_ e: Element, _ name: String, _ title: String, from: String?) async {
        let pt = planPoint(e, corner: true)
        guard let menu = capContextMenuItems(at: pt), let pw = capturePrimary() else { log("no context menu on \(e)"); return }
        await capture(name, title: title, area: planArea, from: from, via: e.identifier, menu: menu.copy() as? NSMenu ?? menu,
                      anchor: CGPoint(x: pt.x - pw.frame.minX, y: pw.frame.maxY - pt.y))
    }

    /// Runs an item of an element's right-click menu.
    func planContextPick(_ e: Element, _ item: String, corner: Bool = false) async throws {
        guard let menu = capContextMenuItems(at: planPoint(e, corner: corner)) ?? capContextMenuItems(at: planPoint(e, corner: false)) else { throw DriverError("no context menu on \(e)") }
        func find(_ m: NSMenu) -> (NSMenu, Int)? {
            for (i, it) in m.items.enumerated() where !it.isSeparatorItem && it.title.hasPrefix(item) { return (m, i) }
            for it in m.items { if let sub = it.submenu, let r = find(sub) { return r } }
            return nil
        }
        guard let (m, i) = find(menu) else { throw DriverError("no item \(item) in \(menu.items.map(\.title))") }
        guard m.items[i].isEnabled else { throw DriverError("menu item is disabled: \(item)") }
        m.performActionForItem(at: i)
        await idle(0.8)
    }

    /// Presses a control, else clicks it.
    func planPress(_ e: Element) async throws {
        if e.performPress() { await idle(0.5); return }
        try await tap(e)
        await idle(0.5)
    }

    /// A sheet's own window is on screen.
    func planWaitSheet(_ timeout: Double = 6) async throws {
        try await capWaitUntil("a sheet opens", timeout: timeout) { hasOverlay }
        await idle(0.4)
    }

    func planWaitNoSheet(_ timeout: Double = 8) async throws {
        try await capWaitUntil("the sheet closes", timeout: timeout) { !hasOverlay }
        await idle(0.4)
    }

    /// The project Empty Short: no screenplay, nothing planned.
    func planEmptyProject() async throws {
        await capSetMode()
        capBigWindow()
        try await createProject("Empty Short")
        await idle(0.8)
    }

    /// Adds a location by its sheet; the new location opens in the inspector.
    func planAddLocation(_ name: String, address: String) async throws {
        try await capAct(id: "add-location")
        _ = try await wait(id: "nl-name", timeout: 8)
        await idle(0.5)
        try await capTypeIn(name, into: "nl-name")
        try await capTypeIn(address, into: "nl-addr")
        try await capAct(id: "nl-submit")
        try await waitText("Added location \u{201C}\(name)\u{201D}")
        _ = try await wait(id: "loc-status", timeout: 10)
        await idle(0.6)
    }

    func planCloseInspector() async {
        for id in ["inspector-close", "moodboard-inspector-close", "panel-inspector-close", "shot-inspector-close"] {
            if let e = query(id: id).first(where: { $0.role == "AXButton" && $0.frame.width > 0 }) { _ = e.performPress(); await idle(0.5) }
        }
    }

    /// Picks a scene in a scene picker: the first item whose title starts with "Scene <n>".
    func planChooseScene(_ id: String, _ n: Int) async throws {
        guard let cell = query(id: id).first(where: { $0.obj is NSPopUpButtonCell })?.obj as? NSPopUpButtonCell, let m = cell.menu else {
            try await capMenuButton(id: id, item: "Scene \(n)")
            return
        }
        func titles(_ m: NSMenu) -> [String] { m.items.flatMap { [$0.title] + ($0.submenu.map(titles) ?? []) } }
        if let t = titles(m).first(where: { $0.hasPrefix("Scene \(n):") || $0.contains("\u{00B7} Scene \(n):") }) {
            try await capMenuButton(id: id, item: t)
        } else {
            try await capMenuButton(id: id, item: "Scene \(n)")
        }
        await idle(0.8)
    }
}

extension Scenarios {
    static let captureVisual: [Scenario] = [

        // MARK: every tab of an empty project

        tour("09-plan-a-empty", order: 9000, project: false) { ui, _ in
            try await ui.planEmptyProject()
            await ui.step("overview empty") {
                try await ui.planTab("Overview")
                _ = try await ui.wait(id: "production-tile-script", timeout: 10)
                await ui.planShot("plan-overview-empty", "Production Overview, nothing planned yet", via: "production-tab-overview")
            }
            await ui.step("catalog empty") {
                try await ui.planTab("Catalog")
                try await ui.waitText("The catalog is empty.")
                await ui.planShot("plan-catalog-empty", "Catalog, empty", from: "plan-overview-empty", via: "production-tab-catalog")
            }
            await ui.step("catalog add empty") {
                try await ui.capAct(id: "add-item")
                _ = try await ui.wait(id: "ai-name", timeout: 8)
                await ui.planShot("plan-catalog-add-empty", "Add Item, no name yet so Add Item is off", from: "plan-catalog-empty", via: "add-item")
                await ui.planMenu(id: "ai-category", "plan-catalog-add-category-menu", "Add Item, the category menu", from: "plan-catalog-add-empty")
                await ui.capRecover()
            }
            await ui.step("locations empty") {
                try await ui.planTab("Locations")
                try await ui.waitText("No locations yet.")
                await ui.planShot("plan-locations-empty", "Locations, empty", from: "plan-overview-empty", via: "production-tab-locations")
            }
            await ui.step("cast empty") {
                try await ui.planTab("Cast & Crew")
                try await ui.waitText("No cast yet.")
                await ui.planShot("plan-cast-empty", "Cast and Crew, no cast yet", from: "plan-overview-empty", via: "production-tab-castCrew")
                try await ui.capTab("Crew")
                await ui.planShot("plan-crew-empty", "Cast and Crew, no crew yet", from: "plan-cast-empty", via: "people-view")
                try await ui.capTab("Cast")
            }
            await ui.step("moodboards empty") {
                try await ui.planTab("Moodboards")
                _ = try await ui.wait(id: "moodboard-empty", timeout: 10)
                await ui.planShot("plan-moodboards-empty", "Moodboards, empty", from: "plan-overview-empty", via: "production-tab-moodboards")
            }
            await ui.step("moodboard new validation") {
                try await ui.capAct(id: "moodboard-empty-new")
                _ = try await ui.wait(id: "mb-name", timeout: 8)
                await ui.planShot("plan-moodboard-new-empty", "New Moodboard, no scenes to reference", from: "plan-moodboards-empty", via: "moodboard-empty-new")
                try await ui.capAct(id: "moodboard-new-sheet-primary")
                try await ui.waitText("Give the board a name.")
                await ui.planShot("plan-moodboard-new-no-name", "New Moodboard, the name is missing", from: "plan-moodboard-new-empty", via: "moodboard-new-sheet-primary")
                await ui.capRecover()
            }
            await ui.step("storyboards empty") {
                try await ui.planTab("Storyboards")
                _ = try await ui.wait(id: "storyboard-empty", timeout: 10)
                await ui.planShot("plan-storyboards-empty", "Storyboards, empty with no screenplay", from: "plan-overview-empty", via: "production-tab-storyboards")
            }
            await ui.step("storyboard new validation") {
                try await ui.capAct(id: "storyboard-empty-new")
                _ = try await ui.wait(id: "new-sb-name", timeout: 8)
                try await ui.capAct(id: "storyboard-new-sheet-primary")
                try await ui.waitText("Give the storyboard a name, or choose a scene.")
                await ui.planShot("plan-storyboard-new-no-name", "New Storyboard, the name is missing", from: "plan-storyboards-empty", via: "storyboard-new-sheet-primary")
                await ui.capRecover()
            }
            await ui.step("shots no script") {
                try await ui.planTab("Shot Lists")
                _ = try await ui.wait(id: "visual-no-script", timeout: 10)
                await ui.planShot("plan-shots-no-script", "Shot Lists, no screenplay scenes yet", from: "plan-overview-empty", via: "production-tab-shotLists")
            }
        },

        // MARK: Overview and Catalog with the breakdown accepted

        tour("09-plan-b-catalog", order: 9100) { ui, _ in
            try await capBreakdownBase(ui)
            await ui.step("overview filled") {
                try await ui.planTab("Overview")
                _ = try await ui.wait(id: "production-tile-script", timeout: 10)
                await ui.planShot("plan-overview-filled", "Production Overview, source chosen and two scenes broken down", via: "production-tab-overview")
            }
            await ui.step("catalog items") {
                try await ui.capAct(id: "production-tile-catalog")
                _ = try await ui.wait(id: "catalog-table", timeout: 10)
                await ui.planShot("plan-catalog-items", "Catalog, items from the accepted breakdown", from: "plan-overview-filled", via: "production-tile-catalog")
            }
            var last = "plan-catalog-items"
            for cat in ["Props", "Wardrobe", "Cast", "Locations"] {
                await ui.step("catalog filter \(cat)") {
                    try await ui.capTab(cat)
                    let n = "plan-catalog-filter-\(ui.slug(cat))"
                    await ui.planShot(n, "Catalog, only \(cat)", from: last, via: cat)
                    last = n
                }
            }
            await ui.step("catalog all") { try await ui.capTab("All") }
            await ui.planMenu(label: "More categories", "plan-catalog-more-menu", "Catalog, more categories", from: "plan-catalog-items")
            await ui.step("catalog search") {
                try await ui.capTypeIn("zzz", into: "catalog-search")
                try await ui.waitText("No catalog items match.")
                await ui.planShot("plan-catalog-search-none", "Catalog, a search that matches nothing", from: "plan-catalog-items", via: "catalog-search")
                try await ui.capTypeIn("", into: "catalog-search")
                _ = try await ui.wait(id: "catalog-table", timeout: 10)
            }
            await ui.step("catalog context") {
                guard let t = ui.query(id: "catalog-table").first?.obj as? NSTableView, t.numberOfRows > 0, let w = t.window else { throw DriverError("no catalog rows") }
                let r = w.convertToScreen(t.convert(t.rect(ofRow: 0), to: nil))
                let pt = NSPoint(x: r.minX + 80, y: r.midY)
                guard let menu = ui.capContextMenuItems(at: pt), let pw = ui.capturePrimary() else { throw DriverError("no context menu on the first row") }
                await ui.capture("plan-catalog-row-menu", title: "Catalog, the right-click menu of an item", area: planArea, from: "plan-catalog-items", via: "catalog-table",
                                 menu: menu.copy() as? NSMenu ?? menu, anchor: CGPoint(x: pt.x - pw.frame.minX, y: pw.frame.maxY - pt.y))
            }
            await ui.step("catalog item open") {
                try await selectTableRow(ui, id: "catalog-table", row: 1)
                _ = try await ui.wait(id: "ci-name", timeout: 10)
                await ui.planShot("plan-catalog-item-open", "Catalog, an item open in the inspector", from: "plan-catalog-items", via: "catalog-table")
                await ui.planMenu(id: "ci-status", "plan-catalog-item-status-menu", "Catalog item, the status menu", from: "plan-catalog-item-open")
            }
            await ui.step("catalog item notes") {
                try await ui.capTypeIn("Borrowed from the depot, return by Friday", into: "ci-notes")
                await ui.planShot("plan-catalog-item-notes", "Catalog item, notes being written", from: "plan-catalog-item-open", via: "ci-notes")
            }
            await ui.step("catalog archive sheet") {
                try await ui.capAct(id: "catalog-archive")
                try await ui.planWaitSheet()
                await ui.planShot("plan-catalog-archive-sheet", "Catalog, archive an item", from: "plan-catalog-item-open", via: "catalog-archive")
                await ui.capRecover()
            }
            await ui.step("catalog delete sheet") {
                if !ui.exists(id: "catalog-delete") { try await selectTableRow(ui, id: "catalog-table", row: 1) }
                _ = try await ui.wait(id: "catalog-delete", timeout: 8)
                try await ui.capAct(id: "catalog-delete")
                try await ui.planWaitSheet()
                await ui.planShot("plan-catalog-delete-sheet", "Catalog, delete an item", from: "plan-catalog-item-open", via: "catalog-delete")
                await ui.capRecover()
                await ui.planCloseInspector()
            }
            await ui.step("catalog add duplicate") {
                // A prop the accepted breakdown put in the catalog, typed again under Props.
                let prop = ["Umbrella", "Bag", "Keys", "Glass"].first { ui.exists(id: "catalog-row-\($0)") } ?? "Umbrella"
                try await ui.capAct(id: "add-item")
                _ = try await ui.wait(id: "ai-name", timeout: 8)
                await ui.idle(0.4)
                await ui.step("catalog add duplicate warning") {
                    try await ui.capTypeIn(prop, into: "ai-name")
                    try await ui.waitText("The catalog already has", timeout: 6)
                    await ui.planShot("plan-catalog-add-duplicate", "Add Item, the catalog already has this name", from: "plan-catalog-items", via: "add-item")
                }
                if !ui.exists(id: "ai-name") {
                    try await ui.capAct(id: "add-item")
                    _ = try await ui.wait(id: "ai-name", timeout: 8)
                }
                try await ui.capTypeIn("Folding umbrella", into: "ai-name")
                await ui.idle(1.0)
                await ui.planShot("plan-catalog-add-filled", "Add Item, a new prop", from: "plan-catalog-items", via: "add-item")
                try await ui.capAct(id: "ai-submit")
                try await ui.waitText("Added \u{201C}Folding umbrella\u{201D} to the catalog")
                await ui.planShot("plan-catalog-added", "Catalog, the new item added and open", from: "plan-catalog-add-filled", via: "ai-submit")
            }
            await ui.step("catalog archived shown") {
                try await ui.capAct(id: "catalog-archive")
                try await ui.planWaitSheet()
                let confirm = ui.windowElements(NSApp.windows.last { $0.isVisible && $0.sheetParent != nil }!).first { $0.role == "AXButton" && ($0.label.hasPrefix("Archive") || $0.title.hasPrefix("Archive")) && $0.isEnabled }
                guard let confirm else { throw DriverError("no Archive button in the sheet") }
                try await ui.planPress(confirm)
                try await ui.planWaitNoSheet()
                await ui.planCloseInspector()
                try await ui.capAct(id: "catalog-show-archived")
                await ui.idle(1.0)
                await ui.planShot("plan-catalog-show-archived", "Catalog, archived items shown", from: "plan-catalog-items", via: "catalog-show-archived")
            }
        },

        // MARK: Locations

        tour("09-plan-c-locations", order: 9200) { ui, _ in
            await ui.step("locations add sheet") {
                try await ui.planTab("Locations")
                try await ui.capAct(id: "add-location")
                _ = try await ui.wait(id: "nl-name", timeout: 8)
                await ui.planShot("plan-location-add-empty", "Add Location, only the name is required so Add is off", from: "plan-locations-empty", via: "add-location")
                try await ui.capTypeIn("Old Railway Station", into: "nl-name")
                try await ui.capTypeIn("Platform 4, Station Road", into: "nl-addr")
                try await ui.capTypeIn("Town Hospital, Station Road", into: "nl-hospital")
                await ui.planShot("plan-location-add-filled", "Add Location, filled in", from: "plan-location-add-empty", via: "nl-name")
                try await ui.capAct(id: "nl-submit")
                try await ui.waitText("Added location \u{201C}Old Railway Station\u{201D}")
                _ = try await ui.wait(id: "loc-status", timeout: 10)
                await ui.planShot("plan-location-open", "Locations, a new location open in the inspector", from: "plan-location-add-filled", via: "nl-submit")
                try await segment(ui, group: "loc-status", "Confirmed")
                await ui.idle(0.8)
                await ui.planShot("plan-location-confirmed", "Location, status set to Confirmed", from: "plan-location-open", via: "loc-status")
                await ui.planCloseInspector()
            }
            let more: [(String, String, String)] = [("Signal Cabin", "Level crossing, North Yard", "Shortlisted"),
                                                    ("Night Bus Depot", "Ring Road, Gate 2", "Idea"),
                                                    ("Riverside Flyover", "Under the east span", "Rejected")]
            for (name, addr, status) in more {
                await ui.step("add location \(name)") {
                    try await ui.planAddLocation(name, address: addr)
                    if status != "Idea" { try await segment(ui, group: "loc-status", status); await ui.idle(0.8) }
                    await ui.planCloseInspector()
                }
            }
            await ui.step("locations four") {
                await ui.idle(0.6)
                await ui.planShot("plan-locations-four", "Locations, four places in four statuses", from: "plan-location-open", via: "add-location")
            }
            var last = "plan-locations-four"
            for status in ["Idea", "Shortlisted", "Confirmed", "Rejected"] {
                await ui.step("locations filter \(status)") {
                    try await ui.capTab(status)
                    let n = "plan-locations-filter-\(ui.slug(status))"
                    await ui.planShot(n, "Locations, only \(status)", from: last, via: status)
                    last = n
                }
            }
            await ui.step("locations all") { try await ui.capTab("All") }
            await ui.step("location card menu") {
                guard let card = ui.planRows("location-card-Signal Cabin").first else { throw DriverError("no card Signal Cabin") }
                await ui.capContext(on: card, name: "plan-location-card-menu", title: "Locations, the right-click menu of a card", area: planArea, from: "plan-locations-four")
            }
            await ui.step("location rejected open") {
                guard let card = ui.planRows("location-card-Riverside Flyover").first else { throw DriverError("no card Riverside Flyover") }
                try await ui.planPress(card)
                _ = try await ui.wait(id: "loc-status", timeout: 10)
                await ui.planShot("plan-location-rejected-open", "Locations, a rejected location open", from: "plan-locations-four", via: "location-card-Riverside Flyover")
            }
            await ui.step("location notes") {
                try await ui.scrollIntoViewPlan(id: "loc-note-parking")
                try await ui.capTypeIn("Unit base in the goods yard", into: "loc-note-parking")
                await ui.planShot("plan-location-notes", "Location, scouting notes", from: "plan-location-rejected-open", via: "loc-note-parking")
            }
            await ui.step("location delete sheet") {
                if !ui.exists(id: "location-delete"), let card = ui.planRows("location-card-Riverside Flyover").first { try await ui.planPress(card) }
                try await ui.capAct(id: "location-delete")
                try await ui.planWaitSheet()
                await ui.planShot("plan-location-delete-sheet", "Locations, delete a location", from: "plan-location-rejected-open", via: "location-delete")
                await ui.capRecover()
                await ui.planCloseInspector()
            }
            await ui.step("locations search") {
                try await ui.capTypeIn("zzz", label: "Search locations")
                try await ui.waitText("No locations match.")
                await ui.planShot("plan-locations-search-none", "Locations, a search that matches nothing", from: "plan-locations-four", via: "Search locations")
                try await ui.capTypeIn("", label: "Search locations")
            }
        },

        // MARK: Cast and Crew

        tour("09-plan-d-people", order: 9300) { ui, _ in
            try await capBreakdownBase(ui)
            await ui.step("cast needs casting") {
                try await ui.planTab("Cast & Crew")
                try await ui.capWaitUntil("a character needs casting", timeout: 10) { ui.all().contains { $0.role == "AXButton" && $0.label.hasPrefix("Assign ") } }
                await ui.planShot("plan-cast-needs-casting", "Cast, characters from the script that need casting", via: "production-tab-castCrew")
            }
            var names: [String] = []
            await ui.step("cast assign sheet") {
                let assigns = ui.all().filter { $0.role == "AXButton" && $0.label.hasPrefix("Assign ") && $0.frame.width > 0 }
                names = assigns.map { String($0.label.dropFirst("Assign ".count)) }
                guard let a = assigns.first else { throw DriverError("no Assign button") }
                try await ui.planPress(a)
                _ = try await ui.wait(id: "as-person", timeout: 8)
                await ui.planShot("plan-cast-assign-sheet", "Assign an actor to a character", from: "plan-cast-needs-casting", via: a.label)
                try await ui.capTypeIn("Meera Iyer", into: "as-person")
                await ui.planShot("plan-cast-assign-filled", "Assign an actor, the name typed", from: "plan-cast-assign-sheet", via: "as-person")
                try await ui.capAct(id: "assign-submit")
                try await ui.planWaitNoSheet()
                await ui.planShot("plan-cast-linked-one", "Cast, one actor linked to a character", from: "plan-cast-assign-filled", via: "assign-submit")
            }
            await ui.step("cast add person") {
                try await ui.capAct(id: "add-person")
                _ = try await ui.wait(id: "as-person", timeout: 8)
                await ui.planShot("plan-cast-add-empty", "Add Person to the cast, nothing chosen so Assign is off", from: "plan-cast-linked-one", via: "add-person")
                try await ui.capTypeIn("Dev Malhotra", into: "as-person")
                if names.count > 1, let c = ui.planRows("character-\(names[1])").first { try await ui.planPress(c) }
                else if let c = ui.planRows("character-").last { try await ui.planPress(c) }
                await ui.planShot("plan-cast-add-character", "Add Person, a character chosen", from: "plan-cast-add-empty", via: "character")
                try await ui.capAct(id: "assign-submit")
                try await ui.planWaitNoSheet()
            }
            await ui.step("cast new character") {
                try await ui.capAct(id: "add-person")
                _ = try await ui.wait(id: "as-person", timeout: 8)
                try await ui.capTypeIn("Rhea Kapoor", into: "as-person")
                try await ui.capTypeIn("THE STATION MASTER", into: "as-new")
                await ui.planShot("plan-cast-add-new-character", "Add Person, a new character not in the script", from: "plan-cast-add-empty", via: "as-new")
                try await ui.capAct(id: "assign-submit")
                try await ui.planWaitNoSheet()
                await ui.planShot("plan-cast-linked", "Cast, actors linked to characters", from: "plan-cast-linked-one", via: "assign-submit")
            }
            await ui.step("cast row menu") {
                guard let row = ui.planRows("cast-row-Meera Iyer").first else { throw DriverError("no cast row") }
                await ui.capContext(on: row, name: "plan-cast-row-menu", title: "Cast, the right-click menu of an actor", area: planArea, from: "plan-cast-linked")
            }
            await ui.step("cast open") {
                guard let row = ui.planRows("cast-row-Meera Iyer").first else { throw DriverError("no cast row") }
                try await ui.planContextPick(row, "Open")
                _ = try await ui.wait(id: "inspector-close", timeout: 10)
                await ui.planShot("plan-cast-open", "Cast, an actor open in the inspector", from: "plan-cast-linked", via: "cast-list")
                await ui.planCloseInspector()
            }
            await ui.step("crew add") {
                try await ui.capTab("Crew")
                try await ui.capAct(id: "add-person")
                _ = try await ui.wait(id: "nc-name", timeout: 8)
                await ui.planShot("plan-crew-add-empty", "Add Person to the crew, empty", from: "plan-crew-empty", via: "add-person")
                await ui.planPopupShot(label: "Department", "plan-crew-department-menu", "Add Person to the crew, the department menu", from: "plan-crew-add-empty")
                try await ui.capTypeIn("Imran Sheikh", into: "nc-name")
                try await ui.capTypeIn("Cinematographer", into: "nc-role")
                await ui.planShot("plan-crew-add-filled", "Add Person to the crew, filled in", from: "plan-crew-add-empty", via: "nc-name")
                try await ui.capAct(id: "nc-submit")
                try await ui.waitText("Added Imran Sheikh to the crew")
                await ui.idle(0.8)
                await ui.planShot("plan-crew-open", "Crew, a new crew member open in the inspector", from: "plan-crew-add-filled", via: "nc-submit")
                await ui.planCloseInspector()
            }
            await ui.step("crew second") {
                try await ui.capAct(id: "add-person")
                _ = try await ui.wait(id: "nc-name", timeout: 8)
                try await ui.capTypeIn("Lata Rao", into: "nc-name")
                try await ui.capTypeIn("Sound recordist", into: "nc-role")
                try await ui.capAct(id: "nc-submit")
                try await ui.waitText("Added Lata Rao to the crew")
                await ui.idle(0.6)
                await ui.planCloseInspector()
                await ui.planShot("plan-crew-list", "Crew, two people with their roles", from: "plan-crew-open", via: "nc-submit")
            }
            await ui.step("crew row menu") {
                guard let row = ui.planRows("crew-row-Imran Sheikh").first else { throw DriverError("no crew row") }
                await ui.capContext(on: row, name: "plan-crew-row-menu", title: "Crew, the right-click menu of a person", area: planArea, from: "plan-crew-list")
                try await ui.planContextPick(row, "Delete")
                try await ui.planWaitSheet()
                await ui.planShot("plan-crew-delete-sheet", "Crew, delete a person", from: "plan-crew-row-menu", via: "Delete")
                await ui.capRecover()
            }
            await ui.step("overview after casting") {
                try await ui.planTab("Overview")
                _ = try await ui.wait(id: "production-tile-cast", timeout: 10)
                await ui.planShot("plan-overview-cast", "Production Overview, after casting", from: "plan-cast-linked", via: "production-tab-overview")
            }
        },

        // MARK: Moodboards

        tour("09-plan-e-moodboards", order: 9500) { ui, _ in
            await ui.step("moodboard new") {
                try await ui.planTab("Moodboards")
                try await ui.capAct(id: "moodboard-new")
                _ = try await ui.wait(id: "mb-name", timeout: 8)
                await ui.idle(0.8)
                await ui.planShot("plan-moodboard-new-sheet", "New Moodboard, with suggestions and a scene to reference", from: "plan-moodboards-empty", via: "moodboard-new")
                await ui.planMenu(id: "mb-scene", "plan-moodboard-scene-menu", "New Moodboard, the scene menu", from: "plan-moodboard-new-sheet")
                if let s = ui.planRows("moodboard-suggestion-").first { try await ui.planPress(s) } else { try await ui.capTypeIn("Overall Look", into: "mb-name") }
                await ui.planShot("plan-moodboard-new-suggestion", "New Moodboard, a suggested name chosen", from: "plan-moodboard-new-sheet", via: "moodboard-suggestion")
                try await ui.capAct(id: "moodboard-new-sheet-primary")
                try await ui.planWaitNoSheet()
                _ = try await ui.wait(id: "moodboard-canvas", timeout: 8)
                await ui.planShot("plan-moodboard-canvas-empty", "Moodboard, a new empty board", from: "plan-moodboard-new-suggestion", via: "moodboard-new-sheet-primary")
            }
            await ui.planMenu(id: "moodboard-add-image", "plan-moodboard-add-image-menu", "Moodboard, the Add Image menu", from: "plan-moodboard-canvas-empty")
            await ui.step("moodboard vault sheet") {
                try await ui.capMenuButton(id: "moodboard-add-image", item: "From Idea Vault")
                try await ui.planWaitSheet()
                await ui.idle(0.8)
                await ui.planShot("plan-moodboard-vault-sheet", "Moodboard, add an image from the Idea Vault, none yet", from: "plan-moodboard-add-image-menu", via: "moodboard-add-image-vault")
                await ui.capRecover()
            }
            await ui.step("moodboard note") {
                try await ui.capAct(id: "moodboard-add-note")
                _ = try await ui.wait(id: "new-note", timeout: 8)
                await ui.planShot("plan-moodboard-note-sheet", "Moodboard, add a note", from: "plan-moodboard-canvas-empty", via: "moodboard-add-note")
                try await ui.capTypeIn("Sodium lamps on wet tarmac, the bus a warm box in the dark", into: "new-note")
                try await ui.capAct(id: "moodboard-note-sheet-primary")
                try await ui.planWaitNoSheet()
            }
            await ui.step("moodboard link") {
                try await ui.capAct(id: "moodboard-add-link")
                _ = try await ui.wait(id: "new-link", timeout: 8)
                await ui.planShot("plan-moodboard-link-sheet", "Moodboard, add a link", from: "plan-moodboard-canvas-empty", via: "moodboard-add-link")
                try await ui.capTypeIn("https://example.com/night-bus-colour", into: "new-link")
                try await ui.capTypeIn("Colour reference", into: "new-link-title")
                try await ui.capAct(id: "moodboard-link-sheet-primary")
                try await ui.planWaitNoSheet()
            }
            await ui.step("moodboard second note") {
                try await ui.capAct(id: "moodboard-add-note")
                _ = try await ui.wait(id: "new-note", timeout: 8)
                try await ui.capTypeIn("Faces lit only by phone screens", into: "new-note")
                try await ui.capAct(id: "new-note-private")
                await ui.planShot("plan-moodboard-note-private", "Moodboard, an internal note", from: "plan-moodboard-note-sheet", via: "new-note-private")
                try await ui.capAct(id: "moodboard-note-sheet-primary")
                try await ui.planWaitNoSheet()
                await ui.planShot("plan-moodboard-canvas-items", "Moodboard, notes and a link on the canvas", from: "plan-moodboard-canvas-empty", via: "moodboard-add-note")
            }
            // New tiles land on top of each other: select each by its visible top left corner and
            // move it with Shift and the arrow keys (the keyboard way to place a tile).
            await ui.step("moodboard spread tiles") {
                let tiles = ui.planRows("moodboard-tile-").sorted { $0.frame.minX < $1.frame.minX }
                for (i, t) in tiles.enumerated() where i > 0 {
                    try await ui.planTileClick(t)
                    for _ in 0..<(i * 9) { try await ui.key("\u{F703}", code: 124, modifiers: [.shift]); await ui.idle(0.05) }
                    for _ in 0..<(i * 2) { try await ui.key("\u{F701}", code: 125, modifiers: [.shift]); await ui.idle(0.05) }
                    await ui.idle(0.8)
                }
            }
            await ui.step("moodboard tile selected") {
                guard let tile = ui.planRows("moodboard-tile-").first(where: { $0.label.contains("Colour") }) ?? ui.planRows("moodboard-tile-").first else { throw DriverError("no tile") }
                try await ui.planTileClick(tile)
                if !ui.exists(id: "moodboard-front") { try await ui.planPress(tile) }
                _ = try await ui.wait(id: "moodboard-front", timeout: 5)
                await ui.planShot("plan-moodboard-tile-selected", "Moodboard, a tile selected with its caption bar", from: "plan-moodboard-canvas-items", via: "moodboard-tile")
                await ui.planContextShot(tile, "plan-moodboard-tile-menu", "Moodboard, the right-click menu of a tile", from: "plan-moodboard-tile-selected")
            }
            await ui.step("moodboard link inspector") {
                guard let tile = ui.planRows("moodboard-tile-").first(where: { $0.label.contains("Colour") }) else { throw DriverError("no link tile") }
                try await ui.planContextPick(tile, "Edit", corner: true)
                _ = try await ui.wait(id: "moodboard-item-url", timeout: 8)
                await ui.planShot("plan-moodboard-inspector-link", "Moodboard, a link open in the inspector", from: "plan-moodboard-tile-menu", via: "Edit")
                await ui.planCloseInspector()
            }
            await ui.step("moodboard note inspector") {
                guard let tile = ui.planRows("moodboard-tile-").first(where: { $0.label.contains("Faces") }) ?? ui.planRows("moodboard-tile-").last else { throw DriverError("no note tile") }
                try await ui.planContextPick(tile, "Edit", corner: true)
                _ = try await ui.wait(id: "moodboard-item-note", timeout: 8)
                await ui.planShot("plan-moodboard-inspector", "Moodboard, an internal note open in the inspector", from: "plan-moodboard-canvas-items", via: "Edit")
                await ui.planCloseInspector()
            }
            await ui.step("moodboard second board") {
                try await ui.capAct(id: "moodboard-new")
                _ = try await ui.wait(id: "mb-name", timeout: 8)
                await ui.idle(0.5)
                try await ui.capTypeIn("Costume", into: "mb-name")
                try await ui.capAct(id: "moodboard-new-sheet-primary")
                try await ui.planWaitNoSheet()
                await ui.planShot("plan-moodboard-two-boards", "Moodboards, two boards in the list", from: "plan-moodboard-canvas-items", via: "moodboard-new")
            }
            await ui.step("moodboard row menu") {
                guard let row = ui.planRows("moodboard-row-").first else { throw DriverError("no board row") }
                await ui.capContext(on: row, name: "plan-moodboard-row-menu", title: "Moodboards, the right-click menu of a board", area: planArea, from: "plan-moodboard-two-boards")
                for (item, name, title) in [("Rename", "plan-moodboard-rename-sheet", "Moodboards, rename a board"),
                                            ("Board Notes", "plan-moodboard-notes-sheet", "Moodboards, board notes"),
                                            ("Delete", "plan-moodboard-delete-sheet", "Moodboards, delete a board")] {
                    await ui.step("moodboard row \(item)") {
                        guard let row = ui.planRows("moodboard-row-").first else { throw DriverError("no board row") }
                        try await ui.planContextPick(row, item)
                        try await ui.planWaitSheet()
                        await ui.planShot(name, title, from: "plan-moodboard-row-menu", via: item)
                        await ui.capRecover()
                    }
                }
            }
            await ui.step("moodboard export") {
                try await ui.capAct(id: "moodboard-export")
                try await ui.planWaitSheet()
                await ui.planShot("plan-moodboard-export-sheet", "Moodboard, export as PDF", from: "plan-moodboard-two-boards", via: "moodboard-export")
                await ui.capRecover()
            }
        },

        // MARK: Storyboards

        tour("09-plan-f-storyboards", order: 9600) { ui, _ in
            await ui.step("storyboard scene") {
                try await ui.planTab("Storyboards")
                _ = try await ui.wait(id: "storyboard-scene-picker", timeout: 10)
                await ui.idle(0.8)
                await ui.planShot("plan-storyboard-scene-empty", "Storyboards, a scene with no panels yet", from: "plan-storyboards-empty", via: "production-tab-storyboards")
                await ui.planMenu(id: "storyboard-scene-picker", "plan-storyboard-scene-menu", "Storyboards, the scene menu", from: "plan-storyboard-scene-empty")
                await ui.planMenu(id: "storyboard-add-panel", "plan-storyboard-add-menu", "Storyboards, the Add Panel menu", from: "plan-storyboard-scene-empty")
            }
            await ui.step("storyboard sketch sheet") {
                try await ui.capAct(id: "storyboard-empty-sketch")
                try await ui.planWaitSheet()
                await ui.idle(0.6)
                await ui.planShot("plan-storyboard-sketch-sheet", "Storyboards, draw a sketch", from: "plan-storyboard-scene-empty", via: "storyboard-empty-sketch")
                await ui.capRecover()
            }
            await ui.step("storyboard panels") {
                try await ui.capAct(id: "storyboard-empty-panel")
                try await ui.capWaitUntil("a panel", timeout: 8) { !ui.planRows("panel-").filter { $0.identifier.count > 10 && !$0.identifier.hasPrefix("panel-inspector") && !$0.identifier.hasPrefix("panel-shot") }.isEmpty }
                await ui.idle(0.6)
                await ui.planCloseInspector()
                for _ in 0..<2 {
                    try await ui.capMenuButton(id: "storyboard-add-panel", item: "Empty Panel")
                    await ui.idle(0.8)
                    await ui.planCloseInspector()
                }
                await ui.planShot("plan-storyboard-panels", "Storyboards, a storyboard with three panels", from: "plan-storyboard-scene-empty", via: "storyboard-add-panel")
            }
            @MainActor func panels() -> [Element] { ui.planRows("panel-").filter { e in !["panel-inspector", "panel-shot", "panel-desc", "panel-framing", "panel-movement", "panel-angle", "panel-duration", "panel-sound", "panel-note", "panel-import", "panel-sketch", "panel-clear", "panel-move", "panel-delete", "panel-link", "panel-create", "panel-open", "panel-unlink"].contains { e.identifier.hasPrefix($0) } } }
            await ui.step("storyboard panel menu") {
                guard let p = panels().first else { throw DriverError("no panel") }
                await ui.capContext(on: p, name: "plan-storyboard-panel-menu", title: "Storyboards, the right-click menu of a panel", area: planArea, from: "plan-storyboard-panels")
            }
            await ui.step("storyboard panel open") {
                guard let p = panels().first else { throw DriverError("no panel") }
                try await ui.planPress(p)
                _ = try await ui.wait(id: "panel-inspector", timeout: 8)
                await ui.planShot("plan-storyboard-panel-open", "Storyboards, a panel open in the inspector", from: "plan-storyboard-panels", via: "panel")
                try await ui.capTypeIn("Arjun waits under the shelter, rain on the roof", into: "panel-desc")
                try await ui.capTypeIn("Wide", into: "panel-framing")
                try await ui.capTypeIn("Slow push in", into: "panel-movement")
                await ui.planShot("plan-storyboard-panel-filled", "Storyboard panel, description and framing", from: "plan-storyboard-panel-open", via: "panel-desc")
                try await ui.capTypeIn("abc", into: "panel-duration")
                try await ui.capTypeIn("Low angle", into: "panel-angle")
                await ui.idle(0.6)
                await ui.planShot("plan-storyboard-panel-duration-error", "Storyboard panel, a duration that is not a number", from: "plan-storyboard-panel-filled", via: "panel-duration")
                try await ui.capTypeIn("3.5", into: "panel-duration")
                try await ui.capTypeIn("Low angle", into: "panel-angle")
            }
            await ui.step("storyboard panel shot") {
                try await ui.scrollIntoViewPlan(id: "panel-create-shot")
                try await ui.capAct(id: "panel-create-shot")
                await ui.idle(1.0)
                await ui.planShot("plan-storyboard-panel-shot", "Storyboard panel, linked to a new shot", from: "plan-storyboard-panel-filled", via: "panel-create-shot")
                await ui.planCloseInspector()
                await ui.planShot("plan-storyboard-panels-described", "Storyboards, panels with a description and a shot", from: "plan-storyboard-panel-shot", via: "panel-inspector-close")
            }
            await ui.planMenu(id: "storyboard-actions", "plan-storyboard-actions-menu", "Storyboards, the storyboard actions menu", from: "plan-storyboard-panels")
            await ui.step("storyboard rename") {
                try await ui.capMenuButton(id: "storyboard-actions", item: "Rename")
                try await ui.planWaitSheet()
                await ui.planShot("plan-storyboard-rename-sheet", "Storyboards, rename a storyboard", from: "plan-storyboard-actions-menu", via: "Rename")
                await ui.capRecover()
            }
            await ui.step("storyboard new standalone") {
                try await ui.capAct(id: "storyboard-new")
                _ = try await ui.wait(id: "new-sb-name", timeout: 8)
                await ui.planShot("plan-storyboard-new-sheet", "New Storyboard, for the chosen scene", from: "plan-storyboard-panels", via: "storyboard-new")
                await ui.planMenu(id: "new-sb-scene", "plan-storyboard-new-scene-menu", "New Storyboard, the scene menu", from: "plan-storyboard-new-sheet")
                await ui.capRecover()
            }
            await ui.step("storyboard export") {
                try await ui.capAct(id: "storyboard-export")
                try await ui.planWaitSheet()
                await ui.planShot("plan-storyboard-export-sheet", "Storyboards, export", from: "plan-storyboard-panels", via: "storyboard-export")
                await ui.capRecover()
            }
        },

        // MARK: Shot Lists

        tour("09-plan-g-shots", order: 9700) { ui, _ in
            await ui.step("shots all scenes") {
                try await ui.planTab("Shot Lists")
                _ = try await ui.wait(id: "shots-scene-picker", timeout: 10)
                await ui.idle(0.8)
                await ui.planShot("plan-shots-all-empty", "Shot Lists, all scenes, no shots yet", from: "plan-shots-no-script", via: "production-tab-shotLists")
                await ui.planMenu(id: "shots-scene-picker", "plan-shots-scene-menu", "Shot Lists, the scene menu", from: "plan-shots-all-empty")
            }
            await ui.step("shots scene") {
                try await ui.planChooseScene("shots-scene-picker", 1)
                _ = try await ui.wait(id: "shots-composer", timeout: 8)
                await ui.planShot("plan-shots-scene-empty", "Shot Lists, a scene with no shots yet", from: "plan-shots-scene-menu", via: "shots-scene-picker")
                try await ui.capAct(id: "shots-composer-add")
                try await ui.waitText("Describe the shot to add it.")
                await ui.planShot("plan-shots-composer-error", "Shot Lists, add a shot with no description", from: "plan-shots-scene-empty", via: "shots-composer-add")
            }
            await ui.step("shots add") {
                for d in ["Wide: the bus pulls into the empty stop", "Medium: Arjun climbs aboard, shaking off rain", "Close: the driver's eyes in the mirror"] {
                    try await ui.capTypeIn(d, into: "shots-composer")
                    try await ui.capAct(id: "shots-composer-add")
                    await ui.idle(0.9)
                }
                await ui.planCloseInspector()
                await ui.planShot("plan-shots-list", "Shot Lists, three shots in the list", from: "plan-shots-scene-empty", via: "shots-composer-add")
            }
            await ui.step("shots grid") {
                try await ui.capTab("Grid")
                await ui.idle(0.6)
                await ui.planShot("plan-shots-grid", "Shot Lists, the Grid view", from: "plan-shots-list", via: "shots-layout")
                try await ui.capTab("List")
            }
            await ui.planMenu(id: "shots-more", "plan-shots-more-menu", "Shot Lists, more scene actions", from: "plan-shots-list")
            await ui.step("shots copy planning") {
                try await ui.capMenuButton(id: "shots-more", item: "Copy Planning")
                try await ui.planWaitSheet()
                await ui.planShot("plan-shots-copy-sheet", "Shot Lists, copy planning to another scene", from: "plan-shots-more-menu", via: "shots-more")
                await ui.capRecover()
            }
            @MainActor func rows() -> [Element] { ui.planRows("shot-").filter { e in !["shot-inspector", "shot-card", "shot-needs", "shot-desc", "shot-size", "shot-movement", "shot-angle", "shot-lens", "shot-camera", "shot-sound", "shot-technical", "shot-delete", "shot-duplicate", "shot-remove", "shot-characters", "shot-character", "shot-reference", "shot-open", "shot-detach", "shot-attach", "shot-create"].contains { e.identifier.hasPrefix($0) } } }
            await ui.step("shot row menu") {
                guard let r = rows().first else { throw DriverError("no shot row") }
                await ui.capContext(on: r, name: "plan-shot-row-menu", title: "Shot Lists, the right-click menu of a shot", area: planArea, from: "plan-shots-list")
            }
            await ui.step("shot move sheet") {
                guard let r = rows().first else { throw DriverError("no shot row") }
                try await ui.planContextPick(r, "Move to Scene")
                try await ui.planWaitSheet()
                await ui.planShot("plan-shot-move-sheet", "Shot Lists, move a shot to another scene", from: "plan-shot-row-menu", via: "Move to Scene")
                await ui.capRecover()
            }
            await ui.step("shot attach sheet") {
                guard let r = rows().first else { throw DriverError("no shot row") }
                try await ui.planContextPick(r, "Attach Storyboard")
                try await ui.planWaitSheet()
                await ui.planShot("plan-shot-attach-sheet", "Shot Lists, attach a storyboard panel, none yet", from: "plan-shot-row-menu", via: "Attach Storyboard")
                await ui.capRecover()
            }
            await ui.step("shot open") {
                guard let r = rows().first else { throw DriverError("no shot row") }
                try await ui.planContextPick(r, "Open")
                _ = try await ui.wait(id: "shot-inspector", timeout: 8)
                await ui.planShot("plan-shot-open", "Shot Lists, a shot open in the inspector", from: "plan-shot-row-menu", via: "Open")
                try await ui.capAct(id: "shot-technical")
                _ = try await ui.wait(id: "shot-size", timeout: 6)
                try await ui.capTypeIn("Wide", into: "shot-size")
                try await ui.capTypeIn("Static", into: "shot-movement")
                try await ui.capTypeIn("24mm", into: "shot-lens")
                try await ui.capTypeIn("Eye level", into: "shot-angle")
                await ui.idle(0.6)
                await ui.planShot("plan-shot-technical", "Shot, camera details filled in", from: "plan-shot-open", via: "shot-size")
                await ui.planMenu(id: "shot-character-suggestions", "plan-shot-characters-menu", "Shot, characters in this scene", from: "plan-shot-open")
                await ui.planCloseInspector()
                await ui.planShot("plan-shots-list-details", "Shot Lists, a shot with camera details", from: "plan-shot-technical", via: "shot-inspector-close")
            }
            await ui.step("shots export") {
                try await ui.capAct(id: "shots-export")
                try await ui.planWaitSheet()
                await ui.planShot("plan-shots-export-sheet", "Shot Lists, export", from: "plan-shots-list", via: "shots-export")
                await ui.capRecover()
            }
        },
    ]
}

fileprivate extension UI {
    /// Scrolls the page until the element is inside the window.
    func scrollIntoViewPlan(id: String) async throws {
        let e = try await wait(id: id, timeout: 8)
        if let v = e.obj as? NSView { v.scrollToVisible(v.bounds); await idle(0.4); return }
        if e.obj.responds(to: NSSelectorFromString("accessibilityScrollToVisible")) {
            _ = e.obj.perform(NSSelectorFromString("accessibilityScrollToVisible"))
            await idle(0.4)
        }
    }
}

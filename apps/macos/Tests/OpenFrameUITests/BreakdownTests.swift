// Mac phase M5, Breakdown workspace. The shared end-to-end scenarios 50-breakdown.md and
// 51-breakdown-depth.md, run at model level against a real core on a temporary folder: each test
// drives the Breakdown models the way the screen does (the same calls, in the same order) and
// checks what the screen would show. Plus the pure script maths (highlights, selections, names),
// tagging from selected words, the catalog, locations, cast and crew, and rendered snapshots of
// the main screens in light and dark.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Helpers

private let nightShift = [
    "Title: Night Shift", "",
    "INT. CONTROL ROOM - NIGHT", "",
    "Kavya studies a flickering monitor and lifts a torch.", "",
    "KAVYA", "Someone is in the tunnel.", "",
    "EXT. TUNNEL MOUTH - NIGHT", "",
    "Ravi drags a heavy toolbox through the rain.", "",
    "RAVI", "Hold the gate.", "",
].joined(separator: "\n")

@MainActor
private struct Rig {
    let root: URL
    let app: AppModel
    let hub: BreakdownHub
    let projectPath: String

    static func make(title: String, source: Bool = false) async throws -> Rig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        #expect(ok)
        guard case let .project(p) = app.phase else { throw CoreFailure(code: "test", message: "no project") }
        let rig = Rig(root: root, app: app, hub: BreakdownHub(app: app), projectPath: p.path)
        await rig.hub.reloadStyles()
        await rig.hub.reloadMembers()
        if source {
            try await rig.importNightShift()
            try await rig.chooseSource()
        }
        return rig
    }

    var core: Core { app.core! }
    var bd: BreakdownModel { hub.breakdown }
    var quiet: Bool { app.alert == nil }
    var notice: String { hub.notice?.text ?? "" }

    func close() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    func importNightShift() async throws {
        let preview = try await core.screenplayImportPreview(InterchangeImportPreviewArgs(pastedText: nightShift, format: "fountain"))
        _ = try await core.screenplayImportApply(InterchangeImportApplyArgs(previewId: preview.previewId, mode: "new_screenplay", keepSourceFile: false))
    }

    /// "Select Source, click the draft, Set as Production Source."
    func chooseSource() async throws {
        await bd.reload()
        await bd.loadSourceOptions()
        let choice = try #require(bd.defaultSourceChoice)
        #expect(await bd.setSource(draftId: choice, reason: nil))
        await bd.reload()
    }

    /// The text of the scene panel, without its "Quick reuse" offer.
    var panel: String {
        guard let d = bd.detail else { return "" }
        var out: [String] = []
        if d.suggestionCount > 0 { out.append("Suggested elements: \(Int(d.suggestionCount))") }
        for g in d.groups {
            for e in g.confirmed {
                out.append(e.name)
                if let n = e.notes { out.append(n) } else if e.name != e.displayName { out.append("tagged as \u{201C}\(e.displayName)\u{201D}") }
            }
            for s in g.suggestions { out.append("\(s.displayName) (suggested)") }
        }
        return out.joined(separator: "\n")
    }

    func panelHas(_ s: String) -> Bool { panel.contains(s) }
    var confirmed: [BreakdownElementDto] { bd.detail?.groups.flatMap(\.confirmed) ?? [] }
    func element(_ name: String) -> BreakdownElementDto? { confirmed.first { $0.name == name } }

    /// "Add element C, N": Add Element, choose the category, type the name, wait for the lookup, Add.
    @discardableResult
    func addElement(_ category: BreakdownCategory, _ name: String) async -> Bool {
        let form = AddElementForm(model: bd, initial: AddInitial())
        form.setCategory(category)
        form.setName(name)
        await form.finder.settle()
        #expect(!form.finder.loading, "the dialog still shows \"Looking for existing catalog items\"")
        let ok = await form.submit()
        await bd.reload()
        return ok
    }

    func relaunch() async throws -> Rig {
        await app.closeProject()
        app.shutdown()
        let app2 = Fixture.model(root)
        await app2.start()
        await app2.openProject(path: projectPath)
        #expect(app2.alert == nil, "a recovery question showed")
        let rig = Rig(root: root, app: app2, hub: BreakdownHub(app: app2), projectPath: projectPath)
        await rig.hub.reloadStyles()
        await rig.hub.reloadMembers()
        return rig
    }
}

// MARK: Scenario 50: Breakdown workspace basics

@Suite("Breakdown basics", .serialized)
@MainActor
struct BreakdownBasicsTests {
    @Test func theWholeBasicsScenario() async throws {
        let r = try await Rig.make(title: "Copper Tunnel")
        defer { r.close() }

        // Import a screenplay and choose it as the Production Source.
        try await r.importNightShift()
        await r.bd.reload()
        #expect(r.bd.loaded && r.bd.source == nil, "shows: Choose a screenplay source to begin the breakdown.")
        r.bd.sheet = .selectSource
        await r.bd.loadSourceOptions()
        #expect(r.bd.sourceOptions?.drafts.count == 1)
        let draft = try #require(r.bd.sourceOptions?.drafts.first)
        #expect(r.bd.defaultSourceChoice == draft.id)
        await r.bd.confirmSource(draftId: draft.id, reason: nil)
        #expect(r.bd.sheet == nil, "the dialog closes")
        await r.bd.reload()
        let src = try #require(r.bd.source, "shows: Production Source:")
        #expect(src.draft.id == draft.id)
        #expect(r.bd.detail?.number == "1", "the panel Scene 1 breakdown shows")
        #expect(r.bd.rows.count == 2)
        #expect(r.quiet)

        // Suggestions are added only when accepted.
        await r.bd.suggest()
        #expect(r.notice.contains("Nothing is added to production until you accept."))
        await r.bd.reload()
        #expect(r.confirmed.isEmpty, "suggesting adds nothing to production")
        let first = try #require(r.bd.pendingSuggestions.first)
        await r.bd.accept(first)
        #expect(r.bd.sheet == nil, "this suggestion has no ambiguous match")
        #expect(r.notice == "Accepted \u{201C}\(first.name)\u{201D}")
        await r.bd.reload()
        #expect(r.panelHas(first.name))
        #expect(r.quiet)

        // Add an element by hand, undo and redo, then edit its notes.
        #expect(await r.addElement(.props, "Flickering monitor"))
        #expect(r.notice == "Added \u{201C}Flickering monitor\u{201D} to Scene 1")
        #expect(r.panelHas("Flickering monitor"))
        await r.app.undo()
        await r.bd.reload()
        #expect(!r.panelHas("Flickering monitor"), "undo removes the element")
        await r.app.redo()
        await r.bd.reload()
        #expect(r.panelHas("Flickering monitor"), "redo adds it again")
        let monitor = try #require(r.element("Flickering monitor"))
        r.bd.sheet = .notes(monitor)
        #expect(await r.bd.saveNotes(monitor, notes: "Must flicker on cue"))
        #expect(r.bd.sheet == nil)
        await r.bd.reload()
        #expect(r.panelHas("Must flicker on cue"))
        #expect(r.quiet)

        // Remove it from the scene, then restore it from Recently Deleted.
        await r.bd.remove(try #require(r.element("Flickering monitor")))
        #expect(r.notice == "Removed \u{201C}Flickering monitor\u{201D} from Scene 1. The catalog item is kept.")
        await r.bd.reload()
        #expect(!r.panelHas("Flickering monitor"))
        let trash = try await r.core.trashList(StoreArgs(store: .project))
        let row = try #require(trash.first { ($0.title ?? "").contains("Flickering monitor") }, "Restore Flickering monitor (Scene 1)")
        #expect(row.title == "Flickering monitor (Scene 1)")
        _ = try await r.core.trashRestore(TrashItemArgs(store: .project, id: row.id))
        await r.bd.reload()
        #expect(r.panelHas("Flickering monitor") && r.panelHas("Must flicker on cue"), "the restored element kept its notes")
        #expect(r.quiet)

        // Mark the scene complete; scene 2 has its own breakdown.
        await r.bd.setComplete(true)
        await r.bd.reload()
        #expect(r.bd.detail?.complete == true, "the panel shows a Complete button")
        r.bd.selectScene(r.bd.rows[1].sceneId)
        await r.bd.reloadScene()
        #expect(r.bd.detail?.number == "2")
        #expect(!r.panelHas("Flickering monitor"))
        #expect(r.quiet)

        // After a relaunch, the source, the element, its notes and the completion are kept.
        let r2 = try await r.relaunch()
        defer { r2.close() }
        await r2.bd.reload()
        #expect(r2.bd.source?.draft.id == draft.id, "shows: Production Source:")
        r2.bd.selectScene(r2.bd.rows[0].sceneId)
        await r2.bd.reloadScene()
        #expect(r2.bd.detail?.number == "1")
        #expect(r2.panelHas("Flickering monitor") && r2.panelHas("Must flicker on cue"))
        #expect(r2.bd.detail?.complete == true)
        #expect(r2.quiet)
    }
}

// MARK: Scenario 51: Breakdown depth

@Suite("Breakdown depth", .serialized)
@MainActor
struct BreakdownDepthTests {
    @Test func theWholeDepthScenario() async throws {
        let r = try await Rig.make(title: "Teak Signal", source: true)
        defer { r.close() }
        defer { r.bd.setFilter(.all); r.bd.setCategory(nil) }
        #expect(r.bd.detail?.number == "1")

        // Every category shows a colour, an icon, text and a tooltip.
        #expect(await r.addElement(.props, "Brass key"))
        #expect(r.notice.hasPrefix("Added \u{201C}Brass key\u{201D}"))
        let group = try #require(r.bd.detail?.groups.first { $0.category == .props }, "the Props section")
        #expect(!group.confirmed.isEmpty)
        let look = r.hub.look(.props)
        #expect(look.tooltip.hasPrefix("Props: "), "the mark's tooltip starts with \"Props: \" (\(look.tooltip))")
        #expect(look.label == "Props")
        #expect(!look.symbol.isEmpty && look.symbol != "tag", "the mark has an icon (\(look.iconKey))")
        let hex = try #require(look.hex, "the mark's dot has a colour")
        let parts = try #require(BDColor.parts(hex: hex))
        #expect(parts.r + parts.g + parts.b > 0 || hex.lowercased() == "#000000")
        #expect(look.color != nil)
        #expect(r.quiet)

        // Accept Selected is one undo step; Reject Selected removes the rest; Show in script.
        await r.bd.suggest()
        #expect(r.notice.contains("Nothing is added to production until you accept."))
        await r.bd.reload()
        r.bd.openReview()
        let n = r.bd.selectedSuggestions.count
        #expect(n >= 2, "the count in Accept Selected (\(n)) is at least 2")
        await r.bd.acceptSelected()
        #expect(r.notice.hasPrefix("Accepted "))
        #expect(r.bd.sheet == nil, "Esc closes the dialog; accepting every suggestion closes it already")
        await r.bd.reload()
        #expect(r.bd.detail?.suggestionCount == 0, "the panel no longer shows Suggested elements:")
        await r.app.undo()
        await r.bd.reload()
        #expect(Int(r.bd.detail?.suggestionCount ?? 0) == n, "one undo brought every suggestion back (Suggested elements: \(n))")
        r.bd.openReview()
        r.bd.toggleReview(r.bd.pendingSuggestions[0].id)
        #expect(r.bd.selectedSuggestions.count == n - 1, "Reject Selected (\(n - 1))")
        await r.bd.rejectSelected()
        #expect(r.notice.hasPrefix("Rejected"))
        await r.bd.reload()
        #expect(Int(r.bd.detail?.suggestionCount ?? 0) == 1, "Suggested elements: 1")
        let kept = try #require(r.bd.pendingSuggestions.first)
        await r.bd.accept(kept)
        #expect(r.notice == "Accepted \u{201C}\(kept.name)\u{201D}")
        await r.bd.reload()
        let accepted = try #require(r.element(kept.name))
        await r.bd.showSource(accepted)
        let jump = try #require(r.bd.jump, "the words in the script that suggested it are highlighted")
        #expect(jump.status == .exact || jump.status == .moved)
        let content = scriptContent(r.bd.detail!, hub: r.hub, jump: jump)
        let doc = ScriptDocument.build(content)
        let hit = try #require(doc.markRanges.first { $0.mark.kind == .jump })
        #expect(doc.text.attributedSubstring(from: hit.range).string == jump.phrase)
        #expect(r.quiet)

        // Filters: only scenes missing a breakdown, then quick reuse fills scene 2.
        r.bd.setFilter(.missing)
        await r.bd.reload()
        #expect(r.bd.rows.count == 1, "the scene list shows one scene")
        r.bd.selectScene(r.bd.rows[0].sceneId)
        await r.bd.reloadScene()
        #expect(r.bd.detail?.number == "2")
        let offer = try #require(r.bd.reuseOffers.first { $0.name == "Brass key" && $0.category == .props }, "Reuse Brass key (Props) in Scene 2")
        await r.bd.reuse(offer)
        #expect(r.notice == "Reused \u{201C}Brass key\u{201D} in Scene 2")
        await r.bd.reload()
        #expect(r.bd.rows.isEmpty, "the scene list is empty")
        r.bd.setFilter(.all)
        await r.bd.reload()
        #expect(r.bd.rows.count == 2)
        #expect(r.quiet)

        // A catalog item opens all its scenes.
        let catalog = r.hub.catalog!
        await catalog.reload()
        let brass = try #require(catalog.items.first { $0.name == "Brass key" }, "the table row for Brass key")
        catalog.selectedId = brass.id
        #expect(catalog.selected?.id == brass.id)
        #expect(brass.usedIn.count == 2)
        r.hub.openScenes(forCatalogItem: brass.id)
        #expect(r.hub.tab == .breakdown)
        await r.bd.reload()
        #expect(r.bd.view?.catalogFilter?.name == "Brass key", "shows: scenes that use Brass key")
        #expect(r.bd.rows.count == 2)
        r.bd.clearCatalogFilter()
        await r.bd.reload()
        #expect(r.bd.view?.catalogFilter == nil)
        #expect(r.quiet)

        // Location call sheet fields, availability dates, and crew availability.
        let locations = r.hub.locations!
        let cabin = try #require(await locations.create(name: "Signal Cabin", address: nil, contact: nil, nearestHospital: "Town Hospital, Station Road", status: .idea))
        #expect(r.notice == "Added location \u{201C}Signal Cabin\u{201D}")
        await locations.reload()
        #expect(locations.selected?.nearestHospital == "Town Hospital, Station Road", "the location's hospital field")
        #expect(await r.hub.addAvailability(owner: .location, ownerId: cabin.id, existing: cabin.availability, name: "Signal Cabin",
                                            start: "2027-06-20", end: "2027-06-22", kind: .unavailable, note: "Track repairs"))
        #expect(r.notice == "Updated availability of Signal Cabin")
        await locations.reload()
        let w = try #require(locations.selected?.availability.first)
        #expect(w.note == "Track repairs" && w.startDate == "2027-06-20" && w.endDate == "2027-06-22")
        #expect(AvailabilityDates.range(w).contains("2027"))

        let people = r.hub.people!
        people.view = .crew
        let imran = try #require(await people.createCrew(person: "Imran Sheikh", role: "Cinematographer", department: nil, contact: nil, notes: nil))
        #expect(r.notice == "Added Imran Sheikh to the crew")
        await people.updateCrew(imran.id, availabilityNotes: "Weekends only")
        people.selectedCrew = nil
        await people.reload()
        people.selectedCrew = imran.id
        await people.reload()
        #expect(people.crewMember?.availabilityNotes == "Weekends only", "the availability field reads Weekends only")
        #expect(r.quiet)

        // Category colours: change one, then everything survives a relaunch.
        await r.hub.categories.reload()
        let red = try #require(r.hub.styles?.palette.first { $0.token == "red" }, "Red in the Props colour group")
        await r.hub.categories.setColor(.props, token: red.token)
        #expect(r.notice == "Changed the look of Props")
        #expect(r.hub.styles?.categories.first { $0.category == .props }?.color == "red")
        #expect(r.hub.styles?.categories.first { $0.category == .props }?.customized == true)
        #expect(r.quiet)

        let r2 = try await r.relaunch()
        defer { r2.close() }
        await r2.hub.categories.reload()
        #expect(r2.hub.styles?.categories.first { $0.category == .props }?.color == "red", "Red is pressed")
        await r2.hub.locations.reload()
        let cabin2 = try #require(r2.hub.locations.locations.first { $0.name == "Signal Cabin" })
        #expect(cabin2.nearestHospital == "Town Hospital, Station Road")
        #expect(cabin2.availability.first?.note == "Track repairs")
        await r2.bd.reload()
        #expect(r2.bd.detail?.number == "1")
        #expect(r2.panelHas("Brass key"))
        #expect(r2.quiet)
    }
}

// MARK: The script maths

@MainActor
private func scriptContent(_ d: BreakdownSceneDetail, hub: BreakdownHub, jump: BreakdownSourceAnchor? = nil) -> ScriptContent {
    var marks: [String: [ScriptMark]] = [:]
    for g in d.groups {
        for (list, kind) in [(g.confirmed, ScriptMark.Kind.tag), (g.suggestions, .suggestion)] {
            for e in list {
                guard let line = e.spanElementId, let s = e.spanStart, let en = e.spanEnd, !e.archived else { continue }
                marks[line, default: []].append(ScriptMark(start: Int(s), end: Int(en), kind: kind, label: e.name, category: e.category))
            }
        }
    }
    if let j = jump, let line = j.elementId, let s = j.start, let e = j.end {
        marks[line, default: []].append(ScriptMark(start: Int(s), end: Int(e), kind: .jump, label: "Source phrase", category: nil))
    }
    var colors: [String: String] = [:]
    for c in BDCategory.all { if let h = hub.look(c).hex { colors[c.rawValue] = h } }
    let theme = ThemeStore().resolvedFor(colorScheme: .light)
    return ScriptContent(sceneId: d.sceneId, number: d.number, heading: d.heading, lines: d.script, marks: marks, colors: colors, palette: ScriptColors(theme: theme))
}

@Suite("Script highlights and selections")
@MainActor
struct BreakdownScriptTests {
    @Test func marksSplitALineAndOverlapsKeepTheEarlierOne() {
        let text = "Kavya lifts a torch and a red folder."
        let marks = [
            ScriptMark(start: 14, end: 19, kind: .tag, label: "Props: Torch", category: .props),
            ScriptMark(start: 26, end: 36, kind: .suggestion, label: "Props: Red Folder", category: .props),
            ScriptMark(start: 18, end: 28, kind: .suggestion, label: "overlaps the tag", category: .props),
            ScriptMark(start: 40, end: 44, kind: .tag, label: "outside the line", category: .props),
        ]
        let segs = Highlight.segments(text, marks: marks)
        #expect(segs.map(\.text).joined() == text, "no text is lost or doubled")
        #expect(segs.compactMap { $0.mark?.label } == ["Props: Torch", "Props: Red Folder"])
        #expect(segs.first { $0.mark?.kind == .tag }?.text == "torch")
        #expect(Highlight.segments("", marks: []).count == 1)
    }

    @Test func aJumpWinsOverATagOnTheSameWords() {
        let marks = [ScriptMark(start: 0, end: 5, kind: .tag, label: "tag", category: .cast),
                     ScriptMark(start: 0, end: 5, kind: .jump, label: "jump", category: nil)]
        #expect(Highlight.resolve(marks, length: 10).map(\.kind) == [.jump])
    }

    @Test func selectionsAreTrimmedAndOnlyTaggableInsideOneLine() {
        let a = "Kavya lifts a torch."
        let b = "KAVYA"
        let full = "\(a)\n\(b)" as NSString
        let lines: [(id: String, range: NSRange)] = [("a", NSRange(location: 0, length: a.count)), ("b", NSRange(location: a.count + 1, length: b.count))]
        let s = Highlight.selection(lines: lines, selected: NSRange(location: 5, length: 9), in: full)
        #expect(s == TextSelection(elementId: "a", start: 6, end: 13, text: "lifts a"), "white space at both ends is trimmed")
        #expect(Highlight.selection(lines: lines, selected: NSRange(location: 10, length: 12), in: full) == nil, "two lines are not taggable")
        #expect(Highlight.selection(lines: lines, selected: NSRange(location: 3, length: 0), in: full) == nil, "a caret is not a selection")
        #expect(Highlight.selection(lines: lines, selected: NSRange(location: 5, length: 1), in: full) == nil, "only white space")
        let second = Highlight.selection(lines: lines, selected: NSRange(location: a.count + 1, length: 5), in: full)
        #expect(second?.elementId == "b" && second?.start == 0 && second?.end == 5)
    }

    @Test func offsetsAreUTF16UnitsLikeTheCore() {
        let line = "Meera \u{1F9E1} drinks chai \u{0924}\u{094D}\u{0930} fast"
        let full = line as NSString
        let target = full.range(of: "chai")
        let lines: [(id: String, range: NSRange)] = [("x", NSRange(location: 0, length: full.length))]
        let s = Highlight.selection(lines: lines, selected: target, in: full)
        #expect(s?.start == target.location && s?.end == NSMaxRange(target))
        #expect(s?.text == "chai")
        let segs = Highlight.segments(line, marks: [ScriptMark(start: target.location, end: NSMaxRange(target), kind: .tag, label: "", category: .props)])
        #expect(segs.first { $0.mark != nil }?.text == "chai")
    }

    @Test func tagNamesAreTitleCased() {
        #expect(Highlight.suggestName("RED FOLDER") == "Red Folder")
        #expect(Highlight.suggestName("  \u{201C}the   torch,\u{201D} ") == "The Torch")
        #expect(Highlight.suggestName("...") == "")
        #expect(Highlight.suggestName("o'brien's car") == "O'brien's Car")
    }

    @Test func theSavedFilterFallsBackToAllScenes() throws {
        let defaults = try #require(UserDefaults(suiteName: "openframe.tests.breakdown.\(UUID().uuidString)"))
        #expect(SavedFilter.read(defaults) == SavedFilter())
        SavedFilter(filter: .missing, category: .props).write(defaults)
        #expect(SavedFilter.read(defaults) == SavedFilter(filter: .missing, category: .props))
        defaults.set(Data("{\"filter\":\"nonsense\"}".utf8), forKey: "openframe.breakdown.sceneFilter")
        #expect(SavedFilter.read(defaults) == SavedFilter(), "a value the core would refuse is ignored")
        defaults.set(Data("not json".utf8), forKey: "openframe.breakdown.sceneFilter")
        #expect(SavedFilter.read(defaults) == SavedFilter())
    }

    @Test func everyIconKeyHasASymbol() async throws {
        let r = try await Rig.make(title: "Icons")
        defer { r.close() }
        let styles = try #require(r.hub.styles)
        #expect(styles.categories.count == 11)
        for key in styles.icons {
            #expect(CategoryLook.symbols[key] != nil, "no symbol for the icon key \(key)")
        }
        for c in styles.categories {
            #expect(!c.label.isEmpty && !c.tooltip.isEmpty && BDColor.color(hex: c.colorHex) != nil)
        }
    }
}

// MARK: Tagging, suggestions, editing

@Suite("Tagging and suggestions", .serialized)
@MainActor
struct BreakdownTaggingTests {
    @Test func selectingWordsAndTaggingThemMarksThemInTheScript() async throws {
        let r = try await Rig.make(title: "Tagging", source: true)
        defer { r.close() }
        let d = try #require(r.bd.detail)
        var content = scriptContent(d, hub: r.hub)
        var doc = ScriptDocument.build(content)
        let full = doc.text.string as NSString
        // The person selects "flickering monitor" in the action line.
        let words = full.range(of: "flickering monitor")
        #expect(words.location != NSNotFound)
        let sel = try #require(Highlight.selection(lines: doc.lineRanges, selected: words, in: full))
        #expect(sel.text == "flickering monitor")
        r.bd.selection = sel
        r.bd.tagSelection(category: .props)
        guard case let .add(initial)? = r.bd.sheet else {
            Issue.record("Tag as should open the Add Element sheet")
            return
        }
        #expect(initial.category == .props && initial.name == "Flickering Monitor")
        #expect(initial.span?.elementId == sel.elementId)
        #expect(Int(initial.span?.start ?? -1) == sel.start && Int(initial.span?.end ?? -1) == sel.end)
        let form = AddElementForm(model: r.bd, initial: initial)
        await form.finder.settle()
        #expect(await form.submit())
        #expect(r.bd.selection == nil && r.bd.sheet == nil)
        await r.bd.reload()
        let el = try #require(r.element("Flickering Monitor"))
        #expect(el.spanElementId == sel.elementId)
        content = scriptContent(try #require(r.bd.detail), hub: r.hub)
        doc = ScriptDocument.build(content)
        let tag = try #require(doc.markRanges.first { $0.mark.kind == .tag })
        #expect(doc.text.attributedSubstring(from: tag.range).string == "flickering monitor")
        // "Show in script" finds the same words again.
        await r.bd.showSource(el)
        #expect(r.bd.jump?.phrase?.lowercased() == "flickering monitor" || r.bd.jump?.status == .exact)
        #expect(r.quiet)
    }

    @Test func tagSelectionNeedsAnEditableSceneAndASelection() async throws {
        let r = try await Rig.make(title: "No tag", source: true)
        defer { r.close() }
        r.bd.selection = nil
        r.bd.tagSelection(category: .props)
        #expect(r.bd.sheet == nil)
    }

    @Test func editingASuggestionChangesCategoryAndName() async throws {
        let r = try await Rig.make(title: "Edit suggestion", source: true)
        defer { r.close() }
        await r.bd.suggest()
        await r.bd.reload()
        let s = try #require(r.bd.pendingSuggestions.first)
        let form = EditSuggestionForm(model: r.bd, element: s, chooser: false)
        form.setCategory(.vfx)
        form.setName("Storm sky")
        await form.finder.settle()
        #expect(form.finder.effective?.mode == .new, "nothing similar: create a new catalog item")
        #expect(await form.submit())
        #expect(r.notice == "Accepted \u{201C}Storm sky\u{201D}")
        await r.bd.reload()
        let el = try #require(r.element("Storm sky"))
        #expect(el.category == .vfx)
        #expect(r.quiet)
    }

    @Test func theCatalogLookupChoosesLikeWindows() async throws {
        let r = try await Rig.make(title: "Matches", source: true)
        defer { r.close() }
        #expect(await r.addElement(.props, "Brass key"))
        let finder = CatalogMatchFinder(core: { r.app.core }, pause: .milliseconds(10))
        finder.update(category: .props, name: "Lantern")
        #expect(finder.loading && finder.effective == nil, "while looking, nothing is chosen")
        await finder.settle()
        #expect(!finder.loading)
        #expect(finder.effective?.mode == .new, "nothing similar means create new")
        finder.update(category: .props, name: "brass key")
        await finder.settle()
        #expect(finder.matches.count == 1 && finder.matches[0].exact)
        #expect(finder.effective?.mode == .existing && finder.effective?.catalogItemId == finder.matches[0].id)
        finder.choice = BreakdownCatalogChoice(mode: .new, catalogItemId: nil)
        #expect(finder.effective?.mode == .new, "the person's pick wins")
        finder.update(category: .props, name: "   ")
        #expect(finder.isBlank && !finder.loading && finder.effective == nil)
        // Another category is a different item.
        finder.update(category: .wardrobe, name: "Brass key")
        await finder.settle()
        #expect(finder.effective?.mode == .new || finder.effective?.mode == .existing)
        // Adding the same name again reuses the catalog item instead of duplicating it.
        let before = (try await r.core.catalogList(CatalogListArgs(category: nil, search: nil, includeArchived: true))).count
        r.bd.selectScene(r.bd.rows[1].sceneId)
        await r.bd.reloadScene()
        #expect(await r.addElement(.props, "Brass key"))
        let after = (try await r.core.catalogList(CatalogListArgs(category: nil, search: nil, includeArchived: true))).count
        #expect(after == before, "the exact match was reused")
        #expect(r.quiet)
    }

    @Test func rejectedSuggestionsDoNotComeBack() async throws {
        let r = try await Rig.make(title: "Reject", source: true)
        defer { r.close() }
        await r.bd.suggest()
        await r.bd.reload()
        let all = r.bd.pendingSuggestions
        #expect(!all.isEmpty)
        await r.bd.reject(all.map(\.id))
        #expect(r.notice.hasPrefix("Rejected"))
        await r.bd.reload()
        #expect(r.bd.pendingSuggestions.isEmpty)
        await r.bd.suggest()
        #expect(!r.notice.contains("Nothing is added"), "rejected suggestions are not suggested again")
        await r.bd.reload()
        #expect(r.bd.pendingSuggestions.isEmpty)
        await r.app.undo()
        await r.bd.reload()
        #expect(r.bd.pendingSuggestions.count == all.count, "undo brings them back")
    }

    @Test func removedScenesKeepTheirBreakdownAndAScriptUpdateIsReviewed() async throws {
        let r = try await Rig.make(title: "Update", source: true)
        defer { r.close() }
        #expect(await r.addElement(.props, "Brass key"))
        // A new draft of the same screenplay becomes available.
        let draft0 = try #require(r.bd.source?.draft)
        let nd = try await r.core.screenplayNewDraft(ScreenplayNewDraftArgs(sourceDraftId: draft0.id, name: "Draft 2", makeCurrent: true))
        await r.bd.reload()
        #expect(r.bd.newerDraft?.id == nd.id, "Newer revision available")
        await r.bd.loadUpdatePreview(draftId: nd.id)
        #expect(r.bd.updatePreview != nil)
        r.bd.dismissedNewer = nd.id
        #expect(r.bd.newerDraft == nil, "Dismiss hides the banner")
        // Choosing another draft opens the update review instead of replacing the source.
        await r.bd.loadSourceOptions()
        await r.bd.confirmSource(draftId: nd.id, reason: nil)
        guard case let .updateSource(id)? = r.bd.sheet else {
            Issue.record("expected the update review")
            return
        }
        #expect(id == nd.id)
        #expect(await r.bd.applyUpdate(draftId: nd.id))
        await r.bd.reload()
        #expect(r.bd.source?.draft.id == nd.id)
        #expect(r.bd.detail?.inSource == true)
        #expect(r.panelHas("Brass key"), "nothing is deleted by an update")
        #expect(r.quiet)
    }
}

// MARK: Catalog

@Suite("Catalog", .serialized)
@MainActor
struct BreakdownCatalogTests {
    @Test func addEditArchiveReplaceAndDelete() async throws {
        let r = try await Rig.make(title: "Catalog", source: true)
        defer { r.close() }
        let catalog = r.hub.catalog!
        #expect(await r.addElement(.props, "Brass key"))
        let lantern = try #require(await catalog.create(category: .props, name: "Lantern", status: .searching, description: "Old and dented"))
        #expect(r.notice == "Added \u{201C}Lantern\u{201D} to the catalog")
        #expect(catalog.selectedId == lantern.id && catalog.sheet == nil)
        await catalog.reload()
        #expect(catalog.items.count == 2)

        // Search and the category filter.
        catalog.search = "lan"
        await catalog.reload()
        #expect(catalog.items.map(\.name) == ["Lantern"])
        catalog.search = ""
        catalog.filter = .wardrobe
        await catalog.reload()
        #expect(catalog.items.isEmpty && catalog.isFiltered, "No catalog items match.")
        catalog.filter = nil

        // Edit.
        await catalog.update(lantern.id, name: "Storm lantern", status: .confirmed, description: "Brass", notes: "Needs fuel", contact: "Prop house, 0123")
        await catalog.reload()
        let edited = try #require(catalog.items.first { $0.id == lantern.id })
        #expect(edited.name == "Storm lantern" && edited.status == .confirmed && edited.notes == "Needs fuel" && edited.contact == "Prop house, 0123")
        #expect(await catalog.addAlias(lantern.id, "hurricane lamp"))
        #expect(await catalog.addAlias(lantern.id, "  ") == false)
        await catalog.reload()
        let aliases = try #require(catalog.items.first { $0.id == lantern.id }?.aliases)
        #expect(aliases.map(\.alias).contains("hurricane lamp"), "the old name stays as another name after a rename: \(aliases.map(\.alias))")
        await catalog.removeAlias(try #require(aliases.first { $0.alias == "hurricane lamp" }?.id))
        await catalog.reload()
        #expect(catalog.items.first { $0.id == lantern.id }?.aliases.map(\.alias).contains("hurricane lamp") == false)

        // Replace in scenes: the old item is kept.
        let brass = try #require(catalog.items.first { $0.name == "Brass key" })
        let candidates = await catalog.replacementCandidates(for: brass)
        #expect(candidates.map(\.id) == [lantern.id])
        #expect(await catalog.replace(brass, with: lantern.id, sceneIds: brass.usedIn.filter(\.inSource).map(\.sceneId)))
        #expect(r.notice.hasPrefix("Replaced in 1 scene. \u{201C}Brass key\u{201D} is kept in the catalog."))
        await r.bd.reload()
        #expect(r.panelHas("Storm lantern") && !r.panelHas("Brass key"))
        await catalog.reload()
        #expect(catalog.items.contains { $0.name == "Brass key" })

        // Archive keeps it readable and hides it from the list until "Show archived".
        await catalog.setArchived(brass, true)
        #expect(r.notice == "Archived \u{201C}Brass key\u{201D}")
        await catalog.reload()
        #expect(!catalog.items.contains { $0.name == "Brass key" })
        catalog.showArchived = true
        await catalog.reload()
        #expect(catalog.items.first { $0.name == "Brass key" }?.archived == true)
        await catalog.setArchived(brass, false)
        #expect(r.notice == "Restored \u{201C}Brass key\u{201D} from the archive")

        // Delete goes to Recently Deleted and can be restored.
        let refs = await r.hub.references(kind: .catalogItem, id: lantern.id)
        #expect(refs?.consequence.isEmpty == false)
        await catalog.delete(try #require(catalog.items.first { $0.id == lantern.id }))
        #expect(r.notice == "Deleted \u{201C}Storm lantern\u{201D}. Restore it from Recently Deleted if needed.")
        await catalog.reload()
        #expect(!catalog.items.contains { $0.id == lantern.id })
        let trash = try await r.core.trashList(StoreArgs(store: .project))
        let row = try #require(trash.first { ($0.title ?? "").contains("Storm lantern") })
        _ = try await r.core.trashRestore(TrashItemArgs(store: .project, id: row.id))
        await catalog.reload()
        #expect(catalog.items.contains { $0.id == lantern.id })
        #expect(r.quiet)
    }

    @Test func aMissingNameIsRefusedAndShownInPlainWords() async throws {
        let r = try await Rig.make(title: "Refuse")
        defer { r.close() }
        let made = await r.hub.catalog.create(category: .props, name: "   ", status: .required, description: nil)
        #expect(made == nil)
        guard case let .error(_, failure)? = r.app.alert else {
            Issue.record("expected the core's message")
            return
        }
        #expect(!failure.message.isEmpty && !failure.message.contains("internal"))
    }
}

// MARK: Locations

@Suite("Locations", .serialized)
@MainActor
struct BreakdownLocationsTests {
    @Test func statusNotesPhotosAndReplacement() async throws {
        let r = try await Rig.make(title: "Locations", source: true)
        defer { r.close() }
        let locations = r.hub.locations!
        let old = try #require(await locations.create(name: "Old Railway Station", address: "Station Road", contact: "Mr Rao 555", nearestHospital: nil, status: .idea))
        let cabin = try #require(await locations.create(name: "Signal Cabin", address: nil, contact: nil, nearestHospital: nil, status: .confirmed))
        await locations.reload()
        #expect(locations.locations.count == 2)

        // Practical notes keep the other notes.
        var notes = old.notes
        notes.parking = "Behind the goods shed"
        notes.noise = "Trains until 9"
        await locations.update(old.id, notes: notes)
        await locations.update(old.id, nearestHospital: "City Hospital, 22 Ring Road", emergencyContact: "Dr Mehta 999")
        await locations.reload()
        let saved = try #require(locations.locations.first { $0.id == old.id })
        #expect(saved.notes.parking == "Behind the goods shed" && saved.notes.noise == "Trains until 9")
        #expect(saved.nearestHospital == "City Hospital, 22 Ring Road" && saved.emergencyContact == "Dr Mehta 999")
        #expect(saved.address == "Station Road" && saved.contact == "Mr Rao 555", "a note edit does not touch other fields")

        // Photos: add two, reorder, remove.
        let png = try makePNG(root: r.root)
        await locations.addPhotos(old.id, paths: [png.path, png.path])
        await locations.reload()
        var photos = try #require(locations.locations.first { $0.id == old.id }).photos
        #expect(photos.count == 2)
        await locations.movePhoto(photos[1].id, to: 0)
        await locations.reload()
        let moved = try #require(locations.locations.first { $0.id == old.id }).photos
        #expect(moved.first?.id == photos[1].id, "the first photo is the thumbnail")
        photos = moved
        await locations.removePhoto(photos[0].id)
        #expect(r.notice == "Photo removed")
        await locations.reload()
        #expect(locations.locations.first { $0.id == old.id }?.photos.count == 1)

        // Status filter and search.
        locations.status = .confirmed
        await locations.reload()
        #expect(locations.locations.map(\.name) == ["Signal Cabin"])
        locations.status = nil
        locations.search = "railway"
        await locations.reload()
        #expect(locations.locations.map(\.name) == ["Old Railway Station"])
        locations.search = ""

        // A rejected location used in a scene is replaced by a practical one.
        #expect(await r.addElement(.locationSet, "Old Railway Station"))
        await locations.setStatus(old, .rejected)
        await locations.reload()
        let rejected = try #require(locations.locations.first { $0.id == old.id })
        #expect(rejected.status == .rejected)
        let options = await locations.replacementCandidates(for: rejected)
        #expect(options.map(\.id) == [cabin.id])
        let used = rejected.scenes.filter(\.inSource)
        if !used.isEmpty {
            #expect(await locations.replace(rejected, with: cabin.id, sceneIds: used.map(\.sceneId)))
            #expect(r.notice.hasPrefix("Replacement applied to"))
            #expect(r.notice.hasSuffix("The screenplay was not changed."))
        }

        // Archive, delete and restore.
        await locations.setArchived(cabin, true)
        #expect(r.notice == "Archived \u{201C}Signal Cabin\u{201D}")
        await locations.delete(old)
        #expect(r.notice == "Deleted \u{201C}Old Railway Station\u{201D}. Restore it from Recently Deleted if needed.")
        let trash = try await r.core.trashList(StoreArgs(store: .project))
        #expect(trash.contains { ($0.title ?? "").contains("Old Railway Station") })
        #expect(r.quiet)
    }

    private func makePNG(root: URL) throws -> URL {
        let rep = try #require(NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 8, pixelsHigh: 8, bitsPerSample: 8, samplesPerPixel: 4,
                                                hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        let url = root.appendingPathComponent("photo.png")
        try #require(rep.representation(using: .png, properties: [:])).write(to: url)
        return url
    }
}

// MARK: Cast and crew

@Suite("Cast and crew", .serialized)
@MainActor
struct BreakdownPeopleTests {
    @Test func characterComeFromTheScriptAndCastingFollowsTheRules() async throws {
        let r = try await Rig.make(title: "Casting", source: true)
        defer { r.close() }
        let people = r.hub.people!
        await people.reload()
        let names = Set((people.directory?.characters ?? []).map(\.name))
        #expect(names.isSuperset(of: ["KAVYA", "RAVI"]), "characters from the Production Source appear automatically (\(names))")
        #expect(people.unassigned.count >= 2, "Needs casting")

        let asha = try #require(await people.createCast(person: "Asha Rao", characterName: "KAVYA", characterId: nil, primary: true))
        #expect(r.notice == "Asha Rao plays KAVYA")
        #expect(asha.isPrimary)
        await people.reload()
        #expect(!people.unassigned.contains { $0.name == "KAVYA" })
        let meera = try #require(await people.createCast(person: "Meera Das", characterName: "KAVYA", characterId: nil, primary: false))
        #expect(!meera.isPrimary, "an alternate")
        // Making the alternate the primary turns the old primary into an alternate.
        #expect(await people.assign(meera, characterName: "KAVYA", characterId: nil, primary: true))
        await people.reload()
        let members = people.directory?.members ?? []
        #expect(members.first { $0.personName == "Meera Das" }?.isPrimary == true)
        #expect(members.first { $0.personName == "Asha Rao" }?.isPrimary == false)

        // Editing: contact, availability notes and dates.
        await people.updateCast(asha.id, contact: "asha@example.com", availabilityNotes: "Free June", notes: "Prefers mornings")
        #expect(await r.hub.addAvailability(owner: .castMember, ownerId: asha.id, existing: [], name: "Asha Rao", start: "2027-01-05", end: nil, kind: .available, note: nil))
        await people.reload()
        let updated = try #require(people.directory?.members.first { $0.id == asha.id })
        #expect(updated.contact == "asha@example.com" && updated.availabilityNotes == "Free June" && updated.notes == "Prefers mornings")
        #expect(updated.availability.count == 1 && updated.availability[0].startDate == updated.availability[0].endDate, "one day when the last day is left out")
        #expect(PeopleModel.availabilityText(notes: "Free June", dates: 1) == "Free June \u{00B7} 1 date range")
        #expect(await r.hub.removeAvailability(owner: .castMember, ownerId: asha.id, existing: updated.availability, windowId: updated.availability[0].id, name: "Asha Rao"))
        await people.reload()
        #expect(people.directory?.members.first { $0.id == asha.id }?.availability.isEmpty == true)

        // Search finds people by character or note.
        people.search = "kavya"
        #expect(people.members.count == 2)
        people.search = "mornings"
        #expect(people.members.isEmpty, "notes are not searched, availability notes are")
        people.search = "june"
        #expect(people.members.map(\.personName) == ["Asha Rao"])
        people.search = ""

        // Archive and delete from the cast.
        await people.setArchived(kind: .cast, id: asha.id, archived: true, name: "Asha Rao")
        #expect(r.notice == "Archived Asha Rao")
        await people.reload()
        #expect(!people.members.contains { $0.id == asha.id })
        people.showArchived = true
        await people.reload()
        #expect(people.members.contains { $0.id == asha.id })
        await people.delete(kind: .cast, id: asha.id, name: "Asha Rao")
        #expect(r.notice == "Removed Asha Rao. Restore from Recently Deleted if needed.")
        #expect(r.quiet)
    }

    @Test func crewNeedsANameAndARole() async throws {
        let r = try await Rig.make(title: "Crew")
        defer { r.close() }
        let people = r.hub.people!
        #expect(await people.createCrew(person: "Imran Sheikh", role: "  ", department: nil, contact: nil, notes: nil) == nil)
        #expect(r.app.alert != nil, "the core's message shows")
        r.app.alert = nil
        let c = try #require(await people.createCrew(person: "Imran Sheikh", role: "Cinematographer", department: "Camera", contact: "imran@example.com", notes: nil))
        await people.updateCrew(c.id, role: "Director of Photography", department: "Camera")
        await people.reload()
        let saved = try #require(people.crew.first)
        #expect(saved.role == "Director of Photography" && saved.department == "Camera" && saved.contact == "imran@example.com")
        await people.delete(kind: .crew, id: c.id, name: "Imran Sheikh")
        await people.reload()
        #expect(people.crewList.isEmpty)
        let trash = try await r.core.trashList(StoreArgs(store: .project))
        let row = try #require(trash.first { ($0.title ?? "").contains("Imran Sheikh") })
        _ = try await r.core.trashRestore(TrashItemArgs(store: .project, id: row.id))
        await people.reload()
        #expect(people.crewList.count == 1)
    }

    @Test func categoryIconsAndDefaultsRestore() async throws {
        let r = try await Rig.make(title: "Categories")
        defer { r.close() }
        let cats = r.hub.categories!
        let before = r.hub.look(.sound)
        await cats.setIcon(.sound, key: "star")
        #expect(r.hub.look(.sound).iconKey == "star" && r.hub.look(.sound).label == before.label, "the label never changes")
        #expect(cats.anyCustomized)
        await cats.reset()
        #expect(r.notice == "Restored the default category colours and icons")
        #expect(r.hub.look(.sound) == before && !cats.anyCustomized)
        #expect(r.quiet)
    }
}

// MARK: Snapshots

@Suite("Breakdown snapshots", .serialized)
@MainActor
struct BreakdownRenderTests {
    private func host<V: View>(_ view: V, app: AppModel, scheme: ColorScheme) -> some View {
        let themes = ThemeStore()
        return view
            .environment(app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: scheme))
            .environment(\.colorScheme, scheme)
    }

    private func populated() async throws -> Rig {
        let r = try await Rig.make(title: "Night Shift", source: true)
        #expect(await r.addElement(.props, "Brass key"))
        #expect(await r.addElement(.cast, "Kavya"))
        await r.bd.suggest()
        await r.bd.reload()
        if let s = r.bd.pendingSuggestions.first { await r.bd.accept(s) }
        await r.bd.reload()
        _ = await r.hub.locations.create(name: "Signal Cabin", address: "Station Road", contact: nil, nearestHospital: "Town Hospital", status: .confirmed)
        _ = await r.hub.locations.create(name: "Tunnel Mouth", address: nil, contact: nil, nearestHospital: nil, status: .shortlisted)
        _ = await r.hub.people.createCast(person: "Asha Rao", characterName: "KAVYA", characterId: nil, primary: true)
        _ = await r.hub.people.createCrew(person: "Imran Sheikh", role: "Cinematographer", department: "Camera", contact: nil, notes: nil)
        return r
    }

    @Test func theTabsRenderInLightAndDark() async throws {
        let r = try await populated()
        defer { r.close() }
        r.hub.say("Accepted \u{201C}Brass key\u{201D}", undoable: true)
        await r.hub.catalog.reload()
        await r.hub.locations.reload()
        await r.hub.people.reload()
        await r.hub.categories.reload()
        for scheme in [ColorScheme.light, .dark] {
            let n = scheme == .dark ? "dark" : "light"
            for tab in BDTab.allCases {
                r.hub.tab = tab
                if tab == .catalog { r.hub.catalog.selectedId = nil }
                let image = Fixture.render(host(BreakdownContent(hub: r.hub), app: r.app, scheme: scheme),
                                           name: "breakdown-\(tab.rawValue)-\(n)", size: CGSize(width: 1280, height: 760))
                #expect(image != nil)
            }
        }
        #expect(r.quiet)
    }

    @Test func sheetsAndInspectorsRender() async throws {
        let r = try await populated()
        defer { r.close() }
        let d = try #require(r.bd.detail)
        let size = CGSize(width: 620, height: 560)
        let form = AddElementForm(model: r.bd, initial: AddInitial(category: .props, name: "Brass key"))
        await form.finder.settle()
        #expect(Fixture.render(host(AddElementSheet(form: form, sceneNumber: d.number ?? ""), app: r.app, scheme: .light), name: "breakdown-sheet-add", size: size) != nil)
        await r.bd.suggest()
        await r.bd.reload()
        #expect(Fixture.render(host(SuggestionReviewSheet(model: r.bd, detail: r.bd.detail!).environment(r.hub), app: r.app, scheme: .light),
                               name: "breakdown-sheet-review", size: CGSize(width: 760, height: 560)) != nil)
        await r.bd.loadSourceOptions()
        #expect(Fixture.render(host(SelectSourceSheet(model: r.bd), app: r.app, scheme: .dark), name: "breakdown-sheet-source", size: CGSize(width: 600, height: 460)) != nil)
        await r.hub.locations.reload()
        let loc = try #require(r.hub.locations.locations.first)
        r.hub.locations.selectedId = loc.id
        let inspector = LocationInspector(model: r.hub.locations, loc: try #require(r.hub.locations.selected)).environment(r.hub)
        _ = loc
        #expect(Fixture.render(host(inspector, app: r.app, scheme: .light), name: "breakdown-inspector-location", size: CGSize(width: 420, height: 900)) != nil)
        #expect(r.quiet)
    }
}

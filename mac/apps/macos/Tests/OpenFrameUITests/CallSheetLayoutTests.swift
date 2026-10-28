// The call sheet screen reads heading by heading like the exported PDF (Windows b1527711,
// CallSheetEditor.test.tsx "reads heading by heading like the exported PDF"), and the paper wraps
// on a narrow window instead of scrolling sideways (Windows 5bc07a01).

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI
import Testing
@testable import OpenFrameUI

@Suite("Call sheet paper in the PDF order", .serialized)
@MainActor
struct CallSheetLayoutTests {
    private func sheet() async throws -> (CSRig, CallSheetEditorModel) {
        var r = try await CSRig.make()
        try await r.importSample()
        try await r.plan()
        return (r, try await r.createSheet())
    }

    @Test func theDefaultSheetHasTheMastheadTheTimeCalloutAndTheSixSections() async throws {
        let (r, e) = try await sheet()
        defer { r.close() }
        let doc = try #require(e.doc)
        let d = try #require(e.detail)
        // The core names the six default sections as the PDF does.
        #expect(d.defaultSections == ["Day and times", "Production", "Locations", "Scene schedule", "Talent / Cast", "Practical / Safety information"])
        // Without the added sections (the core may add some from the project), the six defaults remain.
        var bare = doc
        bare.optional.weather = nil; bare.optional.extras = nil; bare.optional.departments = nil
        bare.optional.advance = nil; bare.optional.crew = nil; bare.optional.communication = nil
        bare.optional.specialNotes = nil; bare.optional.attachments = nil; bare.optional.referenceImages = nil
        let blocks = CSLayout.blocks(bare)
        #expect(blocks == [.masthead, .times, .production, .locations, .scenes, .peopleGroup, .talent, .logisticsGroup, .practical])
        // The masthead and the crew call box carry no section heading, as in the PDF.
        #expect(CSBlock.masthead.heading == nil && CSBlock.times.heading == nil)
        #expect(blocks.compactMap(\.heading) == ["Production", "Locations", "Scene schedule", "Cast & Crew", "Talent / Cast", "Production notes", "Practical / Safety information"])
        #expect(blocks.filter { $0 != .peopleGroup && $0 != .logisticsGroup }.compactMap(\.heading)
            == Array(d.defaultSections.dropFirst()), "the paper's sections and the core's default sections differ")
        // Masthead: the counts line and the shoot day block.
        let scenes = doc.scenes.filter { $0.kind.isEmpty }.count
        #expect(CSLayout.counts(doc) == "\(CSLayout.plural(scenes, "scene", "scenes")) \u{00B7} \(CSLayout.plural(doc.cast.count, "cast member", "cast members")) \u{00B7} \(CSLayout.plural(doc.locations.count, "location", "locations"))")
        #expect(CSLayout.shootDay("Shoot Day 4") == ("Shoot day", "04"))
        #expect(CSLayout.shootDay("Shoot Day 12") == ("Shoot day", "12"))
        #expect(CSLayout.shootDay("Pickup day") == ("Pickup day", nil))
        #expect(CSLayout.shootDay("  ") == ("Shoot day", nil))
        // The crew call box: the crew call, then the other times with the PDF's short labels.
        #expect(CSLayout.times.map(\.0) == ["General call", "Shooting call", "Breakfast", "Lunch", "Est. wrap"])
        #expect(CSLayout.times.last?.1 == "Estimated wrap", "the field keeps its full name for assistive tech")
        // Notes beside the headings.
        #expect(CSLayout.note(.locations, doc, readOnly: false) == CSLayout.plural(doc.locations.count, "location", "locations"))
        #expect(CSLayout.note(.scenes, doc, readOnly: false) == CSLayout.plural(scenes, "scheduled scene", "scheduled scenes"))
        #expect(CSLayout.note(.talent, doc, readOnly: false) == CSLayout.plural(doc.cast.count, "cast member", "cast members"))
        #expect(CSLayout.note(.practical, doc, readOnly: false) == "Review before issuing")
        #expect(CSLayout.note(.practical, doc, readOnly: true) == "Quick reference")
        #expect(r.quiet)
    }

    @Test func withEverySectionAddedItReadsHeadingByHeadingLikeThePDF() async throws {
        let (r, e) = try await sheet()
        defer { r.close() }
        let defs = try #require(e.detail?.optionalSections)
        for def in defs { await e.addSection(key: def.key, label: def.label) }
        e.addCustomField()
        let doc = try #require(e.doc)
        #expect(CSLayout.blocks(doc).compactMap(\.heading) == [
            "Production",
            "Locations",
            "Scene schedule",
            "Cast & Crew",
            "Talent / Cast",
            "Extras",
            "Crew",
            "Department notes",
            "Production notes",
            "Practical / Safety information",
            "Weather",
            "Advance schedule",
            "Walkie channels and contacts",
            "Special notes",
            "Attachments & reference images",
        ])
        // Inside the PDF's sections, the fields keep the PDF's order.
        #expect(CSLayout.productionColumns.flatMap { $0.map(\.0) } == ["Production company", "Production address", "Producer", "Director", "1st AD", "Production contact"])
        #expect(CSLayout.locationFields.map(\.0) == ["Address", "Meeting point", "Access", "Parking", "Nearest hospital", "Emergency contact", "Notes"])
        #expect(CSLayout.practicalLabels(doc) == ["Production emergency contact"] + doc.locations.map { "\($0.name) \u{00B7} Nearest hospital" } + [
            "Parking", "Meeting point", "Transport", "Meals", "Toilets", "Access", "Safety",
            "Special instructions", "Production notes", "Day notes",
        ])
        #expect(CSLayout.advanceFields.first?.0 == "Day")
        #expect(CSLayout.communicationFields.map(\.0) == ["Walkie channels", "Emergency contact", "Production contact", "Note"])
        #expect(CSLayout.sceneColumns[2] == "Heading / description")
        #expect(CSLayout.note(.crew, doc, readOnly: false) == CSLayout.plural(doc.optional.crew?.count ?? 0, "crew member listed", "crew members listed"))
        // Custom fields now live in the practical card: no section of their own.
        #expect(!doc.extraFields.isEmpty)
        #expect(!CSLayout.blocks(doc).compactMap(\.heading).contains("Additional information"))
        #expect(r.quiet)
    }

    @Test func attachmentsAndReferenceImagesShareOneHeadingAndEachCanBeRemoved() async throws {
        let (r, e) = try await sheet()
        defer { r.close() }
        let defs = try #require(e.detail?.optionalSections)
        let images = try #require(defs.first { $0.key == "referenceImages" })
        await e.addSection(key: images.key, label: images.label)
        // Reference images alone still show the merged section.
        #expect(CSLayout.blocks(try #require(e.doc)).last == .attachments)
        let files = try #require(defs.first { $0.key == "attachments" })
        await e.addSection(key: files.key, label: files.label)
        #expect(CSLayout.blocks(try #require(e.doc)).filter { $0 == .attachments }.count == 1)
        // Removing both takes the merged heading away.
        e.edit { $0.optional.attachments = nil; $0.optional.referenceImages = nil }
        #expect(!CSLayout.blocks(try #require(e.doc)).contains(.attachments))
        #expect(await e.flush())
        #expect(r.quiet)
    }

    @Test func thePaperWrapsOnANarrowWindowInsteadOfScrollingSideways() {
        // Two cards of at least 260 points: side by side on a wide paper, one under the other below about 530.
        #expect(CSWrap.columns(width: 808, count: 2, minWidth: 260, spacing: 10) == 2)
        #expect(CSWrap.columns(width: 520, count: 2, minWidth: 260, spacing: 10) == 1)
        #expect(CSWrap.columns(width: 200, count: 2, minWidth: 260, spacing: 10) == 1, "never fewer than one column")
        // The crew call box: six cells of at least 90 points, wrapping onto a second row when narrow.
        #expect(CSWrap.columns(width: 760, count: 6, minWidth: 90, spacing: 0) == 6)
        #expect(CSWrap.columns(width: 400, count: 6, minWidth: 90, spacing: 0) == 4)
        #expect(CSWrap.columns(width: .infinity, count: 6, minWidth: 90, spacing: 0) == 6)
        #expect(CSWrap.columns(width: 500, count: 0, minWidth: 90, spacing: 0) == 1)
    }

    @Test func theNarrowPaperIsNoWiderThanItsWindow() async throws {
        let (r, e) = try await sheet()
        defer { r.close() }
        let defs = try #require(e.detail?.optionalSections)
        for def in defs { await e.addSection(key: def.key, label: def.label) }
        _ = NSApplication.shared
        let themes = ThemeStore()
        let width: CGFloat = 520
        // The document's own width is what it takes at that window width.
        let paper = CallSheetPaper(model: e)
            .environment(r.app)
            .environment(themes)
            .environment(\.ofTheme, themes.resolvedFor(colorScheme: .light))
        let measure = NSHostingController(rootView: paper)
        let fit = measure.sizeThatFits(in: CGSize(width: width, height: 10_000))
        #expect(fit.width <= width + 0.5, "the paper is \(fit.width) points wide in a \(width) point window")
        #expect(r.quiet)
    }
}

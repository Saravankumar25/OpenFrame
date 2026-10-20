// Capture tour section: Production plan (Overview, Catalog, Locations, Cast and Crew, Moodboards, Storyboards, Shot Lists). Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureVisual: [Scenario] = [
        tour("09-visual", order: 9000) { ui, _ in
            try await ui.capNav("Visual Planning")
            await ui.capture("visual-moodboards", title: "Visual Planning: Moodboards", area: "Visual Planning")
            await ui.step("new moodboard") {
                try await ui.capAct(id: "moodboard-new")
                _ = try await ui.wait(id: "mb-name", timeout: 6)
                await ui.capture("moodboard-new-sheet", title: "New moodboard", area: "Visual Planning", from: "visual-moodboards", via: "moodboard-new")
                try await ui.capTypeIn("Night Bus look", into: "mb-name")
                try await ui.capAct(id: "moodboard-new-sheet-primary")
                await ui.idle(1.0)
                await ui.capture("moodboard-canvas", title: "Moodboard canvas", area: "Visual Planning", from: "moodboard-new-sheet")
            }
            await ui.exploreMenu("moodboard-add-image", area: "Visual Planning", prefix: "moodboard-add-image", from: "moodboard-canvas", limit: 3)
            await ui.step("moodboard note") {
                try await ui.capAct(id: "moodboard-add-note")
                await ui.idle(0.8)
                await ui.capture("moodboard-add-note", title: "Moodboard: add a note", area: "Visual Planning", from: "moodboard-canvas", via: "moodboard-add-note")
                try? await ui.fill(nil, "Sodium lamps, wet tarmac", id: "new-note")
                await ui.capRecover()
            }
            await ui.exploreButtons([("moodboard-add-link", "Add a link"), ("moodboard-export", "Export moodboard"), ("moodboard-back", "Back to moodboards")], area: "Visual Planning", prefix: "moodboard", from: "moodboard-canvas")
            await ui.eachTab(group: "visual-tabs", area: "Visual Planning", prefix: "visual-tab", from: "visual-moodboards")
            await ui.step("storyboard") {
                try await ui.capTab("Storyboards")
                await ui.exploreButtons([("storyboard-new", "New storyboard"), ("storyboard-add-panel", "Add a panel"), ("storyboard-add-empty", "Add an empty panel"), ("storyboard-empty-new", "New storyboard")], area: "Visual Planning", prefix: "storyboard", from: "visual-tab-storyboards")
            }
            await ui.step("shots") {
                try await ui.capTab("Shot Lists")
                await ui.exploreButtons([("shots-add", "Add a shot"), ("shots-more", "More"), ("shots-export", "Export shot list")], area: "Visual Planning", prefix: "shots", from: "visual-tab-shot-lists")
            }
        },
    ]
}

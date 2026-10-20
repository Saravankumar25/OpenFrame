// Capture tour section: Breakdown. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureBreakdown: [Scenario] = [
        tour("05-breakdown", order: 5000) { ui, _ in
            try await ui.capNav("Breakdown")
            await ui.capture("breakdown-no-source", title: "Breakdown, choose a screenplay source", area: "Breakdown")
            await ui.step("source") {
                try await ui.capAct(id: "select-source")
                let deadline = Date().addingTimeInterval(15)
                while Date() < deadline, ui.capQueryPrefix("draft-").isEmpty { await ui.idle(0.2) }
                await ui.capture("breakdown-source-sheet", title: "Choose the production source", area: "Breakdown", from: "breakdown-no-source", via: "select-source")
                guard let d = ui.capQueryPrefix("draft-").first else { throw DriverError("no draft") }
                try await ui.press(d)
                try await ui.capAct(id: "set-production-source")
                try await ui.waitText("Production Source:")
                await ui.idle(0.6)
            }
            await ui.capture("breakdown-scenes", title: "Breakdown: scene list and scene panel", area: "Breakdown", from: "breakdown-source-sheet", via: "set-production-source")
            await ui.step("suggest") {
                try await Scenarios.capSelectScene(ui, 1)
                try await ui.wait(label: "Scene 1 breakdown", timeout: 15)
                await ui.capture("breakdown-scene-1", title: "Breakdown: scene 1 panel", area: "Breakdown", from: "breakdown-scenes", via: "scene-list")
                try await ui.capAct(id: "suggest-elements")
                try await ui.waitText("Nothing is added to production until you accept.")
                await ui.capture("breakdown-suggestions", title: "Breakdown: suggested elements waiting for review", area: "Breakdown", from: "breakdown-scene-1", via: "suggest-elements")
                if let b = Scenarios.capAccepts(ui).first { try await ui.press(b); await ui.idle(0.6) }
                await ui.capture("breakdown-accepted-one", title: "Breakdown: one suggestion accepted", area: "Breakdown", from: "breakdown-suggestions", via: "accept")
                var n = 0
                while n < 30, let b = Scenarios.capAccepts(ui).first { try await ui.press(b); n += 1; await ui.idle(0.4) }
                await ui.capture("breakdown-all-accepted", title: "Breakdown: all suggestions accepted", area: "Breakdown", from: "breakdown-accepted-one")
            }
            await ui.step("add element") {
                try await ui.capAct(id: "add-element")
                _ = try await ui.wait(id: "add-name", timeout: 6)
                await ui.capture("breakdown-add-element-sheet", title: "Add an element to the scene", area: "Breakdown", from: "breakdown-all-accepted", via: "add-element")
                await ui.capMenu(id: "add-category", name: "breakdown-add-category-menu", title: "Add element: category", area: "Breakdown", from: "breakdown-add-element-sheet")
                try await ui.capTypeIn("Flickering monitor", into: "add-name")
                await ui.idle(1.2)
                await ui.capture("breakdown-add-element-filled", title: "Add an element, name typed", area: "Breakdown", from: "breakdown-add-element-sheet", via: "add-name")
                await ui.capRecover()
            }
            await ui.step("second scene") {
                try await Scenarios.capSelectScene(ui, 2)
                await ui.idle(0.8)
                await ui.capture("breakdown-scene-2", title: "Breakdown: scene 2 panel", area: "Breakdown", from: "breakdown-scenes", via: "scene-list")
            }
            await ui.exploreButtons([("mark-complete", "Mark scene complete"), ("scene-filter", "Scene filter"), ("script-tag-menu", "Tag selection"), ("quick-reuse", "Quick reuse"),
                                     ("review-suggestions", "Review suggestions"), ("change-source", "Change source"), ("open-needs-review", "Needs review"), ("removed-scenes", "Removed scenes")],
                                    area: "Breakdown", prefix: "breakdown", from: "breakdown-scenes")
            await ui.eachTab(group: "breakdown-tabs", area: "Breakdown", prefix: "breakdown-tab", from: "breakdown-scenes")
            await ui.step("catalog") {
                if ui.exists(id: "catalog-tab") { try await ui.capAct(id: "catalog-tab"); await ui.idle(0.6); await ui.capture("breakdown-catalog", title: "Breakdown: catalog of elements", area: "Breakdown", from: "breakdown-scenes", via: "catalog-tab") }
                if ui.exists(id: "catalog-show-archived") { try await ui.capAct(id: "catalog-show-archived"); await ui.idle(0.4); await ui.capture("breakdown-catalog-archived", title: "Catalog with archived items", area: "Breakdown", from: "breakdown-catalog", via: "catalog-show-archived"); try? await ui.capAct(id: "catalog-show-archived") }
            }
            await ui.step("cast crew") {
                if ui.exists(id: "cast-crew-tab") { try await ui.capAct(id: "cast-crew-tab"); await ui.idle(0.6); await ui.capture("breakdown-cast-crew", title: "Breakdown: cast and crew", area: "Breakdown", from: "breakdown-scenes", via: "cast-crew-tab")
                    await ui.exploreButtons([("add-person", "Add a person"), ("people-view", "People view")], area: "Breakdown", prefix: "breakdown-people", from: "breakdown-cast-crew") }
            }
            await ui.step("locations") {
                if ui.exists(id: "locations-tab") { try await ui.capAct(id: "locations-tab"); await ui.idle(0.6); await ui.capture("breakdown-locations", title: "Breakdown: locations", area: "Breakdown", from: "breakdown-scenes", via: "locations-tab")
                    await ui.exploreButtons([("add-location", "Add a location")], area: "Breakdown", prefix: "breakdown-location", from: "breakdown-locations") }
            }
            await ui.step("categories") {
                if ui.exists(id: "categories-tab") { try await ui.capAct(id: "categories-tab"); await ui.idle(0.6); await ui.capture("breakdown-categories", title: "Breakdown: category styles", area: "Breakdown", from: "breakdown-scenes", via: "categories-tab") }
            }
        },
    ]
}

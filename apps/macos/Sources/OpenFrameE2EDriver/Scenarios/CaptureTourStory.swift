// Capture tour section: Story (Story page, Beat Board, Outline, Characters, Timeline). Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureStory: [Scenario] = [
        Scenario("zz-capture/03-story", timeout: 500) { ui, _, ctx in
            try await ui.capSeed(ctx, order: 3000)
            try await ui.capNav("Story")
            try await ui.wait(id: "story-tabs", timeout: 20)
            await ui.capture("story-board-empty", title: "Beat Board, nothing on it yet", area: "Story")
            await ui.capMenu(id: "story-add-menu", name: "story-add-menu", title: "Beat Board: Add menu", area: "Story", from: "story-board-empty")
            await ui.step("add act") {
                try await ui.capAct(label: "Add Act", role: "AXButton")
                await ui.capture("story-add-act-inline", title: "Adding an act on the board", area: "Story", from: "story-board-empty", via: "story-empty")
                try await ui.fill("Act title", "Act One", submit: true)
                await ui.idle(0.6)
            }
            await ui.step("add sequence") {
                try await ui.capAct(label: "Add Sequence", role: "AXButton")
                await ui.capture("story-add-sequence-sheet", title: "Add a sequence", area: "Story", from: "story-board-empty", via: "story-add-menu")
                try await ui.fill("Sequence name", "The last bus", submit: true)
                await ui.idle(0.5)
            }
            let cards = ["Meera misses the last bus", "The driver will not wait", "A lost bag on the back seat", "The ticket inspector boards", "Dawn at the depot"]
            for (n, c) in cards.enumerated() {
                await ui.step("add card \(n)") {
                    try await ui.capMenuButton(id: "story-add-menu", item: "Scene Card")
                    if n == 0 { await ui.capture("story-add-card-sheet", title: "Add a scene card", area: "Story", from: "story-board-empty", via: "story-add-menu") }
                    try await ui.fill("Short description", c, submit: true)
                    await ui.idle(0.5)
                }
            }
            await ui.capture("story-board-filled", title: "Beat Board with an act, a sequence and scene cards", area: "Story", from: "story-add-card-sheet")
            await ui.step("card drawer") {
                guard let c = ui.capCard(cards[0]) else { throw DriverError("no card") }
                try await ui.tap(c, count: 2)
                if (try? await ui.wait(id: "story-inspector-close", timeout: 3)) == nil, let again = ui.capCard(cards[0]) { try await ui.capCustomAction(again, "Open") }
                try await ui.wait(id: "story-inspector-close", timeout: 6)
                await ui.capture("story-card-drawer", title: "Scene card drawer", area: "Story", from: "story-board-filled", via: "story-card")
                _ = await ui.tryPress(id: "story-inspector-close")
            }
            await ui.capMenu(id: "story-more", name: "story-more-menu", title: "Beat Board: More menu", area: "Story", from: "story-board-filled")
            await ui.step("card display") {
                try await ui.capMenuButton(id: "story-more", item: "Card Display Settings")
                await ui.capture("story-card-display-sheet", title: "Card Display Settings", area: "Story", from: "story-board-filled", via: "story-more")
                await ui.capRecover()
            }
            await ui.step("apply order") {
                try await ui.capMenuButton(id: "story-more", item: "Apply Board Order")
                await ui.capture("story-apply-order-sheet", title: "Apply Board Order to Screenplay", area: "Story", from: "story-board-filled", via: "story-more")
                await ui.capRecover()
            }
            await ui.visit(press: "story-build", name: "story-build-sheet", title: "Build Screenplay from the board", area: "Story", from: "story-board-filled", close: nil)
            await ui.visit(press: "story-parking-toggle", name: "story-parking-lot", title: "Beat Board with the Parking Lot open", area: "Story", from: "story-board-filled")
            _ = await ui.tryPress(id: "story-parking-toggle")
            await ui.step("outline") {
                try await ui.capTab("Outline")
                await ui.capture("story-outline", title: "Story outline", area: "Story", from: "story-board-filled", via: "story-tabs")
            }
            await ui.step("timeline") {
                try await ui.capTab("Timeline")
                await ui.capture("story-timeline", title: "Story timeline", area: "Story", from: "story-outline", via: "story-tabs")
            }
            await ui.step("characters") {
                try await ui.capTab("Characters")
                await ui.capture("story-characters-empty", title: "Characters, none yet", area: "Story", from: "story-timeline", via: "story-tabs")
                for (name, role) in [("Meera", "A night shift nurse"), ("Dev", "The bus driver")] {
                    try await ui.capAct(label: "Add Character")
                    if name == "Meera" { await ui.capture("story-add-character-sheet", title: "Add a character", area: "Story", from: "story-characters-empty", via: "story-add-character") }
                    try await ui.fill(nil, name, id: "story-new-character-name")
                    try await ui.fill(nil, role, id: "story-new-character-role")
                    try await ui.capAct(label: "Create Character")
                    await ui.idle(0.6)
                }
                await ui.capture("story-characters", title: "Characters", area: "Story", from: "story-add-character-sheet")
                await ui.visit(label: "Edit", name: "story-character-edit", title: "Edit a character", area: "Story", from: "story-characters", closeLabel: "Save")
                await ui.capMenu(id: "story-character-more", name: "story-character-menu", title: "Character: More menu", area: "Story", from: "story-characters")
            }
            await ui.step("episodes") {
                try await ui.capTab("Beat Board")
                if ui.exists(id: "story-add-episode") { await ui.visit(press: "story-add-episode", name: "story-add-episode", title: "Add an episode", area: "Story", from: "story-board-filled") }
            }
        },
    ]
}

// Capture tour section: Idea Vault. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureVault: [Scenario] = [
        tour("04-vault", order: 4000, importSample: false) { ui, _ in
            try await ui.capNav("Idea Vault")
            try await ui.wait(id: "vault-items", timeout: 15)
            await ui.capture("vault-empty", title: "Idea Vault, empty", area: "Idea Vault")
            await ui.exploreMenu("vault-add", area: "Idea Vault", prefix: "vault-add", from: "vault-empty")
            for (n, text) in ["A bus that never reaches its stop", "Rain on the depot roof, a single lamp", "What if the driver is the ticket inspector?"].enumerated() {
                await ui.step("quick capture \(n)") {
                    try await ui.capAct(id: "vault-quick-capture")
                    _ = try await ui.wait(id: "quick-capture-idea", timeout: 6)
                    if n == 0 { await ui.capture("vault-quick-capture-sheet", title: "Quick capture an idea", area: "Idea Vault", from: "vault-empty", via: "vault-quick-capture") }
                    try await ui.fill(nil, text, id: "quick-capture-idea")
                    try await ui.capAct(label: "Save", role: "AXButton")
                    await ui.idle(0.8)
                }
            }
            await ui.capture("vault-filled", title: "Idea Vault with ideas", area: "Idea Vault", from: "vault-quick-capture-sheet")
            await ui.eachTab(group: "vault-view-mode", area: "Idea Vault", prefix: "vault-view", from: "vault-filled")
            await ui.step("open card") {
                guard let c = ui.capQueryPrefix("vault-card-").first(where: { $0.frame.width > 0 }) else { throw DriverError("no vault card") }
                try await ui.tap(c, count: 2)
                _ = try await ui.wait(id: "vault-drawer", timeout: 6)
                await ui.capture("vault-item-drawer", title: "Idea Vault item drawer", area: "Idea Vault", from: "vault-filled", via: "vault-card")
                await ui.exploreButtons([("drawer-send-to-story", "Send to Story"), ("drawer-add-tag", "Add tag"), ("drawer-research-flag", "Research flag")], area: "Idea Vault", prefix: "vault-drawer", from: "vault-item-drawer")
                _ = await ui.tryPress(id: "drawer-close")
            }
            await ui.step("folder") {
                guard let f = ui.capQueryPrefix("vault-folder-").first(where: { $0.frame.width > 0 }) else { throw DriverError("no folder") }
                try await ui.tap(f, count: 2)
                await ui.idle(0.6)
                await ui.capture("vault-folder", title: "Inside a folder", area: "Idea Vault", from: "vault-filled", via: "vault-folder")
                _ = await ui.tryPress(id: "vault-back")
            }
            await ui.eachTab(group: "vault-scope", area: "Idea Vault", prefix: "vault-scope", from: "vault-filled")
            try? await ui.capTab("This Project")
            await ui.step("new folder") {
                try await ui.capAct(id: "vault-new-folder")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                await ui.capture("vault-new-folder-sheet", title: "New folder", area: "Idea Vault", from: "vault-empty", via: "vault-new-folder")
                try await ui.fill(nil, "Night scenes", id: "name-sheet-field", submit: true)
                await ui.idle(0.6)
            }
            await ui.step("new collection") {
                try await ui.capAct(id: "vault-new-collection")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                await ui.capture("vault-new-collection-sheet", title: "New collection", area: "Idea Vault", from: "vault-empty", via: "vault-new-collection")
                try await ui.fill(nil, "Look and feel", id: "name-sheet-field", submit: true)
                await ui.idle(0.6)
            }
            await ui.capture("vault-with-folder-and-collection", title: "Idea Vault with a folder and a collection", area: "Idea Vault", from: "vault-new-collection-sheet", via: "name-sheet-field")
            await ui.step("recently deleted") {
                try await ui.capAct(id: "vault-recently-deleted")
                await ui.idle(0.8)
                await ui.capture("vault-recently-deleted", title: "Idea Vault: Recently Deleted", area: "Idea Vault", from: "vault-filled", via: "vault-recently-deleted")
            }
        },
    ]
}

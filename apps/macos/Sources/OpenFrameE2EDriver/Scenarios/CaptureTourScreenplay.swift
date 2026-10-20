// Capture tour section: Screenplay. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureScreenplay: [Scenario] = [
        tour("02-screenplay", order: 2000) { ui, _ in
            await ui.capture("screenplay-editor", title: "Screenplay editor with the example script", area: "Screenplay")
            await ui.step("type") {
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("\n\nEXT. DEPOT - DAWN\n")
                await ui.idle(0.6)
                await ui.capture("screenplay-editor-typing", title: "Screenplay editor while typing", area: "Screenplay", from: "screenplay-editor", via: "screenplay-editor")
            }
            await ui.exploreMenu("screenplay-tools-menu", area: "Screenplay", prefix: "screenplay-tools", from: "screenplay-editor")
            await ui.exploreMenu("screenplay-interchange-menu", area: "Screenplay", prefix: "screenplay-interchange", from: "screenplay-editor")
            await ui.exploreMenu("layout-menu", area: "Screenplay", prefix: "screenplay-layout", from: "screenplay-editor")
            await ui.exploreMenu("draft-menu", area: "Screenplay", prefix: "screenplay-draft", from: "screenplay-editor")
            await ui.capMenu(id: "element-picker", name: "screenplay-element-picker", title: "Screenplay: element type menu", area: "Screenplay", from: "screenplay-editor")
            await ui.exploreButtons([("comments-button", "Comments"), ("compare-button", "Compare drafts"), ("replace-toggle", "Find and replace"),
                                     ("screenplay-print", "Print preview"), ("screenplay-export", "Export screenplay"), ("screenplay-import", "Import screenplay"),
                                     ("screenplay-title", "Title page"), ("scene-navigator", "Scene navigator")], area: "Screenplay", prefix: "screenplay", from: "screenplay-editor")
        },
    ]
}

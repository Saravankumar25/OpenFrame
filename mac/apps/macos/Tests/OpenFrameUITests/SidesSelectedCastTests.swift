// Sides, "Selected cast" (Windows 81e75d16, production/sides.test.tsx): scenes without the selected
// cast are shown as left out, the preview and the export use only the scenes that stay, an empty
// result explains itself, and All cast restores the user's own ticks.

import Foundation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

@Suite("Sides for selected cast", .serialized)
@MainActor
struct SidesSelectedCastTests {
    static let arjun = ScheduleCastRef(key: "char:arjun", character: "ARJUN", actor: "Ravi", initials: "A")
    static let meera = ScheduleCastRef(key: "char:meera", character: "MEERA", actor: "Anu", initials: "M")
    static let zed = ScheduleCastRef(key: "char:zed", character: "ZED", actor: nil, initials: "Z")

    /// A planned project whose first day holds two scenes: scene A with Arjun and Meera, scene B
    /// with Meera only (the cast is set on the schedule the screen reads, as the Windows fixture does).
    private func rig(extraCast: [ScheduleCastRef] = []) async throws -> (CSRig, CSSidesModel, a: String, b: String) {
        var r = try await CSRig.make()
        try await r.importSample()
        try await r.plan()
        var view = try await r.core.scheduleGet()
        let di = try #require(view.days.firstIndex { $0.id == r.dayId })
        var strips: [String] = []
        for (i, item) in view.days[di].items.enumerated() where item.strip != nil {
            view.days[di].items[i].strip?.cast = strips.isEmpty ? [Self.arjun, Self.meera] : [Self.meera]
            strips.append(item.strip!.id)
        }
        try #require(strips.count == 2, "day 1 has \(strips.count) scenes")
        view.days[di].summary.cast = [Self.arjun, Self.meera] + extraCast
        let s: CSSidesModel = r.hub.sides
        s.show(view)
        s.pickDay(r.dayId)
        return (r, s, strips[0], strips[1])
    }

    private func strip(_ s: CSSidesModel, _ id: String) throws -> ScheduleStripDto { try #require(s.strips.first { $0.id == id }) }

    @Test func marksScenesWithoutTheSelectedCastAsLeftOutAndPreviewsOnlyTheOthers() async throws {
        let (r, s, a, b) = try await rig()
        defer { r.close() }
        try #expect(s.ticked(try strip(s, a)) && s.ticked(try strip(s, b)))
        #expect(s.filterLine == nil, "no line while All cast is on")
        s.forMode = .selected
        // Each cast name shows its scene count on the day.
        #expect(s.castLabel(Self.arjun) == "Ravi (ARJUN) \u{00B7} 1 scene")
        #expect(s.castLabel(Self.meera) == "Anu (MEERA) \u{00B7} 2 scenes")
        s.setCast(Self.arjun.key, on: true)
        // Arjun is in scene A only; scene B is left out, unticked, and the screen says why.
        try #expect(!s.included(try strip(s, b)) && !s.ticked(try strip(s, b)))
        try #expect(s.included(try strip(s, a)) && s.ticked(try strip(s, a)))
        #expect(s.filterLine == "1 of 2 scenes have the selected cast. Only those are in the sides.")
        // What the preview, Save and Export send: only the scenes that stay.
        #expect(s.args.stripIds == [a])
        #expect(s.args.castKeys == [Self.arjun.key])
        #expect(s.ready)
        #expect(s.previewKey.contains(a) && !s.previewKey.contains(b))
        #expect(r.quiet)
    }

    @Test func explainsAnEmptyResultAndRestoresEverythingWithAllCast() async throws {
        let (r, s, a, b) = try await rig()
        defer { r.close() }
        // Untick the scene Arjun is in, then pick Arjun: nothing is left.
        s.setScene(a, on: false)
        s.forMode = .selected
        s.setCast(Self.arjun.key, on: true)
        #expect(s.effective.isEmpty && !s.ready)
        #expect(s.notReadyMessage == "All the scenes with the selected cast are unticked. Tick at least one scene to preview the sides.")
        await s.loadPreview()
        #expect(s.preview == nil && s.previewFailure == nil, "the core was asked with no scene")
        // Back to All cast: the scene the user left ticked (B) is still the only one.
        s.forMode = .all
        try #expect(s.ticked(try strip(s, b)))
        try #expect(!s.ticked(try strip(s, a)))
        #expect(s.args.stripIds == [b] && s.args.castKeys == nil)
        #expect(r.quiet)
    }

    @Test func saysSoWhenNoSceneHasTheSelectedCast() async throws {
        let (r, s, a, b) = try await rig(extraCast: [Self.zed])
        defer { r.close() }
        s.forMode = .selected
        #expect(s.castLabel(Self.zed) == "ZED \u{00B7} 0 scenes")
        s.setCast(Self.zed.key, on: true)
        try #expect(!s.included(try strip(s, a)) && !s.included(try strip(s, b)), "every scene is left out")
        #expect(s.sceneCount == 0)
        #expect(s.filterLine == "No scene on this day has the selected cast. Choose other cast, or switch to All cast.")
        #expect(s.notReadyMessage.contains("no sides to show"))
        #expect(!s.ready, "Save Sides stays disabled")
        // The user's ticks are untouched underneath.
        #expect(Set(s.chosen) == [a, b])
        #expect(r.quiet)
    }
}

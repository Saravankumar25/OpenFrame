// The relationship map keeps names and labels apart (owner change list item 3).

import CoreGraphics
import Testing
@testable import OpenFrameUI

struct RelationshipMapLayoutTests {
    typealias Rel = RelationshipMapLayout.Rel

    @Test func theOrderRoundTheCircleAvoidsCrossingLines() {
        // A ring a-b-c-d-a drawn in the order a, c, b, d crosses itself; the layout untangles it.
        let ids = ["a", "c", "b", "d"]
        let rels = [Rel(id: "1", from: "a", to: "b"), Rel(id: "2", from: "b", to: "c"),
                    Rel(id: "3", from: "c", to: "d"), Rel(id: "4", from: "d", to: "a")]
        let order = RelationshipMapLayout.fewestCrossings(ids, rels)
        #expect(order.first == "a")
        #expect(Set(order) == Set(ids))
        // Neighbours on the ring are next to each other on the circle.
        let i = Dictionary(uniqueKeysWithValues: order.enumerated().map { ($1, $0) })
        for r in rels {
            let d = abs(i[r.from]! - i[r.to]!)
            #expect(d == 1 || d == 3, "\(r.from)-\(r.to)")
        }
    }

    @Test func severalRelationshipsBetweenOnePairBendApart() {
        let rels = [Rel(id: "1", from: "a", to: "b"), Rel(id: "2", from: "a", to: "b"), Rel(id: "3", from: "b", to: "a")]
        let r = RelationshipMapLayout.layout(ids: ["a", "b"], rels: rels, size: CGSize(width: 720, height: 420))
        let controls = rels.map { r.edges[$0.id]!.control }
        #expect(Set(controls.map { "\(Int($0.x)),\(Int($0.y))" }).count == 3)
    }

    @Test func labelsKeepClearOfEachOtherWhereThereIsRoom() {
        let ids = (1...6).map { "c\($0)" }
        var rels: [Rel] = []
        for (i, a) in ids.enumerated() { for b in ids[(i + 1)...] { rels.append(Rel(id: "\(a)-\(b)", from: a, to: b)) } }
        let r = RelationshipMapLayout.layout(ids: ids, rels: Array(rels.prefix(8)), size: CGSize(width: 720, height: 460))
        let labels = r.edges.values.map(\.label)
        var clashes = 0
        for i in labels.indices { for j in labels.indices where j > i {
            if abs(labels[i].x - labels[j].x) <= 70 && abs(labels[i].y - labels[j].y) <= 18 { clashes += 1 }
        } }
        #expect(clashes == 0)
        // Every character is inside the drawing.
        for p in r.positions.values { #expect(p.x > 0 && p.x < 720 && p.y > 0 && p.y < 460) }
    }

    @Test func initialsAndHeight() {
        #expect(RelationshipMapLayout.initials("Anjali Thomas") == "AT")
        #expect(RelationshipMapLayout.initials("Appachan") == "A")
        #expect(RelationshipMapLayout.height(for: 2) == 380)
        #expect(RelationshipMapLayout.height(for: 40) == 560)
    }
}

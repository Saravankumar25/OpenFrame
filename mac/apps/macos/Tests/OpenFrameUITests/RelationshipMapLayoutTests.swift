// The relationship maps, ported from the Windows relationshipMap.test.ts (commit b525f1d9): the
// same input always gives the same picture, no character box or label overlaps another, and the
// individual map holds only the character's direct relationships.

import CoreGraphics
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

private typealias L = RelationshipMapLayout

private func people(_ n: Int) -> [L.Node] {
    (0..<n).map { i in L.Node(id: "c\(i)", name: "Person " + String(UnicodeScalar(65 + i % 26)!) + "\(i)") }
}

private func rel(_ id: String, _ from: String, _ to: String, _ label: String = "friend of") -> L.Rel {
    L.Rel(id: id, from: from, to: to, label: label)
}

private func hit(_ a: (x: Double, y: Double, w: Double, h: Double), _ b: (x: Double, y: Double, w: Double, h: Double)) -> Bool {
    abs(a.x - b.x) * 2 < a.w + b.w && abs(a.y - b.y) * 2 < a.h + b.h
}

private func expectClean(_ m: L.Result, sourceLocation: SourceLocation = #_sourceLocation) {
    let boxes = Array(m.boxes.values).map { (x: $0.x, y: $0.y, w: $0.w, h: $0.h) }
    let labels = m.edges.values.map { (x: $0.label.x, y: $0.label.y, w: $0.label.w, h: $0.label.h) }
    for i in boxes.indices {
        let b = boxes[i]
        #expect(b.x - b.w / 2 >= 0, sourceLocation: sourceLocation)
        #expect(b.x + b.w / 2 <= m.W, sourceLocation: sourceLocation)
        #expect(b.y - b.h / 2 >= 0, sourceLocation: sourceLocation)
        #expect(b.y + b.h / 2 <= m.H, sourceLocation: sourceLocation)
        for j in boxes.indices where j > i { #expect(!hit(boxes[i], boxes[j]), sourceLocation: sourceLocation) }
    }
    for i in labels.indices {
        for b in boxes { #expect(!hit(labels[i], b), sourceLocation: sourceLocation) }
        for j in labels.indices where j > i { #expect(!hit(labels[i], labels[j]), sourceLocation: sourceLocation) }
    }
}

@Suite("RelationshipMapLayoutTests individual map")
struct RelationshipIndividualMapTests {
    private let nodes = people(6)
    private let rels = [rel("a", "c0", "c1"), rel("b", "c2", "c0", "father of"), rel("c", "c1", "c3"), rel("d", "c4", "c5")]

    @Test func holdsOnlyTheCharacterAndTheirDirectRelationships() {
        let m = L.layout(nodes, rels, focus: "c0")
        #expect(m.boxes.keys.sorted() == ["c0", "c1", "c2"])
        #expect(m.edges.keys.sorted() == ["a", "b"])
    }

    @Test func putsTheCharacterInTheMiddle() {
        let m = L.layout(nodes, rels, focus: "c0")
        #expect(m.boxes["c0"]?.x == m.W / 2)
        #expect(m.boxes["c0"]?.y == m.H / 2)
    }

    @Test func givesTheSamePictureForTheSameInputInAnyInputOrder() {
        let a = L.layout(nodes, rels, focus: "c0")
        let b = L.layout(nodes.reversed(), rels.reversed(), focus: "c0")
        #expect(a.boxes == b.boxes)
        #expect(a.W == b.W)
    }

    @Test func keepsEveryBoxAndLabelApartForOneToThirtyRelationships() {
        for n in [1, 2, 3, 5, 8, 9, 14, 30] {
            let ns = people(n + 1)
            let rs = ns.dropFirst().enumerated().map { i, x in
                rel("r\(i)", i % 2 == 1 ? x.id : "c0", i % 2 == 1 ? "c0" : x.id, i % 3 != 0 ? "colleague of" : "mother of")
            }
            expectClean(L.layout(ns, rs, focus: "c0"))
        }
    }

    @Test func bendsASecondRelationshipBetweenTheSamePairApartFromTheFirst() {
        let m = L.layout(people(2), [rel("1", "c0", "c1"), rel("2", "c1", "c0", "rival of")], focus: "c0")
        #expect(m.edges["1"]?.path != m.edges["2"]?.path)
        expectClean(m)
    }

    @Test func hasNoEdgesForACharacterWithoutRelationships() {
        let m = L.layout(nodes, rels, focus: "c5-none")
        #expect(m.edges.isEmpty)
    }

    @Test func upToEightSitOnARingAndMoreInTwoColumns() {
        let eight = people(9)
        let ring = L.layout(eight, eight.dropFirst().enumerated().map { rel("r\($0)", "c0", $1.id) }, focus: "c0")
        #expect(Set(ring.boxes.values.filter { $0.x != ring.W / 2 || $0.y != ring.H / 2 }.map(\.x)).count > 2)
        let nine = people(10)
        let cols = L.layout(nine, nine.dropFirst().enumerated().map { rel("r\($0)", "c0", $1.id) }, focus: "c0")
        let xs = Set(cols.boxes.filter { $0.key != "c0" }.values.map(\.x))
        #expect(xs.count == 2)
    }
}

@Suite("RelationshipMapLayoutTests whole film map")
struct RelationshipWholeFilmMapTests {
    @Test func placesTheMostConnectedCharacterOnTheTopLayerOneLayerPerStep() {
        let ns = people(5)
        let rs = [rel("1", "c0", "c1"), rel("2", "c0", "c2"), rel("3", "c0", "c3"), rel("4", "c3", "c4")]
        let m = L.layout(ns, rs)
        func y(_ id: String) -> Double { m.boxes[id]!.y }
        #expect(y("c0") < y("c1"))
        #expect(y("c1") == y("c2"))
        #expect(y("c3") < y("c4"))
        expectClean(m)
    }

    @Test func leavesOutCharactersWithoutRelationships() {
        let m = L.layout(people(4), [rel("1", "c0", "c1")])
        #expect(m.boxes.keys.sorted() == ["c0", "c1"])
    }

    @Test func givesTheSamePictureForTheSameInputInAnyInputOrder() {
        let ns = people(7)
        let rs = [rel("1", "c0", "c1"), rel("2", "c1", "c2"), rel("3", "c0", "c3"), rel("4", "c4", "c5"), rel("5", "c3", "c1"), rel("6", "c5", "c6")]
        let a = L.layout(ns, rs)
        let b = L.layout(ns.reversed(), rs.reversed())
        #expect(a.boxes == b.boxes)
        #expect(a.edges.keys.sorted() == b.edges.keys.sorted())
    }

    @Test func keepsBoxesAndLabelsApartForAStarARingParallelAndSameLayerRelationships() {
        let ns = people(12)
        let star = ns[1..<9].enumerated().map { i, x in rel("s\(i)", "c0", x.id, i % 2 == 1 ? "friend of" : "colleague of") }
        let extra = [rel("p1", "c0", "c1", "rival of"), rel("p2", "c1", "c0", "mentor of"), rel("l1", "c1", "c2"),
                     rel("l2", "c1", "c3"), rel("l3", "c9", "c10"), rel("l4", "c10", "c11")]
        expectClean(L.layout(ns, star + extra))
    }

    @Test func showsEveryRelationshipAndShortensAVeryLongType() {
        let long = "is the second cousin once removed of"
        let m = L.layout(people(2), [rel("1", "c0", "c1", long)])
        #expect((m.edges["1"]?.label.text.count ?? 99) <= 24)
        #expect(m.edges.count == 1)
    }

    /// The arrow points at the character the relationship goes to, also when the line runs upwards.
    @Test func theArrowEndsAtTheCharacterTheRelationshipGoesTo() {
        let m = L.layout(people(3), [rel("1", "c0", "c1"), rel("2", "c0", "c2"), rel("3", "c2", "c0", "child of")])
        let up = try! #require(m.edges["3"])
        let target = try! #require(m.boxes["c0"])
        #expect(abs(Double(up.path.end.y) - (target.y + target.h / 2)) < 0.001)
    }
}

/// Windows Characters.test.tsx: the character's own map shows only their relationships, the whole
/// film map is reachable on its own, and each map has a written list for screen readers.
@Suite("RelationshipMapLayoutTests characters screen")
@MainActor
struct RelationshipMapScreenTests {
    private static func ch(_ id: String, _ name: String) -> StoryCharacterDto {
        StoryCharacterDto(id: id, name: name, archived: false, sceneCount: 1, rev: 1)
    }
    private static let chars = [ch("a", "Arjun"), ch("b", "Bala"), ch("c", "Chitra"), ch("d", "Dev")]
    private static func rel(_ id: String, _ f: StoryCharacterDto, _ t: StoryCharacterDto, _ type: String) -> StoryRelationshipDto {
        StoryRelationshipDto(id: id, fromCharacterId: f.id, fromName: f.name, toCharacterId: t.id, toName: t.name, relationshipType: type, rev: 1)
    }
    private static let rels = [rel("1", chars[0], chars[1], "friend of"), rel("2", chars[2], chars[0], "mother of"), rel("3", chars[1], chars[3], "rival of")]

    private func name(_ id: String) -> String { Self.chars.first { $0.id == id }!.name }

    @Test func opensOneCharactersOwnMapWithOnlyTheirDirectRelationships() {
        let c = RelationshipMapView.Content(chars: Self.chars, relationships: Self.rels, focus: "a")
        #expect(c.title == "ARJUN: relationships")
        // Arjun's own relationships: friend of Bala, mother of Arjun (Chitra). Dev is Bala's, not Arjun's.
        #expect(c.links.map(name).sorted() == ["Bala", "Chitra"])
        #expect(c.pictureLabel.contains("2 relationships"))
        #expect(c.list.count == 2)
        #expect(!c.list.contains { $0.toName == "Dev" || $0.fromName == "Dev" })
    }

    @Test func opensTheWholeFilmMapOnItsOwn() {
        let c = RelationshipMapView.Content(chars: Self.chars, relationships: Self.rels, focus: nil)
        #expect(c.title == "Character relationships")
        #expect(c.pictureLabel.contains("3 relationships"))
        #expect(c.links.count == 4)
    }

    @Test func movesBetweenACharactersMapAndTheWholeFilmAndBack() async throws {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        defer { app.shutdown() }
        let story = StoryModel(app: app)
        let m = CharactersModel(story: story)
        #expect(m.map == nil)
        m.map = RelationshipMapFocus(id: "a")
        m.map = RelationshipMapFocus(id: nil)
        #expect(RelationshipMapView.Content(chars: Self.chars, relationships: Self.rels, focus: m.map?.id).title == "Character relationships")
        // Choosing Dev's name opens Dev's map.
        m.map = RelationshipMapFocus(id: "d")
        #expect(RelationshipMapView.Content(chars: Self.chars, relationships: Self.rels, focus: m.map?.id).title == "DEV: relationships")
        m.map = nil
        #expect(m.map == nil)
    }
}

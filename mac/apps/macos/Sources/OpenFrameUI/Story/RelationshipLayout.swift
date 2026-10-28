// Layout of the character relationship maps, a line-for-line port of the Windows
// relationshipLayout.ts (commit b525f1d9). A pure function from characters and relationships to
// positions, so the same input always gives the same picture and the rules are testable:
//  - individual map (a focus character): the character in the middle, only their direct
//    relationships and the characters on the other end, on a ring (up to 8) or in two columns (more);
//  - whole-film map: layers from the character with the most relationships outward, each layer
//    ordered to keep lines from crossing, one block per group of connected characters.
// Characters are boxes holding their name, so no line can cross a name, and every relationship
// type sits on its own line in a spot clear of other labels and of the boxes.

import CoreGraphics
import Foundation

enum RelationshipMapLayout {
    struct Node: Equatable { let id: String; let name: String }
    struct Rel: Equatable { let id: String; let from: String; let to: String; let label: String }

    /// A character's box; `x` and `y` are its centre.
    struct Box: Equatable {
        var x: Double
        var y: Double
        var w: Double
        var h: Double
        var text: String
    }

    /// A quadratic curve from `start` to `end` bent towards `control`; the arrow sits at `end`.
    struct Curve: Equatable {
        var start: CGPoint
        var control: CGPoint
        var end: CGPoint
    }

    struct Label: Equatable {
        var x: Double
        var y: Double
        var w: Double
        var h: Double
        var text: String
    }

    struct Edge: Equatable {
        var path: Curve
        var label: Label
    }

    struct Result: Equatable {
        var W: Double
        var H: Double
        var boxes: [String: Box]
        var edges: [String: Edge]
    }

    static let boxH: Double = 34
    static let labelH: Double = 16
    private static let margin: Double = 28
    private static let gapX: Double = 30
    private static let layerGap: Double = 170
    private static let componentGap: Double = 56
    static let maxName = 18
    static let maxLabel = 24
    static let ringMax = 8
    private static let tSteps: [Double] = [0.5, 0.44, 0.56, 0.38, 0.62, 0.32, 0.68, 0.26, 0.74, 0.2, 0.8]

    private struct Rect { var x, y, w, h: Double }

    static func shortText(_ s: String, _ max: Int) -> String {
        s.count > max ? String(s.prefix(max - 1)) + "\u{2026}" : s
    }

    private static func boxWidth(_ text: String) -> Double { 26 + 8 * Double(text.count) }
    private static func labelWidth(_ text: String) -> Double { (10 + 6.3 * Double(text.count)).rounded() }

    private static func makeBox(_ n: Node, _ x: Double, _ y: Double) -> Box {
        let text = shortText(n.name.uppercased(), maxName)
        return Box(x: x, y: y, w: boxWidth(text), h: boxH, text: text)
    }

    private static func rect(_ b: Box) -> Rect { Rect(x: b.x, y: b.y, w: b.w, h: b.h) }

    /// Where a ray from the centre of the box towards a point leaves the box.
    private static func exitPoint(_ b: Box, _ to: CGPoint) -> CGPoint {
        let dx = Double(to.x) - b.x
        let dy = Double(to.y) - b.y
        let s = min(dx == 0 ? .infinity : b.w / 2 / abs(dx), dy == 0 ? .infinity : b.h / 2 / abs(dy))
        return s.isFinite ? CGPoint(x: b.x + dx * s, y: b.y + dy * s) : CGPoint(x: b.x, y: b.y)
    }

    private static func overlaps(_ a: Rect, _ b: Rect, pad: Double = 3) -> Bool {
        abs(a.x - b.x) * 2 < a.w + b.w + pad * 2 && abs(a.y - b.y) * 2 < a.h + b.h + pad * 2
    }

    static func quadAt(_ a: CGPoint, _ c: CGPoint, _ b: CGPoint, _ t: Double) -> CGPoint {
        CGPoint(x: (1 - t) * (1 - t) * a.x + 2 * (1 - t) * t * c.x + t * t * b.x,
                y: (1 - t) * (1 - t) * a.y + 2 * (1 - t) * t * c.y + t * t * b.y)
    }

    /// The first spot that keeps a label clear of the other labels and of every box.
    private static func pickSpot(_ spots: [CGPoint], _ w: Double, _ placed: [Rect], _ boxes: [Rect]) -> CGPoint {
        for p in spots {
            let r = Rect(x: p.x, y: p.y, w: w, h: labelH)
            if !placed.contains(where: { overlaps(r, $0) }) && !boxes.contains(where: { overlaps(r, $0) }) { return p }
        }
        return spots[0]
    }

    private static func byName(_ a: Node, _ b: Node) -> Bool {
        let n = a.name.localizedCompare(b.name)
        if n != .orderedSame { return n == .orderedAscending }
        return a.id.localizedCompare(b.id) == .orderedAscending
    }

    private static func bend(_ k: Int) -> Double {
        k == 0 ? 0 : Double((k + 1) / 2) * 40 * (k % 2 == 1 ? 1 : -1)
    }

    /// Lines between two boxes that end on the box edges; parallel relationships bend to alternate sides.
    private static func freeEdge(_ a: Box, _ b: Box, _ k: Int, _ text: String, _ placed: inout [Rect], _ boxes: [Rect]) -> Edge {
        let dx = b.x - a.x
        let dy = b.y - a.y
        let len = hypot(dx, dy) == 0 ? 1 : hypot(dx, dy)
        let bd = bend(k)
        let c = CGPoint(x: (a.x + b.x) / 2 + (-dy / len) * bd, y: (a.y + b.y) / 2 + (dx / len) * bd)
        let p0 = exitPoint(a, c)
        let p1 = exitPoint(b, c)
        let w = labelWidth(text)
        let at = pickSpot(tSteps.map { quadAt(p0, c, p1, $0) }, w, placed, boxes)
        placed.append(Rect(x: at.x, y: at.y, w: w, h: labelH))
        return Edge(path: Curve(start: p0, control: c, end: p1), label: Label(x: at.x, y: at.y, w: w, h: labelH, text: text))
    }

    private static func layoutFocus(_ nodes: [Node], _ rels: [Rel], _ focus: String) -> Result {
        let own = rels.filter { $0.from != $0.to && ($0.from == focus || $0.to == focus) }
        let ends = Set(own.map { $0.from == focus ? $0.to : $0.from })
        let centre = nodes.first { $0.id == focus }
        let others = nodes.filter { ends.contains($0.id) }.sorted(by: byName)
        let n = others.count
        var boxes: [String: Box] = [:]
        let W: Double
        let H: Double
        if n <= ringMax {
            let rx: Double = 330
            let ry: Double = n <= 2 ? 110 : 160
            W = 2 * (rx + 90 + margin)
            H = 2 * (ry + boxH / 2 + margin)
            let start: Double = n < 3 ? 0 : -Double.pi / 2
            for (i, o) in others.enumerated() {
                let a = start + 2 * Double.pi * Double(i) / Double(n)
                boxes[o.id] = makeBox(o, W / 2 + rx * cos(a), H / 2 + ry * sin(a))
            }
        } else {
            let D: Double = 390
            let rowGap: Double = 62
            let left = (n + 1) / 2
            W = 2 * (D + 90 + margin)
            H = Double(left) * rowGap + 2 * margin
            for (i, o) in others.enumerated() {
                let onLeft = i < left
                let idx = onLeft ? i : i - left
                let cnt = onLeft ? left : n - left
                boxes[o.id] = makeBox(o, W / 2 + (onLeft ? -D : D), H / 2 + (Double(idx) - Double(cnt - 1) / 2) * rowGap)
            }
        }
        if let centre { boxes[centre.id] = makeBox(centre, W / 2, H / 2) }
        return Result(W: W, H: H, boxes: boxes, edges: routeFree(own, boxes))
    }

    private static func routeFree(_ rels: [Rel], _ boxes: [String: Box]) -> [String: Edge] {
        let all = boxes.values.map(rect)
        var placed: [Rect] = []
        var pairs: [String: Int] = [:]
        var edges: [String: Edge] = [:]
        for r in rels {
            guard let a = boxes[r.from], let b = boxes[r.to], r.from != r.to else { continue }
            let key = [r.from, r.to].sorted().joined(separator: "|")
            let k = pairs[key, default: 0]
            pairs[key] = k + 1
            edges[r.id] = freeEdge(a, b, k, shortText(r.label, maxLabel), &placed, all)
        }
        return edges
    }

    private static func layoutLayers(_ nodes: [Node], _ rels: [Rel]) -> Result {
        let ids = Set(nodes.map(\.id))
        let live = rels.filter { $0.from != $0.to && ids.contains($0.from) && ids.contains($0.to) }
        var adj: [String: Set<String>] = Dictionary(nodes.map { ($0.id, Set<String>()) }, uniquingKeysWith: { a, _ in a })
        var deg: [String: Int] = Dictionary(nodes.map { ($0.id, 0) }, uniquingKeysWith: { a, _ in a })
        for r in live {
            adj[r.from]!.insert(r.to)
            adj[r.to]!.insert(r.from)
            deg[r.from]! += 1
            deg[r.to]! += 1
        }
        let byId = Dictionary(nodes.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        // Most relationships first, then by name, so the order never depends on the order of the input.
        func rank(_ a: Node, _ b: Node) -> Bool {
            let da = deg[a.id]!, db = deg[b.id]!
            if da != db { return da > db }
            return byName(a, b)
        }
        let sorted = nodes.filter { deg[$0.id]! > 0 }.sorted(by: rank)

        var seen = Set<String>()
        var blocks: [[[String]]] = [] // block > layer > ids
        for root in sorted {
            if seen.contains(root.id) { continue }
            seen.insert(root.id)
            var layers: [[String]] = [[root.id]]
            var cur = layers[0]
            while !cur.isEmpty {
                var next: [String] = []
                for id in cur {
                    for nb in adj[id]!.map({ byId[$0]! }).sorted(by: rank) where !seen.contains(nb.id) {
                        seen.insert(nb.id)
                        next.append(nb.id)
                    }
                }
                if next.isEmpty { break }
                // Order the layer by where its relationships sit in the layer above, to keep lines from crossing.
                let above = Dictionary(cur.enumerated().map { ($1, $0) }, uniquingKeysWith: { a, _ in a })
                func bary(_ id: String) -> Double {
                    let xs = adj[id]!.compactMap { above[$0] }
                    return Double(xs.reduce(0, +)) / Double(max(xs.count, 1))
                }
                next.sort { a, b in
                    let ba = bary(a), bb = bary(b)
                    if ba != bb { return ba < bb }
                    return rank(byId[a]!, byId[b]!)
                }
                layers.append(next)
                cur = next
            }
            blocks.append(layers)
        }

        var boxes: [String: Box] = [:]
        var layerOf: [String: Int] = [:]
        func rowWidth(_ row: [String]) -> Double {
            row.reduce(0) { $0 + makeBox(byId[$1]!, 0, 0).w } + gapX * Double(row.count - 1)
        }
        let W = ([760] + blocks.flatMap { $0 }.map { rowWidth($0) + 2 * margin }).max()!
        var y = margin
        for (bi, layers) in blocks.enumerated() {
            let hasArc = live.contains { r in layers.contains { $0.contains(r.from) && $0.contains(r.to) } }
            y += hasArc ? 96 : 0
            for (li, row) in layers.enumerated() {
                var x = (W - rowWidth(row)) / 2
                for id in row {
                    var b = makeBox(byId[id]!, 0, y + boxH / 2)
                    b.x = x + b.w / 2
                    x += b.w + gapX
                    boxes[id] = b
                    layerOf[id] = bi * 1000 + li
                }
                y += li == layers.count - 1 ? boxH : layerGap
            }
            y += componentGap
        }
        let H = y - componentGap + margin

        let all = boxes.values.map(rect)
        var placed: [Rect] = []
        var pairs: [String: Int] = [:]
        var edges: [String: Edge] = [:]
        for r in live {
            guard let a = boxes[r.from], let b = boxes[r.to] else { continue }
            let key = [r.from, r.to].sorted().joined(separator: "|")
            let k = pairs[key, default: 0]
            pairs[key] = k + 1
            let text = shortText(r.label, maxLabel)
            let w = labelWidth(text)
            if layerOf[r.from] == layerOf[r.to] {
                // Same layer: an arc over the top of the row, high enough to clear the characters between.
                let p0 = CGPoint(x: a.x, y: a.y - a.h / 2)
                let p1 = CGPoint(x: b.x, y: b.y - b.h / 2)
                let spots = [34.0, 58, 82].map { CGPoint(x: (p0.x + p1.x) / 2, y: p0.y - $0) }
                let at = pickSpot(spots, w, placed, all)
                let lift = p0.y - at.y
                placed.append(Rect(x: at.x, y: at.y, w: w, h: labelH))
                edges[r.id] = Edge(path: Curve(start: p0, control: CGPoint(x: at.x, y: p0.y - 2 * lift), end: p1),
                                   label: Label(x: at.x, y: at.y, w: w, h: labelH, text: text))
            } else {
                // Next layer: from the bottom of the upper character to the top of the lower one, so a line never runs through a row.
                let (up, down) = a.y < b.y ? (a, b) : (b, a)
                let flip = up == b
                let p0 = CGPoint(x: up.x, y: up.y + up.h / 2)
                let p1 = CGPoint(x: down.x, y: down.y - down.h / 2)
                let dx = p1.x - p0.x
                let dy = p1.y - p0.y
                let len = hypot(dx, dy) == 0 ? 1 : hypot(dx, dy)
                let bd = bend(k)
                let c = CGPoint(x: (p0.x + p1.x) / 2 + (-dy / len) * bd, y: (p0.y + p1.y) / 2 + (dx / len) * bd)
                let at = pickSpot(tSteps.map { quadAt(p0, c, p1, $0) }, w, placed, all)
                placed.append(Rect(x: at.x, y: at.y, w: w, h: labelH))
                // The arrow points at the character the relationship goes to, so a line drawn upwards is reversed.
                edges[r.id] = Edge(path: flip ? Curve(start: p1, control: c, end: p0) : Curve(start: p0, control: c, end: p1),
                                   label: Label(x: at.x, y: at.y, w: w, h: labelH, text: text))
            }
        }
        return Result(W: W, H: H, boxes: boxes, edges: edges)
    }

    /// With a focus id: that character and their direct relationships only. Without: the whole film.
    static func layout(_ nodes: [Node], _ rels: [Rel], focus: String? = nil) -> Result {
        if let focus, !focus.isEmpty { return layoutFocus(nodes, rels, focus) }
        return layoutLayers(nodes, rels)
    }
}

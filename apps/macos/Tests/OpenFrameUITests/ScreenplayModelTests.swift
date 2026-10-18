// The screenplay editor's UI-side model: snapshot diff and sync, element rules, keyboard flow,
// suggestions and find. Ported from the Windows editor's editor.test.ts, flow.test.ts,
// commands.test.ts and suggest.test.ts. The core stays the source of truth.

import Foundation
import OpenFrameCore
import Testing
@testable import OpenFrameUI

// MARK: Fixtures

private typealias Snap = ScreenplaySnapshot
private typealias Model = ScreenplayModel

private func el(_ id: String, _ type: ElementType, _ text: String, custom: String? = nil) -> ScreenplayElementSnap {
    ScreenplayElementSnap(id: id, type: type, text: text, customElementId: custom)
}

private let baseSnap: Snap = [
    ScreenplaySceneSnap(id: "s1", heading: "INT. POLICE STATION \u{2014} NIGHT", elements: [
        el("e1", .action, "Arjun enters."),
        el("e2", .character, "MEERA"),
        el("e3", .dialogue, "You said you would\nnever come back."),
    ]),
    ScreenplaySceneSnap(id: "s2", heading: "EXT. STREET \u{2014} DAY", elements: [el("e4", .action, "Rain.")]),
    ScreenplaySceneSnap(id: "s3", heading: "INT. MORGUE \u{2014} MORNING", elements: [el("e5", .action, "Cold.")]),
]

private func rulesURL() -> URL {
    var dir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
    for _ in 0..<10 {
        let candidate = dir.appendingPathComponent("apps/desktop/src/workspaces/screenplay/editor/elementRules.json")
        if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
        dir.deleteLastPathComponent()
    }
    fatalError("elementRules.json not found above \(#filePath)")
}

private func loadRules() throws -> ElementRules { try ElementRules(contentsOf: rulesURL()) }

private func ok(_ a: Snap, _ ops: [ScreenplayEditOp], _ b: Snap) throws -> Bool {
    try ScreenplayModel.apply(ops, to: a) == b
}

private func opNames(_ ops: [ScreenplayEditOp]) -> [String] {
    ops.map { op in
        switch op {
        case .insertElement: return "insertElement"
        case .updateElement: return "updateElement"
        case .deleteElement: return "deleteElement"
        case .moveElement: return "moveElement"
        case .insertScene: return "insertScene"
        case .updateScene: return "updateScene"
        case .moveScene: return "moveScene"
        case .deleteScene: return "deleteScene"
        case .splitScene: return "splitScene"
        case .mergeScene: return "mergeScene"
        }
    }
}

// MARK: Blocks and snapshots

@Suite("Screenplay blocks")
struct ScreenplayModelTestsBlock {
    @Test func blocksRoundTripThroughSnapshot() throws {
        let rules = try loadRules()
        let blocks = Model.blocks(from: baseSnap, rules: rules)
        #expect(blocks.first?.kind == .sceneHeading)
        #expect(Model.snapshot(fromBlocks: blocks) == baseSnap)
    }

    @Test func emptySnapshotGivesOneEmptyHeading() throws {
        let blocks = Model.blocks(from: [], rules: try loadRules())
        #expect(blocks == [EditorBlock(id: nil, kind: .sceneHeading)])
    }

    @Test func textBeforeTheFirstHeadingAndBlocksWithoutIdsAreSkipped() {
        let blocks = [
            EditorBlock(id: "x", kind: .action, text: "orphan"),
            EditorBlock(id: "h1", kind: .sceneHeading, text: "INT. A - DAY"),
            EditorBlock(id: nil, kind: .action, text: "no id yet"),
            EditorBlock(id: "a1", kind: .action, text: "kept"),
            EditorBlock(id: nil, kind: .sceneHeading, text: "INT. B - DAY"),
            EditorBlock(id: "a2", kind: .action, text: "stays in scene one"),
        ]
        let snap = Model.snapshot(fromBlocks: blocks)
        #expect(snap.count == 1)
        #expect(snap[0].elements.map(\.id) == ["a1", "a2"])
    }

    @Test func assignIdsFillsMissingAndDuplicateIds() {
        var blocks = [
            EditorBlock(id: "a", kind: .sceneHeading, text: "H"),
            EditorBlock(id: nil, kind: .action, text: "one"),
            EditorBlock(id: "a", kind: .action, text: "two"),
            EditorBlock(id: "", kind: .action, text: "three"),
        ]
        var n = 0
        Model.assignIds(&blocks) { n += 1; return "new\(n)" }
        #expect(blocks.map(\.id) == ["a", "new1", "new2", "new3"])
        let snap = Model.snapshot(fromBlocks: blocks)
        #expect(snap[0].elements.count == 3)
    }

    @Test func pageBreaksHoldNoTextAndCustomKeepsItsDefinition() throws {
        let special: Snap = [ScreenplaySceneSnap(id: "s1", heading: "INT. SIGNAL BOX - NIGHT", elements: [
            el("e1", .custom, "Hello?", custom: "tel"),
            el("e2", .pageBreak, "ignored text"),
            el("e3", .action, "Silence."),
        ])]
        let blocks = Model.blocks(from: special, rules: try loadRules())
        #expect(blocks[1].customElementId == "tel")
        #expect(blocks[2].text == "")
        let back = Model.snapshot(fromBlocks: blocks)
        #expect(back[0].elements[1].text == "")
        #expect(back[0].elements[0].customElementId == "tel")
    }

    @Test func lineBreaksSurviveOnlyInMultilineElements() throws {
        let snap: Snap = [ScreenplaySceneSnap(id: "s1", heading: "H\nX", elements: [
            el("a", .action, "one\ntwo"), el("c", .character, "A\nB"),
        ])]
        let blocks = Model.blocks(from: snap, rules: try loadRules())
        #expect(blocks[0].text == "H X")
        #expect(blocks[1].text == "one\ntwo")
        #expect(blocks[2].text == "A B")
    }

    @Test func snapshotFromCoreScenes() {
        let dto = ScreenplaySceneDto(
            id: "s1", lineageId: "l", number: 1, label: "1", heading: "INT. A - DAY",
            elements: [ScreenplayElementDto(id: "e1", elementType: .custom, text: "t", dual: false, customElementId: "c1")],
            rev: 1)
        let snap = Model.snapshot(fromScenes: [dto])
        #expect(snap == [ScreenplaySceneSnap(id: "s1", heading: "INT. A - DAY", elements: [el("e1", .custom, "t", custom: "c1")])])
    }
}

// MARK: Diff

@Suite("Screenplay diff")
struct ScreenplayModelTestsDiff {
    private func check(_ next: Snap) throws -> [ScreenplayEditOp] {
        let ops = Model.diff(baseSnap, next)
        #expect(try ok(baseSnap, ops, next))
        return ops
    }

    @Test func nothingWhenUnchanged() {
        #expect(Model.diff(baseSnap, baseSnap).isEmpty)
    }

    @Test func typingUpdatesOneElement() throws {
        var next = baseSnap
        next[0].elements[0].text = "Arjun enters carrying a pistol."
        #expect(try check(next) == [.updateElement(id: "e1", elementType: nil, text: "Arjun enters carrying a pistol.", customElementId: nil)])
    }

    @Test func enterSplitsAnElementAndInsertsANewOne() throws {
        var next = baseSnap
        next[0].elements[0].text = "Arjun"
        next[0].elements.insert(el("n1", .action, " enters."), at: 1)
        #expect(opNames(try check(next)) == ["updateElement", "insertElement"])
    }

    @Test func changingABodyElementIntoAHeadingSplitsTheScene() throws {
        var next = baseSnap
        let tail = Array(next[0].elements[1...])
        next[0].elements = [next[0].elements[0]]
        next.insert(ScreenplaySceneSnap(id: "n2", heading: "INT. CORRIDOR \u{2014} NIGHT", elements: tail), at: 1)
        let ops = try check(next)
        #expect(ops.first == .insertScene(id: "n2", index: 1, heading: "INT. CORRIDOR \u{2014} NIGHT"))
        #expect(opNames(ops).filter { $0 == "moveElement" }.count == 2)
    }

    @Test func removingAHeadingMergesScenesAndDeletesTheEmptyScene() throws {
        var next = baseSnap
        next[0].elements += next[1].elements
        next.remove(at: 1)
        let ops = try check(next)
        #expect(ops.contains(.deleteScene(id: "s2")))
        #expect(ops.contains(.moveElement(id: "e4", sceneId: "s1", index: 3)))
    }

    @Test func deletingScenesKeepsTheirTextWithThem() throws {
        let next = baseSnap.filter { $0.id != "s3" }
        #expect(try check(next) == [.deleteScene(id: "s3")])
    }

    @Test func reordersScenesAndEditsHeadings() throws {
        var next = baseSnap
        let s3 = next.remove(at: 2)
        next.insert(s3, at: 0)
        next[1].heading = "INT. POLICE STATION \u{2014} DAY"
        let ops = try check(next)
        #expect(ops.contains(.moveScene(id: "s3", index: 0)))
        #expect(ops.contains(.updateScene(id: "s1", heading: "INT. POLICE STATION \u{2014} DAY")))
    }

    @Test func largeMixedEdit() throws {
        var next = baseSnap
        next[2].elements.append(el("n9", .transition, "CUT TO:"))
        next[0].elements[1].type = .action
        next[1].elements.remove(at: 0)
        next.append(ScreenplaySceneSnap(id: "n8", heading: "", elements: [el("n7", .action, "")]))
        _ = try check(next)
    }

    @Test func insertsAPageBreakAndChangesAnElementToCustom() throws {
        var next = baseSnap
        next[0].elements.insert(el("pb", .pageBreak, ""), at: 1)
        next[1].elements[0].type = .custom
        next[1].elements[0].customElementId = "tel"
        let ops = Model.diff(baseSnap, next)
        #expect(ops.contains(.insertElement(id: "pb", sceneId: "s1", index: 1, elementType: .pageBreak, text: "", customElementId: nil)))
        #expect(ops.contains(.updateElement(id: "e4", elementType: .custom, text: nil, customElementId: "tel")))
        #expect(try ok(baseSnap, ops, next))
    }

    @Test func customBackToActionDropsItsDefinition() throws {
        let special: Snap = [ScreenplaySceneSnap(id: "s1", heading: "H", elements: [el("e1", .custom, "Hello?", custom: "tel")])]
        var back = special
        back[0].elements[0] = el("e1", .action, "Hello?")
        let ops = Model.diff(special, back)
        #expect(ops == [.updateElement(id: "e1", elementType: .action, text: nil, customElementId: nil)])
        #expect(try ok(special, ops, back))
    }

    @Test func insertedCustomElementCarriesItsDefinition() {
        var next = baseSnap
        next[0].elements.append(el("c1", .custom, "x", custom: "tel"))
        let ops = Model.diff(baseSnap, next)
        #expect(ops == [.insertElement(id: "c1", sceneId: "s1", index: 3, elementType: .custom, text: "x", customElementId: "tel")])
    }

    @Test func opsEncodeAsTheCoreExpects() throws {
        let data = try JSONEncoder().encode(ScreenplayEditOp.updateElement(id: "e1", elementType: nil, text: "x", customElementId: nil))
        let json = try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any])
        #expect(json["op"] as? String == "updateElement")
        #expect(json["text"] as? String == "x")
        #expect(json["elementType"] == nil)
    }
}

// MARK: Random round trip

private struct SeededRandom: RandomNumberGenerator {
    var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}

private struct Mutator {
    var rng: SeededRandom
    var counter = 0
    static let types: [ElementType] = [.action, .character, .dialogue, .parenthetical, .transition, .shot, .note]

    mutating func fresh(_ p: String) -> String { counter += 1; return "\(p)\(counter)" }
    mutating func int(_ n: Int) -> Int { Int.random(in: 0..<max(n, 1), using: &rng) }

    mutating func randomSnap() -> Snap {
        var out: Snap = []
        for _ in 0..<(1 + int(5)) {
            var s = ScreenplaySceneSnap(id: fresh("s"), heading: "SCENE \(int(100))")
            for _ in 0..<int(6) {
                s.elements.append(el(fresh("e"), Self.types[int(Self.types.count)], "text \(int(1000))"))
            }
            out.append(s)
        }
        return out
    }

    mutating func mutate(_ snap: Snap) -> Snap {
        var s = snap
        for _ in 0..<(1 + int(6)) {
            switch int(11) {
            case 0: // edit text
                if let (si, ei) = pick(s) { s[si].elements[ei].text = "edited \(int(1000))" }
            case 1: // change type
                if let (si, ei) = pick(s) { s[si].elements[ei].type = Self.types[int(Self.types.count)]; s[si].elements[ei].customElementId = nil }
            case 2: // delete element
                if let (si, ei) = pick(s) { s[si].elements.remove(at: ei) }
            case 3: // insert element
                if !s.isEmpty {
                    let si = int(s.count)
                    let at = int(s[si].elements.count + 1)
                    s[si].elements.insert(el(fresh("n"), Self.types[int(Self.types.count)], "new \(int(100))"), at: at)
                }
            case 4: // move element, maybe across scenes
                if let (si, ei) = pick(s) {
                    let e = s[si].elements.remove(at: ei)
                    let ti = int(s.count)
                    s[ti].elements.insert(e, at: int(s[ti].elements.count + 1))
                }
            case 5: // insert scene
                s.insert(ScreenplaySceneSnap(id: fresh("ns"), heading: "NEW \(int(100))", elements: []), at: int(s.count + 1))
            case 6: // delete scene
                if s.count > 1 { s.remove(at: int(s.count)) }
            case 7: // move scene
                if s.count > 1 { let m = s.remove(at: int(s.count)); s.insert(m, at: int(s.count + 1)) }
            case 8: // edit heading
                if !s.isEmpty { s[int(s.count)].heading = "HEAD \(int(1000))" }
            case 9: // split a scene at an element
                if let (si, ei) = pick(s) {
                    let tail = Array(s[si].elements[ei...])
                    s[si].elements.removeSubrange(ei...)
                    s.insert(ScreenplaySceneSnap(id: fresh("sp"), heading: "SPLIT", elements: tail), at: si + 1)
                }
            default: // merge a scene into the one before it
                if s.count > 1 {
                    let i = 1 + int(s.count - 1)
                    let gone = s.remove(at: i)
                    s[i - 1].elements += gone.elements
                }
            }
        }
        return s
    }

    private mutating func pick(_ s: Snap) -> (Int, Int)? {
        let all = s.enumerated().flatMap { si, sc in sc.elements.indices.map { (si, $0) } }
        return all.isEmpty ? nil : all[int(all.count)]
    }
}

@Suite("Screenplay diff round trip")
struct ScreenplayModelTestsRoundTrip {
    @Test func applyOpsOfDiffGivesTheTarget() throws {
        var m = Mutator(rng: SeededRandom(seed: 20_261_017))
        for i in 0..<200 {
            let a = m.randomSnap()
            let b = m.mutate(a)
            let ops = Model.diff(a, b)
            let got = try Model.apply(ops, to: a)
            #expect(got == b, "case \(i) did not round trip: \(ops)")
            // A second diff from the result is empty.
            #expect(Model.diff(got, b).isEmpty, "case \(i) left a residual diff")
        }
    }

    @Test func chainedEditsStayInSync() throws {
        var m = Mutator(rng: SeededRandom(seed: 7))
        var baseline = m.randomSnap()
        var editor = baseline
        for i in 0..<50 {
            editor = m.mutate(editor)
            let ops = Model.diff(baseline, editor)
            baseline = try Model.apply(ops, to: baseline)
            #expect(baseline == editor, "step \(i)")
        }
    }
}

// MARK: Sync

@MainActor
private final class SyncHarness {
    var current: Snap
    var sent: [[ScreenplayEditOp]] = []
    var applied: [Snap] = []
    var statuses: [ScreenplaySyncStatus] = []
    var failure: ScreenplaySyncFailure = .revert
    var sync: ScreenplaySync!

    init(delayMs: Int = 20, retryMs: Int = 20, send: (@MainActor ([ScreenplayEditOp]) async throws -> Double)? = nil) {
        current = baseSnap
        sync = ScreenplaySync(
            initial: baseSnap, seq: 1,
            deps: .init(
                send: { [unowned self] ops in
                    self.sent.append(ops)
                    if let send { return try await send(ops) }
                    return 2
                },
                read: { [unowned self] in self.current },
                apply: { [unowned self] s in self.applied.append(s); self.current = s },
                onError: { [unowned self] _ in self.failure },
                onStatus: { [unowned self] in self.statuses.append($0) }),
            delayMs: delayMs, retryDelayMs: retryMs)
    }

    func edit(_ f: (inout Snap) -> Void) {
        f(&current)
        sync.markDirty()
    }
}

private struct Boom: Error {}

private func pause(_ ms: Int) async { try? await Task.sleep(for: .milliseconds(ms)) }

@Suite("Screenplay sync", .serialized)
@MainActor
struct ScreenplayModelTestsSync {
    @Test func batchesEditsAndIgnoresItsOwnEchoAndStaleReads() async {
        let t = SyncHarness()
        t.edit { $0[0].elements[0].text = "A" }
        t.edit { $0[0].elements[0].text = "AB" }
        await t.sync.flush()
        #expect(t.sent.count == 1)
        #expect(t.sent[0] == [.updateElement(id: "e1", elementType: nil, text: "AB", customElementId: nil)])
        // A stale read (seq 1) must not overwrite the editor.
        t.sync.receiveServer(seq: 1, snapshot: baseSnap)
        #expect(t.applied.isEmpty)
        // The echo of our own save changes nothing.
        var echo = baseSnap
        echo[0].elements[0].text = "AB"
        t.sync.receiveServer(seq: 2, snapshot: echo)
        #expect(t.applied.isEmpty)
        // A newer server state (an undo) replaces the editor.
        t.sync.receiveServer(seq: 2, snapshot: baseSnap)
        #expect(t.applied.count == 1)
    }

    @Test func debouncesEditsIntoOneSave() async {
        let t = SyncHarness(delayMs: 60)
        t.edit { $0[0].elements[0].text = "A" }
        await pause(30)
        t.edit { $0[0].elements[0].text = "AB" }
        await pause(30)
        // Each edit restarted the quiet period, so nothing has been sent yet.
        #expect(t.sent.isEmpty)
        #expect(t.sync.isDirty)
        await pause(250)
        #expect(t.sent.count == 1)
        #expect(!t.sync.isDirty)
        #expect(t.statuses == [.pending, .saving, .saved])
    }

    @Test func neverReplacesContentWhilePendingEditsExist() async {
        let t = SyncHarness()
        t.edit { $0[1].heading = "EXT. ROAD \u{2014} DAY" }
        t.sync.receiveServer(seq: 5, snapshot: baseSnap)
        #expect(t.applied.isEmpty)
        await t.sync.flush()
        #expect(t.sent.count == 1)
    }

    @Test func revertsToTheCoresStateWhenASaveIsRejected() async {
        let t = SyncHarness(send: { _ in throw Boom() })
        t.edit { $0[0].elements[0].text = "Changed" }
        await t.sync.flush()
        #expect(t.applied.count == 1)
        #expect(t.applied[0][0].elements[0].text == "Arjun enters.")
        #expect(!t.sync.isDirty)
    }

    @Test func retryKeepsTheLocalTextAndResendsLater() async {
        var attempts = 0
        let t = SyncHarness(send: { _ in
            attempts += 1
            if attempts == 1 { throw Boom() }
            return 2
        })
        t.failure = .retry
        t.edit { $0[0].elements[0].text = "Keep me" }
        await t.sync.flush()
        #expect(t.applied.isEmpty)
        #expect(t.sync.currentStatus == .error)
        await pause(250)
        #expect(t.sent.count == 2)
        #expect(t.sent[0] == t.sent[1])
        #expect(t.sync.currentStatus == .saved)
        #expect(t.current[0].elements[0].text == "Keep me")
    }

    @Test func aRevertUsesTheLatestServerDocumentWhenItIsNotOlderThanTheAck() async {
        let t = SyncHarness(send: { _ in throw Boom() })
        var server = baseSnap
        server[1].heading = "EXT. NEW \u{2014} DAY"
        t.edit { $0[0].elements[0].text = "Changed" }
        t.sync.receiveServer(seq: 3, snapshot: server)
        await t.sync.flush()
        #expect(t.applied == [server])
    }

    @Test func saveThenKeepsTheUndoOrder() async {
        // Create New + Enter: the typed name is saved, the character is created, and only then is
        // the Dialogue line saved, so Undo takes them back one at a time.
        var order: [String] = []
        var finish: CheckedContinuation<Void, Never>?
        let t = SyncHarness(send: { ops in
            order.append("save \(opNames(ops).joined(separator: ","))")
            return 2
        })
        t.edit { $0[0].elements[0].text = "ZED" }
        t.sync.saveThen {
            order.append("create character")
            await withCheckedContinuation { finish = $0 }
            order.append("character created")
        }
        // The next edit happens at once (the caret is already in Dialogue)...
        t.edit { $0[0].elements.append(el("d1", .dialogue, "")) }
        let later = Task { await t.sync.flush() }
        await pause(60)
        // ...but its save waits for the character.
        #expect(order == ["save updateElement", "create character"])
        #expect(t.sync.isDirty)
        finish?.resume()
        await later.value
        #expect(order == ["save updateElement", "create character", "character created", "save insertElement"])
        #expect(!t.sync.isDirty)
    }

    @Test func aFailedOutsideChangeNeverStopsLaterSaves() async {
        let t = SyncHarness()
        t.sync.saveThen { throw Boom() }
        t.edit { $0[0].elements[0].text = "Later" }
        await t.sync.flush()
        #expect(t.sent.count == 1)
        #expect(t.sent[0] == [.updateElement(id: "e1", elementType: nil, text: "Later", customElementId: nil)])
    }

    @Test func saveThenWhileASaveIsRunningDoesNotDeadlock() async {
        var release: CheckedContinuation<Void, Never>?
        var calls = 0
        let t = SyncHarness(send: { _ in
            calls += 1
            if calls == 1 { await withCheckedContinuation { release = $0 } }
            return 2 + Double(calls)
        })
        t.edit { $0[0].elements[0].text = "one" }
        let first = Task { await t.sync.flush() }
        await pause(40)
        t.edit { $0[0].elements[0].text = "two" }
        var created = false
        t.sync.saveThen { created = true }
        release?.resume()
        await first.value
        await t.sync.flush()
        #expect(created)
        #expect(t.sent.count == 2)
        #expect(!t.sync.isDirty)
    }

    @Test func disposeStopsFurtherSaves() async {
        let t = SyncHarness()
        t.edit { $0[0].elements[0].text = "x" }
        t.sync.dispose()
        await pause(100)
        await t.sync.flush()
        #expect(t.sent.isEmpty)
    }
}

// MARK: Rules

@Suite("Screenplay element rules")
struct ScreenplayModelTestsRules {
    @Test func followsTheEnterProgression() throws {
        let rules = try loadRules()
        #expect(rules.nextOnEnter(.sceneHeading) == .action)
        #expect(rules.nextOnEnter(.action) == .character)
        #expect(rules.nextOnEnter(.character) == .dialogue)
        #expect(rules.nextOnEnter(.dialogue) == .action)
        #expect(rules.nextOnEnter(.transition) == .sceneHeading)
        #expect(ElementRules.onEnterEmpty(.action) == .sceneHeading)
        #expect(ElementRules.onEnterEmpty(.character) == .action)
        #expect(ElementRules.onEnterEmpty(.sceneHeading) == nil)
        #expect(ElementRules.tabNext(.character) == .parenthetical)
        #expect(ElementRules.tabPrev(.dialogue) == .character)
    }

    @Test func pageBreaksAndCustomElementsAreLeftAlone() {
        for t in [ElementType.pageBreak, .custom] {
            #expect(ElementRules.tabNext(t) == nil)
            #expect(ElementRules.tabPrev(t) == nil)
        }
        #expect(ElementRules.onEnterEmpty(.pageBreak) == nil)
        #expect(ElementRules.onEnterEmpty(.custom) == .action)
    }

    @Test func tableLoadsWithMarginsSpacingAndLabels() throws {
        let rules = try loadRules()
        let h = try #require(rules.rule(for: .sceneHeading))
        #expect(h.label == "Scene Heading")
        #expect(h.shortcut == "Ctrl+1")
        #expect(h.leftIn == 1.5 && h.widthIn == 6.0)
        #expect(h.uppercase && h.keepWithNext && h.spaceBefore == 1)
        let c = try #require(rules.rule(for: .character))
        #expect(c.leftIn == 3.7 && c.widthIn == 3.8)
        #expect(rules.rule(for: "transition")?.align == "right")
        #expect(rules.rule(for: "centered")?.align == "center")
        #expect(rules.rule(for: .note)?.printed == false)
        #expect(rules.label(for: .custom) == "Custom element")
        #expect(rules.isMultiline(.custom))
        #expect(rules.isMultiline(.dialogue) && !rules.isMultiline(.character))
    }

    @Test func shortcutsFollowTheMenuOrder() throws {
        let rules = try loadRules()
        let keys = ElementRules.shortcutOrder.map { rules.rule(for: $0)?.shortcut }
        #expect(keys == (1...7).map { "Ctrl+\($0)" })
        #expect(rules.rule(for: .pageBreak)?.shortcut == "Ctrl+Enter")
    }

    @Test func everyStoredElementTypeHasARule() throws {
        let rules = try loadRules()
        for t in ElementType.allCases where t != .custom {
            #expect(rules.rule(for: t) != nil, "no rule for \(t.rawValue)")
        }
    }
}

// MARK: Keyboard flow

private let customTel = ScreenplayCustomElementDto(
    id: "tel", screenplayId: "sp", name: "Telephone", shortcut: "Ctrl+Alt+1", look: "dialogue", uppercase: true,
    italic: true, spaceBefore: 1, nextElement: "action", exportName: "TELEPHONE", usageCount: 1, rev: 1)
private let customRadio = ScreenplayCustomElementDto(
    id: "radio", screenplayId: "sp", name: "Radio", shortcut: nil, look: "centered", uppercase: false,
    italic: false, spaceBefore: 1, nextElement: "custom:tel", exportName: "RADIO", usageCount: 1, rev: 1)

private func keyboard(_ defs: [ScreenplayCustomElementDto] = [customTel, customRadio]) throws -> ScreenplayKeyboard {
    // A counter keeps generated ids readable and unique inside one test.
    let box = IdBox()
    return ScreenplayKeyboard(rules: try loadRules(), customDefs: defs, newId: { box.next() })
}

private final class IdBox: @unchecked Sendable {
    private let lock = NSLock()
    private var n = 0
    func next() -> String { lock.lock(); defer { lock.unlock() }; n += 1; return "gen\(n)" }
}

private enum Caret { case start, end, at(Int) }

/// A one-scene document whose last block is `type` with `text`, caret at `caret` in it.
private func stateWith(_ type: ElementType, _ text: String, _ caret: Caret = .end, custom: String? = nil) -> ScreenplayEditorState {
    var blocks = [
        EditorBlock(id: "s1", kind: .sceneHeading, text: "INT. DEPOT - NIGHT"),
        EditorBlock(id: "e1", kind: .action, text: "Rain."),
    ]
    if type == .sceneHeading {
        blocks.append(EditorBlock(id: "s2", kind: .sceneHeading, text: text))
    } else {
        blocks.append(EditorBlock(id: "x1", kind: type, text: text, customElementId: custom))
    }
    let len = text.utf16.count
    let off: Int
    switch caret {
    case .start: off = 0
    case .end: off = len
    case .at(let n): off = n
    }
    return ScreenplayEditorState(blocks: blocks, caret: EditorCaret(block: blocks.count - 1, offset: off))
}

private func run(_ r: EditorCommandResult) -> ScreenplayEditorState {
    #expect(r.handled)
    return r.state
}

private let builtIn: [ElementType] = ElementRules.elementTypes.filter { $0 != .pageBreak && $0 != .custom }
private let bodyTypes: [ElementType] = builtIn.filter { $0 != .sceneHeading }

struct TypePair: Sendable, CustomTestStringConvertible {
    var from: ElementType
    var to: ElementType
    var testDescription: String { "\(from.rawValue) to \(to.rawValue)" }
}

private let overridePairs: [TypePair] = bodyTypes.flatMap { f in builtIn.filter { $0 != f }.map { TypePair(from: f, to: $0) } }

@Suite("Screenplay keyboard flow")
struct ScreenplayModelTestsKeyboard {
    @Test(arguments: builtIn)
    func enterAtTheEndMovesToTheNextElementAndKeepsText(t: ElementType) throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: stateWith(t, "TEXT")))
        #expect(s.caretBlock.kind == kb.rules.nextOnEnter(t))
        #expect(s.caretBlock.text.isEmpty)
        let before = s.blocks[s.blocks.count - 2]
        #expect(before.kind == t)
        #expect(before.text == "TEXT")
    }

    @Test(arguments: bodyTypes)
    func enterOnAnEmptyElementConvertsIt(t: ElementType) throws {
        let kb = try keyboard()
        let s0 = stateWith(t, "")
        let s = run(kb.enter(in: s0))
        #expect(s.blocks.count == s0.blocks.count)
        #expect(s.caretBlock.kind == (ElementRules.onEnterEmpty(t) ?? t))
    }

    @Test func emptySceneHeadingEnterDoesNothing() throws {
        let kb = try keyboard()
        let s0 = stateWith(.sceneHeading, "")
        #expect(run(kb.enter(in: s0)).blocks == s0.blocks)
    }

    @Test func enterTwiceFromDialogueStartsANewScene() throws {
        let kb = try keyboard()
        var s = run(kb.enter(in: stateWith(.dialogue, "Go.")))
        #expect(s.caretBlock.kind == .action)
        s = run(kb.enter(in: s))
        #expect(s.caretBlock.kind == .sceneHeading)
    }

    @Test(arguments: bodyTypes)
    func enterInTheMiddleSplitsIntoTwoElements(t: ElementType) throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: stateWith(t, "ABCD", .at(2))))
        let prev = s.blocks[s.blocks.count - 2]
        let last = s.blocks[s.blocks.count - 1]
        #expect([prev.kind, last.kind] == [t, t])
        #expect([prev.text, last.text] == ["AB", "CD"])
        #expect(prev.id == "x1")
        #expect(last.id != "x1")
        #expect(s.caret == EditorCaret(block: s.blocks.count - 1, offset: 0))
    }

    @Test(arguments: bodyTypes)
    func enterAtTheStartAddsABlankAboveAndKeepsIdentity(t: ElementType) throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: stateWith(t, "ABCD", .start)))
        let last = s.blocks[s.blocks.count - 1]
        #expect(last.text == "ABCD")
        #expect(last.id == "x1")
        #expect(s.blocks[s.blocks.count - 2].kind == t)
        #expect(s.blocks[s.blocks.count - 2].text.isEmpty)
        #expect(s.caret.block == s.blocks.count - 1)
    }

    @Test func enterInTheMiddleOfAHeadingMovesTheRestIntoAction() throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: stateWith(.sceneHeading, "EXT. ROAD", .at(4))))
        #expect(s.blocks.last?.kind == .action)
        #expect(s.blocks.last?.text == " ROAD")
        #expect(s.blocks[s.blocks.count - 2].text == "EXT.")
    }

    @Test(arguments: builtIn)
    func shiftEnterIsALineBreakInMultilineElementsAndEnterElsewhere(t: ElementType) throws {
        let kb = try keyboard()
        let s0 = stateWith(t, "TEXT")
        let s = run(kb.lineBreak(in: s0))
        if kb.rules.isMultiline(t) {
            #expect(s.blocks.count == s0.blocks.count)
            #expect(s.caretBlock.text == "TEXT\n")
        } else {
            #expect(s.blocks.count == s0.blocks.count + 1)
        }
    }

    @Test(arguments: bodyTypes)
    func tabAndShiftTabFollowTheCycle(t: ElementType) throws {
        let kb = try keyboard()
        #expect(run(kb.tab(forward: true, in: stateWith(t, "X"))).caretBlock.kind == ElementRules.tabNext(t))
        #expect(run(kb.tab(forward: false, in: stateWith(t, "X"))).caretBlock.kind == ElementRules.tabPrev(t))
    }

    @Test func dialogueBlockCycles() throws {
        let kb = try keyboard()
        var s = stateWith(.action, "")
        var seen: [ElementType] = []
        for _ in 0..<4 {
            s = run(kb.tab(forward: true, in: s))
            seen.append(s.caretBlock.kind)
        }
        #expect(seen == [.character, .parenthetical, .dialogue, .parenthetical])
    }

    @Test func tabNeverLeavesThePageEvenWhenNothingChanges() throws {
        let kb = try keyboard()
        let r = kb.tab(forward: true, in: stateWith(.transition, "CUT TO:"))
        #expect(r.handled)
        #expect(r.state.caretBlock.kind == .action)
    }

    @Test(arguments: overridePairs)
    func manualOverrideSticksThroughTypingReloadAndEnter(p: TypePair) throws {
        let kb = try keyboard()
        let r = kb.setType(p.to, in: stateWith(p.from, "Words"))
        #expect(r.failure == nil)
        var s = run(r)
        #expect(s.caretBlock.kind == p.to)
        // Typing keeps the chosen element.
        s.blocks[s.caret.block].text += " more"
        s.caret.offset = s.caretBlock.text.utf16.count
        #expect(s.caretBlock.kind == p.to)
        // Save and reload (the sync snapshot) keeps it.
        let reloaded = ScreenplayModel.blocks(from: ScreenplayModel.snapshot(fromBlocks: s.blocks), rules: kb.rules)
        #expect(reloaded.last?.kind == p.to)
        // Enter afterwards follows the rules of the chosen element, not the old one.
        let after = run(kb.enter(in: s))
        #expect(after.caretBlock.kind == kb.rules.nextOnEnter(p.to))
    }

    @Test func overridingTheSuggestedNextElementKeepsCharacter() throws {
        let kb = try keyboard()
        var s = run(kb.enter(in: stateWith(.sceneHeading, "INT. BUS - NIGHT")))
        #expect(s.caretBlock.kind == .action)
        s = run(kb.setType(.character, in: s))
        s.blocks[s.caret.block].text = "MEERA"
        s.caret.offset = 5
        s = run(kb.enter(in: s))
        #expect(s.caretBlock.kind == .dialogue)
        #expect(Array(s.kinds.suffix(3)) == [.sceneHeading, .character, .dialogue])
    }

    @Test func enterGoesIntoAnEmptyElementOfTheNextKindInsteadOfStacking() throws {
        let kb = try keyboard()
        // A new screenplay is an empty heading followed by an empty action.
        var s = ScreenplayEditorState(
            blocks: [EditorBlock(id: "s1", kind: .sceneHeading, text: "INT. BUS - NIGHT"), EditorBlock(id: "a1", kind: .action)],
            caret: EditorCaret(block: 0, offset: 16))
        s = run(kb.enter(in: s))
        #expect(s.kinds == [.sceneHeading, .action])
        #expect(s.caretBlock.id == "a1")
        // The next element is not empty: Enter adds one.
        s = run(kb.enter(in: stateWith(.action, "Rain.")))
        #expect(Array(s.kinds.suffix(2)) == [.action, .character])
        // An empty element of another kind is left alone: a new one of the right kind comes first.
        let other = ScreenplayEditorState(
            blocks: [
                EditorBlock(id: "s1", kind: .sceneHeading, text: "INT. BUS - NIGHT"),
                EditorBlock(id: "a1", kind: .action, text: "Rain."),
                EditorBlock(id: "c1", kind: .dialogue),
            ],
            caret: EditorCaret(block: 1, offset: 5))
        #expect(run(kb.enter(in: other)).kinds == [.sceneHeading, .action, .character, .dialogue])
    }

    @Test func theFirstSceneHeadingCannotBecomeABodyElement() throws {
        let kb = try keyboard()
        let s = ScreenplayEditorState(
            blocks: [EditorBlock(id: "s1", kind: .sceneHeading, text: "INT. DEPOT - NIGHT")], caret: EditorCaret(block: 0, offset: 1))
        let r = kb.setType(.action, in: s)
        #expect(!r.handled)
        #expect(r.failure == ScreenplayKeyboard.firstSceneNeedsHeading)
    }

    @Test func unicodeTextSurvivesAnOverride() throws {
        let kb = try keyboard()
        var s = stateWith(.action, "\u{C2E}\u{C40}\u{C30}\u{C3E} \u{928}\u{92E}\u{938}\u{94D}\u{924}\u{947}")
        let text = s.caretBlock.text
        s = run(kb.setType(.dialogue, in: s))
        #expect(s.caretBlock.text == text)
        s = run(kb.setType(.character, in: s))
        #expect(s.caretBlock.text == text)
        let split = run(kb.enter(in: stateWith(.action, text, .at(4))))
        #expect(split.blocks[split.blocks.count - 2].text + split.caretBlock.text == text)
    }

    @Test func changingToAnElementWithoutLineBreaksFlattensThem() throws {
        let kb = try keyboard()
        let s = run(kb.setType(.character, in: stateWith(.dialogue, "one\ntwo")))
        #expect(s.caretBlock.text == "one two")
    }

    @Test func headingChangeChangesIdentity() throws {
        let kb = try keyboard()
        let s = run(kb.setType(.sceneHeading, in: stateWith(.action, "INT. NEW - DAY")))
        #expect(s.caretBlock.kind == .sceneHeading)
        #expect(s.caretBlock.id != "x1")
        let back = run(kb.setType(.action, in: stateWith(.sceneHeading, "INT. OLD - DAY")))
        #expect(back.caretBlock.id != "s2")
    }

    @Test func settingAnElementOverARangeStopsAtPageBreaks() throws {
        let kb = try keyboard()
        let s = ScreenplayEditorState(
            blocks: [
                EditorBlock(id: "s1", kind: .sceneHeading, text: "H"),
                EditorBlock(id: "c", kind: .custom, text: "Hello?", customElementId: "tel"),
                EditorBlock(id: "pb", kind: .pageBreak),
                EditorBlock(id: "a", kind: .action, text: "Silence."),
            ],
            caret: EditorCaret(block: 1, offset: 0), selectionEndBlock: 3)
        let r = kb.setType(.action, in: s)
        #expect(r.handled)
        #expect(r.state.kinds == [.sceneHeading, .action, .pageBreak, .action])
        #expect(r.state.blocks[1].customElementId == nil)
    }
}

// MARK: Page breaks and custom elements

private let special = ScreenplayEditorState(
    blocks: [
        EditorBlock(id: "s1", kind: .sceneHeading, text: "INT. SIGNAL BOX - NIGHT"),
        EditorBlock(id: "e1", kind: .custom, text: "Hello?", customElementId: "tel"),
        EditorBlock(id: "e2", kind: .pageBreak),
        EditorBlock(id: "e3", kind: .action, text: "Silence."),
    ],
    caret: EditorCaret(block: 1, offset: 6))

private func at(_ block: Int, _ caretOffset: Int?, _ s: ScreenplayEditorState = special) -> ScreenplayEditorState {
    var t = s
    t.caret = EditorCaret(block: block, offset: caretOffset ?? t.blocks[block].text.utf16.count)
    return t
}

@Suite("Screenplay page breaks and custom elements")
struct ScreenplayModelTestsCustom {
    @Test func tabLeavesThemAloneAndStaysInThePage() throws {
        let kb = try keyboard()
        let a = kb.tab(forward: true, in: at(1, nil))
        #expect(a.handled)
        #expect(a.state.kinds == special.kinds)
        #expect(a.state.blocks[1].customElementId == "tel")
        let b = kb.tab(forward: true, in: at(2, 0))
        #expect(b.handled && b.state.kinds == special.kinds)
        #expect(run(kb.tab(forward: true, in: at(3, nil))).blocks[3].kind == .character)
    }

    @Test func enterAtTheEndOfACustomElementCreatesItsOwnNextElement() throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: at(1, nil)))
        #expect(s.kinds == [.sceneHeading, .custom, .action, .pageBreak, .action])
    }

    @Test func nextElementCanBeAnotherCustomElement() throws {
        let kb = try keyboard()
        let snap = ScreenplayEditorState(
            blocks: [
                EditorBlock(id: "s1", kind: .sceneHeading, text: "H"),
                EditorBlock(id: "r1", kind: .custom, text: "Over.", customElementId: "radio"),
            ], caret: EditorCaret(block: 1, offset: 5))
        let s = run(kb.enter(in: snap))
        #expect(s.kinds == [.sceneHeading, .custom, .custom])
        #expect(s.blocks[2].customElementId == "tel")
        // A missing definition falls back to action.
        let gone = try keyboard([customRadio])
        #expect(run(gone.enter(in: snap)).blocks[2].kind == .action)
    }

    @Test func enterInTheMiddleSplitsAndBothHalvesKeepTheDefinition() throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: at(1, 3)))
        #expect(s.kinds == [.sceneHeading, .custom, .custom, .pageBreak, .action])
        #expect(s.blocks[1].customElementId == "tel" && s.blocks[2].customElementId == "tel")
        #expect(s.blocks[1].id == "e1")
        #expect(s.blocks[2].id != "e1")
    }

    @Test func enterOnAnEmptyCustomElementTurnsItIntoAction() throws {
        let kb = try keyboard()
        var s0 = special
        s0.blocks[1].text = ""
        let s = run(kb.enter(in: at(1, 0, s0)))
        #expect(s.blocks[1].kind == .action)
        #expect(s.blocks[1].customElementId == nil)
        #expect(s.blocks[1].id == "e1")
    }

    @Test func enterOnAPageBreakAddsAnActionAfterItAndKeepsTheBreak() throws {
        let kb = try keyboard()
        let s = run(kb.enter(in: at(2, 0)))
        #expect(s.kinds == [.sceneHeading, .custom, .pageBreak, .action, .action])
        #expect(s.caretBlock.kind == .action)
        let sb = run(kb.lineBreak(in: at(2, 0)))
        #expect(sb.kinds == [.sceneHeading, .custom, .pageBreak, .action, .action])
    }

    @Test func shiftEnterIsALineBreakInsideACustomElement() throws {
        let kb = try keyboard()
        let s = run(kb.lineBreak(in: at(1, nil)))
        #expect(s.kinds == special.kinds)
        #expect(s.blocks[1].text == "Hello?\n")
    }

    @Test func elementShortcutsNeverTurnAPageBreakIntoText() throws {
        let kb = try keyboard()
        let r = kb.setType(.action, in: at(2, 0))
        #expect(!r.handled)
        #expect(r.failure == ScreenplayKeyboard.pageBreakHasNoText)
    }

    @Test func setTypeRefusesTheSpecialTypes() throws {
        let kb = try keyboard()
        #expect(!kb.setType(.pageBreak, in: at(3, nil)).handled)
        #expect(!kb.setType(.custom, in: at(3, nil)).handled)
    }

    @Test func customShortcutChangesAnActionAndRefusesBreaksAndUnknownDefinitions() throws {
        let kb = try keyboard()
        let r = kb.setCustom("tel", in: at(3, nil))
        #expect(r.handled)
        #expect(r.state.blocks[3].kind == .custom)
        #expect(r.state.blocks[3].id == "e3")
        #expect(r.state.blocks[3].customElementId == "tel")
        #expect(!kb.setCustom("tel", in: at(2, 0)).handled)
        #expect(!kb.setCustom("missing", in: at(3, nil)).handled)
        #expect(kb.customDef(forShortcut: "Ctrl+Alt+1")?.id == "tel")
    }

    @Test func theFirstHeadingCannotBecomeCustom() throws {
        let kb = try keyboard()
        let r = kb.setCustom("tel", in: at(0, 0))
        #expect(!r.handled)
        #expect(r.failure == ScreenplayKeyboard.firstSceneNeedsHeading)
    }

    @Test func changingACustomElementToABuiltInDropsTheDefinition() throws {
        let kb = try keyboard()
        let s = run(kb.setType(.character, in: at(1, nil)))
        let snap = ScreenplayModel.snapshot(fromBlocks: s.blocks)
        #expect(snap[0].elements[0].type == .character)
        #expect(snap[0].elements[0].customElementId == nil)
    }

    @Test(arguments: [ElementType.action, .character, .dialogue, .parenthetical, .transition, .shot])
    func customStickThroughReloadAndEnter(from: ElementType) throws {
        let kb = try keyboard()
        var s = run(kb.setCustom("tel", in: stateWith(from, "Words")))
        s.blocks[s.caret.block].text += " more"
        s.caret.offset = s.caretBlock.text.utf16.count
        let reloaded = ScreenplayModel.blocks(from: ScreenplayModel.snapshot(fromBlocks: s.blocks), rules: kb.rules)
        #expect(reloaded.last?.kind == .custom)
        #expect(reloaded.last?.customElementId == "tel")
        #expect(run(kb.enter(in: s)).caretBlock.kind == .action)
    }

    @Test func ctrlEnterInsertsAPageBreakAndContinuesWithTheSameKind() throws {
        let kb = try keyboard()
        let s = run(kb.insertPageBreak(in: at(1, nil)))
        #expect(s.kinds == [.sceneHeading, .custom, .pageBreak, .custom, .pageBreak, .action])
        #expect(s.blocks[3].customElementId == "tel")
        #expect(s.caretBlock.kind == .custom)
        let h = run(kb.insertPageBreak(in: at(0, nil)))
        #expect(Array(h.kinds.prefix(3)) == [.sceneHeading, .pageBreak, .action])
    }

    @Test func pageBreakSurvivesSaveAndReload() throws {
        let kb = try keyboard()
        let s = run(kb.insertPageBreak(in: at(3, nil)))
        var blocks = s.blocks
        ScreenplayModel.assignIds(&blocks)
        let snap = ScreenplayModel.snapshot(fromBlocks: blocks)
        #expect(snap[0].elements.filter { $0.type == .pageBreak }.count == 2)
    }
}

// MARK: Suggestions

private let suggestScript: [EditorBlock] = [
    EditorBlock(id: "s1", kind: .sceneHeading, text: "INT. BUS DEPOT - NIGHT"),
    EditorBlock(id: "e1", kind: .character, text: "MEERA"),
    EditorBlock(id: "e2", kind: .parenthetical, text: "(whispering)"),
    EditorBlock(id: "e3", kind: .dialogue, text: "Is it late?"),
    EditorBlock(id: "e4", kind: .character, text: "RAVI (V.O.)"),
    EditorBlock(id: "e5", kind: .character, text: "Meera"),
    EditorBlock(id: "s2", kind: .sceneHeading, text: "EXT. ROAD - MOMENTS LATER"),
]

private let suggestProject = WritingSuggestionSources(
    characters: [
        SuggestionValue(name: "ANJALI", source: .story),
        SuggestionValue(name: "MEERA", source: .story),
        SuggestionValue(name: "CONDUCTOR", source: .cast),
    ],
    locations: [SuggestionValue(name: "OLD RAILWAY STATION", source: .production)])

private func labels(_ items: [SuggestItem]) -> [String] { items.map(\.label) }
private typealias Suggest = ScreenplaySuggest

@Suite("Screenplay suggestions")
struct ScreenplayModelTestsSuggest {
    @Test func knowsThePartOfASceneHeading() {
        #expect(Suggest.contextAt(.sceneHeading, "", 0) == SuggestContext(kind: .prefix, from: 0, to: 0, query: ""))
        #expect(Suggest.contextAt(.sceneHeading, "IN", 2)?.kind == .prefix)
        #expect(Suggest.contextAt(.sceneHeading, "IN", 2)?.query == "IN")
        #expect(Suggest.contextAt(.sceneHeading, "INT. BU", 7) == SuggestContext(kind: .location, from: 5, to: 7, query: "BU"))
        #expect(Suggest.contextAt(.sceneHeading, "INT./EXT. CAR - NI", 18) == SuggestContext(kind: .time, from: 16, to: 18, query: "NI"))
        let c = Suggest.contextAt(.sceneHeading, "I/E CAR \u{2013} D", 11)
        #expect(c?.kind == .time)
        #expect(c?.query == "D")
    }

    @Test func offersNothingUnlessTheCaretIsAtTheEndAndNeverInActionOrDialogue() {
        #expect(Suggest.contextAt(.sceneHeading, "INT. BUS", 3) == nil)
        #expect(Suggest.contextAt(.action, "Meera", 5) == nil)
        #expect(Suggest.contextAt(.dialogue, "Hi", 2) == nil)
        #expect(Suggest.contextAt(.character, "ME\nRA", 5) == nil)
    }

    @Test func characterExtensionParentheticalAndTransitionContexts() {
        #expect(Suggest.contextAt(.character, "ME", 2) == SuggestContext(kind: .character, from: 0, to: 2, query: "ME"))
        #expect(Suggest.contextAt(.character, "MEERA (V", 8) == SuggestContext(kind: .extension, from: 7, to: 8, query: "V"))
        #expect(Suggest.contextAt(.character, "MEERA (V.O.)", 12) == nil)
        #expect(Suggest.contextAt(.parenthetical, "(wh", 3) == SuggestContext(kind: .parenthetical, from: 1, to: 3, query: "wh"))
        #expect(Suggest.contextAt(.transition, "CU", 2)?.kind == .transition)
    }

    @Test func readsCueNamesAndHeadingPartsLikeTheCore() {
        #expect(Suggest.cueName("ravi (V.O.) (CONT'D)") == "RAVI")
        #expect(Suggest.cueName("  meera   khan ^") == "MEERA KHAN")
        #expect(Suggest.headingParts("int. old  depot - night") == HeadingParts(prefix: "INT.", location: "OLD DEPOT", time: "NIGHT"))
        #expect(Suggest.headingParts("EXT. \u{C30}\u{C48}\u{C32}\u{C4D}\u{C35}\u{C47} \u{C38}\u{C4D}\u{C1F}\u{C47}\u{C37}\u{C28}\u{C4D} - DAY").location
            == "\u{C30}\u{C48}\u{C32}\u{C4D}\u{C35}\u{C47} \u{C38}\u{C4D}\u{C1F}\u{C47}\u{C37}\u{C28}\u{C4D}")
        #expect(Suggest.headingParts("KITCHEN") == HeadingParts(prefix: "", location: "KITCHEN", time: ""))
    }

    @Test func collectsScriptValuesMostUsedFirstWithoutDuplicates() {
        let v = Suggest.scriptValues(suggestScript)
        #expect(v.characters == ["MEERA", "RAVI"])
        #expect(v.locations == ["BUS DEPOT", "ROAD"])
        #expect(v.times == ["NIGHT", "MOMENTS LATER"])
        #expect(v.parentheticals == ["whispering"])
        #expect(v.prefixes == ["INT.", "EXT."])
    }

    @Test func leavingOutTheElementBeingTyped() {
        // Block 5 is the second "Meera"; leaving it out leaves one use of MEERA.
        let without = Suggest.scriptValues(suggestScript, without: 5)
        #expect(without.characters == ["MEERA", "RAVI"])
        // Block 4 is the only RAVI.
        #expect(Suggest.scriptValues(suggestScript, without: 4).characters == ["MEERA"])
        #expect(Suggest.scriptValues(suggestScript, without: 99) == Suggest.scriptValues(suggestScript))
    }

    @Test func prefixesAndTimesIncludeTheStandardTerms() {
        let v = Suggest.scriptValues(suggestScript)
        #expect(labels(Suggest.suggestions(.init(kind: .prefix, from: 0, to: 0, query: ""), script: v, project: nil, canCreate: true))
            == ScreenplaySuggest.headingPrefixes)
        let times = labels(Suggest.suggestions(.init(kind: .time, from: 0, to: 0, query: ""), script: v, project: nil, canCreate: true))
        for t in ["DAY", "NIGHT", "LATER", "CONTINUOUS", "SAME", "MOMENTS LATER"] { #expect(times.contains(t)) }
    }

    @Test func typingFiltersWordStartsCountAndTheExactValueIsNotOfferedAgain() {
        let v = Suggest.scriptValues(suggestScript)
        let ctx = SuggestContext(kind: .time, from: 0, to: 2, query: "la")
        #expect(labels(Suggest.suggestions(ctx, script: v, project: nil, canCreate: true)) == ["LATER", "MOMENTS LATER"])
        let exact = SuggestContext(kind: .time, from: 0, to: 5, query: "night")
        #expect(labels(Suggest.suggestions(exact, script: v, project: nil, canCreate: true)).isEmpty)
    }

    @Test func prefixMatchesRankAheadOfWordStartMatches() {
        let v = Suggest.scriptValues(suggestScript)
        let ctx = SuggestContext(kind: .time, from: 0, to: 1, query: "m")
        // MORNING and MOMENTS LATER start with m; no word-start only matches remain, order is by list position.
        #expect(labels(Suggest.suggestions(ctx, script: v, project: nil, canCreate: true)) == ["MORNING", "MOMENTS LATER"])
        let l = SuggestContext(kind: .time, from: 0, to: 1, query: "l")
        // LATER starts with l (rank 0); MOMENTS LATER only has a word starting with l (rank 1).
        #expect(labels(Suggest.suggestions(l, script: v, project: nil, canCreate: true)) == ["LATER", "MOMENTS LATER"])
    }

    @Test func charactersComeFromTheScriptTheStoryBoardAndCast() {
        let v = Suggest.scriptValues(suggestScript)
        let items = Suggest.suggestions(.init(kind: .character, from: 0, to: 0, query: ""), script: v, project: suggestProject, canCreate: true)
        #expect(items.map { [$0.label, $0.source.rawValue] } == [
            ["MEERA", "Script"], ["RAVI", "Script"], ["ANJALI", "Story Board"], ["CONDUCTOR", "Cast"],
        ])
    }

    @Test func locationsComeFromTheScriptAndTheProject() {
        let v = Suggest.scriptValues(suggestScript)
        let items = Suggest.suggestions(.init(kind: .location, from: 5, to: 5, query: ""), script: v, project: suggestProject, canCreate: true)
        #expect(items.map(\.label) == ["BUS DEPOT", "ROAD", "OLD RAILWAY STATION"])
        #expect(items.last?.source == .locations)
        #expect(items.first?.insert == "BUS DEPOT - ")
    }

    @Test func offersCreateNewForNewNamesNeverForKnownOnes() {
        let v = Suggest.scriptValues(suggestScript)
        let fresh = Suggest.suggestions(.init(kind: .character, from: 0, to: 3, query: "Zar"), script: v, project: suggestProject, canCreate: true)
        #expect(fresh.last == SuggestItem(label: "Create New: ZAR", insert: "Zar", source: .storyBoard, create: .character))
        let known = Suggest.suggestions(.init(kind: .character, from: 0, to: 5, query: "meera"), script: v, project: suggestProject, canCreate: true)
        #expect(!known.contains { $0.create != nil })
        let loc = Suggest.suggestions(.init(kind: .location, from: 5, to: 9, query: "ROOF"), script: v, project: suggestProject, canCreate: true)
        #expect(loc.last?.create == .location)
        let locked = Suggest.suggestions(.init(kind: .location, from: 5, to: 9, query: "ROOF"), script: v, project: suggestProject, canCreate: false)
        #expect(!locked.contains { $0.create != nil })
    }

    @Test func theListHoldsAtMostEightItemsBeforeCreateNew() {
        var blocks = [EditorBlock(id: "h", kind: .sceneHeading, text: "INT. A - DAY")]
        for i in 0..<20 { blocks.append(EditorBlock(id: "c\(i)", kind: .character, text: "NAME\(i)")) }
        let v = Suggest.scriptValues(blocks)
        let items = Suggest.suggestions(.init(kind: .character, from: 0, to: 0, query: ""), script: v, project: nil, canCreate: true)
        #expect(items.count == 8)
        let typed = Suggest.suggestions(.init(kind: .character, from: 0, to: 2, query: "na"), script: v, project: nil, canCreate: true)
        #expect(typed.count == 9)
        #expect(typed.last?.create == .character)
    }

    @Test func acceptingReplacesOnlyTheTypedPart() {
        let v = Suggest.scriptValues(suggestScript)
        let ctx = Suggest.contextAt(.sceneHeading, "INT. BU", 7)!
        let item = Suggest.suggestions(ctx, script: v, project: suggestProject, canCreate: true)[0]
        #expect(item.label == "BUS DEPOT")
        #expect(Suggest.apply(item, to: "INT. BU", in: ctx) == "INT. BUS DEPOT - ")
        let p = Suggest.contextAt(.parenthetical, "wh", 2)!
        let pItem = Suggest.suggestions(p, script: v, project: nil, canCreate: true)[0]
        #expect(Suggest.apply(pItem, to: "wh", in: p) == "(whispering)")
        let create = Suggest.suggestions(.init(kind: .character, from: 0, to: 3, query: "Zar"), script: v, project: suggestProject, canCreate: true).last!
        #expect(Suggest.apply(create, to: "Zar", in: .init(kind: .character, from: 0, to: 3, query: "Zar")) == "Zar")
        let ext = Suggest.contextAt(.character, "MEERA (V", 8)!
        let eItem = Suggest.suggestions(ext, script: v, project: nil, canCreate: true)[0]
        #expect(Suggest.apply(eItem, to: "MEERA (V", in: ext) == "MEERA (V.O.)")
    }

    @Test func aHeadingIsBuiltPrefixLocationTimeInThreeAccepts() {
        let v = Suggest.scriptValues(suggestScript)
        var text = ""
        for _ in 0..<3 {
            guard let ctx = Suggest.contextAt(.sceneHeading, text, text.utf16.count) else { Issue.record("no context at \(text)"); return }
            let item = Suggest.suggestions(ctx, script: v, project: suggestProject, canCreate: true)[0]
            text = Suggest.apply(item, to: text, in: ctx)
        }
        #expect(text == "INT. BUS DEPOT - DAY")
    }

    @Test func teluguNamesAreOfferedByTheirFirstLetters() {
        let name = "\u{C2E}\u{C40}\u{C30}\u{C3E}"
        let blocks = [EditorBlock(id: "h", kind: .sceneHeading, text: "INT. A - DAY"), EditorBlock(id: "c", kind: .character, text: name)]
        let v = Suggest.scriptValues(blocks)
        let first = String(name.prefix(1))
        let items = Suggest.suggestions(.init(kind: .character, from: 0, to: first.utf16.count, query: first), script: v, project: nil, canCreate: true)
        #expect(items.first?.label == name)
    }
}

// MARK: Find

@Suite("Screenplay find")
struct ScreenplayModelTestsFind {
    @Test func matchesLikeTheCoreReplaceAll() {
        let t = "The folder. FOLDERS of folder_x and Folder!"
        func find(_ q: String, case c: Bool, word: Bool) -> [NSRange] {
            ScreenplayFind.findInText(t, ScreenplayFindOptions(query: q, matchCase: c, wholeWord: word))
        }
        #expect(find("folder", case: true, word: false).count == 2)
        #expect(find("folder", case: false, word: false).count == 4)
        #expect(find("folder", case: false, word: true) == [NSRange(location: 4, length: 6), NSRange(location: 36, length: 6)])
        #expect(ScreenplayFind.findInText("a (b) c", ScreenplayFindOptions(query: "(")) == [NSRange(location: 2, length: 1)])
        #expect(ScreenplayFind.findInText("x", ScreenplayFindOptions(query: "")).isEmpty)
    }

    @Test func scopeToOneSceneAndHiddenNotes() {
        let blocks = suggestScript + [EditorBlock(id: "n", kind: .note, text: "meera note")]
        let all = ScreenplayFind.matches(in: blocks, options: ScreenplayFindOptions(query: "meera"), showNotes: true)
        #expect(all.map(\.block) == [1, 5, 7])
        let hidden = ScreenplayFind.matches(in: blocks, options: ScreenplayFindOptions(query: "meera"), showNotes: false)
        #expect(hidden.map(\.block) == [1, 5])
        let scene = ScreenplayFind.matches(in: blocks, options: ScreenplayFindOptions(query: "meera", sceneId: "s2"), showNotes: true)
        #expect(scene.map(\.block) == [7])
        #expect(ScreenplayFind.matches(in: blocks, options: nil, showNotes: true).isEmpty)
    }
}

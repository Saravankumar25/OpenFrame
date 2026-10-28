// A stand-in for the core's door in the AI screen tests: it records each call and answers with
// the values a test gives. A call can be held until the test lets it go, as a running request is.
// Test fixtures only; the real behaviour is the core's and is covered by the Rust tests.

import Foundation
import OpenFrameCore

final class AIStubCore: CoreInvoking, @unchecked Sendable {
    private let lock = NSLock()
    private var calls: [(op: String, json: String)] = []
    private var replies: [String: String] = [:]
    private var failures: [String: CoreFailure] = [:]
    private var holding: Set<String> = []
    private var waiting: [String: [CheckedContinuation<Void, Never>]] = [:]

    /// The answer to every later call of `op`.
    func reply<T: Encodable>(_ op: String, _ value: T) {
        let json = String(decoding: (try? JSONEncoder().encode(value)) ?? Data("null".utf8), as: UTF8.self)
        lock.withLock { replies[op] = json }
    }

    func fail(_ op: String, _ failure: CoreFailure) {
        lock.withLock { failures[op] = failure }
    }

    /// Calls of `op` wait until `release(op)`.
    func hold(_ op: String) {
        lock.withLock { _ = holding.insert(op) }
    }

    func release(_ op: String) {
        let resume = lock.withLock { () -> [CheckedContinuation<Void, Never>] in
            holding.remove(op)
            return waiting.removeValue(forKey: op) ?? []
        }
        resume.forEach { $0.resume() }
    }

    /// The arguments of each call of `op`, in order.
    func args(_ op: String) -> [[String: Any]] {
        lock.withLock { calls.filter { $0.op == op } }.map {
            (try? JSONSerialization.jsonObject(with: Data($0.json.utf8))) as? [String: Any] ?? [:]
        }
    }

    func count(_ op: String) -> Int { lock.withLock { calls.filter { $0.op == op }.count } }

    func invokeRaw(_ op: String, _ argsJSON: String) async throws -> String {
        let held = lock.withLock { () -> Bool in
            calls.append((op, argsJSON))
            return holding.contains(op)
        }
        if held {
            await withCheckedContinuation { c in
                let stillHeld = lock.withLock { () -> Bool in
                    guard holding.contains(op) else { return false }
                    waiting[op, default: []].append(c)
                    return true
                }
                if !stillHeld { c.resume() }
            }
        }
        let (reply, failure) = lock.withLock { (replies[op], failures[op]) }
        if let failure { throw failure }
        guard let reply else { throw CoreFailure(code: "test.no_reply", message: "No answer for \(op) in this test.") }
        return reply
    }
}

/// Waits until `condition` holds (a call reached the stub, a task finished), at most two seconds.
@MainActor
func eventually(_ condition: () -> Bool) async {
    let deadline = Date().addingTimeInterval(2)
    while !condition(), Date() < deadline { try? await Task.sleep(for: .milliseconds(10)) }
}

enum AIFixtures {
    static func result(
        kind: AiResultKind = .answer, content: String = "Ten scenes.", details: [String] = [], confidence: AiConfidence? = nil,
        provenance: [AiProvenance] = [], items: [AiResultItem] = [], itemsAreAnswer: Bool = false, nav: AiNavTarget? = nil,
        changeSet: AiChangeSetDto? = nil, steps: [AiStepDto] = []
    ) -> AiResultDto {
        AiResultDto(
            id: UUID().uuidString, kind: kind, status: .informational, content: content, details: details, confidence: confidence,
            provenance: provenance, items: items, itemsAreAnswer: itemsAreAnswer, nav: nav, changeSet: changeSet, steps: steps, createdAt: 1)
    }

    static func exchange(_ text: String, conversation: String = "conv1", result: AiResultDto = result()) -> AiExchange {
        AiExchange(requestId: UUID().uuidString, conversationId: conversation, requestText: text, scopeLabel: "Using: Whole Project", result: result, createdAt: 1)
    }

    static func progress(_ conversation: String, _ label: String, step: Int = 1, total: Int = 3, module: String = "ai", name: String = "progress") -> AppEvent {
        .module(module: module, name: name, payload: [
            "conversationId": .string(conversation), "requestId": "r1", "step": .number(Double(step)), "total": .number(Double(total)),
            "label": .string(label),
        ])
    }
}

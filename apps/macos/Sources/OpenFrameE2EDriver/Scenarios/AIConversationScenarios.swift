// Real-app scenarios for the conversational assistant (77-ai-conversation): one conversation that
// keeps its id, the progress line and Cancel, the answer text with its sources, Save conversation
// for review, a navigation answer, proposals applied and rejected in one conversation, and the
// Assistant memory card. They need the real model: the three pinned model files under
// .ai-assets/models (imported through the download card by `Scenarios.aiStart`), an app with the
// engine in Contents/Resources/ai, and OPENFRAME_AI_DETERMINISTIC=1 with a debug build so the
// sampling is fixed. The checks are about the kind of result and its effects, never the wording.

import AppKit
import Foundation

fileprivate extension UI {
    /// A real click that puts the keyboard focus in the Ask box (mouse down and up queued together,
    /// because a text field tracks the mouse until the button comes up).
    func convFocus(_ e: Element) async throws {
        let frame = e.frame
        guard frame.width > 0, frame.height > 0 else { throw DriverError("element has no frame: \(e)") }
        let center = NSPoint(x: frame.midX, y: frame.midY)
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }) else { throw DriverError("no window under \(e)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            guard let ev = NSEvent.mouseEvent(with: type, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                              windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: type == .leftMouseDown ? 1 : 0)
            else { continue }
            NSApp.postEvent(ev, atStart: false)
        }
        await idle(0.4)
    }

    func convOpenAssistant() async throws {
        if exists(id: "ai-panel") { return }
        try await shortcut("j", code: 38)
        _ = try await wait(id: "ai-panel", timeout: 12, "the AI Assistant panel after Command+J")
    }

    /// Types the question and presses Enter; does not wait for the answer.
    func convSend(_ q: String) async throws {
        try await convOpenAssistant()
        let input = try await wait(id: "ai-input", timeout: 60, "the Ask box")
        let ready = Date().addingTimeInterval(60)
        while !input.isEnabled, Date() < ready { await idle(0.3) }
        // The assistant says "Preparing project context" while it indexes the project; a person waits.
        let idx = Date().addingTimeInterval(90)
        while exists(id: "ai-index-notice"), Date() < idx { await idle(0.5) }
        try await convFocus(input)
        try await type(q, into: "ai-input")
        let before = query(id: "ai-question").count
        try await enter()
        // An answer the core gives without the model (a navigation) can replace the waiting
        // question before the next look: either one shows that the question was taken.
        try await convUntil("the question waiting for its answer", timeout: 10) {
            exists(id: "ai-pending-question") || query(id: "ai-question").count > before
        }
    }

    /// "Ask Q": send it and wait until its result shows and nothing is running.
    @discardableResult
    func convAsk(_ q: String, timeout: Double = 240) async throws -> String {
        let before = query(id: "ai-question").count
        try await convSend(q)
        try await convWaitAnswer(after: before, timeout: timeout)
        return convLastKind()
    }

    func convWaitAnswer(after before: Int, timeout: Double = 240) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if query(id: "ai-question").count > before && !exists(id: "ai-working") { return }
            await idle(0.4)
        }
        throw DriverError("no result within \(Int(timeout)) s; the screen shows \(screenText().prefix(400))")
    }

    func convLastKind() -> String {
        all().map(\.identifier).filter { $0.hasPrefix("ai-result-") }.last.map { String($0.dropFirst("ai-result-".count)) } ?? ""
    }

    func convLastAnswer() -> String { query(id: "ai-answer-text").last?.text ?? "" }

    /// Everything a person reads in the element with this id, its children included.
    func convText(_ id: String) -> String {
        var out: [String] = []
        func walk(_ e: Element, _ depth: Int) {
            if !e.text.isEmpty { out.append(e.text) }
            if depth < 12 { for c in e.children { walk(c, depth + 1) } }
        }
        for e in query(id: id) { walk(e, 0) }
        return out.joined(separator: " ")
    }

    /// The Delete button of a remembered entry whose text has these words.
    func convMemoryDelete(_ words: String) -> Element? {
        all().first { $0.role == "AXButton" && $0.label.hasPrefix("Delete: ") && $0.label.lowercased().contains(words) }
    }

    func convUntil(_ what: String, timeout: Double = 30, _ ok: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if ok() { return }
            await idle(0.3)
        }
        try? snapshot("missing-\(what.replacingOccurrences(of: " ", with: "-"))")
        throw DriverError("\(what) within \(Int(timeout)) s; the screen shows \(screenText().prefix(400))")
    }
}

extension Scenarios {
    static let aiConversationGroup: [Scenario] = [
        // Ask, see the progress line while it runs, read the sources, save the conversation, and
        // find the same conversation after a relaunch.
        Scenario("77-ai-conversation/ask-progress-sources-save-and-history", phases: 2, timeout: 900) { ui, phase, ctx in
            let file = ctx.work.appendingPathComponent("conversation-\(UUID().uuidString.prefix(6)).json")
            if phase == 1 {
                try await aiStart(ui, ctx)
                let f = Findings()
                // Nothing to save before the first answer.
                f.check(ui.query(id: "ai-save-conversation").first?.isEnabled == false, "Save conversation for review is off before a conversation")
                let before = ui.query(id: "ai-question").count
                try await ui.convSend("Walk me through the story, scene by scene.")
                // While it runs: the working line and Cancel, and the line changes to what the assistant does.
                var lines: Set<String> = []
                let deadline = Date().addingTimeInterval(240)
                while ui.exists(id: "ai-working"), Date() < deadline {
                    if let l = ui.query(id: "ai-progress-line").first?.text, !l.isEmpty { lines.insert(l) }
                    await ui.idle(0.1)
                }
                try await ui.convWaitAnswer(after: before)
                ui.log("progress lines seen: \(lines.sorted())")
                f.check(!lines.isEmpty, "the working line showed while the answer was written")
                f.check(lines.contains { $0 != "Working on it on this computer\u{2026}" }, "a progress line replaced the waiting line: \(lines.sorted())")
                f.check(isAnswered(ui.convLastKind()), "the result is an answer: \(ui.convLastKind())")
                f.check(!ui.convLastAnswer().contains("<"), "the answer shows no markup")
                // A second question goes on in the same conversation.
                let kind = try await ui.convAsk("How many scenes are in the script?")
                f.check(kind == "answer", "the count is an answer: \(kind)")
                f.check(ui.convLastAnswer().contains("3") || ui.convLastAnswer().lowercased().contains("three"), "the answer states 3: \(ui.convLastAnswer())")
                // The sources are closed until asked for.
                f.check(!ui.exists(id: "ai-sources"), "See sources starts closed")
                if let open = ui.query(id: "ai-see-sources").last { try await ui.press(open) }
                _ = try await ui.wait(id: "ai-sources", timeout: 10, "the sources of the answer")
                f.check(ui.convText("ai-sources").contains("Based on:"), "the sources say what the answer is based on")
                // Save conversation for review writes only where the Save panel says.
                f.check(!FileManager.default.fileExists(atPath: file.path), "nothing is saved on its own")
                let panel = try await ui.choosing(file) { try await ui.press(id: "ai-save-conversation") }
                f.check(panel.title == "Save conversation for review", "the Save panel says what it saves: \(panel.title)")
                try await ui.convUntil("the conversation file is written") { FileManager.default.fileExists(atPath: file.path) }
                let saved = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
                f.check(saved.contains("Walk me through the story") && saved.contains("How many scenes are in the script?"), "the file holds both questions")
                try await ui.waitText("(2 questions)", timeout: 10)
                // Cancelling the panel writes nothing more.
                try FileManager.default.removeItem(at: file)
                _ = try await ui.choosing(nil) { try await ui.press(id: "ai-save-conversation") }
                await ui.idle(1)
                f.check(!FileManager.default.fileExists(atPath: file.path), "a cancelled Save panel writes nothing")
                try f.finish()
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.convOpenAssistant()
                // The earlier turns are loaded when the panel opens, and the next question continues them.
                try await ui.waitText("How many scenes are in the script?", timeout: 30)
                try await ui.convAsk("And which one is at the railway station?")
                try ui.expect(isAnswered(ui.convLastKind()), "the follow-up is answered: \(ui.convLastKind())")
                try ui.expect(ui.query(id: "ai-question").count >= 3, "the follow-up went on in the same conversation")
            }
        },

        // Cancel stops the running request; its answer says nothing was changed, and the Ask box is free.
        Scenario("77-ai-conversation/cancel-stops-the-request", timeout: 600) { ui, _, ctx in
            try await aiStart(ui, ctx)
            let before = ui.query(id: "ai-question").count
            try await ui.convSend("Write a new scene where Meera and Arjun argue at the railway station, then summarise every scene.")
            // Cancel until the core takes it (it is pressable again while nothing could be stopped yet).
            let deadline = Date().addingTimeInterval(60)
            while ui.exists(id: "ai-working"), Date() < deadline {
                if ui.screenText().contains("Stopping\u{2026}") { break }
                if let c = ui.query(id: "ai-cancel").first, c.isEnabled { try await ui.press(c) }
                await ui.idle(0.3)
            }
            try await ui.convWaitAnswer(after: before, timeout: 240)
            let answer = ui.convLastAnswer()
            ui.log("answer after Cancel: \(answer.prefix(200))")
            try ui.expect(answer.contains("Stopped. Nothing was changed.") || isAnswered(ui.convLastKind()), "the request ended in an ordinary answer")
            try ui.expect(ui.convLastKind() != "proposal", "a stopped request proposes nothing")
            let input = try await ui.wait(id: "ai-input", timeout: 10)
            try ui.expect(input.isEnabled, "the Ask box is free again")
            try await ui.convAsk("How many scenes are in the script?")
            try ui.expect(ui.convLastKind() == "answer", "the next question is answered after a Cancel")
        },

        // A navigation answer opens the place: a scene in the Screenplay, a Production tab by its sub.
        Scenario("77-ai-conversation/navigation-answers-open-scene-and-production-tab", timeout: 600) { ui, _, ctx in
            try await aiStart(ui, ctx)
            try await ui.go("Home")
            var kind = try await ui.convAsk("Open Scene 2.")
            try ui.expect(kind == "navigate", "Open Scene 2 is a navigation: \(kind)")
            _ = try await ui.wait(id: "screenplay-editor", timeout: 20, "the Screenplay workspace")
            kind = try await ui.convAsk("Go to the budget.")
            try ui.expect(kind == "navigate", "Go to the budget is a navigation: \(kind)")
            _ = try await ui.wait(id: "budget-workspace", timeout: 20, "Production, Budget (the link's tab, not Overview)")
            kind = try await ui.convAsk("Open the call sheets.")
            try ui.expect(kind == "navigate", "Open the call sheets is a navigation: \(kind)")
            try await ui.convUntil("the Call Sheets workspace shows") { ui.exists(id: "callsheet-list") || ui.exists(id: "callsheet-empty") }
        },

        // Two proposals in one conversation: the first applied, the second rejected.
        Scenario("77-ai-conversation/proposal-applied-then-another-rejected", timeout: 900) { ui, _, ctx in
            try await aiStart(ui, ctx)
            var kind = try await ui.convAsk("Create a task to scout the railway station.")
            try ui.expect(kind == "proposal", "the request is a proposal: \(kind)")
            try await ui.press(try await ui.wait(id: "ai-apply", timeout: 20, "Apply Changes"))
            try await ui.convUntil("the card says it was applied") { ui.query(id: "ai-change-state").last?.text.hasPrefix("Applied") == true }
            kind = try await ui.convAsk("Create a task to book the bus depot.")
            try ui.expect(kind == "proposal", "the second request is a proposal: \(kind)")
            try await ui.press(try await ui.wait(id: "ai-reject", timeout: 20, "Reject"))
            try await ui.convUntil("the card says nothing was changed") { ui.query(id: "ai-change-state").last?.text == "Rejected. Nothing was changed." }
            // The Tasks list holds the applied task and not the rejected one (the questions and the
            // cards in the panel name both, so only the task rows count).
            try await ui.go("Notes and Tasks")
            _ = try await ui.wait(id: "tasks-list", timeout: 20, "the Tasks list")
            @MainActor func rows() -> [String] { ui.all().filter { $0.identifier.hasPrefix("task-row-") }.map { $0.label.lowercased() } }
            try await ui.convUntil("the applied task is in the Tasks list", timeout: 20) { rows().contains { $0.contains("railway station") } }
            try ui.expect(!rows().contains { $0.contains("bus depot") }, "the rejected task was not created: \(rows())")
            try ui.expect(rows().count == 1, "only the applied task was created: \(rows())")
        },

        // A fact the person states is remembered; Settings > Offline AI lists it and Delete forgets it.
        Scenario("77-ai-conversation/assistant-memory-entry-deleted", timeout: 900) { ui, _, ctx in
            try await aiStart(ui, ctx)
            try await ui.convAsk("I'm the assistant director.")
            try await ui.openProjectSettings(page: "Offline AI")
            _ = try await ui.wait(id: "ai-memory-card", timeout: 20, "the Assistant memory card")
            // The entry keeps the person's words; its Delete button names it.
            try await ui.convUntil("the remembered fact with its Delete button") { ui.convMemoryDelete("assistant director") != nil }
            try ui.expect(ui.convText("ai-memory-card").contains("Fact, saved"), "the entry says it is a fact and when it was saved")
            try await ui.press(ui.convMemoryDelete("assistant director")!)
            try await ui.waitText("Forgotten.", timeout: 15)
            try await ui.convUntil("the entry is gone", timeout: 15) { ui.convMemoryDelete("assistant director") == nil }
            try await ui.waitText("Nothing is remembered yet.", timeout: 15)
            try await closeSettings(ui)
        },
    ]
}

// Runs one scenario phase inside the app and writes its result for the runner script.
//
// Environment (set by apps/macos/E2E/run.sh, never by the app):
//   OPENFRAME_E2E_SCENARIO  scenario name, e.g. "01-journey/create-and-type"
//   OPENFRAME_E2E_PHASE     1-based launch number for scenarios that restart the app
//   OPENFRAME_E2E_OUT       folder for result.json, the step log and window pictures
//   OPENFRAME_E2E_FIXTURES  folder with the shared test files (tests/fixtures)

import AppKit
import Foundation

/// How a phase ends. The runner checks the matching exit.
enum PhaseEnd: String {
    /// The scenario quits the app through the Quit command (the clean shutdown path).
    case quit
    /// The scenario stops and waits; the runner kills the app with SIGKILL (a crash).
    case hold
}

struct Scenario {
    let name: String
    let phases: Int
    let timeout: Double
    let end: (Int) -> PhaseEnd
    let body: @MainActor (UI, Int, Context) async throws -> Void

    init(_ name: String, phases: Int = 1, timeout: Double = 240, end: @escaping (Int) -> PhaseEnd = { _ in .quit },
         _ body: @escaping @MainActor (UI, Int, Context) async throws -> Void) {
        self.name = name
        self.phases = phases
        self.timeout = timeout
        self.end = end
        self.body = body
    }
}

/// Facts a scenario needs from outside the app.
struct Context {
    let env: [String: String]
    var fixtures: URL { URL(fileURLWithPath: env["OPENFRAME_E2E_FIXTURES"] ?? "/nonexistent") }
    var dataRoot: URL { URL(fileURLWithPath: env["OPENFRAME_DATA_ROOT"] ?? "/nonexistent") }
    var work: URL { URL(fileURLWithPath: env["OPENFRAME_E2E_WORK"] ?? NSTemporaryDirectory()) }
}

@_cdecl("openframe_e2e_start")
public func openframe_e2e_start() {
    let env = ProcessInfo.processInfo.environment
    guard let name = env["OPENFRAME_E2E_SCENARIO"], let out = env["OPENFRAME_E2E_OUT"] else { return }
    let phase = Int(env["OPENFRAME_E2E_PHASE"] ?? "1") ?? 1
    let outDir = URL(fileURLWithPath: out, isDirectory: true)
    try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
    // Start once the app has its run loop and first window.
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
        MainActor.assumeIsolated { Runner(name: name, phase: phase, out: outDir, env: env).start() }
    }
}

@MainActor
final class Runner {
    let name: String
    let phase: Int
    let out: URL
    let env: [String: String]
    private var lines: [String] = []
    private let started = Date()

    init(name: String, phase: Int, out: URL, env: [String: String]) {
        self.name = name
        self.phase = phase
        self.out = out
        self.env = env
    }

    func log(_ s: String) {
        let line = String(format: "%7.2f  ", Date().timeIntervalSince(started)) + s
        lines.append(line)
        try? lines.joined(separator: "\n").write(to: out.appendingPathComponent("steps-\(phase).log"), atomically: true, encoding: .utf8)
    }

    func start() {
        if name == "__list__" {
            let list = Scenarios.all.map { s in
                ["name": s.name, "phases": s.phases, "timeout": s.timeout,
                 "ends": (1...s.phases).map { s.end($0).rawValue }] as [String: Any]
            }
            if let data = try? JSONSerialization.data(withJSONObject: list, options: [.prettyPrinted]) {
                try? data.write(to: out.appendingPathComponent("scenarios.json"))
            }
            exit(0)
        }
        guard let scenario = Scenarios.all.first(where: { $0.name == name }) else {
            finish(status: "error", message: "unknown scenario \(name)", end: .quit)
            return
        }
        UI.wakeAccessibility()
        let ui = UI(artifacts: out) { [weak self] in self?.log($0) }
        let end = scenario.end(phase)
        // A watchdog: a scenario that hangs fails instead of holding the run.
        DispatchQueue.main.asyncAfter(deadline: .now() + scenario.timeout) { [weak self] in
            MainActor.assumeIsolated {
                try? ui.snapshot("timeout")
                self?.finish(status: "fail", message: "timed out after \(Int(scenario.timeout)) s", end: .quit, force: true)
            }
        }
        Task { @MainActor in
            do {
                log("scenario \(name) phase \(phase) of \(scenario.phases)")
                try await scenario.body(ui, phase, Context(env: env))
                try? ui.snapshot("end")
                finish(status: "pass", message: "", end: end)
            } catch {
                log("FAILED: \(error)")
                try? ui.snapshot("failure")
                // Keep the on-screen text for diagnosis.
                try? ui.screenText().write(to: out.appendingPathComponent("screen-\(phase).txt"), atomically: true, encoding: .utf8)
                try? ui.all().map(\.description).joined(separator: "\n").write(to: out.appendingPathComponent("tree-\(phase).txt"), atomically: true, encoding: .utf8)
                finish(status: "fail", message: "\(error)", end: .quit)
            }
        }
    }

    private var finished = false

    func finish(status: String, message: String, end: PhaseEnd, force: Bool = false) {
        guard !finished else { return }
        finished = true
        let result: [String: Any] = [
            "scenario": name, "phase": phase, "status": status, "message": message,
            "seconds": Date().timeIntervalSince(started), "end": end.rawValue,
        ]
        if let data = try? JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: out.appendingPathComponent("result-\(phase).json"))
        }
        log("result \(status) \(message)")
        switch end {
        case .hold:
            // The runner sends SIGKILL now. Stay alive, doing nothing, until it does.
            break
        case .quit:
            if force { exit(status == "pass" ? 0 : 3) }
            // Command+Q goes into the event queue, as a key press does, so the app's quit path
            // runs from the event loop and not from inside this task.
            // A sheet takes the keyboard from its window, as it does for a person.
            let front = NSApp.keyWindow ?? NSApp.windows.first
            let w = front?.attachedSheet ?? front
            for win in NSApp.windows {
                log("window \(type(of: win)) visible=\(win.isVisible) title=\(win.title) sheet=\(win.attachedSheet.map { String(describing: type(of: $0)) } ?? "-") level=\(win.level.rawValue)")
            }
            NotificationCenter.default.addObserver(forName: NSApplication.willTerminateNotification, object: nil, queue: nil) { [weak self] _ in
                MainActor.assumeIsolated { self?.log("the app reports it will terminate") }
            }
            log("modal window: \(NSApp.modalWindow.map { String(describing: type(of: $0)) } ?? "none")")
            log("quit with Command+Q; key window \(w.map { String(describing: type(of: $0)) } ?? "none"), first responder \(w?.firstResponder.map { String(describing: type(of: $0)) } ?? "none")")
            for type in [NSEvent.EventType.keyDown, .keyUp] {
                if let ev = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: [.command],
                                             timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: w?.windowNumber ?? 0,
                                             context: nil, characters: "q", charactersIgnoringModifiers: "q",
                                             isARepeat: false, keyCode: 12) {
                    NSApp.postEvent(ev, atStart: false)
                }
            }
            // Command+Q that nothing acts on within 5 seconds is a finding of its own; quit
            // through the menu command then, so the run can go on.
            DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
                self.log("Command+Q was not acted on within 5 s; choosing Quit from the app menu")
                try? "Command+Q was ignored".write(to: self.out.appendingPathComponent("cmdq-ignored-\(self.phase).txt"), atomically: true, encoding: .utf8)
                NSApp.perform(#selector(NSApplication.terminate(_:)), with: nil, afterDelay: 0)
            }
            // A quit that does not finish within 20 seconds is a failure of its own.
            DispatchQueue.main.asyncAfter(deadline: .now() + 20) {
                try? "quit did not finish within 20 s".write(to: self.out.appendingPathComponent("quit-hang-\(self.phase).txt"), atomically: true, encoding: .utf8)
                exit(4)
            }
        }
    }
}

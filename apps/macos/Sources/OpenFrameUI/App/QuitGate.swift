// Quitting with a sheet open. AppKit refuses to quit while a window shows a sheet it cannot close
// itself (a SwiftUI sheet such as Project Settings, or the import, export and print sheets): it
// beeps, brings the sheet forward and never asks the app delegate, so Command+Q and Quit in the
// Dock looked dead. As on Windows, where a dialog never keeps the window open, every way to quit
// closes the sheets first, as Escape would, and then quits. The editors' text is stored by
// `applicationShouldTerminate` as usual (5 second cap). What was typed into a sheet and not yet
// applied is let go, as Cancel would. A sheet that refuses to close (a storage move under way)
// stays in front and AppKit beeps, so the person sees why the app is still open.

import AppKit
import CoreServices

@MainActor
enum QuitGate {
    /// How long the sheets get to close after Escape before they are taken down.
    static let sheetWait: Duration = .milliseconds(1500)
    /// The quit itself; tests replace it so the test run goes on. It starts from the run loop,
    /// not from inside a task: while AppKit waits for `applicationShouldTerminate` to answer it
    /// runs a nested loop, and a task running on the main queue would keep the editors' store
    /// (main actor work) from running until the 5 second cap.
    static var terminate: () -> Void = { NSApp.perform(#selector(NSApplication.terminate(_:)), with: nil, afterDelay: 0) }
    private static var quitting = false

    /// The Quit command (Command+Q).
    static func quit() {
        guard !hasSheets() else {
            Task { await closeSheetsThenQuit() }
            return
        }
        terminate()
    }

    static func closeSheetsThenQuit(windows: @escaping () -> [NSWindow] = { NSApp.windows }) async {
        // A second Command+Q while the sheets close is the same request.
        guard !quitting else { return }
        quitting = true
        defer { quitting = false }
        await closeSheets(windows: windows)
        terminate()
    }

    /// Asks every sheet to close as Escape does (SwiftUI updates its own state that way), waits
    /// for them to go, and takes down what is left (AppKit sheets that do not answer Escape).
    static func closeSheets(windows: () -> [NSWindow]) async {
        // Cancel first; a sheet that has no Cancel gets the Escape key itself.
        for sheet in sheets(of: windows()).reversed() {
            _ = (sheet.firstResponder ?? sheet).tryToPerform(#selector(NSResponder.cancelOperation(_:)), with: nil)
        }
        await waitForSheets(windows, .milliseconds(400))
        for sheet in sheets(of: windows()).reversed() { pressEscape(in: sheet) }
        await waitForSheets(windows, sheetWait)
        forceClose(windows())
    }

    private static func waitForSheets(_ windows: () -> [NSWindow], _ limit: Duration) async {
        let clock = ContinuousClock()
        let until = clock.now.advanced(by: limit)
        while hasSheets(windows()) && clock.now < until {
            try? await Task.sleep(for: .milliseconds(50))
        }
    }

    static func hasSheets(_ windows: [NSWindow]? = nil) -> Bool {
        (windows ?? NSApp.windows).contains { !$0.sheets.isEmpty }
    }

    /// Every sheet of the given windows, outermost first.
    static func sheets(of windows: [NSWindow]) -> [NSWindow] {
        var all: [NSWindow] = []
        func walk(_ w: NSWindow) {
            for s in w.sheets {
                all.append(s)
                walk(s)
            }
        }
        for w in windows where w.sheetParent == nil { walk(w) }
        return all
    }

    private static func pressEscape(in sheet: NSWindow) {
        for type in [NSEvent.EventType.keyDown, .keyUp] {
            if let ev = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                         windowNumber: sheet.windowNumber, context: nil, characters: "\u{1b}",
                                         charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                sheet.sendEvent(ev)
            }
        }
    }

    /// Ends the sheets still attached, innermost first, and takes them off the screen. Returns
    /// how many there were.
    @discardableResult
    static func forceClose(_ windows: [NSWindow]) -> Int {
        let left = sheets(of: windows)
        for sheet in left.reversed() {
            sheet.sheetParent?.endSheet(sheet)
            sheet.orderOut(nil)
        }
        return left.count
    }

    // MARK: Quit from the Dock, logout and restart

    /// AppKit's own handler for the "quit" Apple event.
    private static var appKitQuit: (handler: AEEventHandlerUPP, refcon: UnsafeMutableRawPointer?)?

    /// Puts the sheet closing in front of AppKit's handler for the "quit" Apple event, which the
    /// Dock, logout and restart send. Call once AppKit has installed its handlers.
    static func installQuitEventHandler() {
        guard appKitQuit == nil else { return }
        var handler: AEEventHandlerUPP?
        var refcon: UnsafeMutableRawPointer?
        let quitClass = AEEventClass(kCoreEventClass), quitID = AEEventID(kAEQuitApplication)
        guard AEGetEventHandler(quitClass, quitID, &handler, &refcon, false) == noErr, let handler else { return }
        appKitQuit = (handler, refcon)
        AEInstallEventHandler(quitClass, quitID, { event, reply, _ in
            MainActor.assumeIsolated { QuitGate.handleQuitEvent(event, reply) }
        }, nil, false)
    }

    private static func handleQuitEvent(_ event: UnsafePointer<AppleEvent>?, _ reply: UnsafeMutablePointer<AppleEvent>?) -> OSErr {
        // With a sheet open AppKit's handler would refuse; the event is answered now and the app
        // quits once the sheets are closed.
        if hasSheets() {
            Task { await closeSheetsThenQuit() }
            return OSErr(noErr)
        }
        guard let next = appKitQuit else { return OSErr(errAEEventNotHandled) }
        return next.handler(event, reply, next.refcon)
    }
}

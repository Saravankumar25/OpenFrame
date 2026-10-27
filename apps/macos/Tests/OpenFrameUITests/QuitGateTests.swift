// Quitting with a sheet open. AppKit does not ask the app delegate while a window shows a sheet
// it cannot close itself, so Command+Q and Quit in the Dock did nothing (end-to-end runs: the
// Project Settings sheet, the export sheet and Print preview). The Quit command and the "quit"
// Apple event close the sheets first (Cancel, then Escape, then taken down) and then quit; these
// tests check that part with the quit itself replaced, so the test run goes on.

import AppKit
import CoreServices
import Testing
@testable import OpenFrameUI

@Suite("Quit with sheets open", .serialized)
@MainActor
struct QuitGateTests {
    /// A sheet's content that closes the sheet on Cancel, as a SwiftUI sheet does on Escape.
    final class CancelView: NSView {
        var cancelled = 0
        override var acceptsFirstResponder: Bool { true }
        override func cancelOperation(_ sender: Any?) {
            cancelled += 1
            if let w = window { w.sheetParent?.endSheet(w) }
        }
    }

    private func window() -> NSWindow {
        _ = NSApplication.shared
        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        w.isReleasedWhenClosed = false
        w.orderFront(nil)
        return w
    }

    private func sheet(on parent: NSWindow, content: NSView? = nil) -> NSWindow {
        let s = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 300, height: 200), styleMask: [.titled], backing: .buffered, defer: false)
        s.isReleasedWhenClosed = false
        if let content {
            s.contentView = content
            s.makeFirstResponder(content)
        }
        parent.beginSheet(s)
        return s
    }

    private static let appTerminate = QuitGate.terminate

    private func replaceQuit() -> () -> Int {
        var quits = 0
        QuitGate.terminate = { quits += 1 }
        return { quits }
    }

    @Test func takesDownASheetAndASheetOnASheet() {
        let main = window()
        defer { main.orderOut(nil) }
        let settings = sheet(on: main)
        let export = sheet(on: settings)
        #expect(QuitGate.sheets(of: [main, settings, export]) == [settings, export])

        #expect(QuitGate.forceClose([main, settings, export]) == 2)
        #expect(main.attachedSheet == nil)
        #expect(settings.attachedSheet == nil)
        #expect(!settings.isVisible)
        #expect(!export.isVisible)
        #expect(!QuitGate.hasSheets([main, settings, export]))
    }

    @Test func aSheetWithCancelClosesItselfBeforeTheQuit() async {
        let quits = replaceQuit()
        defer { QuitGate.terminate = Self.appTerminate }
        let main = window()
        defer { main.orderOut(nil) }
        let view = CancelView(frame: NSRect(x: 0, y: 0, width: 300, height: 200))
        _ = sheet(on: main, content: view)

        await QuitGate.closeSheetsThenQuit(windows: { [main] })

        #expect(view.cancelled == 1)
        #expect(main.attachedSheet == nil)
        #expect(quits() == 1)
    }

    @Test func aSheetThatIgnoresEscapeIsTakenDownAndTheAppStillQuits() async {
        let quits = replaceQuit()
        defer { QuitGate.terminate = Self.appTerminate }
        let main = window()
        defer { main.orderOut(nil) }
        let s = sheet(on: main)

        await QuitGate.closeSheetsThenQuit(windows: { [main] })

        #expect(main.attachedSheet == nil)
        #expect(!s.isVisible)
        #expect(quits() == 1)
    }

    @Test func withoutSheetsTheQuitIsImmediate() {
        let quits = replaceQuit()
        defer { QuitGate.terminate = Self.appTerminate }
        let main = window()
        defer { main.orderOut(nil) }
        #expect(!QuitGate.hasSheets([main]))
        QuitGate.quit()
        #expect(quits() == 1)
    }

    @Test func quitAppleEventClosesSheetsThenQuits() async throws {
        // Stand in for AppKit's handler, so the event does not end the test run.
        let quitClass = AEEventClass(kCoreEventClass), quitID = AEEventID(kAEQuitApplication)
        QuitEventProbe.calls = 0
        #expect(AEInstallEventHandler(quitClass, quitID, { _, _, _ in
            QuitEventProbe.calls += 1
            return OSErr(noErr)
        }, nil, false) == OSErr(noErr))
        QuitGate.installQuitEventHandler()
        let quits = replaceQuit()
        defer { QuitGate.terminate = Self.appTerminate }
        let event = NSAppleEventDescriptor(
            eventClass: quitClass, eventID: quitID, targetDescriptor: .currentProcess(),
            returnID: AEReturnID(kAutoGenerateReturnID), transactionID: AETransactionID(kAnyTransactionID))

        // With a sheet open: the sheet closes and the app quits; AppKit's handler is not asked.
        let main = window()
        defer { main.orderOut(nil) }
        let s = sheet(on: main)
        _ = try event.sendEvent(options: [.noReply], timeout: 5)
        for _ in 0..<100 where quits() == 0 { try await Task.sleep(for: .milliseconds(50)) }
        #expect(quits() == 1)
        #expect(main.attachedSheet == nil)
        #expect(!s.isVisible)
        #expect(QuitEventProbe.calls == 0)

        // With no sheet: AppKit's own handler answers, as before.
        _ = try event.sendEvent(options: [.noReply], timeout: 5)
        #expect(QuitEventProbe.calls == 1)
    }
}

private enum QuitEventProbe {
    nonisolated(unsafe) static var calls = 0
}

// The main window fits the screen: the frame is brought inside the screen's free area, the
// project sidebar comes back at launch, the AI Assistant narrows in a narrow window, and a page
// that scrolls sideways keeps its minimum width. The real-app proof is the end-to-end group
// 95-reachability/window-fits-* in Sources/OpenFrameE2EDriver/Scenarios/WindowFitScenarios.swift.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameUI

@MainActor
struct WindowFitTests {
    /// A 1280 by 720 screen with the menu bar and a Dock at the bottom, as on the build machine.
    let screen = NSRect(x: 0, y: 58, width: 1280, height: 632)

    @Test func aWindowThatFitsKeepsItsFrame() {
        let frame = NSRect(x: 100, y: 80, width: 900, height: 600)
        #expect(WindowFit.fitted(frame, in: screen, move: true) == frame)
        #expect(WindowFit.fitted(frame, in: screen, move: false) == frame)
    }

    @Test func theFirstLaunchSizeIsCutToTheScreenWithItsTopLeftKept() {
        // The default 1360 by 860 window, placed with its top at the top of the screen.
        let frame = NSRect(x: 0, y: 690 - 860, width: 1360, height: 860)
        let fitted = WindowFit.fitted(frame, in: screen, move: false)
        #expect(fitted == screen)
    }

    @Test func aRestoredWindowUnderTheDockMovesUp() {
        // The frame the owner's app had saved: 20 points under the Dock.
        let frame = NSRect(x: 20, y: 38, width: 1240, height: 652)
        let fitted = WindowFit.fitted(frame, in: screen, move: true)
        #expect(screen.contains(fitted))
        #expect(fitted.width == 1240)
        #expect(fitted.height == 632)
    }

    @Test func aWindowDraggedPartlyAwayStaysWhenOnlyResized() {
        let frame = NSRect(x: 900, y: 100, width: 700, height: 500)
        #expect(WindowFit.fitted(frame, in: screen, move: false) == frame)
        let moved = WindowFit.fitted(frame, in: screen, move: true)
        #expect(screen.contains(moved))
        #expect(moved.size == frame.size)
    }

    @Test func noScreenLeavesTheFrameAlone() {
        let frame = NSRect(x: 0, y: 0, width: 2000, height: 2000)
        #expect(WindowFit.fitted(frame, in: .zero, move: true) == frame)
    }

    @Test func theMinimumFollowsTheScreen() {
        let fit = WindowFit()
        #expect(fit.minimum == WindowFit.preferred)
        let window = NSWindow(contentRect: NSRect(x: 0, y: 100, width: 1000, height: 700),
                              styleMask: [.titled, .resizable], backing: .buffered, defer: true)
        fit.update(for: window)
        let vf = WindowFit.visibleFrame(of: window)
        let chrome = window.frame.height - window.contentLayoutRect.height
        #expect(fit.minimum.width == min(900, vf.width.rounded(.down)))
        #expect(fit.minimum.height == min(600, (vf.height - chrome).rounded(.down)))
        #expect(fit.minimum.height <= 600)
    }

    @Test func aCollapsedSidebarComesBackAtLaunchWithItsWidth() throws {
        let defaults = try #require(UserDefaults(suiteName: "openframe.tests.windowfit.\(UUID().uuidString)"))
        let key = "NSSplitView Subview Frames main-AppWindow-1, SidebarNavigationSplitView"
        defaults.set(["0.000000, 0.000000, 198.000000, 632.000000, YES, NO", "0.000000, 0.000000, 1240.000000, 652.000000, NO, NO"], forKey: key)
        defaults.set(["keep"], forKey: "NSSplitView Subview Frames other")
        SidebarRestore.clearCollapsed(in: defaults)
        let frames = try #require(defaults.array(forKey: key) as? [String])
        // Not collapsed any more, and a width under the sidebar's minimum comes back as 220.
        #expect(frames[0] == "0.000000, 0.000000, 220.000000, 632.000000, NO, NO")
        #expect(frames[1] == "0.000000, 0.000000, 1240.000000, 652.000000, NO, NO")
        #expect(defaults.array(forKey: "NSSplitView Subview Frames other") as? [String] == ["keep"])
    }

    @Test func theAssistantNarrowsInANarrowWindow() {
        #expect(ProjectView.assistantWidth(for: 1060) == 360)
        #expect(ProjectView.assistantWidth(for: 680) == 306)
        #expect(ProjectView.assistantWidth(for: 400) == 220)
        // Dragged wider or narrower, the panel stays between 300 points and 45 percent (at most 640).
        #expect(ProjectView.assistantWidth(for: 1600, preferred: 900) == 640)
        #expect(ProjectView.assistantWidth(for: 1200, preferred: 900) == 540)
        #expect(ProjectView.assistantWidth(for: 1200, preferred: 100) == 300)
        #expect(ProjectView.assistantWidth(for: 1200, preferred: 420) == 420)
    }

    @Test func aPageWiderThanTheWindowKeepsItsMinimumWidth() {
        // Fixed columns of 600 points in a 400 point pane: the page is 600 wide and scrolls.
        let narrow = NSHostingView(rootView: AtLeastWidth(width: 400) { HStack(spacing: 0) { Color.red.frame(width: 300); Color.blue.frame(width: 300) } })
        #expect(narrow.fittingSize.width == 600)
        // Text-only content takes the pane's width and wraps.
        let text = NSHostingView(rootView: AtLeastWidth(width: 400) { Text(String(repeating: "word ", count: 200)) })
        #expect(text.fittingSize.width == 400)
    }

    @Test func theScreenplayPageNeverGetsNarrowerThanItsText() {
        let view = ScreenplayTextView.make()
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 300, height: 400))
        scroll.documentView = view
        ScreenplayEditorHost.fitDocumentWidth(scroll)
        let page = (view.textContainer?.size.width ?? 0) + 2 * ScreenplayTextView.pageGutter
        #expect(view.frame.width == max(scroll.contentView.bounds.width, page))
        scroll.setFrameSize(NSSize(width: 1400, height: 400))
        ScreenplayEditorHost.fitDocumentWidth(scroll)
        #expect(view.frame.width == max(scroll.contentView.bounds.width, page))
    }
}

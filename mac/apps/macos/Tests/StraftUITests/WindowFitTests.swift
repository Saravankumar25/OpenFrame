// The main window fits the screen: the frame is brought inside the screen's free area, the
// project sidebar comes back at launch, Straft AI narrows in a narrow window, a page that
// scrolls sideways keeps its minimum width, and the panes of a page add up to the room there is. The real-app proof is the end-to-end group
// 95-reachability/window-fits-* in Sources/StraftE2EDriver/Scenarios/WindowFitScenarios.swift.

import AppKit
import Foundation
import SwiftUI
import Testing
@testable import StraftUI

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
        let defaults = try #require(UserDefaults(suiteName: "straft.tests.windowfit.\(UUID().uuidString)"))
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

    // MARK: Pane widths

    /// The owner's display: a maximized 1280-point window with the 228-point project sidebar.
    private let maximizedDetail: CGFloat = 1052

    @Test func aSidePaneAndThePageAddUpToTheRoomThereIs() {
        for available: CGFloat in [420, 680, 691, 900, 1052, 1212, 1500, 2200] {
            let fit = PaneFit.sidePane(available: available, minimum: 320, ideal: 380, maximum: 520)
            if fit.floats {
                #expect(fit.width <= available)
                #expect(fit.pageWidth == available)
            } else {
                #expect(fit.width + 1 + fit.pageWidth == available)
                #expect(fit.pageWidth >= PaneFit.pageMinimum)
                #expect(fit.width >= 320)
            }
        }
    }

    @Test func aSidePaneGivesWayToThePageBeforeItFloats() {
        // The usual width where there is room, less towards the minimum on the owner's display.
        #expect(PaneFit.sidePane(available: 1500, minimum: 280, ideal: 340, maximum: 480).width == 340)
        #expect(PaneFit.sidePane(available: maximizedDetail, minimum: 280, ideal: 340, maximum: 480).width == 340)
        #expect(PaneFit.sidePane(available: maximizedDetail, minimum: 320, ideal: 380, maximum: 520).width == 351)
        #expect(PaneFit.sidePane(available: 900, minimum: 320, ideal: 380, maximum: 520).width == 320)
        // No room for the page beside it: the pane floats over the page at its minimum.
        let tight = PaneFit.sidePane(available: 500, minimum: 340, ideal: 400, maximum: 540)
        #expect(tight.floats && tight.width == 340)
    }

    @Test func aDraggedSidePaneKeepsItsWidthWhileThePageHasItsMinimum() {
        #expect(PaneFit.sidePane(available: 1500, minimum: 320, ideal: 380, maximum: 520, dragged: 500).width == 500)
        #expect(PaneFit.sidePane(available: 1500, minimum: 320, ideal: 380, maximum: 520, dragged: 900).width == 520)
        #expect(PaneFit.sidePane(available: 800, minimum: 320, ideal: 380, maximum: 520, dragged: 500).pageWidth == PaneFit.pageMinimum)
        #expect(PaneFit.sidePaneLimit(available: 800, minimum: 320, maximum: 520) == 479)
    }

    @Test func theSceneListGivesWayToTheScriptThenStepsAside() {
        // Room for both at their usual widths.
        #expect(PaneFit.sceneList(available: 1500) == 290)
        #expect(PaneFit.sceneList(available: maximizedDetail) == 290)
        // The owner's display with the inspector open: the list narrows and all the script shows.
        let page = PaneFit.sidePane(available: maximizedDetail, minimum: 280, ideal: 340, maximum: 480).pageWidth
        #expect(PaneFit.sceneList(available: page) == 160)
        // Tighter: down to the list's minimum, then no list.
        #expect(PaneFit.sceneList(available: 461) == 120)
        #expect(PaneFit.sceneList(available: 460) == nil)
        // A list the person made narrow stays narrow, and never takes the script's minimum.
        #expect(PaneFit.sceneList(available: 1500, preferred: 130) == 130)
        #expect(PaneFit.sceneList(available: 500, preferred: 340) == 159)
    }

    @Test func theFileTableKeepsItsNameColumn() {
        // Every column at a maximized window with nothing open.
        #expect(FileTableColumns.fitting(FileTableColumns.tableWidth(pageWidth: maximizedDetail)) == FileTableColumns(kind: true, place: true, modified: true))
        // The details panel open: Where goes first, then Type, and the name keeps its minimum.
        let beside = PaneFit.sidePane(available: maximizedDetail, minimum: 320, ideal: 380, maximum: 520).pageWidth
        #expect(FileTableColumns.fitting(FileTableColumns.tableWidth(pageWidth: beside)) == FileTableColumns(kind: false, place: false, modified: true))
        for width in stride(from: FileTableColumns.smallest, through: 1000, by: 13) {
            let c = FileTableColumns.fitting(width)
            var used = FileTableColumns.smallest
            if c.modified { used += FileTableColumns.modifiedWidth + FileTableColumns.gap }
            if c.kind { used += FileTableColumns.kindWidth + FileTableColumns.gap }
            if c.place { used += FileTableColumns.placeWidth + FileTableColumns.gap }
            #expect(used <= width)
        }
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

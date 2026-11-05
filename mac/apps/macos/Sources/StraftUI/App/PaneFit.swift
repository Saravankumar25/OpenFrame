// Panes that always fit the window.
//
// The system inspector and HSplitView could not do this. An inspector inside the project window's
// detail column lays the page out under itself and reports its own width as a safe area, and each
// pane of a split view holds on to its current width, so the page's minimum, the scene list's
// current width and the inspector added up to more than the window (1597 points in a 1052-point
// column at a maximized 1280-point window). A view that does not fit is centred on its frame, so
// the row ran off both ends: under the project sidebar and past the window's right edge.
//
// Here the widths are worked out from the room there is, in one place, and given to the panes.
// The page takes what is left and scrolls its own content sideways if it must. With no room for
// the page beside a side pane, the pane floats over the page's trailing edge.

import AppKit
import SwiftUI

/// The width rules for the panes of a project page. No views, so they can be tested directly.
enum PaneFit {
    /// The page keeps at least this much beside a side pane; with less, the pane floats over it.
    static let pageMinimum: CGFloat = 320
    /// Side panes come down from their usual width, as far as their minimum, to leave the page this much.
    static let pageComfort: CGFloat = 700
    /// The script page pane never goes below this beside the scene list.
    static let scriptMinimum: CGFloat = 340
    /// The scene list gives way first, as far as `sceneListSnug`, to leave the script this much. A
    /// narrower pane centres the page, so a whole line and its scene numbers still show at 100 percent.
    static let scriptComfort: CGFloat = 550

    static let sceneListMinimum: CGFloat = 120
    static let sceneListSnug: CGFloat = 160
    static let sceneListUsual: CGFloat = 290
    static let sceneListMaximum: CGFloat = 340

    struct SidePane: Equatable {
        var width: CGFloat
        /// True when the pane floats over the page because the two do not fit side by side.
        var floats: Bool
        /// The width left for the page.
        var pageWidth: CGFloat
    }

    /// A side pane in `available` points. `dragged` is the width the person chose, if any: it is
    /// kept while the page still has its minimum, where the usual width gives way to the page's comfort.
    static func sidePane(available: CGFloat, minimum: CGFloat, ideal: CGFloat, maximum: CGFloat, dragged: CGFloat? = nil) -> SidePane {
        let room = max(0, available - 1)
        // The pane and its leading divider, with no room left for the page.
        if room - minimum < pageMinimum {
            let width = min(minimum, room)
            return SidePane(width: width, floats: true, pageWidth: available)
        }
        let width: CGFloat
        if let dragged {
            width = min(max(minimum, min(maximum, dragged)), room - pageMinimum)
        } else {
            width = max(minimum, min(ideal, room - pageComfort))
        }
        let rounded = width.rounded(.down)
        return SidePane(width: rounded, floats: false, pageWidth: available - rounded - 1)
    }

    /// The most a person can drag a side pane to in `available` points.
    static func sidePaneLimit(available: CGFloat, minimum: CGFloat, maximum: CGFloat) -> CGFloat {
        max(minimum, min(maximum, available - 1 - pageMinimum))
    }

    /// Whether the scene list and the script page fit side by side.
    static func sceneListFits(available: CGFloat) -> Bool {
        available >= sceneListMinimum + 1 + scriptMinimum
    }

    /// The scene list's width beside the script page, or nil when the two do not fit.
    static func sceneList(available: CGFloat, preferred: CGFloat = sceneListUsual) -> CGFloat? {
        guard sceneListFits(available: available) else { return nil }
        let room = available - 1
        let wanted = min(sceneListMaximum, max(sceneListMinimum, preferred))
        let gives = max(min(wanted, sceneListSnug), min(wanted, room - scriptComfort))
        return min(gives, room - scriptMinimum).rounded(.down)
    }

    /// The most a person can drag the scene list to in `available` points.
    static func sceneListLimit(available: CGFloat) -> CGFloat {
        max(sceneListMinimum, min(sceneListMaximum, available - 1 - scriptMinimum))
    }
}

/// Fills the size it is offered. Content smaller than that is centred; content that will not fit
/// (an overflow no pane should have) starts at the top leading corner and runs off the far side,
/// where a plain frame would centre it and push its start under the pane before it.
struct PaneFill: Layout {
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        proposal.replacingUnspecifiedDimensions()
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let offered = ProposedViewSize(bounds.size)
        for sub in subviews {
            let size = sub.sizeThatFits(offered)
            let x = size.width > bounds.width ? bounds.minX : bounds.midX - size.width / 2
            let y = size.height > bounds.height ? bounds.minY : bounds.midY - size.height / 2
            sub.place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: offered)
        }
    }
}

/// Hands the width this page has (see `pageWidth`) to the views that decide what fits in it. It
/// comes from the window and the panes, never from the page's content.
struct PageWidthReader<Content: View>: View {
    @Environment(\.pageWidth) private var pageWidth
    @ViewBuilder var content: (CGFloat) -> Content

    var body: some View { content(pageWidth) }
}

extension View {
    /// A pane on the trailing side of the page, in place of the system inspector. `name` keeps the
    /// width the person drags it to. The page keeps its place in the view tree when the pane
    /// opens and closes, so a text view on it keeps its text, caret and scroll position.
    func sidePane<Pane: View>(_ name: String, isPresented: Bool, minimum: CGFloat, ideal: CGFloat, maximum: CGFloat,
                              @ViewBuilder pane: @escaping () -> Pane) -> some View {
        modifier(SidePaneHost(name: name, isPresented: isPresented, minimum: minimum, ideal: ideal, maximum: maximum, pane: pane))
    }
}

private struct SidePaneHost<Pane: View>: ViewModifier {
    let isPresented: Bool
    let minimum: CGFloat
    let ideal: CGFloat
    let maximum: CGFloat
    let pane: () -> Pane
    /// The width the person dragged to (0 means the usual width).
    @AppStorage private var dragged: Double
    @State private var dragStart: CGFloat?
    @Environment(\.ofTheme) private var theme

    init(name: String, isPresented: Bool, minimum: CGFloat, ideal: CGFloat, maximum: CGFloat, pane: @escaping () -> Pane) {
        self.isPresented = isPresented
        self.minimum = minimum
        self.ideal = ideal
        self.maximum = maximum
        self.pane = pane
        _dragged = AppStorage(wrappedValue: 0, "straft.sidePane.\(name).width")
    }

    func body(content: Content) -> some View {
        GeometryReader { geo in
            let available = geo.size.width
            let fit = isPresented
                ? PaneFit.sidePane(available: available, minimum: minimum, ideal: ideal, maximum: maximum, dragged: dragged > 0 ? CGFloat(dragged) : nil)
                : nil
            let pageWidth = fit?.pageWidth ?? available
            PaneFill {
                content.environment(\.pageWidth, pageWidth.rounded(.down))
            }
            .frame(width: pageWidth, height: geo.size.height)
            .frame(width: available, height: geo.size.height, alignment: .leading)
            .overlay(alignment: .trailing) {
                if let fit {
                    HStack(spacing: 0) {
                        Divider()
                        pane()
                            .frame(minWidth: 0, maxWidth: .infinity, maxHeight: .infinity)
                    }
                    .frame(width: fit.width + 1, height: geo.size.height)
                    .background(theme["panel"])
                    .shadow(color: fit.floats ? .black.opacity(0.18) : .clear, radius: 8)
                    .overlay(alignment: .leading) { resizeHandle(current: fit.width, available: available) }
                }
            }
        }
    }

    /// Drag to make the pane wider or narrower, double-click for the usual width, or the arrow keys
    /// (24 points a press) when it has the keyboard.
    private func resizeHandle(current: CGFloat, available: CGFloat) -> some View {
        let limit = PaneFit.sidePaneLimit(available: available, minimum: minimum, maximum: maximum)
        let choose: (CGFloat) -> Void = { dragged = Double(min(limit, max(minimum, $0)).rounded()) }
        return Color.clear
            .frame(width: 6)
            .contentShape(Rectangle())
            .onHover { inside in if inside { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() } }
            .gesture(
                DragGesture(minimumDistance: 1)
                    .onChanged { g in
                        let start = dragStart ?? current
                        dragStart = start
                        choose(start - g.translation.width)
                    }
                    .onEnded { _ in dragStart = nil }
            )
            .onTapGesture(count: 2) { dragged = 0 }
            .focusable()
            .focusEffectDisabled()
            .onKeyPress(.leftArrow) { choose(current + 24); return .handled }
            .onKeyPress(.rightArrow) { choose(current - 24); return .handled }
            .help("Drag to resize this panel. Double-click for the usual width.")
            .accessibilityElement()
            .accessibilityLabel("Resize panel")
            .accessibilityValue("\(Int(current)) points")
            .accessibilityAdjustableAction { direction in
                choose(current + (direction == .increment ? 24 : -24))
            }
            .accessibilityIdentifier("side-pane-resize")
    }
}

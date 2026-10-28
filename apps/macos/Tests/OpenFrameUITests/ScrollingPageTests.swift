// AtLeastWidth (ScrollingPage, the Idea Vault canvas) gives its content the page width, or the
// content's own minimum when that is larger, and measures the content at the page width only.
// A second measurement at a width of zero doubled the layout work of every scrolling page.

import AppKit
import SwiftUI
import Testing
@testable import OpenFrameUI

/// Records the widths its content is offered and the sizes it answers.
@MainActor
private final class Offers {
    var widths: [CGFloat?] = []
    var sizes: [CGSize] = []
}

private struct Probe: Layout {
    let offers: Offers

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let size = subviews.first?.sizeThatFits(proposal) ?? .zero
        MainActor.assumeIsolated {
            offers.widths.append(proposal.width)
            offers.sizes.append(size)
        }
        return size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(at: bounds.origin, proposal: ProposedViewSize(bounds.size))
    }
}

/// Lays the view out in a window, as a scroll view that scrolls sideways offers it: no width.
/// Returns the size the view answered.
@MainActor
private func laidOut<V: View>(_ view: V) -> CGSize {
    let outer = Offers()
    let host = NSHostingView(rootView: ScrollView([.vertical, .horizontal]) { Probe(offers: outer) { view } }.frame(width: 900, height: 600))
    host.frame = NSRect(x: 0, y: 0, width: 900, height: 600)
    let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
    window.contentView = host
    host.layoutSubtreeIfNeeded()
    return outer.sizes.last ?? .zero
}

@Suite("ScrollingPageTests", .serialized)
@MainActor
struct ScrollingPageTests {
    @Test func flexibleContentGetsThePageWidthAndIsMeasuredAtItOnly() {
        let offers = Offers()
        let size = laidOut(AtLeastWidth(width: 640) { Probe(offers: offers) { Text("A note about the pier at low tide.").frame(maxWidth: .infinity) } })
        #expect(size.width == 640)
        #expect(!offers.widths.isEmpty)
        #expect(!offers.widths.contains(0), "the content was measured at a width of zero: \(offers.widths)")
    }

    @Test func contentWithALargerMinimumKeepsItsWidth() {
        let size = laidOut(AtLeastWidth(width: 300) { HStack(spacing: 0) { Color.red.frame(width: 260); Color.blue.frame(width: 260) } })
        #expect(size.width == 520)
    }

    @Test func wrappingTextWrapsToThePageWidth() {
        let long = String(repeating: "The night bus waits at the depot. ", count: 20)
        let size = laidOut(AtLeastWidth(width: 320) { Text(long) })
        #expect(size.width == 320)
        #expect(size.height > 40, "the text wraps onto several lines: \(size)")
    }
}

// A page that fills the window when it is big enough and scrolls when it is not. Pages with a
// centred header, a toolbar row and a table need more height than a short window has; without
// this the window's content grew taller than the window and was clipped at the top. A page whose
// rows have a real minimum width (fixed table columns beside a folder list) scrolls sideways in a
// narrow window instead of being cut at both edges; text still wraps to the window's width.

import SwiftUI
import Foundation

struct ScrollingPage<Content: View>: View {
    @ViewBuilder var content: () -> Content

    var body: some View {
        GeometryReader { geo in
            ScrollView([.vertical, .horizontal]) {
                AtLeastWidth(width: geo.size.width) { content() }
                    .frame(minHeight: geo.size.height, alignment: .top)
            }
            .scrollBounceBehavior(.basedOnSize)
        }
    }
}

/// Gives its content the given width, or the content's own minimum width when that is larger.
/// Inside a scroll view that scrolls sideways the proposal has no width, so without this every
/// line of text would ask for its full length on one line.
///
/// The content is measured at the given width only. A view offered less than its minimum answers
/// with its minimum, so that one measurement also tells when the page must scroll sideways. An
/// extra measurement at a width of zero (asking for the minimum directly) made an empty page,
/// whose ContentUnavailableView sits in a ViewThatFits, take tens of seconds to lay out.
struct AtLeastWidth: Layout {
    var width: CGFloat

    private func measure(_ sub: LayoutSubview, height: CGFloat?) -> CGSize {
        let size = sub.sizeThatFits(ProposedViewSize(width: width, height: height))
        guard size.width.isFinite, size.width.rounded(.up) > width else { return CGSize(width: width, height: size.height) }
        let wider = size.width.rounded(.up)
        return CGSize(width: wider, height: sub.sizeThatFits(ProposedViewSize(width: wider, height: height)).height)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard let sub = subviews.first else { return .zero }
        return measure(sub, height: proposal.height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        guard let sub = subviews.first else { return }
        sub.place(at: bounds.origin, anchor: .topLeading, proposal: ProposedViewSize(width: bounds.width, height: bounds.height))
    }
}

/// Items wrapped into rows, each row centred on its own, so a single card, a short last row or a
/// bar of filters sits in the middle instead of against the leading edge (a lazy adaptive grid
/// pins every row to the leading edge). With `itemWidth` every item gets that width (cards);
/// without it every item keeps its own natural size (filters, chips, buttons).
struct CenteredWrap: Layout {
    var itemWidth: CGFloat?
    var spacing: CGFloat = PageSpacing.medium
    var lineSpacing: CGFloat?

    init(itemWidth: CGFloat? = nil, spacing: CGFloat = PageSpacing.medium, lineSpacing: CGFloat? = nil) {
        self.itemWidth = itemWidth
        self.spacing = spacing
        self.lineSpacing = lineSpacing
    }

    private struct Row { var items: [(index: Int, size: CGSize)] = []; var width: CGFloat = 0; var height: CGFloat = 0 }

    /// The size of one item. An item wider than the whole row is measured again at the row's width,
    /// so a long line of text wraps instead of pushing the row past the window.
    private func size(_ s: LayoutSubview, limit: CGFloat) -> CGSize {
        if let itemWidth { return s.sizeThatFits(ProposedViewSize(width: itemWidth, height: nil)) }
        let ideal = s.sizeThatFits(.unspecified)
        guard limit.isFinite, ideal.width > limit else { return ideal }
        return s.sizeThatFits(ProposedViewSize(width: limit, height: nil))
    }

    private func rows(_ subviews: Subviews, width: CGFloat) -> [Row] {
        var out: [Row] = []
        var row = Row()
        for (i, s) in subviews.enumerated() {
            let sz = size(s, limit: width)
            let w = itemWidth ?? sz.width
            let needed = row.items.isEmpty ? w : row.width + spacing + w
            if needed > width, !row.items.isEmpty {
                out.append(row)
                row = Row()
            }
            row.width = row.items.isEmpty ? w : row.width + spacing + w
            row.height = max(row.height, sz.height)
            row.items.append((i, CGSize(width: w, height: sz.height)))
        }
        if !row.items.isEmpty { out.append(row) }
        return out
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        // SwiftUI asks for the smallest size with a width near zero. Answering with every item on
        // its own line would make the page ask for far more height than it needs, so a width that
        // small gets the height of one line.
        if width < 1 {
            return CGSize(width: 0, height: subviews.map { $0.sizeThatFits(.unspecified).height }.max() ?? 0)
        }
        let rs = rows(subviews, width: width)
        let gap = lineSpacing ?? spacing
        let h = rs.map(\.height).reduce(0, +) + CGFloat(max(0, rs.count - 1)) * gap
        let w = rs.map(\.width).max() ?? 0
        return CGSize(width: proposal.width.map { $0.isFinite ? $0 : w } ?? w, height: h)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var y = bounds.minY
        let gap = lineSpacing ?? spacing
        for row in rows(subviews, width: bounds.width) {
            var x = bounds.midX - row.width / 2
            for item in row.items {
                subviews[item.index].place(at: CGPoint(x: x, y: y + (row.height - item.size.height) / 2), anchor: .topLeading,
                                           proposal: ProposedViewSize(width: item.size.width, height: item.size.height))
                x += item.size.width + spacing
            }
            y += row.height + gap
        }
    }
}

extension View {
    /// Gives the screen exactly the size the window offers. A scroll view or a list inside a screen
    /// can ask for the height of all its content, and the window then grows past the screen and
    /// clips the top of the page. The Schedule has always done this for the same reason.
    func fitsWindow() -> some View {
        GeometryReader { geo in
            self.frame(width: geo.size.width, height: geo.size.height)
        }
    }
}

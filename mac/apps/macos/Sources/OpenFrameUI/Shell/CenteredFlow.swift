// Fixed-width cards in rows that wrap and centre as a whole: one card sits in the middle, a full
// row has equal space on both sides, and a short last row is centred too. For short lists of
// cards (projects, templates, themes, counts); a long gallery keeps a lazy grid.

import SwiftUI

struct CenteredFlow: Layout {
    var cardWidth: CGFloat
    var spacing: CGFloat = PageSpacing.medium

    private func perRow(_ width: CGFloat) -> Int {
        guard width.isFinite, width > 0 else { return 1 }
        return max(1, Int(((width + spacing) / (cardWidth + spacing)).rounded(.down)))
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? (cardWidth * CGFloat(max(1, subviews.count)) + spacing * CGFloat(max(0, subviews.count - 1)))
        let n = perRow(width)
        var y: CGFloat = 0
        var index = 0
        var firstRow = true
        while index < subviews.count {
            let row = subviews[index..<min(index + n, subviews.count)]
            let h = row.map { $0.sizeThatFits(ProposedViewSize(width: cardWidth, height: nil)).height }.max() ?? 0
            y += (firstRow ? 0 : spacing) + h
            firstRow = false
            index += n
        }
        return CGSize(width: width, height: y)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let n = perRow(bounds.width)
        var y = bounds.minY
        var index = 0
        while index < subviews.count {
            let end = min(index + n, subviews.count)
            let row = subviews[index..<end]
            let h = row.map { $0.sizeThatFits(ProposedViewSize(width: cardWidth, height: nil)).height }.max() ?? 0
            let rowWidth = CGFloat(row.count) * cardWidth + CGFloat(row.count - 1) * spacing
            var x = bounds.minX + (bounds.width - rowWidth) / 2
            for sub in row {
                sub.place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: ProposedViewSize(width: cardWidth, height: h))
                x += cardWidth + spacing
            }
            y += h + spacing
            index = end
        }
    }
}

/// A short group of cards centred as a whole (see `CenteredFlow`).
struct CenteredCards<Content: View>: View {
    var cardWidth: CGFloat = 220
    var spacing: CGFloat = PageSpacing.medium
    @ViewBuilder var content: Content

    var body: some View {
        CenteredFlow(cardWidth: cardWidth, spacing: spacing) { content }
            .frame(maxWidth: .infinity)
    }
}

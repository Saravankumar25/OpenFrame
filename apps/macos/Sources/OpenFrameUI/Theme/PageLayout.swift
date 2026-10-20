// One page layout for every screen, so headings, columns and margins line up the same way
// everywhere (owner request of 2026-10-18: titles centred, layouts symmetric).
//
// Rules:
//   - Page content sits in a centred column with equal margins on both sides
//     (`.pageColumn()`); a wide workspace may pass a larger width, never an offset.
//   - A page title and its subtitle are centred over that column (`PageHeader`).
//   - Section headings (side headings) start at the column's leading edge, all at the same
//     inset, with the same space above and below (`SectionHeader`).
//   - Spacing comes from `PageSpacing` (an 8-point grid), not from one-off numbers.
//   - Groups of cards are centred as a group and wrap evenly (`CenteredCardGrid`).

import SwiftUI

public enum PageSpacing {
    /// Inside a control or between a label and its value.
    public static let tight: CGFloat = 4
    /// Between related rows.
    public static let small: CGFloat = 8
    /// Between groups inside a section.
    public static let medium: CGFloat = 16
    /// Between sections.
    public static let large: CGFloat = 24
    /// Above the page title, and between the header and the first section.
    public static let page: CGFloat = 32

    /// Width of the reading column on content pages.
    public static let column: CGFloat = 760
    /// Width of the column on wide workspace pages (tables, boards).
    public static let wideColumn: CGFloat = 1120
}

extension View {
    /// Centres the page content in a column of at most `maxWidth` points with equal margins
    /// on both sides, and at least `PageSpacing.large` from each window edge.
    public func pageColumn(maxWidth: CGFloat = PageSpacing.column) -> some View {
        frame(maxWidth: maxWidth, alignment: .top)
            .padding(.horizontal, PageSpacing.large)
            .frame(maxWidth: .infinity, alignment: .top)
    }
}

/// The title of a page, centred over the page column, with an optional subtitle and an
/// optional row of actions centred below it.
public struct PageHeader<Accessory: View>: View {
    let title: String
    let subtitle: String?
    let accessory: Accessory
    @Environment(\.ofTheme) private var theme

    public init(_ title: String, subtitle: String? = nil, @ViewBuilder accessory: () -> Accessory) {
        self.title = title
        self.subtitle = subtitle
        self.accessory = accessory()
    }

    /// As on Windows (design-system PageHeader): the title and its line on the left, the page's
    /// actions on the right; in a narrow page the actions go under the title instead of off the edge.
    public var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .top, spacing: PageSpacing.medium) {
                titleBlock.fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: PageSpacing.medium)
                actions.fixedSize()
            }
            VStack(alignment: .leading, spacing: PageSpacing.small) {
                titleBlock
                actions
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.top, PageSpacing.page)
        .padding(.bottom, PageSpacing.medium)
    }

    private var titleBlock: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.system(size: 22, weight: .bold))
                .tracking(-0.2)
                .accessibilityAddTraits(.isHeader)
            if let subtitle, !subtitle.isEmpty {
                Text(subtitle)
                    .font(.callout)
                    .foregroundStyle(theme["muted"])
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private var actions: some View {
        HStack(spacing: PageSpacing.small) { accessory }
    }
}

extension PageHeader where Accessory == EmptyView {
    public init(_ title: String, subtitle: String? = nil) {
        self.init(title, subtitle: subtitle) { EmptyView() }
    }
}

/// A side heading inside a page: leading edge of the column, same size and spacing everywhere.
public struct SectionHeader<Trailing: View>: View {
    let title: String
    let trailing: Trailing

    public init(_ title: String, @ViewBuilder trailing: () -> Trailing) {
        self.title = title
        self.trailing = trailing()
    }

    public var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: PageSpacing.small) {
            // Small grey capitals, as the Windows section titles (".h4").
            Text(title)
                .textCase(.uppercase)
                .font(.caption.weight(.bold))
                .tracking(0.7)
                .foregroundStyle(.secondary)
                // VoiceOver reads the title as written, not letter by letter in capitals.
                .accessibilityLabel(title)
                .accessibilityAddTraits(.isHeader)
            Spacer(minLength: PageSpacing.small)
            trailing
        }
        .padding(.top, PageSpacing.large)
        .padding(.bottom, PageSpacing.small)
    }
}

extension SectionHeader where Trailing == EmptyView {
    public init(_ title: String) {
        self.init(title) { EmptyView() }
    }
}

/// Cards of one width laid out in rows that are centred as a whole, so one card sits in the
/// middle and a full row has equal space on both sides.
public struct CenteredCardGrid<Content: View>: View {
    let cardWidth: CGFloat
    let spacing: CGFloat
    let content: Content

    public init(cardWidth: CGFloat = 220, spacing: CGFloat = PageSpacing.medium, @ViewBuilder content: () -> Content) {
        self.cardWidth = cardWidth
        self.spacing = spacing
        self.content = content()
    }

    public var body: some View {
        // A wrapping layout, not an adaptive grid: a grid pins a lone card or a short last row
        // to the leading edge.
        CenteredWrap(itemWidth: cardWidth, spacing: spacing) {
            content
        }
        .frame(maxWidth: .infinity)
    }
}

// Small views shared by the import, export and package sheets: chips, banners, the sheet frame,
// a facts row and a plain table header. Colours come from the theme tokens.

import SwiftUI

struct IXChip: View {
    let text: String
    var tone: IXTone = .plain
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let tokens: (String, String) = switch tone {
        case .plain: ("surface2", "ink2")
        case .green: ("green-soft", "green-t")
        case .yellow: ("yellow-soft", "yellow-t")
        case .red: ("red-soft", "red-t")
        case .blue: ("blue-soft", "blue-t")
        case .purple: ("purple-soft", "purple-t")
        case .dark: ("ink", "panel")
        }
        Text(text)
            .font(.caption.weight(.medium))
            .lineLimit(1)
            .foregroundStyle(theme[tokens.1])
            .padding(.horizontal, 7)
            .padding(.vertical, 2)
            .background(theme[tokens.0], in: Capsule())
            .fixedSize()
    }
}

enum IXBannerTone { case info, warn, err, ok, priv }

struct IXBanner<Actions: View>: View {
    let tone: IXBannerTone
    let text: Text
    @ViewBuilder var actions: () -> Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let (bg, fg, symbol): (String, String, String) = switch tone {
        case .info: ("blue-soft", "blue-t", "info.circle")
        case .warn: ("yellow-soft", "yellow-t", "exclamationmark.triangle")
        case .err: ("red-soft", "red-t", "xmark.octagon")
        case .ok: ("green-soft", "green-t", "checkmark.circle")
        case .priv: ("purple-soft", "purple-t", "lock")
        }
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: symbol).accessibilityHidden(true)
            text.fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 8)
            actions()
        }
        .font(.callout)
        .foregroundStyle(theme[fg])
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme[bg], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        // A banner with buttons keeps them as separate controls; a plain banner reads as one line.
        .accessibilityElement(children: Actions.self == EmptyView.self ? .combine : .contain)
    }
}

extension IXBanner where Actions == EmptyView {
    init(_ tone: IXBannerTone, _ text: String) {
        self.init(tone: tone, text: Text(text), actions: { EmptyView() })
    }

    init(tone: IXBannerTone, text: Text) {
        self.init(tone: tone, text: text, actions: { EmptyView() })
    }

    init(_ tone: IXBannerTone, text: Text) {
        self.init(tone: tone, text: text, actions: { EmptyView() })
    }
}

/// The frame of every sheet: a title, an optional line under it, the content, and a row of buttons.
struct IXSheetFrame<Content: View, Footer: View, Leading: View>: View {
    let title: String
    var sub: String?
    var width: CGFloat = 560
    var idPrefix: String
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @ViewBuilder var leading: () -> Leading
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("\(idPrefix)-title")
                if let sub {
                    Text(sub).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                }
            }
            content()
            HStack(spacing: 10) {
                leading()
                Spacer(minLength: 12)
                footer()
            }
        }
        .padding(22)
        .frame(width: width)
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(idPrefix)
    }
}

extension IXSheetFrame where Leading == EmptyView {
    init(title: String, sub: String? = nil, width: CGFloat = 560, idPrefix: String,
         @ViewBuilder content: @escaping () -> Content, @ViewBuilder footer: @escaping () -> Footer) {
        self.init(title: title, sub: sub, width: width, idPrefix: idPrefix, content: content, footer: footer, leading: { EmptyView() })
    }
}

/// A label above a value on a card.
struct IXFact: View {
    let label: String
    let value: String
    var detail: String?
    var id: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Text(value).font(.body.weight(.semibold)).lineLimit(2).help(value)
            if let detail { Text(detail).font(.caption).foregroundStyle(.secondary).lineLimit(1) }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier(id)
    }
}

/// A two-column list of "label: value" lines on a card (the Windows report tables).
struct IXKeyValueTable: View {
    let rows: [(label: String, value: String)]
    var id: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { i, row in
                HStack(alignment: .firstTextBaseline) {
                    Text(row.label).foregroundStyle(.secondary).frame(width: 170, alignment: .leading)
                    Text(row.value).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
                }
                .font(.callout)
                .padding(.horizontal, 10).padding(.vertical, 6)
                .accessibilityElement(children: .combine)
                if i < rows.count - 1 { Divider() }
            }
        }
        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(id)
    }
}

// The Straft Studio logo: a film-frame outline on a 24 by 24 grid, stroke only, in a diagonal gradient
// whose three stops come from the active theme (owner mocks, design note section 5), so choosing a
// theme recolours it with no per-theme logo data.

import SwiftUI

/// The mark alone: the rectangle, two vertical lines, the middle line and four short ticks.
struct BrandMark: View {
    var size: CGFloat = 32
    @Environment(\.ofTheme) private var theme

    var body: some View {
        FilmFrameShape()
            .stroke(theme.logoGradient, style: StrokeStyle(lineWidth: size * 1.6 / 24, lineCap: .round, lineJoin: .round))
            .frame(width: size, height: size)
            .accessibilityHidden(true)
    }
}

/// The mark and the wordmark: "Open" thin and "Frame" bold, in the theme's text colour. One word, no
/// second line.
struct BrandLogo: View {
    var markSize: CGFloat = 28
    var wordSize: CGFloat = 19
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: markSize * 9 / 28) {
            BrandMark(size: markSize)
            HStack(spacing: 0) {
                Text("Open").fontWeight(.light)
                Text("Frame").fontWeight(.bold)
            }
            .font(.system(size: wordSize))
            .tracking(-0.02 * wordSize)
            .foregroundStyle(theme["ink"])
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Straft Studio")
    }
}

/// The film frame on its 24 by 24 grid: outer rectangle x=2 y=2 w=20 h=20 corner 2.2, verticals at 7
/// and 17, a horizontal at 12, and ticks at 7 and 17 on both sides.
struct FilmFrameShape: Shape {
    func path(in rect: CGRect) -> Path {
        let unit = min(rect.width, rect.height) / 24
        func at(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: rect.minX + x * unit, y: rect.minY + y * unit) }
        var p = Path()
        p.addRoundedRect(
            in: CGRect(origin: at(2, 2), size: CGSize(width: 20 * unit, height: 20 * unit)),
            cornerSize: CGSize(width: 2.2 * unit, height: 2.2 * unit)
        )
        let lines: [(CGFloat, CGFloat, CGFloat, CGFloat)] = [
            (7, 2, 7, 22), (17, 2, 17, 22), (2, 12, 22, 12),
            (2, 7, 7, 7), (2, 17, 7, 17), (22, 7, 17, 7), (22, 17, 17, 17),
        ]
        for (x1, y1, x2, y2) in lines {
            p.move(to: at(x1, y1))
            p.addLine(to: at(x2, y2))
        }
        return p
    }
}

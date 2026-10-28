// The OpenFrame mark, shown on Home, About, the library setup and the store screens.

import SwiftUI

/// The OpenFrame mark in the theme's logo colours (the logo follows the theme).
struct BrandMark: View {
    var size: CGFloat = 32
    @Environment(\.ofTheme) private var theme

    var body: some View {
        RoundedRectangle(cornerRadius: size * 0.24, style: .continuous)
            .fill(theme.logoGradient)
            .frame(width: size, height: size)
            .overlay {
                Image(systemName: "film")
                    .font(.system(size: size * 0.46, weight: .semibold))
                    .foregroundStyle(.white)
            }
            .accessibilityHidden(true)
    }
}

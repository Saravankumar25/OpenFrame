// The title page as the PDF lays it out (Mac phase M4): the core gives every line its position in
// inches, and this view draws them at true proportions on a sheet of paper. Nothing is worked out
// here, so the preview, the print and the PDF match.

import SwiftUI
import OpenFrameCore

struct TitlePagePreview: View {
    let preview: InterchangePrintPreview?
    var failed = false
    @Environment(\.ofTheme) private var theme

    /// Courier ascent per point of size, so a line's baseline lands on its layout position.
    private static let ascentPerPoint = 0.125 / 12

    var body: some View {
        VStack(spacing: 6) {
            if let preview, let page = preview.pages.first {
                GeometryReader { geo in
                    let scale = min(geo.size.width / preview.pageWidthIn, geo.size.height / preview.pageHeightIn)
                    let width = preview.pageWidthIn * scale, height = preview.pageHeightIn * scale
                    Canvas { context, _ in
                        let points = preview.fontSizePt / 72 * scale
                        for line in page.lines where !line.text.isEmpty {
                            var text = AttributedString(line.text)
                            text.font = .system(size: points, design: .monospaced)
                                .weight(line.style == "title_main" ? .bold : .regular)
                            if line.style == "title_main" { text.underlineStyle = .single }
                            text.foregroundColor = .black
                            let y = (line.yIn - Self.ascentPerPoint * preview.fontSizePt) * scale
                            context.draw(Text(text), at: CGPoint(x: line.xIn * scale, y: y), anchor: .topLeading)
                        }
                    }
                    .frame(width: width, height: height)
                    .background(Color.white.shadow(.drop(color: .black.opacity(0.25), radius: 4, y: 1)))
                    .frame(width: geo.size.width, height: geo.size.height, alignment: .top)
                }
                .aspectRatio(preview.pageWidthIn / preview.pageHeightIn, contentMode: .fit)
                .accessibilityElement()
                .accessibilityLabel("Title page preview")
                .accessibilityValue(page.lines.map(\.text).filter { !$0.isEmpty }.joined(separator: ", "))
                Text("\(preview.paper)").font(.caption).foregroundStyle(theme["muted"])
            } else if failed {
                Text("The preview is not available.").font(.callout).foregroundStyle(theme["muted"])
            } else {
                ProgressView().controlSize(.small)
            }
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .accessibilityIdentifier("title-page-preview")
    }
}

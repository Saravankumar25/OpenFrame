// The two button looks of the Mac app. The theme accent is the window tint (it colours switches,
// segmented pickers, selection and the prominent button), but a tinted ordinary button would draw
// its label in the accent too, so every plain push button in the app turned orange. Ordinary
// buttons therefore keep a neutral tint, and the prominent button draws its label in `accent-ink`
// so amber stays readable (white on amber fails the contrast check).
import SwiftUI

/// The default push button: bordered and neutral. Set once for every window by `openFrameTheme`.
public struct StandardButtonStyle: PrimitiveButtonStyle {
    public func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) { configuration.label }
            .buttonStyle(.bordered)
            .tint(.primary)
    }
}

/// The one main action of a screen or sheet: accent fill with dark text.
public struct ProminentButtonStyle: PrimitiveButtonStyle {
    @Environment(\.ofTheme) private var theme

    public func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) {
            configuration.label.foregroundStyle(theme["accent-ink"])
        }
        .buttonStyle(.borderedProminent)
        .tint(theme["accent"])
    }
}

extension PrimitiveButtonStyle where Self == StandardButtonStyle {
    public static var ofStandard: StandardButtonStyle { StandardButtonStyle() }
}

extension PrimitiveButtonStyle where Self == ProminentButtonStyle {
    public static var ofProminent: ProminentButtonStyle { ProminentButtonStyle() }
}

extension View {
    /// A sheet whose content keeps the neutral push buttons. A sheet inherits the window tint but
    /// not the button style set by `openFrameTheme`, so without this its buttons drew orange labels.
    public func themedSheet<Content: View>(
        isPresented: Binding<Bool>, onDismiss: (() -> Void)? = nil,
        @ViewBuilder content: @escaping () -> Content
    ) -> some View {
        sheet(isPresented: isPresented, onDismiss: onDismiss) { content().buttonStyle(.ofStandard) }
    }

    public func themedSheet<Item: Identifiable, Content: View>(
        item: Binding<Item?>, onDismiss: (() -> Void)? = nil,
        @ViewBuilder content: @escaping (Item) -> Content
    ) -> some View {
        sheet(item: item, onDismiss: onDismiss) { content($0).buttonStyle(.ofStandard) }
    }
}

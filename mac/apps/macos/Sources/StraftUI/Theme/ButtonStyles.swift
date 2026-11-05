// The two button looks of the Mac app. The theme accent is the window tint (it colours switches,
// segmented pickers, selection and the prominent button), but a tinted ordinary button would draw
// its label in the accent too, so every plain push button in the app turned orange. Ordinary
// buttons therefore keep a neutral tint, and the prominent button draws its label in `accent-ink`
// so amber stays readable (white on amber fails the contrast check).
import SwiftUI

/// The default push button: bordered and neutral. Set once for every window by `straftTheme`.
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
    @Environment(\.controlActiveState) private var active

    public func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) {
            // An inactive window draws the button without its accent fill, where the dark label
            // would not be readable.
            configuration.label.foregroundStyle(active == .inactive ? Color.primary : theme["accent-ink"])
        }
        .buttonStyle(.borderedProminent)
        .tint(theme["accent"])
    }
}

/// A button with no border: its label in the strong accent, which reads on every theme. The window
/// tint is a fill colour and does not.
public struct BorderlessAccentButtonStyle: PrimitiveButtonStyle {
    @Environment(\.ofTheme) private var theme

    public func makeBody(configuration: Configuration) -> some View {
        Button(role: configuration.role, action: configuration.trigger) { configuration.label }
            .buttonStyle(.borderless)
            .tint(theme["accent-d"])
    }
}

/// A menu with no border, drawn like `BorderlessAccentButtonStyle`.
private struct BorderlessAccentMenu: ViewModifier {
    @Environment(\.ofTheme) private var theme

    func body(content: Content) -> some View {
        content.menuStyle(.borderlessButton).tint(theme["accent-d"])
    }
}

/// A plain text field whose hint is drawn in the theme's muted colour: macOS draws a field's own
/// prompt in its fixed placeholder grey, whatever colour it is given, and that grey falls under 4.5 to 1.
public struct HintedTextField: View {
    let hint: String
    @Binding var text: String
    @Environment(\.ofTheme) private var theme

    public init(_ hint: String, text: Binding<String>) {
        self.hint = hint
        _text = text
    }

    public var body: some View {
        TextField(hint, text: $text, prompt: Text(""))
            .overlay(alignment: .leading) {
                if text.isEmpty {
                    Text(hint).foregroundStyle(theme["muted"]).lineLimit(1).allowsHitTesting(false).accessibilityHidden(true)
                }
            }
    }
}

extension View {
    public func borderlessAccentMenu() -> some View { modifier(BorderlessAccentMenu()) }
}

extension PrimitiveButtonStyle where Self == BorderlessAccentButtonStyle {
    public static var ofBorderless: BorderlessAccentButtonStyle { BorderlessAccentButtonStyle() }
}

extension PrimitiveButtonStyle where Self == StandardButtonStyle {
    public static var ofStandard: StandardButtonStyle { StandardButtonStyle() }
}

extension PrimitiveButtonStyle where Self == ProminentButtonStyle {
    public static var ofProminent: ProminentButtonStyle { ProminentButtonStyle() }
}

extension View {
    /// A sheet whose content keeps the neutral push buttons. A sheet inherits the window tint but
    /// not the button style set by `straftTheme`, so without this its buttons drew orange labels.
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

// Settings (Command+Comma): General, Appearance, Offline AI and Diagnostics. Every value and every
// rule comes from the core; these screens only show it and forward the user's choice.

import AppKit
import OpenFrameCore
import SwiftUI

public enum SettingsTab: String, Hashable, CaseIterable {
    case general, appearance, shortcuts, ai, diagnostics
}

/// Opens the Settings window (Command+Comma), on a tab when one is asked for. Every Settings
/// button, menu item and palette command goes through here.
@MainActor
public enum SettingsRouter {
    /// The tab the window shows next time it appears or is already open.
    public static var requestedTab: SettingsTab?
    /// SwiftUI's own action for the Settings window, taken from the main window (`settingsOpener`).
    /// Recent macOS ignores the old `showSettingsWindow:` message, so the Settings buttons did
    /// nothing while the menu item and Command+Comma still worked.
    static var openAction: OpenSettingsAction?

    public static func open(tab: SettingsTab? = nil) {
        requestedTab = tab
        if let tab { NotificationCenter.default.post(name: .openFrameSettingsTab, object: tab.rawValue) }
        if let openAction {
            openAction()
        } else {
            NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
        }
    }
}

private struct SettingsOpener: ViewModifier {
    @Environment(\.openSettings) private var openSettings
    func body(content: Content) -> some View {
        content.onAppear { SettingsRouter.openAction = openSettings }
    }
}

extension View {
    /// Lets `SettingsRouter.open` open the Settings window from anywhere in the app.
    func settingsOpener() -> some View { modifier(SettingsOpener()) }
}

extension Notification.Name {
    static let openFrameSettingsTab = Notification.Name("studio.openframe.settingsTab")
}

public struct SettingsView: View {
    @State private var tab: SettingsTab

    public init(initialTab: SettingsTab = .general) {
        _tab = State(initialValue: initialTab)
    }

    public var body: some View {
        TabView(selection: $tab) {
            GeneralSettingsView()
                .tabItem { Label("General", systemImage: "gearshape") }
                .tag(SettingsTab.general)
            AppearanceSettingsView()
                .tabItem { Label("Appearance", systemImage: "paintpalette") }
                .tag(SettingsTab.appearance)
            ShortcutsSettingsTab()
                .tabItem { Label("Keyboard Shortcuts", systemImage: "keyboard") }
                .tag(SettingsTab.shortcuts)
            AISettingsView()
                .tabItem { Label("Offline AI", systemImage: "sparkles") }
                .tag(SettingsTab.ai)
            DiagnosticsSettingsView()
                .tabItem { Label("Diagnostics", systemImage: "stethoscope") }
                .tag(SettingsTab.diagnostics)
        }
        .frame(width: 760)
        .frame(minHeight: 460)
        .onAppear {
            if let asked = SettingsRouter.requestedTab { tab = asked; SettingsRouter.requestedTab = nil }
        }
        .onReceive(NotificationCenter.default.publisher(for: .openFrameSettingsTab)) { note in
            if let raw = note.object as? String, let asked = SettingsTab(rawValue: raw) { tab = asked }
        }
    }
}

/// A line of red text for an error from the core, with a stable identifier for tests.
struct SettingsErrorText: View {
    let message: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Label(message, systemImage: "exclamationmark.triangle")
            .font(.callout)
            .foregroundStyle(theme["red-t"])
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityIdentifier("settings-error")
    }
}

func formattedBytes(_ bytes: Double) -> String {
    ByteCountFormatter.string(fromByteCount: Int64(bytes), countStyle: .file)
}

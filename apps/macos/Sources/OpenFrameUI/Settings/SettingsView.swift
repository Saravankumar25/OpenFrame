// Settings (Command+Comma): General, Appearance, Offline AI and Diagnostics. Every value and every
// rule comes from the core; these screens only show it and forward the user's choice.

import AppKit
import OpenFrameCore
import SwiftUI

public enum SettingsTab: String, Hashable, CaseIterable {
    case general, appearance, ai, diagnostics
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
            AISettingsView()
                .tabItem { Label("Offline AI", systemImage: "sparkles") }
                .tag(SettingsTab.ai)
            DiagnosticsSettingsView()
                .tabItem { Label("Diagnostics", systemImage: "stethoscope") }
                .tag(SettingsTab.diagnostics)
        }
        .frame(width: 640)
        .frame(minHeight: 460)
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

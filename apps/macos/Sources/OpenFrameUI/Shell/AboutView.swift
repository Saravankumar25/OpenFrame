// About OpenFrame Studio (Windows: workspaces/settings/About.tsx, scenario 16-about): the app
// version, the core version, the build channel and the licence notice. An internal team build says
// plainly that it is not for sale. There is no price and no Buy button anywhere in it.

import SwiftUI
import OpenFrameCore

/// What the About window shows, as plain values (so a test can check the words).
struct AboutInfo: Equatable {
    var appVersion: String
    var coreVersion: String
    var channelName: String
    /// Set on the internal team build only.
    var notice: String?

    /// The words for internal team builds, the same as the Licence card on Windows.
    static let internalBuildNotice = "Internal build, not for sale."

    static func make(appVersion: String, coreVersion: String = Core.version, channel: Channel?) -> AboutInfo {
        switch channel {
        case .directInstaller?:
            return AboutInfo(appVersion: appVersion, coreVersion: coreVersion, channelName: "Internal team build", notice: internalBuildNotice)
        case .macAppStore?:
            return AboutInfo(appVersion: appVersion, coreVersion: coreVersion, channelName: "Mac App Store", notice: nil)
        case .microsoftStore?:
            return AboutInfo(appVersion: appVersion, coreVersion: coreVersion, channelName: "Microsoft Store", notice: nil)
        case nil:
            return AboutInfo(appVersion: appVersion, coreVersion: coreVersion, channelName: "", notice: nil)
        }
    }

    @MainActor
    static func load(app: AppModel) async -> AboutInfo {
        let version = app.info?.appVersion ?? AppVersion.current
        let channel = try? await app.core?.entitlementStatus().channel
        return make(appVersion: version, channel: channel)
    }
}

struct AboutView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var about: AboutInfo?

    init(preset: AboutInfo? = nil) { _about = State(initialValue: preset) }

    var body: some View {
        VStack(spacing: 10) {
            BrandMark(size: 64)
            Text("OpenFrame Studio")
                .font(.system(size: 22, weight: .semibold))
            if let about {
                Text("Version \(about.appVersion)")
                    .accessibilityIdentifier("about-version")
                Text("Core \(about.coreVersion)")
                    .font(.callout).foregroundStyle(.secondary)
                    .accessibilityIdentifier("about-core")
                if !about.channelName.isEmpty {
                    Text(about.channelName)
                        .font(.callout).foregroundStyle(.secondary)
                        .accessibilityIdentifier("about-channel")
                }
                if let notice = about.notice {
                    Text(notice)
                        .font(.callout.weight(.semibold))
                        .padding(.top, 6)
                        .accessibilityIdentifier("about-notice")
                }
            } else {
                ProgressView().controlSize(.small)
            }
            Text("Your work is stored on this Mac.")
                .font(.caption)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.top, 8)
        }
        .padding(28)
        .frame(width: 340)
        .background(theme["bg"])
        .task { about = await AboutInfo.load(app: app) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("about-window")
    }
}

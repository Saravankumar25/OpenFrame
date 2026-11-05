// About Straft Studio (Windows: workspaces/settings/About.tsx, scenario 16-about): the app
// version, the core version, the build channel, the publisher and the licence notice. An internal
// team build says plainly that it is not for sale. There is no price and no Buy button anywhere in it.

import AppKit
import SwiftUI
import StraftCore

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

/// The notices the app ships with: the publisher, the third-party licence list and the AI engine and
/// model notices (Gemma terms and its prohibited use notice, llama.cpp).
enum LicenceNotices {
    static let publisher = """
    Straft Studio
    Copyright 2026 Vighnajith Advisories Private Limited
    12/1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India
    """

    /// The files that travel in the app bundle, as they stand in the given folders.
    static func text(resources: URL?, engine: URL?) -> String {
        var parts = [publisher]
        if let list = resources?.appendingPathComponent("THIRD_PARTY_LICENSES.md"),
           let body = try? String(contentsOf: list, encoding: .utf8) {
            parts.append(body)
        }
        let notices = engine?.appendingPathComponent("notices", isDirectory: true)
        let names = notices.flatMap { try? FileManager.default.contentsOfDirectory(at: $0, includingPropertiesForKeys: nil) } ?? []
        for file in names.sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
            if let body = try? String(contentsOf: file, encoding: .utf8) {
                parts.append("\(file.lastPathComponent)\n\(body)")
            }
        }
        return parts.joined(separator: "\n\n")
    }

    static func forThisApp() -> String {
        let helpers = Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/ai", isDirectory: true)
        let engine = FileManager.default.fileExists(atPath: helpers.path)
            ? helpers : Bundle.main.resourceURL?.appendingPathComponent("ai", isDirectory: true)
        return text(resources: Bundle.main.resourceURL, engine: engine)
    }
}

private struct LicencesSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Licences").font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
            ScrollView {
                Text(LicenceNotices.forThisApp())
                    .font(.caption.monospaced())
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .accessibilityIdentifier("about-licences-text")
            HStack {
                Spacer()
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction).accessibilityIdentifier("about-licences-done")
            }
        }
        .padding(20)
        .frame(width: 560, height: 480)
        .background(theme["bg"])
    }
}

struct AboutView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var about: AboutInfo?
    @State private var showLicences = false

    init(preset: AboutInfo? = nil) { _about = State(initialValue: preset) }

    var body: some View {
        VStack(spacing: 10) {
            BrandLogo(markSize: 40, wordSize: 28)
            Text("Straft Studio")
                .font(.system(size: 15, weight: .semibold))
            if let about {
                Text("Version \(about.appVersion)")
                    .accessibilityIdentifier("about-version")
                Text("Core \(about.coreVersion)")
                    .font(.callout).foregroundStyle(.ofMuted)
                    .accessibilityIdentifier("about-core")
                if !about.channelName.isEmpty {
                    Text(about.channelName)
                        .font(.callout).foregroundStyle(.ofMuted)
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
                .foregroundStyle(.ofMuted)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.top, 8)
            Text("Copyright 2026 Vighnajith Advisories Private Limited")
                .font(.caption2)
                .foregroundStyle(.ofMuted)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("about-publisher")
            HStack(spacing: 14) {
                Button("Contact us") {
                    if let url = URL(string: LicenceRules.contactAddress) { NSWorkspace.shared.open(url) }
                }
                .accessibilityHint("Opens the contact page in your browser")
                .accessibilityIdentifier("about-contact")
                Button("Licences") { showLicences = true }
                    .accessibilityIdentifier("about-licences")
            }
            .buttonStyle(.link)
            .controlSize(.small)
        }
        .padding(28)
        .frame(width: 340)
        .background(theme["bg"])
        .task { about = await AboutInfo.load(app: app) }
        .themedSheet(isPresented: $showLicences) { LicencesSheet() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("about-window")
    }
}

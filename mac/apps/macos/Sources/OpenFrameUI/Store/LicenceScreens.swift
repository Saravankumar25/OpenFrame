// Trial reminders, the store screens and the once-only screens after a purchase (Windows:
// LicenceScreens.tsx, owner mocks 184 to 198). The core decides which screen is due; this host
// shows one at a time as a sheet over the window.
//
// Mandatory screens cannot be closed: no close button and Escape does nothing. They are the lock
// after the trial or a refund (the core refuses every project operation underneath it, so the
// screen is not the only gate), the Mac first launch, the data note and the tutorial. The
// "Checking your purchase" screen can be closed: a failed check keeps opening and exporting.
// The internal disk image build shows none of these screens.

import AppKit
import SwiftUI
import OpenFrameCore

/// Puts the due screen over the window. Added once, on the root view.
struct LicenceHost: ViewModifier {
    let licence: LicenceModel

    func body(content: Content) -> some View {
        content.sheet(item: Binding(
            get: { licence.screen },
            set: { next in
                // A sheet closing by itself (not mandatory ones) means the screen was dismissed.
                guard next == nil, let shown = licence.screen, !shown.mandatory else { return }
                Task { await licence.dismiss(shown) }
            }
        )) { screen in
            LicenceScreenView(screen: screen, licence: licence)
                .interactiveDismissDisabled(screen.mandatory)
        }
    }
}

extension View {
    func licenceScreens(_ licence: LicenceModel) -> some View { modifier(LicenceHost(licence: licence)) }
}

extension LicenceModel {
    /// A screen the person closed with the window's own gesture (Escape).
    func dismiss(_ screen: LicenceScreen) async {
        switch screen {
        case .reminder: await closeReminder()
        case .checkNeeded: closeCheck()
        case .welcome: await record(.welcome)
        default: break
        }
    }
}

struct LicenceScreenView: View {
    let screen: LicenceScreen
    let licence: LicenceModel

    var body: some View {
        switch screen {
        case let .reminder(days): ReminderDialog(daysLeft: days, licence: licence)
        case .dataNote: DataNoteDialog(licence: licence)
        case .tutorial: TutorialDialog(licence: licence)
        case .welcome: WelcomeDialog(licence: licence)
        case .macFirstLaunch: MacFirstLaunchDialog(licence: licence)
        case let .locked(revoked): LockedDialog(revoked: revoked, licence: licence)
        case .checkNeeded: CheckDialog(licence: licence)
        }
    }
}

// MARK: Shared pieces

/// The frame of every licence screen: a title, an optional line under it, the body and a footer.
private struct Dialog<Body: View, Footer: View>: View {
    let title: String
    var subtitle: String?
    var width: CGFloat = 480
    var identifier = "licence-dialog"
    @ViewBuilder var content: Body
    @ViewBuilder var footer: Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(spacing: PageSpacing.tight) {
                Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                if let subtitle { Text(subtitle).font(.callout).foregroundStyle(.secondary) }
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            content
            HStack(spacing: 8) { footer }
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
        .padding(24)
        .frame(width: width)
        .background(theme["bg"])
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(identifier)
    }
}

private struct Banner: View {
    enum Tone { case info, warn, error }
    let tone: Tone
    let text: String
    var identifier: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Label {
            Text(text).fixedSize(horizontal: false, vertical: true)
        } icon: {
            Image(systemName: tone == .info ? "info.circle" : "exclamationmark.triangle").accessibilityHidden(true)
        }
        .font(.callout)
        .foregroundStyle(tone == .error ? theme["red-t"] : tone == .warn ? theme["yellow-t"] : Color.secondary)
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityIdentifier(identifier ?? "licence-banner")
    }
}

private struct BrandHeader: View {
    var body: some View {
        HStack { Spacer(); BrandMark(size: 48); Spacer() }
    }
}

private struct StoreMessage: View {
    let message: String?
    var body: some View {
        if let message { Banner(tone: .warn, text: message, identifier: "licence-message") }
    }
}

// MARK: Trial reminder (184, 185, 189)

struct ReminderDialog: View {
    let daysLeft: Int
    let licence: LicenceModel

    var body: some View {
        Dialog(title: "Trial reminder") {
            BrandHeader()
            Text(LicenceRules.reminderHeadline(daysLeft)).font(.title2.weight(.semibold)).frame(maxWidth: .infinity)
            Text("Buy the full version to keep using OpenFrame Studio.").frame(maxWidth: .infinity)
            Banner(tone: .info, text: "Your projects stay on your computer either way. Nothing is deleted when the trial ends.")
            StoreMessage(message: licence.message)
        } footer: {
            Button("Continue trial") { Task { await licence.closeReminder() } }
                .keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("licence-continue-trial")
            Button { Task { if await licence.buy() { await licence.closeReminder() } } } label: {
                Label("Buy full version", systemImage: "bolt.fill")
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.defaultAction)
            .disabled(licence.busy)
            .accessibilityIdentifier("licence-buy")
        }
    }
}

// MARK: Data note (193)

struct DataNoteDialog: View {
    let licence: LicenceModel
    @State private var readToEnd = false
    @State private var ticked = false

    var body: some View {
        Dialog(
            title: "Your data, your responsibility", subtitle: "Shown once, right after you buy the full version.",
            width: 560
        ) {
            Banner(tone: .warn, text: "Important. Please read before you continue.")
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    ForEach(Array(LicenceRules.dataPoints.enumerated()), id: \.offset) { i, p in
                        HStack(alignment: .top, spacing: 10) {
                            Text("\(i + 1)").font(.caption.weight(.bold)).frame(width: 22, height: 22)
                                .background(.quaternary, in: Circle()).accessibilityHidden(true)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(p.title).fontWeight(.semibold)
                                Text(p.body).font(.callout).foregroundStyle(.secondary)
                            }
                        }
                        .accessibilityElement(children: .combine)
                    }
                    // A list that fits without scrolling counts as read: this row is then on screen
                    // at once. A longer one is read when it scrolls into view.
                    Color.clear.frame(height: 1).onAppear { readToEnd = true }
                }
                .padding(.trailing, 6)
            }
            .frame(maxHeight: 250)
            .accessibilityLabel("Six points about your data")
            .accessibilityIdentifier("licence-data-points")
            RootFolderCard(heading: "Your root folder", licence: licence)
            Toggle(isOn: $ticked) {
                Text("I have read this and I understand that backing up my data is my responsibility.").fontWeight(.semibold)
            }
            .toggleStyle(.checkbox)
            .disabled(!readToEnd)
            .accessibilityIdentifier("licence-tick")
            if let error = licence.saveError { Banner(tone: .error, text: error) }
        } footer: {
            Text(readToEnd ? (ticked ? "You cannot continue until this is acknowledged." : "Tick the box to continue.") : "Scroll to the end of the list to continue.")
                .font(.caption).foregroundStyle(.secondary)
            Spacer()
            Button("OK, I understand") { Task { await licence.record(.dataNote) } }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
                .disabled(!(readToEnd && ticked))
                .accessibilityIdentifier("licence-ok")
        }
    }
}

private struct RootFolderCard: View {
    let heading: String
    let licence: LicenceModel
    @State private var failure: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(heading).font(.callout.weight(.semibold))
            HStack {
                Text(licence.dataRoot).textSelection(.enabled).font(.callout.monospaced())
                    .lineLimit(3).truncationMode(.middle).accessibilityIdentifier("licence-data-root")
                Spacer()
                Button { Task { failure = await licence.revealDataRoot() } } label: {
                    Label(LicenceRules.revealLabel, systemImage: "folder")
                }
                .controlSize(.small)
                .accessibilityIdentifier("licence-reveal-folder")
            }
            if let failure { Text(failure).font(.caption).foregroundStyle(.red) }
        }
        .padding(10)
        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8))
    }
}

// MARK: File safety tutorial (194 to 197)

struct TutorialDialog: View {
    let licence: LicenceModel
    @State private var step = 0
    @State private var checks = [false, false, false]
    @AccessibilityFocusState private var headingFocused: Bool

    private var last: Bool { step == LicenceRules.tutorialSteps.count - 1 }

    var body: some View {
        Dialog(title: "File safety tutorial", width: 560) {
            HStack(spacing: 6) {
                ForEach(Array(LicenceRules.tutorialSteps.enumerated()), id: \.offset) { i, name in
                    Text("\(i + 1) \(name)")
                        .font(.caption.weight(i == step ? .bold : .regular))
                        .padding(.horizontal, 10).padding(.vertical, 3)
                        .background(i == step ? Color.primary.opacity(0.85) : i < step ? Color.green.opacity(0.25) : Color.secondary.opacity(0.15), in: Capsule())
                        .foregroundStyle(i == step ? Color(nsColor: .windowBackgroundColor) : Color.primary)
                        .accessibilityLabel("\(i + 1) \(name)\(i < step ? ", done" : "")")
                        .accessibilityAddTraits(i == step ? .isSelected : [])
                }
            }
            Text(LicenceRules.tutorialTitles[step]).font(.title3.weight(.semibold))
                .frame(maxWidth: .infinity)
                .accessibilityFocused($headingFocused).accessibilityIdentifier("licence-tutorial-heading")
            stepBody
            if let error = licence.saveError { Banner(tone: .error, text: error) }
        } footer: {
            Text("Step \(step + 1) of 4 · mandatory").font(.caption).foregroundStyle(.secondary)
            Spacer()
            if step > 0 {
                Button("Back") { step -= 1 }.accessibilityIdentifier("licence-back")
            }
            if last {
                Button("Finish") { Task { await licence.record(.tutorial) } }
                    .buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                    .disabled(!checks.allSatisfy { $0 })
                    .accessibilityIdentifier("licence-finish")
            } else {
                Button("Next") { step += 1 }
                    .buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("licence-next")
            }
        }
        // Each new step moves VoiceOver to its heading.
        .onChange(of: step) { _, new in if new > 0 { headingFocused = true } }
    }

    @ViewBuilder private var stepBody: some View {
        switch step {
        case 0:
            RootFolderCard(heading: "Your root folder", licence: licence)
            VStack(alignment: .leading, spacing: 4) {
                Text("Inside it").font(.callout.weight(.semibold))
                Text("Projects: one folder for each project, with its data, drafts and attachments")
                Text("Projects, Backups: copies you make with Create Backup")
                Text("Global Idea Vault: ideas you keep across projects")
            }
            .font(.callout)
            .padding(10).frame(maxWidth: .infinity, alignment: .leading)
            .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8))
            Text("Everything you create is inside this one folder. Remember this location.").font(.callout)
        case 1:
            numbered([
                "Install Google Drive for desktop (or your preferred cloud or external-disk tool).",
                "Choose the root folder shown in step 1 as a folder to sync, or copy it into your Drive folder.",
                "Check in the Drive website that your projects appear there.",
                "Repeat after every working session, or turn on automatic sync.",
            ])
            Banner(tone: .info, text: "Close OpenFrame (or Create Backup) before copying so files are complete. An external drive works as well as Google Drive.")
        case 2:
            numbered([
                "On the old laptop copy the whole root folder to a USB disk or Google Drive.",
                LicenceRules.newLaptopInstallStep(licence.channel),
                "Copy the folder to the new laptop.",
                "Open OpenFrame, choose Open Project, and pick the project inside the folder.",
            ])
            Banner(tone: .warn, text: "Linked external files (for example a reference photo on another drive) must be copied too. OpenFrame lists missing ones after opening.")
        default:
            Text("Confirm that you understand each point.").font(.callout)
            ForEach(Array(LicenceRules.tutorialChecks.enumerated()), id: \.offset) { i, label in
                Toggle(label, isOn: $checks[i]).toggleStyle(.checkbox)
                    .accessibilityIdentifier("licence-check-\(i + 1)")
            }
        }
    }

    private func numbered(_ items: [String]) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(items.enumerated()), id: \.offset) { i, t in
                Text("\(i + 1). \(t)").fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}

// MARK: Welcome (198)

struct WelcomeDialog: View {
    let licence: LicenceModel
    @Environment(AppModel.self) private var app

    var body: some View {
        Dialog(title: "Welcome to the OpenFrame Studio family", width: 560) {
            BrandHeader()
            Text("Thank you for backing independent filmmaking. Be a part of our community.")
                .foregroundStyle(.secondary).frame(maxWidth: .infinity)
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 12), count: 4), spacing: 12) {
                ForEach(LicenceRules.socialLinks) { link in
                    let openable = LicenceRules.isFixedSocialURL(link.url)
                    Button {
                        guard openable, let url = link.url else { return }
                        Task { try? await app.core?.openURL(url) }
                    } label: {
                        VStack(spacing: 2) {
                            Text(link.short).font(.callout.weight(.bold))
                            Text(link.name).font(.caption.weight(.semibold))
                            Text(link.handle).font(.caption2).foregroundStyle(.secondary)
                        }
                        .frame(maxWidth: .infinity).padding(8)
                        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8))
                    }
                    .buttonStyle(.plain)
                    .disabled(!openable)
                    .accessibilityLabel(openable ? "\(link.name) \(link.handle), opens in your browser" : "\(link.name) \(link.handle)")
                    .accessibilityIdentifier("licence-social-\(link.name.lowercased())")
                }
            }
        } footer: {
            Button("OK") { Task { await licence.record(.welcome) } }
                .buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("licence-ok")
        }
    }
}

// MARK: Store screens (187, 190, 192)

struct MacFirstLaunchDialog: View {
    let licence: LicenceModel

    var body: some View {
        Dialog(title: "Welcome to OpenFrame Studio", subtitle: "How would you like to begin?", width: 560) {
            BrandHeader()
            HStack(alignment: .top, spacing: 12) {
                card(
                    "Free trial", "7 days. Everything included. No credit card.", id: "licence-start-trial"
                ) {
                    Button("Start free trial") { Task { await licence.startTrial() } }
                        .disabled(licence.busy).accessibilityIdentifier("licence-start-trial")
                }
                card(
                    "Buy the full version", "Through the App Store, which shows you the details before you buy.", id: "licence-buy-card"
                ) {
                    Button { Task { await licence.buy() } } label: { Label("Buy full version", systemImage: "bolt.fill") }
                        .buttonStyle(.borderedProminent).disabled(licence.busy).accessibilityIdentifier("licence-buy")
                }
            }
            HStack {
                Spacer()
                Button { Task { await licence.restore() } } label: { Label("Restore purchase", systemImage: "arrow.clockwise") }
                    .buttonStyle(.borderless).controlSize(.small).disabled(licence.busy)
                    .accessibilityIdentifier("licence-restore")
                Spacer()
            }
            StoreMessage(message: licence.message)
        } footer: {
            EmptyView()
        }
    }

    private func card<Action: View>(_ title: String, _ text: String, id: String, @ViewBuilder action: () -> Action) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.headline)
            Text(text).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            action()
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
    }
}

/// The lock after the trial or a refund: only Buy and Restore, and it cannot be closed.
struct LockedDialog: View {
    let revoked: Bool
    let licence: LicenceModel

    var body: some View {
        Dialog(title: revoked ? "Your purchase is no longer active" : "Your free trial has ended") {
            BrandHeader()
            Text(revoked
                ? "The App Store reports that this purchase was refunded or revoked. Buy the full version to open your projects again."
                : "Buy the full version to open your projects again.")
                .multilineTextAlignment(.center).frame(maxWidth: .infinity)
            Banner(tone: .info, text: "Your projects are safe on this Mac. Nothing is deleted. Buying or restoring opens them again exactly as you left them.")
            StoreMessage(message: licence.message)
        } footer: {
            VStack(spacing: 8) {
                Button { Task { await licence.buy() } } label: { Label("Buy full version", systemImage: "bolt.fill") }
                    .buttonStyle(.borderedProminent).controlSize(.large).keyboardShortcut(.defaultAction)
                    .disabled(licence.busy).accessibilityIdentifier("licence-buy")
                Button { Task { await licence.restore() } } label: { Label("Restore purchase", systemImage: "arrow.clockwise") }
                    .buttonStyle(.borderless).controlSize(.small).disabled(licence.busy)
                    .accessibilityIdentifier("licence-restore")
            }
            .frame(maxWidth: .infinity)
        }
    }
}

/// The licence could not be confirmed online. Closable: opening and exporting keep working.
struct CheckDialog: View {
    let licence: LicenceModel

    var body: some View {
        Dialog(title: "Checking your purchase", subtitle: licence.busy ? "Talking to the App Store…" : nil) {
            if licence.busy {
                ProgressView().progressViewStyle(.linear).accessibilityLabel("Checking with the App Store")
                    .accessibilityIdentifier("licence-progress")
            } else {
                Banner(
                    tone: .error,
                    text: "We could not reach the App Store. Connect to the internet to check your purchase. If you bought before, your purchase will be restored as soon as the check succeeds."
                )
            }
            Banner(tone: .info, text: "Your projects stay open and can be exported. Nothing is deleted.")
            StoreMessage(message: licence.message)
        } footer: {
            Button("Not now") { licence.closeCheck() }
                .keyboardShortcut(.cancelAction).disabled(licence.busy).accessibilityIdentifier("licence-not-now")
            Button("Buy full version") { Task { await licence.buy() } }
                .disabled(licence.busy).accessibilityIdentifier("licence-buy")
            Button("Try again") { Task { await licence.restore() } }
                .buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                .disabled(licence.busy).accessibilityIdentifier("licence-try-again")
        }
    }
}

// The licence and trial screens' model (Windows: features/licensing/licence.ts and the effects of
// LicenceScreens.tsx). The core decides everything: this model asks `entitlement.notice`, shows
// the one screen it names, and sends the person's choices back as operations. In the internal
// disk image build the core says `none` for ever and nothing here is ever shown.

import AppKit
import Foundation
import Observation
import StraftCore

/// Whether the long-text editors take typing. While a Mac App Store build waits for a purchase check
/// the core refuses changes, so the editors stay read-only rather than hold text they cannot store.
@MainActor
enum LicenceGate {
    static var editingWaits = false
}

/// One screen at a time.
public enum LicenceScreen: Equatable, Identifiable {
    case reminder(daysLeft: Int)
    case dataNote
    case tutorial
    case welcome
    case macFirstLaunch
    case locked(revoked: Bool)
    case clockLocked
    case checkNeeded

    public var id: String {
        switch self {
        case let .reminder(d): "reminder-\(d)"
        case .dataNote: "dataNote"
        case .tutorial: "tutorial"
        case .welcome: "welcome"
        case .macFirstLaunch: "macFirstLaunch"
        case let .locked(revoked): "locked-\(revoked)"
        case .clockLocked: "clockLocked"
        case .checkNeeded: "checkNeeded"
        }
    }

    /// Mandatory screens cannot be closed: no close button, Escape does nothing. They are the
    /// lock after the trial or a refund, the clock lock, the Mac first launch, the data note and
    /// the tutorial.
    public var mandatory: Bool {
        switch self {
        case .dataNote, .tutorial, .macFirstLaunch, .locked, .clockLocked: true
        case .reminder, .welcome, .checkNeeded: false
        }
    }
}

@MainActor
@Observable
public final class LicenceModel {
    public private(set) var notice: EntitlementNotice?
    public private(set) var busy = false
    /// Plain words under the buttons when a purchase or a check did not finish.
    public private(set) var message: String?
    /// The "Checking your purchase" screen was closed with "Not now". It stays closed while the
    /// core still names it, and the top bar offers "Check purchase" (Windows `useCheckScreen`).
    public private(set) var checkClosed = false
    public private(set) var reminderDays: Int?
    /// A save step failed (data note, tutorial): plain words on that screen.
    public private(set) var saveError: String?

    private var dismissedOn: String?
    /// The app is quitting: even a screen that cannot be closed steps aside, or it would hold the
    /// quit. Nothing is unlocked by this: the core still refuses, and the next launch shows it again.
    private var quitting = false
    @ObservationIgnored private var core: Core?
    @ObservationIgnored private var runtime: StoreRuntime?
    @ObservationIgnored private var observers: [NSObjectProtocol] = []
    /// Injected by tests, so a reminder day does not depend on the date the test runs.
    @ObservationIgnored var today: @MainActor () -> String = { LicenceRules.localDay() }
    /// The address opener. A shell action, so it works while the core has locked every project
    /// operation. Injected by tests.
    @ObservationIgnored var openAddress: @MainActor (URL) -> Void = { NSWorkspace.shared.open($0) }
    /// Saves what the editors still hold, before the lock takes hold. Set by the app.
    @ObservationIgnored public var flushBeforeLock: @MainActor () async -> Void = {}
    /// Screens read again what was refused while the lock held. Set by the app.
    @ObservationIgnored public var onLockLifted: @MainActor () -> Void = {}
    /// The lock window was closed for this lock, so it is not asked for twice.
    @ObservationIgnored private var flushedForLock = false

    public init() {}

    // MARK: Lifecycle

    /// Start asking the core. Called once the library is ready and the app opens.
    public func attach(core: Core, runtime: StoreRuntime? = nil, observeSystem: Bool = true) async {
        self.core = core
        self.runtime = runtime
        await refresh()
        guard isStore else { return }
        runtime?.startListening { [weak core] in core?.requestStoreCheck("storeKitChanged") }
        if observeSystem { observeApp() }
    }

    public func detach() {
        runtime?.stopListening()
        for o in observers { NotificationCenter.default.removeObserver(o); NSWorkspace.shared.notificationCenter.removeObserver(o) }
        observers = []
    }

    /// The app coming to the front and a wake ask the store again. The core's worker leaves out a
    /// repeat within a minute.
    private func observeApp() {
        guard observers.isEmpty else { return }
        let center = NotificationCenter.default
        observers.append(center.addObserver(forName: NSApplication.didBecomeActiveNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self else { return }
                Task { await self.refresh() }
                self.core?.requestStoreCheck("resume")
            }
        })
        observers.append(NSWorkspace.shared.notificationCenter.addObserver(forName: NSWorkspace.didWakeNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.core?.requestStoreCheck("wake") }
        })
    }

    // MARK: What is shown

    /// True in a store build. The internal build is never sold: no screen, no chip, no Buy.
    public var isStore: Bool {
        guard let channel = notice?.status.channel else { return false }
        return channel != .directInstaller
    }

    public var channel: Channel? { notice?.status.channel }
    public var state: EntitlementState? { notice?.status.state }
    public var dataRoot: String { notice?.dataRoot ?? "" }

    /// The screen due now, or `nil`.
    public var screen: LicenceScreen? {
        guard let notice, isStore, !quitting else { return nil }
        if let d = reminderDays { return .reminder(daysLeft: d) }
        switch notice.notice {
        case .dataNote: return .dataNote
        case .tutorial: return .tutorial
        case .welcome: return .welcome
        case .macFirstLaunch: return .macFirstLaunch
        case .locked: return .locked(revoked: notice.status.state == .revoked)
        case .clockLocked: return .clockLocked
        case .checkNeeded: return checkClosed ? nil : .checkNeeded
        case .reminder, .none_: return nil
        }
    }

    /// Whether a screen that blocks the app is open (the search palette and the like wait).
    public var blocksApp: Bool { screen?.mandatory == true }

    public var chip: LicenceRules.TrialChip? { isStore ? LicenceRules.trialChip(state) : nil }

    /// `Product.displayPrice` of the lifetime product, read at run time. `nil` until both products
    /// loaded.
    public var lifetimePrice: String? { notice?.status.lifetimePrice }

    /// The moment a running trial ends, for the reminder's end line.
    public var trialEndsAt: Date? {
        (notice?.status.trialEndsAtMs).map { Date(timeIntervalSince1970: $0 / 1000) }
    }

    /// "Start free trial" waits until the App Store has answered with both products.
    public var canStartTrial: Bool { lifetimePrice != nil && !busy }

    /// "Check purchase" in the top bar, after "Not now" on the check screen.
    public var showsCheckPurchase: Bool {
        guard isStore, checkClosed, case .checkNeeded = notice?.notice else { return false }
        return true
    }

    // MARK: Reading

    public func refresh() async {
        guard let core else { return }
        do {
            let next = try await core.entitlementNotice(EntitlementNoticeArgs(today: today()))
            await finishSavesIfLocking(next)
            notice = next
            LicenceGate.editingWaits = editingWaits
            if checkClosed {
                // A closed check screen stays closed only while the core still names it.
                if case .checkNeeded = next.notice {} else { checkClosed = false }
            }
            await showReminderIfDue()
        } catch {
            // The licence screens never block the app because a read failed.
        }
    }

    /// The core just turned to a lock: store what the editors still hold before the lock screen
    /// appears, then tell the core, which refuses every project operation from there on. A lock
    /// that is lifted and comes back flushes again.
    private func finishSavesIfLocking(_ next: EntitlementNotice) async {
        let locking = next.status.channel != .directInstaller
            && (next.notice == .locked || next.notice == .clockLocked)
        guard locking else {
            if flushedForLock { onLockLifted() }
            flushedForLock = false
            return
        }
        guard !flushedForLock, let core else { return }
        flushedForLock = true
        await flushBeforeLock()
        _ = try? await core.entitlementLockReady()
    }

    /// Shown on Application Home or over a project. A reminder is remembered as soon as it is shown, so it appears at most once that day even
    /// if Straft Studio closes while it is open. A failed save only means it may show once more.
    private func showReminderIfDue() async {
        guard let core, isStore, reminderDays == nil, dismissedOn != today(),
              case let .reminder(days) = notice?.notice
        else { return }
        reminderDays = Int(days.rounded())
        _ = try? await core.entitlementRecordNotice(RecordNoticeArgs(notice: .reminder, today: today()))
    }

    public func closeReminder() async {
        reminderDays = nil
        dismissedOn = today()
        await refresh()
    }

    func prepareForQuit() { quitting = true }

    /// The quit did not happen after all: the screens come back, so a locked person has Buy again.
    func quitDidNotHappen() { quitting = false }

    /// The Mac App Store build is waiting for a check or for the trial to start, and the core
    /// refuses changes. Editors stay read-only rather than take text they could not store.
    public var editingWaits: Bool {
        guard isStore, let status = notice?.status else { return false }
        return !status.canEdit
    }

    public func closeCheck() { checkClosed = true }
    public func reopenCheck() { checkClosed = false }

    // MARK: Actions

    /// "Start free trial" (Mac first launch).
    public func startTrial() async { await storeAction { core in try await core.entitlementStartTrial().message } }

    /// "Buy full version": Apple's own payment sheet, which shows the price.
    @discardableResult
    public func buy() async -> Bool {
        await storeAction { core in try await core.entitlementBuy().message }
    }

    /// "Restore purchase", "Try again" and "Check again".
    public func restore() async {
        await storeAction { core in
            _ = try await core.entitlementRestorePurchase()
            return nil
        }
    }

    @discardableResult
    private func storeAction(_ run: (Core) async throws -> String?) async -> Bool {
        guard let core, !busy else { return false }
        busy = true
        message = nil
        defer { busy = false }
        var ok = true
        do {
            message = try await run(core)
            ok = message == nil
        } catch {
            message = CoreFailure(asCore: error).message
            ok = false
        }
        await refresh()
        return ok
    }

    /// "Contact us". Opens the website's contact page in the browser; the app sends nothing.
    public func openContact() {
        guard let url = URL(string: LicenceRules.contactAddress) else { return }
        openAddress(url)
    }

    /// The data note, the tutorial or the welcome was finished.
    public func record(_ kind: NoticeKind) async {
        guard let core else { return }
        saveError = nil
        do {
            notice = try await core.entitlementRecordNotice(RecordNoticeArgs(notice: kind, today: today()))
            LicenceGate.editingWaits = editingWaits
        } catch {
            saveError = CoreFailure(asCore: error).message
            if kind == .welcome { await refresh() }
        }
    }

    /// "Show in Finder" for the one folder to back up.
    public func revealDataRoot() async -> String? {
        guard let core else { return nil }
        do {
            try await core.revealLocation("dataRoot")
            return nil
        } catch {
            return CoreFailure(asCore: error).message
        }
    }

}

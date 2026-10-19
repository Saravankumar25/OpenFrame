// The Mac App Store licence flow through the real core as the store channel (debug test switch),
// with a fake Apple and a fake licensing server that signs its answers: trial start, days left,
// expiry, purchase, restore, revocation, unverified results dropped, environments never mixed, and
// the screens the model names. The real StoreKit purchase sheet needs Xcode's StoreKit testing and
// cannot run here.

import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

@Suite("Store licence", .serialized)
@MainActor
struct StoreLicenceTests {
    @Test func aFreshStoreBuildAsksHowToBeginAndShowsNoChip() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        let status = try await rig.status()
        #expect(status.channel == .macAppStore)
        #expect(status.state == .trialNotStarted)
        #expect(rig.licence.isStore)
        #expect(rig.licence.screen == .macFirstLaunch)
        #expect(rig.licence.screen?.mandatory == true)
        #expect(rig.licence.chip == nil)
    }

    @Test func startingTheTrialCountsSevenDaysFromApplesDateAndTheChipSaysSo() async throws {
        let rig = try await StoreRig.start(now: storeT0.addingTimeInterval(3600))
        defer { rig.finish() }
        await rig.startTrial()
        #expect(rig.source.purchaseCalls == [trialProductID])
        #expect(try await rig.status().state == .trialActive(days_left: 7))
        #expect(rig.licence.screen == nil)
        #expect(rig.licence.chip?.label == "Trial: 7 days left")
        #expect(rig.licence.chip?.ending == false)
        // The server was asked with the signed transaction, never with a Mac date.
        let request = rig.server.requests.last
        #expect((request?["transactionJws"] as? [String])?.count == 1)
        #expect(request?["environment"] as? String == "production")
    }

    @Test func daysLeftFollowTheServersClockAndTheLastThreeShowAReminderOverAProject() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        rig.server.now = storeT0.addingTimeInterval(3 * storeDay)
        await rig.licence.restore()
        #expect(try await rig.status().state == .trialActive(days_left: 4))
        #expect(rig.licence.chip?.ending == false)

        rig.server.now = storeT0.addingTimeInterval(4.5 * storeDay)
        await rig.licence.restore()
        #expect(try await rig.status().state == .trialActive(days_left: 3))
        #expect(rig.licence.chip?.ending == true)
        #expect(rig.licence.chip?.spoken == "Free trial, 3 days left. Your trial ends soon.")
        // No reminder on Application Home; it shows once a project is open.
        #expect(rig.licence.screen == nil)
        await rig.licence.setProjectOpen(true)
        #expect(rig.licence.screen == .reminder(daysLeft: 3))
        #expect(LicenceRules.reminderHeadline(3) == "Your free trial ends in 3 days")
        await rig.licence.closeReminder()
        #expect(rig.licence.screen == nil)
        // Once per calendar day, also after the model reads the core again.
        await rig.licence.setProjectOpen(false)
        await rig.licence.setProjectOpen(true)
        await rig.licence.refresh()
        #expect(rig.licence.screen == nil)
    }

    @Test func whenTheTrialEndsTheAppLocksUntilAPurchaseOrARestore() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        rig.server.now = storeT0.addingTimeInterval(8 * storeDay)
        await rig.licence.restore()
        #expect(try await rig.status().state == .trialEnded)
        #expect(rig.licence.screen == .locked(revoked: false))
        #expect(rig.licence.blocksApp)
        // The core refuses project operations underneath the screen, not only the screen.
        await #expect(throws: CoreFailure.self) {
            _ = try await rig.core.projectCreate(CreateProjectArgs(title: "Blocked", projectType: .featureFilm))
        }
        // Restore with no purchase keeps the lock and says nothing wrong.
        await rig.licence.restore()
        #expect(rig.licence.screen == .locked(revoked: false))
        // Buying lifts it.
        await rig.buyLifetime()
        #expect(try await rig.status().state == .purchased)
        #expect(rig.licence.blocksApp == true, "the data note follows the purchase")
        #expect(rig.licence.screen == .dataNote)
        _ = try await rig.core.projectCreate(CreateProjectArgs(title: "Open again", projectType: .featureFilm))
    }

    @Test func aPurchaseShowsTheThreeSafetyScreensOnceAndInOrder() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.buyLifetime()
        #expect(rig.source.purchaseCalls == [lifetimeProductID])
        #expect(try await rig.status().state == .purchased)
        #expect(rig.licence.chip == nil)
        #expect(rig.licence.screen == .dataNote)
        #expect(!rig.licence.dataRoot.isEmpty)
        await rig.licence.record(.dataNote)
        #expect(rig.licence.screen == .tutorial)
        await rig.licence.record(.tutorial)
        #expect(rig.licence.screen == .welcome)
        await rig.licence.record(.welcome)
        #expect(rig.licence.screen == nil)
        // They do not come back.
        await rig.licence.refresh()
        #expect(rig.licence.screen == nil)
    }

    @Test func restoreFindsAPurchaseMadeOnAnotherMac() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.source.afterSync = [.verified(fakeTransaction(lifetimeProductID, id: "2000000002"))]
        await rig.licence.restore()
        #expect(rig.source.syncCalls == 1)
        #expect(try await rig.status().state == .purchased)
    }

    @Test func aRefundLocksAgainWithTheRevokedWords() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.source.entitlements = [.verified(fakeTransaction(lifetimeProductID, id: "2000000002"))]
        await rig.licence.restore()
        #expect(try await rig.status().state == .purchased)
        rig.source.entitlements = [.verified(fakeTransaction(lifetimeProductID, id: "2000000002", revokedAt: storeT0.addingTimeInterval(60)))]
        rig.source.pushUpdate(.transaction)
        await rig.licence.restore()
        #expect(try await rig.status().state == .revoked)
        #expect(rig.licence.screen == .locked(revoked: true))
    }

    @Test func unverifiedResultsNeverGrantAnything() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.source.entitlements = [.unverified, .unverified]
        await rig.licence.restore()
        #expect(try await rig.status().state != .purchased)
        #expect(rig.runtime.bridge.unverifiedDropped() == 2)
        // An unverified purchase result is not a purchase.
        rig.source.entitlements = []
        rig.source.queue(.success(.success(.unverified)))
        await rig.licence.buy()
        #expect(try await rig.status().state != .purchased)
        #expect(rig.licence.message == "The App Store could not confirm this purchase. Choose Restore purchase to check again.")
    }

    @Test func testPurchasesAndRealPurchasesAreNeverMixed() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        // A production build whose StoreKit holds a sandbox purchase and an Xcode one.
        rig.source.entitlements = [
            .verified(fakeTransaction(lifetimeProductID, id: "3", env: .sandbox)),
            .verified(fakeTransaction(lifetimeProductID, id: "4", env: .xcode)),
        ]
        await rig.licence.restore()
        #expect(try await rig.status().state != .purchased)
        // The same purchase in the build's own environment counts.
        rig.source.entitlements = [.verified(fakeTransaction(lifetimeProductID, id: "5", env: .production))]
        await rig.licence.restore()
        #expect(try await rig.status().state == .purchased)
    }

    @Test func aSandboxBuildAcceptsSandboxPurchasesOnly() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let source = FakeStoreKitSource()
        source.environment = .sandbox
        let server = FakeLicenceServer()
        let runtime = StoreRuntime(source: source, server: server)
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        runtime.install(into: &options)
        options.testStoreResponseKeyHex = server.publicKeyHex
        let core = try await Core.start(options)
        defer { core.stop() }
        source.entitlements = [.verified(fakeTransaction(lifetimeProductID, id: "6", env: .production))]
        _ = try await core.entitlementRestorePurchase()
        #expect(try await core.entitlementStatus().state != .purchased)
        source.entitlements = [.verified(fakeTransaction(lifetimeProductID, id: "7", env: .sandbox))]
        _ = try await core.entitlementRestorePurchase()
        #expect(try await core.entitlementStatus().state == .purchased)
    }

    @Test func anAskToBuyPendingPurchaseGrantsNothingAndACancelChangesNothing() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.source.queue(.success(.pending))
        await rig.licence.startTrial()
        #expect(try await rig.status().state == .trialNotStarted)
        rig.source.queue(.success(.cancelled))
        await rig.licence.buy()
        #expect(try await rig.status().state == .trialNotStarted)
        #expect(rig.licence.screen == .macFirstLaunch)
    }

    @Test func aSecondTrialPurchaseNeverRestartsTheTrial() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial(at: storeT0, id: "2000000001")
        rig.server.now = storeT0.addingTimeInterval(2 * storeDay)
        await rig.licence.restore()
        let before = try await rig.status().state
        #expect(before == .trialActive(days_left: 5))
        // Apple would not sell it again, but even a later trial transaction does not move the end.
        rig.source.entitlements.append(.verified(fakeTransaction(trialProductID, id: "2000000009", at: storeT0.addingTimeInterval(2 * storeDay))))
        await rig.licence.restore()
        #expect(try await rig.status().state == before)
    }

    @Test func storeProblemsAreShownInPlainWordsAndNothingIsLost() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.source.queue(.failure(.purchasesNotAllowed))
        await rig.licence.buy()
        #expect(rig.licence.message == "Purchases are turned off on this Mac. Check Screen Time, then try again.")
        #expect(!rig.licence.busy)
        rig.source.queue(.failure(.notSignedIn))
        await rig.licence.startTrial()
        #expect(rig.licence.message == "Sign in to the App Store, then try again.")
        rig.source.queue(.failure(.noConnection))
        await rig.licence.buy()
        #expect(rig.licence.message == "OpenFrame could not reach the App Store. Connect to the internet, then try again.")
        #expect(try await rig.status().state == .trialNotStarted)
    }

    @Test func aTrialThatCannotBeConfirmedOnlineAsksToCheckAndKeepsOpeningAndExporting() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.server.mode = .offline
        await rig.startTrial()
        let status = try await rig.status()
        #expect(status.state == .needsOnlineCheck)
        #expect(rig.licence.screen == .checkNeeded)
        #expect(rig.licence.screen?.mandatory == false)
        #expect(!rig.licence.blocksApp)
        // "Not now" closes it and the top bar offers "Check purchase".
        rig.licence.closeCheck()
        #expect(rig.licence.screen == nil)
        #expect(rig.licence.showsCheckPurchase)
        rig.licence.reopenCheck()
        #expect(rig.licence.screen == .checkNeeded)
        // Back online, "Try again" confirms the trial and the screen goes away.
        rig.server.mode = .honest
        await rig.licence.restore()
        #expect(try await rig.status().state == .trialActive(days_left: 7))
        #expect(rig.licence.screen == nil)
        #expect(!rig.licence.showsCheckPurchase)
    }

    @Test func aServerAnswerSignedWithAnotherKeyNeverStartsOrExtendsATrial() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        rig.server.mode = .wrongKey
        await rig.startTrial()
        #expect(try await rig.status().state == .needsOnlineCheck)
    }

    @Test func storeKitUpdatesAskTheCoreForANewCheck() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        // A purchase on another Mac arrives as an update; the core checks again by itself.
        rig.source.entitlements = [.verified(fakeTransaction(lifetimeProductID, id: "2000000002"))]
        rig.source.pushUpdate(.transaction)
        var state = try await rig.status().state
        for _ in 0..<300 where state != .purchased {
            try await Task.sleep(for: .milliseconds(20))
            state = try await rig.status().state
        }
        #expect(state == .purchased)
        // The core tells the shell when its check finished.
        await rig.licence.refresh()
        #expect(rig.licence.screen == .dataNote)
    }
}

@Suite("Store internal build")
@MainActor
struct StoreInternalBuildTests {
    @Test func theInternalBuildNeverShowsALicenceScreenNorAsksStoreKit() async throws {
        let rig = try await StoreRig.start(store: false)
        defer { rig.finish() }
        let status = try await rig.status()
        #expect(status.channel == .directInstaller)
        #expect(status.state == .internalBuild)
        #expect(!rig.licence.isStore)
        #expect(rig.licence.screen == nil)
        #expect(rig.licence.chip == nil)
        #expect(!rig.licence.showsCheckPurchase)
        #expect(!rig.licence.blocksApp)
        // Every store action is a no-op there: the core says it is not for sale.
        await rig.licence.buy()
        await rig.licence.startTrial()
        await rig.licence.restore()
        rig.core.requestStoreCheck("launch")
        rig.core.requestStoreCheck("storeKitChanged")
        try await Task.sleep(for: .milliseconds(200))
        #expect(rig.source.calls == 0, "StoreKit is never called in the internal build")
        #expect(rig.server.requests.isEmpty)
        #expect(rig.licence.screen == nil)
        // Not even with a trial that ended: nothing locks and nothing shows.
        let notice = try await rig.core.entitlementNotice(EntitlementNoticeArgs(today: "2026-10-09"))
        #expect(notice.notice == .none_)
    }

    @Test func aStateSetByHandOnTheInternalChannelStillShowsNothing() async throws {
        let rig = try await StoreRig.start(store: false)
        defer { rig.finish() }
        _ = try await rig.core.call("entitlement.debug_set", DebugSetArgs(channel: .directInstaller, state: .trialEnded, resetNotices: true)) as EmptyResult
        await rig.licence.refresh()
        #expect(rig.licence.screen == nil)
        #expect(rig.licence.chip == nil)
    }

    @Test func screenTextsNameNoPriceAndNoPricingModel() {
        let words = [
            LicenceRules.reminderHeadline(1), LicenceRules.reminderHeadline(3), LicenceRules.buyFailed,
            LicenceRules.newLaptopInstallStep(.macAppStore),
        ] + LicenceRules.dataPoints.map(\.body) + LicenceRules.tutorialChecks
        for text in words {
            #expect(!text.contains("$") && !text.contains("€") && !text.lowercased().contains("subscription"))
        }
        #expect(!words.joined().contains("\u{2014}"))
    }
}

@Suite("Licence rules")
struct LicenceRulesTests {
    @Test func theChipClampsToSevenDaysAndSpeaksTheLastDay() {
        #expect(LicenceRules.trialChip(.trialActive(days_left: 9))?.daysLeft == 7)
        #expect(LicenceRules.trialChip(.trialActive(days_left: 0.2))?.daysLeft == 1)
        #expect(LicenceRules.trialChip(.trialActive(days_left: 1))?.label == "Trial: 1 day left")
        #expect(LicenceRules.trialChip(.trialActive(days_left: 1))?.spoken == "Free trial, last day. Your trial ends today.")
        #expect(LicenceRules.trialChip(.purchased) == nil)
        #expect(LicenceRules.trialChip(.trialEnded) == nil)
        #expect(LicenceRules.trialChip(nil) == nil)
    }

    @Test func theLocalDayUsesThePersonsOwnCalendar() {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        // 23:30 UTC on the 8th is already the 9th in India.
        let d = Date(timeIntervalSince1970: 1_791_244_800 + 3 * storeDay + 23.5 * 3600 - 86_400)
        #expect(LicenceRules.localDay(d, calendar: cal) == "2026-10-09")
    }

    @Test func onlyFixedSocialAddressesOpen() {
        #expect(!LicenceRules.isFixedSocialURL(nil))
        #expect(LicenceRules.isFixedSocialURL("https://www.instagram.com/openframestudio"))
        #expect(!LicenceRules.isFixedSocialURL("http://www.instagram.com/x"))
        #expect(!LicenceRules.isFixedSocialURL("https://evil.example.com/x"))
        #expect(!LicenceRules.isFixedSocialURL("https://user:pw@x.com/a"))
        #expect(!LicenceRules.isFixedSocialURL("https://x.com:8443/a"))
        #expect(LicenceRules.socialLinks.allSatisfy { $0.url == nil })
    }
}

// The Mac App Store licence flow through the real core as the store channel (debug test switch),
// with a fake Apple, a fake Keychain and a clock the test moves: trial start, days left, expiry,
// the clock guard, purchase, restore, revocation, unverified results dropped, environments never
// mixed, and the screens the model names. The real StoreKit purchase sheet needs Xcode's StoreKit testing and
// cannot run here.

import Foundation
import Testing
@testable import StraftCore
@testable import StraftUI

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

    @Test func theFirstLaunchShowsThePriceFromTheStoreAndOnlyThenOffersTheTrial() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        #expect(rig.licence.lifetimePrice == fakeDisplayPrice)
        #expect(rig.licence.canStartTrial)
        #expect(LicenceRules.trialDisclosure(price: fakeDisplayPrice).hasSuffix("one-time purchase of \(fakeDisplayPrice)."))
    }

    @Test func aFirstLaunchOfflineLandsOnTheCheckScreenNotABlankOrAGuess() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let source = FakeStoreKitSource()
        source.productsFail = true
        let runtime = StoreRuntime(source: source, keychain: FakeGuardKeychain(), clock: { storeT0 })
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        runtime.install(into: &options)
        options.testStoreChannel = true
        let core = try await Core.start(options)
        defer { core.stop() }
        let licence = LicenceModel()
        licence.today = { "2026-10-09" }
        await licence.attach(core: core, runtime: runtime, observeSystem: false)
        for _ in 0..<50 {
            try await Task.sleep(for: .milliseconds(20))
            await licence.refresh()
        }
        #expect(licence.state == .needsOnlineCheck)
        #expect(licence.screen == .checkNeeded)
        #expect(licence.lifetimePrice == nil)
        #expect(!licence.canStartTrial)
        // Editing waits: the core refuses a new project and the editors do not take typing.
        #expect(licence.editingWaits)
        #expect(LicenceGate.editingWaits)
        await #expect(throws: CoreFailure.self) {
            _ = try await core.projectCreate(CreateProjectArgs(title: "Waits", projectType: .featureFilm))
        }
        // Back online, "Try again" offers the choice.
        source.productsFail = false
        await licence.restore()
        #expect(licence.screen == .macFirstLaunch)
        LicenceGate.editingWaits = false
    }

    @Test func startingTheTrialCountsSevenDaysFromApplesDateAndTheChipSaysSo() async throws {
        let rig = try await StoreRig.start(now: storeT0.addingTimeInterval(3600))
        defer { rig.finish() }
        await rig.startTrial()
        #expect(rig.source.purchaseCalls == [trialProductID])
        #expect(try await rig.status().state == .trialActive(days_left: 7))
        #expect(rig.licence.screen == nil)
        #expect(rig.licence.chip?.label == "Trial ends in 7 days")
        #expect(rig.licence.chip?.ending == false)
        #expect(rig.licence.chip?.spoken == "Free trial. Ends in 7 days.")
        #expect(rig.licence.trialEndsAt == storeT0.addingTimeInterval(7 * storeDay))
        #expect(rig.keychain.stored?.contains("trial_start_ms") == true, "the Keychain record has the start")
    }

    @Test func daysLeftFollowTheMacsClockAndTheLastThreeShowAReminderOnHomeOrOverAProject() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        await rig.advance(days: 3)
        #expect(try await rig.status().state == .trialActive(days_left: 4))
        #expect(rig.licence.chip?.ending == false)
        #expect(rig.licence.screen == nil, "no reminder on days 1 to 4")

        await rig.advance(days: 4.5)
        #expect(try await rig.status().state == .trialActive(days_left: 3))
        #expect(rig.licence.chip?.ending == true)
        #expect(rig.licence.chip?.spoken == "Free trial. Ends in 3 days. Your trial ends soon.")
        // The reminder shows with no project open.
        #expect(rig.licence.screen == .reminder(daysLeft: 3))
        #expect(LicenceRules.reminderHeadline(3) == "Your free trial ends in 3 days")
        await rig.licence.closeReminder()
        #expect(rig.licence.screen == nil)
        // Once per calendar day, also after the model reads the core again.
        await rig.licence.refresh()
        #expect(rig.licence.screen == nil)
    }

    @Test func buyingOnAnyDayRemovesTheChipButtonAndReminderAtOnce() async throws {
        for days in [0.0, 3.0, 6.5] {
            let rig = try await StoreRig.start()
            await rig.startTrial()
            await rig.advance(days: days)
            await rig.licence.closeReminder()
            await rig.buyLifetime()
            #expect(try await rig.status().state == .purchased)
            #expect(rig.licence.chip == nil)
            #expect(rig.licence.screen == .dataNote)
            rig.finish()
        }
    }

    @Test func aTrialEndingWhileOpenFlushesPendingWorkBeforeTheLockAndKeepsItOnDisk() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        let project = try await rig.core.projectCreate(CreateProjectArgs(title: "Kept", projectType: .featureFilm))
        // The editor still holds text: it is stored while the lock is being set, not refused.
        let core = rig.core
        var flushed: Result<StoryPageSaved, Error>?
        rig.licence.flushBeforeLock = {
            do {
                flushed = .success(try await core.storyPageSave(StoryPageSaveArgs(text: "typed just before the trial ended", expectedRev: 0)))
            } catch {
                flushed = .failure(error)
            }
        }
        await rig.advance(days: 8)
        #expect(rig.licence.screen == .locked(revoked: false))
        if case let .failure(error)? = flushed { Issue.record("the save was refused: \(error)") }
        #expect(flushed != nil, "the flush ran before the lock screen")
        // From here the lock holds.
        await #expect(throws: CoreFailure.self) {
            _ = try await core.storyPageGet()
        }
        // Buying opens it again with the text there.
        await rig.buyLifetime()
        let page = try await core.storyPageGet()
        #expect(page.text == "typed just before the trial ended")
        _ = project
    }

    @Test func theLockAndTheCheckAndTheClockScreensOfferContactUsWithTheWebsiteAddress() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        var opened: [URL] = []
        rig.licence.openAddress = { opened.append($0) }
        rig.licence.openContact()
        #expect(opened.map(\.absoluteString) == ["https://straftstudio.com/contact/"])
    }

    @Test func aClockTurnedBackLocksAndClearsByItselfWithNoClick() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        await rig.advance(days: 6)
        #expect(try await rig.status().state == .trialActive(days_left: 1))
        await rig.licence.closeReminder()

        // Back by a week: the core looks by itself and locks once it is asked again.
        rig.clock.now = storeT0
        await rig.licence.restore()
        #expect(try await rig.status().state == .clockChanged)
        #expect(rig.licence.screen == .clockLocked)
        #expect(rig.licence.screen?.mandatory == true)
        #expect(rig.licence.chip == nil)
        await #expect(throws: CoreFailure.self) {
            _ = try await rig.core.projectCreate(CreateProjectArgs(title: "Blocked", projectType: .featureFilm))
        }
        // The clock is fixed: the 5 second look clears it, with no click.
        rig.clock.now = storeT0.addingTimeInterval(6 * storeDay)
        var state = try await rig.status().state
        for _ in 0..<400 where state == .clockChanged {
            try await Task.sleep(for: .milliseconds(25))
            state = try await rig.status().state
        }
        #expect(state == .trialActive(days_left: 1))
        await rig.licence.refresh()
        #expect(rig.licence.screen != .clockLocked)
    }

    @Test func aWrongFutureClockSinceFixedRecoversFromAFreshAppleDateOnCheckAgain() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        // The clock was a year ahead for one check, then fixed.
        await rig.advance(days: 365)
        rig.clock.now = storeT0.addingTimeInterval(storeDay)
        await rig.licence.restore()
        #expect(try await rig.status().state == .clockChanged)
        rig.source.syncSigned = rig.clock.now.addingTimeInterval(-60)
        await rig.licence.restore()
        #expect(try await rig.status().state == .trialActive(days_left: 6))
    }

    @Test func theTrialKeepsCountingOfflineAfterTheFirstStartAndABuyerStaysUnlocked() async throws {
        let keychain = FakeGuardKeychain()
        let rig = try await StoreRig.start(keychain: keychain)
        await rig.startTrial()
        rig.source.productsFail = true
        rig.source.entitlements = []
        await rig.advance(days: 2)
        #expect(try await rig.status().state == .trialActive(days_left: 5))
        await rig.advance(days: 7)
        #expect(try await rig.status().state == .trialEnded)
        rig.finish()

        // A buyer, a new launch, no network.
        let buyer = try await StoreRig.start()
        await buyer.buyLifetime()
        buyer.source.productsFail = true
        buyer.source.entitlements = []
        await buyer.licence.restore()
        #expect(try await buyer.status().state == .purchased)
        buyer.finish()
    }

    @Test func whenTheTrialEndsTheAppLocksUntilAPurchaseOrARestore() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial()
        await rig.advance(days: 8)
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
        let runtime = StoreRuntime(source: source, keychain: FakeGuardKeychain(), clock: { storeT0 })
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        runtime.install(into: &options)
        options.testStoreChannel = true
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
        #expect(rig.licence.message == "This purchase is waiting for approval. Straft Studio updates by itself once it is approved.")
        rig.source.queue(.success(.cancelled))
        await rig.licence.buy()
        #expect(try await rig.status().state == .trialNotStarted)
        #expect(rig.licence.screen == .macFirstLaunch)
    }

    @Test func aSecondTrialPurchaseNeverRestartsTheTrial() async throws {
        let rig = try await StoreRig.start()
        defer { rig.finish() }
        await rig.startTrial(at: storeT0, id: "2000000001")
        await rig.advance(days: 2)
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
        #expect(rig.licence.message == "Straft Studio could not reach the App Store. Connect to the internet, then try again.")
        #expect(try await rig.status().state == .trialNotStarted)
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
            LicenceRules.newLaptopInstallStep(.macAppStore), LicenceRules.buyCardText, LicenceRules.lockSafety,
            LicenceRules.clockTitle, LicenceRules.clockBody, LicenceRules.clockSafety, LicenceRules.contactLink,
            LicenceRules.trialDisclosure(price: ""), LicenceRules.lockBody(revoked: false, price: nil),
            LicenceRules.lockBody(revoked: true, price: nil),
        ] + LicenceRules.dataPoints.map(\.body) + LicenceRules.tutorialChecks
        for text in words {
            #expect(!text.lowercased().contains("subscription"))
        }
        #expect(!words.joined().contains("\u{2014}"))
    }

    /// Prices are never typed: a currency symbol, or digits next to a currency code, in the licence
    /// sources or their strings fails the build's tests.
    @Test func noCurrencyIsTypedInTheLicenceSources() throws {
        let store = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Sources/StraftUI/Store")
        let files = try FileManager.default.contentsOfDirectory(at: store, includingPropertiesForKeys: nil)
        #expect(files.count >= 7)
        let literal = #/"[^"\n]*"/#
        let price = #/[$€£₹¥]\s*\d|\d\s*[$€£₹¥]|\b(USD|EUR|INR|GBP|rupees?|dollars?|euros?)\b\s*\d|\d[\d.,]*\s*(USD|EUR|INR|GBP|rupees?|dollars?|euros?)\b/#
        for file in files where file.pathExtension == "swift" {
            let text = try String(contentsOf: file, encoding: .utf8)
            for match in text.matches(of: literal) {
                let quoted = String(text[match.range])
                #expect(quoted.firstMatch(of: price) == nil, "\(file.lastPathComponent) names a price: \(quoted)")
            }
        }
    }

}

@Suite("Licence rules")
struct LicenceRulesTests {
    @Test func theChipClampsToSevenDaysAndSpeaksTheLastDay() {
        #expect(LicenceRules.trialChip(.trialActive(days_left: 9))?.daysLeft == 7)
        #expect(LicenceRules.trialChip(.trialActive(days_left: 0.2))?.daysLeft == 1)
        #expect(LicenceRules.trialChip(.trialActive(days_left: 1))?.label == "Trial ends in 1 day")
        #expect(LicenceRules.trialChip(.trialActive(days_left: 2))?.label == "Trial ends in 2 days")
        #expect(LicenceRules.trialChip(.trialActive(days_left: 1))?.spoken == "Free trial, last day. Your trial ends within 24 hours.")
        for days in 1...7 {
            #expect(LicenceRules.trialChip(.trialActive(days_left: Double(days)))?.ending == (days <= 3))
        }
        #expect(LicenceRules.trialChip(.purchased) == nil)
        #expect(LicenceRules.trialChip(.trialEnded) == nil)
        #expect(LicenceRules.trialChip(nil) == nil)
    }

    @Test func theLastDayHeadlineSaysTodayOrTomorrowAndTheEndLineIsLocal() {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        let now = Date(timeIntervalSince1970: 1_791_244_800) // 05:30 on the 6th in India
        #expect(LicenceRules.reminderHeadline(1, endsAt: now.addingTimeInterval(3600), now: now, calendar: cal) == "Your free trial ends today")
        #expect(LicenceRules.reminderHeadline(1, endsAt: now.addingTimeInterval(20 * 3600), now: now, calendar: cal) == "Your free trial ends tomorrow")
        #expect(LicenceRules.reminderHeadline(2, endsAt: now, now: now, calendar: cal) == "Your free trial ends in 2 days")
        #expect(LicenceRules.reminderEndLine(now, calendar: cal) == "It ends on Tue 6 Oct at 5:30 AM.")
    }

    @Test func theLockWordsFollowThePackage() {
        #expect(LicenceRules.lockBody(revoked: false, price: fakeDisplayPrice).hasSuffix("One-time purchase of \(fakeDisplayPrice)."))
        #expect(LicenceRules.lockBody(revoked: false, price: nil) == "Buy the full version to keep using Straft Studio.")
        #expect(LicenceRules.lockBody(revoked: true, price: nil).contains("no longer shows a purchase"))
    }

    @Test func theEightOfficialAccountsHaveTheirExactAddresses() {
        #expect(SocialLinks.all.map(\.id) == ["youtube", "facebook", "instagram", "x", "threads", "linkedin", "reddit", "discord"])
        #expect(SocialLinks.all.map(\.url) == [
            "https://www.youtube.com/@StraftStudio",
            "https://www.facebook.com/straftstudio/",
            "https://www.instagram.com/straftstudio/",
            "https://x.com/straftstudio/",
            "https://www.threads.com/@straftstudio/",
            "https://www.linkedin.com/company/straftstudio/",
            "https://www.reddit.com/user/straftstudio/",
            "https://straftstudio.com/discord",
        ])
        for link in SocialLinks.all {
            #expect(link.openableURL?.absoluteString == link.url, "\(link.name) opens")
            #expect(!link.handle.isEmpty && !link.mark.isEmpty, "\(link.name) shows a handle and a mark")
            #expect(!SocialMark(commands: link.mark).path(in: CGRect(x: 0, y: 0, width: 24, height: 24)).isEmpty, "\(link.name) draws")
        }
    }

    @Test func theLicencesSheetNamesThePublisherAndReadsTheShippedFiles() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let notices = root.appendingPathComponent("ai/notices", isDirectory: true)
        try FileManager.default.createDirectory(at: notices, withIntermediateDirectories: true)
        try "list of crates".write(to: root.appendingPathComponent("THIRD_PARTY_LICENSES.md"), atomically: true, encoding: .utf8)
        try "Gemma terms".write(to: notices.appendingPathComponent("gemma-NOTICE.txt"), atomically: true, encoding: .utf8)
        let text = LicenceNotices.text(resources: root, engine: root.appendingPathComponent("ai", isDirectory: true))
        #expect(text.contains("Vighnajith Advisories Private Limited"))
        #expect(text.contains("list of crates"))
        #expect(text.contains("Gemma terms"))
        // A build without the files still names the publisher.
        #expect(LicenceNotices.text(resources: nil, engine: nil) == LicenceNotices.publisher)
    }

    @Test func theLocalDayUsesThePersonsOwnCalendar() {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        // 23:30 UTC on the 8th is already the 9th in India.
        let d = Date(timeIntervalSince1970: 1_791_244_800 + 3 * storeDay + 23.5 * 3600 - 86_400)
        #expect(LicenceRules.localDay(d, calendar: cal) == "2026-10-09")
    }

    @Test func onlyOfficialAddressesOpen() {
        #expect(!SocialLinks.isOfficial(nil))
        #expect(SocialLinks.isOfficial("https://www.instagram.com/straftstudio/"))
        #expect(!SocialLinks.isOfficial("http://www.instagram.com/straftstudio/"))
        #expect(!SocialLinks.isOfficial("https://evil.example.com/x"))
        #expect(!SocialLinks.isOfficial("https://user:pw@x.com/a"))
        #expect(!SocialLinks.isOfficial("https://x.com:8443/a"))
        // Discord only through the branded address, never an invite written into the app.
        #expect(!SocialLinks.isOfficial("https://discord.gg/AuQRHDpzP4"))
        #expect(SocialLinks.isOfficial("https://straftstudio.com/discord"))
    }
}

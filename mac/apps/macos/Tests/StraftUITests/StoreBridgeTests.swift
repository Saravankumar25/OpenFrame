// The StoreKit bridge on the Swift side (Sources/StraftUI/Store/StoreKitBridgeImpl.swift) over
// a fake Apple: the rules of crates/straft-entitlement/src/mac/bridge.rs, and the Keychain record.
// The bridge blocks, so the tests call it off the main thread, as the core does.

import Foundation
import Testing
@testable import StraftCore
@testable import StraftUI

private func json(_ text: String) -> [String: Any] {
    (try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any]) ?? [:]
}

private func off<T: Sendable>(_ work: @escaping @Sendable () -> T) async -> T {
    await Task.detached { work() }.value
}

@Suite("Store bridge", .serialized)
struct StoreBridgeTests {
    @Test func onlyVerifiedTransactionsCrossAndUnverifiedOnesAreCounted() async {
        let source = FakeStoreKitSource()
        source.entitlements = [
            .verified(fakeTransaction(lifetimeProductID, id: "2000000002")),
            .unverified,
            .unverified,
        ]
        let bridge = StoreKitBridgeImpl(source: source)
        let answer = json(await off { bridge.currentEntitlements() })
        #expect(answer["ok"] as? Bool == true)
        let txs = answer["transactions"] as? [[String: Any]] ?? []
        #expect(txs.count == 1)
        #expect(txs.first?["transactionId"] as? String == "2000000002")
        #expect(answer["unverified"] as? Int == 2)
        #expect(bridge.unverifiedDropped() == 2)
    }

    @Test func theTransactionCarriesApplesDatesAndTheEnvironment() async {
        let source = FakeStoreKitSource()
        let bought = Date(timeIntervalSince1970: 1_791_244_800.123)
        let refunded = Date(timeIntervalSince1970: 1_791_300_000)
        let tx = fakeTransaction(lifetimeProductID, id: "2000000002", at: bought, env: .sandbox, revokedAt: refunded, signedAt: refunded)
        source.entitlements = [.verified(tx)]
        let bridge = StoreKitBridgeImpl(source: source)
        let t = (json(await off { bridge.currentEntitlements() })["transactions"] as? [[String: Any]])?.first ?? [:]
        #expect(t["purchaseDateMs"] as? Int64 == 1_791_244_800_123)
        #expect(t["revocationDateMs"] as? Int64 == 1_791_300_000_000)
        #expect(t["environment"] as? String == "sandbox")
        #expect(t["originalPurchaseDateMs"] as? Int64 == 1_791_244_800_123)
        #expect(t["signedDateMs"] as? Int64 == 1_791_300_000_000)
        #expect(t["productId"] as? String == lifetimeProductID)
        #expect(t["appTransactionId"] as? String == "705000000001")
        // The wire shape is the core's `VerifiedTransaction`: it has no field for the Mac's own date.
        #expect(Set(t.keys) == [
            "productId", "transactionId", "originalTransactionId", "appTransactionId", "purchaseDateMs",
            "originalPurchaseDateMs", "signedDateMs", "revocationDateMs", "environment",
        ])
    }

    @Test func aMissingAppTransactionIdIsLeftOutAndNeverFilledIn() async {
        let source = FakeStoreKitSource()
        var tx = fakeTransaction(lifetimeProductID, id: "2000000002")
        tx.appTransactionId = nil
        source.entitlements = [.verified(tx)]
        let bridge = StoreKitBridgeImpl(source: source)
        let t = (json(await off { bridge.currentEntitlements() })["transactions"] as? [[String: Any]])?.first ?? [:]
        #expect(t["appTransactionId"] == nil)
        #expect(t["revocationDateMs"] == nil)
    }

    @Test func foreignProductsAndFamilySharedTransactionsAreSkipped() async {
        let source = FakeStoreKitSource()
        source.entitlements = [
            .verified(fakeTransaction("com.other.app.pro", id: "1")),
            .verified(fakeTransaction(lifetimeProductID, id: "2", shared: true)),
            .verified(fakeTransaction(trialProductID, id: "3")),
        ]
        let bridge = StoreKitBridgeImpl(source: source)
        let txs = json(await off { bridge.currentEntitlements() })["transactions"] as? [[String: Any]] ?? []
        #expect(txs.map { $0["transactionId"] as? String } == ["3"])
    }

    @Test func aPurchaseOfAnUnknownProductNeverReachesStoreKit() async {
        let source = FakeStoreKitSource()
        let bridge = StoreKitBridgeImpl(source: source)
        let answer = json(await off { bridge.purchase(productId: "com.other.app.pro") })
        #expect(answer["ok"] as? Bool == false)
        #expect(source.purchaseCalls.isEmpty)
    }

    @Test func purchaseOutcomesAreMappedAndAnUnverifiedPurchaseIsDropped() async {
        let source = FakeStoreKitSource()
        let bridge = StoreKitBridgeImpl(source: source)
        source.queue(.success(.success(.verified(fakeTransaction(trialProductID, id: "10")))))
        let ok = json(await off { bridge.purchase(productId: trialProductID) })
        #expect(ok["outcome"] as? String == "success")
        #expect((ok["transaction"] as? [String: Any])?["transactionId"] as? String == "10")

        source.queue(.success(.pending))
        #expect(json(await off { bridge.purchase(productId: trialProductID) })["outcome"] as? String == "pending")
        source.queue(.success(.cancelled))
        #expect(json(await off { bridge.purchase(productId: trialProductID) })["outcome"] as? String == "cancelled")

        source.queue(.success(.success(.unverified)))
        let bad = json(await off { bridge.purchase(productId: trialProductID) })
        #expect(bad["ok"] as? Bool == false)
        #expect(bad["error"] as? String == "unverified")
        #expect(bridge.unverifiedDropped() == 1)
        #expect(bad["transaction"] == nil)
    }

    @Test func aPurchaseAnswerForAnotherProductIsRefused() async {
        let source = FakeStoreKitSource()
        let bridge = StoreKitBridgeImpl(source: source)
        source.queue(.success(.success(.verified(fakeTransaction(lifetimeProductID, id: "11")))))
        let answer = json(await off { bridge.purchase(productId: trialProductID) })
        #expect(answer["ok"] as? Bool == false)
    }

    @Test func storeFailuresBecomeTheCodesTheCoreReads() async {
        let source = FakeStoreKitSource()
        let bridge = StoreKitBridgeImpl(source: source)
        for (failure, code) in [
            (StoreFailure.notSignedIn, "notSignedIn"), (.noConnection, "noConnection"), (.productNotFound, "productNotFound"),
            (.purchasesNotAllowed, "purchasesNotAllowed"), (.unavailable, "storeUnavailable"), (.other, "other"),
        ] {
            source.queue(.failure(failure))
            let answer = json(await off { bridge.purchase(productId: lifetimeProductID) })
            #expect(answer["ok"] as? Bool == false)
            #expect(answer["error"] as? String == code)
        }
    }

    @Test func theEnvironmentComesFromTheVerifiedAppTransactionAndHasNoDate() async {
        let source = FakeStoreKitSource()
        source.environment = .sandbox
        let bridge = StoreKitBridgeImpl(source: source)
        let env = json(await off { bridge.environment() })
        #expect(env["environment"] as? String == "sandbox")
        let app = json(await off { bridge.appTransaction() })
        #expect(app["environment"] as? String == "sandbox")
        #expect(app["appTransactionId"] as? String == "705000000001")
        // The date the app was first downloaded is not carried: no field for any date exists.
        #expect(Set(app.keys) == ["ok", "appTransactionId", "environment"])
    }

    @Test func anEnvironmentThatCannotBeReadIsAnError() async {
        let source = FakeStoreKitSource()
        source.appTransactionFails = true
        let bridge = StoreKitBridgeImpl(source: source)
        let env = json(await off { bridge.environment() })
        #expect(env["ok"] as? Bool == false)
    }

    @Test func restoreRunsAppStoreSyncAndPriceComesFromTheStorefront() async {
        let source = FakeStoreKitSource()
        let bridge = StoreKitBridgeImpl(source: source, keychain: FakeGuardKeychain())
        let signed = Date(timeIntervalSince1970: 1_791_300_000)
        source.syncSigned = signed
        let sync = json(await off { bridge.sync() })
        #expect(sync["ok"] as? Bool == true)
        #expect(sync["signedDateMs"] as? Int64 == 1_791_300_000_000)
        #expect(source.syncCalls == 1)
        let products = json(await off { bridge.loadProducts() })
        #expect(products["lifetimeDisplayPrice"] as? String == fakeDisplayPrice)
        // Fewer than both products loaded is the offline signal.
        source.productsFail = true
        let offline = json(await off { bridge.loadProducts() })
        #expect(offline["ok"] as? Bool == false)
        #expect(offline["error"] as? String == "noConnection")
    }

    @Test func theGuardRecordGoesToTheKeychainAndAFailureIsPlainNotACrash() async {
        let keychain = FakeGuardKeychain()
        let bridge = StoreKitBridgeImpl(source: FakeStoreKitSource(), keychain: keychain)
        #expect(json(bridge.loadGuard())["record"] == nil)
        #expect(json(bridge.saveGuard(recordJson: "{\"v\":1}"))["ok"] as? Bool == true)
        #expect(keychain.stored == "{\"v\":1}")
        #expect(json(bridge.loadGuard())["record"] as? String == "{\"v\":1}")
        keychain.fails = true
        #expect(json(bridge.loadGuard())["ok"] as? Bool == false)
        #expect(json(bridge.saveGuard(recordJson: "{}"))["ok"] as? Bool == false)
    }

    @Test func theWallClockIsTheInjectedOne() {
        let bridge = StoreKitBridgeImpl(source: FakeStoreKitSource(), keychain: FakeGuardKeychain(), clock: { storeT0 })
        #expect(bridge.wallClockMs() == 1_791_244_800_000)
    }

    @MainActor
    @Test func theMainThreadIsRefusedSoTheWindowCannotFreeze() {
        let bridge = StoreKitBridgeImpl(source: FakeStoreKitSource())
        let answer = json(bridge.currentEntitlements())
        #expect(answer["ok"] as? Bool == false)
    }

    @Test func storeKitEnvironmentNamesMapToThreeKnownOnes() {
        #expect(StoreEnvironmentName(storeKit: "Production") == .production)
        #expect(StoreEnvironmentName(storeKit: "Sandbox") == .sandbox)
        #expect(StoreEnvironmentName(storeKit: "Xcode") == .xcode)
        #expect(StoreEnvironmentName(storeKit: "LocalTesting") == .xcode)
        #expect(StoreEnvironmentName(storeKit: "Moon") == nil)
    }
}

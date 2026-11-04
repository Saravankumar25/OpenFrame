// Fakes for the Mac App Store tests: a fake Apple (StoreKit), a fake Keychain, a clock the test moves,
// and a rig that starts the real core as the store channel (the debug test switch). Real StoreKit
// purchases cannot run in this package: they need Xcode's StoreKit testing with
// packaging/StraftStudio.storekit.

import Foundation
import Testing
@testable import StraftCore
@testable import StraftUI

/// 2026-10-06 00:00 UTC.
let storeT0 = Date(timeIntervalSince1970: 1_791_244_800)
let storeDay: TimeInterval = 86_400

/// What StoreKit shows in place of a real price in these tests. Not a currency on purpose.
let fakeDisplayPrice = "{price from the App Store}"

func fakeTransaction(
    _ product: String, id: String, at: Date = storeT0, env: StoreEnvironmentName = .production,
    revokedAt: Date? = nil, shared: Bool = false, signedAt: Date? = nil
) -> StoreTransactionRecord {
    StoreTransactionRecord(
        productId: product, transactionId: id, originalTransactionId: id, appTransactionId: "705000000001",
        purchaseDate: at, signedDate: signedAt, revocationDate: revokedAt, environment: env, boughtByThisAccount: !shared
    )
}

/// A fake Apple. Tests set what StoreKit would answer.
final class FakeStoreKitSource: StoreKitSource, @unchecked Sendable {
    private let lock = NSLock()
    private var _entitlements: [StoreVerification<StoreTransactionRecord>] = []
    private var _purchases: [Result<StorePurchaseResult, StoreFailure>] = []
    private var _purchaseCalls: [String] = []
    private var _environment: StoreEnvironmentName = .production
    private var _appTransactionFails = false
    private var _productsFail = false
    private var _syncResult: [StoreVerification<StoreTransactionRecord>]?
    private var _syncSigned: Date?
    private var _syncCalls = 0
    private var _calls = 0
    private var continuation: AsyncStream<StoreUpdate>.Continuation?

    var entitlements: [StoreVerification<StoreTransactionRecord>] {
        get { lock.withLock { _entitlements } }
        set { lock.withLock { _entitlements = newValue } }
    }
    var environment: StoreEnvironmentName {
        get { lock.withLock { _environment } }
        set { lock.withLock { _environment = newValue } }
    }
    var appTransactionFails: Bool {
        get { lock.withLock { _appTransactionFails } }
        set { lock.withLock { _appTransactionFails = newValue } }
    }
    /// The App Store does not answer: `Product.products(for:)` fails, the offline signal.
    var productsFail: Bool {
        get { lock.withLock { _productsFail } }
        set { lock.withLock { _productsFail = newValue } }
    }
    /// What `AppStore.sync()` makes appear in the entitlements (a purchase from another Mac).
    var afterSync: [StoreVerification<StoreTransactionRecord>]? {
        get { lock.withLock { _syncResult } }
        set { lock.withLock { _syncResult = newValue } }
    }
    /// When Apple signed the record `AppStore.sync()` fetched.
    var syncSigned: Date? {
        get { lock.withLock { _syncSigned } }
        set { lock.withLock { _syncSigned = newValue } }
    }
    var purchaseCalls: [String] { lock.withLock { _purchaseCalls } }
    var syncCalls: Int { lock.withLock { _syncCalls } }
    /// Every call of any kind, to prove the internal build never asks StoreKit anything.
    var calls: Int { lock.withLock { _calls } }

    func queue(_ result: Result<StorePurchaseResult, StoreFailure>) { lock.withLock { _purchases.append(result) } }

    func pushUpdate(_ update: StoreUpdate) { lock.withLock { continuation }?.yield(update) }

    func currentEntitlements() async throws -> [StoreVerification<StoreTransactionRecord>] {
        lock.withLock { _calls += 1; return _entitlements }
    }

    func purchase(productId: String) async throws -> StorePurchaseResult {
        let next: Result<StorePurchaseResult, StoreFailure> = lock.withLock {
            _calls += 1
            _purchaseCalls.append(productId)
            return _purchases.isEmpty ? .success(.cancelled) : _purchases.removeFirst()
        }
        let result = try next.get()
        // A purchase shows up in the entitlements, as it does in StoreKit.
        if case let .success(.verified(t)) = result { lock.withLock { _entitlements.append(.verified(t)) } }
        return result
    }

    func appTransaction() async throws -> StoreVerification<StoreAppRecord> {
        let (env, fails) = lock.withLock { _calls += 1; return (_environment, _appTransactionFails) }
        if fails { throw StoreFailure.unavailable }
        return .verified(StoreAppRecord(appTransactionId: "705000000001", environment: env))
    }

    func lifetimeDisplayPrice() async throws -> String {
        let fails = lock.withLock { _calls += 1; return _productsFail }
        if fails { throw StoreFailure.noConnection }
        return fakeDisplayPrice
    }

    func sync() async throws -> Date? {
        lock.withLock {
            _calls += 1
            _syncCalls += 1
            if let list = _syncResult { _entitlements = list; _syncResult = nil }
            return _syncSigned
        }
    }

    func updates() -> AsyncStream<StoreUpdate> {
        AsyncStream { c in lock.withLock { continuation = c } }
    }
}

/// The guard record in memory, so a test can read it and start a new launch over it.
final class FakeGuardKeychain: GuardKeychain, @unchecked Sendable {
    private let lock = NSLock()
    private var text: String?
    var fails = false

    init(_ text: String? = nil) { self.text = text }

    var stored: String? { lock.withLock { text } }

    func load() throws -> String? {
        if fails { throw KeychainFailure(status: -34018) }
        return lock.withLock { text }
    }

    func save(_ text: String) throws {
        if fails { throw KeychainFailure(status: -34018) }
        lock.withLock { self.text = text }
    }
}

/// The Mac's clock, which the test moves.
final class FakeWallClock: @unchecked Sendable {
    private let lock = NSLock()
    private var _now: Date

    init(_ now: Date) { _now = now }

    var now: Date {
        get { lock.withLock { _now } }
        set { lock.withLock { _now = newValue } }
    }
}

/// The real core started as the Mac App Store channel over the fakes.
@MainActor
struct StoreRig {
    let root: URL
    let core: Core
    let source: FakeStoreKitSource
    let keychain: FakeGuardKeychain
    let clock: FakeWallClock
    let runtime: StoreRuntime
    let licence: LicenceModel

    /// `store: false` starts the internal build (no test switch) with the same fakes handed over.
    static func start(store: Bool = true, now: Date = storeT0, keychain: FakeGuardKeychain = FakeGuardKeychain()) async throws -> StoreRig {
        let root = try Fixture.root()
        let source = FakeStoreKitSource()
        let clock = FakeWallClock(now)
        let runtime = StoreRuntime(source: source, keychain: keychain, clock: { clock.now })
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        runtime.install(into: &options)
        options.testStoreChannel = store
        let core = try await Core.start(options)
        let licence = LicenceModel()
        licence.today = { "2026-10-09" }
        await licence.attach(core: core, runtime: runtime, observeSystem: false)
        let rig = StoreRig(root: root, core: core, source: source, keychain: keychain, clock: clock, runtime: runtime, licence: licence)
        if store { await rig.settle() }
        return rig
    }

    /// Wait for the core's launch check, then read the licence state.
    func settle() async {
        for _ in 0..<300 {
            if let s = try? await core.entitlementStatus(), s.state != .needsOnlineCheck { break }
            try? await Task.sleep(for: .milliseconds(20))
        }
        await licence.refresh()
    }

    func status() async throws -> EntitlementStatus { try await core.entitlementStatus() }

    /// Move the Mac's clock to `days` after the trial start and let the core look again.
    func advance(days: Double) async {
        clock.now = storeT0.addingTimeInterval(days * storeDay)
        await licence.restore()
    }

    func finish() {
        licence.detach()
        core.stop()
        try? FileManager.default.removeItem(at: root)
    }
}

extension StoreRig {
    /// Start the trial: Apple records the purchase at `at`.
    func startTrial(at: Date = storeT0, id: String = "2000000001") async {
        source.queue(.success(.success(.verified(fakeTransaction(trialProductID, id: id, at: at)))))
        await licence.startTrial()
    }

    func buyLifetime(id: String = "2000000002") async {
        source.queue(.success(.success(.verified(fakeTransaction(lifetimeProductID, id: id)))))
        await licence.buy()
    }
}

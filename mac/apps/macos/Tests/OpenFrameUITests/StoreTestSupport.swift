// Fakes for the Mac App Store tests: a fake Apple (StoreKit), a fake licensing server that signs
// its answers with a real Ed25519 key, and a rig that starts the real core as the store channel
// (the debug test switch). Real StoreKit purchases cannot run in this package: they need Xcode's
// StoreKit testing with packaging/OpenFrameStudio.storekit.

import CryptoKit
import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

/// 2026-10-06 00:00 UTC.
let storeT0 = Date(timeIntervalSince1970: 1_791_244_800)
let storeDay: TimeInterval = 86_400

private func b64url(_ data: Data) -> String {
    data.base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_")
        .replacingOccurrences(of: "=", with: "")
}

private func fromB64url(_ text: String) -> Data? {
    var s = text.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
    while s.count % 4 != 0 { s += "=" }
    return Data(base64Encoded: s)
}

/// A stand-in for Apple's signed transaction: three base64url parts, the middle one readable by
/// the fake server.
func fakeJWS(_ t: StoreTransactionRecord) -> String {
    var body: [String: Any] = [
        "transactionId": t.transactionId, "productId": t.productId, "environment": t.environment.rawValue,
        "purchaseDateMs": Int64(t.purchaseDate.timeIntervalSince1970 * 1000),
    ]
    if let r = t.revocationDate { body["revocationDateMs"] = Int64(r.timeIntervalSince1970 * 1000) }
    let data = try! JSONSerialization.data(withJSONObject: body, options: [.sortedKeys])
    return "\(b64url(Data("{\"alg\":\"ES256\"}".utf8))).\(b64url(data)).\(b64url(Data("sig".utf8)))"
}

func fakeTransaction(
    _ product: String, id: String, at: Date = storeT0, env: StoreEnvironmentName = .production,
    revokedAt: Date? = nil, shared: Bool = false
) -> StoreTransactionRecord {
    var t = StoreTransactionRecord(
        productId: product, transactionId: id, originalTransactionId: id, appTransactionId: "705000000001",
        purchaseDate: at, revocationDate: revokedAt, environment: env, boughtByThisAccount: !shared, signedJws: ""
    )
    t.signedJws = fakeJWS(t)
    return t
}

/// A fake Apple. Tests set what StoreKit would answer.
final class FakeStoreKitSource: StoreKitSource, @unchecked Sendable {
    private let lock = NSLock()
    private var _entitlements: [StoreVerification<StoreTransactionRecord>] = []
    private var _purchases: [Result<StorePurchaseResult, StoreFailure>] = []
    private var _purchaseCalls: [String] = []
    private var _environment: StoreEnvironmentName = .production
    private var _appTransactionFails = false
    private var _syncResult: [StoreVerification<StoreTransactionRecord>]?
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
    /// What `AppStore.sync()` makes appear in the entitlements (a purchase from another Mac).
    var afterSync: [StoreVerification<StoreTransactionRecord>]? {
        get { lock.withLock { _syncResult } }
        set { lock.withLock { _syncResult = newValue } }
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
        return .verified(StoreAppRecord(appTransactionId: "705000000001", environment: env, signedJws: "aGVhZGVy.YXBw.c2ln"))
    }

    func displayPrice(productId: String) async throws -> String {
        lock.withLock { _calls += 1 }
        return "$89.00"
    }

    func sync() async throws {
        lock.withLock {
            _calls += 1
            _syncCalls += 1
            if let list = _syncResult { _entitlements = list; _syncResult = nil }
        }
    }

    func updates() -> AsyncStream<StoreUpdate> {
        AsyncStream { c in lock.withLock { continuation = c } }
    }
}

/// A fake licensing server. It counts the trial from Apple's purchase date in the JWS plus 168
/// hours on its own clock (`now`), and signs with a real key.
final class FakeLicenceServer: LicenceServerCallbacks, @unchecked Sendable {
    enum Mode { case honest, offline, down, wrongKey }

    private let lock = NSLock()
    private let key = Curve25519.Signing.PrivateKey()
    private let wrongKey = Curve25519.Signing.PrivateKey()
    private var _now: Date
    private var _mode = Mode.honest
    private var _requests: [[String: Any]] = []

    init(now: Date = storeT0) { _now = now }

    var publicKeyHex: String { key.publicKey.rawRepresentation.map { String(format: "%02x", $0) }.joined() }
    var now: Date {
        get { lock.withLock { _now } }
        set { lock.withLock { _now = newValue } }
    }
    var mode: Mode {
        get { lock.withLock { _mode } }
        set { lock.withLock { _mode = newValue } }
    }
    var requests: [[String: Any]] { lock.withLock { _requests } }

    func validate(requestJson: String) -> String {
        guard let req = try? JSONSerialization.jsonObject(with: Data(requestJson.utf8)) as? [String: Any] else {
            return #"{"ok":false,"error":"rejected"}"#
        }
        let (now, mode) = lock.withLock { _requests.append(req); return (_now, _mode) }
        switch mode {
        case .offline: return #"{"ok":false,"error":"noConnection"}"#
        case .down: return #"{"ok":false,"error":"unavailable"}"#
        case .honest, .wrongKey: break
        }
        let env = req["environment"] as? String ?? ""
        var trials: [(id: String, ms: Int64)] = []
        var revoked = false
        for jws in (req["transactionJws"] as? [String]) ?? [] {
            let parts = jws.split(separator: ".")
            guard parts.count == 3, let data = fromB64url(String(parts[1])),
                  let p = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  (p["environment"] as? String) == env
            else { continue }
            if p["revocationDateMs"] != nil { revoked = true; continue }
            if (p["productId"] as? String) == trialProductID, let ms = (p["purchaseDateMs"] as? NSNumber)?.int64Value {
                trials.append((p["transactionId"] as? String ?? "", ms))
            }
        }
        let earliest = trials.min { $0.ms < $1.ms }
        let nowMs = Int64(now.timeIntervalSince1970 * 1000)
        var trial: Any = NSNull()
        if let earliest {
            let ends = earliest.ms + 168 * 3_600_000
            let remaining = max(0, (ends - nowMs) / 1000)
            trial = [
                "trialEndsAtMs": ends, "remainingSeconds": remaining, "appleTransactionId": earliest.id,
                "leaseSeconds": min(remaining, 3600),
            ] as [String: Any]
        }
        let body: [String: Any] = [
            "schema": 1, "nonce": req["nonce"] as? String ?? "", "environment": env, "serverTimeMs": nowMs,
            "trial": trial, "lifetime": false, "revoked": revoked,
        ]
        let payload = try! JSONSerialization.data(withJSONObject: body, options: [.sortedKeys])
        let signer = mode == .wrongKey ? wrongKey : key
        let signature = try! signer.signature(for: Data("openframe-mac-licence-response-v1\0".utf8) + payload)
        return "{\"payload\":\"\(b64url(payload))\",\"signature\":\"\(b64url(signature))\"}"
    }
}

/// The real core started as the Mac App Store channel over the fakes.
@MainActor
struct StoreRig {
    let root: URL
    let core: Core
    let source: FakeStoreKitSource
    let server: FakeLicenceServer
    let runtime: StoreRuntime
    let licence: LicenceModel

    /// `store: false` starts the internal build (no test switch) with the same fakes handed over.
    static func start(store: Bool = true, now: Date = storeT0) async throws -> StoreRig {
        let root = try Fixture.root()
        let source = FakeStoreKitSource()
        let server = FakeLicenceServer(now: now)
        let runtime = StoreRuntime(source: source, server: server)
        var options = CoreStartOptions(appVersion: "0.1.0-test", isolatedDataRoot: root, writeLogs: false)
        runtime.install(into: &options)
        if store { options.testStoreResponseKeyHex = server.publicKeyHex }
        let core = try await Core.start(options)
        let licence = LicenceModel()
        licence.today = { "2026-10-09" }
        await licence.attach(core: core, runtime: runtime, observeSystem: false)
        let rig = StoreRig(root: root, core: core, source: source, server: server, runtime: runtime, licence: licence)
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

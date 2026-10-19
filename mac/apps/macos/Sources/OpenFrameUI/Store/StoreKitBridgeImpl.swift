// The Swift side of the StoreKit bridge (crates/openframe-entitlement/src/mac/bridge.rs). The core
// calls these methods from its own threads and gets JSON text back; this class makes no licence
// decision of its own.
//
// The bridge contract, kept here and checked again by the core:
// 1. Only `.verified` results cross. Every `.unverified` result is dropped and counted.
// 2. Apple's dates (purchase and revocation) in Unix milliseconds UTC, never the Mac's clock.
// 3. The signed JWS travels with every transaction, for the licensing server.
// 4. The environment is reported on every transaction, so test and real purchases never mix.
// 5. `appTransactionID` is optional (macOS 15.4 and later) and never a date. The date the app was
//    first downloaded (`AppTransaction.originalPurchaseDate`) is not read at all.
// Only the two OpenFrame products pass, and only transactions bought by this Apple Account
// (Family Sharing is off).

import Foundation
import OpenFrameCore

public final class StoreKitBridgeImpl: StoreKitCallbacks, @unchecked Sendable {
    private let source: StoreKitSource
    private let lock = NSLock()
    private var unverified: UInt64 = 0
    private var knownEnvironment: StoreEnvironmentName?

    /// How long a query may take before the bridge answers "store unavailable".
    static let queryTimeout: TimeInterval = 30
    /// The payment sheet waits for a person, so it gets longer.
    static let purchaseTimeout: TimeInterval = 600

    public init(source: StoreKitSource = LiveStoreKitSource()) {
        self.source = source
    }

    // MARK: StoreKitCallbacks

    public func currentEntitlements() -> String {
        blocking(timeout: Self.queryTimeout) { [self] in
            do {
                var transactions: [WireTransaction] = []
                for result in try await source.currentEntitlements() {
                    switch result {
                    case let .verified(t):
                        // Not ours: another product, or shared through Family Sharing.
                        guard Self.isOurs(t) else { continue }
                        transactions.append(WireTransaction(t))
                    case .unverified:
                        countUnverified()
                    }
                }
                return Self.encode(EntitlementsAnswer(transactions: transactions, unverified: droppedCount))
            } catch {
                return Self.failure(error)
            }
        }
    }

    public func purchase(productId: String) -> String {
        guard Self.knownProducts.contains(productId) else { return Self.fail("productNotFound") }
        return blocking(timeout: Self.purchaseTimeout) { [self] in
            do {
                switch try await source.purchase(productId: productId) {
                case let .success(result):
                    switch result {
                    case let .verified(t):
                        guard t.productId == productId, Self.isOurs(t) else { return Self.fail("unverified") }
                        return Self.encode(PurchaseAnswer(outcome: "success", transaction: WireTransaction(t)))
                    case .unverified:
                        countUnverified()
                        return Self.fail("unverified")
                    }
                case .pending:
                    return Self.encode(PurchaseAnswer(outcome: "pending", transaction: nil))
                case .cancelled:
                    return Self.encode(PurchaseAnswer(outcome: "cancelled", transaction: nil))
                }
            } catch {
                return Self.failure(error)
            }
        }
    }

    public func appTransaction() -> String {
        blocking(timeout: Self.queryTimeout) { [self] in
            do {
                switch try await source.appTransaction() {
                case let .verified(app):
                    remember(app.environment)
                    return Self.encode(AppAnswer(appTransactionId: app.appTransactionId, environment: app.environment.rawValue, signedJws: app.signedJws))
                case .unverified:
                    countUnverified()
                    return Self.fail("unverified")
                }
            } catch {
                return Self.failure(error)
            }
        }
    }

    /// The environment of this build, from the verified `AppTransaction`. When it cannot be read
    /// the core assumes Production, so transactions from another environment are refused.
    public func environment() -> String {
        if let known = lock.withLock({ knownEnvironment }) {
            return Self.encode(EnvironmentAnswer(environment: known.rawValue))
        }
        return blocking(timeout: Self.queryTimeout) { [self] in
            do {
                guard case let .verified(app) = try await source.appTransaction() else { return Self.fail("unverified") }
                remember(app.environment)
                return Self.encode(EnvironmentAnswer(environment: app.environment.rawValue))
            } catch {
                return Self.failure(error)
            }
        }
    }

    public func productInfo(productId: String) -> String {
        guard Self.knownProducts.contains(productId) else { return Self.fail("productNotFound") }
        return blocking(timeout: Self.queryTimeout) { [self] in
            do {
                let price = try await source.displayPrice(productId: productId)
                return Self.encode(ProductAnswer(productId: productId, displayPrice: price))
            } catch {
                return Self.failure(error)
            }
        }
    }

    public func sync() -> String {
        blocking(timeout: Self.queryTimeout) { [self] in
            do {
                try await source.sync()
                return Self.encode(OkAnswer())
            } catch {
                return Self.failure(error)
            }
        }
    }

    public func unverifiedDropped() -> UInt64 { droppedCount }

    /// Called by the updates listener for an `.unverified` update.
    func countUnverified() {
        lock.withLock { unverified += 1 }
    }

    private var droppedCount: UInt64 { lock.withLock { unverified } }

    private func remember(_ env: StoreEnvironmentName) {
        lock.withLock { knownEnvironment = env }
    }

    static let knownProducts: Set<String> = [trialProductID, lifetimeProductID]

    private static func isOurs(_ t: StoreTransactionRecord) -> Bool {
        knownProducts.contains(t.productId) && t.boughtByThisAccount
    }

    // MARK: Wire shapes (the field names are the contract with bridge.rs)

    private struct WireTransaction: Encodable {
        var productId: String
        var transactionId: String
        var originalTransactionId: String
        var appTransactionId: String?
        var purchaseDateMs: Int64
        var revocationDateMs: Int64?
        var environment: String
        var signedJws: String

        init(_ t: StoreTransactionRecord) {
            productId = t.productId
            transactionId = t.transactionId
            originalTransactionId = t.originalTransactionId
            appTransactionId = t.appTransactionId.flatMap { $0.isEmpty ? nil : $0 }
            purchaseDateMs = StoreKitBridgeImpl.unixMs(t.purchaseDate)
            revocationDateMs = t.revocationDate.map(StoreKitBridgeImpl.unixMs)
            environment = t.environment.rawValue
            signedJws = t.signedJws
        }
    }

    private struct EntitlementsAnswer: Encodable {
        let ok = true
        var transactions: [WireTransaction]
        var unverified: UInt64
    }

    private struct PurchaseAnswer: Encodable {
        let ok = true
        var outcome: String
        var transaction: WireTransaction?
    }

    private struct AppAnswer: Encodable {
        let ok = true
        var appTransactionId: String?
        var environment: String
        var signedJws: String
    }

    private struct EnvironmentAnswer: Encodable {
        let ok = true
        var environment: String
    }

    private struct ProductAnswer: Encodable {
        let ok = true
        var productId: String
        var displayPrice: String
    }

    private struct OkAnswer: Encodable { let ok = true }

    private struct ErrorAnswer: Encodable {
        let ok = false
        var error: String
    }

    /// Unix milliseconds. A conversion for the server, never a comparison with the Mac's clock.
    static func unixMs(_ date: Date) -> Int64 {
        Int64((date.timeIntervalSince1970 * 1000).rounded())
    }

    private static func encode<T: Encodable>(_ value: T) -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        guard let data = try? encoder.encode(value), let text = String(data: data, encoding: .utf8) else {
            return fail("other")
        }
        return text
    }

    private static func fail(_ code: String) -> String {
        encode(ErrorAnswer(error: code))
    }

    private static func failure(_ error: Error) -> String {
        fail((error as? StoreFailure)?.code ?? "other")
    }

    /// Runs async StoreKit work and waits for it on the calling thread. The core calls from its
    /// own threads; the main thread is refused, so the window can never freeze on the App Store.
    private func blocking(timeout: TimeInterval, _ work: @escaping @Sendable () async -> String) -> String {
        if Thread.isMainThread { return Self.fail("mainThread") }
        let done = DispatchSemaphore(value: 0)
        let box = ResultBox()
        Task.detached {
            box.set(await work())
            done.signal()
        }
        if done.wait(timeout: .now() + timeout) == .timedOut { return Self.fail("storeUnavailable") }
        return box.get() ?? Self.fail("other")
    }
}

private final class ResultBox: @unchecked Sendable {
    private let lock = NSLock()
    private var value: String?
    func set(_ text: String) { lock.withLock { value = text } }
    func get() -> String? { lock.withLock { value } }
}

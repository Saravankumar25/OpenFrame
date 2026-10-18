// The StoreKit 2 calls the Mac App Store build needs, behind one small seam.
//
// `LiveStoreKitSource` is the only code that touches StoreKit. Everything above it
// (`StoreKitBridgeImpl`) works on plain values, so the bridge rules (verified results only,
// Apple's dates, the environment, the signed JWS) are tested with a fake source here, and the
// real StoreKit purchase flow is tested in Xcode with StoreKit testing and
// `packaging/OpenFrameStudio.storekit`.

import Foundation
import StoreKit

/// The zero-price non-consumable that starts the 7-day trial.
public let trialProductID = "com.openframe.studio.trial7"
/// The paid non-consumable that unlocks the app for good.
public let lifetimeProductID = "com.openframe.studio.lifetime"

/// Where a transaction was made. Test purchases and real purchases are never mixed.
public enum StoreEnvironmentName: String, Sendable, Codable, Equatable {
    case sandbox, production, xcode

    /// StoreKit's `AppStore.Environment.rawValue` ("Production", "Sandbox", "Xcode", and
    /// "LocalTesting" in newer systems, which is Xcode's local testing).
    init?(storeKit raw: String) {
        switch raw.lowercased() {
        case "production": self = .production
        case "sandbox": self = .sandbox
        case "xcode", "localtesting": self = .xcode
        default: return nil
        }
    }
}

/// One transaction StoreKit verified. No decision is made from it here.
public struct StoreTransactionRecord: Sendable, Equatable {
    public var productId: String
    public var transactionId: String
    public var originalTransactionId: String
    /// Apple's `appTransactionID`, from macOS 15.4 only.
    public var appTransactionId: String?
    /// Apple's purchase date, never the Mac's clock.
    public var purchaseDate: Date
    public var revocationDate: Date?
    public var environment: StoreEnvironmentName
    /// False for a transaction shared through Family Sharing (not used).
    public var boughtByThisAccount: Bool
    /// The signed transaction exactly as StoreKit returned it.
    public var signedJws: String

    public init(
        productId: String, transactionId: String, originalTransactionId: String,
        appTransactionId: String? = nil, purchaseDate: Date, revocationDate: Date? = nil,
        environment: StoreEnvironmentName, boughtByThisAccount: Bool = true, signedJws: String
    ) {
        self.productId = productId
        self.transactionId = transactionId
        self.originalTransactionId = originalTransactionId
        self.appTransactionId = appTransactionId
        self.purchaseDate = purchaseDate
        self.revocationDate = revocationDate
        self.environment = environment
        self.boughtByThisAccount = boughtByThisAccount
        self.signedJws = signedJws
    }
}

/// `AppTransaction.shared`, verified. It carries no date on purpose: the date the app was first
/// downloaded is not the trial start.
public struct StoreAppRecord: Sendable, Equatable {
    public var appTransactionId: String?
    public var environment: StoreEnvironmentName
    public var signedJws: String

    public init(appTransactionId: String? = nil, environment: StoreEnvironmentName, signedJws: String) {
        self.appTransactionId = appTransactionId
        self.environment = environment
        self.signedJws = signedJws
    }
}

/// StoreKit's `VerificationResult`: only `.verified` ever leaves the bridge.
public enum StoreVerification<Value: Sendable>: Sendable {
    case verified(Value)
    case unverified
}

public enum StorePurchaseResult: Sendable {
    case success(StoreVerification<StoreTransactionRecord>)
    /// Ask to Buy or a payment step. Nothing is granted.
    case pending
    case cancelled
}

/// Why StoreKit could not answer. Carries no user data.
public enum StoreFailure: Error, Sendable, Equatable {
    case notSignedIn, noConnection, unavailable, productNotFound, purchasesNotAllowed, other

    /// The code the Rust side reads (`BridgeError`).
    var code: String {
        switch self {
        case .notSignedIn: "notSignedIn"
        case .noConnection: "noConnection"
        case .unavailable: "storeUnavailable"
        case .productNotFound: "productNotFound"
        case .purchasesNotAllowed: "purchasesNotAllowed"
        case .other: "other"
        }
    }
}

/// A change StoreKit reported while the app runs.
public enum StoreUpdate: Sendable, Equatable {
    /// A verified transaction (a purchase, a refund or a revocation).
    case transaction
    case unverified
    /// The App Store storefront changed, which is how another Apple Account signing in shows up.
    case storefront
}

public protocol StoreKitSource: Sendable {
    /// `Transaction.currentEntitlements`, every result.
    func currentEntitlements() async throws -> [StoreVerification<StoreTransactionRecord>]
    /// `Product.purchase()` for one product.
    func purchase(productId: String) async throws -> StorePurchaseResult
    /// `AppTransaction.shared`.
    func appTransaction() async throws -> StoreVerification<StoreAppRecord>
    /// `Product.displayPrice`, formatted for the storefront.
    func displayPrice(productId: String) async throws -> String
    /// `AppStore.sync()`.
    func sync() async throws
    /// `Transaction.updates` and `Storefront.updates`.
    func updates() -> AsyncStream<StoreUpdate>
}

// MARK: Live StoreKit

/// The real StoreKit 2 calls. Not run by the package tests: StoreKit needs a signed app with the
/// bundle identifier from App Store Connect, or Xcode's StoreKit testing.
public struct LiveStoreKitSource: StoreKitSource {
    public init() {}

    public func currentEntitlements() async throws -> [StoreVerification<StoreTransactionRecord>] {
        let appId = await Self.appTransactionId()
        var out: [StoreVerification<StoreTransactionRecord>] = []
        for await result in Transaction.currentEntitlements {
            switch result {
            case let .verified(t):
                if let record = Self.record(t, jws: result.jwsRepresentation, appTransactionId: appId) {
                    out.append(.verified(record))
                } else {
                    out.append(.unverified)
                }
            case .unverified:
                out.append(.unverified)
            }
        }
        return out
    }

    public func purchase(productId: String) async throws -> StorePurchaseResult {
        guard AppStore.canMakePayments else { throw StoreFailure.purchasesNotAllowed }
        let product: Product
        do {
            guard let found = try await Product.products(for: [productId]).first else {
                throw StoreFailure.productNotFound
            }
            product = found
        } catch let failure as StoreFailure {
            throw failure
        } catch {
            throw Self.failure(error)
        }
        do {
            switch try await product.purchase() {
            case let .success(result):
                switch result {
                case let .verified(t):
                    // Finishing tells StoreKit the transaction was received.
                    await t.finish()
                    let appId = await Self.appTransactionId()
                    guard let record = Self.record(t, jws: result.jwsRepresentation, appTransactionId: appId) else {
                        return .success(.unverified)
                    }
                    return .success(.verified(record))
                case .unverified:
                    return .success(.unverified)
                }
            case .pending: return .pending
            case .userCancelled: return .cancelled
            @unknown default: throw StoreFailure.other
            }
        } catch let failure as StoreFailure {
            throw failure
        } catch Product.PurchaseError.purchaseNotAllowed {
            throw StoreFailure.purchasesNotAllowed
        } catch {
            throw Self.failure(error)
        }
    }

    public func appTransaction() async throws -> StoreVerification<StoreAppRecord> {
        do {
            let result = try await AppTransaction.shared
            switch result {
            case let .verified(app):
                guard let env = StoreEnvironmentName(storeKit: app.environment.rawValue) else { throw StoreFailure.other }
                var id: String?
                if #available(macOS 15.4, *) { id = app.appTransactionID }
                return .verified(StoreAppRecord(
                    appTransactionId: id.flatMap { $0.isEmpty ? nil : $0 },
                    environment: env, signedJws: result.jwsRepresentation
                ))
            case .unverified:
                return .unverified
            }
        } catch let failure as StoreFailure {
            throw failure
        } catch {
            throw Self.failure(error)
        }
    }

    public func displayPrice(productId: String) async throws -> String {
        do {
            guard let product = try await Product.products(for: [productId]).first else { throw StoreFailure.productNotFound }
            return product.displayPrice
        } catch let failure as StoreFailure {
            throw failure
        } catch {
            throw Self.failure(error)
        }
    }

    public func sync() async throws {
        do { try await AppStore.sync() } catch { throw Self.failure(error) }
    }

    public func updates() -> AsyncStream<StoreUpdate> {
        AsyncStream { continuation in
            let transactions = Task.detached {
                for await result in Transaction.updates {
                    switch result {
                    case let .verified(t):
                        // A revoked transaction is forwarded too: the core decides what it means.
                        await t.finish()
                        continuation.yield(.transaction)
                    case .unverified:
                        continuation.yield(.unverified)
                    }
                }
            }
            let storefronts = Task.detached {
                for await _ in Storefront.updates { continuation.yield(.storefront) }
            }
            continuation.onTermination = { _ in
                transactions.cancel()
                storefronts.cancel()
            }
        }
    }

    // MARK: Mapping

    private static func appTransactionId() async -> String? {
        guard #available(macOS 15.4, *) else { return nil }
        guard let result = try? await AppTransaction.shared, case let .verified(app) = result else { return nil }
        return app.appTransactionID.isEmpty ? nil : app.appTransactionID
    }

    /// The transaction in the bridge's shape, or `nil` when its environment is not one of the
    /// three known ones.
    private static func record(_ t: Transaction, jws: String, appTransactionId: String?) -> StoreTransactionRecord? {
        guard let env = StoreEnvironmentName(storeKit: t.environment.rawValue) else { return nil }
        return StoreTransactionRecord(
            productId: t.productID, transactionId: String(t.id), originalTransactionId: String(t.originalID),
            appTransactionId: appTransactionId, purchaseDate: t.purchaseDate, revocationDate: t.revocationDate,
            environment: env, boughtByThisAccount: t.ownershipType == .purchased, signedJws: jws
        )
    }

    private static func failure(_ error: Error) -> StoreFailure {
        if let store = error as? StoreKitError {
            switch store {
            case .networkError: return .noConnection
            case .notAvailableInStorefront, .systemError: return .unavailable
            case .userCancelled: return .other
            default: return .unavailable
            }
        }
        if let url = error as? URLError, url.code == .notConnectedToInternet || url.code == .timedOut { return .noConnection }
        return .other
    }
}

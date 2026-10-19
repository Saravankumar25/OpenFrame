// StoreKit 2 bridge for the OpenFrame Studio Mac App Store build.
//
// NOT COMPILED YET. Written on a Windows machine with no Xcode and no Apple account. build.rs
// compiles it for every macOS build of the app, and the Mac workflow (.github/workflows/mac.yml)
// is the first place that happens. It has never run against StoreKit.
//
// Rules this file keeps (docs/engineering/38-mac-app-store-trial.md):
// - StoreKit 2 only. No receipt file, no SKPaymentQueue.
// - Every VerificationResult is matched. Only `.verified` transactions are serialised; unverified
//   ones are counted and never passed on.
// - No entitlement decision and no comparison with the current date. The licence server decides
//   from Apple's signed dates and its own UTC clock.
// - Family Sharing is not used: shared transactions are skipped and counted.
// - Only the two OpenFrame product ids are accepted.
// - Each exported function returns a strdup'd JSON string that the caller frees with
//   storekit_free. Calls block, so they refuse to run on the main thread.

import Foundation
import StoreKit

private let knownProductIDs: Set<String> = [
    "com.openframe.studio.trial7",
    "com.openframe.studio.lifetime",
]
private let maxArgumentBytes = 4096
private let queryTimeout: TimeInterval = 30
private let purchaseTimeout: TimeInterval = 600

// MARK: - JSON shapes (field names are the contract with platform/mod.rs)

private struct TransactionJSON: Encodable {
    let productID: String
    let id: String
    let originalID: String
    let appTransactionID: String?
    let purchaseDateMs: Int64
    let revocationDateMs: Int64?
    let environment: String
    let jwsRepresentation: String
}

private struct EntitlementsJSON: Encodable {
    let ok = true
    let transactions: [TransactionJSON]
    let unverifiedCount: Int
    let skippedCount: Int
}

private struct PurchaseJSON: Encodable {
    let ok = true
    let outcome: String
    let transaction: TransactionJSON?
}

private struct AppTransactionJSON: Encodable {
    let ok = true
    let verified: Bool
    let appTransactionID: String?
    let environment: String?
    let jwsRepresentation: String?
}

private struct EnvironmentJSON: Encodable {
    let ok = true
    let environment: String
}

private struct ProductJSON: Encodable {
    let id: String
    let displayName: String
    let displayPrice: String
}

private struct ProductsJSON: Encodable {
    let ok = true
    let products: [ProductJSON]
}

private struct UpdateJSON: Encodable {
    let kind: String
    let transaction: TransactionJSON?
}

private struct ErrorJSON: Encodable {
    let ok = false
    let error: String
}

// MARK: - Helpers

private func encode<T: Encodable>(_ value: T) -> String {
    guard let data = try? JSONEncoder().encode(value),
          let text = String(data: data, encoding: .utf8)
    else {
        return #"{"ok":false,"error":"encoding_failed"}"#
    }
    return text
}

private func failure(_ code: String) -> String {
    encode(ErrorJSON(error: code))
}

private func cString(_ text: String) -> UnsafeMutablePointer<CChar>? {
    strdup(text)
}

/// Unix milliseconds. A conversion for the server, never a comparison.
private func unixMs(_ date: Date) -> Int64 {
    Int64((date.timeIntervalSince1970 * 1000).rounded())
}

/// A C string argument of at most `maxArgumentBytes` bytes, valid UTF-8.
private func boundedString(_ pointer: UnsafePointer<CChar>?) -> String? {
    guard let pointer else { return nil }
    let length = strnlen(pointer, maxArgumentBytes + 1)
    guard length <= maxArgumentBytes else { return nil }
    let bytes = UnsafeRawBufferPointer(start: pointer, count: length)
    return String(bytes: bytes, encoding: .utf8)
}

private final class ResultBox: @unchecked Sendable {
    private let lock = NSLock()
    private var value: String?

    func set(_ newValue: String) {
        lock.lock()
        value = newValue
        lock.unlock()
    }

    func get() -> String? {
        lock.lock()
        defer { lock.unlock() }
        return value
    }
}

/// Run async StoreKit work and wait for it on the calling (background) thread.
private func runBlocking(timeout: TimeInterval, _ work: @escaping @Sendable () async -> String) -> String {
    if Thread.isMainThread {
        return failure("main_thread")
    }
    let done = DispatchSemaphore(value: 0)
    let box = ResultBox()
    Task.detached {
        box.set(await work())
        done.signal()
    }
    if done.wait(timeout: .now() + timeout) == .timedOut {
        return failure("timeout")
    }
    return box.get() ?? failure("store_error")
}

/// The app transaction id, from the verified AppTransaction (macOS 15.4 and later).
private func currentAppTransactionID() async -> String? {
    guard #available(macOS 15.4, *) else { return nil }
    guard let result = try? await AppTransaction.shared,
          case .verified(let app) = result
    else {
        return nil
    }
    return app.appTransactionID
}

private func serialize(_ transaction: Transaction, jws: String, appTransactionID: String?) -> TransactionJSON {
    TransactionJSON(
        productID: transaction.productID,
        id: String(transaction.id),
        originalID: String(transaction.originalID),
        appTransactionID: appTransactionID,
        purchaseDateMs: unixMs(transaction.purchaseDate),
        revocationDateMs: transaction.revocationDate.map(unixMs),
        environment: transaction.environment.rawValue,
        jwsRepresentation: jws
    )
}

/// Ours to use: one of the two products, bought by this Apple ID (not shared through Family Sharing).
private func isUsable(_ transaction: Transaction) -> Bool {
    knownProductIDs.contains(transaction.productID) && transaction.ownershipType == .purchased
}

// MARK: - Exported C functions

@_cdecl("storekit_current_entitlements")
public func storekit_current_entitlements() -> UnsafeMutablePointer<CChar>? {
    cString(runBlocking(timeout: queryTimeout) {
        let appID = await currentAppTransactionID()
        var transactions: [TransactionJSON] = []
        var unverified = 0
        var skipped = 0
        for await result in Transaction.currentEntitlements {
            switch result {
            case .verified(let transaction):
                if isUsable(transaction) {
                    transactions.append(serialize(transaction, jws: result.jwsRepresentation, appTransactionID: appID))
                } else {
                    skipped += 1
                }
            case .unverified:
                unverified += 1
            }
        }
        return encode(EntitlementsJSON(transactions: transactions, unverifiedCount: unverified, skippedCount: skipped))
    })
}

@_cdecl("storekit_purchase")
public func storekit_purchase(_ productID: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    guard let id = boundedString(productID), knownProductIDs.contains(id) else {
        return cString(failure("bad_argument"))
    }
    return cString(runBlocking(timeout: purchaseTimeout) {
        do {
            guard let product = try await Product.products(for: [id]).first else {
                return failure("not_found")
            }
            switch try await product.purchase() {
            case .success(let result):
                switch result {
                case .verified(let transaction):
                    await transaction.finish()
                    let appID = await currentAppTransactionID()
                    let json = serialize(transaction, jws: result.jwsRepresentation, appTransactionID: appID)
                    return encode(PurchaseJSON(outcome: "purchased", transaction: json))
                case .unverified:
                    return encode(PurchaseJSON(outcome: "unverified", transaction: nil))
                }
            case .pending:
                return encode(PurchaseJSON(outcome: "pending", transaction: nil))
            case .userCancelled:
                return encode(PurchaseJSON(outcome: "cancelled", transaction: nil))
            @unknown default:
                return encode(PurchaseJSON(outcome: "unknown", transaction: nil))
            }
        } catch {
            return failure("store_error")
        }
    })
}

@_cdecl("storekit_app_transaction")
public func storekit_app_transaction() -> UnsafeMutablePointer<CChar>? {
    cString(runBlocking(timeout: queryTimeout) {
        do {
            let result = try await AppTransaction.shared
            switch result {
            case .verified(let app):
                var appID: String?
                if #available(macOS 15.4, *) {
                    appID = app.appTransactionID
                }
                return encode(AppTransactionJSON(
                    verified: true,
                    appTransactionID: appID,
                    environment: app.environment.rawValue,
                    jwsRepresentation: result.jwsRepresentation
                ))
            case .unverified:
                return encode(AppTransactionJSON(
                    verified: false, appTransactionID: nil, environment: nil, jwsRepresentation: nil
                ))
            }
        } catch {
            return failure("store_error")
        }
    })
}

@_cdecl("storekit_environment")
public func storekit_environment() -> UnsafeMutablePointer<CChar>? {
    cString(runBlocking(timeout: queryTimeout) {
        do {
            guard case .verified(let app) = try await AppTransaction.shared else {
                return failure("unverified")
            }
            return encode(EnvironmentJSON(environment: app.environment.rawValue))
        } catch {
            return failure("store_error")
        }
    })
}

@_cdecl("storekit_products")
public func storekit_products(_ productIDsJSON: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    guard let text = boundedString(productIDsJSON),
          let ids = try? JSONDecoder().decode([String].self, from: Data(text.utf8)),
          ids.count <= knownProductIDs.count
    else {
        return cString(failure("bad_argument"))
    }
    let wanted = ids.filter { knownProductIDs.contains($0) }
    return cString(runBlocking(timeout: queryTimeout) {
        do {
            let products = try await Product.products(for: wanted)
            let json = products.map { ProductJSON(id: $0.id, displayName: $0.displayName, displayPrice: $0.displayPrice) }
            return encode(ProductsJSON(products: json))
        } catch {
            return failure("store_error")
        }
    })
}

private let updatesLock = NSLock()
private var updatesTask: Task<Void, Never>?
private var storefrontTask: Task<Void, Never>?

@_cdecl("storekit_start_updates")
public func storekit_start_updates(_ callback: (@convention(c) (UnsafePointer<CChar>?) -> Void)?) -> Int32 {
    guard let callback else { return 0 }
    updatesLock.lock()
    defer { updatesLock.unlock() }
    if updatesTask != nil { return 1 }
    updatesTask = Task.detached {
        for await result in Transaction.updates {
            let json: String
            switch result {
            case .verified(let transaction):
                // Finishing tells StoreKit the update was received. A revoked transaction is
                // forwarded with its revocation date; the server and the app decide what it means.
                await transaction.finish()
                guard isUsable(transaction) else { continue }
                let appID = await currentAppTransactionID()
                let body = serialize(transaction, jws: result.jwsRepresentation, appTransactionID: appID)
                json = encode(UpdateJSON(kind: "transaction", transaction: body))
            case .unverified:
                json = encode(UpdateJSON(kind: "unverified", transaction: nil))
            }
            json.withCString { callback($0) }
        }
    }
    // macOS has no notice for "another Apple Account signed in to the App Store". A storefront
    // change is the closest one; the app checks its entitlements again when it arrives.
    storefrontTask = Task.detached {
        for await _ in Storefront.updates {
            encode(UpdateJSON(kind: "storefront", transaction: nil)).withCString { callback($0) }
        }
    }
    return 1
}

@_cdecl("storekit_free")
public func storekit_free(_ pointer: UnsafeMutablePointer<CChar>?) {
    free(pointer)
}

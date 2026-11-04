// A stand-in for the App Store in a debug build only, for the end-to-end scenarios of the store screens.
// It reads what Apple would answer, and the Mac's clock, from a small file the scenario writes in its
// test folder. Real StoreKit needs a signed app with the App Store Connect identifier, which a test
// run does not have. Never compiled into a release build.

#if DEBUG
import Foundation

public final class ScriptedStoreKitSource: StoreKitSource, @unchecked Sendable {
    public struct Row: Codable, Sendable {
        public var product: String
        public var id: String
        public var atMs: Double
        public var revokedAtMs: Double?
    }

    public struct State: Codable, Sendable {
        /// How far the Mac's clock is from the real one.
        public var clockOffsetMs: Double = 0
        public var productsFail = false
        public var rows: [Row] = []
        /// What the next purchase ends with: success, pending or cancelled.
        public var nextPurchase = "success"
        /// When Apple signed what `AppStore.sync()` fetched: the real time, so a clock that is wrong
        /// shows up against it.
        public var syncSignsWithRealTime = true
    }

    private let file: URL
    private let lock = NSLock()
    private var lastSeen: Data?

    public init(file: URL) {
        self.file = file
    }

    public func read() -> State {
        lock.withLock {
            guard let data = try? Data(contentsOf: file), let state = try? JSONDecoder().decode(State.self, from: data) else {
                return State()
            }
            return state
        }
    }

    private func write(_ state: State) {
        lock.withLock {
            guard let data = try? JSONEncoder().encode(state) else { return }
            try? data.write(to: file, options: .atomic)
            lastSeen = data
        }
    }

    /// The Mac's clock as the scenario set it.
    public func now() -> Date { Date().addingTimeInterval(read().clockOffsetMs / 1000) }

    private func record(_ row: Row) -> StoreTransactionRecord {
        let at = Date(timeIntervalSince1970: row.atMs / 1000)
        return StoreTransactionRecord(
            productId: row.product, transactionId: row.id, originalTransactionId: row.id, purchaseDate: at,
            signedDate: at, revocationDate: row.revokedAtMs.map { Date(timeIntervalSince1970: $0 / 1000) },
            environment: .production
        )
    }

    public func currentEntitlements() async throws -> [StoreVerification<StoreTransactionRecord>] {
        read().rows.map { .verified(record($0)) }
    }

    public func purchase(productId: String) async throws -> StorePurchaseResult {
        var state = read()
        switch state.nextPurchase {
        case "pending": return .pending
        case "cancelled": return .cancelled
        default: break
        }
        let row = Row(product: productId, id: "\(2_000_000_000 + state.rows.count + 1)", atMs: now().timeIntervalSince1970 * 1000)
        state.rows.append(row)
        write(state)
        return .success(.verified(record(row)))
    }

    public func appTransaction() async throws -> StoreVerification<StoreAppRecord> {
        .verified(StoreAppRecord(environment: .production))
    }

    public func lifetimeDisplayPrice() async throws -> String {
        if read().productsFail { throw StoreFailure.noConnection }
        return "{price from the App Store}"
    }

    public func sync() async throws -> Date? {
        let state = read()
        if state.productsFail { throw StoreFailure.noConnection }
        return state.syncSignsWithRealTime ? Date() : nil
    }

    /// A change the scenario made to the file arrives as a StoreKit update.
    public func updates() -> AsyncStream<StoreUpdate> {
        AsyncStream { continuation in
            let task = Task.detached { [self] in
                while !Task.isCancelled {
                    try? await Task.sleep(for: .milliseconds(300))
                    let data = try? Data(contentsOf: file)
                    let changed = lock.withLock {
                        let changed = data != lastSeen
                        lastSeen = data
                        return changed
                    }
                    if changed { continuation.yield(.transaction) }
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }
}

/// The Keychain in the test folder: a debug app has no signed access group to use the real one.
public struct FileGuardKeychain: GuardKeychain {
    let file: URL
    public init(file: URL) { self.file = file }
    public func load() throws -> String? { try? String(contentsOf: file, encoding: .utf8) }
    public func save(_ text: String) throws { try text.write(to: file, atomically: true, encoding: .utf8) }
}
#endif

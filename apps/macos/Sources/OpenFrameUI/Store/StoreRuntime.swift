// What the Mac App Store build hands to the core: the StoreKit 2 bridge, the licensing server
// client and the listener for `Transaction.updates`. The internal disk image build gets the same
// objects and the core ignores them (the channel comes from the build), so they are never called.

import Foundation
import OpenFrameCore

public final class StoreRuntime: @unchecked Sendable {
    public let source: StoreKitSource
    public let bridge: StoreKitBridgeImpl
    public let server: LicenceServerCallbacks
    private let lock = NSLock()
    private var listener: Task<Void, Never>?

    public init(source: StoreKitSource = LiveStoreKitSource(), server: LicenceServerCallbacks = HTTPLicenceServer()) {
        self.source = source
        self.bridge = StoreKitBridgeImpl(source: source)
        self.server = server
    }

    /// Hand the bridge and the server client to the core.
    public func install(into options: inout CoreStartOptions) {
        options.storeKit = bridge
        options.licenceServer = server
    }

    /// Watch `Transaction.updates` and the storefront: each change asks the core for a new check
    /// (a purchase, a refund, a revocation, another Apple Account). Started by the licence model
    /// only in a store build.
    func startListening(onChange: @escaping @Sendable () -> Void) {
        lock.withLock {
            guard listener == nil else { return }
            let updates = source.updates()
            let bridge = bridge
            listener = Task.detached {
                for await update in updates {
                    if update == .unverified { bridge.countUnverified() }
                    onChange()
                }
            }
        }
    }

    func stopListening() {
        lock.withLock {
            listener?.cancel()
            listener = nil
        }
    }
}

// The running Rust core, as the Swift side of the single door (ADR-0004, ADR-0027).
//
// Every call goes through `invokeRaw` (an operation name and JSON arguments) on a background
// queue, so the main thread never waits for the core. Events arrive on core threads and are
// fanned out to every `events()` stream; screens consume them on the main actor.

import Foundation

/// A coded error from the core. `message` is safe to show; `detail` is for diagnostics.
public struct CoreFailure: Error, LocalizedError, Sendable, Equatable {
    public let code: String
    public let message: String
    public let detail: String?
    public let retryable: Bool

    public init(code: String, message: String, detail: String? = nil, retryable: Bool = false) {
        self.code = code
        self.message = message
        self.detail = detail
        self.retryable = retryable
    }

    public var errorDescription: String? { message }

    init(_ error: Error) {
        if let failure = error as? CoreFailure {
            self = failure
        } else if case let CoreError.Failed(code, message, detail, retryable) = error {
            self.init(code: code, message: message, detail: detail, retryable: retryable)
        } else {
            self.init(
                code: "internal.unexpected",
                message: "Something went wrong inside OpenFrame. Your project was not changed.",
                detail: String(describing: error)
            )
        }
    }
}

/// How the app starts the core.
public struct CoreStartOptions: Sendable {
    public var appVersion: String
    /// A test run keeps every file in this absolute folder (ADR-0031). `nil`: the person's data.
    public var isolatedDataRoot: URL?
    /// `Contents/Resources/ai` inside the app bundle.
    public var aiBundleDir: URL?
    public var writeLogs: Bool
    /// The password screen tests' hash. Honoured by the core only in a debug build on a test
    /// folder (ADR-0031); ignored everywhere else.
    public var testTeamPasswordHash: String?
    /// The Mac App Store build's StoreKit 2 object. The core uses it in the store channel only;
    /// the internal disk image build ignores it.
    public var storeKit: (any StoreKitCallbacks)?
    /// The Mac App Store build's client for the licensing server (store channel only).
    public var licenceServer: (any LicenceServerCallbacks)?
    /// The store screen tests: run as the store channel with this licence response public key
    /// (64 hex characters). Honoured by the core only in a debug build on a test folder.
    public var testStoreResponseKeyHex: String?
    /// The first-start library setup tests: the test folder starts like a new computer. Honoured
    /// by the core only in a debug build on a test folder.
    public var testLibrarySetup = false
    /// Temperature 0 and a fixed seed for the Offline AI in the automated tests. Honoured by the
    /// core only in a debug build on a test folder.
    public var testAIDeterministic = false

    public init(appVersion: String, isolatedDataRoot: URL? = nil, aiBundleDir: URL? = nil, writeLogs: Bool = true) {
        self.appVersion = appVersion
        self.isolatedDataRoot = isolatedDataRoot
        self.aiBundleDir = aiBundleDir
        self.writeLogs = writeLogs
    }
}

/// Fans core events out to any number of async streams.
final class EventHub: EventListener, @unchecked Sendable {
    private let lock = NSLock()
    private var continuations: [UUID: AsyncStream<AppEvent>.Continuation] = [:]
    private let decoder = JSONDecoder()

    func onEvent(eventJson: String) {
        guard let data = eventJson.data(using: .utf8),
              let event = try? decoder.decode(AppEvent.self, from: data)
        else { return }
        lock.lock()
        let targets = Array(continuations.values)
        lock.unlock()
        for continuation in targets {
            continuation.yield(event)
        }
    }

    func stream() -> AsyncStream<AppEvent> {
        let id = UUID()
        return AsyncStream(bufferingPolicy: .bufferingNewest(512)) { continuation in
            lock.lock()
            continuations[id] = continuation
            lock.unlock()
            continuation.onTermination = { [weak self] _ in
                guard let self else { return }
                self.lock.lock()
                self.continuations[id] = nil
                self.lock.unlock()
            }
        }
    }

    func finishAll() {
        lock.lock()
        let targets = Array(continuations.values)
        continuations.removeAll()
        lock.unlock()
        for continuation in targets {
            continuation.finish()
        }
    }
}

/// The running core. One per process.
public final class Core: CoreInvoking, @unchecked Sendable {
    private let handle: CoreHandle
    private let hub: EventHub
    private let queue = DispatchQueue(label: "studio.openframe.core", qos: .userInitiated, attributes: .concurrent)

    private init(handle: CoreHandle, hub: EventHub) {
        self.handle = handle
        self.hub = hub
    }

    /// Start the core off the main thread.
    public static func start(_ options: CoreStartOptions) async throws -> Core {
        let hub = EventHub()
        let config = CoreConfig(
            appVersion: options.appVersion,
            isolatedDataRoot: options.isolatedDataRoot?.path,
            aiBundleDir: options.aiBundleDir?.path,
            writeLogs: options.writeLogs,
            testTeamPasswordHash: options.testTeamPasswordHash,
            testStoreResponseKeyHex: options.testStoreResponseKeyHex,
            testLibrarySetup: options.testLibrarySetup,
            testAiDeterministic: options.testAIDeterministic
        )
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    let handle = try CoreHandle.start(config: config, listener: hub, storekit: options.storeKit, licenceServer: options.licenceServer)
                    continuation.resume(returning: Core(handle: handle, hub: hub))
                } catch {
                    continuation.resume(throwing: CoreFailure(error))
                }
            }
        }
    }

    public func invokeRaw(_ op: String, _ argsJSON: String) async throws -> String {
        let handle = self.handle
        return try await withCheckedThrowingContinuation { continuation in
            queue.async {
                do {
                    continuation.resume(returning: try handle.invoke(op: op, argsJson: argsJSON))
                } catch {
                    continuation.resume(throwing: CoreFailure(error))
                }
            }
        }
    }

    /// A new stream of every event from now on.
    public func events() -> AsyncStream<AppEvent> { hub.stream() }

    /// The IPC allow-list, sorted.
    public var operationNames: [String] { handle.operationNames() }

    public func cancelTask(_ taskId: String) -> Bool { handle.cancelTask(taskId: taskId) }

    public func powerEvent(sleeping: Bool) {
        let handle = self.handle
        queue.async { handle.powerEvent(sleeping: sleeping) }
    }

    /// Ask for a licence and trial check in the store channel: `launch`, `resume`, `storeKitChanged`
    /// or `leaseExpired`. Returns at once; the answer arrives as an `entitlement` module event.
    /// Does nothing in the internal build.
    public func requestStoreCheck(_ reason: String) {
        let handle = self.handle
        queue.async { handle.requestStoreCheck(reason: reason) }
    }

    public func openAsset(_ assetId: String, global: Bool = false, reveal: Bool = false) async throws {
        try await run { try $0.openAsset(assetId: assetId, global: global, reveal: reveal) }
    }

    public func assetFile(_ assetId: String, global: Bool = false) async throws -> URL {
        let path = try await run { try $0.assetFile(assetId: assetId, global: global) }
        return URL(fileURLWithPath: path)
    }

    public func revealLocation(_ kind: String, id: String? = nil, path: String? = nil) async throws {
        try await run { try $0.revealLocation(kind: kind, id: id, path: path) }
    }

    public func openURL(_ url: String) async throws {
        try await run { try $0.openUrl(url: url) }
    }

    /// Close the project cleanly and stop background tasks. Blocks until done, so the app can
    /// call it from `applicationWillTerminate`.
    public func stop() {
        handle.stop()
        hub.finishAll()
    }

    public static var version: String { coreVersion() }

    private func run<T: Sendable>(_ body: @escaping @Sendable (CoreHandle) throws -> T) async throws -> T {
        let handle = self.handle
        return try await withCheckedThrowingContinuation { continuation in
            queue.async {
                do {
                    continuation.resume(returning: try body(handle))
                } catch {
                    continuation.resume(throwing: CoreFailure(error))
                }
            }
        }
    }
}

// The HTTPS call to OpenFrame's licensing server during a trial check
// (crates/openframe-entitlement/src/mac/server.rs). The request carries Apple's signed
// transactions and a random nonce, nothing from the person's projects. The core checks the
// signature, nonce and environment of the answer; this class only moves bytes.
//
// Network use: only the two addresses below, over HTTPS, a 10 second timeout, no redirects, and a
// 64 KiB answer cap. Xcode test transactions are never sent anywhere.

import Foundation
import OpenFrameCore

/// The production licensing server. PLACEHOLDER: the owner replaces it with the deployed address
/// (docs/engineering/38-mac-app-store-trial.md). A `.invalid` host never resolves.
public let licenceServerURL = "https://licence.openframe.invalid/v1/mac/validate"
/// The sandbox deployment for TestFlight, App Review and sandbox testers. PLACEHOLDER as above.
public let licenceServerSandboxURL = "https://licence-sandbox.openframe.invalid/v1/mac/validate"

/// One POST, blocking. Tests replace it.
public protocol LicenceTransport: Sendable {
    /// The HTTP status and body, or a thrown error for no connection.
    func post(url: URL, body: Data) throws -> (status: Int, body: Data)
}

public struct URLSessionTransport: LicenceTransport {
    static let maxAnswerBytes = 64 * 1024

    public init() {}

    public func post(url: URL, body: Data) throws -> (status: Int, body: Data) {
        let delegate = NoRedirects()
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 10
        config.timeoutIntervalForResource = 10
        config.httpCookieStorage = nil
        config.urlCache = nil
        config.waitsForConnectivity = false
        let session = URLSession(configuration: config, delegate: delegate, delegateQueue: nil)
        defer { session.finishTasksAndInvalidate() }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("OpenFrameStudio/\(AppVersion.current)", forHTTPHeaderField: "User-Agent")

        let done = DispatchSemaphore(value: 0)
        let box = AnswerBox()
        let task = session.dataTask(with: request) { data, response, error in
            box.set(data: data, status: (response as? HTTPURLResponse)?.statusCode, error: error)
            done.signal()
        }
        task.resume()
        if done.wait(timeout: .now() + 12) == .timedOut {
            task.cancel()
            throw URLError(.timedOut)
        }
        let answer = box.get()
        if let error = answer.error { throw error }
        return (answer.status ?? 0, answer.data ?? Data())
    }

    private final class NoRedirects: NSObject, URLSessionTaskDelegate {
        func urlSession(
            _ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
            newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void
        ) {
            completionHandler(nil)
        }
    }

    private final class AnswerBox: @unchecked Sendable {
        private let lock = NSLock()
        private var data: Data?
        private var status: Int?
        private var error: Error?
        func set(data: Data?, status: Int?, error: Error?) {
            lock.withLock { self.data = data; self.status = status; self.error = error }
        }
        func get() -> (data: Data?, status: Int?, error: Error?) {
            lock.withLock { (data, status, error) }
        }
    }
}

public final class HTTPLicenceServer: LicenceServerCallbacks, @unchecked Sendable {
    private let transport: LicenceTransport
    private let production: String
    private let sandbox: String

    public init(
        transport: LicenceTransport = URLSessionTransport(),
        productionURL: String = licenceServerURL, sandboxURL: String = licenceServerSandboxURL
    ) {
        self.transport = transport
        self.production = productionURL
        self.sandbox = sandboxURL
    }

    /// The address for an environment, or `nil` when none may be used: Xcode transactions are
    /// never sent, a plain `http` address is refused, and the placeholder host never resolves.
    func endpoint(environment: String) -> URL? {
        let raw: String
        switch environment {
        case "production": raw = production
        case "sandbox": raw = sandbox
        default: return nil
        }
        guard let url = URL(string: raw), url.scheme == "https", let host = url.host, !host.hasSuffix(".invalid") else {
            return nil
        }
        return url
    }

    public func validate(requestJson: String) -> String {
        guard let data = requestJson.data(using: .utf8), data.count <= 64 * 1024,
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let environment = object["environment"] as? String
        else { return Self.fail("rejected") }
        if environment == "xcode" { return Self.fail("rejected") }
        guard let url = endpoint(environment: environment) else { return Self.fail("unavailable") }
        let answer: (status: Int, body: Data)
        do {
            answer = try transport.post(url: url, body: data)
        } catch {
            return Self.fail("noConnection")
        }
        guard (200..<300).contains(answer.status) else {
            switch answer.status {
            case 408, 429: return Self.fail("unavailable")
            case 400..<500: return Self.fail("rejected")
            default: return Self.fail("unavailable")
            }
        }
        guard answer.body.count <= URLSessionTransport.maxAnswerBytes, let text = String(data: answer.body, encoding: .utf8) else {
            return Self.fail("unavailable")
        }
        return text
    }

    private static func fail(_ code: String) -> String {
        #"{"ok":false,"error":"\#(code)"}"#
    }
}

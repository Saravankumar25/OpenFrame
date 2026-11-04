import Foundation

/// The seam between generated typed operations and the Rust core. The real implementation calls
/// the UniFFI bridge; tests supply a stub.
public protocol CoreInvoking: Sendable {
    func invokeRaw(_ op: String, _ argsJSON: String) async throws -> String
}

public enum CoreCallError: Error, Sendable {
    case encodingFailed
}

extension CoreInvoking {
    /// Encodes `args` as JSON, invokes the operation and decodes the result.
    /// A result of `null` (or nothing) decodes to `EmptyResult`.
    public func call<A: Encodable, R: Decodable>(_ op: String, _ args: A) async throws -> R {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.withoutEscapingSlashes, .sortedKeys]
        let data = try encoder.encode(args)
        guard let argsJSON = String(data: data, encoding: .utf8) else { throw CoreCallError.encodingFailed }
        let out = try await invokeRaw(op, argsJSON)
        let trimmed = out.trimmingCharacters(in: .whitespacesAndNewlines)
        if R.self == EmptyResult.self, trimmed == "null" || trimmed.isEmpty, let empty = EmptyResult() as? R {
            return empty
        }
        return try JSONDecoder().decode(R.self, from: Data(trimmed.utf8))
    }
}

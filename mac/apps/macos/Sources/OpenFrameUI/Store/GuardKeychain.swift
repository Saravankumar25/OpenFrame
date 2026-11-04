// The clock guard record in the data protection Keychain (ADR-0040, rules G1 to G4). A file in the
// container is easy to restore from a backup; a Keychain item survives deleting and reinstalling the
// app. Swift owns the Keychain; the core reads and writes the record through the StoreKit bridge and
// tolerates anything that comes back. Only the Mac App Store build ever calls this.

import Foundation
import Security

/// Never change this after the first release: a new group cannot read the old record.
/// `REPLACE_WITH_TEAM_ID` comes from the owner's Apple Developer account, Membership page. A store
/// or release build fails its placeholder check while it remains (scripts/check-release-placeholders.mjs).
public let keychainAccessGroup = "REPLACE_WITH_TEAM_ID.app.openframestudio.studio"

public protocol GuardKeychain: Sendable {
    /// The stored text, `nil` when there is none yet.
    func load() throws -> String?
    func save(_ text: String) throws
}

public struct KeychainFailure: Error, Equatable {
    public let status: OSStatus
}

public struct LiveGuardKeychain: GuardKeychain {
    private static let service = "app.openframestudio.studio.trial-guard"
    private static let account = "guard"

    public init() {}

    private func query() -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.service,
            kSecAttrAccount as String: Self.account,
            kSecAttrAccessGroup as String: keychainAccessGroup,
            kSecUseDataProtectionKeychain as String: true,
            kSecAttrSynchronizable as String: false,
        ]
    }

    public func load() throws -> String? {
        var q = query()
        q[kSecReturnData as String] = true
        q[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(q as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw KeychainFailure(status: status) }
        return (item as? Data).flatMap { String(data: $0, encoding: .utf8) }
    }

    public func save(_ text: String) throws {
        let data = Data(text.utf8)
        let update = SecItemUpdate(query() as CFDictionary, [kSecValueData as String: data] as CFDictionary)
        if update == errSecSuccess { return }
        guard update == errSecItemNotFound else { throw KeychainFailure(status: update) }
        var add = query()
        add[kSecValueData as String] = data
        add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        let status = SecItemAdd(add as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainFailure(status: status) }
    }
}

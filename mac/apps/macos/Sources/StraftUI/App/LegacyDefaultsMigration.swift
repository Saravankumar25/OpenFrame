// Preferences of an earlier install: copied once, because the bundle identifier and key names changed
// with the product name. The old domains are only read.

import Foundation

enum LegacyDefaultsMigration {
    static let doneKey = "straft.legacyDefaultsMigrated"
    static let legacyDomains = [
        "app.openframestudio.studio", // LEGACY: Open Frame name kept for migration
        "studio.openframe.mac", // LEGACY: Open Frame name kept for migration
    ]
    static let legacyPrefix = "openframe." // LEGACY: Open Frame name kept for migration

    /// Only the installed app holds a person's preferences: test copies have their own identifiers,
    /// and the sandboxed store build cannot read other domains.
    static func shouldRun(environment: [String: String], bundleId: String?) -> Bool {
        guard bundleId == "app.straftstudio.studio" || bundleId == "app.straftstudio.studio.dev" else { return false }
        return environment["STRAFT_DATA_ROOT"] == nil && environment["APP_SANDBOX_CONTAINER_ID"] == nil
    }

    static func runAtLaunch() {
        if shouldRun(environment: ProcessInfo.processInfo.environment, bundleId: Bundle.main.bundleIdentifier) {
            migrate(into: .standard)
        }
    }

    /// Copies each old value that has no value yet under its new name, bookmark data included. A
    /// missing domain reads as nil. A persistent domain holds no global keys.
    static func migrate(
        into defaults: UserDefaults,
        domains: [String] = legacyDomains,
        read: (String) -> [String: Any]? = { UserDefaults.standard.persistentDomain(forName: $0) }
    ) {
        guard !defaults.bool(forKey: doneKey) else { return }
        for domain in domains {
            for (oldKey, value) in read(domain) ?? [:] {
                let key = oldKey.hasPrefix(legacyPrefix) ? "straft." + oldKey.dropFirst(legacyPrefix.count) : oldKey
                if defaults.object(forKey: key) == nil { defaults.set(value, forKey: key) }
            }
        }
        defaults.set(true, forKey: doneKey)
    }
}

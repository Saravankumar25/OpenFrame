// Preferences of an earlier install come along once, under their new names, and never in a test run.

import Foundation
import Testing
@testable import StraftUI

@Suite("Legacy defaults migration")
struct LegacyDefaultsMigrationTests {
    private func isolated() throws -> (UserDefaults, String) {
        let name = "straft.tests.legacy.\(UUID().uuidString)"
        return (try #require(UserDefaults(suiteName: name)), name)
    }

    @Test func oldKeysMoveToTheNewPrefixAndOtherKeysKeepTheirNames() throws {
        let (defaults, name) = try isolated()
        defer { defaults.removePersistentDomain(forName: name) }
        LegacyDefaultsMigration.migrate(into: defaults, domains: ["old"]) { _ in
            ["openframe.ai.width": 420.0, "of.story.zoom": 125, "vault.mode": "grid"] // LEGACY: Open Frame name kept for migration
        }
        #expect(defaults.double(forKey: "straft.ai.width") == 420)
        #expect(defaults.integer(forKey: "of.story.zoom") == 125)
        #expect(defaults.string(forKey: "vault.mode") == "grid")
        #expect(defaults.object(forKey: "openframe.ai.width") == nil) // LEGACY: Open Frame name kept for migration
        #expect(defaults.bool(forKey: LegacyDefaultsMigration.doneKey))
    }

    @Test func aValueAlreadyThereIsNeverOverwritten() throws {
        let (defaults, name) = try isolated()
        defer { defaults.removePersistentDomain(forName: name) }
        defaults.set(300.0, forKey: "straft.ai.width")
        defaults.set(false, forKey: "straft.ai.open")
        LegacyDefaultsMigration.migrate(into: defaults, domains: ["old"]) { _ in
            ["openframe.ai.width": 420.0, "openframe.ai.open": true] // LEGACY: Open Frame name kept for migration
        }
        #expect(defaults.double(forKey: "straft.ai.width") == 300)
        #expect(defaults.object(forKey: "straft.ai.open") as? Bool == false, "a stored false counts as a value")
    }

    @Test func itRunsOnce() throws {
        let (defaults, name) = try isolated()
        defer { defaults.removePersistentDomain(forName: name) }
        LegacyDefaultsMigration.migrate(into: defaults, domains: ["old"]) { _ in ["vault.mode": "grid"] }
        defaults.removeObject(forKey: "vault.mode")
        var reads = 0
        LegacyDefaultsMigration.migrate(into: defaults, domains: ["old"]) { _ in
            reads += 1
            return ["vault.mode": "list"]
        }
        #expect(reads == 0)
        #expect(defaults.object(forKey: "vault.mode") == nil, "a person's later change stays")
    }

    @Test func missingDomainsSetTheFlagAndDoNotCrash() throws {
        let (defaults, name) = try isolated()
        defer { defaults.removePersistentDomain(forName: name) }
        LegacyDefaultsMigration.migrate(into: defaults, domains: ["straft.tests.no.such.domain.\(UUID().uuidString)"])
        #expect(defaults.bool(forKey: LegacyDefaultsMigration.doneKey))
    }

    @Test func theFirstDomainWinsWhenBothHoldAKey() throws {
        let (defaults, name) = try isolated()
        defer { defaults.removePersistentDomain(forName: name) }
        LegacyDefaultsMigration.migrate(into: defaults, domains: ["first", "second"]) { domain in
            domain == "first" ? ["vault.mode": "grid"] : ["vault.mode": "list", "files.view": "cards"]
        }
        #expect(defaults.string(forKey: "vault.mode") == "grid")
        #expect(defaults.string(forKey: "files.view") == "cards")
    }

    /// Through the real preferences system: a domain on disk, bookmark data as bytes, a dictionary of it.
    @Test func bookmarksInARealDomainComeAlongAsData() throws {
        let (defaults, name) = try isolated()
        let domain = "straft.tests.legacy.source.\(UUID().uuidString)"
        let bookmark = Data([0x62, 0x6F, 0x6F, 0x6B, 0x00, 0xFF])
        UserDefaults.standard.setPersistentDomain(
            ["openframe.linkedFileBookmarks": ["/Films/photo.jpg": bookmark]], // LEGACY: Open Frame name kept for migration
            forName: domain
        )
        defer {
            UserDefaults.standard.removePersistentDomain(forName: domain)
            defaults.removePersistentDomain(forName: name)
        }
        LegacyDefaultsMigration.migrate(into: defaults, domains: [domain])
        let table = defaults.dictionary(forKey: "straft.linkedFileBookmarks") as? [String: Data]
        #expect(table?["/Films/photo.jpg"] == bookmark)
    }

    @Test func itNeverRunsOnATestFolderInTheSandboxOrUnderAnotherIdentifier() {
        let installed = "app.straftstudio.studio"
        #expect(LegacyDefaultsMigration.shouldRun(environment: [:], bundleId: installed))
        #expect(LegacyDefaultsMigration.shouldRun(environment: [:], bundleId: "\(installed).dev"))
        #expect(!LegacyDefaultsMigration.shouldRun(environment: ["STRAFT_DATA_ROOT": "/tmp/x"], bundleId: installed))
        #expect(!LegacyDefaultsMigration.shouldRun(environment: ["APP_SANDBOX_CONTAINER_ID": "x"], bundleId: installed))
        #expect(!LegacyDefaultsMigration.shouldRun(environment: [:], bundleId: "\(installed).e2e1"))
        #expect(!LegacyDefaultsMigration.shouldRun(environment: [:], bundleId: "app.straftstudio.updatecheck.new"))
        #expect(!LegacyDefaultsMigration.shouldRun(environment: [:], bundleId: nil))
        #expect(!LegacyDefaultsMigration.shouldRun(environment: [:], bundleId: Bundle.main.bundleIdentifier), "the test tool itself")
    }
}

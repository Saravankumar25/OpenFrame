// First launch: "Welcome to OpenFrame Studio. What should we call you?" (Windows: app/home/
// DisplayNameDialog.tsx). The name appears on notes, tasks and project activity. There is no
// account; it stays on this computer. The core says whether a name was ever set
// (app.info displayNameSet) and stores it (app.set_display_name).

import Observation
import OpenFrameCore
import SwiftUI

@MainActor
@Observable
final class WelcomeNameModel {
    var name: String
    private(set) var busy = false
    private(set) var error: String?
    /// True when the name was stored; the sheet goes away on it.
    private(set) var done = false

    private weak var app: AppModel?

    init(app: AppModel?, current: String) {
        self.app = app
        name = current
    }

    var trimmed: String { name.trimmingCharacters(in: .whitespacesAndNewlines) }
    var invalid: Bool { trimmed.isEmpty }
    /// Shown only after the person has tried (or emptied the field), as on Windows.
    var showsEmptyError: Bool { invalid }

    /// "Continue". Returns true when the name is stored.
    @discardableResult
    func continueTapped() async -> Bool {
        guard let app, let core = app.core, !invalid, !busy else { return false }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.appSetDisplayName(DisplayNameArgs(displayName: trimmed))
            await app.refreshInfo()
            done = true
            error = nil
            return true
        } catch {
            self.error = CoreFailure(asCore: error).message
            return false
        }
    }

    func edited() { error = nil }
}

extension AppModel {
    /// True until the person has typed a name once (first launch).
    var needsWelcomeName: Bool {
        guard let info else { return false }
        return !info.displayNameSet
    }
}

// 99-store: the Mac App Store trial and purchase screens through the real windows and the real core,
// over a scripted App Store (ScriptedStoreKitSource, debug builds on a test folder only, so the runner
// skips these on a release build). The scenario writes what Apple would answer, and the Mac's clock,
// into fake-store.json in the test folder. Real StoreKit needs a signed app; see
// docs/release/mac-store-test.md for the rest.

import AppKit
import Foundation

private let day = 86_400_000.0
private let trialID = "app.straftstudio.studio.trial7"
private let lifetimeID = "app.straftstudio.studio.lifetime"
private let contact = "https://openframestudio.app/contact/"

@MainActor private struct FakeStore {
    let root: URL
    init(_ ctx: Context) { root = URL(fileURLWithPath: ctx.env["STRAFT_DATA_ROOT"] ?? "/nonexistent", isDirectory: true) }
    var file: URL { root.appendingPathComponent("fake-store.json") }

    func read() -> [String: Any] {
        (try? Data(contentsOf: file)).flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: Any] } ?? [
            "clockOffsetMs": 0.0, "productsFail": false, "rows": [Any](), "nextPurchase": "success", "syncSignsWithRealTime": true,
        ]
    }

    func change(_ edit: (inout [String: Any]) -> Void) throws {
        var s = read()
        edit(&s)
        try JSONSerialization.data(withJSONObject: s).write(to: file, options: .atomic)
    }

    /// The Mac's clock this many days from the real time.
    func setDay(_ d: Double) throws { try change { $0["clockOffsetMs"] = d * day } }
    func offline(_ on: Bool) throws { try change { $0["productsFail"] = on } }

    /// A purchase already on Apple's side, made `daysAgo` before the real time.
    func own(_ product: String, id: String, daysAgo: Double = 0) throws {
        try change {
            var rows = $0["rows"] as? [[String: Any]] ?? []
            rows.append(["product": product, "id": id, "atMs": Date().timeIntervalSince1970 * 1000 - daysAgo * day])
            $0["rows"] = rows
        }
    }

    func refundAll() throws {
        try change {
            var rows = $0["rows"] as? [[String: Any]] ?? []
            for i in rows.indices {
                let at = rows[i]["atMs"] as? Double ?? 0
                rows[i]["revokedAtMs"] = at + 1000
            }
            $0["rows"] = rows
        }
    }

    var opened: [String] {
        ((try? String(contentsOf: root.appendingPathComponent("opened-addresses.txt"), encoding: .utf8)) ?? "")
            .split(separator: "\n").map(String.init)
    }
}

extension UI {
    /// The window comes up with a licence screen over it, not with Application Home.
    fileprivate func waitLicence(_ id: String) async throws {
        try await wait(id: id, timeout: 30, "the licence screen \(id)")
    }

    /// The window behind a closed licence screen, past the first-launch name question.
    fileprivate func settleBehindLicence() async throws {
        try await answerWelcome()
        try await waitHome()
    }

    fileprivate func startTrialFromFirstLaunch() async throws {
        try await waitLicence("licence-start-trial")
        try await press(id: "licence-start-trial", role: "AXButton", timeout: 15)
        try await waitGone(id: "licence-start-trial", timeout: 15)
        try await settleBehindLicence()
    }

    fileprivate func contactOpens(_ store: FakeStore) async throws {
        let before = store.opened.count
        try await press(id: "licence-contact", timeout: 10)
        let deadline = Date().addingTimeInterval(5)
        while store.opened.count == before && Date() < deadline { await idle(0.1) }
        try expect(store.opened.last == contact, "Contact us opens exactly \(contact), not \(store.opened.last ?? "nothing")")
    }

    /// Escape on a screen that cannot be closed leaves it there.
    fileprivate func cannotBeClosed(_ id: String) async throws {
        try await escape()
        await idle(0.5)
        try expect(exists(id: id), "Escape leaves the mandatory screen \(id) open")
    }
}

extension Scenarios {
    static let storeGroup: [Scenario] = [
        // Mocks 187, 200, 188. Price from StoreKit, chip on Home and in a project, gone after buying.
        Scenario("99-store/first-launch-trial-chip-on-home-and-project-then-buy", phases: 2) { ui, phase, ctx in
            let store = FakeStore(ctx)
            if phase == 1 {
                try await ui.waitLicence("licence-start-trial")
                try await ui.waitText("How would you like to begin?", timeout: 10)
                try await ui.waitText("No payment now.", timeout: 10)
                let text = ui.screenText()
                try ui.expect(text.contains("one-time purchase of {price from the App Store}."), "the disclosure states the price from the store")
                try ui.expect(text.contains("locks until you buy the full version"), "the disclosure says what stops when the trial ends")
                try ui.expect(ui.exists(id: "licence-restore"), "Restore purchase is on the first launch")
                try await ui.cannotBeClosed("licence-start-trial")
                try await ui.contactOpens(store)
                try ui.snapshot("first-launch")
                try await ui.startTrialFromFirstLaunch()
                // Application Home carries the chip and the button (mock 200).
                try await ui.waitText("Free trial. Ends in 7 days", timeout: 15)
                try ui.expect(ui.exists(id: "trial-buy"), "Buy full version is beside the chip on Home")
                try ui.snapshot("home-chip")
                // So does a project (mock 188).
                try await ui.createProject("Trial Project")
                try await ui.waitText("Free trial. Ends in 7 days", timeout: 15)
                try ui.expect(ui.exists(id: "trial-buy"), "Buy full version is beside the chip in a project")
                try ui.snapshot("project-chip")
                // Buying removes them at once and the three screens follow in order, once.
                try await ui.press(id: "trial-buy", role: "AXButton")
                try await ui.waitLicence("licence-tick")
                try ui.expect(!ui.exists(id: "trial-chip"), "the chip is gone at once")
                try ui.expect(!ui.exists(id: "trial-buy"), "the button is gone at once")
                try await ui.waitText("back up your root folder", timeout: 10)
                try await ui.cannotBeClosed("licence-tick")
                try await ui.press(id: "licence-tick")
                try await ui.press(id: "licence-ok", role: "AXButton")
                try await ui.waitLicence("licence-next")
                for _ in 0..<3 { try await ui.press(id: "licence-next", role: "AXButton"); await ui.idle(0.2) }
                for i in 1...3 { try await ui.press(id: "licence-check-\(i)") }
                try await ui.press(id: "licence-finish", role: "AXButton")
                try await ui.waitText("Welcome to the Straft Studio family", timeout: 15)
                try await ui.press(id: "licence-ok", role: "AXButton")
                try await ui.waitGone(id: "licence-ok", timeout: 15)
                try await ui.closeProject()
                return
            }
            // Never again after a restart, and the buyer has no chip.
            try await ui.startApp(ctx)
            await ui.idle(1.5)
            try ui.expect(!ui.exists(id: "licence-dialog"), "no licence screen after the restart")
            try ui.expect(!ui.exists(id: "trial-chip"), "no chip after the restart")
        },

        // Mocks 189. Days 5, 6, 7 only, once per day, on Application Home.
        Scenario("99-store/reminder-on-home-on-day-five-once-per-day", phases: 2) { ui, phase, ctx in
            let store = FakeStore(ctx)
            if phase == 1 {
                try store.own(trialID, id: "2000000001", daysAgo: 3.5)
                try await ui.startApp(ctx)
                try await ui.waitText("Free trial. Ends in 4 days", timeout: 20)
                await ui.idle(1)
                try ui.expect(!ui.exists(id: "licence-continue-trial"), "no reminder on trial day 4")
                try store.own(trialID, id: "2000000009", daysAgo: 4.5)
                try await ui.waitLicence("licence-continue-trial")
                try await ui.waitText("Your free trial ends in 3 days", timeout: 10)
                try ui.expect(ui.exists(id: "licence-reminder-end"), "the reminder says the real end moment")
                try ui.expect(ui.screenText().contains("It ends on "), "the end line is there")
                try ui.snapshot("reminder-home")
                try await ui.press(id: "licence-continue-trial", role: "AXButton")
                try await ui.waitGone(id: "licence-continue-trial", timeout: 10)
                try ui.expect(ui.screenText().contains("Free trial. Ends in 3 days. Your trial ends soon."), "the chip stays, amber, and says so")
                return
            }
            try await ui.startApp(ctx)
            await ui.idle(2)
            try ui.expect(!ui.exists(id: "licence-continue-trial"), "the reminder does not come back the same day")
        },

        // Mocks 190. The trial ends while open: typed text is saved before the lock, which cannot be
        // closed; Buy lifts it and the text is there.
        Scenario("99-store/trial-ends-while-open-keeps-unsaved-text-and-locks", phases: 1) { ui, _, ctx in
            let store = FakeStore(ctx)
            try store.own(trialID, id: "2000000001", daysAgo: 1)
            try await ui.startApp(ctx)
            try await ui.createProject("Locked Out")
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            await ui.caretToEnd(tv)
            let line = "Nadia hides the last reel under the stairs."
            try await ui.typeHere(line)
            // No wait for Saved: the lock itself must store what the editor holds.
            try store.setDay(8)
            try await ui.waitLicence("licence-buy")
            try await ui.waitText("Your free trial has ended", timeout: 10)
            try ui.expect(ui.screenText().contains("One-time purchase of {price from the App Store}."), "the lock states the price from the store")
            try ui.expect(ui.exists(id: "licence-restore"), "Restore purchase is on the lock")
            try ui.snapshot("locked")
            try await ui.cannotBeClosed("licence-buy")
            try await ui.contactOpens(store)
            try await ui.press(id: "licence-restore")
            await ui.idle(0.5)
            try ui.expect(ui.exists(id: "licence-buy"), "Restore with no purchase keeps the lock")
            try await ui.press(id: "licence-buy", role: "AXButton")
            try await ui.waitLicence("licence-tick")
            try await ui.press(id: "licence-tick")
            try await ui.press(id: "licence-ok", role: "AXButton")
            try await ui.waitLicence("licence-next")
            for _ in 0..<3 { try await ui.press(id: "licence-next", role: "AXButton"); await ui.idle(0.2) }
            for i in 1...3 { try await ui.press(id: "licence-check-\(i)") }
            try await ui.press(id: "licence-finish", role: "AXButton")
            try await ui.waitText("Welcome to the Straft Studio family", timeout: 15)
            try await ui.press(id: "licence-ok", role: "AXButton")
            try await ui.waitGone(id: "licence-ok", timeout: 15)
            // The text is on disk: closing and opening reads it back from the project.
            try await ui.closeProject()
            try await ui.press(id: "project-card-Locked Out", timeout: 15)
            let reopened = try await ui.editor()
            try ui.expect(reopened.string.contains(line), "the line typed before the lock is there: \(reopened.string.suffix(160))")
        },

        // A refund arrives while open: the lock with the refund words.
        Scenario("99-store/refund-while-open-locks-with-the-refund-words", phases: 1) { ui, _, ctx in
            let store = FakeStore(ctx)
            try store.own(lifetimeID, id: "2000000002")
            try await ui.waitLicence("licence-tick")
            try await ui.press(id: "licence-tick")
            try await ui.press(id: "licence-ok", role: "AXButton")
            try await ui.waitLicence("licence-next")
            for _ in 0..<3 { try await ui.press(id: "licence-next", role: "AXButton"); await ui.idle(0.2) }
            for i in 1...3 { try await ui.press(id: "licence-check-\(i)") }
            try await ui.press(id: "licence-finish", role: "AXButton")
            try await ui.waitText("Welcome to the Straft Studio family", timeout: 15)
            try await ui.press(id: "licence-ok", role: "AXButton")
            try await ui.waitGone(id: "licence-ok", timeout: 15)
            try await ui.settleBehindLicence()
            try store.refundAll()
            try await ui.waitText("Your purchase is no longer active", timeout: 20)
            try ui.expect(ui.screenText().contains("no longer shows a purchase"), "the refund words")
            try ui.snapshot("refunded")
            try await ui.cannotBeClosed("licence-buy")
            try await ui.contactOpens(store)
        },

        // Mock 201. A clock turned back locks everything, the screen cannot be closed, and it clears
        // by itself as soon as the clock is right.
        Scenario("99-store/clock-turned-back-locks-and-clears-by-itself", phases: 1) { ui, _, ctx in
            let store = FakeStore(ctx)
            try store.own(trialID, id: "2000000001", daysAgo: 1)
            try await ui.startApp(ctx)
            try await ui.waitText("Free trial. Ends in 6 days", timeout: 20)
            try store.setDay(-3)
            try await ui.waitLicence("licence-check-again")
            try await ui.waitText("Your Mac's clock looks wrong", timeout: 10)
            try ui.expect(!ui.exists(id: "trial-chip"), "no chip while the clock is wrong")
            try ui.snapshot("clock")
            try await ui.cannotBeClosed("licence-check-again")
            try await ui.contactOpens(store)
            try store.setDay(0)
            try await ui.waitGone(id: "licence-check-again", timeout: 20)
            try await ui.waitText("Free trial. Ends in 6 days", timeout: 15)
        },

        // Mock 192. The first launch with no App Store lands on the check screen, not on a blank or a
        // guess. It can be closed and offers Contact us; Try again finds the store.
        Scenario("99-store/first-launch-offline-shows-the-check-screen", phases: 1) { ui, _, ctx in
            let store = FakeStore(ctx)
            try store.offline(true)
            try await ui.waitLicence("licence-try-again")
            try await ui.waitText("Checking your purchase", timeout: 10)
            try ui.expect(!ui.exists(id: "licence-start-trial"), "no trial is offered without the store")
            try ui.snapshot("check")
            try await ui.contactOpens(store)
            try store.offline(false)
            try await ui.press(id: "licence-try-again", role: "AXButton")
            try await ui.waitLicence("licence-start-trial")
        },

        // The trial and a buyer carry on with no network after the first start.
        Scenario("99-store/offline-relaunch-keeps-the-trial-and-the-buyer", phases: 3) { ui, phase, ctx in
            let store = FakeStore(ctx)
            switch phase {
            case 1:
                try await ui.startTrialFromFirstLaunch()
                try await ui.waitText("Free trial. Ends in 7 days", timeout: 15)
            case 2:
                try store.offline(true)
                try store.change { $0["rows"] = [Any]() }
                try store.setDay(2)
                try await ui.startApp(ctx)
                try await ui.waitText("Free trial. Ends in 5 days", timeout: 20)
                try ui.expect(!ui.exists(id: "licence-try-again"), "no check screen for a known trial")
                try store.offline(false)
                try store.own(lifetimeID, id: "2000000002")
                try await ui.waitLicence("licence-tick")
            default:
                try store.offline(true)
                try store.change { $0["rows"] = [Any]() }
                try await ui.waitLicence("licence-tick")
                try ui.expect(!ui.exists(id: "trial-chip"), "a buyer is never locked or counted down offline")
            }
        },

        // Kill the app mid-edit (package file 05, U6): the crash question comes up and the work is back.
        Scenario("98-quit/killed-mid-edit-the-recovery-brings-the-text-back", phases: 2, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            let line = "Ravi runs for the last ferry."
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Hard Stop")
                try await ui.go("Screenplay")
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere(line)
                try await ui.waitText("Saved", timeout: 20)
                return
            }
            try await ui.openProject("Hard Stop", expectRecovery: true)
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            try ui.expect(tv.string.contains(line), "the line typed before the kill is back: \(tv.string.suffix(160))")
        },
    ]
}

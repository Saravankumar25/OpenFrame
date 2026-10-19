// Pure rules and fixed text for the trial, purchase and first-run safety screens (Windows:
// features/licensing/rules.ts, owner mocks 182 to 198). The Rust core decides which screen is due
// (`entitlement.notice`); these helpers only shape what is shown. No screen names a price, a
// currency or a pricing model: the App Store's own purchase window does.

import Foundation
import OpenFrameCore

public enum LicenceRules {
    /// Trial reminders and the amber chip start when this many days or fewer are left.
    public static let lastDays = 3

    /// The person's local calendar day as `YYYY-MM-DD` (the core stores the reminder day in this
    /// form, so "once per calendar day" follows the person's own midnight).
    public static func localDay(_ now: Date = Date(), calendar: Calendar = .current) -> String {
        let c = calendar.dateComponents([.year, .month, .day], from: now)
        return String(format: "%04d-%02d-%02d", c.year ?? 0, c.month ?? 0, c.day ?? 0)
    }

    public struct TrialChip: Equatable {
        public var daysLeft: Int
        /// The last 3 days: amber, and VoiceOver says the trial ends soon.
        public var ending: Bool
        public var label: String
        public var spoken: String
    }

    /// The corner chip, shown from trial day 1 until the purchase. `nil` in every other state.
    public static func trialChip(_ state: EntitlementState?) -> TrialChip? {
        guard case let .trialActive(days) = state else { return nil }
        let daysLeft = max(1, min(7, Int(days.rounded())))
        let ending = daysLeft <= lastDays
        let label = daysLeft == 1 ? "Trial: 1 day left" : "Trial: \(daysLeft) days left"
        let spoken = daysLeft == 1
            ? "Free trial, last day. Your trial ends today."
            : "Free trial, \(daysLeft) days left.\(ending ? " Your trial ends soon." : "")"
        return TrialChip(daysLeft: daysLeft, ending: ending, label: label, spoken: spoken)
    }

    /// Day 5 "ends in 3 days", day 6 "ends in 2 days", day 7 "ends today".
    public static func reminderHeadline(_ daysLeft: Int) -> String {
        daysLeft <= 1 ? "Your free trial ends today" : "Your free trial ends in \(daysLeft) days"
    }

    /// The store's name as the screens say it. The Mac build has one store.
    public static let storeName = "the App Store"

    /// Tutorial step 3, second point: how to install on the new laptop.
    public static func newLaptopInstallStep(_ channel: Channel?) -> String {
        switch channel {
        case .macAppStore:
            "Install OpenFrame Studio on the new laptop from the App Store, signed in with the same Apple Account."
        case .microsoftStore:
            "Install OpenFrame Studio on the new laptop from the Microsoft Store, signed in with the same Microsoft account."
        default:
            "Install OpenFrame Studio on the new laptop."
        }
    }

    public static let revealLabel = "Show in Finder"

    public struct SocialLink: Equatable, Identifiable, Sendable {
        public var name: String
        public var short: String
        public var handle: String
        /// `nil` until the owner sends the real addresses: such an account cannot be opened.
        public var url: String?
        public var id: String { name }
    }

    public static let socialLinks: [SocialLink] = [
        SocialLink(name: "Instagram", short: "IG", handle: "@openframestudio"),
        SocialLink(name: "YouTube", short: "YT", handle: "@openframestudio"),
        SocialLink(name: "X", short: "X", handle: "@openframestudio"),
        SocialLink(name: "Facebook", short: "f", handle: "@openframestudio"),
        SocialLink(name: "LinkedIn", short: "in", handle: "@openframestudio"),
        SocialLink(name: "Reddit", short: "Rd", handle: "@openframestudio"),
        SocialLink(name: "Discord", short: "Dc", handle: "@openframestudio"),
        SocialLink(name: "Threads", short: "Th", handle: "@openframestudio"),
    ]

    private static let socialHosts: Set<String> = [
        "www.instagram.com", "instagram.com", "www.youtube.com", "youtube.com", "x.com",
        "www.facebook.com", "facebook.com", "www.linkedin.com", "linkedin.com",
        "www.reddit.com", "reddit.com", "discord.gg", "discord.com", "www.threads.net", "threads.net",
    ]

    /// A fixed social address: https, a known social site, no user name or password, no port.
    public static func isFixedSocialURL(_ url: String?) -> Bool {
        guard let url, let u = URL(string: url), u.scheme == "https", u.user == nil, u.password == nil,
              u.port == nil, let host = u.host
        else { return false }
        return socialHosts.contains(host)
    }

    public static let buyFailed = "OpenFrame could not start the purchase right now. Everything keeps working."
    public static let saveFailed = "OpenFrame could not save this step. Try again."

    public static let dataPoints: [(title: String, body: String)] = [
        ("Your projects live only on this computer.",
         "OpenFrame has no cloud. We do not hold a copy, so we cannot recover your work if the computer is lost, stolen, damaged or reset."),
        ("Back up the whole root folder.",
         "Pick the one folder where your OpenFrame projects are saved and copy it to Google Drive (or another cloud drive) and, if you can, to an external disk. Repeat it often."),
        ("Moving to a new laptop is your job, too.",
         "Copy that same root folder to the new computer, install OpenFrame, then use Open Project and pick the folder."),
        ("Keep at least two copies.",
         "One on this computer and one somewhere else. A copy inside the same computer does not protect you."),
        ("Never delete or rename the root folder by hand.",
         "Use OpenFrame to rename, archive or delete projects."),
        ("Backups inside OpenFrame are separate copies.",
         "Create Backup saves a copy; still keep it off this computer."),
    ]

    public static let tutorialSteps = ["Where it lives", "Back up", "New laptop", "Check"]
    public static let tutorialTitles = [
        "Where your data lives", "Back up the root folder to Google Drive", "Moving to a new laptop", "Final check",
    ]
    public static let tutorialChecks = [
        "I know where my root folder is.",
        "I will back it up to Google Drive or an external disk regularly.",
        "I know I must copy the folder myself when I change laptops.",
    ]
}

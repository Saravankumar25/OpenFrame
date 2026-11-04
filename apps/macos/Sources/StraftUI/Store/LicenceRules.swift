// Pure rules and fixed text for the trial, purchase and first-run safety screens (Windows:
// features/licensing/rules.ts, owner mocks 182 to 203). The Rust core decides which screen is due
// (`entitlement.notice`); these helpers only shape what is shown. A price is never typed here. The
// first-launch and lock screens show the one StoreKit supplies at run time, because Apple's
// Guideline 3.1.1 asks for the later charge to be disclosed before a trial starts.

import Foundation
import StraftCore

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

    /// The address "Contact us" opens in the browser. The app has no form and sends nothing.
    public static let contactAddress = "https://openframestudio.app/contact/"
    public static let contactLink = "Need help? Contact us"

    /// The corner chip, shown from trial day 1 until the purchase. `nil` in every other state.
    public static func trialChip(_ state: EntitlementState?) -> TrialChip? {
        guard case let .trialActive(days) = state else { return nil }
        let daysLeft = max(1, min(7, Int(days.rounded())))
        let ending = daysLeft <= lastDays
        let label = daysLeft == 1 ? "Trial ends in 1 day" : "Trial ends in \(daysLeft) days"
        let spoken = daysLeft == 1
            ? "Free trial, last day. Your trial ends within 24 hours."
            : "Free trial. Ends in \(daysLeft) days.\(ending ? " Your trial ends soon." : "")"
        return TrialChip(daysLeft: daysLeft, ending: ending, label: label, spoken: spoken)
    }

    /// Day 5 "ends in 3 days", day 6 "ends in 2 days". On the last day "ends today" when the end
    /// falls on today's local date, otherwise "ends tomorrow".
    public static func reminderHeadline(
        _ daysLeft: Int, endsAt: Date? = nil, now: Date = Date(), calendar: Calendar = .current
    ) -> String {
        guard daysLeft <= 1 else { return "Your free trial ends in \(daysLeft) days" }
        if let endsAt, !calendar.isDate(endsAt, inSameDayAs: now) { return "Your free trial ends tomorrow" }
        return "Your free trial ends today"
    }

    /// The line with the real end moment, in the person's own time zone: "It ends on Thu 5 Nov at 9:42 PM."
    public static func reminderEndLine(
        _ endsAt: Date, calendar: Calendar = .current, locale: Locale = Locale(identifier: "en_US_POSIX")
    ) -> String {
        let day = DateFormatter()
        day.calendar = calendar
        day.timeZone = calendar.timeZone
        day.locale = locale
        day.dateFormat = "EEE d MMM"
        let time = DateFormatter()
        time.calendar = calendar
        time.timeZone = calendar.timeZone
        time.locale = locale
        time.dateFormat = "h:mm a"
        return "It ends on \(day.string(from: endsAt)) at \(time.string(from: endsAt))."
    }

    /// Mock 187, the free trial card. `price` is `Product.displayPrice`, read at run time.
    public static func trialDisclosure(price: String) -> String {
        "7 days. Everything included. No payment now. When the trial ends, Straft Studio locks until you buy the full version. Your projects stay safe on this Mac. The full version is a one-time purchase of \(price)."
    }

    public static let buyCardText = "One-time purchase through the App Store. Pay once, use it for good."

    /// Mock 190. The price line is left out until StoreKit has supplied a price.
    public static func lockBody(revoked: Bool, price: String?) -> String {
        if revoked {
            return "The store no longer shows a purchase for this account, so Straft Studio is locked. Your projects are still on this Mac. If this is a mistake, choose Restore purchase or contact us."
        }
        let buy = "Buy the full version to keep using Straft Studio."
        guard let price else { return buy }
        return "\(buy) One-time purchase of \(price)."
    }

    public static let lockSafety = "Your projects are safe on this Mac. Buying unlocks them again exactly as you left them."

    /// Mock 201.
    public static let clockTitle = "Your Mac's clock looks wrong"
    public static let clockBody = "Straft Studio checks the date to count your free trial. Turn on Set time and date automatically in System Settings, then choose Check again."
    public static let clockSafety = "Your projects are safe on this Mac. This screen clears by itself as soon as the clock is right."

    /// The store's name as the screens say it. The Mac build has one store.
    public static let storeName = "the App Store"

    /// Tutorial step 3, second point: how to install on the new laptop.
    public static func newLaptopInstallStep(_ channel: Channel?) -> String {
        switch channel {
        case .macAppStore:
            "Install Straft Studio on the new laptop from the App Store, signed in with the same Apple Account."
        case .microsoftStore:
            "Install Straft Studio on the new laptop from the Microsoft Store, signed in with the same Microsoft account."
        default:
            "Install Straft Studio on the new laptop."
        }
    }

    public static let revealLabel = "Show in Finder"

    public struct SocialLink: Equatable, Identifiable, Sendable {
        public var name: String
        public var short: String
        public var handle: String
        /// `nil` until the owner sends the real addresses: such an account cannot be opened, and a
        /// release build does not show it.
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

    /// The rows the welcome popup shows. These handles are placeholders, so a release build shows
    /// only rows with a real address; a debug or test build keeps them all for the mocks.
    public static func visibleSocialLinks(release: Bool, links: [SocialLink] = socialLinks) -> [SocialLink] {
        release ? links.filter { isFixedSocialURL($0.url) } : links
    }

    #if DEBUG
    public static let isReleaseBuild = false
    #else
    public static let isReleaseBuild = true
    #endif

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

    public static let buyFailed = "Straft Studio could not start the purchase right now. Everything keeps working."

    public static let dataPoints: [(title: String, body: String)] = [
        ("Your projects live only on this computer.",
         "Straft Studio has no cloud. We do not hold a copy, so we cannot recover your work if the computer is lost, stolen, damaged or reset."),
        ("Back up the whole root folder.",
         "Pick the one folder where your Straft Studio projects are saved and copy it to Google Drive (or another cloud drive) and, if you can, to an external disk. Repeat it often."),
        ("Moving to a new laptop is your job, too.",
         "Copy that same root folder to the new computer, install Straft Studio, then use Open Project and pick the folder."),
        ("Keep at least two copies.",
         "One on this computer and one somewhere else. A copy inside the same computer does not protect you."),
        ("Never delete or rename the root folder by hand.",
         "Use Straft Studio to rename, archive or delete projects."),
        ("Backups inside Straft Studio are separate copies.",
         "Create Backup saves a copy; still keep it off this computer. Before you delete Straft Studio from this Mac, back up your root folder."),
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

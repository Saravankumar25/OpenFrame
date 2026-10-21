// Shared pieces of the Call Sheets workspace: the tables each screen reads (the same lists as
// apps/desktop/src/api/schedule.ts), the status words, date and number formatting, the small
// views (chips, banners, the sheet frame) and the route other workspaces use to open a call
// sheet. Colours come from the theme.

import AppKit
import Foundation
import Observation
import SwiftUI
import OpenFrameCore

// MARK: Tables

enum CSTables {
    /// Every table the schedule projections read.
    static let schedule = [
        "shooting_schedule", "shooting_day", "schedule_strip", "schedule_marker", "schedule_warning_ack",
        "dood_override", "call_sheet", "production_source", "screenplay", "screenplay_draft", "screenplay_scene",
        "screenplay_element", "breakdown_element", "catalog_item", "location", "cast_member", "story_character",
        "project",
    ]
    static let callSheet = schedule + ["asset"]
    static let sides = schedule + ["side"]
    static let reports = schedule + ["production_report"]
    static let members = ["project_member"]
}

// MARK: Status words

enum CSStatus {
    /// Draft, Needs refresh or Final. Anything else reads Draft.
    static func word(_ status: String) -> String {
        status == "Final" || status == "Needs refresh" ? status : "Draft"
    }

    static func tone(_ status: String) -> CSTone {
        switch word(status) {
        case "Final": .green
        case "Needs refresh": .yellow
        default: .plain
        }
    }
}

/// What a sheet shows where nobody has entered a value yet (never invented text).
let csNeedsInput = "Needs input"

// MARK: Formatting

enum CSFormat {
    /// "14 Jun, 09:30" (Windows `formatDateTime`).
    static func dateTime(_ ms: Double) -> String {
        let f = DateFormatter()
        f.locale = Locale.current
        f.setLocalizedDateFormatFromTemplate("d MMM HH:mm")
        return f.string(from: Date(timeIntervalSince1970: ms / 1000))
    }

    /// "5h 30m", "45m", "10h" (Windows `formatMinutes`).
    static func minutes(_ m: Double) -> String {
        let total = Int(m.rounded())
        let h = total / 60
        let r = total % 60
        if h == 0 { return "\(r)m" }
        return r == 0 ? "\(h)h" : "\(h)h \(r)m"
    }

    /// A scene number from the schedule: whole numbers without a decimal point.
    static func number(_ n: Double?) -> String {
        guard let n else { return "Not numbered" }
        return n == n.rounded() ? String(Int(n)) : String(n)
    }

    /// "Break: MEAL at 12:30 (45m)".
    static func marker(_ m: ScheduleMarkerDto) -> String {
        var s = m.label.uppercased()
        if let t = m.atTime, !t.isEmpty { s += " at \(t)" }
        if let d = m.durationMinutes, d > 0 { s += " (\(minutes(d)))" }
        return s
    }

    private static let bad = CharacterSet(charactersIn: "<>:\"/\\|?*")

    private static func clean(_ part: String) -> String {
        var s = part.replacingOccurrences(of: "\u{2013}", with: "-").replacingOccurrences(of: "\u{2014}", with: "-")
        s = String(s.unicodeScalars.map { bad.contains($0) || $0.value < 32 ? " " : Character($0) })
        return s.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ").trimmingCharacters(in: .whitespaces)
    }

    /// "Project - Document - 2026-09-30.pdf" (Windows `exportFileName`).
    static func exportFileName(project: String?, document: String, format: String, date: Date = Date()) -> String {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        let parts = [clean(project ?? ""), clean(document), f.string(from: date)].filter { !$0.isEmpty }
        var base = parts.joined(separator: " - ")
        while base.hasSuffix(".") || base.hasSuffix(" ") { base.removeLast() }
        if base.count > 150 { base = String(base.prefix(150)).trimmingCharacters(in: .whitespaces) }
        if base.isEmpty { base = "Export" }
        return "\(base).\(format)"
    }
}

// MARK: Chips and banners

enum CSTone {
    case plain, green, yellow, red, blue

    var tokens: (bg: String, fg: String) {
        switch self {
        case .plain: ("surface2", "ink2")
        case .green: ("green-soft", "green-t")
        case .yellow: ("yellow-soft", "yellow-t")
        case .red: ("red-soft", "red-t")
        case .blue: ("blue-soft", "blue-t")
        }
    }
}

struct CSChip: View {
    let text: String
    var tone: CSTone = .plain
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Text(text)
            .lineLimit(1)
            .font(.caption.weight(.medium))
            .foregroundStyle(theme[tone.tokens.fg])
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(theme[tone.tokens.bg], in: Capsule())
            .fixedSize()
    }
}

struct CSBanner<Actions: View>: View {
    enum Tone { case info, warn, err, ok }
    let tone: Tone
    let text: String
    @ViewBuilder var actions: () -> Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let (bg, fg, symbol): (String, String, String) = switch tone {
        case .info: ("blue-soft", "blue-t", "info.circle")
        case .warn: ("yellow-soft", "yellow-t", "exclamationmark.triangle")
        case .err: ("red-soft", "red-t", "xmark.octagon")
        case .ok: ("green-soft", "green-t", "checkmark.circle")
        }
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: symbol).accessibilityHidden(true)
            Text(text).fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 8)
            actions()
        }
        .font(.callout)
        .foregroundStyle(theme[fg])
        .padding(.horizontal, 10).padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme[bg], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .accessibilityElement(children: .contain)
    }
}

extension CSBanner where Actions == EmptyView {
    init(_ tone: Tone, _ text: String) {
        self.init(tone: tone, text: text, actions: { EmptyView() })
    }
}

/// The frame of a sheet: title, optional line under it, content, and a footer of buttons.
struct CSSheetFrame<Content: View, Footer: View, Leading: View>: View {
    let title: String
    var sub: String?
    var width: CGFloat = 520
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @ViewBuilder var leading: () -> Leading
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            VStack(spacing: PageSpacing.tight) {
                Text(title).font(.title3.weight(.semibold)).multilineTextAlignment(.center).accessibilityAddTraits(.isHeader)
                if let sub { Text(sub).font(.callout).foregroundStyle(.secondary).multilineTextAlignment(.center).fixedSize(horizontal: false, vertical: true) }
            }
            .frame(maxWidth: .infinity)
            content()
            HStack(spacing: 8) {
                leading()
                Spacer(minLength: 0)
                footer()
            }
        }
        .padding(PageSpacing.large)
        .frame(width: width)
        .background(theme["panel"])
    }
}

extension CSSheetFrame where Leading == EmptyView {
    init(title: String, sub: String? = nil, width: CGFloat = 520,
         @ViewBuilder content: @escaping () -> Content, @ViewBuilder footer: @escaping () -> Footer) {
        self.init(title: title, sub: sub, width: width, content: content, footer: footer, leading: { EmptyView() })
    }
}

// MARK: Opening a call sheet from another workspace

/// Other workspaces (the Schedule, the Daily View of the Production tab) ask for a call sheet here:
/// set a request, then switch to the Call Sheets workspace. The workspace takes the request when
/// it shows, or at once when it is already on screen.
@MainActor
@Observable
public final class CallSheetsRoute {
    public static let shared = CallSheetsRoute()

    public enum Request: Equatable, Sendable {
        /// Open this call sheet.
        case sheet(String)
        /// Open the call sheet of this shooting day, creating it when the day has none.
        case day(String)
        /// Show the sides of this shooting day.
        case sides(day: String)
        /// Show the daily view of this shooting day.
        case daily(day: String)
    }

    public private(set) var pending: Request?
    private init() {}

    public func request(_ r: Request) { pending = r }

    func take() -> Request? {
        defer { pending = nil }
        return pending
    }
}

// MARK: Misc

extension Array {
    subscript(safe i: Int) -> Element? { indices.contains(i) ? self[i] : nil }
}

/// Posts the switch to another workspace (the same notification the sidebar uses).
@MainActor
func csShowWorkspace(_ ws: Workspace) {
    ProjectNav.show(ws)
}

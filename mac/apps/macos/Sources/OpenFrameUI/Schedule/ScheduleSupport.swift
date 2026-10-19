// Pure helpers of the Schedule workspace: how minutes, clock times and page counts are written and
// read, the filters of the stripboard and the list, the month grid of the calendar, and the words
// shown on strips, breaks and call sheet chips. Mirrors apps/desktop/src/api/schedule.ts,
// call-sheets/shared/scheduleFilter.ts and calendar.ts. Nothing here changes data: the core
// decides everything that is stored.

import Foundation
import OpenFrameCore

/// What the person typed, read: nothing, a value, or something that cannot be understood.
enum SchedParsed<T: Equatable>: Equatable {
    case empty
    case value(T)
    case invalid

    var isInvalid: Bool { if case .invalid = self { true } else { false } }
    /// The value, or nil for an empty field (and for an invalid one: check `isInvalid` first).
    var number: T? { if case let .value(v) = self { v } else { nil } }
}

enum SchedFormat {
    // MARK: Minutes

    /// "5h 30m", "45m", "10h".
    static func minutes(_ m: Double) -> String { minutes(Int(m.rounded())) }
    static func minutes(_ m: Int) -> String {
        let h = m / 60
        let r = m % 60
        if h == 0 { return "\(r)m" }
        if r == 0 { return "\(h)h" }
        return "\(h)h " + String(format: "%02d", r) + "m"
    }

    /// A duration typed by a person: "90", "1:30", "1h 30m", "1.5h", "45m".
    static func parseDuration(_ input: String) -> SchedParsed<Int> {
        let s = input.trimmingCharacters(in: .whitespaces).lowercased()
        if s.isEmpty { return .empty }
        if s.allSatisfy(\.isNumber), let n = Int(s) { return .value(n) }
        let colon = s.split(separator: ":", omittingEmptySubsequences: false)
        if colon.count == 2, let h = Int(colon[0]), colon[1].count >= 1, colon[1].count <= 2, let m = Int(colon[1]),
           colon[0].allSatisfy(\.isNumber), colon[1].allSatisfy(\.isNumber) {
            return .value(h * 60 + m)
        }
        // "1h 30m", "1.5h", "45m", "45 min"
        guard let regex = try? NSRegularExpression(pattern: #"^(?:(\d+(?:\.\d+)?)\s*h)?\s*(?:(\d+)\s*m(?:in)?)?$"#) else { return .invalid }
        let range = NSRange(s.startIndex..., in: s)
        guard let match = regex.firstMatch(in: s, range: range) else { return .invalid }
        func group(_ i: Int) -> String? {
            let r = match.range(at: i)
            return r.location == NSNotFound ? nil : (s as NSString).substring(with: r)
        }
        let hours = group(1), mins = group(2)
        if hours == nil && mins == nil { return .invalid }
        let h = Double(hours ?? "0") ?? 0
        return .value(Int((h * 60).rounded()) + (Int(mins ?? "0") ?? 0))
    }

    // MARK: Clock

    /// Clock minutes after midnight as "07:00"; "01:30 (+1)" after the next midnight.
    static func clock(_ m: Double) -> String { clock(Int(m)) }
    static func clock(_ m: Int) -> String {
        let day = m / 1440
        let t = ((m % 1440) + 1440) % 1440
        let base = String(format: "%02d:%02d", t / 60, t % 60)
        return day > 0 ? "\(base) (+\(day))" : base
    }

    /// A typed clock time ("7", "7:30", "07:30", "730", "19.30") as minutes after midnight (0 to 1439).
    static func parseClock(_ input: String) -> SchedParsed<Int> {
        let s = input.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ".", with: ":")
        if s.isEmpty { return .empty }
        var h: Int
        var m: Int
        let parts = s.split(separator: ":", omittingEmptySubsequences: false)
        if parts.count == 2, parts[0].count >= 1, parts[0].count <= 2, parts[1].count == 2,
           let a = Int(parts[0]), let b = Int(parts[1]), parts.allSatisfy({ $0.allSatisfy(\.isNumber) }) {
            h = a
            m = b
        } else if parts.count == 1, s.count <= 4, s.allSatisfy(\.isNumber), let n = Int(s) {
            if s.count <= 2 {
                h = n
                m = 0
            } else {
                h = n / 100
                m = n % 100
            }
        } else {
            return .invalid
        }
        if h > 23 || m > 59 { return .invalid }
        return .value(h * 60 + m)
    }

    // MARK: Pages

    /// "1 2/8", "3/8", "2" from eighths of a page.
    static func pages(_ eighths: Double) -> String { pages(Int(eighths)) }
    static func pages(_ eighths: Int) -> String {
        let whole = eighths / 8
        let rest = eighths % 8
        if whole == 0 { return "\(rest)/8" }
        if rest == 0 { return "\(whole)" }
        return "\(whole) \(rest)/8"
    }

    /// "1 2/8", "3/8", "2" or "1.25" as eighths.
    static func parsePages(_ input: String) -> SchedParsed<Int> {
        let s = input.trimmingCharacters(in: .whitespaces)
        if s.isEmpty { return .empty }
        if let regex = try? NSRegularExpression(pattern: #"^(?:(\d+)\s+)?(\d+)/8$"#),
           let match = regex.firstMatch(in: s, range: NSRange(s.startIndex..., in: s)) {
            func group(_ i: Int) -> Int {
                let r = match.range(at: i)
                return r.location == NSNotFound ? 0 : Int((s as NSString).substring(with: r)) ?? 0
            }
            return .value(group(1) * 8 + group(2))
        }
        if let d = Double(s), s.allSatisfy({ $0.isNumber || $0 == "." }), d >= 0 { return .value(Int((d * 8).rounded())) }
        return .invalid
    }

    // MARK: Words

    static func plural(_ n: Int, _ word: String) -> String { "\(n) \(word)\(n == 1 ? "" : "s")" }

    /// Draft, Needs refresh or Final: the only three words (anything else reads as Draft).
    static func callSheetWord(_ status: String) -> String {
        status == "Final" || status == "Needs refresh" ? status : "Draft"
    }

    /// "Scene 4" or "Scene".
    static func sceneName(_ s: ScheduleStripDto) -> String {
        s.number.map { "Scene \(Int($0))" } ?? "Scene"
    }

    /// "Meal Break at 13:00 (45m)". The board shows it in capitals; the spoken and searchable text keeps the label.
    static func markerText(_ m: ScheduleMarkerDto) -> String {
        let base = m.label
        let time = m.atTime.map { " at \($0)" } ?? ""
        let dur = (m.durationMinutes ?? 0) > 0 ? " (\(minutes(m.durationMinutes ?? 0)))" : ""
        return base + time + dur
    }

    /// The spoken name of a strip row.
    static func stripLabel(_ s: ScheduleStripDto) -> String {
        var parts = [
            s.number.map { "Scene \(Int($0))" } ?? "Scene (not in script)",
            s.ieLabel,
            s.locationName ?? "location missing",
            s.synopsis.isEmpty ? s.heading : s.synopsis,
            "\(pages(s.pageEighths)) pages",
        ]
        if let e = s.estimatedMinutes { parts.append("estimated \(minutes(e))") }
        if let t = s.startLabel { parts.append("starts at \(t)") }
        if s.sourceState != "Current" { parts.append("script: " + (s.changeKinds.isEmpty ? s.sourceState : s.changeKinds.joined(separator: ", "))) }
        return parts.joined(separator: ", ")
    }

    static func sourceChip(_ s: ScheduleStripDto) -> String? {
        switch s.sourceState {
        case "Removed": "Removed from script"
        case "New": "New scene"
        case "Current": nil
        default: "Script changed"
        }
    }
}

// MARK: Dates

enum SchedDate {
    private static var utc: Calendar {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = TimeZone(identifier: "UTC")!
        c.firstWeekday = 2
        return c
    }

    /// `YYYY-MM-DD` as a date, or nil when it is not a real date.
    static func parse(_ text: String) -> Date? {
        let parts = text.split(separator: "-", omittingEmptySubsequences: false)
        guard parts.count == 3, parts[0].count == 4, parts[1].count == 2, parts[2].count == 2,
              let y = Int(parts[0]), let m = Int(parts[1]), let d = Int(parts[2]) else { return nil }
        var comps = DateComponents()
        comps.year = y; comps.month = m; comps.day = d
        guard let date = utc.date(from: comps) else { return nil }
        let back = utc.dateComponents([.year, .month, .day], from: date)
        return back.year == y && back.month == m && back.day == d ? date : nil
    }

    static func iso(_ date: Date) -> String {
        let c = utc.dateComponents([.year, .month, .day], from: date)
        return String(format: "%04d-%02d-%02d", c.year ?? 0, c.month ?? 0, c.day ?? 0)
    }

    static func iso(year: Int, month0: Int, day: Int) -> String {
        String(format: "%04d-%02d-%02d", year, month0 + 1, day)
    }
}

// MARK: Calendar grid

struct SchedCalendarCell: Hashable {
    let iso: String
    let day: Int
    let inMonth: Bool
}

enum SchedCalendarGrid {
    /// Six Monday-first weeks covering the month `month0` (0-based) of `year`.
    static func monthGrid(year: Int, month0: Int) -> [SchedCalendarCell] {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "UTC")!
        var comps = DateComponents()
        comps.year = year; comps.month = month0 + 1; comps.day = 1
        guard let first = cal.date(from: comps) else { return [] }
        let weekday = cal.component(.weekday, from: first) // 1 = Sunday
        let offset = (weekday + 5) % 7 // Monday = 0
        guard let start = cal.date(byAdding: .day, value: -offset, to: first) else { return [] }
        return (0..<42).compactMap { i in
            guard let d = cal.date(byAdding: .day, value: i, to: start) else { return nil }
            let c = cal.dateComponents([.year, .month, .day], from: d)
            return SchedCalendarCell(iso: SchedDate.iso(year: c.year ?? 0, month0: (c.month ?? 1) - 1, day: c.day ?? 0), day: c.day ?? 0, inMonth: (c.month ?? 0) == month0 + 1)
        }
    }

    static func label(year: Int, month0: Int) -> String {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "UTC")!
        var comps = DateComponents()
        comps.year = year; comps.month = month0 + 1; comps.day = 1
        guard let d = cal.date(from: comps) else { return "" }
        let f = DateFormatter()
        f.locale = Locale.current
        f.timeZone = TimeZone(identifier: "UTC")
        f.setLocalizedDateFormatFromTemplate("LLLL yyyy")
        return f.string(from: d)
    }

    static func shift(year: Int, month0: Int, by delta: Int) -> (year: Int, month0: Int) {
        let t = year * 12 + month0 + delta
        return (Int((Double(t) / 12).rounded(.down)), ((t % 12) + 12) % 12)
    }

    static let weekdays = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"]
}

// MARK: Filters

/// The filters of the stripboard and the list. View state only: they decide which strips are
/// shown or faded, and never change the schedule.
struct ScheduleFilter: Equatable {
    var location = ""
    var cast = ""
    /// "" any, "D" day, "N" night.
    var dayNight = ""
    var minEighths: Int?
    var maxEighths: Int?
    enum Scheduled: String { case any, scheduled, unscheduled }
    enum Breakdown: String { case any, complete, incomplete }
    enum Revision: String { case any, changed, current }
    var scheduled: Scheduled = .any
    var breakdown: Breakdown = .any
    var day = ""
    var revision: Revision = .any

    var isActive: Bool { self != ScheduleFilter() }

    enum Length: String, CaseIterable { case any, short, mid, long }

    var length: Length {
        get {
            if minEighths == nil, maxEighths == 8 { return .short }
            if minEighths == 9, maxEighths == 24 { return .mid }
            if minEighths == 25, maxEighths == nil { return .long }
            return .any
        }
        set {
            switch newValue {
            case .short: minEighths = nil; maxEighths = 8
            case .mid: minEighths = 9; maxEighths = 24
            case .long: minEighths = 25; maxEighths = nil
            case .any: minEighths = nil; maxEighths = nil
            }
        }
    }

    func matches(_ s: ScheduleStripDto) -> Bool {
        if !location.isEmpty, !s.locations.contains(where: { $0.key == location }) { return false }
        if !cast.isEmpty, !s.cast.contains(where: { $0.key == cast }) { return false }
        if !dayNight.isEmpty, s.dayNight != dayNight { return false }
        if let lo = minEighths, Int(s.pageEighths) < lo { return false }
        if let hi = maxEighths, Int(s.pageEighths) > hi { return false }
        if scheduled == .scheduled, s.dayId == nil { return false }
        if scheduled == .unscheduled, s.dayId != nil { return false }
        if breakdown == .complete, !s.breakdownComplete { return false }
        if breakdown == .incomplete, s.breakdownComplete { return false }
        if !day.isEmpty, s.dayId != day { return false }
        if revision == .changed, s.sourceState == "Current" { return false }
        if revision == .current, s.sourceState != "Current" { return false }
        return true
    }
}

struct SchedFilterChoice: Hashable, Identifiable {
    let value: String
    let label: String
    var id: String { value }
}

struct SchedFilterOptions {
    var locations: [SchedFilterChoice] = []
    var cast: [SchedFilterChoice] = []
    var days: [SchedFilterChoice] = []
}

extension ScheduleView {
    /// Every strip of the board (days in order, then Unscheduled).
    var allStrips: [ScheduleStripDto] {
        days.flatMap { $0.items.compactMap(\.strip) } + unscheduled
    }

    var filterOptions: SchedFilterOptions {
        var locations: [String: String] = [:]
        var cast: [String: String] = [:]
        for s in allStrips {
            for l in s.locations { locations[l.key] = l.name }
            for c in s.cast { cast[c.key] = c.actor.map { "\(c.character) (\($0))" } ?? c.character }
        }
        func sorted(_ m: [String: String]) -> [SchedFilterChoice] {
            m.map { SchedFilterChoice(value: $0.key, label: $0.value) }.sorted { $0.label.localizedCompare($1.label) == .orderedAscending }
        }
        return SchedFilterOptions(
            locations: sorted(locations), cast: sorted(cast),
            days: days.filter { !$0.isOffDay }.map { SchedFilterChoice(value: $0.id, label: $0.title) })
    }
}

extension ScheduleShootDayDto {
    /// The ids of the day's items, scenes and breaks, in order.
    var itemIds: [String] { items.compactMap { $0.strip?.id ?? $0.marker?.id } }
}

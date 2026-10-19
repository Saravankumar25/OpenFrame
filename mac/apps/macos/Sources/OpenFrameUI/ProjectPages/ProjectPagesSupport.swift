// Shared helpers of the project pages (Files, Notes and Tasks, Activity, Recently Deleted): the
// plain-words formatting that Windows keeps in app/home/format.ts, the jump to another workspace,
// the file panels, and the holding of access to files that live outside the project folder.
// Nothing here decides a rule: the core decides, these helpers only word and route.

import AppKit
import Foundation
import OpenFrameCore
import SwiftUI

// MARK: Wording

enum PageText {
    private static func startOfDay(_ date: Date) -> Date { Calendar.current.startOfDay(for: date) }

    private static func time(_ date: Date) -> String {
        date.formatted(date: .omitted, time: .shortened)
    }

    private static func shortDate(_ date: Date, year: Bool) -> String {
        var style = Date.FormatStyle().day().month(.abbreviated)
        if year { style = style.year() }
        return date.formatted(style)
    }

    /// "Today 10:41", "Yesterday 09:12", "12 Sep 10:02" (lists).
    static func dayAndTime(ms: Double, now: Date = Date()) -> String {
        let date = Date(timeIntervalSince1970: ms / 1000)
        let today = startOfDay(now)
        if date >= today { return "Today \(time(date))" }
        if date >= Calendar.current.date(byAdding: .day, value: -1, to: today) ?? today { return "Yesterday \(time(date))" }
        return "\(shortDate(date, year: false)) \(time(date))"
    }

    /// "Today", "Yesterday", "3 days ago", "12 Sep 2026" (tables).
    static func dayWord(ms: Double, now: Date = Date()) -> String {
        let date = Date(timeIntervalSince1970: ms / 1000)
        let today = startOfDay(now)
        if date >= today { return "Today" }
        let days = Calendar.current.dateComponents([.day], from: startOfDay(date), to: today).day ?? 0
        if days <= 1 { return "Yesterday" }
        if days < 7 { return "\(days) days ago" }
        return shortDate(date, year: true)
    }

    /// "30 days left", "1 day left". The core counts whole days (1 to 30).
    static func daysLeft(_ days: Double) -> String {
        let n = max(1, Int(days.rounded()))
        return "\(n) day\(n == 1 ? "" : "s") left"
    }

    static func fileSize(_ bytes: Double?) -> String {
        guard let bytes else { return "" }
        if bytes < 1024 { return "\(Int(bytes)) B" }
        if bytes < 1024 * 1024 { return "\(Int((bytes / 1024).rounded())) KB" }
        if bytes < 1024 * 1024 * 1024 { return String(format: "%.1f MB", bytes / (1024 * 1024)) }
        return String(format: "%.1f GB", bytes / (1024 * 1024 * 1024))
    }

    // Files

    /// The Type column: PDF, Document, Spreadsheet, Image, Screenplay and so on.
    static func fileKind(mediaType: String, name: String) -> String {
        let lower = name.lowercased()
        if mediaType == "application/pdf" { return "PDF" }
        if mediaType.contains("spreadsheet") || mediaType == "application/vnd.ms-excel" || lower.hasSuffix(".csv") { return "Spreadsheet" }
        if mediaType.contains("presentation") || mediaType == "application/vnd.ms-powerpoint" { return "Presentation" }
        if mediaType.contains("word") || mediaType == "application/rtf" || mediaType == "application/msword" { return "Document" }
        if mediaType.hasPrefix("text/") { return lower.hasSuffix(".fountain") || lower.hasSuffix(".fdx") ? "Screenplay" : "Text" }
        if mediaType == "application/xml", lower.hasSuffix(".fdx") { return "Screenplay" }
        if mediaType.hasPrefix("image/") { return "Image" }
        if lower.hasSuffix(".psd") || lower.hasSuffix(".ai") { return "Artwork" }
        if mediaType.hasPrefix("audio/") { return "Audio" }
        if mediaType.hasPrefix("video/") { return "Video" }
        return "File"
    }

    /// The short text on the file icon.
    static func fileBadge(mediaType: String, name: String) -> String {
        if mediaType == "application/pdf" { return "PDF" }
        if mediaType.contains("word") || mediaType == "application/rtf" || mediaType == "application/msword" { return "DOC" }
        if mediaType.hasPrefix("audio/") { return "AUD" }
        if mediaType.hasPrefix("video/") { return "VID" }
        if mediaType.hasPrefix("image/") { return "IMG" }
        let ext = (name as NSString).pathExtension.uppercased()
        return ext.isEmpty ? "FILE" : String(ext.prefix(4))
    }

    // Notes and tasks

    static func noteLabel(title: String?, body: String) -> String {
        if let t = title?.trimmingCharacters(in: .whitespacesAndNewlines), !t.isEmpty { return t }
        let first = body.split(separator: "\n", omittingEmptySubsequences: true)
            .map { $0.trimmingCharacters(in: .whitespaces) }.first { !$0.isEmpty } ?? ""
        if first.isEmpty { return "Untitled note" }
        return first.count > 60 ? "\(first.prefix(60))\u{2026}" : first
    }

    /// The grey second line of a note row (up to 120 characters of what is not the title).
    static func notePreview(title: String?, body: String) -> String {
        let lines = body.split(separator: "\n", omittingEmptySubsequences: true)
            .map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        let hasTitle = !(title?.trimmingCharacters(in: .whitespaces).isEmpty ?? true)
        let rest = hasTitle ? lines : Array(lines.dropFirst())
        return String(rest.joined(separator: " ").prefix(120))
    }

    static func dueLabel(ms: Double, now: Date = Date()) -> (text: String, overdue: Bool) {
        let date = Date(timeIntervalSince1970: ms / 1000)
        return ("due \(shortDate(date, year: false))", date < startOfDay(now))
    }

    /// A due date is stored at noon of the chosen local day, as on Windows.
    static func dueMs(day: Date) -> Double {
        let noon = Calendar.current.date(bySettingHour: 12, minute: 0, second: 0, of: day) ?? day
        return (noon.timeIntervalSince1970 * 1000).rounded()
    }

    // Activity

    /// "Renamed file to “X”" becomes "renamed file to “X”" so it reads after the person's name.
    static func sentence(summary: String) -> String {
        if summary.range(of: "^[A-Z][a-z]+ed\\b", options: .regularExpression) != nil, let first = summary.first {
            return first.lowercased() + summary.dropFirst()
        }
        return summary
    }

    // Recently Deleted

    /// What the person is told after a restore. The core's own note (the item could not go back
    /// to its place) wins; otherwise the list's view of the item is used.
    static func restoredMessage(_ item: DeletedItemView, restored: DeletedItemRow?) -> String {
        let name = item.title.map { "\u{201C}\($0)\u{201D}" } ?? "the \(item.typeLabel.lowercased())"
        if let note = restored?.restoredNote, !note.isEmpty { return "Restored \(name). \(note)" }
        if item.parentMissing { return "Restored \(name). Its original place no longer exists, so it was put in Unassigned." }
        return "Restored \(name) to \(item.wasIn)"
    }

    /// What a row says when the item's original place is gone. A draft waits for its deleted
    /// screenplay, and a screenplay for its deleted episode: the core refuses those restores
    /// until the parent is back.
    static func parentMissingNote(objectType: String) -> String {
        switch objectType {
        case "screenplay_draft": "Its screenplay is in Recently Deleted too. Restore the screenplay first."
        case "screenplay": "Its episode is in Recently Deleted too. Restore the episode first."
        default: "That place no longer exists, so it restores to Unassigned"
        }
    }
}

// MARK: Going to another workspace

/// What a jump into Notes, Tasks or Files asks the workspace to open.
struct PageIntent: Equatable {
    var noteId: String?
    var taskId: String?
    var fileId: String?
    var isEmpty: Bool { noteId == nil && taskId == nil && fileId == nil }
}

@MainActor
enum PageNav {
    /// Set just before a jump, taken by the workspace that opens.
    static var pending = PageIntent()

    static func take() -> PageIntent {
        defer { pending = PageIntent() }
        return pending
    }

    /// True when the core sent a place to go to.
    static func canOpen(_ nav: JSONValue) -> Bool {
        if case .null = nav { return false }
        return nav.objectValue != nil
    }

    /// Opens the workspace a nav target ({"workspace": ..., "taskId": ...}) names.
    static func open(_ nav: JSONValue) {
        guard canOpen(nav) else { return }
        // The search router already maps every workspace name to a Mac workspace.
        let hit = SearchHit(store: .project, entityType: "", entityId: "", title: "", snippet: "", context: "", nav: nav, rank: 0)
        let target = SearchRouting.target(for: hit)
        guard target.workspace != nil || target.opensSettings else { return }
        pending = PageIntent(noteId: nav["noteId"]?.stringValue, taskId: nav["taskId"]?.stringValue, fileId: nav["fileId"]?.stringValue)
        SearchRouting.go(target)
    }
}

// MARK: Files outside the project

/// Access to files that stay where they are. A linked file lives outside the project folder, so
/// the App Sandbox build must be handed access again each time. The core owns the stored path and
/// opens the file itself; Swift keeps a security-scoped bookmark per linked path and holds access
/// while the core works. Outside the sandbox this does nothing harmful: the bookmark is simply
/// resolved and released.
@MainActor
struct ExternalAccess {
    static let shared = ExternalAccess()
    private let defaults: UserDefaults
    private let key = "openframe.linkedFileBookmarks"

    init(defaults: UserDefaults = .standard) { self.defaults = defaults }

    private var table: [String: Data] {
        get { (defaults.dictionary(forKey: key) as? [String: Data]) ?? [:] }
        nonmutating set { defaults.set(newValue, forKey: key) }
    }

    /// Keeps a bookmark for a file the person just picked. Call while the picker's access is live.
    func remember(_ url: URL) {
        guard let data = try? url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil) else { return }
        var t = table
        t[url.path] = data
        table = t
    }

    func forget(path: String) {
        var t = table
        t[path] = nil
        table = t
    }

    func hasBookmark(for path: String) -> Bool { table[path] != nil }

    /// Runs `body` with access to the file at `path` held, when a bookmark for it exists.
    func holding<T>(path: String?, _ body: () async throws -> T) async rethrows -> T {
        var held: URL?
        if let path, let data = table[path] {
            var stale = false
            if let url = try? URL(resolvingBookmarkData: data, options: .withSecurityScope, relativeTo: nil, bookmarkDataIsStale: &stale),
               url.startAccessingSecurityScopedResource() {
                held = url
                if stale { remember(url) }
            }
        }
        defer { held?.stopAccessingSecurityScopedResource() }
        return try await body()
    }
}

// MARK: Panels

@MainActor
enum PageFilePanels {
    /// The system panel for adding files. Returns the chosen files (access to them is live only
    /// while the caller holds them, so callers start access before using the paths).
    static func chooseFiles(copy: Bool) -> [URL] {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = true
        panel.title = copy ? "Add files to this project" : "Link files to this project"
        panel.prompt = copy ? "Add" : "Link"
        return panel.runModal() == .OK ? panel.urls : []
    }

    static func chooseOne(title: String) -> URL? {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.title = title
        panel.prompt = "Choose"
        return panel.runModal() == .OK ? panel.url : nil
    }

    static func chooseCopyDestination(title: String, name: String) -> URL? {
        let panel = NSSavePanel()
        panel.title = title
        panel.nameFieldStringValue = name
        panel.prompt = "Save"
        return panel.runModal() == .OK ? panel.url : nil
    }
}

// MARK: Page layout pieces

/// The title block every project page starts with: a centred heading, one line of plain words and a row of actions.
struct ProjectPageHeader<Actions: View>: View {
    let title: String
    let subtitle: String
    let identifier: String
    @ViewBuilder var actions: Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: PageSpacing.small) {
            Text(title).font(.system(size: 26, weight: .semibold)).tracking(-0.3)
                .multilineTextAlignment(.center)
                .accessibilityAddTraits(.isHeader)
                .accessibilityIdentifier("\(identifier)-title")
            Text(subtitle).foregroundStyle(theme["muted"])
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("\(identifier)-subtitle")
            actions
                .padding(.top, PageSpacing.tight)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, PageSpacing.large)
        .padding(.top, PageSpacing.page)
        .padding(.bottom, PageSpacing.large)
    }
}

/// A rounded panel the pages put lists in.
struct PageCard<Content: View>: View {
    @ViewBuilder var content: Content
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .center, spacing: 0) { content }
            .frame(maxWidth: .infinity)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).stroke(theme["line"], lineWidth: 1))
    }
}

/// A small coloured label ("Done", "Linked (external)", "Unavailable").
struct PageChip: View {
    enum Tone { case plain, green, blue, red }
    let text: String
    var tone: Tone = .plain
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Text(text)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(background, in: Capsule())
            .foregroundStyle(foreground)
    }

    private var background: Color {
        switch tone {
        case .plain: theme["line"]
        case .green: theme["green-soft"]
        case .blue: theme["blue-soft"]
        case .red: theme["red-soft"]
        }
    }

    private var foreground: Color {
        switch tone {
        case .plain: theme["ink2"]
        case .green: theme["green-t"]
        case .blue: theme["blue-t"]
        case .red: theme["red-t"]
        }
    }
}

extension AppModel {
    /// The toast of a finished change with an Undo button, for the store the change was made in.
    func pageToast(_ text: String, store: StoreSel = .project) {
        ToastCenter.shared.undoable(text, app: self, store: store)
    }

    func pageInfo(_ text: String) {
        ToastCenter.shared.show(text)
    }
}

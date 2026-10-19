// Pure helpers for the screenplay import and export sheets (Windows: features/interchange/model.ts)
// and the file panels every interchange and package sheet uses. No project truth lives here: the
// Rust core reads, writes and decides; these are names, labels and small rules for the screens.

import AppKit
import Foundation
import UniformTypeIdentifiers
import OpenFrameCore

enum IXExportFormat: String, CaseIterable, Identifiable, Sendable {
    case pdf, fdx, fountain, docx, txt

    var id: String { rawValue }
    var label: String {
        switch self {
        case .pdf: "PDF"
        case .fdx: "Final Draft (.fdx)"
        case .fountain: "Fountain"
        case .docx: "DOCX"
        case .txt: "Plain text (.txt)"
        }
    }
    var hint: String? {
        switch self {
        case .pdf: "professional, printable"
        case .docx: "editable"
        case .txt: "screenplay layout in plain characters"
        default: nil
        }
    }
    var fileExtension: String { rawValue }
    /// Shown under "Additional formats" rather than in the main list.
    var additional: Bool { self == .txt }
    static var main: [IXExportFormat] { allCases.filter { !$0.additional } }
}

enum IXImportFilter {
    /// Extensions the Open panel offers for a screenplay.
    static let extensions = ["pdf", "fdx", "fountain", "spmd", "txt", "docx"]
    static let summary = "PDF \u{00B7} Final Draft (.fdx) \u{00B7} Fountain \u{00B7} TXT \u{00B7} DOCX"
}

enum IXNames {
    private static let windowsBad: Set<Character> = ["<", ">", ":", "\"", "/", "\\", "|", "?", "*"]

    /// Suggested export file name: "Black Rain - Draft 6 - Shooting Draft.pdf".
    static func defaultExportFileName(screenplayTitle: String, draftName: String, format: IXExportFormat) -> String {
        let parts = [screenplayTitle, draftName].map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
        var base = parts.joined(separator: " - ")
            .replacingOccurrences(of: "\u{2013}", with: "-")
            .replacingOccurrences(of: "\u{2014}", with: "-")
        base = String(base.map { windowsBad.contains($0) ? " " : $0 })
        base = String(base.unicodeScalars.filter { $0.value >= 32 })
        base = base.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ")
        while let last = base.last, last == "." || last == " " { base.removeLast() }
        if base.isEmpty { base = "Screenplay" }
        if base.count > 100 { base = String(base.prefix(100)).trimmingCharacters(in: .whitespaces) }
        return "\(base).\(format.fileExtension)"
    }

    /// Last component of a path.
    static func fileName(_ path: String) -> String {
        let name = (path as NSString).lastPathComponent
        return name.isEmpty ? path : name
    }

    static func stem(_ name: String) -> String {
        let s = (name as NSString).deletingPathExtension
        return s.isEmpty ? name : s
    }
}

enum IXTone { case plain, green, yellow, red, blue, purple, dark }

enum IXLabels {
    static func sceneStatus(_ status: String) -> (tone: IXTone, label: String) {
        switch status {
        case "ok": (.green, "OK")
        case "uncertain_heading": (.yellow, "Uncertain heading")
        case "potentially_empty": (.yellow, "Potentially empty")
        case "no_heading": (.plain, "Before first scene")
        default: (.plain, status)
        }
    }

    static func confidence(_ c: String) -> (tone: IXTone, label: String) {
        switch c {
        case "high": (.green, "High confidence")
        case "medium": (.yellow, "Medium confidence")
        default: (.red, "Low confidence")
        }
    }

    private static let sourceNoun: [String: String] = [
        "pdf": "PDF", "fdx": "Final Draft file", "fountain": "Fountain file", "txt": "text file",
        "docx": "Word document", "pasted": "pasted text",
    ]

    enum BannerTone { case ok, warn, info }

    /// Headline for the preview banner. Never presents uncertain parsing as certain.
    static func previewBanner(_ p: InterchangeImportPreview) -> (tone: BannerTone, title: String, text: String) {
        let n = p.warnings.count
        let noun = sourceNoun[p.sourceFormat] ?? "file"
        let count = n == 0 ? "No warnings." : "\(n) warning\(n == 1 ? "" : "s")."
        if p.confidence == "low" || p.needsReview {
            return (.warn, count, "This \(noun) was interpreted with limited certainty. Review the items below before importing. You may continue with warnings.")
        }
        if n > 0 {
            return (.info, count, "The screenplay structure was recognised. Some details were adjusted on the way in. See below.")
        }
        return (.ok, count, "The screenplay structure was recognised.")
    }

    static func attentionFirst(_ warnings: [InterchangeWarning]) -> [InterchangeWarning] {
        warnings.filter { $0.level == "attention" } + warnings.filter { $0.level != "attention" }
    }

    static func pages(_ n: Double) -> String { "~\(Int(n))" }

    /// Stage of an import failure for the "Import failed" sheet, from the error code.
    static func failureStage(code: String, pasted: Bool) -> String {
        if code.hasPrefix("import.pdf") { return "Reading text from the PDF" }
        if code == "import.unsupported_format" { return "Checking the file type" }
        if code == "import.unsafe_archive" || code.hasPrefix("import.docx") { return "Opening the document" }
        if code.hasPrefix("import.fdx") { return "Reading the Final Draft file" }
        if code == "import.empty_source" || code == "import.too_large" { return pasted ? "Reading the pasted text" : "Reading the file" }
        if code.hasPrefix("validation") || code.hasPrefix("not_found") { return "Reading the file" }
        if code.hasPrefix("storage") { return "Saving the imported screenplay" }
        return "Interpreting the screenplay"
    }

    /// Failures after which the file can still be kept as an ordinary project file.
    static func canKeepAsProjectFile(code: String) -> Bool {
        code.hasPrefix("import.") && code != "import.too_large" && code != "import.empty_source"
    }

    static func draftLabel(screenplayTitle: String, draftName: String, multiple: Bool) -> String {
        multiple ? "\(screenplayTitle) \u{00B7} \(draftName)" : draftName
    }

    static func statusTone(_ status: String) -> IXTone {
        switch status {
        case "Locked": .green
        case "Review": .blue
        case "Revision": .yellow
        default: .plain
        }
    }

    static func targetLabel(title: String, episodeTitle: String?) -> String {
        episodeTitle.map { "\($0) \u{00B7} \(title)" } ?? title
    }
}

enum IXImportMode: String, Hashable, Sendable {
    case newScreenplay = "new_screenplay"
    case newDraft = "new_draft"
}

enum IXTargets {
    /// Destinations offered in the preview. The current draft is never replaced.
    static func modes(_ p: InterchangeImportPreview) -> [(mode: IXImportMode, label: String)] {
        var out: [(IXImportMode, String)] = []
        if p.canCreateScreenplay { out.append((.newScreenplay, p.episodic ? "New Screenplay for episode" : "New Screenplay")) }
        let n = p.existingScreenplays.count
        if n > 0 { out.append((.newDraft, n == 1 ? "New Draft in \u{201C}\(p.existingScreenplays[0].title)\u{201D}" : "New Draft in\u{2026}")) }
        return out.map { (mode: $0.0, label: $0.1) }
    }

    /// The chosen destination is complete. With several screenplays the one that gets the draft must be chosen.
    static func ready(_ p: InterchangeImportPreview, mode: IXImportMode, screenplayId: String, episodeId: String) -> Bool {
        switch mode {
        case .newScreenplay: p.canCreateScreenplay && (!p.episodic || p.episodes.contains { $0.id == episodeId })
        case .newDraft: p.existingScreenplays.count == 1 || p.existingScreenplays.contains { $0.id == screenplayId }
        }
    }
}

// MARK: File panels

/// How the sheets ask for a file. The real implementation shows NSOpenPanel and NSSavePanel; the
/// panel returns a URL the app may use, which also holds under the App Sandbox. Tests hand in a
/// stub that answers with a path.
@MainActor
protocol FilePicking {
    func openFile(title: String, extensions: [String], message: String?) -> URL?
    func saveFile(title: String, suggestedName: String, directory: URL?, message: String?) -> URL?
}

@MainActor
struct PanelFilePicker: FilePicking {
    func openFile(title: String, extensions: [String], message: String? = nil) -> URL? {
        let panel = NSOpenPanel()
        panel.title = title
        panel.message = message ?? ""
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        if !extensions.isEmpty {
            panel.allowedContentTypes = extensions.compactMap { UTType(filenameExtension: $0) }
            panel.allowsOtherFileTypes = true
        }
        return panel.runModal() == .OK ? panel.url : nil
    }

    func saveFile(title: String, suggestedName: String, directory: URL?, message: String? = nil) -> URL? {
        let panel = NSSavePanel()
        panel.title = title
        panel.message = message ?? ""
        panel.nameFieldStringValue = suggestedName
        panel.canCreateDirectories = true
        if let directory { panel.directoryURL = directory }
        let ext = (suggestedName as NSString).pathExtension
        if !ext.isEmpty, let type = UTType(filenameExtension: ext) {
            panel.allowedContentTypes = [type]
            panel.allowsOtherFileTypes = false
        }
        return panel.runModal() == .OK ? panel.url : nil
    }
}

/// Holds access to a picked file for as long as the sheet that needs it lives. Outside the App
/// Sandbox this is harmless; inside it, the access the panel granted stays valid until released.
final class ScopedAccess: @unchecked Sendable {
    private let lock = NSLock()
    private var held: [URL] = []

    func hold(_ url: URL) {
        guard url.startAccessingSecurityScopedResource() else { return }
        lock.lock(); held.append(url); lock.unlock()
    }

    func release() {
        lock.lock()
        let urls = held
        held = []
        lock.unlock()
        for url in urls { url.stopAccessingSecurityScopedResource() }
    }

    deinit { release() }
}

extension FilePicking {
    func openFile(title: String, extensions: [String]) -> URL? { openFile(title: title, extensions: extensions, message: nil) }
    func saveFile(title: String, suggestedName: String) -> URL? { saveFile(title: title, suggestedName: suggestedName, directory: nil, message: nil) }
}

extension CoreFailure {
    /// The words a sheet shows for a failed call. The core's message is already plain.
    var plain: String { message }
}

// Mac phase M7 test helpers: the shared fixtures of the Rust crate (FDX round-trip suite, hostile
// files), a rig with a real core on a temporary folder, a stub for the file panels, and readers
// for what a draft holds. Used by the Interchange and Packages suites.

import AppKit
import Foundation
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

enum IXFixtures {
    /// crates/openframe-import-export/tests/fixtures, found by walking up from this file (the
    /// package sits in apps/macos in the repository and in a private copy elsewhere).
    static let root: URL = {
        var dir = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        for _ in 0..<12 {
            let candidate = dir.appendingPathComponent("crates/openframe-import-export/tests/fixtures", isDirectory: true)
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
            dir = dir.deletingLastPathComponent()
        }
        fatalError("the shared fixtures folder was not found")
    }()

    static let suiteClasses = [
        "01_basic", "02_dialogue_heavy", "03_parenthetical_heavy", "04_transitions", "05_action_heavy",
        "06_multilingual", "07_custom_elements", "08_revision_marked", "09_scene_numbered_production_draft",
        "10_title_page", "11_explicit_page_breaks", "12_dialogue_spanning_pages", "13_unusual_legacy_constructs",
    ]

    static func suite(_ name: String, _ file: String = "source.fdx") -> URL {
        root.appendingPathComponent("fdx-suite/\(name)/\(file)")
    }
    static func screenplay(_ file: String) -> URL { root.appendingPathComponent("screenplay/\(file)") }
    static func hostile(_ path: String) -> URL { root.appendingPathComponent("hostile/\(path)") }
}

let nightBus = [
    "Title: Night Bus", "",
    "INT. POLICE STATION - NIGHT", "",
    "Arjun confesses to Inspector Rao.", "",
    "ARJUN", "I was there that night.", "",
    "EXT. BUS STOP - NIGHT", "",
    "Arjun meets Meera again at a rainy bus stop.", "",
    "MEERA", "You came back.", "",
    "EXT. RAILWAY STATION - DAY", "",
    "Meera waits for the late train with a red umbrella.", "",
].joined(separator: "\n")

/// A file panel that answers with a prepared path instead of showing.
@MainActor
final class StubPicker: FilePicking {
    var open: URL?
    var save: URL?
    private(set) var openCalls: [(title: String, extensions: [String])] = []
    private(set) var saveCalls: [(title: String, name: String)] = []

    func openFile(title: String, extensions: [String], message: String?) -> URL? {
        openCalls.append((title, extensions))
        return open
    }
    func saveFile(title: String, suggestedName: String, directory: URL?, message: String?) -> URL? {
        saveCalls.append((title, suggestedName))
        return save
    }
}

@MainActor
struct IXRig {
    let root: URL
    let app: AppModel
    let out: URL

    static func make(title: String = "Out And Back") async throws -> IXRig {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: title, projectType: .featureFilm))
        #expect(ok)
        let out = root.appendingPathComponent("exports", isDirectory: true)
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        return IXRig(root: root, app: app, out: out)
    }

    var core: Core { app.core! }
    func file(_ name: String) -> URL { out.appendingPathComponent(name) }
    func close() { app.shutdown(); try? FileManager.default.removeItem(at: root) }

    /// "Import the Night Bus screenplay": paste, Preview, Import, as the sheet does.
    @discardableResult
    func importPasted(_ text: String, draftName: String = "") async throws -> InterchangeImportReport {
        let m = ImportScreenplayModel(app: app)
        m.pasted = text
        await m.runPreview()
        #expect(m.step == .preview)
        m.draftName = draftName
        await m.runImport()
        return try #require(m.report)
    }

    /// Preview a file and import it, as a new screenplay or as a new draft in the first screenplay.
    @discardableResult
    func importFile(_ url: URL, asDraft: String? = nil, keep: Bool = false) async throws -> InterchangeImportReport {
        let m = ImportScreenplayModel(app: app)
        m.choose(file: url)
        await m.runPreview()
        guard m.step == .preview, let p = m.preview else {
            throw CoreFailure(code: m.failure?.code ?? "test", message: m.failure?.message ?? "preview failed")
        }
        if let asDraft, let first = p.existingScreenplays.first {
            m.mode = .newDraft
            m.screenplayId = first.id
            m.draftName = asDraft
        }
        m.keepSource = keep
        await m.runImport()
        guard let r = m.report else { throw CoreFailure(code: "test", message: m.errorText ?? "import failed") }
        return r
    }

    func export(_ draftId: String, _ format: IXExportFormat, to name: String, configure: (ExportScreenplayModel) -> Void = { _ in }) async throws -> InterchangeScreenplayExportResult {
        let m = ExportScreenplayModel(app: app, draftId: draftId)
        await m.load()
        m.format = format
        configure(m)
        let ok = await m.export(to: file(name))
        guard ok, let r = m.result else { throw CoreFailure(code: "test", message: m.error ?? "export failed") }
        return r
    }

    /// "type: text" for every element of the draft, scene headings as "scene_heading: text".
    func lines(_ draftId: String) async throws -> [String] {
        let doc = try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId))
        return doc.scenes.flatMap { s in
            ["scene_heading: \(s.heading)"] + s.elements.map { "\($0.elementType.rawValue): \($0.text)" }
        }
    }

    func headings(_ draftId: String) async throws -> [String] {
        try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId)).scenes.map(\.heading)
    }

    func labels(_ draftId: String) async throws -> [String] {
        try await core.screenplayDocument(ScreenplayDraftIdArgs(draftId: draftId)).scenes.map(\.label)
    }

    func sources() async throws -> InterchangeSources { try await core.interchangeSources() }
}

func bytes(_ url: URL) -> Data { (try? Data(contentsOf: url)) ?? Data() }

// Mac phase M5, Idea Vault: the shared scenarios 20-idea-vault and 21-idea-vault-flow, driven
// through the screen's models against the real core on a temporary folder, plus the rules of the
// presentation helpers, the editor's saving, the recorder's refusals, and rendered snapshots in
// light and dark. (Clicking the real window needs full Xcode and arrives in phase M9.)
//
// Not covered here, with the reason: dragging files from Finder is an operating system drag the
// test driver cannot do (the Windows scenario says the same); the add-files operation it calls is
// tested below. The real microphone is replaced by a recorder that writes a short sound file,
// because a test run must not ask for microphone access.

import AppKit
import AVFoundation
import Foundation
import SwiftUI
import Testing
@testable import OpenFrameCore
@testable import OpenFrameUI

// MARK: Helpers

/// A microphone stand-in that "records" a short sound file.
@MainActor
final class FakeCapture: AudioCapturing {
    var allowed = true
    var seconds: Double = 1.2
    private var file: URL?

    func requestPermission() async -> Bool { allowed }

    func begin() throws {
        file = FileManager.default.temporaryDirectory.appendingPathComponent("fake-voice-\(UUID().uuidString).wav")
        try FakeCapture.wav(seconds: seconds).write(to: file!)
    }

    var elapsed: TimeInterval { seconds }

    func finish() throws -> RecordedAudio {
        RecordedAudio(fileURL: file!, durationMs: seconds * 1000, mediaType: "audio/wav", fileExtension: "wav")
    }

    func cancel() {
        if let file { try? FileManager.default.removeItem(at: file) }
    }

    /// A mono 16-bit 8 kHz WAV with a quiet tone.
    static func wav(seconds: Double) -> Data {
        let rate = 8000
        let count = Int(Double(rate) * seconds)
        var pcm = Data()
        for i in 0..<count {
            let v = Int16(1000 * sin(2 * Double.pi * 440 * Double(i) / Double(rate)))
            pcm.append(contentsOf: withUnsafeBytes(of: v.littleEndian) { Array($0) })
        }
        func le32(_ n: Int) -> Data { withUnsafeBytes(of: UInt32(n).littleEndian) { Data($0) } }
        func le16(_ n: Int) -> Data { withUnsafeBytes(of: UInt16(n).littleEndian) { Data($0) } }
        var d = Data("RIFF".utf8)
        d.append(le32(36 + pcm.count)); d.append(Data("WAVEfmt ".utf8)); d.append(le32(16)); d.append(le16(1)); d.append(le16(1))
        d.append(le32(rate)); d.append(le32(rate * 2)); d.append(le16(2)); d.append(le16(16))
        d.append(Data("data".utf8)); d.append(le32(pcm.count)); d.append(pcm)
        return d
    }
}

@MainActor
private struct Session {
    let root: URL
    var app: AppModel
    var vault: IdeaVaultModel

    /// A first launch with one project, the Idea Vault of that project shown.
    static func start(project: String) async throws -> Session {
        let root = try Fixture.root()
        let app = Fixture.model(root)
        await app.start()
        let ok = await app.createProject(CreateProjectArgs(title: project, projectType: .featureFilm))
        #expect(ok)
        let vault = IdeaVaultModel(app: app, store: .project)
        await vault.reload()
        return Session(root: root, app: app, vault: vault)
    }

    /// Close the project, quit, start again on the same data and open the project.
    mutating func relaunch(project title: String) async throws {
        let recentBefore = try await app.core!.projectListRecent(ListRecentArgs(includeArchived: false))
        guard let row = recentBefore.first(where: { $0.title == title }) else {
            Issue.record("project \(title) is not in the recent list")
            return
        }
        await app.closeProject()
        app.shutdown()
        let next = Fixture.model(root)
        await next.start()
        await next.openProject(path: row.path)
        app = next
        vault = IdeaVaultModel(app: next, store: .project)
        await vault.reload()
    }

    func finish() {
        app.shutdown()
        try? FileManager.default.removeItem(at: root)
    }

    /// Nothing went wrong on screen: no alert, no load error.
    var quiet: Bool { app.alert == nil && vault.loadFailure == nil }
    var names: [String] { vault.items.map(\.displayName) }
    func item(startingWith prefix: String) -> VaultItemDto? { vault.items.first { $0.displayName.hasPrefix(prefix) } }
}

private let lighthouse = "A lighthouse keeper collects letters that never arrived."

@Suite("IdeaVaultTests", .serialized)
@MainActor
struct IdeaVaultTests {
    // MARK: Scenario 20: basics

    @Test func quickCaptureAddsAnIdeaToTheProjectVault() async throws {
        var s = try await Session.start(project: "Lantern Coast")
        defer { s.finish() }
        #expect(s.vault.items.isEmpty)
        #expect(s.vault.loaded)
        try await s.vault.quickCapture(lighthouse)
        #expect(s.vault.notice?.text == "Saved to the Project Idea Vault")
        #expect(s.vault.items.count == 1)
        #expect(s.names[0].hasPrefix("A lighthouse keeper"))
        #expect(s.vault.items[0].itemType == .note)
        #expect(s.vault.items[0].body == lighthouse)
        #expect(s.quiet)
    }

    @Test func aTitleInTheDrawerSavesAutomaticallyThenUndoAndRedo() async throws {
        var s = try await Session.start(project: "Lantern Coast")
        defer { s.finish() }
        try await s.vault.quickCapture(lighthouse)
        let earlier = s.names[0]
        let editor = ItemEditorModel(vault: s.vault, id: s.vault.items[0].id)
        s.vault.open(editor.id)
        await editor.refresh()
        #expect(editor.draft.body == lighthouse)
        #expect(editor.statusText == "Saved automatically")

        editor.edit(.title, to: "Lighthouse keeper")
        #expect(editor.status == .saving)
        let saved = await editor.flush()
        #expect(saved)
        #expect(editor.statusText == "Saved automatically")
        s.vault.closeDrawer()
        #expect(s.vault.openId == nil)
        #expect(s.names == ["Lighthouse keeper"])

        await s.app.undo()
        await s.vault.reload()
        #expect(s.names == [earlier])
        await s.app.redo()
        await s.vault.reload()
        #expect(s.names == ["Lighthouse keeper"])
        #expect(s.quiet)
    }

    @Test func deleteUndoRedoThenRestoreFromRecentlyDeleted() async throws {
        var s = try await Session.start(project: "Lantern Coast")
        defer { s.finish() }
        try await s.vault.quickCapture(lighthouse)
        let editor = ItemEditorModel(vault: s.vault, id: s.vault.items[0].id)
        await editor.refresh()
        editor.edit(.title, to: "Lighthouse keeper")
        await editor.flush()

        // Click the card, press Delete.
        let card = try #require(s.item(startingWith: "Lighthouse keeper"))
        s.vault.select(card, command: false, shift: false)
        #expect(s.vault.selectedItems.map(\.id) == [card.id])
        await s.vault.delete(s.vault.selectedItems)
        #expect(s.vault.notice?.text == "Deleted “Lighthouse keeper”. You can restore from Recently Deleted.")
        #expect(s.vault.items.isEmpty)
        #expect(s.vault.selection.isEmpty)

        await s.app.undo()
        await s.vault.reload()
        #expect(s.names == ["Lighthouse keeper"])
        await s.app.redo()
        await s.vault.reload()
        #expect(s.vault.items.isEmpty)

        s.vault.page = .deleted
        await s.vault.reloadTrash()
        let row = try #require(s.vault.trash.first { $0.deleted.title == "Lighthouse keeper" })
        #expect(row.typeLabel == "Note")
        #expect(VaultText.daysLeft(row.daysLeft).hasSuffix("left"))
        await s.vault.restore(row)
        s.vault.page = .vault
        #expect(s.names == ["Lighthouse keeper"])
        #expect(s.vault.trash.isEmpty)
        #expect(s.quiet)
    }

    @Test func purgeRemovesAnItemForGood() async throws {
        var s = try await Session.start(project: "Lantern Coast")
        defer { s.finish() }
        try await s.vault.quickCapture("Short-lived idea")
        await s.vault.delete(s.vault.items)
        await s.vault.reloadTrash()
        let row = try #require(s.vault.trash.first)
        await s.vault.purge(row)
        #expect(s.vault.trash.isEmpty)
        #expect(s.vault.notice?.text.hasPrefix("Permanently deleted") == true)
        await s.vault.reload()
        #expect(s.vault.items.isEmpty)
        #expect(s.quiet)
    }

    @Test func afterARelaunchTheTitledIdeaAndItsTextAreKept() async throws {
        var s = try await Session.start(project: "Lantern Coast")
        defer { s.finish() }
        try await s.vault.quickCapture(lighthouse)
        let editor = ItemEditorModel(vault: s.vault, id: s.vault.items[0].id)
        await editor.refresh()
        editor.edit(.title, to: "Lighthouse keeper")
        await editor.flush()

        try await s.relaunch(project: "Lantern Coast")
        if case .recovery? = s.app.alert { Issue.record("a recovery question showed after a clean close") }
        #expect(s.names == ["Lighthouse keeper"])
        let reopened = ItemEditorModel(vault: s.vault, id: s.vault.items[0].id)
        await reopened.refresh()
        #expect(reopened.draft.body == lighthouse)
        #expect(reopened.draft.title == "Lighthouse keeper")
        #expect(s.quiet)
    }

    // MARK: Scenario 21: full flow

    @Test func aVoiceNoteIsRecordedKeptAndPlaysAfterARelaunch() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        let recorder = VoiceNoteModel(capture: FakeCapture())
        #expect(recorder.canStart)
        await recorder.start()
        #expect(recorder.phase == .recording)
        #expect(recorder.statusText == "Recording…")
        recorder.name = "Night bus idea"
        try await Task.sleep(for: .milliseconds(400))
        let saved = await recorder.save(into: s.vault)
        let item = try #require(saved)
        #expect(item.itemType == .voice)
        #expect(item.asset?.durationMs ?? 0 > 0)
        #expect(s.names.contains { $0.hasPrefix("Night bus idea") })
        #expect(s.quiet)

        try await s.relaunch(project: "Letter Town")
        let again = try #require(s.item(startingWith: "Night bus idea"))
        let asset = try #require(again.asset)
        #expect(asset.available)
        let url = try await s.app.core!.assetFile(asset.id, global: false)
        let player = AudioPlayerModel()
        player.load(url)
        #expect(player.loaded && !player.failed)
        #expect(player.duration > 0)
        #expect(VaultText.metaLine(again, collections: []).hasPrefix("Voice note · 0:01"))
    }

    @Test func theMicrophoneBeingRefusedIsExplainedAndNothingIsStored() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        let fake = FakeCapture()
        fake.allowed = false
        let recorder = VoiceNoteModel(capture: fake)
        await recorder.start()
        guard case let .blocked(message, canOpenSettings) = recorder.phase else {
            Issue.record("expected a blocked recorder, got \(recorder.phase)")
            return
        }
        #expect(canOpenSettings)
        #expect(message.contains("microphone"))
        #expect(message.contains("audio file"))
        #expect(!recorder.canSave)
        let saved = await recorder.save(into: s.vault)
        #expect(saved == nil)
        #expect(s.vault.items.isEmpty)
    }

    @Test func researchFlagsSetShownAsWordsUndoneAndChanged() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        try await s.vault.quickCapture("Police procedure: order of evidence handling at a small station.")
        let item = try #require(s.item(startingWith: "Police procedure"))

        await s.vault.setResearchFlag([item], .needsExpertCheck)
        #expect(s.vault.notice?.text.contains("as Needs Expert Check") == true)
        #expect(VaultText.researchFlag(s.item(startingWith: "Police procedure")?.researchFlag)?.label == "Needs Expert Check")

        await s.vault.setResearchFlag([item], .checked)
        let checked = VaultText.researchFlag(s.item(startingWith: "Police procedure")?.researchFlag)?.label
        #expect(checked == "Checked")
        #expect(checked?.contains("Needs Expert") == false)

        await s.app.undo()
        await s.vault.reload()
        #expect(s.item(startingWith: "Police procedure")?.researchFlag == .needsExpertCheck)
        await s.app.redo()
        await s.vault.reload()
        #expect(s.item(startingWith: "Police procedure")?.researchFlag == .checked)

        // The drawer shows the same flag and the honesty line.
        let editor = ItemEditorModel(vault: s.vault, id: item.id)
        await editor.refresh()
        #expect(editor.item?.researchFlag == .checked)
        #expect(VaultHints.researchFlag == "Verification flags are yours to set. OpenFrame never marks research as true just because you imported it.")
        #expect(s.quiet)

        await s.vault.setResearchFlag([item], nil)
        #expect(s.item(startingWith: "Police procedure")?.researchFlag == nil)
        #expect(s.vault.notice?.text.hasPrefix("Cleared the research flag") == true)
    }

    @Test func theGlobalVaultCapturesAndCopiesIntoTheProject() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        s.vault.chooseStore(.global)
        #expect(s.vault.scope == .global)
        #expect(s.vault.copyLabel == "Copy to Project")
        await s.vault.reload()
        try await s.vault.quickCapture("A city where every letter is delivered one year late.")
        #expect(s.vault.notice?.text == "Saved to the Global Idea Vault")
        let idea = try #require(s.item(startingWith: "A city where every letter"))

        try await s.vault.copy([idea], collectionId: nil)
        #expect(s.vault.notice?.text.contains("to the Project Idea Vault") == true)
        // The original stays in Global.
        #expect(s.item(startingWith: "A city where every letter") != nil)

        s.vault.chooseStore(.project)
        #expect(s.vault.copyLabel == "Copy to Global Vault")
        await s.vault.reload()
        let copy = try #require(s.item(startingWith: "A city where every letter"))
        #expect(copy.store == .project)
        #expect(copy.sourceGlobalItemId == idea.id)
        #expect(s.quiet)

        try await s.relaunch(project: "Letter Town")
        #expect(s.item(startingWith: "A city where every letter") != nil)
        s.vault.chooseStore(.global)
        await s.vault.reload()
        #expect(s.item(startingWith: "A city where every letter") != nil)
        #expect(s.quiet)
    }

    @Test func theGlobalVaultIsTheOnlyOneWhenNoProjectIsOpen() async throws {
        let root = try Fixture.root()
        defer { try? FileManager.default.removeItem(at: root) }
        let app = Fixture.model(root)
        await app.start()
        defer { app.shutdown() }
        #expect(app.phase == .home)
        let vault = IdeaVaultModel(app: app, store: .project)
        #expect(vault.scope == .global)
        #expect(!vault.hasProject)
        #expect(vault.copyLabel == nil)
        await vault.reload()
        try await vault.quickCapture("A note with no project open")
        #expect(vault.items.count == 1)
        #expect(vault.items[0].store == .global)
        // The Global vault has its own history.
        await vault.undo()
        #expect(vault.items.isEmpty)
        await vault.redo()
        #expect(vault.items.count == 1)
        #expect(app.alert == nil)
    }

    @Test func sendToStoryMakesASceneCardAndTheVaultOriginalStays() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        let idea = "Meera finds her own handwriting on an old envelope."
        try await s.vault.quickCapture(idea)
        let item = try #require(s.item(startingWith: "Meera finds her own"))
        await s.vault.loadStoryTargets()
        try await s.vault.sendToStory(item, target: .sceneCard, parentType: "parking", parentId: nil)
        #expect(s.vault.notice?.action == .openStory)
        #expect(s.vault.notice?.text.isEmpty == false)
        await s.vault.reload()
        #expect(s.item(startingWith: "Meera finds her own") != nil)

        let board = try await s.app.core!.storyBoard(StoryScopeArgs())
        let cards: [String] = board.parking.compactMap { if case let .card(c) = $0 { c.shortDescription } else { nil } }
        #expect(cards.contains(idea))
        #expect(s.quiet)
    }

    // MARK: Beyond the shared scenarios

    @Test func foldersCollectionsTagsPinsAndSearchOrganiseWithoutChangingItems() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        try await s.vault.quickCapture("Rain on the tin roof")
        try await s.vault.quickCapture("Harbour at dawn")
        let rain = try #require(s.item(startingWith: "Rain on"))
        let harbour = try #require(s.item(startingWith: "Harbour"))

        try await s.vault.createFolder(name: "Locations", parentId: nil)
        let folder = try #require(s.vault.folders.first)
        try await s.vault.move([harbour], to: folder.id)
        #expect(s.vault.notice?.text == "Moved “Harbour at dawn” to Locations")
        s.vault.show(.folder(folderId: folder.id))
        await s.vault.reload()
        #expect(s.names == ["Harbour at dawn"])
        // Folder mode shows the same folder through the cursor, and sub-folders under it.
        s.vault.setMode(.folder)
        #expect(s.vault.folderCursor == folder.id)
        s.vault.openFolder(nil)
        await s.vault.reload()
        #expect(s.vault.folders.map(\.name) == ["Locations"])
        #expect(s.names == ["Rain on the tin roof"])
        s.vault.setMode(.grid)
        s.vault.show(.all)

        let made = try await s.vault.createCollection(name: "Visual References")
        try await s.vault.addToCollection([rain], collectionId: try #require(made).id)
        try await s.vault.addTags([rain, harbour], VaultText.parseTags("rain, #night, Rain"))
        await s.vault.setPinned([rain], true)
        s.vault.show(.all)
        await s.vault.reload()
        let sections = VaultSections.build(s.vault.items, splitPinned: true)
        #expect(sections.map(\.header) == ["Pinned", "Everything else"])
        #expect(s.vault.overview?.tags.map(\.tag).sorted() == ["night", "rain"])
        #expect(s.vault.overview?.pinned == 1)

        s.vault.searchText = "harbour"
        s.vault.applySearch()
        await s.vault.reload()
        #expect(s.names == ["Harbour at dawn"])
        #expect(s.vault.searching)
        s.vault.searchText = "zzz-nothing"
        s.vault.applySearch()
        await s.vault.reload()
        #expect(s.vault.items.isEmpty)

        // Deleting a folder keeps its items.
        s.vault.searchText = ""
        s.vault.applySearch()
        await s.vault.deleteFolder(folder)
        await s.vault.reload()
        #expect(s.vault.items.count == 2)
        #expect(s.vault.notice?.text.hasPrefix("Deleted folder “Locations”. Its items were kept") == true)
        #expect(s.quiet)
    }

    @Test func addingFilesKeepsTheGoodOnesAndListsTheFailures() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        let dir = s.root.appendingPathComponent("incoming", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let text = dir.appendingPathComponent("reference notes.txt")
        try "Shot list ideas".write(to: text, atomically: true, encoding: .utf8)
        let png = dir.appendingPathComponent("sketch.png")
        let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 8, pixelsHigh: 8, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        try rep.representation(using: .png, properties: [:])!.write(to: png)

        let clean = await s.vault.addFiles([text.path, png.path])
        #expect(clean?.added.count == 2)
        #expect(clean?.failed.isEmpty == true)
        #expect(s.vault.notice?.text == "Added 2 items")
        let image = try #require(s.vault.items.first { $0.itemType == .image })
        #expect(image.asset?.storageMode == "managed")
        // The picture can be read through the managed file the core hands back.
        let loaded = await VaultThumbnails.shared.image(core: s.app.core!, assetId: image.asset!.id, global: false, maxPixel: 64)
        #expect(loaded != nil)

        let mixed = await s.vault.addFiles([dir.appendingPathComponent("missing.pdf").path, text.path])
        #expect(mixed?.failed.count == 1)
        #expect(mixed?.added.count == 1)
        guard case let .result(result)? = s.vault.dialog else {
            Issue.record("expected the add result sheet")
            return
        }
        #expect(result.failed[0].name == "missing.pdf")
        #expect(!result.failed[0].reason.isEmpty)
        #expect(s.vault.items.count == 3)
    }

    @Test func aLinkItemKeepsTheLinkAndRefusesABadOne() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        let item = try await s.vault.addURL("https://example.com/lighthouses", note: "Keeper interviews")
        #expect(item.itemType == .url)
        #expect(item.url == "https://example.com/lighthouses")
        #expect(VaultText.host(of: item.url) == "example.com/lighthouses")
        let vault = s.vault
        do {
            _ = try await vault.addURL("not a link at all", note: "")
            Issue.record("a bad link was accepted")
        } catch {
            #expect(!CoreFailure(asCore: error).message.isEmpty)
        }
        #expect(s.vault.items.count == 1)

        // A bad link typed into the drawer is explained next to the field, not as an alert.
        let editor = ItemEditorModel(vault: s.vault, id: item.id)
        await editor.refresh()
        editor.edit(.url, to: "ftp//broken")
        await editor.flush()
        #expect(editor.urlError != nil || editor.status == .saved)
    }

    @Test func theEditorKeepsTypingAndTakesOtherChangesOnlyForFieldsNotBeingEdited() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        try await s.vault.quickCapture("First draft")
        let id = s.vault.items[0].id
        let editor = ItemEditorModel(vault: s.vault, id: id)
        await editor.refresh()

        // Something else (another window, undo) renames the item while the body is being typed.
        editor.edit(.body, to: "First draft, and more")
        _ = try await s.app.core!.vaultUpdate(VaultUpdateArgs(store: .project, id: id, title: "Renamed elsewhere"))
        await editor.refresh()
        #expect(editor.draft.title == "Renamed elsewhere")
        #expect(editor.draft.body == "First draft, and more")
        await editor.flush()
        #expect(editor.status == .saved)
        #expect(s.vault.items.first?.body == "First draft, and more")

        // Closing right after typing saves at once, without waiting for the pause.
        editor.edit(.caption, to: "kept")
        #expect(editor.status == .saving)
        #expect(await editor.flush())
        await editor.refresh()
        #expect(editor.item?.caption == "kept")

        // An item that is gone says so.
        await s.vault.delete(s.vault.items)
        let gone = ItemEditorModel(vault: s.vault, id: id)
        await gone.refresh()
        #expect(gone.missing)
    }

    @Test func aSlowerOlderAnswerNeverReplacesANewerList() async throws {
        var s = try await Session.start(project: "Letter Town")
        defer { s.finish() }
        try await s.vault.quickCapture("One")
        let vault = s.vault
        async let first: Void = vault.reload()
        vault.searchText = "zzz"
        vault.applySearch()
        async let second: Void = vault.reload()
        _ = await (first, second)
        #expect(s.vault.items.isEmpty)
    }

    // MARK: Presentation rules

    @Test func presentationHelpersFollowTheWindowsRules() {
        #expect(VaultText.duration(ms: 42_000) == "0:42")
        #expect(VaultText.duration(ms: 725_000) == "12:05")
        #expect(VaultText.duration(ms: 3_723_000) == "1:02:03")
        #expect(VaultText.duration(ms: nil) == nil)
        #expect(VaultText.duration(ms: -1) == nil)
        #expect(VaultText.extLabel("script.final.docx") == "DOCX")
        #expect(VaultText.extLabel("README") == "FILE")
        #expect(VaultText.host(of: "https://example.com/a/b?x=1") == "example.com/a/b?x=1")
        #expect(VaultText.host(of: "https://example.com/") == "example.com")
        #expect(VaultText.parseTags("rain, #Night,\nrain , two  words") == ["rain", "Night", "two words"])
        #expect(VaultText.daysLeft(0.2) == "1 day left")
        #expect(VaultText.daysLeft(29.6) == "30 days left")
        #expect(VaultText.scopeLabel(.global) == "Global Idea Vault")
        #expect(VaultText.researchFlags.map(\.label) == ["Unchecked", "Checked", "Needs Expert Check"])

        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "UTC")!
        let now = Date(timeIntervalSince1970: 1_760_000_000)  // 9 Oct 2025
        func ms(_ daysAgo: Double) -> Double { (now.timeIntervalSince1970 - daysAgo * 86_400) * 1000 }
        #expect(VaultText.relativeDay(ms: ms(0), now: now, calendar: cal) == "today")
        #expect(VaultText.relativeDay(ms: ms(1), now: now, calendar: cal) == "yesterday")
        #expect(VaultText.relativeDay(ms: ms(10), now: now, calendar: cal) == "29 Sep")
        #expect(VaultText.relativeDay(ms: ms(400), now: now, calendar: cal) == "4 Sep 2024")
    }

    @Test func foldersFormATreeWithoutLoopsAndOrphansGoToTheTop() {
        func f(_ id: String, _ parent: String?) -> VaultFolderDto { VaultFolderDto(id: id, name: id, parentId: parent, itemCount: 0, rev: 1) }
        let tree = VaultFolders.tree([f("a", nil), f("b", "a"), f("c", "b"), f("orphan", "missing")])
        #expect(tree.map(\.folder.id) == ["a", "b", "c", "orphan"])
        #expect(tree.map(\.depth) == [0, 1, 2, 0])
        #expect(VaultFolders.path([f("a", nil), f("b", "a")], to: "b").map(\.id) == ["a", "b"])
        // A loop in bad data ends instead of hanging.
        #expect(VaultFolders.path([f("x", "y"), f("y", "x")], to: "x").count == 2)
    }

    @Test func selectionFollowsClickCommandAndShift() {
        let order = ["a", "b", "c", "d", "e"]
        var s = VaultSelection.next(current: [], ordered: order, id: "b", anchor: nil, command: false, shift: false)
        #expect(s == ["b"])
        s = VaultSelection.next(current: s, ordered: order, id: "d", anchor: "b", command: false, shift: true)
        #expect(s == ["b", "c", "d"])
        s = VaultSelection.next(current: s, ordered: order, id: "c", anchor: "b", command: true, shift: false)
        #expect(s == ["b", "d"])
        s = VaultSelection.next(current: ["a"], ordered: order, id: "e", anchor: "c", command: true, shift: true)
        #expect(s == ["a", "c", "d", "e"])
    }

    @Test func dragPayloadsRoundTripAndForeignTextIsIgnored() {
        let text = VaultDragPayload.encode(store: .global, ids: ["x1", "x2"])
        let back = VaultDragPayload.decode(text)
        #expect(back?.store == .global)
        #expect(back?.ids == ["x1", "x2"])
        #expect(VaultDragPayload.decode("just some text") == nil)
        #expect(VaultDragPayload.decode("openframe-vault:other:a") == nil)
    }

    // MARK: Rendering

    @Test func screensRenderInLightAndDark() async throws {
        var s = try await Session.start(project: "Night Train")
        defer { s.finish() }
        let m = s.vault
        try await m.quickCapture("The conductor has no ticket of his own.")
        try await m.quickCapture("Rain on the tin roof. A long line that keeps going so the card has to wrap onto several lines and be clipped by the card height.")
        _ = try await m.addURL("https://example.com/night-trains", note: "How the sleeper services run")
        let png = s.root.appendingPathComponent("platform.png")
        let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 320, pixelsHigh: 200, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
        NSColor.systemTeal.setFill()
        NSRect(x: 0, y: 0, width: 320, height: 200).fill()
        NSColor.white.setFill()
        NSRect(x: 40, y: 40, width: 120, height: 80).fill()
        NSGraphicsContext.restoreGraphicsState()
        try rep.representation(using: .png, properties: [:])!.write(to: png)
        _ = await m.addFiles([png.path])
        let recorder = VoiceNoteModel(capture: FakeCapture())
        await recorder.start()
        recorder.name = "Night bus idea"
        _ = await recorder.save(into: m)
        let note = try #require(s.item(startingWith: "The conductor"))
        await m.setPinned([note], true)
        await m.setResearchFlag([try #require(s.item(startingWith: "Rain on"))], .needsExpertCheck)
        try await m.createFolder(name: "Locations", parentId: nil)
        _ = try await m.createCollection(name: "Visual References")
        try await m.addTags([note], ["rain", "night"])
        await m.reload()
        m.say("Saved to the Project Idea Vault", undo: .project)
        m.select(note, command: false, shift: false)

        let themes = ThemeStore()
        for (scheme, name) in [(ColorScheme.light, "light"), (.dark, "dark")] {
            let theme = themes.resolvedFor(colorScheme: scheme)
            func hosted<V: View>(_ view: V) -> some View {
                view.environment(s.app).environment(themes).environment(\.ofTheme, theme).environment(\.colorScheme, scheme)
            }
            for mode in [VaultDisplayMode.grid, .list] {
                UserDefaults.standard.set(mode.rawValue, forKey: "vault.mode")
                m.mode = mode
                try await snapshot(hosted(IdeaVaultScreen(model: m)), name: "idea-vault-\(mode.rawValue)-\(name)")
            }
            UserDefaults.standard.set("grid", forKey: "vault.mode")
            m.mode = .grid
            m.open(note.id)
            try await snapshot(hosted(VaultDrawer(model: m, id: note.id)), name: "idea-vault-drawer-note-\(name)", size: CGSize(width: 400, height: 780))
            if let image = m.items.first(where: { $0.itemType == .image }) {
                m.open(image.id)
                try await snapshot(hosted(VaultDrawer(model: m, id: image.id)), name: "idea-vault-drawer-image-\(name)", size: CGSize(width: 400, height: 780))
            }
            if let voice = m.items.first(where: { $0.itemType == .voice }) {
                m.open(voice.id)
                try await snapshot(hosted(VaultDrawer(model: m, id: voice.id)), name: "idea-vault-drawer-voice-\(name)", size: CGSize(width: 400, height: 780))
            }
            m.closeDrawer()
            m.dialog = .quick
            try await snapshot(hosted(QuickCaptureSheet(model: m, close: {})), name: "idea-vault-quick-\(name)", size: CGSize(width: 480, height: 300))
            m.dialog = nil
            await m.delete([note])
            m.page = .deleted
            await m.reloadTrash()
            try await snapshot(hosted(VaultDeletedView(model: m)), name: "idea-vault-deleted-\(name)")
            m.page = .vault
            await m.reload()
            if let row = m.trash.first { await m.restore(row) }
            await m.reload()
        }
        // The empty Global vault.
        let empty = IdeaVaultModel(app: s.app, store: .global)
        await empty.reload()
        let theme = themes.resolvedFor(colorScheme: .light)
        try await snapshot(
            IdeaVaultScreen(model: empty).environment(s.app).environment(themes).environment(\.ofTheme, theme),
            name: "idea-vault-empty-global")
    }
}

/// A real window-backed render, so lists, text editors and the inspector draw as they do on screen.
@MainActor
private func snapshot<V: View>(_ view: V, name: String, size: CGSize = CGSize(width: 1280, height: 780)) async throws {
    let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height))
    host.frame = NSRect(origin: .zero, size: size)
    let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
    window.contentView = host
    window.appearance = nil
    host.layoutSubtreeIfNeeded()
    // Let loads that start on appear (pictures, the open item) finish.
    for _ in 0..<12 {
        try await Task.sleep(for: .milliseconds(60))
        host.layoutSubtreeIfNeeded()
    }
    guard let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else {
        Issue.record("could not draw \(name)")
        return
    }
    host.cacheDisplay(in: host.bounds, to: rep)
    try FileManager.default.createDirectory(at: Fixture.snapshots, withIntermediateDirectories: true)
    if let png = rep.representation(using: .png, properties: [:]) {
        try png.write(to: Fixture.snapshots.appendingPathComponent("\(name).png"))
    }
    #expect(rep.pixelsWide > 0)
}

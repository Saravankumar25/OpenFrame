// Build Screenplay from the Story Board and Apply Board Order to the written screenplay. The
// preview comes first and nothing is created until the person confirms. An existing screenplay
// is never overwritten: the build adds a new draft or a new screenplay beside the others.
// Owner decision 2026-10-07: a card without a scene heading is never built silently and no
// heading is ever invented. The preview asks for each one: add a heading, or leave the card out.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class BuildScreenplayModel {
    let story: StoryModel
    let preselect: [String]

    private(set) var preview: StoryBuildPreview?
    private(set) var failure: String?
    private(set) var busy = false

    var included: Set<String> = []
    var headings: [String: String] = [:]
    var mode: StoryDescriptionMode = .planningNote
    var destination: StoryBuildDestination = .newScreenplay
    var screenplayId = ""
    var title = ""

    init(story: StoryModel, preselect: [String] = []) {
        self.story = story
        self.preselect = preselect
    }

    func load() async {
        guard let core = story.app.core else { return }
        do {
            let p = try await core.storyBuildPreview(StoryBuildPreviewArgs(episodeId: story.episodeId))
            preview = p
            failure = nil
            included = Set(preselect.isEmpty ? p.rows.map(\.cardId) : preselect)
            destination = p.screenplays.isEmpty ? .newScreenplay : .newDraft
            screenplayId = p.screenplays.first?.id ?? ""
            title = p.defaultTitle
        } catch {
            failure = CoreFailure(asCore: error).message
        }
    }

    var rows: [StoryBuildRow] { preview?.rows ?? [] }
    var existing: [StoryScreenplayOption] { preview?.screenplays ?? [] }
    var target: StoryScreenplayOption? { existing.first { $0.id == screenplayId } }

    func heading(for row: StoryBuildRow) -> String { headings[row.cardId] ?? row.sceneHeading ?? "" }
    func isEdited(_ row: StoryBuildRow) -> Bool { headings[row.cardId] != nil }
    func headingOK(_ row: StoryBuildRow) -> Bool { StoryLogic.headingIsValid(heading(for: row)) }

    var chosen: [StoryBuildRow] { rows.filter { included.contains($0.cardId) } }
    var missing: Int { chosen.filter { !headingOK($0) }.count }
    var titleClash: Bool {
        let t = title.trimmingCharacters(in: .whitespaces).lowercased()
        return existing.contains { $0.title.trimmingCharacters(in: .whitespaces).lowercased() == t }
    }

    var canBuild: Bool {
        guard !chosen.isEmpty, missing == 0, !busy else { return false }
        return destination == .newScreenplay
            ? !title.trimmingCharacters(in: .whitespaces).isEmpty && !titleClash
            : target != nil
    }

    var summary: String {
        var s = "\(chosen.count) of \(rows.count) card\(rows.count == 1 ? "" : "s") will be built"
        if missing > 0 { s += " \u{00B7} \(missing) need\(missing == 1 ? "s" : "") a heading" }
        return s
    }

    var missingBanner: String {
        "\(missing == 1 ? "1 card has" : "\(missing) cards have") no valid scene heading. Add a heading (for example INT. POLICE STATION - NIGHT) or leave \(missing == 1 ? "the card" : "those cards") out. OpenFrame never makes up a heading."
    }

    func leaveOut(_ cardId: String) { included.remove(cardId) }
    func setIncluded(_ cardId: String, _ on: Bool) {
        if on { included.insert(cardId) } else { included.remove(cardId) }
    }
    func setHeading(_ cardId: String, _ text: String) { headings[cardId] = text }

    /// Builds the draft. On success the Story workspace says so and opens the Screenplay.
    @discardableResult
    func build() async -> StoryBuildResult? {
        guard canBuild, let core = story.app.core else { return nil }
        busy = true
        defer { busy = false }
        do {
            let include = chosen.map { row -> StoryBuildInclude in
                let typed = headings[row.cardId]
                let h = typed.flatMap { $0.trimmingCharacters(in: .whitespaces).isEmpty ? nil : $0 }
                return StoryBuildInclude(cardId: row.cardId, heading: h)
            }
            let result = try await core.storyBuildScreenplay(StoryBuildArgs(
                episodeId: story.episodeId, include: include, descriptionMode: mode, destination: destination,
                screenplayId: destination == .newDraft ? screenplayId : nil,
                title: destination == .newScreenplay ? title.trimmingCharacters(in: .whitespaces) : nil,
                draftName: nil))
            story.didBuild(result)
            return result
        } catch {
            story.fail("Could not build the screenplay", error)
            return nil
        }
    }
}

extension StoryModel {
    /// After a build: say so, then show the new draft in the Screenplay workspace.
    func didBuild(_ r: StoryBuildResult) {
        let n = Int(r.sceneCount)
        say("Built \u{201C}\(r.draftName)\u{201D} with \(n) scene\(n == 1 ? "" : "s")", undoable: true)
        sheet = nil
        NotificationCenter.default.post(
            name: .openFrameShowScreenplayTarget, object: nil,
            userInfo: [
                "episodeId": episodeId as Any, "screenplayId": r.screenplayId, "draftId": r.draftId,
                "sceneId": r.firstSceneId,
            ])
        NotificationCenter.default.post(name: .openFrameShowWorkspace, object: Workspace.screenplay.rawValue)
    }
}

extension Notification.Name {
    /// Posted with `userInfo` keys screenplayId, draftId, sceneId and episodeId when another
    /// workspace wants the Screenplay to open a particular draft and scene.
    static let openFrameShowScreenplayTarget = Notification.Name("studio.openframe.showScreenplayTarget")
}

// MARK: Apply board order

@MainActor
@Observable
final class ApplyOrderModel {
    let story: StoryModel
    private(set) var preview: StoryOrderPreview?
    private(set) var failure: String?
    private(set) var busy = false
    /// The draft the person chose when the project holds several screenplays.
    var draftChoice: String?

    init(story: StoryModel) { self.story = story }

    func load() async {
        guard let core = story.app.core else { return }
        do {
            preview = try await core.storyOrderPreview(StoryOrderArgs(episodeId: story.episodeId, draftId: draftChoice, confirmed: false))
            failure = nil
        } catch { failure = CoreFailure(asCore: error).message }
    }

    func choose(_ draftId: String) async {
        draftChoice = draftId
        await load()
    }

    var canApply: Bool {
        guard let p = preview else { return false }
        return !busy && p.draftId != nil && !p.moves.isEmpty && !p.locked
    }

    @discardableResult
    func apply() async -> Bool {
        guard let p = preview, canApply, let core = story.app.core else { return false }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.storyApplyOrder(StoryOrderArgs(episodeId: story.episodeId, draftId: p.draftId, confirmed: true))
            story.say("Applied the Story Board order to \u{201C}\(p.draftName ?? "the screenplay")\u{201D}", undoable: true)
            story.sheet = nil
            return true
        } catch {
            story.fail("Could not apply the order", error)
            return false
        }
    }
}

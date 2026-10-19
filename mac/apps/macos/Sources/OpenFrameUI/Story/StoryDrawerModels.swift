// The models behind the inspector (the drawer on Windows) for a scene card, a beat and a
// sequence. Simple edits save by themselves after a short pause in typing and when the field
// loses focus; the core folds a burst of typing into one undo step. The status line reads
// Saving, Saved or Not saved, as on Windows.

import Foundation
import Observation
import OpenFrameCore

enum StorySaveState: Equatable {
    case idle, saving, saved, error

    var text: String {
        switch self {
        case .idle: ""
        case .saving: "Saving\u{2026}"
        case .saved: "Saved"
        case .error: "Not saved"
        }
    }
}

/// The status line of one inspector: the state of the field saved last.
@MainActor
@Observable
final class StorySaveStatus {
    private(set) var state: StorySaveState = .idle
    func set(_ s: StorySaveState) { state = s }
}

/// A text field that saves after a pause in typing and on demand. A failed save puts the old
/// value back as "last saved" so the next flush retries it.
@MainActor
@Observable
final class FieldSaver {
    var text: String
    /// True while the field has keyboard focus: the saved value then never overwrites typing.
    var focused = false

    @ObservationIgnored private var last: String
    @ObservationIgnored private var typed: String
    @ObservationIgnored private let status: StorySaveStatus
    @ObservationIgnored private let delay: Duration
    @ObservationIgnored private let save: @MainActor (String) async throws -> Void
    @ObservationIgnored private let onError: @MainActor (Error) -> Void
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private var inflight: Task<Bool, Never>?

    init(
        initial: String = "", status: StorySaveStatus, delay: Duration = .milliseconds(700),
        onError: @escaping @MainActor (Error) -> Void = { _ in },
        save: @escaping @MainActor (String) async throws -> Void
    ) {
        text = initial
        last = initial
        typed = initial
        self.status = status
        self.delay = delay
        self.save = save
        self.onError = onError
    }

    /// The person typed.
    func edit(_ next: String) {
        text = next
        typed = next
        timer?.cancel()
        timer = Task { [weak self, delay] in
            try? await Task.sleep(for: delay)
            guard !Task.isCancelled, let self else { return }
            _ = await self.flush(next, cancelTimer: false)
        }
    }

    /// Saves what was typed last. Returns false when the save failed.
    @discardableResult
    func flush() async -> Bool { await flush(typed) }

    private func flush(_ next: String, cancelTimer: Bool = true) async -> Bool {
        if cancelTimer { timer?.cancel() }
        if let running = inflight { _ = await running.value }
        if next == last { return true }
        let before = last
        last = next
        status.set(.saving)
        let run = Task { @MainActor [weak self] () -> Bool in
            guard let self else { return false }
            do {
                try await self.save(next)
                self.status.set(.saved)
                return true
            } catch {
                self.status.set(.error)
                self.onError(error)
                if self.last == next { self.last = before }
                return false
            }
        }
        inflight = run
        let ok = await run.value
        inflight = nil
        return ok
    }

    /// A newer value arrived from the core (not our own save): show it unless the person is typing.
    func external(_ value: String) {
        guard !focused else { return }
        text = value
        last = value
        typed = value
    }
}

// MARK: Scene card

@MainActor
@Observable
final class CardDrawerModel {
    let story: StoryModel
    let cardId: String
    let status = StorySaveStatus()
    private(set) var detail: StoryCardDetail?
    private(set) var missing = false
    private(set) var fields: Fields?

    struct Fields {
        var heading: FieldSaver
        var short: FieldSaver
        var long: FieldSaver
        var notes: FieldSaver
    }

    init(story: StoryModel, cardId: String) {
        self.story = story
        self.cardId = cardId
    }

    static let longDescriptionMax = 100_000
    static let attachmentsMax = 50

    private func makeFields() -> Fields {
        let id = cardId
        let story = story
        let status = status
        func saver(_ write: @escaping @MainActor (Core, String) async throws -> Void) -> FieldSaver {
            FieldSaver(status: status, onError: { story.fail("Could not save the scene card", $0) }) { value in
                guard let core = story.app.core else { return }
                try await write(core, value)
            }
        }
        return Fields(
            heading: saver { _ = try await $0.storyUpdateCard(StoryUpdateCardArgs(id: id, sceneHeading: $1)) },
            short: saver { _ = try await $0.storyUpdateCard(StoryUpdateCardArgs(id: id, shortDescription: $1)) },
            long: saver { _ = try await $0.storyUpdateCard(StoryUpdateCardArgs(id: id, longDescription: $1)) },
            notes: saver { _ = try await $0.storyUpdateCard(StoryUpdateCardArgs(id: id, notes: $1)) })
    }

    func reload() async {
        guard let core = story.app.core else { return }
        do {
            let d = try await core.storyCard(StoryIdArgs(id: cardId))
            detail = d
            missing = false
            let f = fields ?? makeFields()
            fields = f
            f.heading.external(d.card.sceneHeading ?? "")
            f.short.external(d.card.shortDescription)
            f.long.external(d.card.longDescription ?? "")
            f.notes.external(d.card.notes ?? "")
        } catch {
            missing = true
        }
    }

    /// Writes any typed text now (the drawer is closing).
    func flushAll() async {
        guard let f = fields else { return }
        _ = await f.heading.flush()
        _ = await f.short.flush()
        _ = await f.long.flush()
        _ = await f.notes.flush()
    }

    var title: String {
        guard let c = detail?.card else { return "Scene Card" }
        let first = c.shortDescription.trimmingCharacters(in: .whitespacesAndNewlines).split(separator: "\n").first.map(String.init) ?? ""
        if !first.isEmpty { return first }
        if let h = c.sceneHeading, !h.isEmpty { return h }
        return "Untitled scene"
    }

    var parked: Bool { detail?.card.parentType == .parking }
    var ref: StoryItemRef { StoryItemRef(kind: .card, id: cardId) }
}

// MARK: Beat

@MainActor
@Observable
final class BeatDrawerModel {
    let story: StoryModel
    let beatId: String
    let status = StorySaveStatus()
    let text: FieldSaver
    let note: FieldSaver

    init(story: StoryModel, beatId: String) {
        self.story = story
        self.beatId = beatId
        let status = status
        func saver(_ write: @escaping @MainActor (Core, String) async throws -> Void) -> FieldSaver {
            FieldSaver(status: status, onError: { story.fail("Could not save the beat", $0) }) { value in
                guard let core = story.app.core else { return }
                try await write(core, value)
            }
        }
        text = saver { _ = try await $0.storyUpdateBeat(StoryUpdateBeatArgs(id: beatId, text: $1)) }
        note = saver { _ = try await $0.storyUpdateBeat(StoryUpdateBeatArgs(id: beatId, note: $1)) }
    }

    var beat: StoryBeatDto? {
        guard let board = story.board, let loc = StoryLogic.locate(board, StoryItemRef(kind: .beat, id: beatId)),
              case let .beat(b) = loc.item else { return nil }
        return b
    }

    var container: StoryContainerRef? {
        guard let board = story.board else { return nil }
        return StoryLogic.locate(board, StoryItemRef(kind: .beat, id: beatId))?.container
    }

    func sync() {
        guard let b = beat else { return }
        text.external(b.text)
        note.external(b.note ?? "")
    }

    func setColor(_ name: String) async {
        guard let core = story.app.core else { return }
        status.set(.saving)
        do {
            _ = try await core.storyUpdateBeat(StoryUpdateBeatArgs(id: beatId, color: name))
            status.set(.saved)
        } catch {
            status.set(.error)
            story.fail("Could not save the beat", error)
        }
    }

    func flushAll() async {
        _ = await text.flush()
        _ = await note.flush()
    }
}

// MARK: Sequence

@MainActor
@Observable
final class SequenceDrawerModel {
    let story: StoryModel
    let sequenceId: String
    let status = StorySaveStatus()
    let title: FieldSaver
    let note: FieldSaver

    init(story: StoryModel, sequenceId: String) {
        self.story = story
        self.sequenceId = sequenceId
        let status = status
        title = FieldSaver(status: status, onError: { story.fail("Could not save the sequence", $0) }) { value in
            guard let core = story.app.core, !value.trimmingCharacters(in: .whitespaces).isEmpty else { return }
            _ = try await core.storyUpdateSequence(StoryUpdateSequenceArgs(id: sequenceId, title: value.trimmingCharacters(in: .whitespaces)))
        }
        note = FieldSaver(status: status, onError: { story.fail("Could not save the sequence", $0) }) { value in
            guard let core = story.app.core else { return }
            _ = try await core.storyUpdateSequence(StoryUpdateSequenceArgs(id: sequenceId, note: value))
        }
    }

    var sequence: StorySequenceDto? {
        guard let board = story.board else { return nil }
        return StoryLogic.sequences(board).first { $0.id == sequenceId }
    }

    /// The act the sequence sits in, or "Unassigned".
    var actTitle: String {
        guard let board = story.board, let s = sequence else { return "" }
        if let a = s.actId { return board.acts.first { $0.id == a }?.title ?? "Act" }
        return "Unassigned"
    }

    func sync() {
        guard let s = sequence else { return }
        title.external(s.title)
        note.external(s.note ?? "")
    }

    func flushAll() async {
        _ = await title.flush()
        _ = await note.flush()
    }
}

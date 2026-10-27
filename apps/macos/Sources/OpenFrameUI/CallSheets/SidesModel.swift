// The Sides tab (Windows production/tabs/sides.tsx): "What script pages does this team need for
// this shooting day?" Sides hold only the chosen day's screenplay scenes from the Production
// Source. Saving creates a snapshot; the screenplay is never changed.

import Foundation
import Observation
import OpenFrameCore

enum CSSidesFor: String, CaseIterable, Identifiable {
    case all, selected
    var id: String { rawValue }
    var title: String { self == .all ? "All cast" : "Selected cast" }
}

@MainActor
@Observable
final class CSSidesModel {
    unowned let hub: CallSheetsHub

    private(set) var schedule: ScheduleView?
    private(set) var saved: [ScheduleSideRow] = []
    private(set) var loaded = false
    private(set) var failure: CoreFailure?

    var dayId = ""
    /// nil means every scene of the day.
    private(set) var selected: Set<String>?
    var forMode: CSSidesFor = .all
    private(set) var castKeys: Set<String> = []
    var cover = false
    var draftName = true

    private(set) var preview: ScheduleSidesContent?
    private(set) var previewFailure: CoreFailure?
    private(set) var previewLoading = false

    private(set) var openId: String?
    private(set) var openSaved: ScheduleSideDetail?
    private(set) var openFailure: CoreFailure?

    init(hub: CallSheetsHub) { self.hub = hub }

    // MARK: Derived

    var hasSchedule: Bool { schedule?.schedule != nil }
    var days: [ScheduleShootDayDto] { (schedule?.days ?? []).filter { !$0.isOffDay } }
    var day: ScheduleShootDayDto? { days.first { $0.id == dayId } }
    var strips: [ScheduleStripDto] { day?.items.compactMap(\.strip) ?? [] }
    var chosen: [String] {
        guard let selected else { return strips.map(\.id) }
        return strips.map(\.id).filter { selected.contains($0) }
    }
    var dayCast: [ScheduleCastRef] { day?.summary.cast ?? [] }

    // "Selected cast": the same rule the core applies (a scene counts when one of its confirmed
    // breakdown cast keys is picked), worked out here so the checklist, the preview and the export
    // agree (Windows 81e75d16).
    var filtering: Bool { forMode == .selected && !castKeys.isEmpty }
    private func hasCast(_ s: ScheduleStripDto) -> Bool { s.cast.contains { castKeys.contains($0.key) } }
    /// Whether the scene stays in the sides under the cast filter.
    func included(_ s: ScheduleStripDto) -> Bool { !filtering || hasCast(s) }
    /// The tick shown on a scene: off for a scene the cast filter leaves out.
    func ticked(_ s: ScheduleStripDto) -> Bool { included(s) && chosen.contains(s.id) }
    var sceneCount: Int { strips.filter(included).count }
    /// The scenes sent to the core: ticked and kept by the cast filter. The user's own ticks on
    /// left-out scenes are kept, so All cast brings them back unchanged.
    var effective: [String] { strips.filter { ticked($0) }.map(\.id) }

    /// How many of the day's scenes this cast member is in.
    func sceneCount(castKey: String) -> Int { strips.filter { $0.cast.contains { $0.key == castKey } }.count }

    func castLabel(_ c: ScheduleCastRef) -> String {
        let name = c.actor.map { "\($0) (\(c.character))" } ?? c.character
        return "\(name) \u{00B7} \(Self.plural(sceneCount(castKey: c.key), "scene"))"
    }

    static func plural(_ n: Int, _ word: String) -> String { "\(n) \(word)\(n == 1 ? "" : "s")" }

    /// The line under the scene checklist while the cast filter is on.
    var filterLine: String? {
        guard filtering else { return nil }
        if sceneCount == 0 { return "No scene on this day has the selected cast. Choose other cast, or switch to All cast." }
        return "\(sceneCount) of \(Self.plural(strips.count, "scene")) have the selected cast. Only those are in the sides."
    }

    /// What the preview area says while there is nothing to preview.
    var notReadyMessage: String {
        if filtering, sceneCount == 0 {
            return "No scene on this day has the selected cast, so there are no sides to show. Choose other cast, or switch to All cast. Cast comes from each scene\u{2019}s confirmed breakdown."
        }
        if filtering, effective.isEmpty {
            return "All the scenes with the selected cast are unticked. Tick at least one scene to preview the sides."
        }
        return "Choose a shooting day and at least one scene\(forMode == .selected ? " and cast member" : "") to preview the sides."
    }

    var ready: Bool {
        day != nil && !effective.isEmpty && (forMode == .all || !castKeys.isEmpty)
    }

    var args: ScheduleSidesArgs {
        ScheduleSidesArgs(
            dayId: dayId.isEmpty ? nil : dayId,
            stripIds: effective,
            castKeys: forMode == .selected ? Array(castKeys).sorted() : nil,
            includeCover: cover,
            showDraftName: draftName)
    }

    /// Changes whenever the preview must be fetched again.
    var previewKey: String {
        "\(dayId)|\(effective.joined(separator: ","))|\(forMode.rawValue)|\(castKeys.sorted().joined(separator: ","))|\(cover)|\(draftName)|\(openId ?? "")"
    }

    // MARK: Choices

    func pickDay(_ id: String) {
        dayId = id
        selected = nil
        castKeys = []
    }

    func focusDay(_ id: String) {
        if days.contains(where: { $0.id == id }) { pickDay(id) } else { pendingDay = id }
        openId = nil
    }
    @ObservationIgnored private var pendingDay: String?

    func setScene(_ id: String, on: Bool) {
        var set = Set(chosen)
        if on { set.insert(id) } else { set.remove(id) }
        selected = set
    }

    func setCast(_ key: String, on: Bool) {
        if on { castKeys.insert(key) } else { castKeys.remove(key) }
    }

    // MARK: Loading

    func reload() async {
        guard let core = hub.core else { return }
        do {
            let view = try await core.scheduleGet()
            failure = nil
            saved = (try? await core.sidesList()) ?? saved
            show(view)
        } catch {
            failure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    /// Shows a schedule read from the core and keeps the chosen day when it is still there.
    func show(_ view: ScheduleView) {
        schedule = view
        if let p = pendingDay, days.contains(where: { $0.id == p }) {
            pickDay(p)
            pendingDay = nil
        }
        if dayId.isEmpty || day == nil, let first = days.first { pickDay(first.id) }
        loaded = true
    }

    /// The live preview (or the open saved snapshot).
    func loadPreview() async {
        guard let core = hub.core else { return }
        if let id = openId {
            do {
                openSaved = try await core.sidesGet(ScheduleDocIdArgs(id: id))
                openFailure = nil
            } catch {
                openSaved = nil
                openFailure = CoreFailure(asCore: error)
            }
            return
        }
        guard ready else {
            preview = nil
            previewFailure = nil
            return
        }
        previewLoading = true
        defer { previewLoading = false }
        do {
            preview = try await core.sidesPreview(args)
            previewFailure = nil
        } catch {
            preview = nil
            previewFailure = CoreFailure(asCore: error)
        }
    }

    // MARK: Actions

    func openSnapshot(_ id: String) {
        openId = id
        openSaved = nil
        openFailure = nil
    }

    func backToLive() { openId = nil }

    func save() async {
        guard ready, let core = hub.core else { return }
        do {
            let id = try await core.sidesCreate(args)
            hub.say("Saved sides", undoable: true)
            saved = (try? await core.sidesList()) ?? saved
            openSnapshot(id)
        } catch {
            hub.fail("Could not save the sides", error)
        }
    }

    func deleteSaved(_ id: String) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.sidesDelete(ScheduleDocIdArgs(id: id))
            hub.say("Deleted saved sides", undoable: true)
            saved = (try? await core.sidesList()) ?? saved
            if openId == id { openId = nil }
        } catch {
            hub.fail("Could not delete the saved sides", error)
        }
    }

    func exportPDF() {
        if let id = openId, let s = openSaved {
            hub.exportSides(args: nil, sideId: id, name: s.title)
        } else {
            hub.exportSides(args: args, sideId: nil, name: "Sides - \(day?.title ?? "Selected scenes")")
        }
    }
}

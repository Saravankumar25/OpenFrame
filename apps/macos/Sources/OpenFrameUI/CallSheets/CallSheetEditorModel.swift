// The model of one open call sheet (Windows CallSheetEditor.tsx). The screen edits a local copy of
// the document; every change is saved to the call sheet half a second later (and before any
// other action), so nothing typed is lost. Derived values (scenes, times, cast, locations) come
// from the schedule through the core; editing the sheet never changes the screenplay, the
// breakdown or the schedule.

import Foundation
import Observation
import OpenFrameCore

enum CSDialog: Equatable { case refresh, finalize }

/// What happened when the person pressed Finalize in the dialog.
enum FinalizeOutcome: Equatable {
    case done
    /// The crew call is empty: the dialog closes and the field is marked.
    case missingCrewCall
    /// The schedule changed: the dialog offers "Finalize this version anyway".
    case stale
    case failed(String)
}

@MainActor
@Observable
final class CallSheetEditorModel {
    unowned let hub: CallSheetsHub
    let id: String

    private(set) var detail: CallSheetDetail?
    private(set) var failure: CoreFailure?
    private(set) var loaded = false
    /// The local copy the fields edit.
    private(set) var doc: CallSheetDocument?
    private(set) var saving = false
    var dialog: CSDialog?
    var crewInvalid = false
    /// Bumped when the crew call field should take the focus.
    private(set) var focusCrewToken = 0
    private(set) var preview: CallSheetRefreshPreview?
    private(set) var previewFailure: CoreFailure?
    private(set) var busy = false

    @ObservationIgnored private var dirty = false
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private var inflight: Task<Void, Never>?

    static let saveDelay: Duration = .milliseconds(500)

    init(hub: CallSheetsHub, id: String) {
        self.hub = hub
        self.id = id
    }

    // MARK: Derived

    var editable: Bool { (detail?.editable ?? false) && hub.canEdit }
    var word: String { CSStatus.word(detail?.status ?? "Draft") }
    var replaced: Bool { detail?.supersededBy != nil }
    var hasUnsavedEdits: Bool { dirty }

    // MARK: Loading

    func reload() async {
        guard let core = hub.core else { return }
        do {
            let d = try await core.callsheetsGet(CallSheetIdArgs(id: id))
            detail = d
            failure = nil
            if !dirty { doc = d.document }
        } catch {
            failure = CoreFailure(asCore: error)
        }
        loaded = true
    }

    // MARK: Editing and saving

    /// Changes the local copy and schedules a save. Does nothing on a final or read-only sheet.
    func edit(_ change: (inout CallSheetDocument) -> Void) {
        guard editable, var d = doc else { return }
        change(&d)
        guard d != doc else { return }
        doc = d
        dirty = true
        timer?.cancel()
        timer = Task { [weak self] in
            try? await Task.sleep(for: Self.saveDelay)
            guard !Task.isCancelled else { return }
            _ = await self?.saveNow()
        }
    }

    /// Saves now and waits for a save already running. True when nothing is left unsaved.
    @discardableResult
    func flush() async -> Bool {
        timer?.cancel()
        timer = nil
        return await saveNow()
    }

    private func saveNow() async -> Bool {
        if let running = inflight { await running.value }
        guard dirty, let sent = doc, let core = hub.core else { return !dirty }
        saving = true
        let run = Task { [weak self] in
            do {
                _ = try await core.callsheetsUpdate(CallSheetUpdateArgs(id: self?.id ?? "", document: sent))
                // Only the document that was sent is marked saved: an edit made while it was on
                // its way stays unsaved and goes in the next save.
                if self?.doc == sent { self?.dirty = false }
            } catch {
                self?.hub.fail("Could not save the call sheet", error)
            }
        }
        inflight = run
        await run.value
        inflight = nil
        saving = false
        return !dirty
    }

    // MARK: Sections

    /// "Add Section": the core fills the section from the project where it has the data.
    func addSection(key: String, label: String) async {
        guard let core = hub.core else { return }
        guard await flush() else { return }
        do {
            _ = try await core.callsheetsAddSection(CallSheetAddSectionArgs(id: id, section: key))
            hub.say("Added \(label) to the call sheet", undoable: true)
            await reload()
        } catch {
            hub.fail("Could not add the section", error)
        }
    }

    func addCustomField() {
        edit { $0.extraFields.append(CallSheetExtraField(label: "", value: "")) }
    }

    func isShown(_ key: String) -> Bool {
        guard let o = doc?.optional else { return false }
        switch key {
        case "weather": return o.weather != nil
        case "extras": return o.extras != nil
        case "departments": return o.departments != nil
        case "advance": return o.advance != nil
        case "crew": return o.crew != nil
        case "communication": return o.communication != nil
        case "referenceImages": return o.referenceImages != nil
        case "attachments": return o.attachments != nil
        case "specialNotes": return o.specialNotes != nil
        default: return false
        }
    }

    func attach(section: String, path: String) async {
        guard let core = hub.core else { return }
        guard await flush() else { return }
        do {
            _ = try await core.callsheetsAttach(CallSheetAttachArgs(id: id, section: section, path: path))
            hub.say("Attached a file to the call sheet", undoable: true)
            await reload()
        } catch {
            hub.fail("Could not attach the file", error)
        }
    }

    // MARK: Refresh, finalize, revisions

    func loadPreview() async {
        guard let core = hub.core else { return }
        preview = nil
        previewFailure = nil
        do {
            preview = try await core.callsheetsRefreshPreview(CallSheetIdArgs(id: id))
        } catch {
            previewFailure = CoreFailure(asCore: error)
        }
    }

    func showRefresh() async {
        guard await flush() else { return }
        dialog = .refresh
    }

    func showFinalize() async {
        guard await flush() else { return }
        dialog = .finalize
    }

    /// "Apply Update" or "Create New Revision". Returns the id of a new revision, if one was made.
    @discardableResult
    func applyRefresh() async -> String? {
        guard let core = hub.core, let p = preview, !busy else { return nil }
        busy = true
        defer { busy = false }
        do {
            if p.createsRevision {
                let newId = try await core.callsheetsNewRevision(CallSheetIdArgs(id: id))
                hub.say("Started a new call sheet revision. The final one is unchanged.", undoable: true)
                dialog = nil
                hub.open(newId)
                return newId
            }
            _ = try await core.callsheetsRefresh(CallSheetIdArgs(id: id))
            hub.say("Updated the call sheet from the schedule", undoable: true)
            dialog = nil
            await reload()
            return nil
        } catch {
            hub.fail("Could not update the call sheet", error)
            return nil
        }
    }

    func finalize(acknowledgeStale: Bool) async -> FinalizeOutcome {
        guard let core = hub.core else { return .failed("No project is open.") }
        busy = true
        defer { busy = false }
        guard await flush() else { return .failed("The call sheet could not be saved first.") }
        do {
            _ = try await core.callsheetsFinalize(CallSheetFinalizeArgs(id: id, acknowledgeStale: acknowledgeStale))
            hub.say("Finalized the call sheet", undoable: true)
            dialog = nil
            await reload()
            return .done
        } catch {
            let f = CoreFailure(asCore: error)
            if f.code == "validation.crew_call" {
                ToastCenter.shared.show(f.message, kind: .error)
                dialog = nil
                crewInvalid = true
                focusCrewToken += 1
                return .missingCrewCall
            }
            if f.code == "validation.call_sheet_stale" { return .stale }
            hub.fail("Could not finalize the call sheet", error)
            return .failed(f.message)
        }
    }

    func delete() async -> Bool {
        guard let core = hub.core, let title = detail?.title else { return false }
        timer?.cancel()
        dirty = false
        do {
            _ = try await core.callsheetsDelete(CallSheetIdArgs(id: id))
            hub.say("Deleted \(title)", undoable: true)
            await hub.closeEditor()
            return true
        } catch {
            hub.fail("Could not delete the call sheet", error)
            return false
        }
    }

    func openRevision(_ otherId: String) async {
        guard await flush() else { return }
        hub.open(otherId)
    }
}

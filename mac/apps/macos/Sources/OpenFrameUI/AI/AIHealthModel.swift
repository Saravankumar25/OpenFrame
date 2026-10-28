// Settings, Offline AI: the status, the technical details, the AI health card and Export AI report,
// and the learned-memory screen. Figures come from the local, numbers-only AI log. Nothing is sent.

import Foundation
import Observation
import OpenFrameCore

@MainActor
@Observable
final class AISettingsModel {
    let app: AppModel
    var status: AiStatusDto?
    var diagnostics: AiDiagnosticsDto?
    var health: AiHealthDto?
    /// The last failure: of reading the state, or of an action (Remove, Stop, graphics, export).
    var error: String?
    /// The error came from reading the state, so the next good read clears it. An action's failure
    /// stays until the next action: the page reads the state every two seconds and must not wipe it.
    private var errorFromLoad = false
    var busy = false
    var removing = false
    /// Asks the person where to save the report. Tests set a fixed answer.
    var chooseSaveLocation: @MainActor (_ title: String, _ name: String) -> URL? = { AIFilePicker.chooseSaveLocation(title: $0, name: $1) }
    /// The last export, for the line under the button.
    var lastExport: String?

    init(app: AppModel) {
        self.app = app
    }

    private var core: Core? { app.core }
    var mode: AILogic.PanelMode { AILogic.panelMode(status) }

    func load() async {
        guard let core else { return }
        do {
            status = try await core.aiStatus()
            diagnostics = try await core.aiDiagnostics()
            health = try await core.aiHealth()
            if errorFromLoad { self.error = nil; errorFromLoad = false }
        } catch {
            if self.error == nil || errorFromLoad {
                self.error = CoreFailure(asCore: error).message
                errorFromLoad = true
            }
        }
    }

    /// An action failed: its message stays on the page.
    private func failed(_ error: Error) {
        self.error = CoreFailure(asCore: error).message
        errorFromLoad = false
    }

    /// An action starts: the message of the one before goes.
    private func clearError() {
        error = nil
        errorFromLoad = false
    }

    func stopRuntime() async {
        guard let core else { return }
        clearError()
        do {
            _ = try await core.aiStopRuntime()
            ToastCenter.shared.show("Offline AI stopped. It starts again when you ask something.")
        } catch {
            failed(error)
        }
        await load()
    }

    /// Graphics card: Automatic or Off (ADR-0029 decision 6).
    func setGraphics(off: Bool) async {
        guard let core else { return }
        clearError()
        do {
            _ = try await core.aiGraphicsSet(AiGraphicsArgs(off: off))
            ToastCenter.shared.show(off ? "OpenFrame AI will run on the processor." : "OpenFrame AI will use a checked graphics card when there is one.")
        } catch {
            failed(error)
        }
        await load()
    }

    /// Remove OpenFrame AI. The screens show what is really on the disk afterwards.
    func remove() async {
        guard let core, !busy else { return }
        busy = true
        defer { busy = false; removing = false }
        clearError()
        do {
            _ = try await core.aiDownloadRemove()
            ToastCenter.shared.show("OpenFrame AI was removed. You can download it again here.")
        } catch {
            failed(error)
        }
        await load()
    }

    /// Export AI report: the numbers-only log, written to a file the person chooses.
    @discardableResult
    func exportReport() async -> AiReportResult? {
        guard let core, !busy else { return nil }
        guard let url = chooseSaveLocation("Export AI report", "openframe-ai-report.json") else { return nil }
        busy = true
        defer { busy = false }
        do {
            let r = try await core.aiExportReport(AiReportArgs(path: url.path))
            lastExport = "Saved \(r.fileName) (\(Int(r.rows)) requests)."
            ToastCenter.shared.show(lastExport!)
            clearError()
            return r
        } catch {
            failed(error)
            ToastCenter.shared.show(self.error ?? "", kind: .error)
            return nil
        }
    }
}

enum AIHealthText {
    static func seconds(_ ms: Double?) -> String { ms.map { String(format: "%.1f s", $0 / 1000) } ?? "none yet" }

    static func applied(_ h: AiHealthDto) -> String {
        if let rate = h.applyRatePct {
            return "\(Int(rate.rounded()))% (\(Int(h.applied)) of \(Int(h.applied + h.rejected + h.undone)) decided)"
        }
        return "\(Int(h.applied)) of \(Int(h.proposals))"
    }

    static func weakest(_ h: AiHealthDto) -> String {
        guard let w = h.weakestTool else { return "none stands out" }
        return "\(w.name): \(Int(w.failurePct.rounded()))% of \(Int(w.uses)) uses went wrong"
    }

    /// The rows of the health table, label and value, in Windows order.
    static func rows(_ h: AiHealthDto) -> [(label: String, value: String)] {
        [
            ("Requests counted", "\(Int(h.requests))"),
            ("Typical answer time", seconds(h.medianTotalMs)),
            ("Proposals applied", applied(h)),
            ("Checks per answer", h.averagePasses.map { String(format: "%.1f", $0) } ?? "none yet"),
            ("Errors", "\(Int(h.errors))"),
            ("Runs on", h.runsOn ?? "not known yet"),
            ("Weakest tool", weakest(h)),
        ]
    }
}

import AppKit
import Foundation
import ObjectiveC

// MARK: system file panels
//
// On this build machine the system Open and Save panel service does not start for any app (a
// 30-line AppKit program shows the same "service failed" alert), so a real panel can never be
// answered. The driver answers in its place: while a scenario waits for a panel, the panel's
// modal run returns at once with the file the scenario chose, and the panel object the app built
// (title, message, button, allowed file types, suggested name) is recorded for the scenario to
// check. Everything else, the button press and what the app does with the file, is the real app.

/// What the app asked the panel to show.
struct PanelInfo {
    var kind = ""
    var title = ""
    var message = ""
    var prompt = ""
    var suggestedName = ""
    var types: [String] = []
}

nonisolated(unsafe) private var answerKey: UInt8 = 0

/// The file this panel was answered with, if the driver answered it.
private func answered(_ panel: AnyObject) -> NSURL? {
    objc_getAssociatedObject(panel, &answerKey) as? NSURL
}

@MainActor private final class PanelAnswers {
    static var installed = false
    static var armed = false
    static var answer: URL?
    static var info: PanelInfo?
    static var unexpected: [String] = []

    static func install() {
        guard !installed else { return }
        installed = true
        let block: @convention(block) (NSSavePanel) -> Int = { panel in
            MainActor.assumeIsolated { PanelAnswers.handle(panel) }
        }
        for cls in [NSSavePanel.self, NSOpenPanel.self] {
            class_replaceMethod(cls, NSSelectorFromString("runModal"), imp_implementationWithBlock(block), "q@:")
            // The panel reads its answer from the url getters; answered panels return the chosen file.
            for name in ["URL", "URLs"] {
                let sel = NSSelectorFromString(name)
                guard let m = class_getInstanceMethod(cls, sel) else { continue }
                let original = method_getImplementation(m)
                if name == "URL" {
                    typealias Fn = @convention(c) (AnyObject, Selector) -> NSURL?
                    let orig = unsafeBitCast(original, to: Fn.self)
                    let b: @convention(block) (AnyObject) -> NSURL? = { panel in answered(panel) ?? orig(panel, sel) }
                    class_replaceMethod(cls, sel, imp_implementationWithBlock(b), method_getTypeEncoding(m))
                } else {
                    typealias Fn = @convention(c) (AnyObject, Selector) -> NSArray
                    let orig = unsafeBitCast(original, to: Fn.self)
                    let b: @convention(block) (AnyObject) -> NSArray = { panel in answered(panel).map { [$0] as NSArray } ?? orig(panel, sel) }
                    class_replaceMethod(cls, sel, imp_implementationWithBlock(b), method_getTypeEncoding(m))
                }
            }
        }
    }

    static func handle(_ panel: NSSavePanel) -> Int {
        let open = panel is NSOpenPanel
        var i = PanelInfo()
        i.kind = open ? "open" : "save"
        i.title = panel.title
        i.message = panel.message ?? ""
        i.prompt = panel.prompt ?? ""
        i.suggestedName = panel.nameFieldStringValue
        i.types = panel.allowedContentTypes.map(\.identifier)
        guard armed else {
            unexpected.append("\(i.kind) panel \"\(i.title)\" opened with no scenario step waiting for it")
            return NSApplication.ModalResponse.cancel.rawValue
        }
        armed = false
        info = i
        guard let url = answer else { return NSApplication.ModalResponse.cancel.rawValue }
        objc_setAssociatedObject(panel, &answerKey, url as NSURL, .OBJC_ASSOCIATION_RETAIN)
        return NSApplication.ModalResponse.OK.rawValue
    }
}

extension UI {
    /// Runs `action`, which opens a system Open or Save panel, and answers that panel with `url`
    /// (nil presses Cancel). Returns what the panel asked for, so the scenario can check its title.
    @discardableResult
    func choosing(_ url: URL?, _ action: () async throws -> Void) async throws -> PanelInfo {
        PanelAnswers.install()
        PanelAnswers.answer = url
        PanelAnswers.info = nil
        PanelAnswers.armed = true
        defer { PanelAnswers.armed = false }
        log("expecting a file panel, answer \(url?.lastPathComponent ?? "cancel")")
        try await action()
        // Some commands open the panel from a task that runs after the command returns.
        let deadline = Date().addingTimeInterval(10)
        while PanelAnswers.info == nil, Date() < deadline { await idle(0.1) }
        guard let info = PanelAnswers.info else { throw DriverError("the action opened no file panel") }
        log("file panel \(info.kind) \"\(info.title)\" answered")
        return info
    }

    /// Panels that opened with nobody waiting for them.
    func unexpectedPanels() -> [String] { PanelAnswers.unexpected }
}


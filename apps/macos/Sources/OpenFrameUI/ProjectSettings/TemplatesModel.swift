// Project Settings, Templates tab (Windows: workspaces/settings/Templates.tsx; FSD 59 and 113).
// Built-in (read-only), Yours (kept on this computer for every project) and This project.
// "Use in This Project" copies a template into the project and "Create from template" makes the
// document it describes. Both are independent copies: later template edits never change what was
// made from it. The blank path is always available.

import Observation
import OpenFrameCore
import SwiftUI

struct TemplateTypeInfo: Hashable {
    let value: String
    let label: String
    let listLabel: String
    let key: String
}

enum TemplateKinds {
    static let types: [TemplateTypeInfo] = [
        TemplateTypeInfo(value: "screenplay", label: "Screenplay starter", listLabel: "Scene headings", key: "scenes"),
        TemplateTypeInfo(value: "call_sheet", label: "Call sheet", listLabel: "Added sections (Weather, Crew, Extras\u{2026})", key: "sections"),
        TemplateTypeInfo(value: "shot_list", label: "Shot list", listLabel: "Shots", key: "shots"),
        TemplateTypeInfo(value: "moodboard", label: "Moodboard", listLabel: "Sections", key: "sections"),
        TemplateTypeInfo(value: "report", label: "Report", listLabel: "Sections", key: "sections"),
        TemplateTypeInfo(value: "story_board", label: "Story Board starter", listLabel: "Acts", key: "acts"),
        TemplateTypeInfo(value: "breakdown", label: "Breakdown document", listLabel: "Categories", key: "categories"),
        TemplateTypeInfo(value: "budget", label: "Simple budget", listLabel: "Lines (Category: item)", key: "lines"),
        TemplateTypeInfo(value: "project_starter", label: "Project starter", listLabel: "Acts", key: "acts"),
    ]

    /// Template types that can create a document in the project (templates.apply).
    static let creates: Set<String> = ["screenplay", "call_sheet", "shot_list", "moodboard", "story_board", "budget"]

    static func info(_ type: String) -> TemplateTypeInfo { types.first { $0.value == type } ?? types[0] }

    /// Lines of text to template content for a type ({ description, <key>: [...] }), keeping other keys.
    static func content(type: String, description: String, lines: String, base: [String: JSONValue] = [:]) -> [String: JSONValue] {
        let items = lines.split(separator: "\n", omittingEmptySubsequences: true)
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
        var next = base
        next[info(type).key] = .array(items.map { .string($0) })
        let d = description.trimmingCharacters(in: .whitespacesAndNewlines)
        if d.isEmpty { next["description"] = nil } else { next["description"] = .string(d) }
        return next
    }

    /// One list entry as text: plain lines stay as they are; built-in entries are objects.
    static func itemText(_ item: JSONValue) -> String {
        if let s = item.stringValue { return s }
        if case let .object(o) = item {
            let main = [o["heading"], o["description"]].compactMap { $0?.stringValue }.first
            if let category = o["category"]?.stringValue, let main { return "\(category): \(main)" }
            if let main {
                if let size = o["size"]?.stringValue { return "\(main) (\(size))" }
                return main
            }
        }
        return String(describing: item)
    }

    static func list(_ content: [String: JSONValue], key: String) -> [String] {
        (content[key]?.arrayValue ?? []).map(itemText)
    }

    static func createLabel(_ type: String) -> String {
        switch type {
        case "screenplay": "Start the screenplay"
        case "call_sheet": "Create a call sheet"
        case "shot_list": "Add shots to a scene"
        case "moodboard": "Create a moodboard"
        case "story_board": "Add to the Story Board"
        case "budget": "Add budget lines"
        default: "Create from template"
        }
    }

    static func scopeLabel(_ scope: TemplateScope) -> String {
        switch scope {
        case .builtin: "Built-in"
        case .global: "Yours \u{00B7} every project"
        case .project: "This project"
        }
    }
}

@MainActor
@Observable
final class TemplatesModel {
    private(set) var listing: TemplatesListing?
    private(set) var loading = true
    private(set) var error: String?
    private weak var app: AppModel?

    init(app: AppModel?) { self.app = app }

    func load() async {
        guard let core = app?.core else { return }
        do {
            listing = try await core.templatesList(TemplateListArgs())
            error = nil
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
        loading = false
    }

    // MARK: Changes (each returns true when the core did it)

    @discardableResult
    func copyIntoProject(_ t: TemplateDto) async -> Bool {
        await run("Could not add the template") { core, app in
            let copy = try await core.templatesCopy(CopyTemplateArgs(scope: t.scope, id: t.id, toScope: .project))
            ToastCenter.shared.undoable("Added template \u{201C}\(copy.name)\u{201D} to this project", app: app)
        }
    }

    @discardableResult
    func keepForAll(_ t: TemplateDto) async -> Bool {
        await run("Could not save the template") { core, _ in
            let copy = try await core.templatesCopy(CopyTemplateArgs(scope: t.scope, id: t.id, toScope: .global))
            ToastCenter.shared.show("\u{201C}\(copy.name)\u{201D} is now in your templates for every project.")
        }
    }

    @discardableResult
    func remove(_ t: TemplateDto) async -> Bool {
        await run("Could not delete the template") { core, app in
            _ = try await core.templatesDelete(TemplateRefArgs(scope: t.scope, id: t.id))
            if t.scope == .project {
                ToastCenter.shared.undoable("Moved template \u{201C}\(t.name)\u{201D} to Recently Deleted", app: app)
            } else {
                ToastCenter.shared.show("Deleted template \u{201C}\(t.name)\u{201D}. Projects that already used it are not affected.")
            }
        }
    }

    /// Create (template nil) or update. `scope` is only used for a new template.
    @discardableResult
    func save(
        existing: TemplateDto?, name: String, type: String, scope: TemplateScope, description: String, lines: String
    ) async -> Bool {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return false }
        let content = TemplateKinds.content(
            type: existing?.templateType ?? type, description: description, lines: lines, base: existing?.content ?? [:])
        return await run("Could not save the template") { core, _ in
            if let existing {
                _ = try await core.templatesUpdate(UpdateTemplateArgs(scope: existing.scope, id: existing.id, name: trimmed, content: content))
                ToastCenter.shared.show("Saved template \u{201C}\(trimmed)\u{201D}.")
            } else {
                _ = try await core.templatesCreate(CreateTemplateArgs(scope: scope, templateType: type, name: trimmed, content: content))
                ToastCenter.shared.show(scope == .global
                    ? "Created \u{201C}\(trimmed)\u{201D} in your templates."
                    : "Created \u{201C}\(trimmed)\u{201D} in this project.")
            }
        }
    }

    /// Create the document a template describes. Returns the summary, or nil on failure.
    func apply(_ t: TemplateDto, dayId: String?, sceneId: String?) async -> AppliedTemplate? {
        guard let app, let core = app.core else { return nil }
        do {
            let done = try await core.templatesApply(ApplyTemplateArgs(scope: t.scope, id: t.id, dayId: dayId, sceneId: sceneId))
            ToastCenter.shared.undoable(done.summary, app: app)
            await load()
            return done
        } catch {
            app.show("Could not create from the template", error)
            return nil
        }
    }

    /// Shooting days without a call sheet (for "Create a call sheet") and scenes (for "Add shots").
    func shootingDaysWithoutCallSheet() async -> [ScheduleShootDayDto] {
        guard let core = app?.core, let view = try? await core.scheduleGet() else { return [] }
        return view.days.filter { !$0.isOffDay && $0.callSheet == nil }
    }

    func scenes() async -> [VisualScene] {
        guard let core = app?.core, let s = try? await core.visualScenes() else { return [] }
        return s.scenes
    }

    static func sceneLabel(_ s: VisualScene) -> String {
        let head = s.heading.trimmingCharacters(in: .whitespaces).isEmpty ? "Untitled scene" : s.heading.trimmingCharacters(in: .whitespaces)
        return "\(s.groupLabel.map { "\($0) \u{00B7} " } ?? "")Scene \(s.number): \(head)"
    }

    /// Go where the new document is (the workspace named in the core's answer).
    static func go(_ done: AppliedTemplate) {
        let hit = SearchHit(
            store: .project, entityType: "", entityId: "", title: "", snippet: "", context: "",
            nav: .object(done.nav), rank: 0)
        NotificationCenter.default.post(name: .openFrameCloseProjectSettings, object: nil)
        SearchRouting.go(SearchRouting.target(for: hit))
    }

    private func run(_ title: String, _ work: (Core, AppModel) async throws -> Void) async -> Bool {
        guard let app, let core = app.core else { return false }
        do {
            try await work(core, app)
            await load()
            return true
        } catch {
            app.show(title, error)
            return false
        }
    }
}

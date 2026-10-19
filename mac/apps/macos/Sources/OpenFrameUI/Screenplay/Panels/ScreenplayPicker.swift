// The screenplay picker (Mac phase M4), as picker.tsx on Windows: a project can hold several
// screenplays, each with its own drafts, scene numbers and page setup. The menu switches between
// them and offers New, Rename and Delete. Delete moves the screenplay with all its drafts to
// Recently Deleted for 30 days.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class ScreenplayPickerModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?
    @ObservationIgnored var say: (String) -> Void = { _ in }

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel, say: @escaping (String) -> Void) {
        self.app = app
        self.workspace = workspace
        self.say = say
    }

    private var core: Core? { app?.core }
    var screenplays: [ScreenplaySummary] { workspace?.overview?.screenplays ?? [] }
    var open: ScreenplayDto? { workspace?.overview?.screenplay }
    var permissions: ScreenplayPermissions? { workspace?.overview?.permissions }

    static let maxTitle = 200

    /// Another screenplay of the list already uses this title (case and outer spaces ignored).
    static func titleTaken(_ screenplays: [ScreenplaySummary], _ title: String, except id: String? = nil) -> Bool {
        let t = title.trimmingCharacters(in: .whitespaces).lowercased()
        return screenplays.contains { $0.id != id && $0.title.trimmingCharacters(in: .whitespaces).lowercased() == t }
    }

    static let takenMessage = "Another screenplay here already has this title. Choose a different title."

    func draftCountLabel(_ s: ScreenplaySummary) -> String { PanelText.plural(Int(s.draftCount), "draft") }

    func menuLabel(_ s: ScreenplaySummary) -> String {
        "\(s.title) \u{00B7} \(draftCountLabel(s))\(s.id == open?.id ? " \u{00B7} Open" : "")"
    }

    func switchTo(_ id: String) async {
        await workspace?.show(screenplayId: id, draftId: nil)
    }

    func create(title: String) async -> String? {
        guard let core else { return "OpenFrame is not ready." }
        let t = title.trimmingCharacters(in: .whitespaces)
        if Self.titleTaken(screenplays, t) { return Self.takenMessage }
        await workspace?.flush()
        do {
            let r = try await core.screenplayCreate(CreateScreenplayArgs(title: t, episodeId: workspace?.overview?.episodeId))
            say("Created screenplay \(PanelText.quoted(r.screenplay.title))")
            await workspace?.show(screenplayId: r.screenplay.id, draftId: r.draft.id)
            return nil
        } catch {
            return PanelText.message(error, fallback: "The screenplay could not be created.")
        }
    }

    func rename(title: String) async -> String? {
        guard let core, let open else { return nil }
        let t = title.trimmingCharacters(in: .whitespaces)
        if Self.titleTaken(screenplays, t, except: open.id) { return Self.takenMessage }
        do {
            let r = try await core.screenplayRename(ScreenplayRenameArgs(screenplayId: open.id, title: t))
            say("Renamed the screenplay to \(PanelText.quoted(r.title))")
            await workspace?.loadOverview()
            return nil
        } catch {
            return PanelText.message(error, fallback: "The screenplay could not be renamed.")
        }
    }

    func delete() async -> String? {
        guard let core, let open else { return nil }
        let next = screenplays.first { $0.id != open.id }?.id
        await workspace?.flush()
        do {
            _ = try await core.screenplayDelete(ScreenplayDeleteArgs(screenplayId: open.id))
            say("Moved \(PanelText.quoted(open.title)) to Recently Deleted")
            await workspace?.show(screenplayId: next, draftId: nil)
            return nil
        } catch {
            return PanelText.message(error, fallback: "The screenplay could not be deleted.")
        }
    }
}

/// The toolbar menu. Its label reads "Screenplay: <title>. Switch screenplay".
struct ScreenplayPickerMenu: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @State private var model = ScreenplayPickerModel()

    var body: some View {
        // A task on a toolbar item does not run, so the model is connected while the view is built.
        // It only stores references, and a toolbar item shows the title as soon as the overview is in.
        let _ = model.attach(app, workspace, say: { state.say($0) })
        let title = model.open?.title ?? "Screenplay"
        Menu {
            Section(model.screenplays.count > 1 ? "Screenplays in this project (\(model.screenplays.count))" : "Screenplay") {
                ForEach(model.screenplays, id: \.id) { s in
                    Button(model.menuLabel(s)) { Task { await model.switchTo(s.id) } }
                        .disabled(s.id == model.open?.id)
                }
            }
            if model.permissions?.edit == true {
                Divider()
                Button("New Screenplay\u{2026}") { state.sheet = .newScreenplay }
                Button("Rename Screenplay\u{2026}") { state.sheet = .renameScreenplay }
            }
            if model.permissions?.delete == true {
                Divider()
                Button("Delete Screenplay\u{2026}", role: .destructive) { state.sheet = .deleteScreenplay }
            }
        } label: {
            Label(title, systemImage: "film")
        }
        .help("Switch screenplay")
        .accessibilityLabel("Screenplay: \(title). Switch screenplay")
        .accessibilityIdentifier("screenplay-picker")
        .task { model.attach(app, workspace, say: { state.say($0) }) }
    }
}

struct NewScreenplaySheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @State private var model = ScreenplayPickerModel()
    @State private var title = ""
    @State private var error: String?

    private var taken: Bool { !title.trimmingCharacters(in: .whitespaces).isEmpty && ScreenplayPickerModel.titleTaken(model.screenplays, title) }

    var body: some View {
        PanelSheetFrame(title: "New Screenplay",
                        sub: "A separate screenplay with its own drafts, locked drafts, scene numbers and page setup. Your other screenplays are not changed.") {
            VStack(alignment: .leading, spacing: 8) {
                PanelField(label: "Title", hint: "For example the short version, or a second episode you shoot together.") {
                    TextField("Title", text: $title)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Title")
                        .accessibilityIdentifier("screenplay-title")
                        .onSubmit(create)
                }
                if let message = error ?? (taken ? ScreenplayPickerModel.takenMessage : nil) {
                    PanelBanner(tone: .err, text: message)
                }
            }
        } footer: {
            Button("Cancel", role: .cancel) { state.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Create Screenplay", action: create)
                .keyboardShortcut(.defaultAction)
                .disabled(title.trimmingCharacters(in: .whitespaces).isEmpty || taken)
        }
        .task { model.attach(app, workspace, say: { state.say($0) }) }
        .onChange(of: title) { error = nil }
    }

    private func create() {
        guard !title.trimmingCharacters(in: .whitespaces).isEmpty, !taken else { return }
        Task {
            error = await model.create(title: title)
            if error == nil { state.sheet = nil }
        }
    }
}

struct RenameScreenplaySheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @State private var model = ScreenplayPickerModel()
    @State private var title = ""
    @State private var error: String?

    private var taken: Bool { !title.trimmingCharacters(in: .whitespaces).isEmpty && ScreenplayPickerModel.titleTaken(model.screenplays, title, except: model.open?.id) }
    private var ready: Bool {
        let t = title.trimmingCharacters(in: .whitespaces)
        return !t.isEmpty && !taken && t != (model.open?.title ?? "").trimmingCharacters(in: .whitespaces)
    }

    var body: some View {
        PanelSheetFrame(title: "Rename Screenplay", sub: "Only the name changes. Drafts and their text stay as they are.") {
            VStack(alignment: .leading, spacing: 8) {
                PanelField(label: "Title") {
                    TextField("Title", text: $title)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Title")
                        .accessibilityIdentifier("screenplay-title")
                        .onSubmit(save)
                }
                if let message = error ?? (taken ? ScreenplayPickerModel.takenMessage : nil) {
                    PanelBanner(tone: .err, text: message)
                }
            }
        } footer: {
            Button("Cancel", role: .cancel) { state.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Rename", action: save).keyboardShortcut(.defaultAction).disabled(!ready)
        }
        .task {
            model.attach(app, workspace, say: { state.say($0) })
            title = model.open?.title ?? ""
        }
        .onChange(of: title) { error = nil }
    }

    private func save() {
        guard ready else { return }
        Task {
            error = await model.rename(title: title)
            if error == nil { state.sheet = nil }
        }
    }
}

struct DeleteScreenplaySheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = ScreenplayPickerModel()
    @State private var error: String?

    var body: some View {
        let title = model.open?.title ?? workspace.overview?.screenplay?.title ?? ""
        let others = model.screenplays.count - 1
        PanelSheetFrame(title: "Delete \(PanelText.quoted(title))?") {
            VStack(alignment: .leading, spacing: 8) {
                Text("\(PanelText.quoted(title)) and all its drafts move to Recently Deleted. They stay there for 30 days, and you can restore the screenplay with its drafts, locked drafts, scene numbers and page setup as they are now.")
                    .fixedSize(horizontal: false, vertical: true)
                Text(others > 0 ? "Your other screenplays are not changed."
                                : "This is the only screenplay in this project. Afterwards you can start a new one or restore this one from Recently Deleted.")
                    .font(.callout).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                if let error { PanelBanner(tone: .warn, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { state.sheet = nil }.keyboardShortcut(.cancelAction)
            Button("Delete Screenplay", role: .destructive) {
                Task {
                    error = await model.delete()
                    if error == nil { state.sheet = nil }
                }
            }
        }
        .task { model.attach(app, workspace, say: { state.say($0) }) }
    }
}

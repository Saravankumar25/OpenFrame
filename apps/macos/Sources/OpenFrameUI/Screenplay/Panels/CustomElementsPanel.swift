// Custom elements (Mac phase M4): extra paragraph kinds for one screenplay, such as a telephone
// voice or a lower third. Behaviour follows CustomElementsDialog in pagesetup.tsx on Windows:
// name, look, shortcut, blank lines before, what Enter creates, capitals, italic and the name
// used in Final Draft files. The core owns every rule; this file asks for an operation and shows
// its answer. The shortcut is stored the way Windows stores it ("Ctrl+Alt+3") so both apps read
// the same record; the Mac shows it as Command+Option+3, and the editor accepts that chord.

import SwiftUI
import OpenFrameCore

/// The form of one custom element, before it is saved.
struct CustomElementForm: Equatable {
    var id: String?
    var name = ""
    /// "Ctrl+Alt+1" to "Ctrl+Alt+9", or empty for none.
    var shortcut = ""
    var look = "action"
    var uppercase = false
    var italic = false
    var spaceBefore = 1
    var nextElement = "action"
    var exportName = ""

    init() {}

    init(_ e: ScreenplayCustomElementDto) {
        id = e.id
        name = e.name
        shortcut = e.shortcut ?? ""
        look = e.look
        uppercase = e.uppercase
        italic = e.italic
        spaceBefore = Int(e.spaceBefore)
        nextElement = e.nextElement
        exportName = e.exportName
    }
}

enum CustomElementOptions {
    static let looks: [(value: String, label: String)] = [
        ("action", "Like Action"), ("centered", "Centred"), ("character", "Like Character"),
        ("parenthetical", "Like Parenthetical"), ("dialogue", "Like Dialogue"),
        ("transition", "Like Transition (right)"), ("shot", "Like Shot"),
    ]

    static let nextBuiltIns: [(value: String, label: String)] = [
        ("action", "Action"), ("scene_heading", "Scene Heading"), ("character", "Character"),
        ("parenthetical", "Parenthetical"), ("dialogue", "Dialogue"), ("transition", "Transition"),
        ("shot", "Shot"), ("note", "Note"),
    ]

    /// The stored shortcuts: none, then 1 to 9.
    static let shortcuts: [String] = [""] + (1...9).map { "Ctrl+Alt+\($0)" }

    /// What the Mac shows for a stored shortcut.
    static func shortcutLabel(_ stored: String?) -> String {
        guard let stored, !stored.isEmpty else { return "None" }
        return stored.replacingOccurrences(of: "Ctrl+Alt+", with: "Command+Option+")
    }

    static func lookLabel(_ value: String) -> String { looks.first { $0.value == value }?.label ?? value }

    /// The choices for "Enter creates": the built-in elements, then every custom element.
    static func nextOptions(_ elements: [ScreenplayCustomElementDto]) -> [(value: String, label: String)] {
        nextBuiltIns + elements.map { ("custom:\($0.id)", $0.name) }
    }

    /// The line under a custom element's name in the list.
    static func summary(_ e: ScreenplayCustomElementDto) -> String {
        var parts = [lookLabel(e.look)]
        if e.uppercase { parts.append("capitals") }
        if e.italic { parts.append("italic") }
        if let s = e.shortcut, !s.isEmpty { parts.append(shortcutLabel(s)) }
        parts.append("Final Draft name \(e.exportName)")
        let n = Int(e.usageCount)
        parts.append("used \(n) time\(n == 1 ? "" : "s")")
        return parts.joined(separator: " \u{00B7} ")
    }
}

@MainActor
@Observable
final class CustomElementsModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?
    /// Says what happened. The default shows a toast with Undo (project undo takes it back).
    @ObservationIgnored var notify: (String) -> Void = { _ in }

    var form: CustomElementForm?
    var error: String?
    var busy = false

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel) {
        self.app = app
        self.workspace = workspace
        notify = { [weak app] text in
            if let app { ToastCenter.shared.undoable(text, app: app) }
        }
    }

    private var core: Core? { app?.core }
    var elements: [ScreenplayCustomElementDto] { workspace?.document?.customElements ?? [] }
    var canEdit: Bool { workspace?.overview?.permissions.edit == true }
    var nextOptions: [(value: String, label: String)] { CustomElementOptions.nextOptions(elements) }
    var canSave: Bool { !(form?.name.trimmingCharacters(in: .whitespaces).isEmpty ?? true) && !busy }

    func startAdd() { error = nil; form = CustomElementForm() }
    func startEdit(_ e: ScreenplayCustomElementDto) { error = nil; form = CustomElementForm(e) }
    func cancelForm() { error = nil; form = nil }

    /// Save the form: a new element, or the changes of the one being edited. True when saved.
    @discardableResult
    func save() async -> Bool {
        guard let core, let form, let screenplayId = workspace?.overview?.screenplay?.id else { return false }
        busy = true
        defer { busy = false }
        await workspace?.flush()
        var args = ScreenplayCustomElementArgs(
            name: form.name, shortcut: form.shortcut, look: form.look, uppercase: form.uppercase,
            italic: form.italic, spaceBefore: Double(form.spaceBefore), nextElement: form.nextElement,
            exportName: form.exportName)
        do {
            if let id = form.id {
                args.id = id
                _ = try await core.screenplayUpdateCustomElement(args)
                notify("Changed \(PanelText.quoted(form.name))")
            } else {
                args.screenplayId = screenplayId
                _ = try await core.screenplayCreateCustomElement(args)
                notify("Added the custom element \(PanelText.quoted(form.name))")
            }
            self.form = nil
            error = nil
            await reload()
            return true
        } catch {
            self.error = PanelText.message(error, fallback: "The custom element could not be saved.")
            return false
        }
    }

    /// Delete an element nothing uses. The core refuses one that is in use.
    func delete(_ e: ScreenplayCustomElementDto) async {
        guard let core, e.usageCount == 0 else { return }
        do {
            _ = try await core.screenplayDeleteCustomElement(ScreenplayIdOnlyArgs(id: e.id))
            notify("Moved \(PanelText.quoted(e.name)) to Recently Deleted")
            await reload()
        } catch {
            app?.show("Could not delete the custom element", error)
        }
    }

    private func reload() async {
        if let id = workspace?.draft?.id { await workspace?.loadDocument(id) }
    }
}

struct CustomElementsSheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = CustomElementsModel()

    var body: some View {
        PanelSheetFrame(
            title: "Custom elements",
            sub: "Extra paragraph kinds for this screenplay, such as a telephone voice or a lower third. Final Draft custom elements arrive here when you import.",
            width: 640
        ) {
            VStack(alignment: .leading, spacing: 10) {
                if let error = model.error, model.form == nil { PanelBanner(tone: .err, text: error) }
                if model.elements.isEmpty && model.form == nil {
                    Text("No custom elements yet.").foregroundStyle(theme["muted"])
                }
                if !model.elements.isEmpty {
                    VStack(spacing: 0) {
                        ForEach(model.elements, id: \.id) { e in
                            row(e)
                            if e.id != model.elements.last?.id { Divider() }
                        }
                    }
                    .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
                    .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("custom-elements-list")
                }
                if model.canEdit, model.form == nil {
                    Button("Add custom element") { model.startAdd() }
                        .accessibilityIdentifier("custom-element-add")
                }
                if model.form != nil { formView }
            }
        } footer: {
            Button("Done") { state.sheet = nil }
                .keyboardShortcut(model.form == nil ? .defaultAction : .cancelAction)
                .accessibilityIdentifier("custom-elements-done")
        }
        .task { model.attach(app, workspace) }
        .accessibilityIdentifier("custom-elements-sheet")
    }

    private func row(_ e: ScreenplayCustomElementDto) -> some View {
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(e.name).fontWeight(.semibold)
                Text(CustomElementOptions.summary(e)).font(.caption).foregroundStyle(theme["muted"])
            }
            Spacer(minLength: 8)
            if model.canEdit {
                Button("Edit") { model.startEdit(e) }
                    .accessibilityLabel("Edit \(e.name)")
                    .accessibilityIdentifier("custom-element-edit-\(e.id)")
                Button("Delete") { Task { await model.delete(e) } }
                    .disabled(e.usageCount > 0)
                    .help(e.usageCount > 0 ? "In use: change those paragraphs to another element first." : "Delete this element")
                    .accessibilityLabel("Delete \(e.name)")
                    .accessibilityIdentifier("custom-element-delete-\(e.id)")
            }
        }
        .padding(.horizontal, 10).padding(.vertical, 7)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("custom-element-row-\(e.id)")
    }

    @ViewBuilder private var formView: some View {
        if let form = model.form {
            let bind = { (path: WritableKeyPath<CustomElementForm, String>) in
                Binding<String>(get: { model.form?[keyPath: path] ?? "" }, set: { model.form?[keyPath: path] = $0 })
            }
            VStack(alignment: .leading, spacing: 10) {
                PanelField(label: "Name") {
                    TextField("Name", text: bind(\.name)).textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Name").accessibilityIdentifier("custom-element-name")
                }
                HStack(alignment: .top, spacing: 12) {
                    PanelField(label: "Looks") {
                        Picker("Looks", selection: bind(\.look)) {
                            ForEach(CustomElementOptions.looks, id: \.value) { Text($0.label).tag($0.value) }
                        }
                        .labelsHidden().accessibilityLabel("Looks").accessibilityIdentifier("custom-element-look")
                    }
                    PanelField(label: "Shortcut") {
                        Picker("Shortcut", selection: bind(\.shortcut)) {
                            ForEach(CustomElementOptions.shortcuts, id: \.self) { Text(CustomElementOptions.shortcutLabel($0)).tag($0) }
                        }
                        .labelsHidden().accessibilityLabel("Shortcut").accessibilityIdentifier("custom-element-shortcut")
                    }
                    PanelField(label: "Blank lines before") {
                        Picker("Blank lines before", selection: Binding(
                            get: { model.form?.spaceBefore ?? 1 }, set: { model.form?.spaceBefore = $0 })
                        ) {
                            ForEach(0...3, id: \.self) { Text(String($0)).tag($0) }
                        }
                        .labelsHidden().accessibilityLabel("Blank lines before").accessibilityIdentifier("custom-element-space")
                    }
                    PanelField(label: "Enter creates") {
                        Picker("Enter creates", selection: bind(\.nextElement)) {
                            ForEach(model.nextOptions, id: \.value) { Text($0.label).tag($0.value) }
                        }
                        .labelsHidden().accessibilityLabel("Enter creates").accessibilityIdentifier("custom-element-next")
                    }
                }
                Toggle("Capital letters", isOn: Binding(get: { model.form?.uppercase ?? false }, set: { model.form?.uppercase = $0 }))
                    .toggleStyle(.checkbox).accessibilityIdentifier("custom-element-uppercase")
                Toggle("Italic", isOn: Binding(get: { model.form?.italic ?? false }, set: { model.form?.italic = $0 }))
                    .toggleStyle(.checkbox).accessibilityIdentifier("custom-element-italic")
                PanelField(label: "Name in Final Draft files", hint: "Used when exporting and importing .fdx. Empty uses the name in capitals.") {
                    TextField("Name in Final Draft files", text: bind(\.exportName)).textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Name in Final Draft files").accessibilityIdentifier("custom-element-export")
                }
                if let error = model.error { PanelBanner(tone: .err, text: error) }
                HStack {
                    Spacer()
                    Button("Cancel") { model.cancelForm() }.accessibilityIdentifier("custom-element-cancel")
                    Button(form.id == nil ? "Add" : "Save") { Task { await model.save() } }
                        .keyboardShortcut(.defaultAction)
                        .disabled(!model.canSave)
                        .accessibilityIdentifier("custom-element-save")
                }
            }
            .padding(10)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("custom-element-form")
        }
    }
}

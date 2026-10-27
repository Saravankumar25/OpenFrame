// The Templates page of Project Settings (Windows: Templates.tsx, mock 153).

import OpenFrameCore
import SwiftUI

struct TemplatesView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: TemplatesModel?
    @State private var editing: EditTarget?
    @State private var preview: TemplateDto?
    @State private var applying: TemplateDto?
    @State private var deleting: TemplateDto?

    struct EditTarget: Identifiable {
        let template: TemplateDto?
        var id: String { template.map { "\($0.scope.rawValue):\($0.id)" } ?? "new" }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: PageSpacing.medium) {
                PageHeader("Templates", subtitle: "Would a saved starting point help me begin faster?") {
                    Button { editing = EditTarget(template: nil) } label: { Label("Create Template", systemImage: "plus") }
                        .buttonStyle(.ofProminent)
                        .accessibilityIdentifier("template-create")
                }
                content
                Text("Templates are optional. Every document can also start blank.")
                    .font(.caption).foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
            }
            .padding(.bottom, PageSpacing.large)
            .pageColumn(maxWidth: PageSpacing.column + 100)
        }
        .task {
            if model == nil { model = TemplatesModel(app: app) }
            await model?.load()
        }
        .task(id: app.changes.key(["template"])) { await model?.load() }
        .themedSheet(item: $editing) { target in
            if let model { TemplateEditSheet(model: model, template: target.template) }
        }
        .themedSheet(item: Binding(get: { preview.map(IdentifiedTemplate.init) }, set: { preview = $0?.template })) { item in
            if let model {
                TemplatePreviewSheet(model: model, template: item.template) { action in
                    switch action {
                    case .edit: editing = EditTarget(template: item.template)
                    case .create: applying = item.template
                    }
                }
            }
        }
        .themedSheet(item: Binding(get: { applying.map(IdentifiedTemplate.init) }, set: { applying = $0?.template })) { item in
            if let model { TemplateApplySheet(model: model, template: item.template) }
        }
        .confirmationDialog(
            "Delete template \u{201C}\(deleting?.name ?? "")\u{201D}?",
            isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } }),
            titleVisibility: .visible
        ) {
            Button("Delete Template", role: .destructive) {
                if let t = deleting { Task { await model?.remove(t) } }
                deleting = nil
            }
            .accessibilityIdentifier("template-delete-confirm")
            Button("Cancel", role: .cancel) { deleting = nil }
                .accessibilityIdentifier("template-delete-cancel")
        } message: {
            Text("This template is removed from this computer. Projects and documents that already used it are not affected.")
        }
        .accessibilityIdentifier("project-settings-templates")
    }

    struct IdentifiedTemplate: Identifiable {
        let template: TemplateDto
        var id: String { "\(template.scope.rawValue):\(template.id)" }
    }

    @ViewBuilder private var content: some View {
        if let model {
            if model.loading {
                ProgressView().frame(maxWidth: .infinity, minHeight: 120)
            } else if let listing = model.listing {
                section("Built-in", "Starting points that come with OpenFrame. Use one to copy it into this project.", listing.builtin, empty: "", id: "builtin")
                section("Yours", "Kept on this computer and available in every project.", listing.global,
                        empty: "Save a project template to My Templates, or create one, to reuse it in every project.", id: "global")
                section("This project", "Copies kept inside this project. Changing a template never changes anything already made from it.", listing.project,
                        empty: "No templates in this project yet. Use a built-in template or one of yours, or create one.", id: "project")
            } else {
                VStack(spacing: 8) {
                    Text("Templates couldn't be loaded.").font(.headline)
                    if let e = model.error { Text(e).foregroundStyle(.secondary).multilineTextAlignment(.center) }
                    Button("Try Again") { Task { await model.load() } }
                        .accessibilityIdentifier("templates-retry")
                }
                .frame(maxWidth: .infinity)
                .accessibilityIdentifier("templates-error")
            }
        } else {
            ProgressView()
        }
    }

    private func section(_ title: String, _ sub: String, _ items: [TemplateDto], empty: String, id: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            SectionHeader(title)
            Text(sub).font(.caption).foregroundStyle(.secondary).padding(.top, -PageSpacing.small)
            if items.isEmpty {
                Text(empty)
                    .font(.callout).foregroundStyle(.secondary)
                    .padding(14).frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
                    .accessibilityIdentifier("templates-empty-\(id)")
            } else {
                CenteredCards(cardWidth: 220, spacing: 12) {
                    ForEach(items, id: \.id) { t in card(t) }
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier("templates-section-\(id)")
    }

    private func card(_ t: TemplateDto) -> some View {
        ZStack(alignment: .topTrailing) {
            Button { preview = t } label: {
                VStack(alignment: .leading, spacing: 6) {
                    RoundedRectangle(cornerRadius: 6, style: .continuous).fill(theme["accent-soft"]).frame(height: 56)
                        .overlay(Image(systemName: "doc.on.doc").foregroundStyle(theme["accent"]))
                    Text(t.name).fontWeight(.semibold).lineLimit(1)
                    Text(t.typeLabel + (t.description.map { $0.isEmpty ? "" : " \u{00B7} \($0)" } ?? ""))
                        .font(.caption).foregroundStyle(.secondary).lineLimit(2)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Preview \(t.name), \(t.typeLabel)")
            .accessibilityIdentifier("template-card-\(t.scope.rawValue)-\(t.id)")
            Menu {
                menuItems(t)
            } label: {
                Label("Actions", systemImage: "ellipsis.circle")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .frame(width: 28, height: 28)
            .padding(18)
            .accessibilityLabel("Actions for \(t.name)")
            .accessibilityIdentifier("template-actions-\(t.scope.rawValue)-\(t.id)")
        }
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
    }

    @ViewBuilder private func menuItems(_ t: TemplateDto) -> some View {
        Button("Preview") { preview = t }
            .accessibilityIdentifier("template-menu-preview")
        if TemplateKinds.creates.contains(t.templateType) {
            Button(TemplateKinds.createLabel(t.templateType)) { applying = t }
                .accessibilityIdentifier("template-menu-create")
        }
        if t.scope != .project {
            Button("Use in This Project") { Task { await model?.copyIntoProject(t) } }
                .accessibilityIdentifier("template-menu-use")
        }
        if t.scope == .project {
            Button("Save to My Templates") { Task { await model?.keepForAll(t) } }
                .accessibilityIdentifier("template-menu-keep")
        }
        if t.scope != .builtin {
            Button("Edit") { editing = EditTarget(template: t) }
                .accessibilityIdentifier("template-menu-edit")
            Divider()
            Button("Delete\u{2026}", role: .destructive) {
                if t.scope == .project { Task { await model?.remove(t) } } else { deleting = t }
            }
            .accessibilityIdentifier("template-menu-delete")
        } else {
            Button("Copy to My Templates") { Task { await model?.keepForAll(t) } }
                .accessibilityIdentifier("template-menu-copy")
        }
    }
}

// MARK: Preview

enum TemplatePreviewAction { case edit, create }

struct TemplatePreviewSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: TemplatesModel
    let template: TemplateDto
    let onAction: (TemplatePreviewAction) -> Void

    var body: some View {
        let lists = template.content.filter { $0.value.arrayValue != nil }.sorted { $0.key < $1.key }
        VStack(alignment: .leading, spacing: 12) {
            VStack(spacing: PageSpacing.tight) {
                Text(template.name).font(.title2.weight(.semibold)).accessibilityIdentifier("template-preview-name")
                Text("\(template.typeLabel) \u{00B7} \(TemplateKinds.scopeLabel(template.scope))")
                    .font(.callout).foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    if let d = template.description, !d.isEmpty { Text(d) }
                    if lists.isEmpty {
                        Text("This template has no preset content yet.").foregroundStyle(.secondary)
                            .accessibilityIdentifier("template-preview-empty")
                    } else {
                        ForEach(lists, id: \.key) { key, value in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(key.prefix(1).uppercased() + key.dropFirst()).font(.headline)
                                ForEach(Array((value.arrayValue ?? []).enumerated()), id: \.offset) { i, item in
                                    Text("\(i + 1). \(TemplateKinds.itemText(item))").font(.callout)
                                }
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            HStack {
                Spacer()
                Button("Close") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("template-preview-close")
                if template.scope != .builtin {
                    Button("Edit") { dismiss(); onAction(.edit) }
                        .accessibilityIdentifier("template-preview-edit")
                }
                if TemplateKinds.creates.contains(template.templateType) {
                    Button(TemplateKinds.createLabel(template.templateType)) { dismiss(); onAction(.create) }
                        .accessibilityIdentifier("template-preview-create")
                }
                if template.scope == .project {
                    Button("Save to My Templates") { dismiss(); Task { await model.keepForAll(template) } }
                        .keyboardShortcut(.defaultAction)
                        .accessibilityIdentifier("template-preview-keep")
                } else {
                    Button("Use in This Project") { dismiss(); Task { await model.copyIntoProject(template) } }
                        .keyboardShortcut(.defaultAction)
                        .accessibilityIdentifier("template-preview-use")
                }
            }
        }
        .padding(20)
        .sheetSize(width: 560, height: 460)
        .accessibilityIdentifier("template-preview-sheet")
    }
}

// MARK: Create and edit

struct TemplateEditSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme
    let model: TemplatesModel
    let template: TemplateDto?
    @State private var name: String
    @State private var type: String
    @State private var scope: TemplateScope
    @State private var description: String
    @State private var lines: String
    @State private var touched = false
    @State private var busy = false

    init(model: TemplatesModel, template: TemplateDto?) {
        self.model = model
        self.template = template
        _name = State(initialValue: template?.name ?? "")
        _type = State(initialValue: template?.templateType ?? "call_sheet")
        _scope = State(initialValue: template?.scope == .global ? .global : .project)
        _description = State(initialValue: template?.description ?? "")
        _lines = State(initialValue: template.map {
            TemplateKinds.list($0.content, key: TemplateKinds.info($0.templateType).key).joined(separator: "\n")
        } ?? "")
    }

    private var invalid: Bool { name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    private var def: TemplateTypeInfo { TemplateKinds.info(type) }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(spacing: PageSpacing.tight) {
                Text(template == nil ? "Create Template" : "Edit Template").font(.title2.weight(.semibold))
                Text(template == nil ? "A saved starting point for new documents." : "Changes apply to future use only.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            Form {
                TextField("Name", text: $name, prompt: Text("e.g. My short-film call sheet"))
                    .accessibilityLabel("Template name")
                    .accessibilityIdentifier("template-name")
                if touched && invalid {
                    Text("Give the template a name.").font(.caption).foregroundStyle(theme["red-t"])
                        .accessibilityIdentifier("template-name-error")
                }
                if let template {
                    LabeledContent("Type", value: TemplateKinds.info(template.templateType).label)
                    LabeledContent("Keep in", value: template.scope == .global ? "Your templates (every project)" : "This project")
                } else {
                    Picker("Type", selection: $type) {
                        ForEach(TemplateKinds.types, id: \.value) { Text($0.label).tag($0.value) }
                    }
                    .accessibilityIdentifier("template-type")
                    Picker("Keep in", selection: $scope) {
                        Text("This project").tag(TemplateScope.project)
                        Text("Every project").tag(TemplateScope.global)
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("template-scope")
                }
                TextField("Description", text: $description, prompt: Text("e.g. Standard one-page"))
                    .accessibilityIdentifier("template-description")
                VStack(alignment: .leading, spacing: 4) {
                    Text(def.listLabel)
                    TextEditor(text: $lines)
                        .font(.body)
                        .frame(minHeight: 130)
                        .overlay(RoundedRectangle(cornerRadius: 6).stroke(theme["line"]))
                        .accessibilityLabel(def.listLabel)
                        .accessibilityIdentifier("template-lines")
                    Text("One per line. These become the \(def.listLabel.lowercased()) of new documents made from this template.")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
            .formStyle(.columns)
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("template-cancel")
                Button(template == nil ? "Create" : "Save") { Task { await save() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(busy)
                    .accessibilityIdentifier("template-save")
            }
        }
        .padding(20)
        .frame(width: 560)
        .accessibilityIdentifier("template-edit-sheet")
    }

    private func save() async {
        touched = true
        guard !invalid, !busy else { return }
        busy = true
        let ok = await model.save(existing: template, name: name, type: type, scope: scope, description: description, lines: lines)
        busy = false
        if ok { dismiss() }
    }
}

// MARK: Create from template

struct TemplateApplySheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: TemplatesModel
    let template: TemplateDto
    @State private var days: [ScheduleShootDayDto] = []
    @State private var scenes: [VisualScene] = []
    @State private var loaded = false
    @State private var dayId = ""
    @State private var sceneId = ""
    @State private var busy = false

    private var type: String { template.templateType }
    private var day: String { dayId.isEmpty ? (days.first?.id ?? "") : dayId }
    private var scene: String { sceneId.isEmpty ? (scenes.first?.sceneId ?? "") : sceneId }
    private var blocked: Bool { (type == "call_sheet" && day.isEmpty) || (type == "shot_list" && scene.isEmpty) }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(spacing: PageSpacing.tight) {
                Text(TemplateKinds.createLabel(type)).font(.title3.weight(.semibold))
                Text("From \u{201C}\(template.name)\u{201D}. The new document is an independent copy.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            if type == "call_sheet" {
                if loaded && days.isEmpty {
                    Text("Every shooting day already has a call sheet, or the schedule has no shooting day yet.")
                        .font(.callout).foregroundStyle(.secondary)
                        .accessibilityIdentifier("template-apply-no-days")
                } else {
                    Picker("Shooting day", selection: Binding(get: { day }, set: { dayId = $0 })) {
                        ForEach(days, id: \.id) { Text($0.title).tag($0.id) }
                    }
                    .accessibilityIdentifier("template-apply-day")
                }
            }
            if type == "shot_list" {
                if loaded && scenes.isEmpty {
                    Text("Write or import a screenplay first; shots belong to a scene.")
                        .font(.callout).foregroundStyle(.secondary)
                        .accessibilityIdentifier("template-apply-no-scenes")
                } else {
                    Picker("Scene", selection: Binding(get: { scene }, set: { sceneId = $0 })) {
                        ForEach(scenes, id: \.sceneId) { Text(TemplatesModel.sceneLabel($0)).tag($0.sceneId) }
                    }
                    .accessibilityIdentifier("template-apply-scene")
                }
            }
            Text("Undo removes everything this creates in one step. If any part can't be made, nothing is created.")
                .font(.callout)
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("template-apply-cancel")
                Button("Create") { Task { await create() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(blocked || busy || !loaded)
                    .accessibilityIdentifier("template-apply-create")
            }
        }
        .padding(20)
        .frame(width: 460)
        .task {
            if type == "call_sheet" { days = await model.shootingDaysWithoutCallSheet() }
            if type == "shot_list" { scenes = await model.scenes() }
            loaded = true
        }
        .accessibilityIdentifier("template-apply-sheet")
    }

    private func create() async {
        guard !blocked, !busy else { return }
        busy = true
        let done = await model.apply(template, dayId: type == "call_sheet" ? day : nil, sceneId: type == "shot_list" ? scene : nil)
        busy = false
        if let done {
            dismiss()
            TemplatesModel.go(done)
        }
    }
}

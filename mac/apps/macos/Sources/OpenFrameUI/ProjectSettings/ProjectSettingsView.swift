// Project Settings: a sheet over the project window with Project settings, Keyboard shortcuts,
// Templates, Advanced and Storage (Windows: workspaces/settings/index.tsx). The app-wide pages (Writing,
// Appearance, Offline AI) are in the Settings window (Command+Comma); the button at the bottom
// opens it.

import OpenFrameCore
import SwiftUI

extension Notification.Name {
    /// Asks the open project window to show Project Settings (the toolbar button, the project
    /// menu, the Go menu and the search palette all post this). The object is a tab raw value or nil.
    static let openFrameShowProjectSettings = Notification.Name("studio.openframe.showProjectSettings")
    /// Closes the Project Settings sheet (after "Create from template" has opened the new document).
    static let openFrameCloseProjectSettings = Notification.Name("studio.openframe.closeProjectSettings")
}

/// What the notification carries and how the project window reacts.
@MainActor
enum ProjectSettingsRouter {
    static func open(tab: ProjectSettingsTab? = nil) {
        NotificationCenter.default.post(name: .openFrameShowProjectSettings, object: tab?.rawValue)
    }
}

struct ProjectSettingsView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme
    @State private var tab: ProjectSettingsTab

    init(initialTab: ProjectSettingsTab = .general) {
        _tab = State(initialValue: initialTab)
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Picker("Project settings page", selection: $tab) {
                    ForEach(ProjectSettingsTab.allCases) { t in
                        Text(t.title).tag(t).accessibilityIdentifier("project-settings-tab-\(t.rawValue)")
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .accessibilityLabel("Project settings page")
                .accessibilityIdentifier("project-settings-tabs")
                .frame(maxWidth: 720)
                Spacer()
                Button("Done") { dismiss() }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("project-settings-done")
            }
            .padding(.horizontal, 16).padding(.vertical, 10)
            Divider()
            Group {
                switch tab {
                case .general: ProjectGeneralSettingsView()
                case .writing: WritingSettingsView()
                case .shortcuts: ShortcutsSettingsTab()
                case .templates: TemplatesView()
                case .offlineAI: AISettingsView()
                case .advanced: ProjectAdvancedSettingsView()
                case .appearance: AppearanceSettingsView()
                case .storage: storagePage
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            Divider()
            HStack {
                Text("Offline AI, Appearance and Keyboard shortcuts apply to every project on this Mac.")
                    .font(.caption).foregroundStyle(.secondary)
                Spacer()
                Button("App Settings\u{2026}") { SettingsRouter.open() }
                    .accessibilityIdentifier("project-settings-open-app-settings")
            }
            .padding(.horizontal, 16).padding(.vertical, 8)
        }
        .sheetFrame(minWidth: 860, idealWidth: 920, minHeight: 600, idealHeight: 680)
        .background(theme["bg"])
        // A container, so the sheet's id does not replace the ids of Done and the rest inside it.
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("project-settings-sheet")
        .onReceive(NotificationCenter.default.publisher(for: .openFrameCloseProjectSettings)) { _ in dismiss() }
        // Asked again while open (Move or back up in the General tab): show that page.
        .onReceive(NotificationCenter.default.publisher(for: .openFrameShowProjectSettings)) { note in
            if let asked = (note.object as? String).flatMap(ProjectSettingsTab.init(rawValue:)) { tab = asked }
        }
    }

    private var storagePage: some View {
        VStack(alignment: .leading, spacing: 0) {
            PageHeader("Storage", subtitle: "Where your work is kept on this computer, and how to move, back up or reconnect it.") { EmptyView() }
                .padding(.horizontal, 20).padding(.top, 16)
            StoragePanel()
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("project-settings-storage")
    }
}

// MARK: General

struct ProjectGeneralSettingsView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: ProjectSettingsModel?

    var body: some View {
        Group {
            if let model { ProjectGeneralForm(model: model) } else { ProgressView() }
        }
        .task {
            if model == nil { model = ProjectSettingsModel(app: app) }
            await model?.load()
        }
        .task(id: app.changes.any) { await model?.load() }
    }
}

struct ProjectGeneralForm: View {
    @Bindable var model: ProjectSettingsModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: PageSpacing.medium) {
                PageHeader("Project settings", subtitle: "Project identity fields.") {
                    HStack(spacing: PageSpacing.small) {
                        Button("Revert") { model.discard() }
                            .disabled(!model.dirty || model.saving)
                            .accessibilityIdentifier("project-settings-revert")
                        Button("Save") { Task { await model.save() } }
                            .buttonStyle(.ofProminent)
                            .disabled(!model.canSave)
                            .accessibilityIdentifier("project-settings-save")
                    }
                }
                if model.loading {
                    ProgressView().frame(maxWidth: .infinity, minHeight: 160)
                } else {
                    HStack(alignment: .top, spacing: PageSpacing.medium) {
                        form.frame(maxWidth: 560, alignment: .leading)
                        storedCard.frame(width: 260)
                    }
                    .frame(maxWidth: .infinity)
                }
                if let error = model.error { SettingsErrorText(message: error).frame(maxWidth: .infinity) }
            }
            .padding(.bottom, PageSpacing.large)
            .pageColumn(maxWidth: PageSpacing.column + 100)
        }
        .accessibilityIdentifier("project-settings-general")
    }

    private var form: some View {
        VStack(alignment: .leading, spacing: 12) {
            field("Title", required: true, error: model.showTitleError && model.titleMissing ? "A title is required." : nil) {
                TextField("Title", text: $model.draft.title)
                    .accessibilityLabel("Title")
                    .accessibilityIdentifier("project-settings-title")
            }
            field("Type", required: true, hint: model.typeHint) {
                Picker("Type", selection: $model.draft.projectType) {
                    ForEach(ProjectSettingsDraft.typeChoices(current: model.initial.projectType), id: \.self) { Text($0.rawValue).tag($0) }
                }
                .labelsHidden()
                .accessibilityLabel("Type")
                .accessibilityIdentifier("project-settings-type")
            }
            HStack(alignment: .top, spacing: 12) {
                field("Language") {
                    TextField("Add later", text: $model.draft.language)
                        .accessibilityLabel("Language")
                        .accessibilityIdentifier("project-settings-language")
                }
                field("Genre") {
                    TextField("Add later", text: $model.draft.genre)
                        .accessibilityLabel("Genre")
                        .accessibilityIdentifier("project-settings-genre")
                }
            }
            HStack(alignment: .top, spacing: 12) {
                field("Creator") {
                    TextField("Add later", text: $model.draft.creator)
                        .accessibilityLabel("Creator")
                        .accessibilityIdentifier("project-settings-creator")
                }
                field("Status") {
                    Picker("Status", selection: $model.draft.status) {
                        ForEach(ProjectStatus.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                    }
                    .labelsHidden()
                    .accessibilityLabel("Status")
                    .accessibilityIdentifier("project-settings-status")
                }
            }
            field("Logline") {
                TextField("One or two sentences about the film (optional)", text: $model.draft.logline, axis: .vertical)
                    .lineLimit(2...4)
                    .accessibilityLabel("Logline")
                    .accessibilityIdentifier("project-settings-logline")
            }
            field("Project notes") {
                TextField("Shoot window, budget limits, anything to remember about this project",
                          text: $model.draft.projectNotes, axis: .vertical)
                    .lineLimit(3...8)
                    .accessibilityLabel("Project notes")
                    .accessibilityIdentifier("project-settings-notes")
            }
            Text("Everything here is optional except title and type. Language, genre and creator are reused on exported documents.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .textFieldStyle(.roundedBorder)
        .padding(16)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
    }

    private var storedCard: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Stored on this computer").font(.headline)
            if let p = model.settings?.project {
                Text(p.path)
                    .font(.callout).textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("project-settings-path")
                Button { Task { await model.openLocation() } } label: {
                    Label("Open project location", systemImage: "folder")
                }
                .controlSize(.small)
                .accessibilityIdentifier("project-settings-open-location")
                Button("Move or back up\u{2026}") { ProjectSettingsRouter.open(tab: .storage) }
                    .buttonStyle(.link)
                    .controlSize(.small)
                    .accessibilityIdentifier("project-settings-move-or-back-up")
                Divider()
                Text("Created \(Formatting.dateTime(ms: p.createdAt))").font(.caption).foregroundStyle(.secondary)
                Text("Last changed \(Formatting.dateTime(ms: p.updatedAt))").font(.caption).foregroundStyle(.secondary)
            }
            Text("The project works fully offline. Everything stays in this folder.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Where this project is stored")
    }

    private func field<Content: View>(
        _ label: String, required: Bool = false, error: String? = nil, hint: String? = nil,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(required ? "\(label) (required)" : label).font(.callout.weight(.medium))
            content()
            if let error {
                Text(error).font(.caption).foregroundStyle(theme["red-t"])
                    .accessibilityIdentifier("project-settings-error-\(label.lowercased())")
            } else if let hint {
                Text(hint).font(.caption).foregroundStyle(.secondary)
                    .accessibilityIdentifier("project-settings-hint-\(label.lowercased())")
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

// MARK: Advanced

/// Breakdown category colours and icons: the same page as the Breakdown workspace's Categories
/// tab, over its own hub.
struct ProjectAdvancedSettingsView: View {
    @Environment(AppModel.self) private var app
    @State private var hub: BreakdownHub?

    var body: some View {
        Group {
            if let hub {
                BreakdownCategorySettingsView(model: hub.categories)
                    .environment(hub)
                    .overlay(alignment: .bottom) { BDNoticeBar(hub: hub) }
            } else {
                ProgressView()
            }
        }
        .task {
            if hub == nil { hub = BreakdownHub(app: app) }
            await hub?.reloadStyles()
            await hub?.reloadMembers()
        }
        .task(id: app.changes.key(BDTables.categoryStyle)) { await hub?.reloadStyles() }
        .accessibilityIdentifier("project-settings-advanced")
    }
}

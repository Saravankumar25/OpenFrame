// General: display name and the versions. Where the data lives is on the Storage tab.

import AppKit
import OpenFrameCore
import SwiftUI

struct GeneralSettingsView: View {
    @Environment(AppModel.self) private var app
    @State private var info: AppInfo?
    @State private var name = ""
    @State private var saving = false
    @State private var error: String?
    @State private var saved = false

    var body: some View {
        Form {
            Section("You") {
                TextField("Display name", text: $name)
                    .accessibilityLabel("Display name")
                    .accessibilityIdentifier("settings-display-name")
                    .onSubmit { Task { await save() } }
                HStack {
                    Button("Save Name") { Task { await save() } }
                        .disabled(saving || name.trimmingCharacters(in: .whitespaces).isEmpty || name == info?.profile.displayName)
                        .accessibilityIdentifier("settings-save-name")
                    if saved { Text("Saved").foregroundStyle(.secondary) }
                }
                if let error { SettingsErrorText(message: error) }
                Text("This name appears on your notes, comments and history in the projects you open.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            Section("Version") {
                LabeledContent("OpenFrame Studio", value: info?.appVersion ?? AppVersion.current)
                LabeledContent("Core", value: Core.version)
            }
        }
        .formStyle(.grouped)
        .task(id: app.isRunning) { await load() }
    }

    private func load() async {
        guard let core = app.core else { return }
        do {
            let loaded = try await core.appInfo()
            info = loaded
            name = loaded.profile.displayName
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }

    private func save() async {
        guard let core = app.core, !saving else { return }
        saving = true
        error = nil
        saved = false
        defer { saving = false }
        do {
            let profile = try await core.appSetDisplayName(DisplayNameArgs(displayName: name))
            name = profile.displayName
            info = try await core.appInfo()
            saved = true
        } catch {
            self.error = CoreFailure(asCore: error).message
        }
    }
}

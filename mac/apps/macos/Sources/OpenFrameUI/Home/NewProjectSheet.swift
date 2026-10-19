// New Project: title, type, optional details, and where it goes. A folder that a cloud service
// syncs gets the same warning as on Windows (project.location_check), because sync tools can
// damage an open database.

import AppKit
import SwiftUI
import OpenFrameCore

struct NewProjectSheet: View {
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @Environment(\.ofTheme) private var theme

    @State private var title = ""
    @State private var type: ProjectType = .featureFilm
    @State private var genre = ""
    @State private var creator = ""
    @State private var parentDir: URL?
    @State private var cloud: LocationCheck?
    @State private var creating = false
    @State private var error: String?
    @FocusState private var titleFocused: Bool

    private var trimmed: String { title.trimmingCharacters(in: .whitespacesAndNewlines) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("New Project")
                .font(.title2.weight(.semibold))
                .frame(maxWidth: .infinity)
                .padding(.bottom, PageSpacing.medium)
            Form {
                TextField("Title", text: $title, prompt: Text("Untitled film"))
                    .focused($titleFocused)
                    .accessibilityIdentifier("new-project-title")
                Picker("Type", selection: $type) {
                    ForEach(ProjectType.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                .accessibilityLabel("Type")
                .accessibilityIdentifier("new-project-type")
                TextField("Genre", text: $genre, prompt: Text("Optional"))
                TextField("Written by", text: $creator, prompt: Text("Optional"))
                LabeledContent("Location") {
                    HStack {
                        Text(parentDir?.path ?? app.info?.projectsDir ?? "Documents/OpenFrame/Projects")
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .foregroundStyle(.secondary)
                        Button("Choose…") { chooseFolder() }
                    }
                }
            }
            .formStyle(.grouped)
            .scrollDisabled(true)

            if let service = cloud?.cloudService {
                CloudWarning(service: service, alternative: cloud?.localAlternative) { alt in
                    parentDir = URL(fileURLWithPath: alt, isDirectory: true)
                    cloud = nil
                }
                .padding(.top, 8)
            }
            if let error {
                Text(error)
                    .font(.callout)
                    .foregroundStyle(theme["red-t"])
                    .padding(.top, 8)
                    .accessibilityIdentifier("new-project-error")
            }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button("Create") { create() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(trimmed.isEmpty || creating)
                    .accessibilityIdentifier("new-project-create")
            }
            .padding(.top, 16)
        }
        .padding(PageSpacing.large)
        .frame(width: 500)
        .onAppear { titleFocused = true }
        .task(id: parentDir) { await checkLocation() }
    }

    private func chooseFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.prompt = "Choose"
        panel.message = "Choose the folder that will hold the new project"
        if panel.runModal() == .OK { parentDir = panel.url }
    }

    private func checkLocation() async {
        guard let core = app.core else { return }
        let path = parentDir?.path ?? app.info?.projectsDir ?? ""
        guard !path.isEmpty else { return }
        cloud = try? await core.projectLocationCheck(LocationCheckArgs(path: path))
    }

    private func create() {
        guard !trimmed.isEmpty else { return }
        creating = true
        error = nil
        let args = CreateProjectArgs(
            title: trimmed,
            projectType: type,
            genre: genre.isEmpty ? nil : genre,
            creator: creator.isEmpty ? nil : creator,
            parentDir: parentDir?.path
        )
        Task {
            if await app.createProject(args) {
                dismiss()
            } else {
                // The app model shows the core's message in an alert; keep the sheet open so
                // nothing typed is lost.
                creating = false
            }
        }
    }
}

struct CloudWarning: View {
    let service: String
    let alternative: String?
    let useAlternative: (String) -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("\(service) syncs this folder", systemImage: "icloud.and.arrow.up")
                .font(.subheadline.weight(.semibold))
            Text("Sync services can copy a project while OpenFrame is writing to it, which can damage it. Keep projects in a folder that is not synced, and make backups instead.")
                .font(.callout)
                .fixedSize(horizontal: false, vertical: true)
            if let alternative {
                Button("Use \(alternative)") { useAlternative(alternative) }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["yellow-soft"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["yellow-line"]))
        .accessibilityIdentifier("cloud-warning")
    }
}

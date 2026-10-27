// The library setup screen fills the window, like the team gate: it cannot be dismissed, and
// Quit (or closing the window) closes the app without entering it.

import AppKit
import SwiftUI
import OpenFrameCore

struct LibrarySetupView: View {
    @State var model: LibrarySetupModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                BrandMark(size: 48)
                Text("Set up your OpenFrame library")
                    .font(.system(size: 24, weight: .semibold)).tracking(-0.3)
                    .accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("library-setup-title")
                Text("OpenFrame keeps your projects and your Global Idea Vault in one folder, your OpenFrame library. Choose where it should be. You can move it later in Settings > Storage.")
                    .fixedSize(horizontal: false, vertical: true)

                if model.missing, let root = model.status.root {
                    IXBanner(tone: .warn, text: Text("Your library was not found at \(root). If it is on an external drive, connect the drive and choose Try Again. If you moved the folder, locate it below.")) {
                        Button("Try Again") { Task { await model.tryAgain() } }
                            .disabled(model.busy)
                            .accessibilityIdentifier("library-setup-try-again")
                    }
                    .accessibilityIdentifier("library-setup-missing")
                }

                if !model.status.found.isEmpty { foundSection }

                if model.choices.count > 1 {
                    Picker("Where to keep your library", selection: Binding(get: { model.choice }, set: { model.choose($0) })) {
                        ForEach(model.choices) { c in Text(c.label).tag(c) }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .accessibilityLabel("Where to keep your library")
                    .accessibilityIdentifier("library-setup-choice")
                }

                choiceCard

                if model.storeBuild {
                    Text(StorageRules.storeBuildNote).font(.caption).foregroundStyle(.secondary)
                        .accessibilityIdentifier("library-setup-store-note")
                }
                if let error = model.error {
                    IXBanner(.err, error).textSelection(.enabled).accessibilityIdentifier("library-setup-error")
                }
                HStack {
                    Spacer()
                    Button("Quit OpenFrame") { NSApplication.shared.terminate(nil) }
                        .accessibilityIdentifier("library-setup-quit")
                }
            }
            .frame(maxWidth: 680, alignment: .leading)
            .padding(.horizontal, 16)
            .padding(.vertical, 40)
            .frame(maxWidth: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme["bg"])
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("library-setup")
    }

    private var foundSection: some View {
        card {
            Text("OpenFrame data found on this computer").font(.headline).accessibilityAddTraits(.isHeader)
            ForEach(Array(model.status.found.enumerated()), id: \.element.path) { index, f in
                HStack(alignment: .center, spacing: 12) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(f.libraryRoot ?? f.path).font(.callout).textSelection(.enabled)
                            .fixedSize(horizontal: false, vertical: true)
                        Text(StorageRules.foundText(f)).font(.caption).foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    Spacer(minLength: 8)
                    Button("Use This Library") { Task { await model.connect(f.libraryRoot ?? f.path) } }
                        .buttonStyle(.ofProminent)
                        .disabled(model.busy)
                        .accessibilityLabel("Use This Library, \(f.libraryRoot ?? f.path)")
                        .accessibilityIdentifier("library-setup-use-\(index)")
                }
            }
        }
        .accessibilityLabel("OpenFrame data found on this computer")
        .accessibilityIdentifier("library-setup-found")
    }

    @ViewBuilder private var choiceCard: some View {
        card {
            switch model.choice {
            case .useDefault:
                Text(model.status.suggested).font(.callout).textSelection(.enabled)
                    .accessibilityIdentifier("library-setup-suggested")
                Text("This folder is in your own home folder, outside Documents. iCloud Drive does not sync it with Desktop & Documents, so cloud syncing can never lock or replace your files while you work. Back up this folder.")
                    .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                Button("Create Library Here") { Task { await model.useDefault() } }
                    .buttonStyle(.ofProminent)
                    .disabled(model.busy)
                    .accessibilityIdentifier("library-setup-create-default")
            case .newLocation:
                Text("Choose a folder. OpenFrame makes a folder named OpenFrame inside it for your library.")
                    .font(.caption).foregroundStyle(.secondary)
                Button { Task { await model.pickParent() } } label: { Label("Choose Folder\u{2026}", systemImage: "folder") }
                    .disabled(model.busy)
                    .accessibilityIdentifier("library-setup-choose-parent")
                if let path = model.newLibraryPath {
                    Text("Your library will be \(path)").font(.callout).textSelection(.enabled)
                        .accessibilityIdentifier("library-setup-new-path")
                    if let cloud = model.found?.cloudService {
                        IXBanner(.warn, "That folder is synced by \(cloud). Syncing can lock or replace files while OpenFrame writes to them. A folder that is not synced is safer.")
                            .accessibilityIdentifier("library-setup-synced")
                    }
                    Button("Create Library") { Task { await model.createLibrary() } }
                        .buttonStyle(.ofProminent)
                        .disabled(model.busy)
                        .accessibilityIdentifier("library-setup-create")
                }
            case .locate:
                Text("After a reinstall, or when you moved your library or brought it from another drive, choose its folder. OpenFrame shows what it found before connecting anything, and never makes an empty library in its place.")
                    .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                Button { Task { await model.pickExisting() } } label: { Label("Choose Folder\u{2026}", systemImage: "folder") }
                    .disabled(model.busy)
                    .accessibilityIdentifier("library-setup-choose-existing")
                if let found = model.found {
                    IXBanner(found.libraryRoot != nil ? .info : .warn, StorageRules.inspectionText(found))
                        .textSelection(.enabled)
                        .accessibilityIdentifier("library-setup-preview")
                    if let root = found.libraryRoot {
                        Button("Connect Library") { Task { await model.connect(root) } }
                            .buttonStyle(.ofProminent)
                            .disabled(model.busy)
                            .accessibilityIdentifier("library-setup-connect")
                    }
                }
            }
        }
        .accessibilityLabel("Your choice")
    }

    private func card<C: View>(@ViewBuilder _ content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 10) { content() }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
            .accessibilityElement(children: .contain)
    }
}

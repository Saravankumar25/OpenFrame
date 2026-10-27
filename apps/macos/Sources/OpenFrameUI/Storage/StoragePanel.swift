// Settings > Storage: the library folder (the one to back up), the folder for new projects, the
// Global Idea Vault, the open project and the app's own technical data, each with the actions
// that change it (Windows: features/storage/StoragePanel.tsx). Shown in the Settings window and
// in Project Settings.

import SwiftUI
import OpenFrameCore

struct StoragePanel: View {
    @Environment(AppModel.self) private var app
    @State private var model: StorageModel?

    var body: some View {
        Group {
            if let model { StoragePanelContent(model: model) } else { ProgressView().frame(maxWidth: .infinity, minHeight: 200) }
        }
        .task {
            if model == nil { model = StorageModel(app: app) }
            await model?.load()
        }
        // A project opened, closed or moved, or the vault changed: read the places again.
        .task(id: reloadKey) { await model?.load() }
        .accessibilityIdentifier("settings-storage")
    }

    private var reloadKey: String {
        let project: String = if case let .project(p) = app.phase { p.path } else { "" }
        return "\(project)-\(app.changes.key(["vault_item", "asset"], store: .global))-\(app.relocations)"
    }
}

struct StoragePanelContent: View {
    @Bindable var model: StorageModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                if let error = model.error {
                    IXBanner(tone: .err, text: Text(error)) { dismiss }
                        .accessibilityIdentifier("storage-error")
                } else if let notice = model.notice {
                    IXBanner(tone: .ok, text: Text(notice)) { dismiss }
                        .accessibilityIdentifier("storage-notice")
                }
                if let s = model.info {
                    sections(s)
                } else if let e = model.loadError {
                    SettingsErrorText(message: e)
                } else {
                    ProgressView().frame(maxWidth: .infinity, minHeight: 160)
                }
            }
            .padding(20)
            .frame(maxWidth: 820, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .themedSheet(item: $model.action) { action in
            StorageTaskSheet(model: action) { model.action = nil }
        }
        .themedSheet(item: Binding(get: { model.vaultFound.map(VaultFoundItem.init) }, set: { if $0 == nil { model.vaultFound = nil } })) { item in
            ConnectVaultSheet(found: item.found, connect: { try await model.connectVault(item.found) }) { model.vaultFound = nil }
        }
        .themedSheet(item: Binding(get: { model.libraryFound.map(LibraryFoundItem.init) }, set: { if $0 == nil { model.libraryFound = nil } })) { item in
            ConnectLibrarySheet(
                found: item.found, current: model.info?.libraryDir ?? "", same: model.isCurrentLibrary(item.found),
                connect: { try await model.connectLibrary(item.found) }) { model.libraryFound = nil }
        }
        .confirmationDialog(
            "Take over these private notes?",
            isPresented: Binding(get: { model.adopting != nil }, set: { if !$0 { model.adopting = nil } }),
            presenting: model.adopting
        ) { owner in
            Button("Make Them Mine") { Task { await model.adopt(owner) } }
                .accessibilityIdentifier("storage-adopt-confirm")
            Button("Cancel", role: .cancel) { model.adopting = nil }
        } message: { owner in
            Text("\(StorageRules.noteCount(owner.noteCount)) will become yours and visible only to you. This is recorded in Activity and cannot be undone.")
        }
    }

    private var dismiss: some View {
        Button { model.dismissMessages() } label: { Image(systemName: "xmark") }
            .buttonStyle(.borderless)
            .accessibilityLabel("Dismiss")
            .accessibilityIdentifier("storage-message-dismiss")
    }

    @ViewBuilder private func sections(_ s: StorageInfo) -> some View {
        let relocate = model.canRelocate
        if !relocate {
            IXBanner(.info, StorageRules.storeBuildNote).accessibilityIdentifier("storage-store-note")
        }
        StoragePlace(
            title: "Your OpenFrame library", path: s.libraryDir, id: "library",
            note: "Back up this folder. It holds library.json, which remembers who you are on this computer and where your work is, so a reinstall finds everything again. By default it also holds your projects and the Global Idea Vault."
        ) {
            folderButton("storage-library-open") { await model.reveal("dataRoot") }
            if relocate {
                Button("Move Library\u{2026}") { model.moveLibrary() }
                    .accessibilityIdentifier("storage-library-move")
                Button("Locate Existing Library\u{2026}") { Task { await model.locateLibrary() } }
                    .accessibilityIdentifier("storage-library-locate")
            }
        }
        StoragePlace(
            title: "Folder for new projects", path: s.projectsDir + (s.projectsDirIsDefault ? " (default)" : ""), id: "projects",
            note: "New projects are created here unless you choose another folder when you create one. Changing it does not move the projects you already have."
        ) {
            if relocate {
                Button("Change Folder\u{2026}") { Task { await model.changeProjectsDir() } }
                    .accessibilityIdentifier("storage-projects-change")
            }
        }
        StoragePlace(
            title: "Global Idea Vault", path: s.vaultDir + (s.vaultDirIsDefault ? " (default)" : ""), id: "vault",
            note: s.vaultExists
                ? "Your ideas, research and their pictures, shared by every project."
                : "Not created yet. It is created the first time you open the Idea Vault, or you can connect an existing vault."
        ) {
            folderButton("storage-vault-open") { await model.reveal("globalVault") }.disabled(!s.vaultExists)
            Button("Back Up\u{2026}") { model.archive(.backup) }.disabled(!s.vaultExists)
                .accessibilityIdentifier("storage-vault-backup")
            Button("Export Package\u{2026}") { model.archive(.package) }.disabled(!s.vaultExists)
                .accessibilityIdentifier("storage-vault-export")
            Button("Restore\u{2026}") { Task { await model.restore() } }
                .accessibilityIdentifier("storage-vault-restore")
            if relocate {
                Button("Move\u{2026}") { model.moveVault() }.disabled(!s.vaultExists)
                    .accessibilityIdentifier("storage-vault-move")
                Button("Locate Existing Vault\u{2026}") { Task { await model.locateVault() } }
                    .accessibilityIdentifier("storage-vault-locate")
            }
        } footer: {
            if let o = model.orphans, o.count > 0 {
                HStack(spacing: 8) {
                    Text(StorageRules.orphanLine(o)).font(.callout)
                    Button("Add Them Back") { Task { await model.recoverVaultFiles() } }
                        .buttonStyle(.ofProminent)
                        .disabled(model.busy)
                        .accessibilityIdentifier("storage-vault-add-back")
                }
            }
        }
        if let project = s.projectDir {
            StoragePlace(
                title: "This project", path: project, id: "project",
                note: "Everything in this project lives in this folder. Moving it keeps its history, members and private notes."
            ) {
                folderButton("storage-project-open") { await model.reveal("project") }
                if relocate {
                    Button("Move Project\u{2026}") { model.moveProject() }
                        .accessibilityIdentifier("storage-project-move")
                }
            } footer: {
                if !model.orphanOwners.isEmpty { formerOwners }
            }
        }
        StoragePlace(
            title: "App data", path: s.appDataDir, id: "app-data",
            note: "Settings, the recent project list, the assistant's files and logs. Not your work: uninstalling may remove it, and your library stays."
        ) {
            folderButton("storage-app-data-open") { await model.reveal("appData") }
        }
    }

    private var formerOwners: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Private notes of a former owner").font(.callout.weight(.semibold))
            Text("These private notes belong to someone who is no longer a member of this project, often you under an earlier setup of OpenFrame. Only take them over if they are yours.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            ForEach(model.orphanOwners, id: \.ownerUserId) { owner in
                HStack(spacing: 8) {
                    Text(StorageRules.noteCount(owner.noteCount)).font(.callout)
                    Spacer()
                    Button("Make Them Mine\u{2026}") { model.adopting = owner }
                        .disabled(model.busy)
                        .accessibilityLabel("Make Them Mine, \(StorageRules.noteCount(owner.noteCount))")
                        .accessibilityIdentifier("storage-adopt-\(owner.ownerUserId)")
                }
            }
        }
        .padding(.top, 4)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("storage-former-owners")
    }

    private func folderButton(_ id: String, _ run: @escaping () async -> Void) -> some View {
        Button { Task { await run() } } label: { Label("Open Folder", systemImage: "folder") }
            .accessibilityIdentifier(id)
    }
}

/// One place: a title, the folder, plain words about it, and its buttons.
struct StoragePlace<Buttons: View, Footer: View>: View {
    let title: String
    let path: String
    let id: String
    let note: String
    @ViewBuilder var buttons: () -> Buttons
    @ViewBuilder var footer: () -> Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.headline).accessibilityAddTraits(.isHeader)
            Text(path)
                .font(.callout).textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityLabel("Folder")
                .accessibilityValue(path)
                .accessibilityIdentifier("storage-\(id)-path")
            Text(note).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            StorageButtonRow { buttons() }
                .controlSize(.small)
            footer()
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier("storage-\(id)")
    }
}

extension StoragePlace where Footer == EmptyView {
    init(title: String, path: String, id: String, note: String, @ViewBuilder buttons: @escaping () -> Buttons) {
        self.init(title: title, path: path, id: id, note: note, buttons: buttons, footer: { EmptyView() })
    }
}

private struct VaultFoundItem: Identifiable {
    let found: VaultFolderInspection
    var id: String { found.vaultDir ?? "" }
}

private struct LibraryFoundItem: Identifiable {
    let found: LibraryInspection
    var id: String { found.path }
}

// MARK: Connect sheets

/// What Locate Existing Vault found, confirmed before the vault is connected. An empty vault
/// needs a second yes.
struct ConnectVaultSheet: View {
    let found: VaultFolderInspection
    let connect: () async throws -> Void
    let close: () -> Void
    @State private var sureEmpty = false
    @State private var busy = false
    @State private var error: String?

    private var empty: Bool { found.ideas == 0 }

    var body: some View {
        IXSheetFrame(title: "Connect This Vault?", width: 520, idPrefix: "connect-vault") {
            VStack(alignment: .leading, spacing: 10) {
                Text(StorageRules.locateSummary(found)).font(.callout).textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("connect-vault-summary")
                Text("OpenFrame will use this vault from now on. Your current vault is not moved or deleted, and you can connect it again later.")
                    .font(.callout).fixedSize(horizontal: false, vertical: true)
                if empty {
                    Toggle("This vault has no ideas. Connect it anyway.", isOn: $sureEmpty)
                        .toggleStyle(.checkbox)
                        .accessibilityIdentifier("connect-vault-empty")
                }
                if let error { IXBanner(.err, error).accessibilityIdentifier("connect-vault-error") }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } footer: {
            Button("Cancel", action: close).keyboardShortcut(.cancelAction).disabled(busy)
                .accessibilityIdentifier("connect-vault-cancel")
            Button(empty ? "Connect Empty Vault" : "Connect Vault") {
                busy = true
                Task {
                    do { try await connect() } catch { self.error = CoreFailure(asCore: error).message }
                    busy = false
                }
            }
            .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
            .disabled(busy || (empty && !sureEmpty))
            .accessibilityIdentifier("connect-vault-confirm")
        }
    }
}

/// Locate Existing Library: what was found, then Connect.
struct ConnectLibrarySheet: View {
    let found: LibraryInspection
    let current: String
    let same: Bool
    let connect: () async throws -> Void
    let close: () -> Void
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        IXSheetFrame(title: "Connect This Library?", width: 520, idPrefix: "connect-library") {
            VStack(alignment: .leading, spacing: 10) {
                IXBanner(found.libraryRoot != nil ? .info : .warn, StorageRules.inspectionText(found))
                    .accessibilityIdentifier("connect-library-found")
                if same {
                    Text("This is the library you use now. Nothing to change.").font(.callout)
                } else if found.libraryRoot != nil {
                    Text("OpenFrame will use this library from now on. Your current library at \(current) is not moved or deleted, and you can connect it again later.")
                        .font(.callout).fixedSize(horizontal: false, vertical: true)
                }
                if let error { IXBanner(.err, error).accessibilityIdentifier("connect-library-error") }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } footer: {
            Button("Cancel", action: close).keyboardShortcut(.cancelAction).disabled(busy)
                .accessibilityIdentifier("connect-library-cancel")
            if found.libraryRoot != nil && !same {
                Button("Connect Library") {
                    busy = true
                    Task {
                        do { try await connect() } catch { self.error = CoreFailure(asCore: error).message }
                        busy = false
                    }
                }
                .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                .disabled(busy)
                .accessibilityIdentifier("connect-library-confirm")
            }
        }
    }
}

/// Buttons in a row that wraps to the next line when the window is narrow (or the text is large).
struct StorageButtonRow: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, line: CGFloat = 0, widest: CGFloat = 0
        for sub in subviews {
            let size = sub.sizeThatFits(.unspecified)
            if x > 0 && x + size.width > width {
                y += line + spacing
                x = 0
                line = 0
            }
            x += size.width + spacing
            line = max(line, size.height)
            widest = max(widest, x - spacing)
        }
        return CGSize(width: proposal.width ?? widest, height: y + line)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, line: CGFloat = 0
        for sub in subviews {
            let size = sub.sizeThatFits(.unspecified)
            if x > bounds.minX && x + size.width > bounds.maxX {
                y += line + spacing
                x = bounds.minX
                line = 0
            }
            sub.place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: ProposedViewSize(size))
            x += size.width + spacing
            line = max(line, size.height)
        }
    }
}

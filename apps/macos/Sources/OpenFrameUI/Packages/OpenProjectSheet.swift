// The Open Project Package / Restore Backup sheet (mockup 171).

import SwiftUI
import OpenFrameCore

struct OpenProjectSheet: View {
    @State var model: OpenProjectModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let t = model.task
        let ended = model.report != nil || t.state == "failed" || t.state == "cancelled"
        IXSheetFrame(
            title: model.headline,
            sub: model.info == nil ? "Open a Full Project Package or restore a Backup Package from another disk or Mac." : nil,
            width: 580, idPrefix: "open-project-sheet"
        ) {
            content
        } footer: {
            if ended {
                Button("Close") { model.finish(); close() }.keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("open-project-close")
            } else {
                Button("Cancel") { model.finish(); close() }.keyboardShortcut(.cancelAction).disabled(t.running)
                    .accessibilityIdentifier("open-project-cancel")
                if model.info != nil && !model.started {
                    Button(model.primaryLabel) {
                        if model.mode == .replace { model.confirmReplace = true } else { Task { await model.start() } }
                    }
                    .keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("open-project-confirm")
                }
            }
        }
        .confirmationDialog("Replace the existing project?", isPresented: $model.confirmReplace, titleVisibility: .visible) {
            Button("Back Up and Replace", role: .destructive) { Task { await model.start(.replace) } }
                .accessibilityIdentifier("open-project-replace-confirm")
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("\u{201C}\(model.collision?.title ?? "")\u{201D} will be backed up to the Backups folder and then replaced by the package. Work saved after the package was created will only be in that backup.")
        }
    }

    @ViewBuilder private var content: some View {
        let t = model.task
        if let r = model.report {
            VStack(alignment: .leading, spacing: 6) {
                IXBanner(tone: .ok, text: Text(r.identityKept ? "Project opened." : "Opened as a copy.").bold())
                    .accessibilityIdentifier("open-project-done")
                Label("Validated \u{00B7} \(r.message)", systemImage: "checkmark").font(.callout)
                if let b = r.safetyBackup { Text("Safety backup: \(b)").font(.caption).foregroundStyle(.secondary).textSelection(.enabled) }
                if r.migratedFrom != nil { Text("The project was upgraded from an older OpenFrame version; a safety copy was kept.").font(.caption).foregroundStyle(.secondary) }
                PKLinkedFiles(files: r.missingExternal)
            }
        } else if model.started {
            switch t.state {
            case "failed": IXBanner(.err, "\(t.errorMessage ?? "The package could not be opened.") Nothing was changed.").accessibilityIdentifier("open-project-failed")
            case "cancelled": IXBanner(.info, "Cancelled. Nothing was changed.").accessibilityIdentifier("open-project-cancelled")
            default: PKProgress(task: t) { model.cancelTask() }
            }
        } else if let i = model.info {
            details(i)
        } else {
            VStack(spacing: 10) {
                Image(systemName: "archivebox").font(.system(size: 28)).foregroundStyle(.secondary).accessibilityHidden(true)
                Text("Choose a package").font(.headline)
                Text(".ofproject (Full Project Package) or .ofbackup (Backup Package). The package is validated before anything is created.")
                    .font(.callout).foregroundStyle(.secondary).multilineTextAlignment(.center)
                if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("open-project-error") }
                Button(model.checking ? "Checking\u{2026}" : "Choose Package\u{2026}") {
                    if let url = picker.openFile(title: "Open Project Package", extensions: PKInfo.extensions(PKInfo.projectOpenTypes)) {
                        Task { await model.choose(file: url) }
                    }
                }
                .buttonStyle(.borderedProminent).disabled(model.checking)
                .accessibilityIdentifier("open-project-choose")
            }
            .frame(maxWidth: .infinity).padding(.vertical, 12)
        }
    }

    @ViewBuilder private func details(_ i: PackageInspection) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                IXFact(label: "Package", value: i.typeLabel, id: "open-project-fact-package")
                IXFact(label: "Project", value: i.manifest.sourceProjectTitle, id: "open-project-fact-project")
            }
            HStack(spacing: 8) {
                IXFact(label: "Created", value: PKText.day(ms: i.manifest.exportedAt), detail: i.manifest.label, id: "open-project-fact-created")
                VStack(alignment: .leading, spacing: 2) {
                    Text("Size").font(.caption).foregroundStyle(.secondary)
                    HStack { Text(PKText.formatBytes(i.sizeBytes)).fontWeight(.semibold); IXChip(text: "Valid", tone: .green) }
                }
                .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("open-project-fact-size")
            }
            if let c = model.collision {
                modeCard(.copy, title: "Open as Copy", chip: ("Default", .green),
                         text: "Creates \u{201C}\(i.manifest.sourceProjectTitle) (copy)\u{201D} with a new identity. Your existing project is not touched.",
                         id: "open-project-mode-copy")
                modeCard(.replace, title: "Replace Existing After Backup", chip: ("High impact", .red),
                         text: "Saves a backup of \u{201C}\(c.title)\u{201D} first, then replaces it with the package\(c.isOpen ? " (it will be closed)" : "").",
                         id: "open-project-mode-replace")
            } else {
                IXBanner(.info, "No project with this identity exists here. It will open as the same project, keeping its identity.")
                    .accessibilityIdentifier("open-project-no-collision")
            }
            Text("Import report").font(.callout.weight(.semibold))
            Label("Validated" + (model.missingExternal.isEmpty
                ? " \u{00B7} all files are included or available."
                : " \u{00B7} \(model.missingExternal.count) linked external file\(model.missingExternal.count == 1 ? " is" : "s are") missing on this Mac (the project still opens)."),
                  systemImage: "checkmark").font(.callout)
            PKLinkedFiles(files: i.external)
            if let n = model.notesNotice {
                IXBanner(tone: .info, text: Text("Private notes.").bold() + Text(" " + n)).accessibilityIdentifier("open-project-notes")
            }
            Text("Importing never silently replaces the current project.").font(.caption).foregroundStyle(.secondary)
            if let e = model.startError { IXBanner(.err, e).accessibilityIdentifier("open-project-start-error") }
        }
    }

    private func modeCard(_ m: PackageOpenMode, title: String, chip: (String, IXTone), text: String, id: String) -> some View {
        Button { model.mode = m } label: {
            HStack(alignment: .top, spacing: 8) {
                Image(systemName: model.mode == m ? "largecircle.fill.circle" : "circle")
                    .foregroundStyle(model.mode == m ? theme["accent"] : theme["muted"]).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    HStack { Text(title).fontWeight(.semibold); IXChip(text: chip.0, tone: chip.1) }
                    Text(text).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                }
                Spacer()
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(model.mode == m ? theme["accent"] : theme["line"]))
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(model.mode == m ? .isSelected : [])
        .accessibilityIdentifier(id)
    }
}

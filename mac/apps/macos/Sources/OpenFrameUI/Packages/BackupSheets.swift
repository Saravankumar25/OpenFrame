// The Create Backup sheet (mockup 169) and the Export Full Project sheet (mockup 170).

import SwiftUI
import OpenFrameCore

struct BackupSheet: View {
    @State var model: BackupModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void

    var body: some View {
        let t = model.task
        let finished = model.report != nil || t.state == "cancelled" || t.state == "failed"
        IXSheetFrame(
            title: "Create Backup", sub: "A recoverable copy of supported project state.", width: 560, idPrefix: "backup-sheet"
        ) {
            if let p = model.preview {
                content(p)
            } else if let e = model.loadError {
                IXBanner(.err, e)
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 120)
            }
        } footer: {
            if finished {
                Button("Close") { model.finish(); close() }.keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                    .accessibilityIdentifier("backup-close")
            } else {
                Button("Cancel") { model.finish(); close() }.keyboardShortcut(.cancelAction).disabled(t.running)
                    .accessibilityIdentifier("backup-cancel")
                Button("Create Backup") { Task { await model.start() } }
                    .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                    .disabled(!model.canStart)
                    .accessibilityIdentifier("backup-create")
            }
        }
        .task { await model.load() }
    }

    @ViewBuilder private func content(_ p: BackupPreview) -> some View {
        let t = model.task
        if let r = model.report {
            PKDoneBanner(title: "Backup created.", path: r.path, app: model.app) {
                Text("\(r.summary). \(PKText.formatBytes(r.bytes)).")
                PKLinkedFiles(files: r.external + r.missing)
            }
        } else if t.taskId != nil {
            switch t.state {
            case "cancelled": IXBanner(.info, "Backup cancelled. No backup file was left behind.").accessibilityIdentifier("backup-cancelled")
            case "failed": IXBanner(.err, "\(t.errorMessage ?? "The backup could not be created.") Your project was not changed.").accessibilityIdentifier("backup-failed")
            default: PKProgress(task: t) { model.cancel() }
            }
        } else {
            VStack(alignment: .leading, spacing: 10) {
                labelled("Backup name", hint: "Project name, date and time, and an optional label.") {
                    TextField("Backup name", text: .constant(model.name)).textFieldStyle(.roundedBorder).disabled(true)
                        .accessibilityIdentifier("backup-name")
                }
                labelled("Label (optional)") {
                    TextField("e.g. before-rewrite", text: $model.label).textFieldStyle(.roundedBorder)
                        .onChange(of: model.label) { _, v in if v.count > 80 { model.label = String(v.prefix(80)) } }
                        .accessibilityIdentifier("backup-label")
                }
                labelled("Save to", hint: "Choose another disk if you can. Project backups are separate from your active project.") {
                    HStack {
                        TextField("Choose where to save the backup", text: .constant(model.destination?.path ?? ""))
                            .textFieldStyle(.roundedBorder).disabled(true)
                            .accessibilityLabel("Backup location")
                            .accessibilityIdentifier("backup-location")
                        Button("Browse\u{2026}") {
                            if let url = picker.saveFile(title: "Save backup to", suggestedName: model.name, directory: model.defaultDirectory, message: nil) {
                                model.choose(destination: url)
                            }
                        }
                        .accessibilityIdentifier("backup-browse")
                    }
                }
                Toggle("Copy linked external files into the backup (portable backup)", isOn: $model.portable)
                    .toggleStyle(.checkbox).accessibilityIdentifier("backup-portable")
                Text("What this backup contains").font(.callout.weight(.semibold))
                Text("Idea Vault, Beat Board, screenplay drafts, breakdown, catalog, production planning, comments"
                     + (p.privateNoteCount > 0 ? ", your private notes" : "") + ".")
                    .font(.callout).foregroundStyle(.secondary)
                PKFileStatusTable(rows: [
                    ("Files stored in the project \u{00B7} \(PKText.formatBytes(p.includedBytes))", "Included", Int(p.includedFiles)),
                    ("Linked external files", model.portable ? "Copied" : "External", p.external.count),
                    ("Missing linked files", "Missing", p.missing.count),
                ])
                PKLinkedFiles(files: p.external + p.missing)
                if let e = model.startError { IXBanner(.err, e).accessibilityIdentifier("backup-error") }
            }
        }
    }

    private func labelled<C: View>(_ title: String, hint: String? = nil, @ViewBuilder _ c: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title).font(.callout.weight(.semibold))
            c()
            if let hint { Text(hint).font(.caption).foregroundStyle(.secondary) }
        }
    }
}

struct ExportProjectSheet: View {
    @State var model: ExportProjectModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void

    var body: some View {
        let t = model.task
        let finished = model.report != nil || t.state == "cancelled" || t.state == "failed"
        IXSheetFrame(title: "Export Full Project", width: 560, idPrefix: "export-project-sheet") {
            VStack(alignment: .leading, spacing: 10) {
                Text("A Full Project Package moves your whole working project to another Mac. It is not a review package and not a PDF collection.")
                    .font(.callout)
                if let p = model.preview {
                    content(p)
                } else if let e = model.loadError {
                    IXBanner(.err, e)
                } else {
                    ProgressView().frame(maxWidth: .infinity, minHeight: 100)
                }
            }
        } footer: {
            if finished {
                Button("Close") { model.finish(); close() }.keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                    .accessibilityIdentifier("export-project-close")
            } else {
                Button("Cancel") { model.finish(); close() }.keyboardShortcut(.cancelAction).disabled(t.running)
                    .accessibilityIdentifier("export-project-cancel")
                Button("Export Project Package\u{2026}") {
                    guard let url = picker.saveFile(title: "Export Project Package", suggestedName: model.suggestedName, directory: model.defaultDirectory, message: nil) else { return }
                    Task { await model.export(to: url) }
                }
                .keyboardShortcut(.defaultAction).buttonStyle(.ofProminent)
                .disabled(t.running || model.preview == nil)
                .accessibilityIdentifier("export-project-confirm")
            }
        }
        .task { await model.load() }
    }

    @ViewBuilder private func content(_ p: BackupPreview) -> some View {
        let t = model.task
        if let r = model.report {
            PKDoneBanner(title: "Project package exported.", path: r.path, app: model.app) {
                Text("\(r.summary). \(PKText.formatBytes(r.bytes)). Open it on the other Mac with Open Project Package\u{2026}.")
                PKLinkedFiles(files: r.external + r.missing)
            }
        } else if t.taskId != nil {
            switch t.state {
            case "cancelled": IXBanner(.info, "Export cancelled. No package file was left behind.").accessibilityIdentifier("export-project-cancelled")
            case "failed": IXBanner(.err, t.errorMessage ?? "The package could not be exported.").accessibilityIdentifier("export-project-failed")
            default: PKProgress(task: t) { model.cancel() }
            }
        } else {
            VStack(alignment: .leading, spacing: 8) {
                Text("Include").font(.callout.weight(.semibold))
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(p.groups, id: \.label) { g in
                        Toggle("\(g.label)\(g.count > 0 ? " (\(Int(g.count)))" : "")", isOn: .constant(true))
                            .toggleStyle(.checkbox).disabled(true)
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Included in the package")
                IXBanner(tone: .priv, text: Text("Private project transfer.").bold() + Text(" Private notes are included because this is for you, not for outside reviewers."))
                    .accessibilityIdentifier("export-project-private")
                Toggle("Copy linked external files into the package (\(p.external.count))", isOn: $model.portable)
                    .toggleStyle(.checkbox).accessibilityIdentifier("export-project-portable")
                if !p.missing.isEmpty {
                    Text("\(p.missing.count) linked file\(p.missing.count == 1 ? " is" : "s are") missing and will be listed as missing. The project still opens.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if let e = model.startError { IXBanner(.err, e).accessibilityIdentifier("export-project-error") }
            }
        }
    }
}

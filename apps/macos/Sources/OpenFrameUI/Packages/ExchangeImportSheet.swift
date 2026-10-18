// The Import Exchange Package sheet (mockups 161 and 162, and the stale-package choice).

import SwiftUI
import OpenFrameCore

struct ExchangeImportSheet: View {
    @State var model: ExchangeImportModel
    var picker: FilePicking = PanelFilePicker()
    /// Opens the read-only viewer for this session's package; the sheet comes back when it closes.
    var viewPackage: (PackageImportSession) -> Void = { _ in }
    var openQueue: () -> Void = {}
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    private func leave() { Task { await model.cancel(); close() } }

    var body: some View {
        Group {
            if let s = model.session {
                if model.staleStep { staleView(s) } else { mainView(s) }
            } else {
                chooseView
            }
        }
        .task { await model.resume() }
    }

    // MARK: Choose

    private var chooseView: some View {
        IXSheetFrame(title: "Import Exchange Package", sub: "What exactly will this imported package change?", width: 540, idPrefix: "exchange-import-sheet") {
            VStack(spacing: 10) {
                Image(systemName: "square.and.arrow.down").font(.system(size: 28)).foregroundStyle(.secondary).accessibilityHidden(true)
                Text("Choose a package").font(.headline)
                Text("Review, response, Story Board, Breakdown, Shot List, Schedule or Call Sheet review packages. The package is validated first; nothing changes until you choose what to import.")
                    .font(.callout).foregroundStyle(.secondary).multilineTextAlignment(.center)
                if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("exchange-import-error") }
                Button(model.busy ? "Validating\u{2026}" : "Choose Package\u{2026}") {
                    if let url = picker.openFile(title: "Import Exchange Package", extensions: PKInfo.extensions(PKInfo.exchangeOpenTypes)) {
                        Task { await model.open(file: url) }
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.busy)
                .accessibilityIdentifier("exchange-import-choose")
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 12)
        } footer: {
            Button("Cancel", action: leave).keyboardShortcut(.cancelAction).accessibilityIdentifier("exchange-import-cancel")
        }
    }

    // MARK: Stale

    private func staleView(_ s: PackageImportSession) -> some View {
        IXSheetFrame(
            title: s.staleTitle ?? "This package was created from an older project state.",
            sub: "Staleness does not mean rejection: the package needs review.", width: 540, idPrefix: "exchange-stale-sheet"
        ) {
            VStack(alignment: .leading, spacing: 8) {
                Text("This package was created from an older project state.").font(.callout).foregroundStyle(.secondary)
                ForEach(model.staleOptions, id: \.choice) { o in
                    Button { model.staleChoice = o.choice } label: {
                        HStack(alignment: .top, spacing: 8) {
                            Image(systemName: model.staleChoice == o.choice ? "largecircle.fill.circle" : "circle")
                                .foregroundStyle(model.staleChoice == o.choice ? theme["accent"] : theme["muted"]).accessibilityHidden(true)
                            VStack(alignment: .leading, spacing: 2) {
                                HStack { Text(o.title).fontWeight(.semibold); if o.isDefault { IXChip(text: "Default", tone: .green) } }
                                Text(o.hint).font(.callout).foregroundStyle(.secondary)
                            }
                            Spacer()
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous)
                            .strokeBorder(model.staleChoice == o.choice ? theme["accent"] : theme["line"]))
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(model.staleChoice == o.choice ? .isSelected : [])
                    .accessibilityIdentifier("exchange-stale-\(o.choice.rawValue)")
                }
                if let e = model.errorText { IXBanner(.err, e) }
            }
        } footer: {
            Button("Cancel", action: leave).keyboardShortcut(.cancelAction).accessibilityIdentifier("exchange-stale-cancel")
            Button("Continue") {
                Task { if await model.continueStale() == .view, let s = model.session { viewPackage(s) } }
            }
            .keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent).disabled(model.busy)
            .accessibilityIdentifier("exchange-stale-continue")
        }
    }

    // MARK: Changes

    private func mainView(_ s: PackageImportSession) -> some View {
        let result = model.result
        return IXSheetFrame(
            title: "Import Exchange Package", sub: "What exactly will this imported package change?", width: 760, idPrefix: "exchange-import-sheet"
        ) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 8) {
                    IXFact(label: "Package type", value: s.typeLabel, id: "exchange-fact-type")
                    IXFact(label: "Source project", value: s.sourceProjectTitle, id: "exchange-fact-project")
                    IXFact(label: "Source version", value: s.sourceVersion, detail: "Yours: \(s.hostVersion)", id: "exchange-fact-version")
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Compatibility").font(.caption).foregroundStyle(.secondary)
                        IXChip(text: s.compatibility, tone: PKText.compatibilityTone(s.compatibility))
                    }
                    .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("exchange-fact-compatibility")
                }
                Text("Exported by \(s.exportedBy) \u{00B7} \(PKText.day(ms: s.exportedAt)) \u{00B7} \(s.fileName)")
                    .font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("exchange-import-exported-by")
                banner(s, result)
                ForEach(s.warnings, id: \.self) { IXBanner(.info, $0) }
                if model.onlyText {
                    IXBanner(tone: .info, text: Text("Comparing \(s.sourceVersion) (package) with \(s.hostVersion) (yours). Text differences are shown for review only. Import never changes screenplay text.")) {
                        Button("Show all changes") { model.showAllChanges() }.controlSize(.small)
                            .accessibilityIdentifier("exchange-show-all")
                    }
                }
                if !model.visibleGroups.isEmpty { changesTable }
                if !model.finished && s.compatibility != "Rejected" {
                    VStack(alignment: .leading, spacing: 2) {
                        Toggle("Create Backup Before Import", isOn: $model.backup).toggleStyle(.checkbox)
                            .accessibilityIdentifier("exchange-import-backup")
                        Text("Ambiguous and unmatched notes go to the Review Queue; nothing is discarded. Conflicts are never applied unless you select them.")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }
                if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("exchange-import-error") }
            }
        } footer: {
            footer(s, result)
        } leading: {
            Text("Validation happened before anything was changed.").font(.caption).foregroundStyle(.secondary)
        }
    }

    @ViewBuilder private func banner(_ s: PackageImportSession, _ result: PackageImportResult?) -> some View {
        if let r = result {
            let tone: IXBannerTone = r.status == "Failed" ? .err : (r.status == "Applied" || r.status == "Undone") ? .ok : .info
            VStack(alignment: .leading, spacing: 2) {
                IXBanner(tone: tone, text: Text("\(PKText.resultLabel(r.status)).").bold() + Text(" " + r.message))
                if let b = r.safetyBackup { Text("Backup before import: \(b)").font(.caption).foregroundStyle(.secondary).textSelection(.enabled) }
            }
            .accessibilityIdentifier("exchange-import-result")
        } else if let rej = s.rejection {
            IXBanner(.err, "\(rej) Your project was not changed.").accessibilityIdentifier("exchange-import-rejected")
        } else if s.stale {
            IXBanner(.warn, "\(s.staleTitle ?? "") Staleness does not mean rejection. Review what you import.").accessibilityIdentifier("exchange-import-stale")
        } else if model.commentsReceived > 0 {
            let n = model.commentsReceived
            IXBanner(tone: .ok, text: Text("\(n) new comment\(n == 1 ? "" : "s") received.").bold() + Text(" Comments are attached to the matching scenes and text where safe."))
                .accessibilityIdentifier("exchange-import-comments")
        }
    }

    private var changesTable: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Changes by object").frame(maxWidth: .infinity, alignment: .leading)
                Text("Count").frame(width: 60)
                Text("State").frame(width: 150, alignment: .leading)
                Text("Apply").frame(width: 60)
            }
            .font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            .padding(.horizontal, 10).padding(.vertical, 6)
            .background(theme["surface2"])
            Divider()
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(model.visibleGroups, id: \.key) { g in
                        groupRow(g)
                        if model.expanded.contains(g.key) {
                            ForEach(model.visibleChanges.filter { $0.group == g.key }, id: \.id) { changeRow($0) }
                        }
                        Divider()
                    }
                }
            }
            .frame(maxHeight: 280)
        }
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Changes by object")
        .accessibilityIdentifier("exchange-changes")
    }

    private func groupRow(_ g: PackageChangeGroup) -> some View {
        let chip = PKText.stateChip(g.state)
        let open = model.expanded.contains(g.key)
        let sel = model.groupState(g.key)
        return HStack {
            HStack(spacing: 6) {
                Button { model.toggleExpanded(g.key) } label: { Image(systemName: open ? "chevron.down" : "chevron.right").frame(width: 14) }
                    .buttonStyle(.borderless)
                    .accessibilityLabel("\(open ? "Hide" : "Show") \(g.label)")
                    .accessibilityIdentifier("exchange-expand-\(g.key)")
                Text(g.label).fontWeight(.semibold)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text("\(Int(g.count))").frame(width: 60)
            IXChip(text: chip.label, tone: chip.tone).frame(width: 150, alignment: .leading)
            Group {
                if g.applicable > 0 && !model.finished {
                    Button {
                        model.toggleGroup(g.key, sel != .all)
                    } label: {
                        Image(systemName: sel == .all ? "checkmark.square.fill" : sel == .some ? "minus.square.fill" : "square")
                    }
                    .buttonStyle(.borderless)
                    .accessibilityLabel("Apply \(g.label)")
                    .accessibilityValue(sel == .all ? "all" : sel == .some ? "some" : "none")
                    .accessibilityIdentifier("exchange-apply-group-\(g.key)")
                } else {
                    Text("None").font(.caption).foregroundStyle(.secondary)
                }
            }
            .frame(width: 60)
        }
        .font(.callout)
        .padding(.horizontal, 10).padding(.vertical, 6)
    }

    private func changeRow(_ c: PackageChange) -> some View {
        let chip = PKText.stateChip(c.state)
        return HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 1) {
                Text(c.label)
                if let d = c.detail { Text(d).font(.caption).foregroundStyle(.secondary) }
                if !c.candidates.isEmpty {
                    Text("Possible places: " + c.candidates.map(\.label).joined(separator: " \u{00B7} ")).font(.caption).foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading).padding(.leading, 28)
            Spacer().frame(width: 60)
            IXChip(text: chip.label, tone: chip.tone).frame(width: 150, alignment: .leading)
            Group {
                if c.applicable && !model.finished {
                    Toggle("Apply \(c.label)", isOn: Binding(get: { model.selected.contains(c.id) }, set: { model.toggle(c.id, $0) }))
                        .labelsHidden().toggleStyle(.checkbox)
                        .accessibilityLabel("Apply \(c.label)")
                        .accessibilityIdentifier("exchange-apply-\(c.id)")
                }
            }
            .frame(width: 60)
        }
        .font(.callout)
        .padding(.horizontal, 10).padding(.vertical, 4)
        .accessibilityElement(children: .contain)
    }

    @ViewBuilder private func footer(_ s: PackageImportSession, _ result: PackageImportResult?) -> some View {
        if model.finished {
            if let r = result, r.canUndo, s.status != "Undone" {
                Button("Undo Import") { Task { await model.undo() } }.disabled(model.busy)
                    .accessibilityIdentifier("exchange-import-undo")
            }
            if (result?.queued ?? 0) > 0 {
                Button("Open Review Queue", action: openQueue).accessibilityIdentifier("exchange-open-queue")
            }
            Button("Close") { Task { await model.cancel(); close() } }.keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                .accessibilityIdentifier("exchange-import-close")
        } else {
            Button("Cancel", action: leave).keyboardShortcut(.cancelAction).accessibilityIdentifier("exchange-import-cancel")
            if s.compatibility != "Rejected" {
                if s.canView { Button("View Package") { viewPackage(s) }.accessibilityIdentifier("exchange-view-package") }
                if s.canCreateRecord {
                    Button("Keep as Review Record") { Task { await model.apply(.record) } }.disabled(model.busy)
                        .accessibilityIdentifier("exchange-keep-record")
                }
                if s.canCopyAsNew {
                    Button("Copy as New") { Task { await model.apply(.copyAsNew) } }.disabled(model.busy)
                        .accessibilityIdentifier("exchange-copy-as-new")
                }
                Button("Comments Only") { Task { await model.apply(.commentsOnly) } }.disabled(model.busy)
                    .accessibilityIdentifier("exchange-comments-only")
                Button("Accept All Safe") { Task { await model.apply(.allSafe) } }.disabled(model.busy)
                    .accessibilityIdentifier("exchange-accept-safe")
                Button("Import Selected") { Task { await model.apply(.selected) } }
                    .keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                    .disabled(model.busy || model.selected.isEmpty)
                    .accessibilityIdentifier("exchange-import-selected")
            }
        }
    }
}

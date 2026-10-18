// The list of call sheets and the Create Call Sheet sheet (Windows CallSheetList.tsx).

import SwiftUI
import OpenFrameCore

struct CallSheetListView: View {
    @Bindable var model: CallSheetListModel
    @Environment(AppModel.self) private var app
    @Environment(CallSheetsHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                VStack(alignment: .leading, spacing: 12) {
                    CSPageHeader(title: "Call Sheets", sub: "Which shooting day document do I need?") {
                        Button { model.beginCreate() } label: { Label("Create Call Sheet", systemImage: "plus") }
                            .buttonStyle(.borderedProminent)
                            .keyboardShortcut("n", modifiers: [.command, .option])
                            .disabled(!hub.canEdit)
                            .accessibilityIdentifier("callsheet-create")
                    }
                    if let f = model.failure {
                        CSBanner(.err, f.message).accessibilityIdentifier("callsheet-list-error")
                    }
                    if model.isEmpty { empty } else { table }
                }
                .padding(16)
            }
        }
        .task(id: app.changes.key(CSTables.callSheet)) { await model.reload() }
        .sheet(item: $model.create) { c in
            CSCreateSheet(model: c) { model.create = nil }
        }
        .accessibilityIdentifier("callsheet-list")
    }

    private var empty: some View {
        ContentUnavailableView {
            Label("Create a call sheet from a shooting day.", systemImage: "doc.richtext")
        } description: {
            Text("Scenes, cast and locations are filled in from the schedule. You add call times and practical notes.")
        } actions: {
            Button("Create Call Sheet") { model.beginCreate() }
                .buttonStyle(.borderedProminent)
                .disabled(!hub.canEdit)
                .accessibilityIdentifier("callsheet-empty-create")
        }
        .accessibilityIdentifier("callsheet-empty")
    }

    private var table: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 12) {
                HStack(spacing: 4) {
                    Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                    TextField("Search", text: $model.text)
                        .textFieldStyle(.plain)
                        .accessibilityLabel("Search call sheets")
                        .accessibilityIdentifier("callsheet-search")
                }
                .padding(.horizontal, 8).padding(.vertical, 5)
                .frame(width: 260)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
                Picker("Filter call sheets", selection: $model.filter) {
                    ForEach(CSFilter.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize()
                .accessibilityLabel("Filter call sheets")
                .accessibilityIdentifier("callsheet-filter")
                Toggle("Show replaced revisions", isOn: $model.showOld)
                    .accessibilityIdentifier("callsheet-show-replaced")
                Spacer()
            }
            VStack(spacing: 0) {
                HStack {
                    Text("Shoot day").frame(maxWidth: .infinity, alignment: .leading)
                    Text("Date").frame(width: 150, alignment: .leading)
                    Text("Status").frame(width: 120, alignment: .leading)
                    Text("Revision").frame(width: 200, alignment: .leading)
                    Text("Actions").frame(width: 190, alignment: .trailing)
                }
                .font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
                .padding(.horizontal, 10).padding(.vertical, 6)
                Divider()
                List(selection: $model.selection) {
                    ForEach(model.filtered, id: \.id) { r in
                        row(r).tag(r.id)
                    }
                    if model.filtered.isEmpty {
                        Text("No matching call sheets.").foregroundStyle(theme["muted"])
                            .accessibilityIdentifier("callsheet-no-match")
                    }
                }
                .listStyle(.plain)
                .onKeyPress(.return) {
                    if let id = model.selection { hub.open(id); return .handled }
                    return .ignored
                }
            }
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
        }
    }

    private func row(_ r: CallSheetListRow) -> some View {
        HStack {
            HStack(spacing: 4) {
                Text(r.dayLabel).fontWeight(.semibold)
                if !r.dayExists { Text("(day deleted)").font(.caption).foregroundStyle(theme["muted"]) }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Group {
                if let d = r.dateLabel { Text(d) } else { Text("No date").foregroundStyle(theme["muted"]) }
            }
            .frame(width: 150, alignment: .leading)
            CSChip(text: CSStatus.word(r.status), tone: CSStatus.tone(r.status))
                .frame(width: 120, alignment: .leading)
            HStack(spacing: 4) {
                Text("v\(Int(r.revision))")
                if r.replaced { Text("(replaced by a newer revision)").font(.caption).foregroundStyle(theme["muted"]) }
            }
            .frame(width: 200, alignment: .leading)
            HStack(spacing: 6) {
                Button("Open") { hub.open(r.id) }
                    .accessibilityIdentifier("callsheet-open-\(r.id)")
                Button("Export") { hub.exportCallSheet(id: r.id, title: r.title, status: r.status) }
                    .accessibilityIdentifier("callsheet-export-\(r.id)")
                Menu {
                    menuItems(r)
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .accessibilityLabel("\(r.title) actions")
                .accessibilityIdentifier("callsheet-actions-\(r.id)")
            }
            .controlSize(.small)
            .frame(width: 190, alignment: .trailing)
        }
        .contentShape(Rectangle())
        .onTapGesture(count: 2) { hub.open(r.id) }
        .contextMenu { menuItems(r) }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("callsheet-row-\(r.id)")
    }

    @ViewBuilder private func menuItems(_ r: CallSheetListRow) -> some View {
        Button("Open") { hub.open(r.id) }
            .accessibilityIdentifier("callsheet-menu-open")
        Button("Refresh") { hub.open(r.id, action: "refresh") }
            .disabled(!r.sourceChanged || r.replaced || !r.dayExists)
            .accessibilityIdentifier("callsheet-menu-refresh")
        Button("Finalize") { hub.open(r.id, action: "finalize") }
            .disabled(CSStatus.word(r.status) == "Final")
            .accessibilityIdentifier("callsheet-menu-finalize")
        Button("Export PDF\u{2026}") { hub.exportCallSheet(id: r.id, title: r.title, status: r.status) }
            .accessibilityIdentifier("callsheet-menu-export")
        Divider()
        Button("Delete", role: .destructive) { Task { await model.delete(r) } }
            .disabled(!hub.canEdit)
            .accessibilityIdentifier("callsheet-menu-delete")
    }
}

extension CreateCallSheetModel: Identifiable {
    nonisolated var id: ObjectIdentifier { ObjectIdentifier(self) }
}

struct CSCreateSheet: View {
    @Bindable var model: CreateCallSheetModel
    var onClose: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        CSSheetFrame(title: "Create Call Sheet") {
            VStack(alignment: .leading, spacing: 10) {
                if !model.hasSchedule {
                    CSBanner(tone: .info, text: "Create a shooting schedule and a shooting day first.") {
                        Button("Open Schedule") { onClose(); csShowWorkspace(.schedule) }
                            .controlSize(.small)
                            .accessibilityIdentifier("callsheet-create-open-schedule")
                    }
                } else if model.candidates.isEmpty {
                    CSBanner(.info, "Every shooting day already has a call sheet. Open it from the list to update it.")
                } else {
                    Picker("Source shooting day", selection: $model.dayId) {
                        ForEach(model.candidates, id: \.id) { Text($0.title).tag($0.id) }
                    }
                    .accessibilityIdentifier("callsheet-create-day")
                    if !model.templates.isEmpty {
                        Picker("Template", selection: $model.templateKey) {
                            Text("None").tag("none")
                            ForEach(model.templates, id: \.id) { Text($0.name).tag(CreateCallSheetModel.key($0)) }
                        }
                        .accessibilityIdentifier("callsheet-create-template")
                    }
                    Text("Will be filled in automatically").font(.headline)
                    VStack(alignment: .leading, spacing: 3) {
                        ForEach([
                            "Title, date, day number and day times from the schedule",
                            "Scene schedule with start times, meals and travel rows",
                            "Talent with cast ID numbers",
                            "Locations with address, parking and access",
                            "Extras, department notes, the advance schedule and crew, when the project has them",
                        ], id: \.self) { Text("\u{2022} \($0)").font(.callout) }
                    }
                    Text("You add call times, weather and day-specific notes. Anything nobody has entered reads Needs input.")
                        .font(.footnote).foregroundStyle(theme["muted"])
                }
            }
        } footer: {
            Button("Cancel", action: onClose)
                .keyboardShortcut(.cancelAction)
                .accessibilityIdentifier("callsheet-create-cancel")
            Button("Create Call Sheet") {
                Task {
                    if await model.confirm() { onClose() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(!model.canCreate)
            .accessibilityIdentifier("callsheet-create-confirm")
        }
        .task { await model.loadTemplates() }
        .accessibilityIdentifier("callsheet-create-sheet")
    }
}

// The Reports tab (Windows production/tabs/reports.tsx) and the Daily View tab (daily.tsx).

import SwiftUI
import OpenFrameCore

// MARK: Reports

struct CSReportsView: View {
    @Bindable var model: CSReportsModel
    @Environment(AppModel.self) private var app
    @Environment(CallSheetsHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollingPage {
        VStack(spacing: PageSpacing.medium) {
            CSPageHeader(title: "Reports", sub: "What concise production information do I need to see or send?") {
                HStack {
                    if model.savedId == nil {
                        Button { Task { await model.saveSnapshot() } } label: { Label("Save snapshot", systemImage: "square.and.arrow.down.on.square") }
                            .disabled(model.live == nil || !hub.canEdit)
                            .accessibilityIdentifier("reports-save")
                    }
                    Button { model.exportReport() } label: { Label("Export", systemImage: "square.and.arrow.down") }
                        .disabled(!model.canExport)
                        .accessibilityIdentifier("reports-export")
                }
            }
            ViewThatFits(in: .horizontal) {
                HStack(alignment: .top, spacing: PageSpacing.medium) {
                    sidebar.frame(width: 230)
                    main.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                }
                VStack(spacing: PageSpacing.medium) {
                    sidebar.frame(maxWidth: .infinity)
                    main.frame(maxWidth: .infinity, alignment: .topLeading)
                }
            }
        }
        .pageColumn(maxWidth: PageSpacing.wideColumn)
        .padding(.bottom, PageSpacing.medium)
        }
        .task(id: "\(model.key)|\(app.changes.key(CSTables.reports))") { await model.reload() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("reports-tab")
    }

    private var sidebar: some View {
        Group {
            VStack(alignment: .leading, spacing: 2) {
                ForEach(CSReportKind.all) { r in
                    let on = model.savedId == nil && model.type == r.value
                    Button { model.choose(r.value) } label: {
                        Text(r.label)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal, 8).padding(.vertical, 5)
                            .background(on ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(on ? .isSelected : [])
                    .accessibilityIdentifier("reports-type-\(csSlug(r.value))")
                }
                if !model.list.isEmpty {
                    Text("Saved snapshots").font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
                        .padding(.top, 10).padding(.horizontal, 8)
                    ForEach(model.list, id: \.id) { s in
                        HStack(spacing: 2) {
                            Button { model.openSnapshot(s.id) } label: {
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(s.title)
                                    Text(CSFormat.dateTime(s.generatedAt)).font(.caption).foregroundStyle(theme["muted"])
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, 8).padding(.vertical, 4)
                                .background(model.savedId == s.id ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .accessibilityIdentifier("reports-saved-\(s.id)")
                            Button { Task { await model.deleteSnapshot(s.id) } } label: { Image(systemName: "trash") }
                                .buttonStyle(.borderless)
                                .accessibilityLabel("Delete \(s.title)")
                                .accessibilityIdentifier("reports-delete-\(s.id)")
                        }
                    }
                }
            }
            .padding(8)
        }
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Reports")
        .accessibilityIdentifier("reports-list")
    }

    @ViewBuilder private var main: some View {
        if model.savedId != nil {
            if let s = model.saved {
                Group {
                    VStack(alignment: .leading, spacing: 8) {
                        CSBanner(
                            tone: .info,
                            text: "Saved snapshot \u{201C}\(s.title)\u{201D}\(s.data.filter.map { " (filtered by \u{201C}\($0)\u{201D})" } ?? ""). It does not change when production data changes."
                        ) {
                            Button("Back to live reports") { model.backToLive() }
                                .controlSize(.small)
                                .accessibilityIdentifier("reports-back-to-live")
                        }
                        Text(s.data.title).font(.headline)
                        CSReportTable(data: s.data)
                    }
                }
            } else if let f = model.savedFailure {
                CSBanner(.err, f.message)
            } else {
                ProgressView()
            }
        } else {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 6) {
                    Image(systemName: "magnifyingglass").foregroundStyle(theme["muted"]).accessibilityHidden(true)
                    TextField("Filter rows (scene, location, name\u{2026})", text: $model.filter)
                        .textFieldStyle(.roundedBorder)
                        .frame(maxWidth: 360)
                        .accessibilityLabel("Filter report rows")
                        .accessibilityIdentifier("reports-filter")
                    if !model.filter.isEmpty {
                        Button("Clear") { model.filter = "" }
                            .controlSize(.small)
                            .accessibilityIdentifier("reports-filter-clear")
                    }
                }
                if let d = model.live {
                    Group {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(d.title).font(.headline)
                            CSReportTable(data: d)
                        }
                    }
                } else if let f = model.liveFailure {
                    CSBanner(.err, f.message).accessibilityIdentifier("reports-error")
                } else {
                    ProgressView()
                }
            }
        }
    }
}

struct CSReportTable: View {
    let data: ReportData
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text((data.sourceLabel.isEmpty ? "" : "Source: \(data.sourceLabel) \u{00B7} ") + "Generated \(CSFormat.dateTime(data.generatedAt))")
                .font(.callout).foregroundStyle(theme["muted"])
                .accessibilityIdentifier("reports-generated")
            if !data.columns.isEmpty {
                ScrollView(.horizontal) {
                Grid(alignment: .topLeading, horizontalSpacing: 14, verticalSpacing: 5) {
                    GridRow {
                        ForEach(Array(data.columns.enumerated()), id: \.offset) { _, c in Text(c).fontWeight(.semibold) }
                    }
                    Divider().gridCellUnsizedAxes(.horizontal)
                    ForEach(Array(data.rows.enumerated()), id: \.offset) { i, r in
                        GridRow {
                            ForEach(Array(r.enumerated()), id: \.offset) { _, cell in Text(cell).textSelection(.enabled) }
                        }
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("reports-row-\(i)")
                    }
                }
                .font(.callout)
                .frame(minWidth: 720, alignment: .leading)
                .padding(10)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
                .accessibilityIdentifier("reports-table")
            }
            if let n = data.note {
                Text(n).foregroundStyle(theme["muted"]).padding(20).frame(maxWidth: .infinity)
                    .accessibilityIdentifier("reports-note")
            }
        }
    }
}

// MARK: Daily View

struct CSDailyView: View {
    @Bindable var model: CSDailyModel
    @Environment(AppModel.self) private var app
    @Environment(CallSheetsHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let v = model.view, v.hasSchedule, let d = v.day {
                ScrollView { loaded(v, d).pageColumn(maxWidth: PageSpacing.wideColumn).padding(.bottom, PageSpacing.medium) }
            } else if let f = model.failure {
                CSBanner(.err, f.message).padding(16)
            } else {
                ContentUnavailableView {
                    Label("Select or create a shooting day.", systemImage: "calendar")
                } description: {
                    Text("The daily view shows everything you need to know for one shooting day, derived from the schedule.")
                } actions: {
                    Button("Open Schedule") { csShowWorkspace(.schedule) }
                        .accessibilityIdentifier("daily-open-schedule")
                }
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("daily-empty")
            }
        }
        .task(id: "\(app.changes.key(CSTables.schedule))|\(model.dayId ?? "")") { await model.reload() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("daily-tab")
    }

    private func loaded(_ v: ScheduleDailyView, _ d: ScheduleDailyDay) -> some View {
        let s = d.summary
        let calls = d.cast.compactMap(\.callTime)
        return VStack(spacing: PageSpacing.medium) {
            CSPageHeader(title: d.isToday ? "Today: \(d.label)" : d.label, sub: "\(d.dateLabel ?? "No date yet") \u{00B7} What do I need to know for this day?") {
                HStack {
                    if d.isOffDay { CSChip(text: "Off day") }
                    Picker("Shooting day", selection: Binding(get: { d.id }, set: { model.pick($0) })) {
                        ForEach(v.days, id: \.id) { Text($0.title).tag($0.id) }
                    }
                    .labelsHidden().fixedSize()
                    .accessibilityLabel("Shooting day")
                    .accessibilityIdentifier("daily-day")
                }
            }
            if !model.openIssues.isEmpty {
                let issues = model.openIssues
                CSBanner(
                    tone: .warn,
                    text: issues.count == 1 ? issues[0].message : "\(issues.count) things to check on this day: \(issues.map(\.message).joined(separator: " "))"
                ) {
                    Button("Open day") { csShowWorkspace(.schedule) }
                        .controlSize(.small)
                        .accessibilityIdentifier("daily-open-day")
                }
                .accessibilityIdentifier("daily-issues")
            }
            VStack(alignment: .leading, spacing: 12) {
                HStack(alignment: .top, spacing: 12) {
                    card("Scenes", id: "scenes") {
                        if d.scenes.isEmpty {
                            Text("No scenes scheduled.").foregroundStyle(theme["muted"])
                        } else {
                            HStack {
                                ForEach(d.scenes, id: \.id) { sc in
                                    Button(CSFormat.number(sc.number)) { csShowWorkspace(.breakdown) }
                                        .controlSize(.small)
                                        .help(sc.heading)
                                        .accessibilityIdentifier("daily-scene-\(sc.id)")
                                }
                            }
                            Text("\(Int(s.sceneCount)) scene\(s.sceneCount == 1 ? "" : "s")"
                                + (s.estimatedMinutes > 0 ? " \u{00B7} \(CSFormat.minutes(s.estimatedMinutes)) estimated" : "")
                                + (s.missingEstimates > 0 ? " \u{00B7} \(Int(s.missingEstimates)) without estimate" : ""))
                                .font(.caption).foregroundStyle(theme["muted"])
                        }
                    }
                    card("Location", id: "location") {
                        if d.locations.isEmpty {
                            Text("Missing").foregroundStyle(theme["red-t"])
                        } else {
                            ForEach(d.locations, id: \.key) { l in
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(l.name).font(.system(size: 15, weight: .bold))
                                    Text([l.status, l.address ?? "Address missing"].compactMap { $0 }.joined(separator: " \u{00B7} "))
                                        .font(.caption).foregroundStyle(theme["muted"])
                                }
                            }
                        }
                    }
                    card("Cast", id: "cast") {
                        if d.cast.isEmpty {
                            Text("No cast confirmed for these scenes.").foregroundStyle(theme["muted"])
                        } else {
                            Text(d.cast.map { $0.actor ?? $0.character }.joined(separator: " \u{00B7} "))
                                .font(.system(size: 15, weight: .bold))
                            Text((calls.isEmpty ? "Call times not entered yet" : "Calls \(calls.joined(separator: " / "))")
                                + (d.crewCall.map { " \u{00B7} Crew call \($0)" } ?? ""))
                                .font(.caption).foregroundStyle(theme["muted"])
                        }
                    }
                }
                HStack(alignment: .top, spacing: 12) {
                    callSheetCard(d)
                    card("Important notes", id: "notes") {
                        if let n = d.notes, !n.isEmpty { Text(n) } else { Text("No day notes.").foregroundStyle(theme["muted"]) }
                        if !d.markers.isEmpty {
                            HStack { ForEach(d.markers, id: \.id) { CSChip(text: CSFormat.marker($0)) } }
                        }
                    }
                }
                HStack(alignment: .top, spacing: 12) {
                    card("Important breakdown items", id: "items") {
                        if d.breakdownItems.isEmpty {
                            Text("No confirmed props, wardrobe or other items for these scenes.").foregroundStyle(theme["muted"])
                        } else {
                            HStack { ForEach(d.breakdownItems, id: \.name) { CSChip(text: $0.name) } }
                        }
                    }
                    card("Scene order", id: "order") {
                        if d.scenes.isEmpty {
                            Text("Nothing scheduled yet.").foregroundStyle(theme["muted"])
                        } else {
                            ForEach(Array(d.scenes.enumerated()), id: \.offset) { i, sc in
                                Text("\(i + 1). \(CSFormat.number(sc.number)): \(sc.heading)"
                                    + (sc.estimatedMinutes.map { " (\(CSFormat.minutes($0)))" } ?? ""))
                                    .font(.callout)
                            }
                        }
                    }
                }
            }
        }
    }

    private func callSheetCard(_ d: ScheduleDailyDay) -> some View {
        card("Call sheet", id: "call-sheet") {
            if let b = d.callSheet {
                CSChip(text: CSStatus.word(b.status), tone: CSStatus.tone(b.status))
                Text(model.callSheetNote(b)).font(.callout)
                Button("Open Call Sheet") { Task { await model.openCallSheet() } }
                    .buttonStyle(.borderedProminent).controlSize(.small)
                    .accessibilityIdentifier("daily-open-call-sheet")
            } else if d.isOffDay {
                Text("Off days have no call sheet.").foregroundStyle(theme["muted"])
            } else {
                Text("No call sheet yet for this day.").font(.callout)
                Button("Create Call Sheet") { Task { await model.openCallSheet() } }
                    .buttonStyle(.borderedProminent).controlSize(.small)
                    .disabled(!hub.canEdit)
                    .accessibilityIdentifier("daily-create-call-sheet")
            }
        }
    }

    private func card<C: View>(_ title: String, id: String, @ViewBuilder content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.headline)
            content()
        }
        .padding(PageSpacing.medium)
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier("daily-card-\(id)")
    }
}

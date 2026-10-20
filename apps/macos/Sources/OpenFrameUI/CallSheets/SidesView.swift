// The Sides tab (Windows production/tabs/sides.tsx): setup on the left, the sides on a paper
// preview on the right, saved snapshots under the setup.

import SwiftUI
import OpenFrameCore

struct CSSidesView: View {
    @Bindable var model: CSSidesModel
    @Environment(AppModel.self) private var app
    @Environment(CallSheetsHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Group {
            if !model.loaded {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if !model.hasSchedule {
                ContentUnavailableView {
                    Label("Sides need a shooting schedule.", systemImage: "doc.text")
                } description: {
                    Text("Create a shooting schedule and a shooting day first. Sides contain that day's screenplay scenes.")
                } actions: {
                    Button("Open Schedule") { ProjectNav.show(.production, tab: .schedule) }
                        .accessibilityIdentifier("sides-open-schedule")
                }
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("sides-empty")
            } else {
                ScrollingPage {
                VStack(spacing: PageSpacing.medium) {
                    CSPageHeader(title: "Sides", sub: "What script pages does this team need for this shooting day?") { EmptyView() }
                    HStack(alignment: .top, spacing: PageSpacing.medium) {
                        setup.frame(width: 320)
                        preview.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                    }
                }
                .pageColumn(maxWidth: PageSpacing.wideColumn)
                .padding(.bottom, PageSpacing.medium)
                }
            }
        }
        .task(id: app.changes.key(CSTables.sides)) { await model.reload() }
        .task(id: "\(model.previewKey)|\(app.changes.key(CSTables.sides))") { await model.loadPreview() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("sides-tab")
    }

    // MARK: Setup

    private var setup: some View {
        Group {
            VStack(alignment: .leading, spacing: 8) {
                Text("Setup").font(.headline)
                if model.days.isEmpty {
                    Text("Create a shooting day first.").font(.callout).foregroundStyle(theme["muted"])
                } else {
                    Picker("Shooting day", selection: Binding(get: { model.dayId }, set: { model.pickDay($0) })) {
                        ForEach(model.days, id: \.id) { Text($0.title).tag($0.id) }
                    }
                    .accessibilityIdentifier("sides-day")
                    Text("Scenes").fontWeight(.semibold)
                    if model.strips.isEmpty { Text("No scenes scheduled on this day.").font(.callout).foregroundStyle(theme["muted"]) }
                    ForEach(model.strips, id: \.id) { s in
                        Toggle("\(CSFormat.number(s.number)): \(s.heading)", isOn: Binding(
                            get: { model.chosen.contains(s.id) },
                            set: { model.setScene(s.id, on: $0) }))
                            .accessibilityIdentifier("sides-scene-\(s.id)")
                    }
                    Text("For").fontWeight(.semibold).padding(.top, 6)
                    Picker("Sides for", selection: $model.forMode) {
                        ForEach(CSSidesFor.allCases) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.segmented).labelsHidden()
                    .accessibilityLabel("Sides for")
                    .accessibilityIdentifier("sides-for")
                    if model.forMode == .selected {
                        if model.dayCast.isEmpty { Text("No confirmed cast on this day.").font(.callout).foregroundStyle(theme["muted"]) }
                        ForEach(model.dayCast, id: \.key) { c in
                            Toggle(c.actor.map { "\($0) (\(c.character))" } ?? c.character, isOn: Binding(
                                get: { model.castKeys.contains(c.key) },
                                set: { model.setCast(c.key, on: $0) }))
                                .accessibilityIdentifier("sides-cast-\(csSlug(c.key))")
                        }
                    }
                    Divider()
                    Toggle("Include call-sheet cover page", isOn: $model.cover)
                        .accessibilityIdentifier("sides-cover")
                    Toggle("Show draft / revision name", isOn: $model.draftName)
                        .accessibilityIdentifier("sides-draft-name")
                    HStack {
                        Button("Preview") {
                            if model.openId != nil { model.backToLive() } else { Task { await model.loadPreview() } }
                        }
                        .disabled(!model.ready)
                        .accessibilityIdentifier("sides-preview")
                        Button("Save Sides") { Task { await model.save() } }
                            .buttonStyle(.borderedProminent)
                            .disabled(!model.ready || !hub.canEdit)
                            .accessibilityIdentifier("sides-save")
                        Button { model.exportPDF() } label: { Label("Export PDF", systemImage: "square.and.arrow.down") }
                            .disabled(!model.ready)
                            .accessibilityIdentifier("sides-export")
                    }
                    Text("Saved sides are snapshots. Later script edits don't change them.")
                        .font(.footnote).foregroundStyle(theme["muted"])
                }
                if !model.saved.isEmpty {
                    Divider()
                    Text("Saved sides").font(.headline)
                    ForEach(model.saved, id: \.id) { s in
                        HStack {
                            Button { model.openSnapshot(s.id) } label: {
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(s.title)
                                    Text("\(Int(s.sceneCount)) scenes \u{00B7} \(CSFormat.dateTime(s.createdAt))")
                                        .font(.caption).foregroundStyle(theme["muted"])
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .accessibilityIdentifier("sides-saved-\(s.id)")
                            Button { Task { await model.deleteSaved(s.id) } } label: { Label("Delete", systemImage: "trash") }
                                .buttonStyle(.borderless)
                                .accessibilityLabel("Delete \(s.title)")
                                .accessibilityIdentifier("sides-delete-\(s.id)")
                        }
                        .padding(.vertical, 3).padding(.horizontal, 4)
                        .background(s.id == model.openId ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
            .padding(PageSpacing.medium)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(theme["line"]))
    }

    // MARK: Preview

    @ViewBuilder private var preview: some View {
        if model.openId != nil {
            if let s = model.openSaved {
                Group {
                    VStack(alignment: .leading, spacing: 8) {
                        CSBanner(tone: .info, text: "Saved snapshot \u{201C}\(s.title)\u{201D} \u{00B7} \(CSFormat.dateTime(s.createdAt))") {
                            HStack {
                                Button("Export PDF") { model.exportPDF() }
                                    .accessibilityIdentifier("sides-snapshot-export")
                                Button("Back to live preview") { model.backToLive() }
                                    .accessibilityIdentifier("sides-back-to-live")
                            }
                            .controlSize(.small)
                        }
                        CSSidesPaper(content: s.content).frame(maxWidth: .infinity)
                    }
                }
            } else if let f = model.openFailure {
                CSBanner(.err, f.message)
            } else {
                ProgressView()
            }
        } else if !model.ready {
            Text("Choose a shooting day and at least one scene\(model.forMode == .selected ? " and cast member" : "") to preview the sides.")
                .foregroundStyle(theme["muted"]).padding(PageSpacing.large)
                .multilineTextAlignment(.center).frame(maxWidth: .infinity)
                .accessibilityIdentifier("sides-choose-hint")
        } else if let f = model.previewFailure {
            CSBanner(.warn, f.message).accessibilityIdentifier("sides-preview-error")
        } else if let c = model.preview {
            CSSidesPaper(content: c).frame(maxWidth: .infinity).padding(.bottom, PageSpacing.medium)
        } else {
            ProgressView()
        }
    }
}

/// The sides on a page: scene headings, action and dialogue in screenplay layout.
struct CSSidesPaper: View {
    let content: ScheduleSidesContent

    private func style(_ type: String) -> (indent: CGFloat, width: CGFloat?, bold: Bool, upper: Bool, trailing: Bool) {
        switch type {
        case "character": (180, nil, false, true, false)
        case "dialogue": (110, 300, false, false, false)
        case "parenthetical": (150, 220, false, false, false)
        case "transition": (0, nil, false, true, true)
        case "shot": (0, nil, true, true, false)
        default: (0, nil, false, false, false)
        }
    }

    private var coverLine: String {
        var s = content.dayLabel ?? ""
        if let d = content.dateLabel { s += s.isEmpty ? d : " \u{00B7} \(d)" }
        if let c = content.cover?.crewCall, !c.isEmpty { s += " \u{00B7} CREW CALL \(c)" }
        return s
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(content.header).fontWeight(.bold).frame(maxWidth: .infinity)
            if let cover = content.cover {
                VStack(alignment: .leading, spacing: 3) {
                    Text("CALL SHEET COVER").font(.system(size: 11, weight: .bold))
                    Text(coverLine)
                    ForEach(cover.locations, id: \.self) { Text($0) }
                    ForEach(cover.cast, id: \.character) { c in
                        HStack {
                            Text(c.actor ?? csNeedsInput).frame(width: 160, alignment: .leading)
                            Text(c.character).frame(width: 160, alignment: .leading)
                            Text(c.callTime ?? "")
                        }
                    }
                    if let n = cover.notes { Text(n) }
                    Divider()
                }
                .accessibilityIdentifier("sides-cover-page")
            }
            ForEach(content.scenes, id: \.sceneId) { s in
                VStack(alignment: .leading, spacing: 6) {
                    Text("\(s.number). \(s.heading)").fontWeight(.bold)
                    ForEach(Array(s.elements.filter { $0.elementType != "scene_heading" }.enumerated()), id: \.offset) { _, e in
                        let st = style(e.elementType)
                        Text(st.upper ? e.text.uppercased() : e.text)
                            .fontWeight(st.bold ? .bold : .regular)
                            .frame(maxWidth: st.width ?? .infinity, alignment: st.trailing ? .trailing : .leading)
                            .padding(.leading, st.indent)
                            .frame(maxWidth: .infinity, alignment: st.trailing ? .trailing : .leading)
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Scene \(s.number)")
            }
            if content.showDraftName {
                Text("Source: \(content.sourceLabel)").font(.caption).frame(maxWidth: .infinity)
                    .accessibilityIdentifier("sides-source-label")
            }
        }
        .font(.system(size: 12, design: .monospaced))
        .foregroundStyle(Color.black)
        .padding(28)
        .frame(maxWidth: 720, alignment: .leading)
        .background(Color.white)
        .environment(\.colorScheme, .light)
        .shadow(color: .black.opacity(0.12), radius: 4, y: 1)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Sides preview")
        .accessibilityIdentifier("sides-paper")
    }
}

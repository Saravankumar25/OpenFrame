// The Review Queue sheet and the Import and Export History sheet.

import SwiftUI
import OpenFrameCore

struct ReviewQueueSheet: View {
    @State var model: ReviewQueueModel
    let close: () -> Void

    var body: some View {
        IXSheetFrame(
            title: "Review Queue", sub: "Imported comments that need a decision. No comment is ever silently discarded.",
            width: 700, idPrefix: "review-queue-sheet"
        ) {
            ScrollView {
                VStack(alignment: .leading, spacing: 8) {
                    if !model.loaded {
                        ProgressView().frame(maxWidth: .infinity, minHeight: 100)
                    } else if model.items.isEmpty {
                        VStack(spacing: 2) {
                            Text("Nothing waiting").font(.headline)
                            Text("When an imported comment can't be matched safely, it waits here for you.").foregroundStyle(.secondary)
                        }
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: .infinity)
                        .accessibilityIdentifier("review-queue-empty")
                    } else {
                        Text("Ambiguous mapping (\(model.ambiguous.count))").font(.callout.weight(.semibold))
                        ForEach(model.ambiguous, id: \.id) { QueueRow(model: model, item: $0) }
                        Text("Unmatched Review Notes (\(model.unmatched.count))").font(.callout.weight(.semibold)).padding(.top, 6)
                        ForEach(model.unmatched, id: \.id) { QueueRow(model: model, item: $0) }
                        if !model.attached.isEmpty {
                            Text("Attached (\(model.attached.count))").font(.callout.weight(.semibold)).padding(.top, 6)
                            ForEach(model.attached, id: \.id) { QueueRow(model: model, item: $0) }
                        }
                    }
                    if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("review-queue-error") }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minHeight: 200, maxHeight: 420)
        } footer: {
            Button("Close", action: close).keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                .accessibilityIdentifier("review-queue-close")
        }
        .task { await model.load() }
    }
}

private struct QueueRow: View {
    let model: ReviewQueueModel
    let item: PackageQueueItem
    @State private var choosing = false
    @State private var target = ""
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let attached = item.status == "Attached"
        VStack(alignment: .leading, spacing: 4) {
            (Text("\u{201C}\(item.body)\u{201D}").bold() + Text(" from \(item.authorName)").foregroundStyle(.secondary)).font(.callout)
            if let q = item.quotedText { Text("on \u{201C}\(q)\u{201D}").font(.caption).italic().foregroundStyle(.secondary) }
            Text([item.sourceLabel, item.reason].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{00B7} "))
                .font(.caption).foregroundStyle(.secondary)
            if attached {
                HStack { IXChip(text: "Attached", tone: .green); Text(item.attachedLabel ?? "").font(.caption) }
            } else {
                HStack {
                    ForEach(item.candidates, id: \.sceneId) { c in
                        Button("Attach to \(c.label.components(separatedBy: " \u{00B7} ").first ?? c.label)") {
                            Task { await model.attach(item, to: c.sceneId) }
                        }
                        .controlSize(.small).disabled(model.busyItem == item.id)
                        .accessibilityIdentifier("queue-attach-\(item.id)-\(c.sceneId)")
                    }
                    if !choosing {
                        Button(item.kind == "ambiguous" ? "Choose\u{2026}" : "Attach manually\u{2026}") {
                            target = model.scenes.first?.id ?? ""
                            choosing = true
                        }
                        .controlSize(.small).disabled(model.busyItem == item.id || model.scenes.isEmpty)
                        .accessibilityIdentifier("queue-choose-\(item.id)")
                    } else {
                        Picker("Scene", selection: $target) {
                            ForEach(model.scenes, id: \.id) { Text("\($0.label) \u{00B7} \($0.heading)").tag($0.id) }
                        }
                        .labelsHidden().fixedSize().accessibilityLabel("Scene")
                        .accessibilityIdentifier("queue-scene-\(item.id)")
                        Button("Attach") { Task { await model.attach(item, to: target) } }
                            .controlSize(.small).buttonStyle(.borderedProminent).disabled(target.isEmpty || model.busyItem == item.id)
                            .accessibilityIdentifier("queue-attach-chosen-\(item.id)")
                        Button("Cancel") { choosing = false }.controlSize(.small).buttonStyle(.borderless)
                    }
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .opacity(attached ? 0.65 : 1)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("queue-item-\(item.id)")
    }
}

struct PackageHistorySheet: View {
    @State var model: PackageHistoryModel
    var openReview: (String) -> Void = { _ in }
    var openRecord: (String) -> Void = { _ in }
    var openSession: (String) -> Void = { _ in }
    let close: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        IXSheetFrame(
            title: "Import & Export History", sub: "Every package this project exported or imported, and what happened.",
            width: 760, idPrefix: "package-history-sheet"
        ) {
            VStack(alignment: .leading, spacing: 10) {
                Picker("History", selection: $model.tab) {
                    ForEach(PKHistoryTab.allCases, id: \.self) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize()
                .frame(maxWidth: .infinity, alignment: .center)
                .accessibilityLabel("History").accessibilityIdentifier("history-tabs")
                ScrollView {
                    VStack(alignment: .leading, spacing: 6) {
                        switch model.tab {
                        case .imports: imports
                        case .records: records
                        case .reviews: reviews
                        case .log: logView
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(minHeight: 220, maxHeight: 380)
                if model.tab == .imports {
                    IXBanner(.info, "Undo removes everything one import added, as long as it wasn't changed afterwards.")
                }
                if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("history-error") }
            }
        } footer: {
            Button("Close", action: close).keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
                .accessibilityIdentifier("history-close")
        }
        .background {
            // Escape closes the sheet, as on every Mac sheet; Return is already the Close button.
            Button("Close", action: close).keyboardShortcut(.cancelAction).opacity(0).accessibilityHidden(true)
        }
        .task(id: model.tab) { await model.load() }
        .confirmationDialog("Delete this review record?", isPresented: Binding(get: { model.deleting != nil }, set: { if !$0 { model.deleting = nil } }),
                            titleVisibility: .visible) {
            Button("Delete", role: .destructive) { Task { await model.deleteRecord() } }
                .accessibilityIdentifier("history-record-delete-confirm")
            Button("Cancel", role: .cancel) { model.deleting = nil }
        } message: {
            Text("It moves to Recently Deleted and can be restored from there.")
        }
    }

    @ViewBuilder private var imports: some View {
        if model.sessions.isEmpty {
            empty("No imports yet", id: "history-empty-imports")
        } else {
            ForEach(model.sessions, id: \.id) { s in
                row {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(s.fileName).fontWeight(.semibold)
                        Text("\(s.typeLabel) \u{00B7} from \u{201C}\(s.sourceProjectTitle)\u{201D}").font(.caption).foregroundStyle(.secondary)
                        if let m = s.message { Text(m).font(.caption) }
                    }
                    Spacer()
                    IXChip(text: PKText.resultLabel(s.status), tone: PackageHistoryModel.resultTone(s.status))
                    Text(PKText.day(ms: s.createdAt)).font(.caption).foregroundStyle(.secondary).frame(width: 130, alignment: .trailing)
                    if s.canUndo {
                        Button("Undo") { Task { await model.undo(s) } }.controlSize(.small)
                            .accessibilityIdentifier("history-undo-\(s.id)")
                    }
                    if s.status == "Previewed" {
                        Button("Review") { openSession(s.id) }.controlSize(.small)
                            .accessibilityIdentifier("history-review-\(s.id)")
                    }
                }
                .accessibilityIdentifier("history-import-\(s.id)")
            }
        }
    }

    @ViewBuilder private var records: some View {
        if model.records.isEmpty {
            empty("No review records", detail: "A stale package can be kept as a separate review record instead of being imported.", id: "history-empty-records")
        } else {
            ForEach(model.records, id: \.id) { r in
                row {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(r.title).fontWeight(.semibold)
                        Text("from \u{201C}\(r.sourceProjectTitle)\u{201D} \u{00B7} \(Int(r.commentCount)) comments \u{00B7} kept \(PKText.day(ms: r.createdAt))")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    Button("View") { openRecord(r.id) }.controlSize(.small).accessibilityIdentifier("history-record-view-\(r.id)")
                    Button("Delete") { model.deleting = r }.controlSize(.small).buttonStyle(.borderless)
                        .accessibilityIdentifier("history-record-delete-\(r.id)")
                }
                .accessibilityIdentifier("history-record-\(r.id)")
            }
        }
    }

    @ViewBuilder private var reviews: some View {
        if model.reviews.isEmpty {
            empty("No open reviews", detail: "Review packages you open with Open Review Package\u{2026} are listed here.", id: "history-empty-reviews")
        } else {
            ForEach(model.reviews, id: \.packageId) { r in
                row {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(r.projectTitle): \(r.draftName)").fontWeight(.semibold)
                        Text("from \(r.exportedBy) \u{00B7} \(Int(r.myCommentCount)) of your comments"
                             + (r.responseExportedAt.map { " \u{00B7} response exported \(PKText.day(ms: $0))" } ?? ""))
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    Button("Open") { openReview(r.packageId) }.controlSize(.small).accessibilityIdentifier("history-review-open-\(r.packageId)")
                }
                .accessibilityIdentifier("history-review-\(r.packageId)")
            }
        }
    }

    @ViewBuilder private var logView: some View {
        if model.log.isEmpty {
            empty("No packages yet", id: "history-empty-log")
        } else {
            ForEach(model.log, id: \.id) { l in
                row {
                    IXChip(text: l.direction == "export" ? "Export" : "Import", tone: l.direction == "export" ? .blue : .purple)
                        .frame(width: 70, alignment: .leading)
                    Text(l.summary).font(.callout)
                    Spacer()
                    Text(PKText.day(ms: l.at) + (l.actorName.map { " \u{00B7} \($0)" } ?? "")).font(.caption).foregroundStyle(.secondary)
                }
                .accessibilityIdentifier("history-log-\(l.id)")
            }
        }
    }

    private func empty(_ title: String, detail: String? = nil, id: String) -> some View {
        VStack(spacing: 2) {
            Text(title).font(.headline)
            if let detail { Text(detail).foregroundStyle(.secondary) }
        }
        .multilineTextAlignment(.center)
        .frame(maxWidth: .infinity)
        .padding(PageSpacing.small)
        .accessibilityIdentifier(id)
    }

    private func row<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        HStack(alignment: .center, spacing: 8) { c() }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
            .accessibilityElement(children: .contain)
    }
}

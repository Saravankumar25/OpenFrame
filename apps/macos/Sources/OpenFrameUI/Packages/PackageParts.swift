// Small blocks shared by the package sheets: the task progress bar with its Cancel, the file status
// table, the linked-file list and the "done" banner with Show in Folder.

import SwiftUI
import OpenFrameCore

struct PKProgress: View {
    let task: PackageTaskFollower
    let cancel: () -> Void
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(task.message).font(.callout.weight(.semibold))
                Spacer()
                Text("\(task.percent)%").font(.caption).foregroundStyle(.secondary)
            }
            ProgressView(value: Double(task.percent), total: 100)
                .accessibilityLabel("Progress")
                .accessibilityValue("\(task.percent) percent")
            HStack {
                Text("Your project stays open and saved while this runs.").font(.caption).foregroundStyle(.secondary)
                Spacer()
                Button("Cancel", action: cancel).controlSize(.small).accessibilityIdentifier("package-task-cancel")
            }
        }
        .padding(12)
        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Package progress")
        .accessibilityIdentifier("package-task-progress")
    }
}

enum PKFileStatus {
    static func label(_ s: String) -> String {
        switch s {
        case "Included": "Included"
        case "Copied": "Included (copied)"
        case "External": "External, not copied"
        case "Missing": "Missing"
        default: s
        }
    }
    static func tone(_ s: String) -> IXTone {
        switch s {
        case "Included", "Copied": .green
        case "External": .yellow
        case "Missing": .red
        default: .plain
        }
    }
}

struct PKFileStatusTable: View {
    let rows: [(label: String, status: String, count: Int)]
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { i, r in
                HStack {
                    Text("\(r.label) (\(r.count))").font(.callout)
                    Spacer()
                    IXChip(text: PKFileStatus.label(r.status), tone: PKFileStatus.tone(r.status))
                }
                .padding(.horizontal, 10).padding(.vertical, 6)
                .accessibilityElement(children: .combine)
                if i < rows.count - 1 { Divider() }
            }
        }
        .background(theme["surface"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("What this backup contains")
        .accessibilityIdentifier("package-file-status")
    }
}

/// Names of linked files with their status (never claims a missing file is included).
struct PKLinkedFiles: View {
    let files: [BackupFileEntry]

    var body: some View {
        if !files.isEmpty {
            VStack(alignment: .leading, spacing: 2) {
                ForEach(files.prefix(8), id: \.assetId) { f in
                    Text("\(f.name): \(PKFileStatus.label(f.status))" + (f.path.map { " \u{00B7} \($0)" } ?? ""))
                        .textSelection(.enabled)
                }
                if files.count > 8 { Text("and \(files.count - 8) more") }
            }
            .font(.caption).foregroundStyle(.secondary)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Linked files")
            .accessibilityIdentifier("package-linked-files")
        }
    }
}

struct PKDoneBanner<Detail: View>: View {
    let title: String
    var path: String?
    let app: AppModel
    @ViewBuilder var detail: () -> Detail

    var body: some View {
        IXBanner(tone: .ok, text: Text(title).bold()) {
            if let path {
                Button("Show in folder") { Task { try? await app.core?.revealLocation("exported", path: path) } }
                    .controlSize(.small)
                    .accessibilityIdentifier("package-show-in-folder")
            }
        }
        .accessibilityIdentifier("package-done")
        VStack(alignment: .leading, spacing: 2) {
            detail()
            if let path { Text(path).font(.caption).foregroundStyle(.secondary).textSelection(.enabled).fixedSize(horizontal: false, vertical: true) }
        }
        .font(.callout)
    }
}

extension PKDoneBanner where Detail == EmptyView {
    init(title: String, path: String?, app: AppModel) {
        self.init(title: title, path: path, app: app, detail: { EmptyView() })
    }
}

// Project Home: what the project holds and where to continue (project.home). A brand-new
// project shows three starting actions and no invented numbers (FSD 6.3).

import SwiftUI
import OpenFrameCore

struct ProjectHomeView: View {
    let project: ProjectSummary
    let go: (Workspace) -> Void
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var home: ProjectHome?
    @State private var failure: CoreFailure?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(project.title)
                        .font(.system(size: 28, weight: .semibold))
                        .tracking(-0.5)
                    Text([project.projectType.rawValue, project.genre, project.creator.map { "by \($0)" }]
                        .compactMap { $0 }.joined(separator: " · "))
                        .foregroundStyle(.secondary)
                }
                if let failure {
                    Text(failure.message).foregroundStyle(theme["red-t"])
                }
                if let home {
                    if home.isEmpty {
                        startingActions
                    } else {
                        counts(home.counts)
                        activity(home.recentActivity)
                    }
                } else if failure == nil {
                    ProgressView()
                }
            }
            .padding(32)
            .frame(maxWidth: 900, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .background(theme["bg"])
        .task(id: app.changes.any) {
            do {
                home = try await app.core?.projectHome()
                failure = nil
            } catch {
                failure = CoreFailure(asCore: error)
            }
        }
        .accessibilityIdentifier("project-home")
    }

    private var startingActions: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Start here").font(.headline)
            HStack(spacing: 14) {
                action("Write the screenplay", "doc.text", .screenplay)
                action("Plan the story", "rectangle.3.group", .story)
                action("Collect ideas", "lightbulb", .ideaVault)
            }
        }
        .accessibilityIdentifier("project-home-start")
    }

    private func action(_ title: String, _ symbol: String, _ ws: Workspace) -> some View {
        Button { go(ws) } label: {
            VStack(alignment: .leading, spacing: 10) {
                Image(systemName: symbol).font(.title2).foregroundStyle(theme["accent"])
                Text(title).font(.headline)
            }
            .padding(18)
            .frame(maxWidth: .infinity, minHeight: 96, alignment: .topLeading)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
    }

    private func counts(_ counts: [HomeCount]) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 12)], spacing: 12) {
            ForEach(counts, id: \.key) { c in
                VStack(alignment: .leading, spacing: 4) {
                    Text("\(Int(c.count))").font(.system(size: 24, weight: .semibold)).monospacedDigit()
                    Text(c.label).font(.callout).foregroundStyle(.secondary)
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                .accessibilityElement(children: .combine)
            }
        }
    }

    private func activity(_ items: [ActivityEntry]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Recent activity").font(.headline)
            if items.isEmpty {
                Text("Nothing yet.").foregroundStyle(.secondary)
            }
            ForEach(items, id: \.id) { entry in
                HStack(alignment: .firstTextBaseline) {
                    Text(entry.summary)
                    Spacer()
                    Text(Formatting.relative(ms: entry.at)).font(.caption).foregroundStyle(.tertiary)
                }
                Divider()
            }
        }
    }
}

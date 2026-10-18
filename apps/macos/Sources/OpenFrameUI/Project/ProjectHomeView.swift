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
            VStack(alignment: .leading, spacing: PageSpacing.large) {
                PageHeader(project.title,
                           subtitle: [project.projectType.rawValue, project.genre, project.creator.map { "by \($0)" }]
                               .compactMap { $0 }.joined(separator: " · "))
                if let failure {
                    Text(failure.message).foregroundStyle(theme["red-t"])
                        .frame(maxWidth: .infinity)
                }
                if let home {
                    if home.isEmpty {
                        startingActions
                    } else {
                        counts(home.counts)
                        activity(home.recentActivity)
                    }
                } else if failure == nil {
                    ProgressView().frame(maxWidth: .infinity)
                }
            }
            .padding(.bottom, PageSpacing.page)
            .pageColumn()
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
        VStack(alignment: .leading, spacing: 0) {
            SectionHeader("Start here")
            HStack(spacing: PageSpacing.medium) {
                action("Write the screenplay", "doc.text", .screenplay)
                action("Plan the story", "rectangle.3.group", .story)
                action("Collect ideas", "lightbulb", .ideaVault)
            }
        }
        .accessibilityIdentifier("project-home-start")
    }

    private func action(_ title: String, _ symbol: String, _ ws: Workspace) -> some View {
        Button { go(ws) } label: {
            VStack(spacing: 10) {
                Image(systemName: symbol).font(.title2).foregroundStyle(theme["accent"])
                Text(title).font(.headline).multilineTextAlignment(.center)
            }
            .padding(PageSpacing.medium)
            .frame(maxWidth: .infinity, minHeight: 96, alignment: .center)
            .background(theme["panel"], in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(theme["line"]))
            .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(PressableCardStyle())
    }

    private func counts(_ counts: [HomeCount]) -> some View {
        CenteredCards(cardWidth: 160, spacing: PageSpacing.medium) {
            ForEach(counts, id: \.key) { c in
                VStack(spacing: 4) {
                    Text("\(Int(c.count))").font(.system(size: 24, weight: .semibold)).monospacedDigit()
                    Text(c.label).font(.callout).foregroundStyle(.secondary)
                }
                .padding(PageSpacing.medium)
                .frame(maxWidth: .infinity, alignment: .center)
                .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                .accessibilityElement(children: .combine)
            }
        }
    }

    private func activity(_ items: [ActivityEntry]) -> some View {
        VStack(alignment: .leading, spacing: PageSpacing.small) {
            SectionHeader("Recent activity")
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

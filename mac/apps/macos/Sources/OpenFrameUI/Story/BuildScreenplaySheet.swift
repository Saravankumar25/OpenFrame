// Build Screenplay from the Story Board: a preview first, then the build. A card without a valid
// scene heading is never built silently and no heading is invented: the preview asks for each
// one. An existing screenplay is never overwritten (a new draft or a new screenplay is added).

import SwiftUI
import OpenFrameCore

struct BuildScreenplaySheet: View {
    @Bindable var model: StoryModel
    let preselect: [String]
    @State private var b: BuildScreenplayModel?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(spacing: PageSpacing.tight) {
                Text("Build Screenplay from Story Board").font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                Text("Preview the order before anything is created.").font(.callout).foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            if let b, b.preview != nil {
                form(b)
            } else if let f = b?.failure {
                StoryBanner(tone: .err, text: "The preview could not be prepared. \(f)")
                HStack { Spacer(); Button("Close") { model.sheet = nil }.keyboardShortcut(.cancelAction) }
            } else {
                ProgressView().frame(maxWidth: .infinity, minHeight: 120)
            }
        }
        .padding(22)
        .frame(width: 780)
        .background(theme["panel"])
        .task {
            let made = BuildScreenplayModel(story: model, preselect: preselect)
            b = made
            await made.load()
        }
    }

    @ViewBuilder private func form(_ b: BuildScreenplayModel) -> some View {
        if b.rows.isEmpty {
            StoryBanner(tone: .info, text: "There are no Scene Cards in the active Story Board yet\(b.preview?.parkedCount ?? 0 > 0 ? " (Parking Lot cards are excluded)" : ""). Add Scene Cards to Acts or Sequences first.")
            HStack { Spacer(); Button("Close") { model.sheet = nil }.keyboardShortcut(.cancelAction) }
        } else {
            StoryBanner(tone: .info, text: "Cards are used in Story Board order. Parking Lot cards are excluded. Nothing is created until you confirm.")
            if b.missing > 0 { StoryBanner(tone: .warn, text: b.missingBanner) }
            table(b)
            HStack(alignment: .top, spacing: 24) {
                descriptionChoices(b).frame(maxWidth: .infinity, alignment: .leading)
                destinationChoices(b).frame(maxWidth: .infinity, alignment: .leading)
            }
            HStack {
                Text(b.summary).font(.callout).foregroundStyle(.secondary).accessibilityIdentifier("build-summary")
                Spacer()
                Button("Cancel") { model.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Build Screenplay") { Task { await b.build() } }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .disabled(!b.canBuild)
                    .accessibilityIdentifier("build-confirm")
            }
        }
    }

    private func table(_ b: BuildScreenplayModel) -> some View {
        VStack(spacing: 0) {
            HStack {
                Text("#").frame(width: 30, alignment: .leading)
                Text("Story Board scene").frame(maxWidth: .infinity, alignment: .leading)
                Text("Heading").frame(width: 300, alignment: .leading)
                Text("Include").frame(width: 60, alignment: .leading)
            }
            .font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
            .padding(.horizontal, 10).padding(.vertical, 6)
            .background(theme["surface2"])
            Divider()
            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(b.rows, id: \.cardId) { r in
                        row(b, r)
                        Divider()
                    }
                }
            }
            .frame(maxHeight: 280)
        }
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line"]))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Cards to build")
    }

    private func row(_ b: BuildScreenplayModel, _ r: StoryBuildRow) -> some View {
        let inc = b.included.contains(r.cardId)
        let ok = b.headingOK(r)
        let n = Int(r.order)
        return HStack(alignment: .center) {
            Text("\(n)").frame(width: 30, alignment: .leading)
            VStack(alignment: .leading, spacing: 2) {
                Text(r.shortDescription.isEmpty ? "Blank scene card" : r.shortDescription).italic(r.shortDescription.isEmpty).lineLimit(2)
                Text(r.location + (r.alreadyBuilt ? " \u{00B7} used in a screenplay before" : "")).font(.caption).foregroundStyle(theme["muted"])
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Group {
                if r.headingValid && !b.isEdited(r) {
                    Text(r.sceneHeading ?? "").font(.system(size: 12.5, design: .monospaced))
                } else {
                    HStack(spacing: 6) {
                        TextField("Add a heading\u{2026}", text: Binding(get: { b.heading(for: r) }, set: { b.setHeading(r.cardId, $0) }))
                            .textFieldStyle(.roundedBorder)
                            .font(.system(size: 12.5, design: .monospaced))
                            .accessibilityLabel("Heading for scene \(n)")
                        if !ok && inc {
                            Text("Heading needed").font(.caption2.weight(.semibold)).padding(.horizontal, 6).padding(.vertical, 2)
                                .background(theme["yellow-soft"], in: Capsule()).foregroundStyle(theme["yellow-t"]).fixedSize()
                            Button("Leave out") { b.leaveOut(r.cardId) }
                                .controlSize(.small)
                                .accessibilityLabel("Leave out scene \(n)")
                        }
                    }
                }
            }
            .frame(width: 300, alignment: .leading)
            Toggle("Include scene \(n)", isOn: Binding(get: { inc }, set: { b.setIncluded(r.cardId, $0) }))
                .labelsHidden()
                .toggleStyle(.checkbox)
                .frame(width: 60, alignment: .leading)
        }
        .padding(.horizontal, 10).padding(.vertical, 6)
        .opacity(inc ? 1 : 0.5)
    }

    private func descriptionChoices(_ b: BuildScreenplayModel) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("How to use the card description").font(.headline)
            StoryChoice(title: "Use as a scene planning note (default)", detail: "The description is kept as a note outside the printed script.", selected: b.mode == .planningNote) { b.mode = .planningNote }
            StoryChoice(title: "Insert as temporary action text", detail: "Editable draft text you are expected to rewrite.", selected: b.mode == .actionText) { b.mode = .actionText }
        }
    }

    private func destinationChoices(_ b: BuildScreenplayModel) -> some View {
        let existing = b.existing
        return VStack(alignment: .leading, spacing: 8) {
            Text("Destination").font(.headline)
            if !existing.isEmpty {
                Text("\(existing.count == 1 ? "\u{201C}\(existing[0].title)\u{201D} already has a screenplay" : "This project already has screenplays")\(existing.contains { $0.hasWrittenScenes } ? " with written scenes" : ""). Building again will not overwrite it.")
                    .font(.callout).foregroundStyle(theme["muted"])
            }
            StoryChoice(
                title: "New Draft in the existing screenplay",
                detail: b.target.map { "\u{201C}\($0.nextDraftName)\u{201D} is added; \($0.currentDraftName.map { "\u{201C}\($0)\u{201D}" } ?? "the current draft") is untouched." } ?? "Available once a screenplay exists.",
                selected: b.destination == .newDraft, disabled: existing.isEmpty
            ) { if !existing.isEmpty { b.destination = .newDraft } }
            if b.destination == .newDraft, existing.count > 1 {
                Picker("Screenplay", selection: Binding(get: { b.screenplayId }, set: { b.screenplayId = $0 })) {
                    ForEach(existing, id: \.id) { Text($0.title).tag($0.id) }
                }
                .padding(.leading, 26)
            }
            StoryChoice(
                title: existing.isEmpty ? "New Screenplay" : "Build a new screenplay document",
                detail: existing.isEmpty ? "A new screenplay with its first draft." : "A separate screenplay alongside the current one, with its own drafts. The existing screenplay is not changed.",
                selected: b.destination == .newScreenplay
            ) { b.destination = .newScreenplay }
            if b.destination == .newScreenplay {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Title of the new screenplay").font(.caption).foregroundStyle(theme["muted"])
                    TextField("Title", text: Binding(get: { b.title }, set: { b.title = String($0.prefix(200)) }))
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Title of the new screenplay")
                    if b.titleClash {
                        Text("Another screenplay already has this title. Choose a different title.").font(.caption).foregroundStyle(theme["red-t"])
                    }
                }
                .padding(.leading, 26)
            }
        }
    }
}

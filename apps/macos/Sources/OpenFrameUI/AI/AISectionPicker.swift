// The section picker of the assistant (ADR-0041). The person chooses where the assistant reads: a
// list of sections from `ai.sections`, up to three, with chips to take one away and a smaller choice
// under a section that has many items (which draft, scene, day or call sheet). Nothing is picked for
// them and nothing follows the current screen. The core checks every pick.

import OpenFrameCore
import SwiftUI

/// The button that opens the list.
struct AISectionPickerButton: View {
    @Bindable var model: AIAssistantModel
    @State private var open = false

    var body: some View {
        Button {
            open.toggle()
        } label: {
            Label("Sections", systemImage: "square.stack.3d.up")
        }
        .help(AIText.sectionsIntro)
        .accessibilityLabel("Sections")
        .accessibilityValue(model.picks.isEmpty ? "None picked" : "\(model.picks.count) picked")
        .accessibilityHint(AIText.sectionsIntro)
        .accessibilityIdentifier("ai-section-picker")
        .popover(isPresented: $open, arrowEdge: .bottom) {
            AISectionList(model: model)
                .frame(width: 300, height: 420)
        }
    }
}

/// The picked sections, each with a button that takes it away.
struct AISectionChips: View {
    @Bindable var model: AIAssistantModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if !model.picks.isEmpty {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(model.picks) { c in
                    chip(c)
                }
            }
        }
    }

    private func chip(_ c: AISectionChoice) -> some View {
        let label = model.sectionLabel(c)
        return HStack(spacing: 4) {
            Text(verbatim: label)
                .font(.caption.weight(.medium))
                .lineLimit(1)
                .truncationMode(.middle)
            Button {
                model.removeSection(c.leaf)
            } label: {
                Label("Remove", systemImage: "xmark.circle.fill").labelStyle(.iconOnly).imageScale(.small)
            }
            .buttonStyle(.plain)
            .disabled(model.pending != nil)
            .help("Remove \(label)")
            .accessibilityLabel("Remove \(label)")
            .accessibilityIdentifier("ai-section-chip-remove-\(c.leaf)")
        }
        .foregroundStyle(theme["ink2"])
        .padding(.horizontal, 7)
        .padding(.vertical, 2)
        .background(theme["accent-soft"], in: Capsule())
        .accessibilityElement(children: .contain)
        .accessibilityLabel(label)
        .accessibilityIdentifier("ai-section-chip-\(c.leaf)")
    }
}

/// The hierarchical list: branches, their leaves, and the instance choice under a picked leaf.
struct AISectionList: View {
    @Bindable var model: AIAssistantModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                Text(AIText.sectionsIntro).font(.caption).foregroundStyle(theme["muted"])
                    .fixedSize(horizontal: false, vertical: true)
                if model.picks.count >= AILogic.maxSections {
                    Text(AIText.sectionsAtMost).font(.caption.weight(.medium)).foregroundStyle(theme["ink2"])
                        .fixedSize(horizontal: false, vertical: true)
                        .accessibilityIdentifier("ai-section-limit")
                }
                ForEach(model.sections?.branches ?? [], id: \.label) { branch in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(verbatim: branch.label)
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(theme["muted"])
                            .accessibilityAddTraits(.isHeader)
                        ForEach(branch.leaves, id: \.id) { leaf in
                            row(leaf, branch: branch.label)
                        }
                    }
                }
            }
            .padding(12)
        }
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Sections")
        .accessibilityIdentifier("ai-section-list")
    }

    private func row(_ leaf: AiSectionLeaf, branch: String) -> some View {
        let picked = model.picks.contains { $0.leaf == leaf.id }
        let enabled = AILogic.sectionRowEnabled(leaf, picks: model.picks) && model.pending == nil
        let note = AILogic.sectionRowNote(leaf, picks: model.picks)
        return VStack(alignment: .leading, spacing: 4) {
            Button {
                model.toggleSection(leaf.id)
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: picked ? "checkmark.circle.fill" : "circle")
                        .foregroundStyle(picked ? theme["accent"] : theme["muted"])
                        .accessibilityHidden(true)
                    Text(verbatim: leaf.label).font(.callout)
                    Spacer(minLength: 0)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(!enabled)
            .help(note ?? (picked ? "Remove \(leaf.label)" : "Add \(leaf.label)"))
            .accessibilityLabel("\(branch), \(leaf.label)")
            .accessibilityValue(picked ? "Picked" : "Not picked")
            .accessibilityHint(note ?? "")
            .accessibilityIdentifier("ai-section-row-\(leaf.id)")
            if picked, let kind = leaf.picker {
                instances(kind, leaf: leaf.id)
                    .padding(.leading, 22)
            }
        }
    }

    @ViewBuilder
    private func instances(_ kind: AiSectionPicker, leaf: String) -> some View {
        let choice = model.picks.first { $0.leaf == leaf }
        switch kind {
        case .screenplay: screenplayChoices(choice, leaf: leaf)
        case .scene: scenePicker(choice, leaf: leaf, scenes: model.sections?.scenes ?? [])
        case .day: dayPicker(choice, leaf: leaf)
        case .callSheet:
            dayPicker(choice, leaf: leaf)
            picker(
                "Call sheet", id: "ai-section-sheet-\(leaf)", all: AIText.allCallSheets,
                options: (model.sections?.callSheets ?? []).map { ($0.id, $0.label) },
                selection: choice?.callSheetId
            ) { model.chooseCallSheet($0, for: leaf) }
        }
    }

    @ViewBuilder
    private func screenplayChoices(_ choice: AISectionChoice?, leaf: String) -> some View {
        let screenplays = AILogic.screenplays(model.sections)
        if screenplays.count > 1 {
            picker(
                "Screenplay", id: "ai-section-screenplay-\(leaf)", all: AIText.chooseScreenplay,
                options: screenplays.map { ($0.screenplayId, $0.screenplayTitle) },
                selection: choice?.screenplayId
            ) { id in Task { await model.chooseScreenplay(id, for: leaf) } }
        }
        let drafts = AILogic.drafts(model.sections, screenplay: choice?.screenplayId)
        if !drafts.isEmpty {
            picker(
                "Draft", id: "ai-section-draft-\(leaf)", all: AIText.currentDraft,
                options: drafts.map { ($0.id, $0.name) },
                selection: choice?.draftId
            ) { id in Task { await model.chooseDraft(id, for: leaf) } }
        }
        scenePicker(choice, leaf: leaf, scenes: model.draftScenes ?? model.sections?.scenes ?? [])
    }

    private func scenePicker(_ choice: AISectionChoice?, leaf: String, scenes: [AiSectionInstance]) -> some View {
        picker(
            "Scene", id: "ai-section-scene-\(leaf)", all: AIText.allScenes,
            options: scenes.map { ($0.id, $0.label) },
            selection: choice?.sceneId
        ) { model.chooseScene($0, for: leaf) }
    }

    private func dayPicker(_ choice: AISectionChoice?, leaf: String) -> some View {
        picker(
            "Day", id: "ai-section-day-\(leaf)", all: AIText.allDays,
            options: (model.sections?.days ?? []).map { ($0.id, $0.label) },
            selection: choice?.shootingDayId
        ) { model.chooseDay($0, for: leaf) }
    }

    /// One small choice: the first row means all of them.
    private func picker(
        _ title: String, id: String, all: String, options: [(String, String)], selection: String?,
        choose: @escaping (String?) -> Void
    ) -> some View {
        Picker(title, selection: Binding(get: { selection }, set: { choose($0) })) {
            Text(all).tag(String?.none)
            ForEach(options, id: \.0) { option in
                Text(verbatim: option.1).tag(String?.some(option.0))
            }
        }
        .controlSize(.small)
        .font(.caption)
        .accessibilityLabel(title)
        .accessibilityIdentifier(id)
    }
}

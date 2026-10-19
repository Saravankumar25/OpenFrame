// Story Timeline: optional Story Day and time-of-day notes on screenplay scenes. A grouping view
// only; OpenFrame never reorders scenes for you. Mirrors workspaces/story/Timeline.tsx.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class TimelineModel {
    let story: StoryModel
    private(set) var data: StoryTimelineDto?
    private(set) var loaded = false
    var screenplayId: String?
    var picked: Set<String> = []
    var assigning = false

    init(story: StoryModel) { self.story = story }

    func reload() async {
        guard let core = story.app.core else { return }
        do {
            data = try await core.storyTimeline(StoryTimelineArgs(episodeId: story.episodeId, screenplayId: screenplayId))
            loaded = true
        } catch {
            loaded = true
            story.fail("Could not load the timeline", error)
        }
    }

    func toggle(_ sceneId: String, _ on: Bool) {
        if on { picked.insert(sceneId) } else { picked.remove(sceneId) }
    }

    /// Gives the picked scenes a Story Day, or clears it with nil.
    @discardableResult
    func assign(_ day: String?) async -> Bool {
        guard let core = story.app.core else { return false }
        let ids = Array(picked)
        do {
            _ = try await core.storyAssignStoryDay(StoryAssignDayArgs(sceneIds: ids, storyDay: day))
            story.say(day.map { "Assigned Story Day \($0)" } ?? "Cleared Story Day", undoable: true)
            picked = []
            assigning = false
            return true
        } catch { story.fail("Could not assign the story day", error); return false }
    }

    func setTimeNote(_ sceneId: String, _ note: String) async {
        guard let core = story.app.core else { return }
        do { _ = try await core.storySetTimeNote(StoryTimeNoteArgs(sceneId: sceneId, timeNote: note.isEmpty ? nil : note)) }
        catch { story.fail("Could not save the time note", error) }
    }

    static func dayLabel(_ d: String) -> String {
        let t = d.trimmingCharacters(in: .whitespaces)
        return !t.isEmpty && t.allSatisfy(\.isNumber) ? "Day \(t)" : d
    }
}

struct StoryTimelineView: View {
    @Bindable var story: StoryModel
    @State private var m: TimelineModel
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme

    init(story: StoryModel) {
        self.story = story
        _m = State(initialValue: TimelineModel(story: story))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(spacing: PageSpacing.small) {
                VStack(spacing: PageSpacing.tight) {
                    Text("Story Timeline").font(.title2.weight(.semibold)).accessibilityAddTraits(.isHeader)
                    Text("Optional. Useful only when time continuity matters.").foregroundStyle(theme["muted"])
                }
                HStack(spacing: PageSpacing.small) {
                if let d = m.data, d.screenplays.count > 1 {
                    Picker("Screenplay", selection: Binding(get: { d.screenplayId ?? "" }, set: { m.screenplayId = $0 })) {
                        ForEach(d.screenplays, id: \.id) { Text($0.title).tag($0.id) }
                    }
                    .frame(width: 220)
                }
                Button { m.assigning = true } label: {
                    Label("Assign Story Day\(m.picked.isEmpty ? "" : " (\(m.picked.count))")", systemImage: "calendar")
                }
                .buttonStyle(.borderedProminent)
                .disabled(m.picked.isEmpty)
                .accessibilityIdentifier("story-assign-day")
                }
            }
            .frame(maxWidth: .infinity)
            if let d = m.data {
                if d.draftId == nil {
                    ContentUnavailableView {
                        Label("No screenplay scenes yet.", systemImage: "calendar")
                    } description: {
                        Text("Story days belong to screenplay scenes. Build a screenplay from the Story Board or write one first. Story Board cards never need a story day.")
                    } actions: {
                        Button("Build Screenplay") {
                            story.tab = .board
                            story.sheet = .build(preselect: [])
                        }
                        .buttonStyle(.borderedProminent)
                    }
                } else {
                    Text("\(d.draftName ?? "") \u{00B7} \(StoryLogic.plural(Int(d.sceneCount), "scene")). Scene numbers come from the screenplay order.")
                        .font(.callout).foregroundStyle(theme["muted"])
                        .frame(maxWidth: .infinity)
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 12) {
                            ForEach(d.groups, id: \.storyDay) { g in group(g) }
                            Text("This is a grouping view only. OpenFrame never reorders scenes for you.").font(.caption).foregroundStyle(theme["muted"])
                        }
                    }
                }
            } else if m.loaded {
                Spacer()
            } else {
                ProgressView()
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .task(id: app.changes.key(StoryTables.timeline) &+ story.viewRevision &+ (story.episodeId?.hashValue ?? 0) &+ (m.screenplayId?.hashValue ?? 0)) {
            await m.reload()
        }
        .sheet(isPresented: $m.assigning) { AssignDaySheet(m: m) }
        .accessibilityIdentifier("story-timeline")
    }

    private func group(_ g: StoryDayGroup) -> some View {
        VStack(spacing: 0) {
            HStack {
                Text(g.storyDay.map(TimelineModel.dayLabel) ?? "Unassigned").fontWeight(.bold).textCase(.uppercase)
                Text(StoryLogic.plural(g.scenes.count, "scene")).font(.caption).padding(.horizontal, 6).background(.quaternary, in: Capsule())
                if g.storyDay == nil {
                    Text("Scenes you have not given a story day yet. This is fine: story days are optional.").font(.caption).foregroundStyle(theme["muted"])
                }
                Spacer()
            }
            .padding(.horizontal, 12).padding(.vertical, 8)
            .background(theme["surface2"])
            ForEach(g.scenes, id: \.sceneId) { s in
                Divider()
                TimelineSceneRow(m: m, s: s)
            }
        }
        .background(theme["panel"])
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(g.storyDay.map { "Day \($0)" } ?? "Unassigned")
    }
}

private struct TimelineSceneRow: View {
    @Bindable var m: TimelineModel
    let s: StoryTimelineScene
    @State private var note = ""
    @State private var last = ""
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: 10) {
            Toggle("Select scene \(Int(s.number))", isOn: Binding(get: { m.picked.contains(s.sceneId) }, set: { m.toggle(s.sceneId, $0) }))
                .labelsHidden().toggleStyle(.checkbox)
            Text("Sc \(Int(s.number))").fontWeight(.bold).frame(width: 48, alignment: .leading)
            Text(s.heading.isEmpty ? "(no heading)" : s.heading).font(.system(size: 12.5, design: .monospaced)).lineLimit(1)
            Spacer()
            TextField("time note", text: $note)
                .textFieldStyle(.roundedBorder).frame(width: 160)
                .focused($focused)
                .accessibilityLabel("Time of day note for scene \(Int(s.number))")
                .onSubmit { save() }
                .onChange(of: focused) { _, now in if !now { save() } }
        }
        .padding(.horizontal, 12).padding(.vertical, 6)
        .onAppear { note = s.timeNote ?? ""; last = note }
        .onChange(of: s.timeNote) { _, v in if !focused { note = v ?? ""; last = note } }
    }

    private func save() {
        guard note != last else { return }
        last = note
        let n = note
        Task { await m.setTimeNote(s.sceneId, n) }
    }
}

private struct AssignDaySheet: View {
    @Bindable var m: TimelineModel
    @State private var day = ""
    @FocusState private var focused: Bool

    var body: some View {
        StorySheetFrame(title: "Assign Story Day", sub: StoryLogic.plural(m.picked.count, "scene") + " selected", width: 380) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Story Day").font(.callout.weight(.medium))
                TextField("Story Day", text: $day).textFieldStyle(.roundedBorder).focused($focused)
                    .accessibilityLabel("Story Day")
                    .onSubmit { assign() }
                Text("For example 1, 2, 3, or a label such as Flashback.").font(.caption).foregroundStyle(.secondary)
            }
        } footer: {
            Button("Clear Story Day") { Task { await m.assign(nil) } }
            Button("Cancel") { m.assigning = false }.keyboardShortcut(.cancelAction)
            Button("Assign") { assign() }.keyboardShortcut(.defaultAction).disabled(day.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .onAppear { focused = true }
    }

    private func assign() {
        let d = day.trimmingCharacters(in: .whitespaces)
        guard !d.isEmpty else { return }
        Task { await m.assign(d) }
    }
}

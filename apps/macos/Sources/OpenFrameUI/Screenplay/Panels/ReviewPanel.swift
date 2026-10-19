// Review rounds (Mac phase M4), as ReviewView.tsx on Windows: start a round on a draft with
// reviewers and a deadline, read the notes it collected grouped by scene, answer a note, add a
// reviewer, and complete the round. Completing a review never locks the script.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class ReviewPanelModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?
    @ObservationIgnored var say: (String) -> Void = { _ in }

    var rounds: [ScreenplayReviewRoundDto] = []
    var roundId: String?
    var threads: [CommentThread] = []
    var selectedThread: String?
    var loading = false

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel, say: @escaping (String) -> Void) {
        self.app = app
        self.workspace = workspace
        self.say = say
    }

    private var core: Core? { app?.core }
    var drafts: [ScreenplayDraftDto] { workspace?.overview?.drafts ?? [] }
    var permissions: ScreenplayPermissions? { workspace?.overview?.permissions }
    var round: ScreenplayReviewRoundDto? { rounds.first { $0.id == roundId } }

    func reload() async {
        guard let core, let screenplayId = workspace?.overview?.screenplay?.id else { return }
        loading = true
        defer { loading = false }
        do {
            rounds = try await core.screenplayReviewRounds(ScreenplayReviewRoundsArgs(screenplayId: screenplayId))
            if roundId == nil || round == nil { roundId = rounds.first?.id }
            if let id = roundId {
                threads = try await core.commentList(CommentListArgs(reviewRoundId: id))
            } else {
                threads = []
            }
        } catch {
            app?.show("Could not load the review rounds", error)
        }
    }

    func select(_ id: String?) async {
        roundId = id
        selectedThread = nil
        await reload()
    }

    /// Start a round. Returns the message to show when the core refuses.
    func start(draftId: String, name: String, reviewers: [String], deadline: String?) async -> String? {
        guard let core else { return "OpenFrame is not ready." }
        await workspace?.flush()
        do {
            let r = try await core.screenplayStartReview(ScreenplayStartReviewArgs(
                draftId: draftId, name: name, reviewers: reviewers, deadline: deadline?.panelOptional))
            say("Started review \(PanelText.quoted(r.name))")
            roundId = r.id
            await workspace?.loadOverview()
            await reload()
            return nil
        } catch {
            return PanelText.message(error, fallback: "The review could not be started.")
        }
    }

    func addReviewer(_ name: String) async -> String? {
        guard let core, let round else { return nil }
        do {
            _ = try await core.screenplayUpdateReview(ScreenplayUpdateReviewArgs(
                reviewRoundId: round.id, reviewers: round.reviewers + [name.trimmingCharacters(in: .whitespaces)]))
            await reload()
            return nil
        } catch {
            return PanelText.message(error, fallback: "The reviewer could not be added.")
        }
    }

    func complete() async {
        guard let core, let round else { return }
        do {
            _ = try await core.screenplayCompleteReview(ScreenplayReviewRoundIdArgs(reviewRoundId: round.id))
            say("Completed \(PanelText.quoted(round.name)). The script was not locked.")
            await workspace?.loadOverview()
            await reload()
        } catch {
            app?.show("Could not complete the review", error)
        }
    }

    /// The notes of the round grouped by scene, in scene order.
    var groups: [(label: String, order: Double, items: [CommentThread])] {
        var map: [String: (label: String, order: Double, items: [CommentThread])] = [:]
        var keys: [String] = []
        for t in threads {
            let c = t.comment
            let key = c.sceneId ?? "draft"
            let label: String
            if let n = c.sceneNumber {
                label = "Scene \(Int(n)): \(c.sceneHeading.flatMap { $0.isEmpty ? nil : $0 } ?? "Untitled scene")"
            } else if c.targetType == "screenplay_draft" {
                label = "Whole draft"
            } else {
                label = "Deleted or moved scene"
            }
            if map[key] == nil { map[key] = (label, c.sceneNumber ?? 0, []); keys.append(key) }
            map[key]!.items.append(t)
        }
        return keys.compactMap { map[$0] }.sorted { $0.order < $1.order }
    }

    static func deadlineText(_ iso: String) -> String {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        guard let d = f.date(from: iso) else { return iso }
        return d.formatted(.dateTime.day().month(.abbreviated))
    }
}

struct ReviewSheet: View {
    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    /// Open the New Review dialog for this draft at once (from a draft's menu).
    let startFor: String?
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = ReviewPanelModel()
    @State private var newReview = false
    @State private var newDraftId: String?
    @State private var addReviewer = false

    var body: some View {
        VStack(spacing: 0) {
            PanelTitle("Review").padding(.top, PageSpacing.medium)
            HStack(spacing: 10) {
                if model.rounds.count > 1 {
                    Picker("Review round", selection: Binding(get: { model.roundId ?? "" }, set: { id in Task { await model.select(id) } })) {
                        ForEach(model.rounds, id: \.id) { Text($0.name + ($0.status == "Complete" ? " (complete)" : "")).tag($0.id) }
                    }
                    .frame(width: 240)
                    .accessibilityLabel("Review round")
                }
                Spacer()
                if model.permissions?.edit == true {
                    Button("New Review\u{2026}") { newDraftId = nil; newReview = true }.accessibilityIdentifier("review-new")
                }
                if let r = model.round, r.status == "Open" {
                    if model.permissions?.edit == true { Button("Add Reviewer\u{2026}") { addReviewer = true } }
                    if model.permissions?.resolveComments == true {
                        Button("Complete Review") { Task { await model.complete() } }
                            .accessibilityIdentifier("review-complete")
                    }
                }
                Button("Done") { state.sheet = nil }.keyboardShortcut(.cancelAction)
            }
            .padding(PageSpacing.medium)
            Divider()
            if let r = model.round { roundBody(r) } else if model.loading { ProgressView().controlSize(.small).frame(maxWidth: .infinity, maxHeight: .infinity) } else { empty }
        }
        .frame(width: 880, height: 600)
        .background(theme["panel"])
        .task {
            model.attach(app, workspace, say: { state.say($0) })
            await model.reload()
            if let startFor { newDraftId = startFor; newReview = true }
        }
        .task(id: app.changes.key(["comment", "screenplay_review_round"])) { await model.reload() }
        .sheet(isPresented: $newReview) { NewReviewSheet(model: model, initialDraftId: newDraftId) }
        .sheet(isPresented: $addReviewer) { AddReviewerSheet(model: model) }
        .accessibilityIdentifier("review-sheet")
    }

    private var empty: some View {
        VStack(spacing: 8) {
            Text("No review rounds yet").font(.headline)
            Text("Start a review on a draft, add reviewers, and collect their notes as comments on the script.")
                .foregroundStyle(theme["muted"]).multilineTextAlignment(.center)
            if model.permissions?.edit == true {
                Button("New Review\u{2026}") { newDraftId = nil; newReview = true }
            }
        }
        .padding(30)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func roundBody(_ r: ScreenplayReviewRoundDto) -> some View {
        HSplitView {
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    Text(r.name).font(.title2.weight(.semibold))
                    let meta = ["Source draft: \(r.draftName)",
                                r.reviewers.isEmpty ? nil : "Reviewers: \(r.reviewers.joined(separator: ", "))",
                                r.deadline.map { "Deadline \(ReviewPanelModel.deadlineText($0))" }].compactMap { $0 }
                    Text(meta.joined(separator: " \u{00B7} ")).font(.callout).foregroundStyle(theme["muted"])
                    HStack(spacing: 6) {
                        PanelChip(text: "Open notes: \(Int(r.openCount + r.discussionCount))", tone: .yellow)
                        PanelChip(text: "Resolved: \(Int(r.resolvedCount))", tone: .green)
                        PanelChip(text: r.status == "Open" ? "In progress" : "Complete", tone: r.status == "Open" ? .blue : .plain)
                    }
                    if r.status == "Complete" {
                        PanelBanner(tone: .ok, text: "This review is complete. Its comments stay in review history.")
                    }
                    if model.groups.isEmpty {
                        Text("No notes yet. Open the script, select text, and choose Add Comment to add the first note to this review.")
                            .foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                    }
                    ForEach(model.groups, id: \.label) { g in
                        Text(g.label).font(.headline).padding(.top, 6)
                        ForEach(g.items, id: \.comment.id) { t in
                            Button { model.selectedThread = t.comment.id } label: {
                                HStack(alignment: .firstTextBaseline, spacing: 8) {
                                    PanelChip(text: t.comment.status, tone: t.comment.status == "Resolved" ? .green : t.comment.status == "In Discussion" ? .blue : .yellow)
                                    Text(t.comment.body).frame(maxWidth: .infinity, alignment: .leading)
                                    Text(t.comment.authorName).font(.caption).foregroundStyle(theme["muted"])
                                }
                                .padding(8)
                                .background(model.selectedThread == t.comment.id ? theme["accent-soft"] : theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
                            }
                            .buttonStyle(.plain)
                            .accessibilityAddTraits(model.selectedThread == t.comment.id ? .isSelected : [])
                        }
                    }
                }
                .padding(16)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minWidth: 400)
            VStack(alignment: .leading, spacing: 8) {
                Text("Selected thread").font(.headline)
                if let t = model.threads.first(where: { $0.comment.id == model.selectedThread }) {
                    ScrollView {
                        ThreadCard(thread: t, comments: workspaceComments, focused: true)
                    }
                } else {
                    Text("Select a note to read and answer it.").foregroundStyle(theme["muted"])
                    Spacer()
                }
            }
            .padding(14)
            .frame(minWidth: 280, idealWidth: 320)
        }
    }

    /// Comment actions for the review sheet share the comments model of the panel state.
    private var workspaceComments: CommentsPanelModel { state.comments }
}

struct NewReviewSheet: View {
    let model: ReviewPanelModel
    let initialDraftId: String?
    @Environment(\.dismiss) private var dismiss
    @State private var draftId = ""
    @State private var name = ""
    @State private var reviewers: [String] = []
    @State private var who = ""
    @State private var hasDeadline = false
    @State private var deadline = Date()
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "New Review", width: 480) {
            VStack(alignment: .leading, spacing: 10) {
                PanelField(label: "Review name") {
                    TextField("Review name", text: $name).textFieldStyle(.roundedBorder).accessibilityLabel("Review name")
                }
                PanelField(label: "Source draft") {
                    Picker("Source draft", selection: $draftId) {
                        ForEach(model.drafts, id: \.id) { Text($0.name).tag($0.id) }
                    }
                    .labelsHidden()
                    .accessibilityLabel("Source draft")
                    .onChange(of: draftId) { _, id in
                        if let d = model.drafts.first(where: { $0.id == id }), name.hasPrefix("Review of ") { name = "Review of \(d.name)" }
                    }
                }
                PanelField(label: "Reviewers") {
                    VStack(alignment: .leading, spacing: 6) {
                        ForEach(reviewers, id: \.self) { r in
                            HStack {
                                Text(r)
                                Spacer()
                                Button { reviewers.removeAll { $0 == r } } label: { Image(systemName: "xmark.circle") }
                                    .buttonStyle(.borderless)
                                    .accessibilityLabel("Remove \(r)")
                            }
                        }
                        HStack {
                            TextField("Add reviewer", text: $who).textFieldStyle(.roundedBorder)
                                .accessibilityLabel("Reviewer name")
                                .onSubmit(add)
                            Button("Add", action: add).disabled(who.trimmingCharacters(in: .whitespaces).isEmpty)
                        }
                    }
                }
                PanelField(label: "Deadline (optional)") {
                    HStack {
                        Toggle("Set a deadline", isOn: $hasDeadline)
                        if hasDeadline { DatePicker("Deadline", selection: $deadline, displayedComponents: .date).labelsHidden() }
                    }
                }
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Start Review") {
                add()
                Task {
                    let f = DateFormatter()
                    f.calendar = Calendar(identifier: .gregorian)
                    f.locale = Locale(identifier: "en_US_POSIX")
                    f.dateFormat = "yyyy-MM-dd"
                    error = await model.start(draftId: draftId, name: name, reviewers: reviewers, deadline: hasDeadline ? f.string(from: deadline) : nil)
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || draftId.isEmpty)
        }
        .onAppear {
            draftId = initialDraftId ?? model.round?.draftId ?? model.drafts.first { $0.isCurrent }?.id ?? model.drafts.first?.id ?? ""
            name = "Review of \(model.drafts.first { $0.id == draftId }?.name ?? "Draft")"
        }
    }

    private func add() {
        let w = who.trimmingCharacters(in: .whitespaces)
        if !w.isEmpty, !reviewers.contains(where: { $0.caseInsensitiveCompare(w) == .orderedSame }) { reviewers.append(w) }
        who = ""
    }
}

struct AddReviewerSheet: View {
    let model: ReviewPanelModel
    @Environment(\.dismiss) private var dismiss
    @State private var who = ""
    @State private var error: String?

    var body: some View {
        PanelSheetFrame(title: "Add reviewer", width: 360) {
            VStack(alignment: .leading, spacing: 8) {
                PanelField(label: "Reviewer name") {
                    TextField("Reviewer name", text: $who).textFieldStyle(.roundedBorder).accessibilityLabel("Reviewer name")
                }
                if let error { PanelBanner(tone: .err, text: error) }
            }
        } footer: {
            Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
            Button("Add") {
                Task {
                    error = await model.addReviewer(who)
                    if error == nil { dismiss() }
                }
            }
            .keyboardShortcut(.defaultAction)
            .disabled(who.trimmingCharacters(in: .whitespaces).isEmpty)
        }
    }
}

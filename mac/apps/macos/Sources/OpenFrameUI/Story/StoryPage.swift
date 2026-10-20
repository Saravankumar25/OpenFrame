// The Story page (owner request 2026-10-20): a free page in the Story workspace, left of the
// Beat Board, to write the story in prose like a notepad. It has no link to the Beat Board, the
// screenplay or any other page. The text is saved as it is typed (story.save_page) and can be
// exported as plain text, PDF or a Word document (story.export_page).

import AppKit
import Observation
import OpenFrameCore
import SwiftUI

@MainActor
@Observable
final class StoryPageModel {
    let app: AppModel
    var text = ""
    private(set) var rev: Int64 = 0
    private(set) var loaded = false
    private(set) var canEdit = true
    private(set) var state: SaveState = .saved
    private(set) var error: String?
    /// Another window saved the page after this one read it.
    private(set) var conflict = false
    var exportModel: CSExportModel?

    enum SaveState: Equatable { case saved, saving, unsaved }

    @ObservationIgnored private var saveTask: Task<Void, Never>?
    @ObservationIgnored private var savedText = ""

    init(app: AppModel) { self.app = app }

    var words: Int {
        text.split(whereSeparator: { !($0.isLetter || $0.isNumber || $0 == "'" || $0 == "\u{2019}") })
            .filter { $0.contains(where: { $0.isLetter || $0.isNumber }) }.count
    }

    func load() async {
        guard let core = app.core else { return }
        do {
            let p = try await core.storyPage()
            apply(p)
            text = p.body
            loaded = true
            error = nil
        } catch {
            self.error = CoreFailure(asCore: error).message
            loaded = true
        }
    }

    private func apply(_ p: StoryPageDto) {
        rev = Int64(p.rev)
        savedText = p.body
        canEdit = p.canEdit
        conflict = false
    }

    /// Called on every keystroke: saves a second after typing stops.
    func edited() {
        guard text != savedText else { state = .saved; return }
        state = .unsaved
        saveTask?.cancel()
        saveTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(1))
            guard !Task.isCancelled else { return }
            await self?.save()
        }
    }

    /// Saves now (leaving the page, closing the project, exporting).
    func save() async {
        saveTask?.cancel()
        guard let core = app.core, text != savedText, !conflict else {
            if text == savedText { state = .saved }
            return
        }
        let sending = text
        state = .saving
        do {
            let p = try await core.storySavePage(StorySavePageArgs(body: sending, baseRev: Double(rev)))
            apply(p)
            state = text == sending ? .saved : .unsaved
            error = nil
            if text != sending { edited() }
        } catch {
            let f = CoreFailure(asCore: error)
            if f.code == "conflict.stale" { conflict = true }
            self.error = f.message
            state = .unsaved
        }
    }

    /// Takes the saved page again after a conflict. What was typed here is put on the clipboard first.
    func reloadAfterConflict() async {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        conflict = false
        await load()
        state = .saved
    }

    func export() async {
        await save()
        let title: String? = {
            if case let .project(p) = app.phase { return p.title }
            return nil
        }()
        exportModel = CSExportModel(
            title: "Export Story",
            sub: "The Story page as you wrote it.",
            source: "Source: the Story page",
            documentName: "Story",
            note: "Export creates a copy of the page; the page does not change.",
            formats: [
                CSExportFormat(value: "txt", label: "Plain text (TXT)", hint: "opens anywhere"),
                CSExportFormat(value: "pdf", label: "PDF", hint: "printable"),
                CSExportFormat(value: "docx", label: "Word (DOCX)", hint: "to keep editing"),
            ],
            projectTitle: title
        ) { [weak self] format, path, _ in
            guard let core = self?.app.core else { throw CoreFailure(code: "no_project", message: "No project is open.") }
            return try await core.storyExportPage(ExportStoryPageArgs(format: format, path: path))
        }
    }
}

struct StoryPageView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: StoryPageModel?
    @FocusState private var focused: Bool

    var body: some View {
        Group {
            if let m = model, m.loaded {
                page(m)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task {
            if model == nil { model = StoryPageModel(app: app) }
            await model?.load()
        }
        .onDisappear { if let m = model { Task { await m.save() } } }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("story-page")
    }

    private func page(_ m: StoryPageModel) -> some View {
        VStack(spacing: 0) {
            HStack(spacing: PageSpacing.small) {
                Text("Story").font(.title3.weight(.semibold))
                Text("A free page to write your story. It is not linked to the Beat Board or the screenplay.")
                    .font(.caption).foregroundStyle(theme["muted"])
                    .lineLimit(2)
                Spacer(minLength: PageSpacing.small)
                Text(statusText(m)).font(.caption).foregroundStyle(theme["muted"])
                    .accessibilityIdentifier("story-page-status")
                Button {
                    Task { await m.export() }
                } label: {
                    Label("Export", systemImage: "square.and.arrow.up")
                }
                .disabled(m.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .help("Export the Story page as plain text, PDF or a Word document")
                .accessibilityIdentifier("story-page-export")
            }
            .padding(.horizontal, PageSpacing.large).padding(.vertical, PageSpacing.small)
            if m.conflict {
                BDBanner(tone: .warn, content: {
                    Text("The Story page was changed in another window. Reload it to see that version; what you typed here is copied to the clipboard first.")
                }, actions: {
                    Button("Reload") { Task { await m.reloadAfterConflict() } }
                })
                .padding(.horizontal, PageSpacing.large)
            } else if let e = m.error {
                BDBanner(.err, e).padding(.horizontal, PageSpacing.large)
            }
            Divider()
            ZStack(alignment: .topLeading) {
                theme["well"]
                TextEditor(text: Binding(get: { m.text }, set: { m.text = $0; m.edited() }))
                    .font(.system(size: 16, design: .serif))
                    .lineSpacing(5)
                    .scrollContentBackground(.hidden)
                    .padding(.horizontal, 48).padding(.vertical, 40)
                    .background(theme["panel"], in: RoundedRectangle(cornerRadius: 6))
                    .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line"]))
                    .frame(maxWidth: 820)
                    .frame(maxWidth: .infinity)
                    .padding(PageSpacing.large)
                    .focused($focused)
                    .disabled(!m.canEdit)
                    .accessibilityLabel("Story")
                    .accessibilityIdentifier("story-page-text")
                if m.text.isEmpty {
                    Text("Write your story here.")
                        .font(.system(size: 16, design: .serif))
                        .foregroundStyle(theme["muted"])
                        .padding(.horizontal, 53).padding(.vertical, 40)
                        .frame(maxWidth: 820, alignment: .leading)
                        .frame(maxWidth: .infinity)
                        .padding(PageSpacing.large)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
            }
        }
        .sheet(item: Binding(get: { m.exportModel }, set: { m.exportModel = $0 })) { e in
            CSExportSheet(model: e) { m.exportModel = nil }
        }
        .onAppear { focused = true }
    }

    private func statusText(_ m: StoryPageModel) -> String {
        let words = m.words == 1 ? "1 word" : "\(m.words) words"
        switch m.state {
        case .saved: return "\(words) \u{00B7} Saved"
        case .saving: return "\(words) \u{00B7} Saving"
        case .unsaved: return "\(words) \u{00B7} Not saved yet"
        }
    }
}

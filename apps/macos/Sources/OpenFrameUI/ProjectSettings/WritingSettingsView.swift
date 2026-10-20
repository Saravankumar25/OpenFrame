// Settings > Writing (Windows: workspaces/settings/Writing.tsx): spell checking on or off, the
// main and second spelling language, and the project's own dictionary. The choices are stored in
// the project by the core (writing.proofing, writing.set_proofing, writing.dictionary*). On the
// Mac the words are checked by the system spelling service, so the languages offered are the ones
// macOS has; the screenplay editor follows the on/off choice and the main language.

import AppKit
import Observation
import OpenFrameCore
import SwiftUI

/// What the screenplay editor needs to know about spelling, for every open editor at once.
@MainActor
@Observable
final class SpellingPreference {
    static let shared = SpellingPreference()
    /// Check spelling as I write.
    var enabled = true
    /// BCP 47 tag of the main language ("en-US").
    var mainLanguage = "en-US"

    /// The system's name for a language tag ("en-US" becomes "en_US").
    static func systemCode(_ tag: String) -> String { tag.replacingOccurrences(of: "-", with: "_") }
    /// The BCP 47 tag for one of the system's languages ("en_GB" becomes "en-GB").
    static func tag(_ systemCode: String) -> String { systemCode.replacingOccurrences(of: "_", with: "-") }

    func apply(_ s: ProofingSettings) {
        enabled = s.enabled
        mainLanguage = s.mainLanguage
        let code = Self.systemCode(s.mainLanguage)
        let available = NSSpellChecker.shared.availableLanguages
        if let match = available.first(where: { $0 == code }) ?? available.first(where: { $0.hasPrefix(String(code.prefix(2))) }) {
            NSSpellChecker.shared.automaticallyIdentifiesLanguages = false
            _ = NSSpellChecker.shared.setLanguage(match)
        }
        NotificationCenter.default.post(name: .openFrameSpellingChanged, object: nil)
    }

    /// Reads the project's choice (when a project opens).
    func load(from core: Core) async {
        if let state = try? await core.writingProofing() { apply(state.settings) }
    }
}

extension Notification.Name {
    /// Spell checking was turned on or off, or its language changed.
    static let openFrameSpellingChanged = Notification.Name("studio.openframe.spellingChanged")
}

@MainActor
@Observable
final class WritingSettingsModel {
    let app: AppModel
    private(set) var state: WritingProofingState?
    private(set) var words: [String] = []
    private(set) var failure: String?
    var newWord = ""

    init(app: AppModel) { self.app = app }

    /// The languages the system can check, as tags, with readable names, sorted by name.
    var languages: [(tag: String, name: String)] {
        let current = [state?.settings.mainLanguage, state?.settings.secondLanguage].compactMap { $0 }
        let tags = Set(NSSpellChecker.shared.availableLanguages.map(SpellingPreference.tag) + current)
        return tags.map { ($0, Self.name($0)) }.sorted { $0.name < $1.name }
    }

    static func name(_ tag: String) -> String {
        Locale.current.localizedString(forIdentifier: SpellingPreference.systemCode(tag)) ?? tag
    }

    func load() async {
        guard let core = app.core else { return }
        do {
            state = try await core.writingProofing()
            words = try await core.writingDictionary().words
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error).message
        }
    }

    func set(enabled: Bool? = nil, main: String? = nil, second: String? = nil) async {
        guard let core = app.core else { return }
        do {
            let s = try await core.writingSetProofing(WritingSetProofingArgs(enabled: enabled, mainLanguage: main, secondLanguage: second))
            SpellingPreference.shared.apply(s)
            await load()
        } catch {
            failure = CoreFailure(asCore: error).message
        }
    }

    func addWord() async {
        let w = newWord.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !w.isEmpty, let core = app.core else { return }
        do {
            words = try await core.writingDictionaryAdd(WritingWordArgs(word: w)).words
            NSSpellChecker.shared.learnWord(w)
            newWord = ""
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error).message
        }
    }

    func remove(_ w: String) async {
        guard let core = app.core else { return }
        do {
            words = try await core.writingDictionaryRemove(WritingWordArgs(word: w)).words
            if NSSpellChecker.shared.hasLearnedWord(w) { NSSpellChecker.shared.unlearnWord(w) }
            failure = nil
        } catch {
            failure = CoreFailure(asCore: error).message
        }
    }
}

struct WritingSettingsView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: WritingSettingsModel?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: PageSpacing.large) {
                if let m = model, let s = m.state {
                    content(m, s)
                } else if let f = model?.failure {
                    Text(f).foregroundStyle(theme["red-t"])
                } else {
                    ProgressView().frame(maxWidth: .infinity)
                }
            }
            .padding(PageSpacing.large)
            .frame(maxWidth: PageSpacing.column, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .task {
            if model == nil { model = WritingSettingsModel(app: app) }
            await model?.load()
        }
        .accessibilityIdentifier("project-settings-writing")
    }

    @ViewBuilder private func content(_ m: WritingSettingsModel, _ s: WritingProofingState) -> some View {
        VStack(alignment: .leading, spacing: PageSpacing.small) {
            SectionHeader("Spelling")
            Toggle("Check spelling as I write", isOn: Binding(get: { s.settings.enabled }, set: { v in Task { await m.set(enabled: v) } }))
                .disabled(!s.canEdit)
                .accessibilityIdentifier("writing-spelling-enabled")
            Picker("Main language", selection: Binding(get: { s.settings.mainLanguage }, set: { v in Task { await m.set(main: v) } })) {
                ForEach(m.languages, id: \.tag) { Text($0.name).tag($0.tag) }
            }
            .disabled(!s.canEdit || !s.settings.enabled)
            .frame(maxWidth: 420)
            .accessibilityIdentifier("writing-main-language")
            Picker("Second language", selection: Binding(get: { s.settings.secondLanguage ?? "" }, set: { v in Task { await m.set(second: v) } })) {
                Text("None").tag("")
                ForEach(m.languages.filter { $0.tag != s.settings.mainLanguage }, id: \.tag) { Text($0.name).tag($0.tag) }
            }
            .disabled(!s.canEdit || !s.settings.enabled)
            .frame(maxWidth: 420)
            .accessibilityIdentifier("writing-second-language")
            Text("Words are checked on this Mac by the system spelling service. Character names and words you add below are never marked as mistakes.")
                .font(.caption).foregroundStyle(theme["muted"])
                .fixedSize(horizontal: false, vertical: true)
        }
        VStack(alignment: .leading, spacing: PageSpacing.small) {
            SectionHeader("Project dictionary")
            HStack {
                TextField("Add a word", text: Binding(get: { m.newWord }, set: { m.newWord = $0 }))
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 280)
                    .onSubmit { Task { await m.addWord() } }
                    .accessibilityIdentifier("writing-dictionary-new")
                Button("Add Word") { Task { await m.addWord() } }
                    .disabled(m.newWord.trimmingCharacters(in: .whitespaces).isEmpty || !s.canEdit)
                    .accessibilityIdentifier("writing-dictionary-add")
            }
            if m.words.isEmpty {
                Text("No words added yet.").foregroundStyle(theme["muted"])
            }
            ForEach(m.words, id: \.self) { w in
                HStack {
                    Text(w).textSelection(.enabled)
                    Spacer()
                    Button("Remove") { Task { await m.remove(w) } }
                        .controlSize(.small)
                        .disabled(!s.canEdit)
                        .accessibilityLabel("Remove \(w)")
                }
                Divider()
            }
            if let f = m.failure { Text(f).foregroundStyle(theme["red-t"]) }
        }
    }
}

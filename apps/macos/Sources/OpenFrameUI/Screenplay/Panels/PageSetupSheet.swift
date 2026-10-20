// Page setup and title page (Mac phase M4), as pagesetup.tsx and the title page dialog on
// Windows: paper, font, size, spacing, header and footer, the print switches, saved presets, and
// the title page fields. Saved with the screenplay; the page view, print preview and PDF all use it.

import SwiftUI
import OpenFrameCore

@MainActor
@Observable
final class PageSetupModel {
    @ObservationIgnored weak var app: AppModel?
    @ObservationIgnored weak var workspace: ScreenplayWorkspaceModel?
    @ObservationIgnored var say: (String) -> Void = { _ in }

    var info: ScreenplayPageSetupInfo?
    var setup: ScreenplayPageSetup?
    var titlePage: ScreenplayTitlePage?
    /// The title page as the PDF lays it out (the core draws it), for the preview beside the fields.
    var preview: InterchangePrintPreview?
    var previewFailed = false
    var presetName = ""
    var error: String?
    var busy = false

    static let placeholders = ["{title}", "{draft}", "{revision}", "{date}", "{page}"]

    init() {}

    func attach(_ app: AppModel, _ workspace: ScreenplayWorkspaceModel, say: @escaping (String) -> Void) {
        self.app = app
        self.workspace = workspace
        self.say = say
    }

    private var core: Core? { app?.core }
    private var screenplayId: String? { workspace?.overview?.screenplay?.id }
    var canEdit: Bool { workspace?.overview?.permissions.edit == true }

    func load() async {
        guard let core, let id = screenplayId else { return }
        do {
            let i = try await core.screenplayPageSetup(ScreenplayIdArgs(screenplayId: id))
            info = i
            setup = i.setup
            titlePage = workspace?.overview?.screenplay?.titlePage
        } catch {
            self.error = PanelText.message(error, fallback: "The page setup could not be loaded.")
        }
    }

    /// Apply a preset's settings to the form. Save keeps them.
    func apply(_ preset: ScreenplayPagePreset) { setup = preset.setup }

    func insert(_ placeholder: String, intoHeader: Bool) {
        guard var s = setup else { return }
        if intoHeader { s.header = Self.appending(placeholder, to: s.header) } else { s.footer = Self.appending(placeholder, to: s.footer) }
        setup = s
    }

    static func appending(_ p: String, to text: String) -> String {
        text + (text.isEmpty || text.hasSuffix(" ") ? "" : " ") + p
    }

    func save() async -> Bool {
        guard let core, let id = screenplayId, let setup else { return false }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.screenplayUpdatePageSetup(ScreenplayUpdatePageSetupArgs(screenplayId: id, setup: setup))
            say("Changed the page setup")
            await workspace?.loadOverview()
            return true
        } catch {
            self.error = PanelText.message(error, fallback: "The page setup could not be saved.")
            return false
        }
    }

    func savePreset() async {
        guard let core, let setup else { return }
        let name = presetName.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return }
        do {
            _ = try await core.screenplaySavePagePreset(ScreenplaySavePagePresetArgs(name: name, setup: setup))
            say("Saved the preset \(PanelText.quoted(name))")
            presetName = ""
            error = nil
            await reloadPresets()
        } catch {
            self.error = PanelText.message(error, fallback: "The preset could not be saved.")
        }
    }

    func deletePreset(_ p: ScreenplayPagePreset) async {
        guard let core else { return }
        do {
            _ = try await core.screenplayDeletePagePreset(ScreenplayIdOnlyArgs(id: p.id))
            say("Moved the preset \(PanelText.quoted(p.name)) to Recently Deleted")
            await reloadPresets()
        } catch {
            app?.show("Could not delete the preset", error)
        }
    }

    private func reloadPresets() async {
        guard let core, let id = screenplayId, let i = try? await core.screenplayPageSetup(ScreenplayIdArgs(screenplayId: id)) else { return }
        info = i
    }

    /// Ask the core to lay the title page out, a moment after typing stops (the caller waits).
    func refreshPreview() async {
        guard let core, let id = screenplayId, let titlePage else { return }
        do {
            let p = try await core.screenplayTitlePagePreview(ScreenplayTitlePagePreviewArgs(
                screenplayId: id, titlePage: titlePage, draftId: workspace?.draft?.id))
            preview = p
            previewFailed = false
        } catch {
            preview = nil
            previewFailed = true
        }
    }

    func saveTitlePage() async -> Bool {
        guard let core, let id = screenplayId, let titlePage else { return false }
        busy = true
        defer { busy = false }
        do {
            _ = try await core.screenplayUpdateTitlePage(ScreenplayTitlePageArgs(screenplayId: id, titlePage: titlePage))
            if let setup, setup.includeTitlePage != info?.setup.includeTitlePage {
                _ = try await core.screenplayUpdatePageSetup(ScreenplayUpdatePageSetupArgs(screenplayId: id, setup: setup))
            }
            say("Saved the title page")
            await workspace?.loadOverview()
            return true
        } catch {
            self.error = PanelText.message(error, fallback: "The title page could not be saved.")
            return false
        }
    }
}

struct PageSetupSheet: View {
    enum Page: String, CaseIterable, Identifiable { case page = "Page", titlePage = "Title Page"; var id: String { rawValue } }

    let workspace: ScreenplayWorkspaceModel
    let state: ScreenplayPanelState
    var initialPage: Page = .page
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model = PageSetupModel()
    @State private var page = Page.page

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            PanelTitle(page == .page ? "Page setup" : "Title page",
                       sub: page == .page ? "Saved with this screenplay. The page view, print preview and PDF all use it."
                                          : "Shown on the title page when you export or print.")
            Picker("Section", selection: $page) {
                ForEach(Page.allCases) { Text($0.rawValue).tag($0) }
            }
            .pickerStyle(.segmented).labelsHidden().accessibilityLabel("Section")
            if let error = model.error { PanelBanner(tone: .err, text: error) }
            ScrollView {
                if page == .page { pageForm } else { titleForm }
            }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { state.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Save") {
                    Task {
                        let ok = page == .page ? await model.save() : await model.saveTitlePage()
                        if ok { state.sheet = nil }
                    }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canEdit || model.busy || (page == .page ? model.setup == nil : model.titlePage == nil))
                .accessibilityIdentifier("page-setup-save")
            }
        }
        .padding(PageSpacing.large)
        .frame(width: page == .page ? 560 : 860, height: 640)
        .background(theme["panel"])
        .task {
            page = initialPage
            model.attach(app, workspace, say: { state.say($0) })
            await model.load()
        }
        .task(id: "\(page.rawValue)|\(model.titlePage.map { $0.hashValue } ?? 0)") {
            guard page == .titlePage, model.titlePage != nil else { return }
            // The first preview is at once; later ones wait until typing stops.
            if model.preview != nil { try? await Task.sleep(for: .milliseconds(300)) }
            if !Task.isCancelled { await model.refreshPreview() }
        }
        .accessibilityIdentifier("page-setup-sheet")
    }

    @ViewBuilder private var pageForm: some View {
        if let info = model.info, model.setup != nil {
            VStack(alignment: .leading, spacing: 12) {
                PanelField(label: "Preset", hint: "Applies the preset's settings here; Save keeps them.") {
                    PanelWrapRow {
                        ForEach(info.presets, id: \.id) { p in
                            HStack(spacing: 2) {
                                Button(p.name) { model.apply(p) }
                                if !p.builtIn, model.canEdit {
                                    Button { Task { await model.deletePreset(p) } } label: { Label("Delete", systemImage: "xmark.circle") }
                                        .buttonStyle(.borderless)
                                        .accessibilityLabel("Delete the preset \(p.name)")
                                }
                            }
                        }
                    }
                }
                HStack(alignment: .top, spacing: 12) {
                    PanelField(label: "Paper") {
                        Picker("Paper", selection: bind(\.paper)) {
                            Text("US Letter").tag(ScreenplayPaper.letter)
                            Text("A4").tag(ScreenplayPaper.a4)
                        }.labelsHidden().accessibilityLabel("Paper")
                    }
                    PanelField(label: "Font") {
                        Picker("Font", selection: bind(\.font)) {
                            ForEach(info.fonts, id: \.self) { Text($0).tag($0) }
                        }.labelsHidden().accessibilityLabel("Font")
                    }
                    PanelField(label: "Size") {
                        Picker("Size", selection: Binding(get: { Int(model.setup?.fontSize ?? 12) }, set: { model.setup?.fontSize = Double($0) })) {
                            ForEach(Int(info.minFontSize)...Int(info.maxFontSize), id: \.self) { n in
                                Text(n == 12 ? "12 pt (standard)" : "\(n) pt").tag(n)
                            }
                        }.labelsHidden().accessibilityLabel("Size")
                    }
                }
                HStack(alignment: .top, spacing: 12) {
                    PanelField(label: "Line spacing") {
                        Picker("Line spacing", selection: bind(\.lineSpacing)) {
                            Text("Single (standard)").tag(ScreenplayLineSpacing.single)
                            Text("1.5 lines").tag(ScreenplayLineSpacing.oneAndHalf)
                            Text("Double").tag(ScreenplayLineSpacing.double)
                        }.labelsHidden().accessibilityLabel("Line spacing")
                    }
                    PanelField(label: "First page number") {
                        Stepper(value: Binding(get: { Int(model.setup?.firstPageNumber ?? 1) }, set: { model.setup?.firstPageNumber = Double(max(1, $0)) }), in: 1...9999) {
                            Text("\(Int(model.setup?.firstPageNumber ?? 1))").monospacedDigit()
                        }
                        .accessibilityLabel("First page number")
                    }
                }
                headerFooter("Header", isHeader: true)
                headerFooter("Footer", isHeader: false)
                Toggle("Show the page number, header and footer on the first page", isOn: bind(\.headerOnFirstPage))
                Toggle("Include the title page when exporting and printing", isOn: bind(\.includeTitlePage))
                Toggle("Print scene numbers", isOn: bind(\.sceneNumbers))
                Toggle("Mark revised lines with an asterisk", isOn: bind(\.revisionMarks))
                Text("\(Int(info.linesPerPage)) lines fit on a page with the saved settings.").font(.caption).foregroundStyle(theme["muted"])
                if model.canEdit {
                    HStack {
                        TextField("Save these settings as a preset", text: $model.presetName).textFieldStyle(.roundedBorder)
                            .accessibilityLabel("New preset name")
                        Button("Save Preset") { Task { await model.savePreset() } }
                            .disabled(model.presetName.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                }
            }
            .disabled(!model.canEdit)
        } else {
            ProgressView().controlSize(.small).frame(maxWidth: .infinity)
        }
    }

    private func headerFooter(_ label: String, isHeader: Bool) -> some View {
        PanelField(label: label, hint: "Optional. Placeholders fill in on every page.") {
            TextField(label, text: isHeader ? bind(\.header) : bind(\.footer), prompt: Text(isHeader ? "{title} {draft}" : "Page {page}"))
                .textFieldStyle(.roundedBorder)
                .accessibilityLabel(label)
            PanelWrapRow {
                ForEach(PageSetupModel.placeholders, id: \.self) { p in
                    Button(p) { model.insert(p, intoHeader: isHeader) }
                        .controlSize(.small)
                        .accessibilityLabel("Insert \(p) into the \(label.lowercased())")
                }
            }
        }
    }

    private func bind<V>(_ key: WritableKeyPath<ScreenplayPageSetup, V>) -> Binding<V> where V: Equatable {
        Binding(
            get: { model.setup![keyPath: key] },
            set: { model.setup?[keyPath: key] = $0 })
    }

    @ViewBuilder private var titleForm: some View {
        if model.titlePage != nil {
            HStack(alignment: .top, spacing: 18) {
            VStack(alignment: .leading, spacing: 10) {
                titleField("Title", \.title)
                titleField("Credit", \.credit, prompt: "Written by")
                titleField("Written by", \.writtenBy)
                titleField("Based on", \.source)
                titleField("Draft line", \.draftLine, prompt: "Shooting Draft")
                titleField("Revision line", \.revisionLine)
                titleField("Date", \.date)
                titleField("Contact name", \.contactName)
                titleField("Address", \.address)
                titleField("Phone", \.phone)
                titleField("Email", \.email)
                titleField("Representation", \.representation)
                titleField("Copyright", \.copyright)
                PanelField(label: "Notes") {
                    PanelTextArea(label: "Notes", text: Binding(get: { model.titlePage?.notes ?? "" }, set: { model.titlePage?.notes = $0 }), minHeight: 56)
                }
                Toggle("Include the title page when exporting and printing", isOn: bind(\.includeTitlePage))
                    .disabled(model.setup == nil)
                    .accessibilityIdentifier("title-page-include")
            }
            .disabled(!model.canEdit)
            TitlePagePreview(preview: model.preview, failed: model.previewFailed)
                .frame(width: 300)
            }
        } else {
            ProgressView().controlSize(.small).frame(maxWidth: .infinity)
        }
    }

    private func titleField(_ label: String, _ key: WritableKeyPath<ScreenplayTitlePage, String>, prompt: String = "") -> some View {
        PanelField(label: label) {
            TextField(label, text: Binding(get: { model.titlePage?[keyPath: key] ?? "" }, set: { model.titlePage?[keyPath: key] = $0 }),
                      prompt: Text(prompt))
                .textFieldStyle(.roundedBorder)
                .accessibilityLabel(label)
        }
    }
}

/// Views in a row that wraps to the next line.
struct PanelWrapRow<Content: View>: View {
    @ViewBuilder var content: () -> Content

    var body: some View {
        PanelWrapLayout(spacing: 6) { content() }
    }
}

struct PanelWrapLayout: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0, maxX: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x > 0, x + s.width > width { x = 0; y += rowHeight + spacing; rowHeight = 0 }
            x += s.width + spacing
            rowHeight = max(rowHeight, s.height)
            maxX = max(maxX, x - spacing)
        }
        return CGSize(width: maxX, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowHeight: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x > bounds.minX, x + s.width > bounds.maxX { x = bounds.minX; y += rowHeight + spacing; rowHeight = 0 }
            v.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(s))
            x += s.width + spacing
            rowHeight = max(rowHeight, s.height)
        }
    }
}

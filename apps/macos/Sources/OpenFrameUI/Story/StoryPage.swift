// The Story page (Story > Story tab, Windows StoryPage.tsx): one writing page for the writer's
// free text, shown as an A4 sheet like the screenplay page. It is not linked to cards, scenes,
// the screenplay or the assistant. The text lives in the project (story.page.get and
// story.page.save) with its formatting: font size, bold, italic, underline and highlight for any
// stretch of text, underline and highlight in a choice of colours. Export writes TXT (words
// only), PDF and Word (with the formatting, on A4); the core does the export.
//
// Nothing typed is lost: the page is saved 400 ms after the last change, on leaving the editor,
// when the tab or workspace changes, when the project closes and when the app quits
// (`StoryPageModel.flushAll`, called with the screenplay's, also before a storage move). Another
// window that saved first is not an error: a page on screen sends its save again on the newest
// revision with what the writer typed. A page that left the screen never writes over a newer
// revision, and an older change never replaces a newer one.

import AppKit
import Observation
import OpenFrameCore
import SwiftUI

@MainActor
@Observable
final class StoryPageModel {
    let app: AppModel

    enum Status: Equatable { case saved, saving, failed }

    /// The page as it was read: the editor is made from it once.
    struct Loaded: Equatable {
        var text: String
        var format: String?
    }

    private(set) var loaded: Loaded?
    private(set) var loadError: String?
    private(set) var status: Status = .saved
    /// Why the core refused the last save (too long, formatting it cannot read), in its words.
    private(set) var saveError: String?
    private(set) var words = 0
    /// What the toolbar shows for the selection.
    var tools = StoryPageTools()
    var exportModel: CSExportModel?

    static let saveDelay: Duration = .milliseconds(400)

    @ObservationIgnored private(set) var rev: Double = 0
    /// The newest content not yet stored, and the change it came from. Every change counts up, so
    /// an older text never replaces a newer one.
    @ObservationIgnored private var pending: Loaded?
    @ObservationIgnored private var pendingVersion = 0
    @ObservationIgnored private var version = 0
    /// The newest change handed to the save queue, and the newest one stored.
    @ObservationIgnored private var queuedVersion = 0
    @ObservationIgnored private var savedVersion = 0
    /// Content the core refused: it is not sent again until the writer changes it.
    @ObservationIgnored private var refused: Loaded?
    /// False once the page left the screen: its late saves never overwrite a newer revision.
    @ObservationIgnored private var onScreen = true
    @ObservationIgnored private var timer: Task<Void, Never>?
    /// The editor of this page, for the toolbar.
    @ObservationIgnored weak var editor: StoryPageTextView?
    /// The project the page was read from: its text is never stored in another project. The id,
    /// not the folder: a project or library move keeps the id and changes the folder.
    @ObservationIgnored private var projectId: String?

    private var openProjectId: String? {
        if case let .project(p) = app.phase { return p.id }
        return nil
    }

    init(app: AppModel) {
        self.app = app
        Self.register(self)
    }

    var statusLabel: String {
        switch status {
        case .saved: "Saved"
        case .saving: "Saving\u{2026}"
        case .failed: "Not saved yet"
        }
    }

    var wordsLabel: String { words == 1 ? "1 word" : "\(words.formatted()) words" }

    // MARK: Saves run one after another, also across a quick leave and return

    private static var lastSave: Task<Void, Never>?

    static func queue(_ job: @escaping @MainActor () async -> Void) -> Task<Void, Never> {
        let before = lastSave
        let run = Task { @MainActor in
            await before?.value
            await job()
        }
        lastSave = run
        return run
    }

    /// Waits until no save is running: a page that opens again never shows older text.
    static func settledSaves() async {
        var seen: Task<Void, Never>?
        repeat {
            seen = lastSave
            await seen?.value
        } while seen != lastSave
    }

    // MARK: Every open page, for closing and quitting

    private final class Weak { weak var model: StoryPageModel?; init(_ m: StoryPageModel) { model = m } }
    private static var live: [Weak] = []

    private static func register(_ model: StoryPageModel) {
        live.removeAll { $0.model == nil }
        live.append(Weak(model))
    }

    /// Stores what every open Story page holds. Called before the project closes, before a
    /// project or library move and when the app quits.
    static func flushAll() async {
        for entry in live { await entry.model?.flush() }
        await settledSaves()
    }

    // MARK: Reading and saving

    func load() async {
        await Self.settledSaves()
        guard let core = app.core else {
            loadError = "No project is open."
            return
        }
        do {
            let p = try await core.storyPageGet()
            projectId = openProjectId
            rev = p.rev
            words = StoryPageDocument.wordCount(p.text)
            loaded = Loaded(text: p.text, format: p.format)
            loadError = nil
        } catch {
            loadError = CoreFailure(asCore: error).message
        }
    }

    /// The page is on screen (again).
    func appear() { onScreen = true }

    /// Called by the editor after every change: stored 400 ms after the last one.
    func changed(text: String, format: String?) {
        version += 1
        pending = Loaded(text: text, format: format)
        pendingVersion = version
        onScreen = true
        status = .saving
        timer?.cancel()
        timer = Task { [weak self] in
            try? await Task.sleep(for: Self.saveDelay)
            guard !Task.isCancelled else { return }
            await self?.flush()
        }
    }

    func changed(_ s: NSAttributedString) {
        let c = StoryPageDocument.content(of: s)
        changed(text: c.text, format: c.format)
    }

    /// Hands what is waiting to the save queue. Nil when nothing is waiting, or when it is what
    /// the core refused (sent again only once the writer changed it).
    private func take() -> Task<Void, Never>? {
        timer?.cancel()
        timer = nil
        guard let content = pending, content != refused else { return nil }
        let v = pendingVersion
        pending = nil
        queuedVersion = max(queuedVersion, v)
        return Self.queue { await self.persist(content, version: v) }
    }

    /// Stores what is waiting now. True when the page is stored.
    @discardableResult
    func flush() async -> Bool {
        if let save = take() { await save.value } else { await Self.settledSaves() }
        return status != .failed && pending == nil
    }

    /// Leaving the editor: the save starts now (in the save queue), so a page that opens again
    /// waits for it. The queue holds the model until its save has run.
    func leave() { _ = take() }

    /// The page left the screen. A page that opens again reads the stored text; this one still
    /// stores what it holds, but never over a newer revision.
    func close() {
        onScreen = false
        leave()
    }

    private func persist(_ c: Loaded, version v: Int) async {
        guard v > savedVersion else { return }
        if v == version { words = StoryPageDocument.wordCount(c.text) }
        guard let core = app.core, projectId == nil || projectId == openProjectId else {
            keep(c, version: v, error: nil)
            return
        }
        do {
            let saved: StoryPageSaved
            do {
                saved = try await core.storyPageSave(StoryPageSaveArgs(text: c.text, format: c.format, expectedRev: rev))
            } catch {
                // Another window saved first. A page on screen takes its revision and keeps what
                // the writer typed. A page that left the screen does not: the newer revision
                // came from a page opened after it, and its newer text wins.
                guard CoreFailure(asCore: error).code.hasPrefix("conflict"), onScreen else { throw error }
                rev = try await core.storyPageGet().rev
                saved = try await core.storyPageSave(StoryPageSaveArgs(text: c.text, format: c.format, expectedRev: rev))
            }
            rev = saved.rev
            savedVersion = max(savedVersion, v)
            if pending != nil, pendingVersion <= v { pending = nil }
            if refused != nil, v >= pendingVersion { refused = nil }
            if v == version {
                status = .saved
                saveError = nil
            }
        } catch {
            let failure = CoreFailure(asCore: error)
            if failure.code.hasPrefix("conflict"), !onScreen {
                // A newer revision is stored: this older text is dropped, never written over it.
                if v >= queuedVersion, pending == nil { status = .saved }
                return
            }
            keep(c, version: v, error: failure.code.hasPrefix("validation") ? failure : nil)
        }
    }

    /// A save that failed keeps its text for the next try (the next change, leaving, closing),
    /// unless a newer change was queued or stored since. A refused save keeps the core's reason
    /// and is not sent again until the text changes.
    private func keep(_ c: Loaded, version v: Int, error refusal: CoreFailure?) {
        guard v >= queuedVersion, v > savedVersion else { return }
        if pending == nil {
            pending = c
            pendingVersion = v
        }
        if let refusal, pendingVersion == v {
            refused = c
            saveError = refusal.message
        }
        status = .failed
    }

    // MARK: Export

    /// Stores the text, then opens the export dialog (the file must hold what was typed).
    func export() async {
        guard await flush() else { return }
        let title: String? = {
            if case let .project(p) = app.phase { return p.title }
            return nil
        }()
        exportModel = CSExportModel(
            title: "Export Story",
            source: "Source: the Story page",
            documentName: "Story",
            note: "Export creates a snapshot and does not change your project.",
            formats: Self.exportFormats,
            projectTitle: title
        ) { [weak self] format, path, _ in
            guard let core = self?.app.core else { throw CoreFailure(code: "no_project", message: "No project is open.") }
            return try await core.storyPageExport(ExportStoryPageArgs(format: format, path: path))
        }
    }

    static let exportFormats = [
        CSExportFormat(value: "txt", label: "Plain text (.txt)", hint: "words only"),
        CSExportFormat(value: "pdf", label: "PDF", hint: "A4, with the formatting"),
        CSExportFormat(value: "docx", label: "Word (.docx)", hint: "A4, with the formatting"),
    ]
}

/// The Story page's view choices: kept per person on this Mac, never project data. The keys are
/// the Story page's own (the Beat Board's zoom is `of.story.zoom`).
enum StoryPagePrefs {
    static let zoomKey = "openframe.story.pageZoom"
    static let darkKey = "openframe.story.darkPage"
    static let zoomMin = 0.5
    static let zoomMax = 2.0
    static let zoomStep = 0.1
    /// The sheet is drawn at the screenplay page's scale (80 points to the inch).
    static let baseScale: CGFloat = 80.0 / 72.0

    static func clamp(_ z: Double) -> Double { (min(zoomMax, max(zoomMin, z)) * 100).rounded() / 100 }

    /// A stored zoom, or 100 percent when none is stored or it is out of range.
    static func zoom(stored z: Double) -> Double { z >= zoomMin && z <= zoomMax ? z : 1 }

    /// Command and the mouse wheel (or a two-finger scroll): up zooms in, down zooms out.
    static func wheel(_ z: Double, deltaY: Double) -> Double { clamp(z * exp(deltaY / 400)) }

    static func label(_ z: Double) -> String { "\(Int((z * 100).rounded()))%" }
}

// MARK: - Views

struct StoryPageView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: StoryPageModel?
    @AppStorage(StoryPagePrefs.zoomKey) private var storedZoom = 1.0
    @AppStorage(StoryPagePrefs.darkKey) private var darkPage = false

    private var zoom: Double { StoryPagePrefs.zoom(stored: storedZoom) }

    /// A page model can be handed in (snapshots); the screen makes its own otherwise.
    init(model: StoryPageModel? = nil) { _model = State(initialValue: model) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            if let m = model {
                if let e = m.loadError {
                    ContentUnavailableView("The Story page could not be opened.", systemImage: "exclamationmark.triangle", description: Text(e))
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if let loaded = m.loaded {
                    page(m, loaded)
                } else {
                    ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task {
            if model == nil { model = StoryPageModel(app: app) }
            if model?.loaded == nil { await model?.load() }
        }
        .onAppear { model?.appear() }
        .onDisappear { model?.close() }
        .themedSheet(item: Binding(get: { model?.exportModel }, set: { model?.exportModel = $0 })) { e in
            CSExportSheet(model: e) { model?.exportModel = nil }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("story-page")
    }

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: PageSpacing.small) {
            Text("Story").font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
            Text("Write your story freely. This page is not linked to anything else.")
                .font(.caption).foregroundStyle(theme["muted"]).lineLimit(2)
            Spacer(minLength: PageSpacing.small)
            Button {
                if let m = model { Task { await m.export() } }
            } label: {
                Label("Export", systemImage: "square.and.arrow.up")
            }
            .buttonStyle(.ofStandard)
            .disabled(model?.loaded == nil)
            .help("Export the Story page as plain text, PDF or a Word document")
            .accessibilityIdentifier("story-page-export")
        }
        .padding(.horizontal, PageSpacing.large).padding(.vertical, PageSpacing.small)
    }

    private func page(_ m: StoryPageModel, _ loaded: StoryPageModel.Loaded) -> some View {
        VStack(spacing: 6) {
            StoryPageToolbar(model: m, darkPage: $darkPage, zoom: zoom, setZoom: { storedZoom = StoryPagePrefs.clamp($0) })
                .padding(.horizontal, PageSpacing.large)
            StoryPageEditor(model: m, initial: loaded, dark: darkPage, zoom: zoom, setZoom: { storedZoom = StoryPagePrefs.clamp($0) })
                .background(theme["well"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                .padding(.horizontal, PageSpacing.large)
            VStack(alignment: .trailing, spacing: 2) {
                HStack {
                    Text(m.wordsLabel)
                    Spacer()
                    Text(m.statusLabel)
                        .accessibilityIdentifier("story-page-status")
                        .accessibilityAddTraits(.updatesFrequently)
                }
                // A save the core refused: its reason, until the writer changes the text.
                if let e = m.saveError, m.status == .failed {
                    Text(e)
                        .foregroundStyle(theme["red"])
                        .multilineTextAlignment(.trailing)
                        .fixedSize(horizontal: false, vertical: true)
                        .textSelection(.enabled)
                        .accessibilityIdentifier("story-page-save-error")
                }
            }
            .font(.caption).foregroundStyle(theme["muted"])
            .padding(.horizontal, PageSpacing.large + 4).padding(.bottom, 6)
        }
    }
}

/// The formatting toolbar: font size, Bold, Italic, Underline and Highlight (each colour from a
/// list beside it), Dark page and the page zoom.
struct StoryPageToolbar: View {
    @Bindable var model: StoryPageModel
    @Binding var darkPage: Bool
    let zoom: Double
    let setZoom: (Double) -> Void
    @Environment(\.ofTheme) private var theme

    private func run(_ action: (StoryPageTextView) -> Void) {
        guard let v = model.editor else { return }
        action(v)
        v.window?.makeFirstResponder(v)
    }

    var body: some View {
        let t = model.tools
        HStack(spacing: 4) {
            Picker("Font size", selection: Binding(
                get: { t.size ?? -1 },
                set: { pt in if pt > 0 { run { $0.setSize(pt) } } })) {
                if t.size == nil { Text("Mixed").tag(-1.0) }
                if let s = t.size, !StoryPageDocument.sizes.contains(s) { Text(Self.sizeLabel(s)).tag(s) }
                ForEach(StoryPageDocument.sizes, id: \.self) { Text(Self.sizeLabel($0)).tag($0) }
            }
            .labelsHidden()
            .frame(width: 80)
            .help("Font size (Command+[ and Command+])")
            .accessibilityLabel("Font size")
            .accessibilityIdentifier("story-page-size")
            Divider().frame(height: 18).padding(.horizontal, 4)
            tool("Bold", "bold", "Command+B", on: t.bold) { $0.toggle(\.bold) }
            tool("Italic", "italic", "Command+I", on: t.italic) { $0.toggle(\.italic) }
            colored(.underline, "Underline", "underline", "Command+U")
            colored(.highlight, "Highlight", "highlighter", "Command+Shift+H")
            Spacer()
            Button { darkPage.toggle() } label: { Image(systemName: darkPage ? "moon.fill" : "moon") }
                .buttonStyle(.ofStandard)
                .background(darkPage ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                .help(darkPage ? "Light page" : "Dark page")
                .accessibilityLabel("Dark page")
                .accessibilityValue(darkPage ? "On" : "Off")
                .accessibilityIdentifier("story-page-dark")
            HStack(spacing: 2) {
                Button { setZoom(zoom - StoryPagePrefs.zoomStep) } label: { Image(systemName: "minus.magnifyingglass") }
                    .disabled(zoom <= StoryPagePrefs.zoomMin)
                    .help("Zoom out (Command and the mouse wheel)")
                    .accessibilityLabel("Zoom out")
                    .accessibilityIdentifier("story-page-zoom-out")
                Button(StoryPagePrefs.label(zoom)) { setZoom(1) }
                    .monospacedDigit()
                    .frame(minWidth: 48)
                    .help("Reset the page zoom")
                    .accessibilityLabel("Page zoom \(StoryPagePrefs.label(zoom)), reset")
                    .accessibilityIdentifier("story-page-zoom-reset")
                Button { setZoom(zoom + StoryPagePrefs.zoomStep) } label: { Image(systemName: "plus.magnifyingglass") }
                    .disabled(zoom >= StoryPagePrefs.zoomMax)
                    .help("Zoom in (Command and the mouse wheel)")
                    .accessibilityLabel("Zoom in")
                    .accessibilityIdentifier("story-page-zoom-in")
            }
            .buttonStyle(.borderless)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Page zoom")
        }
        .padding(.horizontal, 6).padding(.vertical, 4)
        .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Formatting")
    }

    static func sizeLabel(_ s: Double) -> String { s == s.rounded() ? "\(Int(s))" : String(format: "%.1f", s) }

    private func tool(_ label: String, _ symbol: String, _ keys: String, on: Bool, action: @escaping (StoryPageTextView) -> Void) -> some View {
        Button { run(action) } label: { Image(systemName: symbol).frame(width: 18) }
            .buttonStyle(.borderless)
            .padding(4)
            .background(on ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
            .foregroundStyle(on ? theme["accent-d"] : theme["ink"])
            .help("\(label) (\(keys))")
            .accessibilityLabel(label)
            .accessibilityValue(on ? "On" : "Off")
            .accessibilityAddTraits(on ? .isSelected : [])
            .accessibilityIdentifier("story-page-\(label.lowercased())")
    }

    /// Split button: the main part puts on the last colour, the arrow opens the list.
    private func colored(_ kind: StoryPageTools.Colored, _ label: String, _ symbol: String, _ keys: String) -> some View {
        let t = model.tools
        let palette = kind == .underline ? StoryPageDocument.underlines : StoryPageDocument.highlights
        let current = t.color(kind).flatMap { c in palette.first { $0.name == c } }
        return HStack(spacing: 0) {
            tool(label, symbol, keys, on: t.isOn(kind)) { $0.toggleColored(kind) }
                .overlay(alignment: .bottom) {
                    if let current {
                        Capsule().fill(Color(nsColor: StoryPageDocument.color(current.light))).frame(height: 3).padding(.horizontal, 6).padding(.bottom, 1)
                            .accessibilityHidden(true)
                    }
                }
            Menu {
                ForEach(palette, id: \.name) { sw in
                    Button { run { $0.pickColor(kind, sw.name) } } label: {
                        Label { Text("\(label) \(Self.colorName(sw.name))") } icon: { Image(nsImage: Self.swatch(sw.light)) }
                    }
                }
                Divider()
                Button("No \(label.lowercased())") { run { $0.setColor(kind, nil) } }
            } label: {
                Image(systemName: "chevron.down").font(.system(size: 9))
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .help("\(label) colour")
            .accessibilityLabel("\(label) colour")
            .accessibilityIdentifier("story-page-\(kind.rawValue)-colour")
        }
    }

    /// The colour as the list names it ("Underline text colour", "Highlight green").
    static func colorName(_ c: String) -> String { c == "ink" ? "text colour" : c }

    private static func swatch(_ hex: String) -> NSImage {
        NSImage(size: NSSize(width: 14, height: 14), flipped: false) { r in
            StoryPageDocument.color(hex).setFill()
            NSBezierPath(roundedRect: r.insetBy(dx: 0.5, dy: 0.5), xRadius: 3, yRadius: 3).fill()
            NSColor.separatorColor.setStroke()
            NSBezierPath(roundedRect: r.insetBy(dx: 0.5, dy: 0.5), xRadius: 3, yRadius: 3).stroke()
            return true
        }
    }
}

/// The desk with the A4 sheet: an AppKit text view in a scroll view that zooms the page only
/// (Command and the mouse wheel, a pinch, or the toolbar), never the window.
struct StoryPageEditor: NSViewRepresentable {
    let model: StoryPageModel
    let initial: StoryPageModel.Loaded
    let dark: Bool
    let zoom: Double
    let setZoom: (Double) -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }

    /// The scroll view takes the room it is offered; the sheet inside scrolls. Its size must not
    /// follow the sheet, which would push the pane past a narrow window.
    func sizeThatFits(_ proposal: ProposedViewSize, nsView: StoryPageScrollView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 300, height: proposal.height ?? 300)
    }

    func makeNSView(context: Context) -> StoryPageScrollView {
        let scroll = StoryPageScrollView()
        scroll.contentView = StoryCenteringClipView()
        scroll.hasVerticalScroller = true
        scroll.hasHorizontalScroller = true
        scroll.autohidesScrollers = true
        scroll.drawsBackground = false
        scroll.allowsMagnification = true
        scroll.minMagnification = StoryPagePrefs.baseScale * StoryPagePrefs.zoomMin
        scroll.maxMagnification = StoryPagePrefs.baseScale * StoryPagePrefs.zoomMax
        let view = StoryPageTextView.make()
        view.load(StoryPageDocument.attributed(initial.text, initial.format))
        view.showPage(dark: dark)
        let m = model
        view.onChange = { [weak m] s in m?.changed(s) }
        view.onTools = { [weak m] t in m?.tools = t }
        view.onLeave = { [weak m] in m?.leave() }
        model.editor = view
        model.tools = view.tools
        scroll.documentView = view
        scroll.magnification = StoryPagePrefs.baseScale * zoom
        let coordinator = context.coordinator
        scroll.onWheelZoom = { [weak scroll, weak coordinator] deltaY in
            guard let scroll else { return }
            coordinator?.setZoom(StoryPagePrefs.wheel(scroll.magnification / StoryPagePrefs.baseScale, deltaY: deltaY))
        }
        coordinator.magnifyObserver = NotificationCenter.default.addObserver(
            forName: NSScrollView.didEndLiveMagnifyNotification, object: scroll, queue: .main
        ) { [weak scroll, weak coordinator] _ in
            MainActor.assumeIsolated {
                guard let scroll else { return }
                coordinator?.setZoom(StoryPagePrefs.clamp(scroll.magnification / StoryPagePrefs.baseScale))
            }
        }
        DispatchQueue.main.async { [weak view] in view?.window?.makeFirstResponder(view) }
        return scroll
    }

    func updateNSView(_ scroll: StoryPageScrollView, context: Context) {
        context.coordinator.setZoom = setZoom
        guard let view = scroll.documentView as? StoryPageTextView else { return }
        if view.isDarkPage != dark { view.showPage(dark: dark) }
        let wanted = StoryPagePrefs.baseScale * zoom
        if abs(scroll.magnification - wanted) > 0.001 { scroll.magnification = wanted }
    }

    static func dismantleNSView(_ scroll: StoryPageScrollView, coordinator: Coordinator) {
        if let o = coordinator.magnifyObserver { NotificationCenter.default.removeObserver(o) }
        coordinator.magnifyObserver = nil
        (scroll.documentView as? StoryPageTextView)?.onLeave = nil
    }

    @MainActor
    final class Coordinator {
        var setZoom: (Double) -> Void = { _ in }
        var magnifyObserver: NSObjectProtocol?
    }
}

/// Command and the mouse wheel zoom the page; the wheel alone scrolls.
final class StoryPageScrollView: NSScrollView {
    var onWheelZoom: ((Double) -> Void)?

    override func scrollWheel(with event: NSEvent) {
        if event.modifierFlags.contains(.command), let onWheelZoom {
            let d = event.hasPreciseScrollingDeltas ? event.scrollingDeltaY : event.scrollingDeltaY * 40
            onWheelZoom(Double(d))
        } else {
            super.scrollWheel(with: event)
        }
    }
}

/// Keeps the sheet in the middle of the desk when the desk is wider than the page. It never
/// takes the sheet's colour (a text view hands its colour to the clip view), so the desk shows.
final class StoryCenteringClipView: NSClipView {
    override var drawsBackground: Bool {
        get { false }
        set { super.drawsBackground = false }
    }

    override func constrainBoundsRect(_ proposedBounds: NSRect) -> NSRect {
        var r = super.constrainBoundsRect(proposedBounds)
        if let doc = documentView, r.width > doc.frame.width {
            r.origin.x = (doc.frame.width - r.width) / 2
        }
        return r
    }
}

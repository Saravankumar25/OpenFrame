// Shows the import, export, print and package sheets over the main window, from any workspace.
// The File menu and the Screenplay toolbar ask for a sheet by name; the presenter makes the model,
// hosts the SwiftUI sheet in an AppKit sheet window and removes it again when it closes. The
// editors store what they hold first, so a backup, an export or a print always shows the latest
// words. One sheet at a time.

import AppKit
import SwiftUI
import OpenFrameCore

enum InterchangeDialog {
    case importScreenplay(openScreenplayId: String?)
    case exportScreenplay(draftId: String?)
    case printPreview(draftId: String?)
    case backup
    case exportProject
    case openProject
    case exportExchange(PackageType)
    case importExchange(sessionId: String?)
    /// Opens the file panel first, then the viewer.
    case openReview
    case viewer(PKViewerSource, key: String, back: PackagesBack?)
    case queue
    case history
}

/// Where the viewer returns to when it closes.
indirect enum PackagesBack {
    case importExchange(sessionId: String)
    case history
}

@MainActor
final class InterchangePresenter {
    static let shared = InterchangePresenter()

    private var app: AppModel?
    private var themes: ThemeStore?
    private var sheetWindow: NSWindow?
    private weak var parent: NSWindow?
    /// Replaceable in tests: the panels answer with paths instead of showing.
    var picker: FilePicking = PanelFilePicker()

    /// Called by the app's menus and by the anchor view in the workspaces.
    func configure(app: AppModel? = nil, themes: ThemeStore? = nil) {
        if let app { self.app = app }
        if let themes { self.themes = themes }
    }

    var isPresenting: Bool { sheetWindow != nil }

    func present(_ dialog: InterchangeDialog) {
        guard let app else { return }
        Task { @MainActor in
            await ScreenplayWorkspaceModel.flushAll()
            self.show(dialog, app: app)
        }
    }

    private func show(_ dialog: InterchangeDialog, app: AppModel) {
        if dialog.isOpenReview {
            guard let url = picker.openFile(title: "Open Review Package", extensions: PKInfo.extensions(PKInfo.reviewOpenTypes)) else { return }
            let model = ReviewViewerModel(app: app, source: .review, key: "")
            present(view: ReviewViewerSheet(model: model, picker: picker, close: { [weak self] in self?.dismiss() }),
                    app: app, openAfter: { await model.open(file: url) })
            return
        }
        let close: () -> Void = { [weak self] in self?.dismiss() }
        switch dialog {
        case let .importScreenplay(id):
            present(view: ImportScreenplaySheet(model: ImportScreenplayModel(app: app, openScreenplayId: id), picker: picker, close: close), app: app)
        case let .exportScreenplay(draftId):
            present(view: ExportScreenplaySheet(model: ExportScreenplayModel(app: app, draftId: draftId), picker: picker, close: close), app: app)
        case let .printPreview(draftId):
            Task { @MainActor in
                guard let choice = await self.resolveDraft(app: app, draftId: draftId) else {
                    ToastCenter.shared.show("There is no screenplay to print yet.")
                    return
                }
                let model = PrintPreviewModel(app: app, draftId: choice.id, screenplayTitle: choice.screenplayTitle, draftName: choice.draftName)
                self.present(view: PrintPreviewSheet(model: model, picker: self.picker, back: nil, close: close), app: app)
            }
        case .backup:
            present(view: BackupSheet(model: BackupModel(app: app), picker: picker, close: close), app: app)
        case .exportProject:
            present(view: ExportProjectSheet(model: ExportProjectModel(app: app), picker: picker, close: close), app: app)
        case .openProject:
            present(view: OpenProjectSheet(model: OpenProjectModel(app: app), picker: picker, close: close), app: app)
        case let .exportExchange(type):
            present(view: ExchangeExportSheet(model: ExchangeExportModel(app: app, packageType: type), picker: picker, close: close), app: app)
        case let .importExchange(sessionId):
            present(view: ExchangeImportSheet(
                model: ExchangeImportModel(app: app, sessionId: sessionId), picker: picker,
                viewPackage: { [weak self] s in
                    self?.replace(with: .viewer(.session, key: s.id, back: .importExchange(sessionId: s.id)))
                },
                openQueue: { [weak self] in self?.replace(with: .queue) },
                close: close), app: app)
        case .openReview:
            break
        case let .viewer(source, key, back):
            let model = ReviewViewerModel(app: app, source: source, key: key)
            present(view: ReviewViewerSheet(model: model, picker: picker, close: { [weak self] in
                guard let self else { return }
                switch back {
                case let .importExchange(id)?: self.replace(with: .importExchange(sessionId: id))
                case .history?: self.replace(with: .history)
                case nil: self.dismiss()
                }
            }), app: app)
        case .queue:
            present(view: ReviewQueueSheet(model: ReviewQueueModel(app: app), close: close), app: app)
        case .history:
            present(view: PackageHistorySheet(
                model: PackageHistoryModel(app: app),
                openReview: { [weak self] id in self?.replace(with: .viewer(.review, key: id, back: .history)) },
                openRecord: { [weak self] id in self?.replace(with: .viewer(.record, key: id, back: .history)) },
                openSession: { [weak self] id in self?.replace(with: .importExchange(sessionId: id)) },
                close: close), app: app)
        }
    }

    /// File > Export PDF: the save panel, then the PDF with the draft's page-setup choices (title page,
    /// scene numbers, revision marks), without the options sheet.
    func exportPDFDirectly(app: AppModel, draftId: String?) async {
        configure(app: app)
        await ScreenplayWorkspaceModel.flushAll()
        let model = ExportScreenplayModel(app: app, draftId: draftId)
        await model.load()
        guard let choice = model.choice else {
            ToastCenter.shared.show("There is no screenplay to export yet.")
            return
        }
        model.format = .pdf
        let name = IXNames.defaultExportFileName(screenplayTitle: choice.screenplayTitle, draftName: choice.draftName, format: .pdf)
        guard let url = picker.saveFile(title: "Export PDF", suggestedName: name, directory: nil, message: nil) else { return }
        if !(await model.export(to: url)), let e = model.error {
            app.alert = .error(title: "Could not export the PDF", failure: CoreFailure(code: "export", message: e))
        }
    }

    /// The draft a menu command works on: the one asked for, else the current draft of the first screenplay.
    func resolveDraft(app: AppModel, draftId: String?) async -> IXDraftChoice? {
        let model = ExportScreenplayModel(app: app, draftId: draftId)
        await model.load()
        return model.choice
    }

    private func present<V: View>(view: V, app: AppModel, openAfter: (@MainActor () async -> Void)? = nil) {
        guard sheetWindow == nil else { return }
        let store = themes ?? ThemeStore()
        let root = view
            .environment(app)
            .environment(store)
            .openFrameTheme(store)
        let host = NSHostingController(rootView: root)
        let window = NSWindow(contentViewController: host)
        window.styleMask = [.titled]
        window.title = ""
        window.isReleasedWhenClosed = false
        sheetWindow = window
        if let parent = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first(where: { $0.isVisible }) {
            self.parent = parent
            parent.beginSheet(window)
        } else {
            window.makeKeyAndOrderFront(nil)
        }
        // Keyboard focus starts in the sheet's content, so Escape and the arrow keys reach it.
        window.makeFirstResponder(host.view)
        if let openAfter { Task { await openAfter() } }
    }

    /// Close the sheet that is showing and show another after it is gone.
    func replace(with dialog: InterchangeDialog) {
        dismiss()
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(150))
            self.present(dialog)
        }
    }

    func dismiss() {
        guard let window = sheetWindow else { return }
        if let parent, parent.attachedSheet === window { parent.endSheet(window) }
        window.orderOut(nil)
        sheetWindow = nil
        parent = nil
    }
}

private extension InterchangeDialog {
    var isOpenReview: Bool { if case .openReview = self { true } else { false } }
}

// MARK: Anchor

/// Gives the presenter the app model and the theme from inside the window. Put it on a view that is
/// always on screen (Home, a project); several anchors do no harm.
private struct InterchangeAnchor: ViewModifier {
    @Environment(AppModel.self) private var app
    @Environment(ThemeStore.self) private var themes

    func body(content: Content) -> some View {
        content.onAppear { InterchangePresenter.shared.configure(app: app, themes: themes) }
    }
}

extension View {
    /// Lets the File menu and the toolbars show the import, export, print and package sheets here.
    func interchangeAnchor() -> some View { modifier(InterchangeAnchor()) }
}

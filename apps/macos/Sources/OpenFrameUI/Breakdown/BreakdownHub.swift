// The root model of the Breakdown workspace. It owns the models of the five tabs (Breakdown,
// Catalog, Locations, Cast and Crew, Categories), the notice line that says what just happened,
// the category looks and the "may this person edit" hint, and it moves the person between tabs
// when a scene, a catalog item or a location is opened from another tab. It holds no production
// data of its own: every read and change is a core operation (ADR-0027).

import Foundation
import Observation
import OpenFrameCore

enum BDTab: String, CaseIterable, Identifiable {
    case breakdown, catalog, locations, castCrew, categories
    var id: String { rawValue }
    var title: String {
        switch self {
        case .breakdown: "Breakdown"
        case .catalog: "Catalog"
        case .locations: "Locations"
        case .castCrew: "Cast & Crew"
        case .categories: "Categories"
        }
    }
}

/// A line at the bottom of the screen that says what just happened, with Undo when the change
/// went through the project history.
struct BDNotice: Identifiable, Equatable {
    let id = UUID()
    var text: String
    var undoable: Bool
}

@MainActor
@Observable
final class BreakdownHub {
    let app: AppModel
    var tab: BDTab = .breakdown
    var notice: BDNotice?
    private(set) var looks = CategoryLooks()
    private(set) var styles: BreakdownCategoryStyles?
    /// A hint for the screens. The core still refuses what a role may not do.
    private(set) var canEdit = true

    @ObservationIgnored private(set) var breakdown: BreakdownModel!
    @ObservationIgnored private(set) var catalog: CatalogModel!
    @ObservationIgnored private(set) var locations: LocationsModel!
    @ObservationIgnored private(set) var people: PeopleModel!
    @ObservationIgnored private(set) var categories: CategoryStylesModel!
    @ObservationIgnored private var noticeTask: Task<Void, Never>?

    init(app: AppModel) {
        self.app = app
        breakdown = BreakdownModel(hub: self)
        catalog = CatalogModel(hub: self)
        locations = LocationsModel(hub: self)
        people = PeopleModel(hub: self)
        categories = CategoryStylesModel(hub: self)
    }

    var core: Core? { app.core }

    // MARK: Shared reads

    func reloadStyles() async {
        guard let core else { return }
        if let s = try? await core.breakdownCategories() {
            styles = s
            looks = CategoryLooks(styles: s.categories)
        }
    }

    func reloadMembers() async {
        guard let core else { return }
        if let members = try? await core.projectMembers() {
            let me = members.first { $0.isYou }
            canEdit = me == nil || me?.role == "Owner" || me?.role == "Editor"
        }
    }

    func look(_ c: BreakdownCategory) -> CategoryLook { looks.look(c) }

    // MARK: Notices and errors

    func say(_ text: String, undoable: Bool = false) {
        let n = BDNotice(text: text, undoable: undoable)
        notice = n
        noticeTask?.cancel()
        noticeTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(8))
            guard !Task.isCancelled else { return }
            if self?.notice?.id == n.id { self?.notice = nil }
        }
    }

    func dismissNotice() {
        noticeTask?.cancel()
        notice = nil
    }

    func undoLast() async {
        dismissNotice()
        await app.undo()
    }

    func fail(_ title: String, _ error: Error) {
        app.show(title, error)
    }

    // MARK: Moving between tabs

    /// A scene chip anywhere opens that scene's breakdown.
    func openScene(_ sceneId: String) {
        breakdown.focusScene(sceneId)
        tab = .breakdown
    }

    /// "Show all scenes with this item" and the catalog's "Open Scenes".
    func openScenes(forCatalogItem id: String) {
        breakdown.focusCatalogItem(id)
        tab = .breakdown
    }

    func openInCatalog(_ id: String) {
        catalog.focus(id)
        tab = .catalog
    }
}

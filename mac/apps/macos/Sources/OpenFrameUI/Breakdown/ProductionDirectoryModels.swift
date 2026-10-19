// The models of the Catalog, Locations, Cast and Crew and Categories tabs. Each one holds what the
// screen shows and the person's view (search, filter, the open record, the sheet) and sends every
// change to a `catalog.*`, `locations.*`, `cast.*`, `crew.*`, `availability.*` or `breakdown.*`
// operation. They mirror apps/desktop/src/workspaces/production/tabs (catalog.tsx, locations.tsx,
// cast-crew.tsx) and settings/BreakdownCategories.tsx.

import Foundation
import Observation
import OpenFrameCore

// MARK: Availability dates

/// Dates are plain days ("2027-06-20"), never moments, so a time zone cannot move them.
enum AvailabilityDates {
    private static var calendar: Calendar {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = .current
        return c
    }

    static func format(_ date: Date) -> String {
        let p = calendar.dateComponents([.year, .month, .day], from: date)
        return String(format: "%04d-%02d-%02d", p.year ?? 1970, p.month ?? 1, p.day ?? 1)
    }

    static func parse(_ iso: String) -> Date? {
        let parts = iso.split(separator: "-").compactMap { Int($0) }
        guard parts.count == 3 else { return nil }
        return calendar.date(from: DateComponents(year: parts[0], month: parts[1], day: parts[2]))
    }

    static func display(_ iso: String) -> String {
        parse(iso).map { $0.formatted(.dateTime.day().month(.abbreviated).year()) } ?? iso
    }

    /// "20 Jun 2027 to 22 Jun 2027", or one day.
    static func range(_ w: AvailabilityWindowDto) -> String {
        w.startDate == w.endDate ? display(w.startDate) : "\(display(w.startDate)) to \(display(w.endDate))"
    }

    static func input(_ w: AvailabilityWindowDto) -> AvailabilityWindowInput {
        AvailabilityWindowInput(startDate: w.startDate, endDate: w.endDate, kind: w.kind, note: w.note)
    }
}

extension AvailabilityKind {
    var title: String { self == .unavailable ? "Not available" : "Available" }
}

extension BreakdownHub {
    /// Saving replaces the owner's list of windows as one undo step.
    @discardableResult
    func setAvailability(owner: AvailabilityOwner, ownerId: String, windows: [AvailabilityWindowInput], name: String) async -> Bool {
        guard let core else { return false }
        do {
            _ = try await core.availabilitySet(AvailabilitySetArgs(ownerType: owner, ownerId: ownerId, windows: windows))
            say("Updated availability of \(name)", undoable: true)
            return true
        } catch {
            fail("Could not save the dates", error)
            return false
        }
    }

    @discardableResult
    func addAvailability(owner: AvailabilityOwner, ownerId: String, existing: [AvailabilityWindowDto], name: String,
                         start: String, end: String?, kind: AvailabilityKind, note: String?) async -> Bool {
        let trimmed = note?.trimmingCharacters(in: .whitespacesAndNewlines)
        let window = AvailabilityWindowInput(startDate: start, endDate: (end?.isEmpty ?? true) ? start : end!, kind: kind,
                                             note: (trimmed?.isEmpty ?? true) ? nil : trimmed)
        return await setAvailability(owner: owner, ownerId: ownerId, windows: existing.map(AvailabilityDates.input) + [window], name: name)
    }

    @discardableResult
    func removeAvailability(owner: AvailabilityOwner, ownerId: String, existing: [AvailabilityWindowDto], windowId: String, name: String) async -> Bool {
        await setAvailability(owner: owner, ownerId: ownerId, windows: existing.filter { $0.id != windowId }.map(AvailabilityDates.input), name: name)
    }

    func references(kind: ProductionRecordKind, id: String) async -> ProductionReferences? {
        guard let core else { return nil }
        return try? await core.productionReferences(ProductionReferencesArgs(kind: kind, id: id))
    }
}

// MARK: Catalog

enum CatalogSheet: Identifiable {
    case add(BreakdownCategory)
    case archive(CatalogItemDto)
    case replace(CatalogItemDto)
    case delete(CatalogItemDto)

    var id: String {
        switch self {
        case .add: "add"
        case let .archive(i): "archive-\(i.id)"
        case let .replace(i): "replace-\(i.id)"
        case let .delete(i): "delete-\(i.id)"
        }
    }
}

@MainActor
@Observable
final class CatalogModel {
    unowned let hub: BreakdownHub
    private(set) var items: [CatalogItemDto] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    private(set) var fetched: CatalogItemDto?
    var search = ""
    var filter: BreakdownCategory?
    var showArchived = false
    var selectedId: String?
    var sheet: CatalogSheet?
    @ObservationIgnored private var generation = 0

    init(hub: BreakdownHub) { self.hub = hub }

    var canEdit: Bool { hub.canEdit }
    var selected: CatalogItemDto? {
        guard let id = selectedId else { return nil }
        return items.first { $0.id == id } ?? (fetched?.id == id ? fetched : nil)
    }
    var isFiltered: Bool { !search.trimmingCharacters(in: .whitespaces).isEmpty || filter != nil }

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        let q = search.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            let list = try await core.catalogList(CatalogListArgs(category: filter, search: q.isEmpty ? nil : q, includeArchived: showArchived))
            guard gen == generation else { return }
            items = list
            loadFailure = nil
            loaded = true
            if let id = selectedId, !list.contains(where: { $0.id == id }) {
                fetched = try? await core.catalogGet(CatalogIdArgs(id: id))
            }
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    func focus(_ id: String) {
        selectedId = id
        showArchived = true
    }

    @discardableResult
    func create(category: BreakdownCategory, name: String, status: CatalogStatus, description: String?) async -> CatalogItemDto? {
        guard let core = hub.core else { return nil }
        do {
            let d = description?.trimmingCharacters(in: .whitespacesAndNewlines)
            let i = try await core.catalogCreate(CatalogCreateArgs(category: category.rawValue, name: name.trimmingCharacters(in: .whitespacesAndNewlines),
                                                                     status: status, description: (d?.isEmpty ?? true) ? nil : d))
            hub.say("Added \(BDText.quoted(i.name)) to the catalog", undoable: true)
            selectedId = i.id
            sheet = nil
            return i
        } catch {
            hub.fail("Could not add the item", error)
            return nil
        }
    }

    func update(_ id: String, name: String? = nil, status: CatalogStatus? = nil, description: String? = nil, notes: String? = nil, contact: String? = nil) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.catalogUpdate(CatalogUpdateArgs(id: id, name: name, status: status, description: description, notes: notes, contact: contact))
        } catch {
            hub.fail("Could not save the item", error)
        }
    }

    func setArchived(_ i: CatalogItemDto, _ archived: Bool) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.catalogSetArchived(CatalogArchiveArgs(id: i.id, archived: archived))
            hub.say(archived ? "Archived \(BDText.quoted(i.name))" : "Restored \(BDText.quoted(i.name)) from the archive", undoable: true)
            sheet = nil
        } catch {
            hub.fail("Could not change the item", error)
        }
    }

    func delete(_ i: CatalogItemDto) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.catalogDelete(CatalogIdArgs(id: i.id))
            hub.say("Deleted \(BDText.quoted(i.name)). Restore it from Recently Deleted if needed.", undoable: true)
            sheet = nil
            if selectedId == i.id { selectedId = nil }
        } catch {
            hub.fail("Could not delete the item", error)
        }
    }

    func addAlias(_ id: String, _ alias: String) async -> Bool {
        guard let core = hub.core else { return false }
        let a = alias.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !a.isEmpty else { return false }
        do {
            _ = try await core.catalogAddAlias(CatalogAliasArgs(id: id, alias: a))
            return true
        } catch {
            hub.fail("Could not add the name", error)
            return false
        }
    }

    func removeAlias(_ aliasId: String) async {
        guard let core = hub.core else { return }
        do { _ = try await core.catalogRemoveAlias(CatalogIdArgs(id: aliasId)) } catch { hub.fail("Could not remove the name", error) }
    }

    func setImage(_ id: String, path: String?) async {
        guard let core = hub.core else { return }
        do { _ = try await core.catalogSetImage(CatalogImageArgs(id: id, path: path)) } catch { hub.fail("Could not change the image", error) }
    }

    /// Items of the same category that can stand in for `item`.
    func replacementCandidates(for item: CatalogItemDto) async -> [CatalogItemDto] {
        guard let core = hub.core else { return [] }
        let list = (try? await core.catalogList(CatalogListArgs(category: item.category, search: nil, includeArchived: false))) ?? []
        return list.filter { $0.id != item.id }
    }

    @discardableResult
    func replace(_ item: CatalogItemDto, with toId: String, sceneIds: [String]) async -> Bool {
        guard let core = hub.core else { return false }
        do {
            let r = try await core.catalogReplaceInScenes(CatalogReplaceArgs(fromId: item.id, toId: toId, sceneIds: sceneIds))
            hub.say("Replaced in \(BDText.plural(r.scenesUpdated.asInt, "scene")). \(BDText.quoted(item.name)) is kept in the catalog.", undoable: true)
            sheet = nil
            return true
        } catch {
            hub.fail("Could not replace the item", error)
            return false
        }
    }
}

// MARK: Locations

enum LocationSheet: Identifiable {
    case add
    case replace(ProductionLocationDto)
    case delete(ProductionLocationDto)

    var id: String {
        switch self {
        case .add: "add"
        case let .replace(l): "replace-\(l.id)"
        case let .delete(l): "delete-\(l.id)"
        }
    }
}

@MainActor
@Observable
final class LocationsModel {
    unowned let hub: BreakdownHub
    private(set) var locations: [ProductionLocationDto] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    private(set) var fetched: ProductionLocationDto?
    var search = ""
    var status: ProductionLocationStatus?
    var showArchived = false
    var selectedId: String?
    var sheet: LocationSheet?
    @ObservationIgnored private var generation = 0

    init(hub: BreakdownHub) { self.hub = hub }

    var canEdit: Bool { hub.canEdit }
    var selected: ProductionLocationDto? {
        guard let id = selectedId else { return nil }
        return locations.first { $0.id == id } ?? (fetched?.id == id ? fetched : nil)
    }
    var isFiltered: Bool { !search.trimmingCharacters(in: .whitespaces).isEmpty || status != nil }

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        let q = search.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            let list = try await core.locationsList(ProductionLocationListArgs(status: status, search: q.isEmpty ? nil : q, includeArchived: showArchived))
            guard gen == generation else { return }
            locations = list
            loadFailure = nil
            loaded = true
            if let id = selectedId, !list.contains(where: { $0.id == id }) {
                fetched = try? await core.locationsGet(ProductionLocationIdArgs(id: id))
            }
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    @discardableResult
    func create(name: String, address: String?, contact: String?, nearestHospital: String?, status: ProductionLocationStatus) async -> ProductionLocationDto? {
        guard let core = hub.core else { return nil }
        func clean(_ s: String?) -> String? {
            let t = s?.trimmingCharacters(in: .whitespacesAndNewlines)
            return (t?.isEmpty ?? true) ? nil : t
        }
        do {
            let l = try await core.locationsCreate(ProductionLocationCreateArgs(
                name: name.trimmingCharacters(in: .whitespacesAndNewlines), address: clean(address), contact: clean(contact),
                nearestHospital: clean(nearestHospital), emergencyContact: nil, status: status))
            hub.say("Added location \(BDText.quoted(l.name))", undoable: true)
            selectedId = l.id
            sheet = nil
            return l
        } catch {
            hub.fail("Could not add the location", error)
            return nil
        }
    }

    func update(_ id: String, name: String? = nil, address: String? = nil, contact: String? = nil,
                nearestHospital: String? = nil, emergencyContact: String? = nil, notes: ProductionLocationNotes? = nil) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.locationsUpdate(ProductionLocationUpdateArgs(
                id: id, name: name, address: address, contact: contact, nearestHospital: nearestHospital,
                emergencyContact: emergencyContact, notes: notes))
        } catch {
            hub.fail("Could not save the location", error)
        }
    }

    func setStatus(_ l: ProductionLocationDto, _ s: ProductionLocationStatus) async {
        guard let core = hub.core, s != l.status else { return }
        do { _ = try await core.locationsSetStatus(ProductionLocationStatusArgs(id: l.id, status: s)) } catch { hub.fail("Could not change the status", error) }
    }

    func setArchived(_ l: ProductionLocationDto, _ archived: Bool) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.locationsSetArchived(ProductionLocationArchiveArgs(id: l.id, archived: archived))
            hub.say(archived ? "Archived \(BDText.quoted(l.name))" : "Restored \(BDText.quoted(l.name)) from the archive", undoable: true)
        } catch {
            hub.fail("Could not change the location", error)
        }
    }

    func delete(_ l: ProductionLocationDto) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.locationsDelete(ProductionLocationIdArgs(id: l.id))
            hub.say("Deleted \(BDText.quoted(l.name)). Restore it from Recently Deleted if needed.", undoable: true)
            sheet = nil
            if selectedId == l.id { selectedId = nil }
        } catch {
            hub.fail("Could not delete the location", error)
        }
    }

    func addPhotos(_ id: String, paths: [String]) async {
        guard let core = hub.core, !paths.isEmpty else { return }
        do { _ = try await core.locationsAddPhotos(ProductionLocationPhotosArgs(id: id, paths: paths)) } catch { hub.fail("Could not add the photos", error) }
    }

    func movePhoto(_ photoId: String, to index: Int) async {
        guard let core = hub.core else { return }
        do { _ = try await core.locationsMovePhoto(ProductionLocationPhotoMoveArgs(photoId: photoId, index: Double(index))) } catch { hub.fail("Could not move the photo", error) }
    }

    func removePhoto(_ photoId: String) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.locationsRemovePhoto(ProductionLocationIdArgs(id: photoId))
            hub.say("Photo removed", undoable: true)
        } catch {
            hub.fail("Could not remove the photo", error)
        }
    }

    func replacementCandidates(for l: ProductionLocationDto) async -> [ProductionLocationDto] {
        guard let core = hub.core else { return [] }
        let list = (try? await core.locationsList(ProductionLocationListArgs(status: nil, search: nil, includeArchived: false))) ?? []
        return list.filter { $0.id != l.id && $0.status != .rejected }
    }

    @discardableResult
    func replace(_ l: ProductionLocationDto, with id: String, sceneIds: [String]) async -> Bool {
        guard let core = hub.core else { return false }
        do {
            let r = try await core.locationsReplace(ProductionLocationReplaceArgs(locationId: l.id, replacementId: id, sceneIds: sceneIds))
            hub.say("Replacement applied to \(BDText.plural(r.scenesUpdated.asInt, "scene")). The screenplay was not changed.", undoable: true)
            sheet = nil
            return true
        } catch {
            hub.fail("Could not replace the location", error)
            return false
        }
    }
}

// MARK: Cast and crew

enum PeopleView: String, CaseIterable, Identifiable {
    case cast, crew
    var id: String { rawValue }
    var title: String { self == .cast ? "Cast" : "Crew" }
}

enum PeopleSheet: Identifiable {
    case assign(member: ProductionCastMemberDto?, character: String?)
    case addCrew
    case delete(kind: PeopleView, id: String, name: String)

    var id: String {
        switch self {
        case let .assign(m, c): "assign-\(m?.id ?? "new")-\(c ?? "")"
        case .addCrew: "add-crew"
        case let .delete(k, id, _): "delete-\(k.rawValue)-\(id)"
        }
    }
}

enum Departments {
    static let all = ["Direction", "Camera", "Sound", "Art", "Costume", "Makeup", "Production", "Editing"]
}

@MainActor
@Observable
final class PeopleModel {
    unowned let hub: BreakdownHub
    private(set) var directory: ProductionCastDirectory?
    private(set) var crew: [ProductionCrewMemberDto] = []
    private(set) var loaded = false
    private(set) var loadFailure: CoreFailure?
    var view: PeopleView = .cast
    var search = ""
    var showArchived = false
    var selectedCast: String?
    var selectedCrew: String?
    var sheet: PeopleSheet?
    @ObservationIgnored private var generation = 0

    init(hub: BreakdownHub) { self.hub = hub }

    var canEdit: Bool { hub.canEdit }
    private var query: String { search.trimmingCharacters(in: .whitespaces).lowercased() }

    var members: [ProductionCastMemberDto] {
        (directory?.members ?? []).filter { m in
            (showArchived || !m.archived || m.id == selectedCast)
                && (query.isEmpty || "\(m.personName) \(m.characterName ?? "") \(m.availabilityNotes ?? "")".lowercased().contains(query))
        }
    }

    /// Characters nobody plays yet.
    var unassigned: [ProductionCastCharacterDto] {
        (directory?.characters ?? []).filter { $0.primaryCastId == nil && (query.isEmpty || $0.name.lowercased().contains(query)) }
    }

    var crewList: [ProductionCrewMemberDto] {
        crew.filter { c in
            (showArchived || !c.archived || c.id == selectedCrew)
                && (query.isEmpty || "\(c.personName) \(c.role) \(c.department ?? "") \(c.availabilityNotes ?? "")".lowercased().contains(query))
        }
    }

    var castMember: ProductionCastMemberDto? { directory?.members.first { $0.id == selectedCast } }
    var crewMember: ProductionCrewMemberDto? { crew.first { $0.id == selectedCrew } }

    func reload() async {
        guard let core = hub.core else { return }
        generation += 1
        let gen = generation
        do {
            let withCast = showArchived || selectedCast != nil
            let withCrew = showArchived || selectedCrew != nil
            async let dir = core.castList(ProductionPeopleListArgs(includeArchived: withCast))
            async let crewRows = core.crewList(ProductionPeopleListArgs(includeArchived: withCrew))
            let (d, c) = try await (dir, crewRows)
            guard gen == generation else { return }
            directory = d
            crew = c
            loadFailure = nil
            loaded = true
        } catch {
            guard gen == generation else { return }
            loadFailure = CoreFailure(asCore: error)
            loaded = true
        }
    }

    func summary(of m: ProductionCastMemberDto) -> String {
        Self.availabilityText(notes: m.availabilityNotes, dates: m.availability.count)
    }

    /// "Free in June · 2 date ranges".
    static func availabilityText(notes: String?, dates: Int) -> String {
        [notes ?? "", dates > 0 ? BDText.plural(dates, "date range") : ""].filter { !$0.isEmpty }.joined(separator: " \u{00B7} ")
    }

    // MARK: Cast

    @discardableResult
    func createCast(person: String, characterName: String?, characterId: String?, primary: Bool) async -> ProductionCastMemberDto? {
        guard let core = hub.core else { return nil }
        do {
            let m = try await core.castCreate(ProductionCastCreateArgs(
                personName: person.trimmingCharacters(in: .whitespacesAndNewlines),
                characterId: characterId, characterName: characterId == nil ? characterName : nil, primary: primary))
            hub.say("\(m.personName) plays \(m.characterName ?? "")", undoable: true)
            sheet = nil
            return m
        } catch {
            hub.fail("Could not add the person", error)
            return nil
        }
    }

    @discardableResult
    func assign(_ member: ProductionCastMemberDto, characterName: String?, characterId: String?, primary: Bool) async -> Bool {
        guard let core = hub.core else { return false }
        do {
            let m = try await core.castAssign(ProductionCastAssignArgs(id: member.id, characterId: characterId,
                                                                        characterName: characterId == nil ? characterName : nil, primary: primary))
            hub.say("\(m.personName) plays \(m.characterName ?? "")", undoable: true)
            sheet = nil
            return true
        } catch {
            hub.fail("Could not assign the character", error)
            return false
        }
    }

    func updateCast(_ id: String, personName: String? = nil, contact: String? = nil, availabilityNotes: String? = nil, notes: String? = nil) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.castUpdate(ProductionCastUpdateArgs(id: id, personName: personName, contact: contact, availabilityNotes: availabilityNotes, notes: notes))
        } catch {
            hub.fail("Could not save the person", error)
        }
    }

    func setCastPhoto(_ id: String, path: String?) async {
        guard let core = hub.core else { return }
        do { _ = try await core.castSetPhoto(ProductionCastPhotoArgs(id: id, path: path)) } catch { hub.fail("Could not change the photo", error) }
    }

    // MARK: Crew

    @discardableResult
    func createCrew(person: String, role: String, department: String?, contact: String?, notes: String?) async -> ProductionCrewMemberDto? {
        guard let core = hub.core else { return nil }
        func clean(_ s: String?) -> String? {
            let t = s?.trimmingCharacters(in: .whitespacesAndNewlines)
            return (t?.isEmpty ?? true) ? nil : t
        }
        do {
            let c = try await core.crewCreate(ProductionCrewCreateArgs(
                personName: person.trimmingCharacters(in: .whitespacesAndNewlines), role: role.trimmingCharacters(in: .whitespacesAndNewlines),
                department: clean(department), contact: clean(contact), notes: clean(notes)))
            hub.say("Added \(c.personName) to the crew", undoable: true)
            selectedCrew = c.id
            sheet = nil
            return c
        } catch {
            hub.fail("Could not add the person", error)
            return nil
        }
    }

    func updateCrew(_ id: String, personName: String? = nil, role: String? = nil, department: String? = nil,
                    contact: String? = nil, availabilityNotes: String? = nil, notes: String? = nil) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.crewUpdate(ProductionCrewUpdateArgs(id: id, personName: personName, role: role, department: department,
                                                                    contact: contact, availabilityNotes: availabilityNotes, notes: notes))
        } catch {
            hub.fail("Could not save the person", error)
        }
    }

    // MARK: Both

    func setArchived(kind: PeopleView, id: String, archived: Bool, name: String) async {
        guard let core = hub.core else { return }
        do {
            if kind == .cast {
                _ = try await core.castSetArchived(ProductionPeopleArchiveArgs(id: id, archived: archived))
            } else {
                _ = try await core.crewSetArchived(ProductionPeopleArchiveArgs(id: id, archived: archived))
            }
            hub.say(archived ? "Archived \(name)" : "Restored \(name) from the archive", undoable: true)
        } catch {
            hub.fail("Could not change the person", error)
        }
    }

    func delete(kind: PeopleView, id: String, name: String) async {
        guard let core = hub.core else { return }
        do {
            if kind == .cast {
                _ = try await core.castDelete(ProductionPeopleIdArgs(id: id))
                if selectedCast == id { selectedCast = nil }
            } else {
                _ = try await core.crewDelete(ProductionPeopleIdArgs(id: id))
                if selectedCrew == id { selectedCrew = nil }
            }
            hub.say("Removed \(name). Restore from Recently Deleted if needed.", undoable: true)
            sheet = nil
        } catch {
            hub.fail("Could not remove the person", error)
        }
    }
}

// MARK: Categories

@MainActor
@Observable
final class CategoryStylesModel {
    unowned let hub: BreakdownHub
    init(hub: BreakdownHub) { self.hub = hub }

    var styles: BreakdownCategoryStyles? { hub.styles }
    var canEdit: Bool { hub.canEdit }
    var anyCustomized: Bool { styles?.categories.contains { $0.customized } ?? false }

    func reload() async { await hub.reloadStyles() }

    func setColor(_ category: BreakdownCategory, token: String) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.breakdownSetCategoryStyle(BreakdownCategoryStyleArgs(category: category.rawValue, color: token, icon: nil))
            hub.say("Changed the look of \(category.rawValue)", undoable: true)
            await hub.reloadStyles()
        } catch {
            hub.fail("Could not change the colour", error)
        }
    }

    func setIcon(_ category: BreakdownCategory, key: String) async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.breakdownSetCategoryStyle(BreakdownCategoryStyleArgs(category: category.rawValue, color: nil, icon: key))
            hub.say("Changed the look of \(category.rawValue)", undoable: true)
            await hub.reloadStyles()
        } catch {
            hub.fail("Could not change the icon", error)
        }
    }

    func reset() async {
        guard let core = hub.core else { return }
        do {
            _ = try await core.breakdownResetCategoryStyles()
            hub.say("Restored the default category colours and icons", undoable: true)
            await hub.reloadStyles()
        } catch {
            hub.fail("Could not restore the defaults", error)
        }
    }
}

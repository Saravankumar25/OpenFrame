// Small views shared by the Schedule screens: the tab buttons, a field that saves when you leave
// it, the stripboard strip and break rows, and the menus that do what dragging does. Every control
// has an identifier and, when it is only an icon, a spoken name.

import AppKit
import SwiftUI
import OpenFrameCore

// MARK: Tabs

/// Segmented buttons with an identifier on each segment (a native picker hides them).
struct SchedTabs<Value: Hashable>: View {
    struct Item: Identifiable {
        let value: Value
        let title: String
        var symbol: String?
        let id: String
    }
    let items: [Item]
    @Binding var selection: Value
    let label: String
    let identifier: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: 2) {
            ForEach(items) { item in
                let on = item.value == selection
                Button {
                    selection = item.value
                } label: {
                    HStack(spacing: 4) {
                        if let s = item.symbol { Image(systemName: s).imageScale(.small).accessibilityHidden(true) }
                        Text(item.title)
                    }
                    .font(.callout.weight(on ? .semibold : .regular))
                    .padding(.horizontal, 10).padding(.vertical, 4)
                    .foregroundStyle(on ? theme["accent-ink"] : theme["ink2"])
                    .background(on ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .contentShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
                .accessibilityIdentifier(item.id)
            }
        }
        .padding(2)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(label)
        .accessibilityIdentifier(identifier)
    }
}

// MARK: Fields

/// A text field that keeps a local copy while you type and saves when you leave it or press
/// Return. The save may answer with a line to show under the field. Escape puts the saved text back.
struct SchedCommitField: View {
    let id: String
    let label: String
    let value: String
    var placeholder = ""
    var hint: String?
    var disabled = false
    var showsLabel = true
    var monospaced = false
    let commit: (String) async -> String?

    @State private var draft = ""
    @State private var error: String?
    @FocusState private var focused: Bool
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            if showsLabel { Text(label).font(.caption.weight(.semibold)).foregroundStyle(theme["muted"]) }
            TextField(placeholder, text: $draft)
                .textFieldStyle(.roundedBorder)
                .font(monospaced ? .body.monospacedDigit() : .body)
                .focused($focused)
                .disabled(disabled)
                .onSubmit { Task { await save() } }
                .onExitCommand {
                    draft = value
                    error = nil
                    focused = false
                }
                .accessibilityLabel(label)
                .accessibilityIdentifier(id)
            if let error {
                Text(error).font(.caption).foregroundStyle(theme["red-t"]).fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("\(id)-error")
            } else if let hint {
                Text(hint).font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
            }
        }
        .onAppear { draft = value }
        .onChange(of: value) { _, new in if !focused { draft = new } }
        .onChange(of: focused) { _, now in if !now { Task { await save() } } }
    }

    private func save() async {
        let text = draft.trimmingCharacters(in: .whitespaces)
        if text == value.trimmingCharacters(in: .whitespaces) {
            error = nil
            return
        }
        error = await commit(text)
    }
}

/// A multi-line field that saves when you leave it.
struct SchedNotesField: View {
    let id: String
    let label: String
    let value: String
    var disabled = false
    let commit: (String) async -> Void

    @State private var draft = ""
    @FocusState private var focused: Bool
    @Environment(\.ofTheme) private var theme

    var body: some View {
        TextEditor(text: $draft)
            .font(.body)
            .scrollContentBackground(.hidden)
            .padding(4)
            .frame(minHeight: 84)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).strokeBorder(theme["line"]))
            .focused($focused)
            .disabled(disabled)
            .accessibilityLabel(label)
            .accessibilityIdentifier(id)
            .onAppear { draft = value }
            .onChange(of: value) { _, new in if !focused { draft = new } }
            .onChange(of: focused) { _, now in
                if !now, draft.trimmingCharacters(in: .whitespacesAndNewlines) != value { Task { await commit(draft) } }
            }
    }
}

// MARK: Strips

enum SchedStripColors {
    /// The stripboard convention: INT day white, EXT day yellow, INT night blue, EXT night green.
    static func background(_ cls: String) -> Color {
        switch cls {
        case "xd": Color(red: 1.0, green: 0.96, blue: 0.62)
        case "xn": Color(red: 0.74, green: 0.91, blue: 0.74)
        case "in": Color(red: 0.75, green: 0.85, blue: 1.0)
        default: Color.white
        }
    }
}

private let stripInk = Color(red: 0.1, green: 0.1, blue: 0.1)

/// One scene on the board, in the list of a day, or in the pool. Click opens its details; drag moves
/// it; the menu and Option+Up and Option+Down do the same without a mouse.
struct SchedStripRow: View {
    let strip: ScheduleStripDto
    var model: ScheduleModel?
    var dim = false
    var warn = false
    var selected = false
    var compact = false
    var onOpen: (() -> Void)?

    private var key: String { strip.number.map { String(Int($0)) } ?? strip.id }

    private var numberText: String { strip.number.map { String(Int($0)) } ?? "-" }
    private var estimateText: String { strip.estimatedMinutes.map { SchedFormat.minutes($0) } ?? "" }
    private var pagesText: String { SchedFormat.pages(strip.pageEighths) + (strip.pagesOverridden ? "*" : "") }
    private var spokenLabel: String {
        dim ? SchedFormat.stripLabel(strip) + ", does not match the filter" : SchedFormat.stripLabel(strip)
    }
    private var outline: Color { selected ? Color.accentColor : Color.black.opacity(0.18) }
    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: 5, style: .continuous) }

    private var leading: some View {
        HStack(spacing: 8) {
            Text(numberText).font(.callout.weight(.bold)).frame(minWidth: 22, alignment: .leading)
            Text(strip.ieLabel).font(.caption.weight(.semibold)).monospaced().fixedSize()
            if let loc = strip.locationName {
                Text(loc).font(.callout.weight(.medium)).lineLimit(1).layoutPriority(1)
            } else {
                Text("No location").font(.callout.italic()).foregroundStyle(Color(red: 0.6, green: 0.1, blue: 0.1)).lineLimit(1)
            }
            if !compact {
                Text(strip.synopsis.isEmpty ? strip.heading : strip.synopsis).font(.caption).lineLimit(1).opacity(0.75)
            }
        }
    }

    private var trailing: some View {
        HStack(spacing: 8) {
            if let word = SchedFormat.sourceChip(strip) {
                Text(word).font(.caption2.weight(.semibold)).padding(.horizontal, 6).padding(.vertical, 1)
                    .background(Color.white.opacity(0.7), in: Capsule())
                    .accessibilityIdentifier("strip-\(key)-source-state")
            }
            if warn {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Color(red: 0.75, green: 0.25, blue: 0.1))
                    .imageScale(.small).help("This scene has an open warning").accessibilityHidden(true)
            }
            castBadges
            if let t = strip.startLabel {
                Text(t).font(.caption.weight(.bold).monospacedDigit()).help("Start time entered for this scene")
            }
            Text(estimateText).font(.caption.monospacedDigit()).frame(minWidth: 34, alignment: .trailing).help("Estimated shooting time")
            Text(pagesText).font(.caption.monospacedDigit()).frame(minWidth: 30, alignment: .trailing)
                .help(strip.pagesOverridden ? "Page count entered manually" : "Page count from the screenplay")
            if let model { SchedStripMenu(model: model, strip: strip) }
        }
    }

    private var card: some View {
        HStack(spacing: 8) {
            leading
            Spacer(minLength: 4)
            trailing
        }
        .foregroundStyle(stripInk)
        .padding(.horizontal, 8).padding(.vertical, 6)
        .background(SchedStripColors.background(strip.stripClass), in: shape)
        .overlay(shape.strokeBorder(outline, lineWidth: selected ? 2 : 1))
        .opacity(dim ? 0.32 : (strip.sourceState == "Removed" ? 0.72 : 1))
        .strikethrough(strip.sourceState == "Removed", color: Color(red: 0.6, green: 0.18, blue: 0.15))
        .contentShape(Rectangle())
    }

    var body: some View {
        card
            .onTapGesture { onOpen?() }
            .accessibilityElement(children: .contain)
            .accessibilityLabel(spokenLabel)
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { onOpen?() }
            .accessibilityIdentifier("strip-\(key)")
            .contextMenu { if let model { SchedStripMenuItems(model: model, strip: strip) } }
            .focusable()
            .onKeyPress(.return) {
                onOpen?()
                return onOpen == nil ? .ignored : .handled
            }
            .onKeyPress(keys: [.upArrow, .downArrow]) { press in
                moveByKey(press.key == .upArrow ? -1 : 1, press)
            }
    }

    private func castHelp(_ c: ScheduleCastRef) -> String {
        if let a = c.actor { return a + " (" + c.character + ")" }
        return c.character
    }

    private var castBadges: some View {
        HStack(spacing: -4) {
            ForEach(Array(strip.cast.prefix(4)), id: \.key) { c in
                Text(c.initials).font(.system(size: 9, weight: .bold))
                    .frame(width: 18, height: 18).background(Color.white.opacity(0.8), in: Circle())
                    .overlay(Circle().strokeBorder(Color.black.opacity(0.2)))
                    .help(castHelp(c))
            }
            if strip.cast.count > 4 { Text("+\(strip.cast.count - 4)").font(.system(size: 9, weight: .bold)).padding(.leading, 6) }
        }
        .accessibilityHidden(true)
    }

    /// Option+Up and Option+Down move the scene within its day.
    private func moveByKey(_ delta: Int, _ press: KeyPress) -> KeyPress.Result {
        guard let model, strip.dayId != nil, press.modifiers.contains(.option) else { return .ignored }
        Task { await model.moveStripBy(strip, delta) }
        return .handled
    }
}

/// The menu of a scene: details, move to a day, move up and down, take it off the day.
struct SchedStripMenu: View {
    let model: ScheduleModel
    let strip: ScheduleStripDto

    var body: some View {
        Menu {
            SchedStripMenuItems(model: model, strip: strip)
        } label: {
            Label("Actions", systemImage: "ellipsis")
        }
        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
        .accessibilityLabel("Scene \(strip.number.map { String(Int($0)) } ?? "") actions")
        .accessibilityIdentifier("strip-actions-\(strip.number.map { String(Int($0)) } ?? strip.id)")
    }
}

struct SchedStripMenuItems: View {
    let model: ScheduleModel
    let strip: ScheduleStripDto

    var body: some View {
        let menu = model.moveMenu(for: strip)
        let key = strip.number.map { String(Int($0)) } ?? strip.id
        Button("Details\u{2026}") { model.drawer = .strip(strip.id) }
            .accessibilityIdentifier("strip-menu-details-\(key)")
        Divider()
        Menu("Move to") {
            if menu.days.isEmpty { Text("Create a shooting day first") }
            ForEach(menu.days) { c in
                Button(c.day.title) { Task { await model.moveStripToDay(strip, c.day) } }
                    .disabled(c.disabled || model.isFinalized)
                    .accessibilityIdentifier("strip-menu-move-to-\(key)-\(c.day.number.map { String(Int($0)) } ?? c.day.id)")
            }
        }
        .accessibilityIdentifier("strip-menu-move-to-\(key)")
        if menu.isScheduled {
            Button("Move Up") { Task { await model.moveStripBy(strip, -1) } }
                .disabled(!menu.canMoveUp || model.isFinalized)
                .accessibilityIdentifier("strip-menu-move-up-\(key)")
            Button("Move Down") { Task { await model.moveStripBy(strip, 1) } }
                .disabled(!menu.canMoveDown || model.isFinalized)
                .accessibilityIdentifier("strip-menu-move-down-\(key)")
            Button("Remove from Day") { Task { await model.removeFromDay(strip) } }
                .disabled(model.isFinalized)
                .accessibilityIdentifier("strip-menu-remove-\(key)")
        }
    }
}

// MARK: Breaks

/// A break or note between scenes.
struct SchedMarkerRow: View {
    let marker: ScheduleMarkerDto
    var model: ScheduleModel?
    var onOpen: (() -> Void)?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack {
            Spacer(minLength: 0)
            Text(SchedFormat.markerText(marker)).textCase(.uppercase).font(.callout.weight(.semibold)).lineLimit(1)
            Spacer(minLength: 0)
            if let model {
                Menu {
                    SchedMarkerMenuItems(model: model, marker: marker)
                } label: {
                    Label("Actions", systemImage: "ellipsis")
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .accessibilityLabel("Break actions")
                .accessibilityIdentifier("marker-actions-\(marker.id)")
            }
        }
        .foregroundStyle(theme["ink2"])
        .padding(.horizontal, 8).padding(.vertical, 5)
        .background(theme["surface3"], in: RoundedRectangle(cornerRadius: 5, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 5, style: .continuous).strokeBorder(theme["line2"], style: StrokeStyle(lineWidth: 1, dash: [4, 3])))
        .contentShape(Rectangle())
        .onTapGesture { onOpen?() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(marker.markerType == "Custom" ? "Note" : "Break"): \(SchedFormat.markerText(marker))")
        .accessibilityAddTraits(.isButton)
        .accessibilityAction { onOpen?() }
        .accessibilityIdentifier("marker-\(marker.id)")
        .contextMenu { if let model { SchedMarkerMenuItems(model: model, marker: marker) } }
    }
}

struct SchedMarkerMenuItems: View {
    let model: ScheduleModel
    let marker: ScheduleMarkerDto

    var body: some View {
        let day = model.day(marker.dayId)
        let order = day?.itemIds ?? []
        let idx = order.firstIndex(of: marker.id)
        Button("Edit Break\u{2026}") { model.sheet = .marker(SchedMarkerDialogMode(dayId: marker.dayId, type: marker.markerType, marker: marker)) }
            .accessibilityIdentifier("marker-menu-edit-\(marker.id)")
        let canUp: Bool = (idx ?? 0) > 0
        Button("Move Up") { Task { await model.moveMarkerBy(marker, -1) } }
            .disabled(!canUp || model.isFinalized)
            .accessibilityIdentifier("marker-menu-move-up-\(marker.id)")
        let canDown: Bool = idx != nil && idx! < order.count - 1
        Button("Move Down") { Task { await model.moveMarkerBy(marker, 1) } }
            .disabled(!canDown || model.isFinalized)
            .accessibilityIdentifier("marker-menu-move-down-\(marker.id)")
        Divider()
        Button("Delete Break", role: .destructive) { Task { await model.deleteMarker(marker) } }
            .accessibilityIdentifier("marker-menu-delete-\(marker.id)")
    }
}

// MARK: Day menu

/// The menu of a shooting day: details, date, breaks, off day, insert, duplicate, call sheet, delete.
struct SchedDayMenuItems: View {
    let model: ScheduleModel
    let day: ScheduleShootDayDto
    var detail = false

    private var key: String { day.number.map { String(Int($0)) } ?? day.id }

    var body: some View {
        let hasScenes = day.summary.sceneCount > 0
        let index = model.days.firstIndex { $0.id == day.id } ?? 0
        if !detail {
            Button("Open day details") { model.openDayId = day.id }.accessibilityIdentifier("day-menu-open-\(key)")
        }
        Button("Edit date") { model.sheet = .day(.date(day)) }.accessibilityIdentifier("day-menu-edit-date-\(key)")
        Divider()
        Menu("Add to this day") {
            Button("Meal Break") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Meal")) }
                .accessibilityIdentifier("day-menu-meal-\(key)")
            Button("Travel") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Travel")) }
                .accessibilityIdentifier("day-menu-travel-\(key)")
            Button("Company Move") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Company Move")) }
                .accessibilityIdentifier("day-menu-company-move-\(key)")
            Button("Custom Note") { model.sheet = .marker(SchedMarkerDialogMode(dayId: day.id, type: "Custom")) }
                .accessibilityIdentifier("day-menu-custom-\(key)")
        }
        .accessibilityIdentifier("day-menu-add-\(key)")
        Divider()
        Button(day.isOffDay ? "Set as Shooting Day" : "Set as Off Day") { Task { await model.setOffDay(day, !day.isOffDay) } }
            .disabled((!day.isOffDay && hasScenes) || model.isFinalized)
            .accessibilityIdentifier("day-menu-off-\(key)")
        if !detail {
            Button("Add Day Note") { model.openDayId = day.id }.accessibilityIdentifier("day-menu-note-\(key)")
        }
        Button("Insert day after") { model.sheet = .day(.create(offDay: false, afterDayId: day.id)) }
            .disabled(model.isFinalized).accessibilityIdentifier("day-menu-insert-\(key)")
        Button("Duplicate day (breaks and notes only)") { Task { await model.duplicateDay(day) } }
            .disabled(model.isFinalized).accessibilityIdentifier("day-menu-duplicate-\(key)")
        Button("Move Day Earlier") { Task { await model.moveDay(day, by: -1) } }
            .disabled(index == 0 || model.isFinalized).accessibilityIdentifier("day-menu-earlier-\(key)")
        Button("Move Day Later") { Task { await model.moveDay(day, by: 1) } }
            .disabled(index >= model.days.count - 1 || model.isFinalized).accessibilityIdentifier("day-menu-later-\(key)")
        if !day.isOffDay {
            Divider()
            Button(day.callSheet != nil ? "Open Call Sheet" : "Create Call Sheet") { Task { await model.openCallSheet(day) } }
                .accessibilityIdentifier("day-menu-call-sheet-\(key)")
        }
        Divider()
        Button("Delete day", role: .destructive) { Task { await model.deleteDay(day) } }
            .disabled(model.isFinalized).accessibilityIdentifier("day-menu-delete-\(key)")
    }
}

struct SchedDayMenu: View {
    let model: ScheduleModel
    let day: ScheduleShootDayDto
    var detail = false

    var body: some View {
        Menu {
            SchedDayMenuItems(model: model, day: day, detail: detail)
        } label: {
            Label("Actions", systemImage: "ellipsis.circle")
        }
        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
        .accessibilityLabel(detail ? "More day actions" : "\(day.label) actions")
        .accessibilityIdentifier(detail ? "day-more-actions" : "day-actions-\(day.number.map { String(Int($0)) } ?? day.id)")
    }
}

// MARK: Chips

struct SchedChip: View {
    let text: String
    var tone: BDTone = .plain
    var symbol: String?
    var body: some View { BDChip(text: text, tone: tone, symbol: symbol) }
}

extension ScheduleModel {
    /// Moves a break one place up or down in its day.
    func moveMarkerBy(_ m: ScheduleMarkerDto, _ delta: Int) async {
        guard let d = day(m.dayId), let idx = d.itemIds.firstIndex(of: m.id) else { return }
        let to = idx + delta
        guard to >= 0, to < d.itemIds.count else { return }
        await moveMarker(m.id, to: d.id, index: to, dayLabel: d.label)
    }
}

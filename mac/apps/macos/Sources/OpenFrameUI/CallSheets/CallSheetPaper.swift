// The paper of the call sheet editor (Windows CallSheetEditor.tsx sections). The six default
// sections are always shown; the rest come in through Add Section and stay hidden while they are
// not added. Editable values use a dashed outline; a value nobody has entered reads "Needs input"
// in red. The paper keeps printed-document colours (white page, black text) in every theme, the
// way a printed call sheet looks.

import AppKit
import SwiftUI
import OpenFrameCore

// Printed-document colours (not theme colours).
private let paperInk = Color.black
private let paperEdit = Color(red: 0.84, green: 0.62, blue: 0.20)
private let paperNeedFill = Color(red: 0.99, green: 0.89, blue: 0.88)
private let paperNeedLine = Color(red: 0.84, green: 0.45, blue: 0.42)
private let paperNeedInk = Color(red: 0.56, green: 0.17, blue: 0.13)
private let paperBar = Color(red: 0.16, green: 0.17, blue: 0.20)

func csSlug(_ s: String) -> String {
    let lower = s.lowercased()
    var out = ""
    var dash = false
    for ch in lower {
        if ch.isLetter || ch.isNumber { out.append(ch); dash = false } else if !dash, !out.isEmpty { out.append("-"); dash = true }
    }
    while out.hasSuffix("-") { out.removeLast() }
    return out
}

extension CallSheetEditorModel {
    func bind(_ get: @escaping (CallSheetDocument) -> String, _ set: @escaping (inout CallSheetDocument, String) -> Void) -> Binding<String> {
        Binding(
            get: { self.doc.map(get) ?? "" },
            set: { v in self.edit { set(&$0, v) } }
        )
    }

    func bind(_ kp: WritableKeyPath<CallSheetDocument, String>) -> Binding<String> {
        bind({ $0[keyPath: kp] }, { $0[keyPath: kp] = $1 })
    }
}

// MARK: Fields

/// One editable value: dashed outline, red when it is required and empty.
struct CSEdit: View {
    let label: String
    @Binding var text: String
    var need = false
    var placeholder: String?
    var disabled = false
    var wide = false
    var minWidth: CGFloat = 70
    var invalid = false

    /// What an empty field shows: "Needs input" when a value is required, else the hint.
    static func prompt(text: String, need: Bool, placeholder: String?) -> String {
        need && text.trimmingCharacters(in: .whitespaces).isEmpty ? csNeedsInput : (placeholder ?? "")
    }

    var body: some View {
        let missing = (need && text.trimmingCharacters(in: .whitespaces).isEmpty) || invalid
        let prompt = Self.prompt(text: text, need: need, placeholder: placeholder)
        TextField(label, text: $text, prompt: Text(prompt).foregroundStyle(missing ? paperNeedInk : Color.gray))
            .textFieldStyle(.plain)
            .foregroundStyle(missing ? paperNeedInk : paperInk)
            .padding(.horizontal, 4).padding(.vertical, 1)
            .frame(minWidth: minWidth, maxWidth: wide ? .infinity : nil, alignment: .leading)
            .background(missing && !disabled ? paperNeedFill : Color.clear)
            .overlay {
                if !disabled {
                    RoundedRectangle(cornerRadius: 2)
                        .strokeBorder(missing ? paperNeedLine : paperEdit, style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                }
            }
            .disabled(disabled)
            .accessibilityLabel(label)
            .accessibilityIdentifier("callsheet-field-\(csSlug(label))")
    }
}

struct CSArea: View {
    let label: String
    @Binding var text: String
    var disabled = false

    var body: some View {
        TextField(label, text: $text, axis: .vertical)
            .lineLimit(2...6)
            .textFieldStyle(.plain)
            .foregroundStyle(paperInk)
            .padding(4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay {
                if !disabled {
                    RoundedRectangle(cornerRadius: 2).strokeBorder(paperEdit, style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                }
            }
            .disabled(disabled)
            .accessibilityLabel(label)
            .accessibilityIdentifier("callsheet-field-\(csSlug(label))")
    }
}

/// A label and a one-line editable value.
struct CSLabeled: View {
    let label: String
    @Binding var text: String
    var need = false
    var disabled = false

    var body: some View {
        HStack(spacing: 6) {
            Text(label).font(.system(size: 11.5, weight: .semibold)).foregroundStyle(paperInk)
                .frame(minWidth: 128, alignment: .leading)
            CSEdit(label: label, text: $text, need: need, disabled: disabled, wide: true)
        }
        .padding(.vertical, 1.5)
    }
}

struct CSPaperSection<Content: View>: View {
    let title: String
    var onRemove: (() -> Void)?
    var disabled = false
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title.uppercased()).font(.system(size: 11, weight: .bold)).tracking(0.6)
                Spacer()
                if let onRemove, !disabled {
                    Button(action: onRemove) { Label("Remove", systemImage: "xmark").font(.caption2) }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Remove \(title)")
                        .accessibilityIdentifier("callsheet-remove-section-\(csSlug(title))")
                }
            }
            .foregroundStyle(Color.white)
            .padding(.horizontal, 8).padding(.vertical, 3)
            .background(paperBar)
            content().padding(.horizontal, 4).padding(.bottom, 6)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier("callsheet-section-\(csSlug(title))")
    }
}

struct CSRemoveRow: View {
    let label: String
    let action: () -> Void
    var body: some View {
        Button(action: action) { Label("Remove", systemImage: "xmark.circle").font(.caption2).foregroundStyle(paperInk.opacity(0.6)) }
            .buttonStyle(.plain)
            .accessibilityLabel(label)
            .accessibilityIdentifier("callsheet-\(csSlug(label))")
    }
}

// MARK: The paper

struct CallSheetPaper: View {
    @Bindable var model: CallSheetEditorModel
    @FocusState private var crewFocused: Bool

    var body: some View {
        if let doc = model.doc, let d = model.detail {
            let ro = !model.editable
            VStack(alignment: .leading, spacing: 10) {
                header(doc, d, ro)
                times(doc, ro)
                locations(doc, ro)
                scenes(doc, d, ro)
                talent(doc, ro)
                practical(doc, ro)
                optionalSections(doc, ro)
            }
            .padding(22)
            .frame(maxWidth: 860, alignment: .leading)
            .background(Color.white)
            .foregroundStyle(paperInk)
            .environment(\.colorScheme, .light)
            .font(.system(size: 12))
            .shadow(color: .black.opacity(0.12), radius: 4, y: 1)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Call sheet document")
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("callsheet-document")
            .onChange(of: model.focusCrewToken) { _, _ in crewFocused = true }
        }
    }

    // MARK: Production header

    private func header(_ doc: CallSheetDocument, _ d: CallSheetDetail, _ ro: Bool) -> some View {
        CSPaperSection(title: "Production header") {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Spacer()
                    CSEdit(label: "Production title", text: model.bind(\.title), need: true, disabled: ro, minWidth: 220)
                        .font(.system(size: 20, weight: .bold))
                    Spacer()
                }
                HStack(spacing: 4) {
                    Spacer()
                    Text(doc.dayLabel + (doc.dayOf.isEmpty ? "" : " (\(doc.dayOf))") + " \u{00B7} ").fontWeight(.bold)
                    if let long = d.dateLong {
                        Text(long).fontWeight(.bold)
                    } else {
                        Text("\(csNeedsInput): set the date in the schedule").foregroundStyle(paperNeedInk).fontWeight(.bold)
                    }
                    Spacer()
                }
                .accessibilityIdentifier("callsheet-day-line")
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 16), GridItem(.flexible())], alignment: .leading, spacing: 0) {
                    CSLabeled(label: "Production company", text: model.bind(\.header.company), disabled: ro)
                    CSLabeled(label: "Production office address", text: model.bind(\.header.address), disabled: ro)
                    CSLabeled(label: "Producer", text: model.bind(\.header.producer), disabled: ro)
                    CSLabeled(label: "Director", text: model.bind(\.header.director), disabled: ro)
                    CSLabeled(label: "1st AD", text: model.bind(\.header.firstAd), disabled: ro)
                    CSLabeled(label: "Production contact", text: model.bind(\.header.contact), disabled: ro)
                }
            }
        }
    }

    // MARK: Day and times

    private func times(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Day and times") {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Spacer()
                    Text("CREW CALL:")
                    CSEdit(
                        label: "Crew call",
                        text: Binding(
                            get: { model.doc?.crewCall ?? "" },
                            set: { v in
                                model.crewInvalid = false
                                model.edit { $0.crewCall = v }
                            }),
                        need: true, disabled: ro, invalid: model.crewInvalid)
                        .focused($crewFocused)
                    Spacer()
                }
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 16), count: 3), alignment: .leading, spacing: 0) {
                    CSLabeled(label: "General call", text: model.bind(\.times.generalCall), disabled: ro)
                    CSLabeled(label: "Shooting call", text: model.bind(\.times.shootingCall), disabled: ro)
                    CSLabeled(label: "Breakfast", text: model.bind(\.times.breakfast), disabled: ro)
                    CSLabeled(label: "Lunch", text: model.bind(\.times.lunch), disabled: ro)
                    CSLabeled(label: "Estimated wrap", text: model.bind(\.times.estimatedWrap), disabled: ro)
                }
                HStack(alignment: .top, spacing: 6) {
                    Text("Notes from the day").font(.system(size: 11.5, weight: .semibold)).frame(minWidth: 128, alignment: .leading)
                    CSArea(label: "Notes from the day", text: model.bind(\.dayNotes), disabled: ro)
                }
            }
        }
    }

    // MARK: Locations

    private func locations(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Locations") {
            VStack(alignment: .leading, spacing: 6) {
                if doc.locations.isEmpty {
                    Text("\(csNeedsInput): no location for these scenes").foregroundStyle(paperNeedInk)
                        .accessibilityIdentifier("callsheet-no-locations")
                }
                ForEach(Array(doc.locations.enumerated()), id: \.offset) { i, l in
                    VStack(alignment: .leading, spacing: 0) {
                        Text(l.name).fontWeight(.bold)
                        LazyVGrid(columns: [GridItem(.flexible(), spacing: 16), GridItem(.flexible())], alignment: .leading, spacing: 0) {
                            locField("Address", i, true, ro, { $0.address }, { $0.address = $1 })
                            locField("Meeting point", i, false, ro, { $0.meetingPoint }, { $0.meetingPoint = $1 })
                            locField("Parking", i, false, ro, { $0.parking }, { $0.parking = $1 })
                            locField("Access", i, false, ro, { $0.access }, { $0.access = $1 })
                            locField("Nearest hospital", i, true, ro, { $0.hospital }, { $0.hospital = $1 })
                            locField("Location contact", i, false, ro, { $0.emergencyContact }, { $0.emergencyContact = $1 })
                            locField("Notes", i, false, ro, { $0.notes }, { $0.notes = $1 })
                        }
                    }
                }
            }
        }
    }

    private func locField(
        _ label: String, _ i: Int, _ need: Bool, _ ro: Bool,
        _ get: @escaping (CallSheetDocLocation) -> String,
        _ set: @escaping (inout CallSheetDocLocation, String) -> Void
    ) -> some View {
        CSLabeled(
            label: label,
            text: model.bind({ $0.locations[safe: i].map(get) ?? "" }, { d, v in if d.locations.indices.contains(i) { set(&d.locations[i], v) } }),
            need: need, disabled: ro)
    }

    // MARK: Scene schedule

    private func scenes(_ doc: CallSheetDocument, _ d: CallSheetDetail, _ ro: Bool) -> some View {
        CSPaperSection(title: "Scene schedule") {
            if doc.scenes.isEmpty {
                Text("No scenes on this day.").foregroundStyle(.gray).accessibilityIdentifier("callsheet-no-scenes")
            } else {
                Grid(alignment: .topLeading, horizontalSpacing: 8, verticalSpacing: 3) {
                    GridRow {
                        ForEach(["Start", "Sc", "Set and description", "I/E", "D/N", "Pages", "Cast", "Location"], id: \.self) {
                            Text($0).font(.system(size: 10.5, weight: .bold))
                        }
                    }
                    Divider().gridCellUnsizedAxes(.horizontal)
                    ForEach(Array(doc.scenes.enumerated()), id: \.offset) { i, s in
                        let estimate = d.estimatedStarts[safe: i] ?? ""
                        if !s.kind.isEmpty {
                            GridRow {
                                startField(i, s, estimate, ro, label: "Start of \(s.heading.isEmpty ? s.kind : s.heading)")
                                HStack(spacing: 4) {
                                    Text(s.kind).fontWeight(.bold)
                                    if !s.heading.isEmpty, s.heading != s.kind { Text("\u{00B7} \(s.heading)") }
                                    if let m = s.minutes { Text("\u{00B7} \(Int(m)) min") }
                                }
                                .gridCellColumns(7)
                                .padding(.vertical, 2)
                                .background(Color.gray.opacity(0.14))
                                .accessibilityIdentifier("callsheet-scene-row-\(i)-\(csSlug(s.kind))")
                            }
                        } else {
                            GridRow {
                                startField(i, s, estimate, ro, label: "Start of scene \(s.number)")
                                Text(s.number)
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(s.heading)
                                    if !s.description.isEmpty { Text(s.description).font(.system(size: 10.5)) }
                                }
                                Text(s.intExt)
                                Text(s.dayNight)
                                Text(s.pages)
                                Text(s.castIds)
                                Text(s.location)
                            }
                            .accessibilityElement(children: .contain)
                            .accessibilityIdentifier("callsheet-scene-row-\(i)")
                        }
                    }
                }
            }
        }
    }

    private func startField(_ i: Int, _ s: CallSheetDocScene, _ estimate: String, _ ro: Bool, label: String) -> some View {
        CSEdit(
            label: label,
            text: model.bind({ $0.scenes[safe: i]?.start ?? "" }, { d, v in if d.scenes.indices.contains(i) { d.scenes[i].start = v } }),
            placeholder: estimate.isEmpty ? nil : "about \(estimate)", disabled: ro, minWidth: 64)
    }

    // MARK: Talent

    private func talent(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Talent") {
            if doc.cast.isEmpty {
                Text("No cast in these scenes.").foregroundStyle(.gray).accessibilityIdentifier("callsheet-no-cast")
            } else {
                Grid(alignment: .topLeading, horizontalSpacing: 8, verticalSpacing: 3) {
                    GridRow {
                        ForEach(["ID", "Actor", "Character", "Status", "Pickup", "Makeup", "Call", "Contact and notes"], id: \.self) {
                            Text($0).font(.system(size: 10.5, weight: .bold))
                        }
                    }
                    Divider().gridCellUnsizedAxes(.horizontal)
                    ForEach(Array(doc.cast.enumerated()), id: \.offset) { i, c in
                        GridRow {
                            Text(c.castId)
                            castField("Actor for \(c.character)", i, need: true, ro, { $0.actor }, { $0.actor = $1 })
                            Text(c.character)
                            castField("Work status of \(c.character)", i, placeholder: "SW, W, WF", ro, { $0.status }, { $0.status = $1 })
                            castField("Pickup for \(c.character)", i, ro, { $0.pickup }, { $0.pickup = $1 })
                            castField("Makeup time for \(c.character)", i, ro, { $0.makeup }, { $0.makeup = $1 })
                            castField("Call time for \(c.character)", i, need: true, ro, { $0.callTime }, { $0.callTime = $1 })
                            HStack(spacing: 4) {
                                castField("Contact for \(c.character)", i, placeholder: "Contact", ro, { $0.contact }, { $0.contact = $1 })
                                castField("Notes for \(c.character)", i, placeholder: "Notes", ro, { $0.notes }, { $0.notes = $1 })
                            }
                        }
                        .accessibilityElement(children: .contain)
                        .accessibilityIdentifier("callsheet-cast-row-\(i)")
                    }
                }
            }
        }
    }

    private func castField(
        _ label: String, _ i: Int, need: Bool = false, placeholder: String? = nil, _ ro: Bool,
        _ get: @escaping (CallSheetDocCast) -> String,
        _ set: @escaping (inout CallSheetDocCast, String) -> Void
    ) -> some View {
        CSEdit(
            label: label,
            text: model.bind({ $0.cast[safe: i].map(get) ?? "" }, { d, v in if d.cast.indices.contains(i) { set(&d.cast[i], v) } }),
            need: need, placeholder: placeholder, disabled: ro, minWidth: 56)
    }

    // MARK: Practical notes

    private func practical(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Practical notes") {
            VStack(alignment: .leading, spacing: 0) {
                CSLabeled(label: "Parking", text: model.bind(\.practical.parking), disabled: ro)
                CSLabeled(label: "Meeting point", text: model.bind(\.practical.meetingPoint), disabled: ro)
                CSLabeled(label: "Transport", text: model.bind(\.practical.travelNotes), disabled: ro)
                CSLabeled(label: "Meals", text: model.bind(\.practical.mealBreak), disabled: ro)
                CSLabeled(label: "Toilets", text: model.bind(\.practical.toilets), disabled: ro)
                CSLabeled(label: "Access", text: model.bind(\.practical.access), disabled: ro)
                CSLabeled(label: "Safety", text: model.bind(\.practical.safety), disabled: ro)
                CSLabeled(label: "Emergency contact", text: model.bind(\.practical.emergencyContact), need: true, disabled: ro)
                HStack(alignment: .top, spacing: 6) {
                    Text("Special instructions").font(.system(size: 11.5, weight: .semibold)).frame(minWidth: 128, alignment: .leading)
                    CSArea(label: "Special instructions", text: model.bind(\.practical.specialInstructions), disabled: ro)
                }.padding(.vertical, 1.5)
                HStack(alignment: .top, spacing: 6) {
                    Text("Production notes").font(.system(size: 11.5, weight: .semibold)).frame(minWidth: 128, alignment: .leading)
                    CSArea(label: "Production notes", text: model.bind(\.practical.productionNotes), disabled: ro)
                }.padding(.vertical, 1.5)
            }
        }
    }

    // MARK: Added sections

    @ViewBuilder private func optionalSections(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        let o = doc.optional
        if o.weather != nil { weather(ro) }
        if let extras = o.extras { extrasSection(extras, ro) }
        if let deps = o.departments { departmentsSection(deps, ro) }
        if o.advance != nil { advanceSection(ro) }
        if let crew = o.crew { crewSection(crew, ro) }
        if o.communication != nil { communicationSection(ro) }
        if o.specialNotes != nil {
            CSPaperSection(title: "Special notes", onRemove: { model.edit { $0.optional.specialNotes = nil } }, disabled: ro) {
                CSArea(label: "Special notes", text: model.bind({ $0.optional.specialNotes ?? "" }, { $0.optional.specialNotes = $1 }), disabled: ro)
            }
        }
        if let a = o.attachments { attachmentsSection(a, ro) }
        if let r = o.referenceImages { referenceSection(r, ro) }
        if !doc.extraFields.isEmpty { extraFieldsSection(doc.extraFields, ro) }
    }

    private func weather(_ ro: Bool) -> some View {
        CSPaperSection(title: "Weather", onRemove: { model.edit { $0.optional.weather = nil } }, disabled: ro) {
            VStack(alignment: .leading, spacing: 0) {
                Text("Type the forecast, sunrise and sunset. OpenFrame never looks them up online.")
                    .font(.system(size: 10.5)).foregroundStyle(.gray)
                CSLabeled(label: "Forecast", text: model.bind({ $0.optional.weather ?? "" }, { $0.optional.weather = $1 }), need: true, disabled: ro)
                HStack(spacing: 16) {
                    CSLabeled(label: "Sunrise", text: model.bind(\.optional.sunrise), need: true, disabled: ro)
                    CSLabeled(label: "Sunset", text: model.bind(\.optional.sunset), need: true, disabled: ro)
                }
            }
        }
    }

    private func extrasSection(_ rows: [CallSheetExtra], _ ro: Bool) -> some View {
        CSPaperSection(title: "Extras", onRemove: { model.edit { $0.optional.extras = nil } }, disabled: ro) {
            VStack(alignment: .leading, spacing: 4) {
                Grid(alignment: .topLeading, horizontalSpacing: 8, verticalSpacing: 3) {
                    GridRow {
                        ForEach(["Description", "Quantity", "Call", "On set", "Notes"], id: \.self) { Text($0).font(.system(size: 10.5, weight: .bold)) }
                        if !ro { Text("") }
                    }
                    ForEach(Array(rows.enumerated()), id: \.offset) { i, e in
                        GridRow {
                            extraCell(i, "description", need: false, ro, { $0.description }, { $0.description = $1 })
                            extraCell(i, "quantity", need: false, ro, { $0.quantity }, { $0.quantity = $1 })
                            extraCell(i, "callTime", need: true, ro, { $0.callTime }, { $0.callTime = $1 })
                            extraCell(i, "onSet", need: false, ro, { $0.onSet }, { $0.onSet = $1 })
                            extraCell(i, "notes", need: false, ro, { $0.notes }, { $0.notes = $1 })
                            if !ro {
                                CSRemoveRow(label: "Remove extras row \(e.description.isEmpty ? String(i + 1) : e.description)") {
                                    model.edit { if $0.optional.extras?.indices.contains(i) == true { $0.optional.extras?.remove(at: i) } }
                                }
                            }
                        }
                    }
                }
                if !ro {
                    Button { model.edit { $0.optional.extras?.append(CallSheetExtra(key: "", description: "", quantity: "", callTime: "", onSet: "", notes: "")) } } label: {
                        Label("Add extras row", systemImage: "plus")
                    }
                    .controlSize(.small)
                    .accessibilityIdentifier("callsheet-add-extras-row")
                }
            }
        }
    }

    private func extraCell(
        _ i: Int, _ name: String, need: Bool, _ ro: Bool,
        _ get: @escaping (CallSheetExtra) -> String, _ set: @escaping (inout CallSheetExtra, String) -> Void
    ) -> some View {
        CSEdit(
            label: "Extras \(i + 1) \(name)",
            text: model.bind({ $0.optional.extras?[safe: i].map(get) ?? "" }, { d, v in
                if d.optional.extras?.indices.contains(i) == true { set(&d.optional.extras![i], v) }
            }),
            need: need, disabled: ro, minWidth: 56)
    }

    private func departmentsSection(_ rows: [CallSheetDepartmentNote], _ ro: Bool) -> some View {
        CSPaperSection(title: "Department notes", onRemove: { model.edit { $0.optional.departments = nil } }, disabled: ro) {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(Array(rows.enumerated()), id: \.offset) { i, dep in
                    HStack(alignment: .top, spacing: 6) {
                        CSEdit(
                            label: "Department \(i + 1)",
                            text: model.bind({ $0.optional.departments?[safe: i]?.department ?? "" }, { d, v in
                                if d.optional.departments?.indices.contains(i) == true { d.optional.departments![i].department = v }
                            }),
                            placeholder: "Department", disabled: ro, minWidth: 110)
                        CSArea(
                            label: "Notes for \(dep.department.isEmpty ? "department \(i + 1)" : dep.department)",
                            text: model.bind({ $0.optional.departments?[safe: i]?.notes ?? "" }, { d, v in
                                if d.optional.departments?.indices.contains(i) == true { d.optional.departments![i].notes = v }
                            }), disabled: ro)
                        if !ro {
                            CSRemoveRow(label: "Remove \(dep.department.isEmpty ? "department \(i + 1)" : dep.department)") {
                                model.edit { if $0.optional.departments?.indices.contains(i) == true { $0.optional.departments?.remove(at: i) } }
                            }
                        }
                    }
                }
                if !ro {
                    Button { model.edit { $0.optional.departments?.append(CallSheetDepartmentNote(department: "", notes: "")) } } label: {
                        Label("Add department", systemImage: "plus")
                    }
                    .controlSize(.small)
                    .accessibilityIdentifier("callsheet-add-department")
                }
            }
        }
    }

    private func advanceSection(_ ro: Bool) -> some View {
        CSPaperSection(title: "Advance schedule", onRemove: { model.edit { $0.optional.advance = nil } }, disabled: ro) {
            VStack(alignment: .leading, spacing: 0) {
                advanceField("Next day", need: true, ro, { $0.day }, { $0.day = $1 })
                advanceField("Location", ro, { $0.location }, { $0.location = $1 })
                advanceField("Cast", ro, { $0.cast }, { $0.cast = $1 })
                advanceField("Props", ro, { $0.props }, { $0.props = $1 })
                advanceField("Wardrobe", ro, { $0.wardrobe }, { $0.wardrobe = $1 })
                advanceField("Preparation", ro, { $0.preparation }, { $0.preparation = $1 })
            }
        }
    }

    private func advanceField(
        _ label: String, need: Bool = false, _ ro: Bool,
        _ get: @escaping (CallSheetAdvance) -> String, _ set: @escaping (inout CallSheetAdvance, String) -> Void
    ) -> some View {
        CSLabeled(
            label: label,
            text: model.bind({ $0.optional.advance.map(get) ?? "" }, { d, v in if d.optional.advance != nil { set(&d.optional.advance!, v) } }),
            need: need, disabled: ro)
    }

    private func crewSection(_ rows: [CallSheetCrewMember], _ ro: Bool) -> some View {
        CSPaperSection(title: "Crew", onRemove: { model.edit { $0.optional.crew = nil } }, disabled: ro) {
            VStack(alignment: .leading, spacing: 4) {
                Grid(alignment: .topLeading, horizontalSpacing: 8, verticalSpacing: 3) {
                    GridRow {
                        ForEach(["Department", "Role", "Name", "Call", "Contact"], id: \.self) { Text($0).font(.system(size: 10.5, weight: .bold)) }
                        if !ro { Text("") }
                    }
                    ForEach(Array(rows.enumerated()), id: \.offset) { i, m in
                        GridRow {
                            crewCell(i, m, "department", ro, { $0.department }, { $0.department = $1 })
                            crewCell(i, m, "role", ro, { $0.role }, { $0.role = $1 })
                            crewCell(i, m, "name", ro, { $0.name }, { $0.name = $1 })
                            crewCell(i, m, "callTime", ro, { $0.callTime }, { $0.callTime = $1 })
                            crewCell(i, m, "contact", ro, { $0.contact }, { $0.contact = $1 })
                            if !ro {
                                CSRemoveRow(label: "Remove \(m.name.isEmpty ? "crew row \(i + 1)" : m.name)") {
                                    model.edit { if $0.optional.crew?.indices.contains(i) == true { $0.optional.crew?.remove(at: i) } }
                                }
                            }
                        }
                    }
                }
                if !ro {
                    Button { model.edit { $0.optional.crew?.append(CallSheetCrewMember(key: "", name: "", role: "", department: "", callTime: "", contact: "")) } } label: {
                        Label("Add crew row", systemImage: "plus")
                    }
                    .controlSize(.small)
                    .accessibilityIdentifier("callsheet-add-crew-row")
                }
            }
        }
    }

    private func crewCell(
        _ i: Int, _ m: CallSheetCrewMember, _ name: String, _ ro: Bool,
        _ get: @escaping (CallSheetCrewMember) -> String, _ set: @escaping (inout CallSheetCrewMember, String) -> Void
    ) -> some View {
        CSEdit(
            label: "Crew \(m.name.isEmpty ? String(i + 1) : m.name) \(name)",
            text: model.bind({ $0.optional.crew?[safe: i].map(get) ?? "" }, { d, v in
                if d.optional.crew?.indices.contains(i) == true { set(&d.optional.crew![i], v) }
            }),
            disabled: ro, minWidth: 56)
    }

    private func communicationSection(_ ro: Bool) -> some View {
        CSPaperSection(title: "Walkie channels and contacts", onRemove: { model.edit { $0.optional.communication = nil } }, disabled: ro) {
            VStack(alignment: .leading, spacing: 0) {
                commField("Walkie channels", ro, { $0.walkieChannels }, { $0.walkieChannels = $1 })
                commField("Production contact", ro, { $0.productionContact }, { $0.productionContact = $1 })
                commField("Emergency contact", ro, { $0.emergencyContact }, { $0.emergencyContact = $1 })
                commField("Note", ro, { $0.note }, { $0.note = $1 })
            }
        }
    }

    private func commField(
        _ label: String, _ ro: Bool,
        _ get: @escaping (CallSheetCommunication) -> String, _ set: @escaping (inout CallSheetCommunication, String) -> Void
    ) -> some View {
        CSLabeled(
            label: label,
            text: model.bind({ $0.optional.communication.map(get) ?? "" }, { d, v in if d.optional.communication != nil { set(&d.optional.communication!, v) } }),
            disabled: ro)
    }

    private func attachmentsSection(_ items: [CallSheetAttachment], _ ro: Bool) -> some View {
        CSPaperSection(title: "Attachments", onRemove: { model.edit { $0.optional.attachments = nil } }, disabled: ro) {
            HStack(spacing: 8) {
                ForEach(Array(items.enumerated()), id: \.offset) { i, a in
                    HStack(spacing: 4) {
                        Image(systemName: "paperclip")
                        Text(a.name)
                        if !ro {
                            CSRemoveRow(label: "Remove \(a.name)") {
                                model.edit { if $0.optional.attachments?.indices.contains(i) == true { $0.optional.attachments?.remove(at: i) } }
                            }
                        }
                    }
                    .padding(.horizontal, 6).padding(.vertical, 2)
                    .background(Color.gray.opacity(0.15), in: Capsule())
                }
                if items.isEmpty { Text("No attachments yet.").foregroundStyle(.gray) }
                if !ro {
                    Button { chooseFile("attachments", images: false) } label: { Label("Add file\u{2026}", systemImage: "plus") }
                        .controlSize(.small)
                        .accessibilityIdentifier("callsheet-add-attachment")
                }
            }
        }
    }

    private func referenceSection(_ items: [CallSheetAttachment], _ ro: Bool) -> some View {
        CSPaperSection(title: "Reference images", onRemove: { model.edit { $0.optional.referenceImages = nil } }, disabled: ro) {
            HStack(alignment: .top, spacing: 8) {
                ForEach(Array(items.enumerated()), id: \.offset) { i, a in
                    VStack(spacing: 2) {
                        CSAssetThumb(model: model, attachment: a)
                        if !ro {
                            CSRemoveRow(label: "Remove \(a.name)") {
                                model.edit { if $0.optional.referenceImages?.indices.contains(i) == true { $0.optional.referenceImages?.remove(at: i) } }
                            }
                        }
                    }
                }
                if items.isEmpty { Text("No images yet.").foregroundStyle(.gray) }
                if !ro {
                    Button { chooseFile("referenceImages", images: true) } label: { Label("Add image\u{2026}", systemImage: "plus") }
                        .controlSize(.small)
                        .accessibilityIdentifier("callsheet-add-reference-image")
                }
            }
        }
    }

    private func extraFieldsSection(_ fields: [CallSheetExtraField], _ ro: Bool) -> some View {
        CSPaperSection(title: "Additional information") {
            VStack(alignment: .leading, spacing: 3) {
                ForEach(Array(fields.enumerated()), id: \.offset) { i, f in
                    HStack(spacing: 6) {
                        CSEdit(
                            label: "Field \(i + 1) name",
                            text: model.bind({ $0.extraFields[safe: i]?.label ?? "" }, { d, v in if d.extraFields.indices.contains(i) { d.extraFields[i].label = v } }),
                            placeholder: "Field name", disabled: ro, minWidth: 110)
                        CSEdit(
                            label: "Field \(i + 1) value",
                            text: model.bind({ $0.extraFields[safe: i]?.value ?? "" }, { d, v in if d.extraFields.indices.contains(i) { d.extraFields[i].value = v } }),
                            disabled: ro, wide: true)
                        if !ro {
                            CSRemoveRow(label: "Remove field \(f.label.isEmpty ? String(i + 1) : f.label)") {
                                model.edit { if $0.extraFields.indices.contains(i) { $0.extraFields.remove(at: i) } }
                            }
                        }
                    }
                }
            }
        }
    }

    private func chooseFile(_ section: String, images: Bool) {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        if images { panel.allowedContentTypes = [.png, .jpeg, .gif, .webP, .bmp] }
        panel.prompt = "Attach"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task { await model.attach(section: section, path: url.path) }
    }
}

/// A small picture of an attached reference image.
struct CSAssetThumb: View {
    let model: CallSheetEditorModel
    let attachment: CallSheetAttachment
    @State private var image: NSImage?

    var body: some View {
        Group {
            if let image {
                Image(nsImage: image).resizable().scaledToFill()
                    .frame(width: 96, height: 72).clipped()
                    .overlay(Rectangle().stroke(Color.gray.opacity(0.5)))
                    .accessibilityLabel(attachment.name)
            } else {
                Text(attachment.name).padding(.horizontal, 6).padding(.vertical, 2)
                    .background(Color.gray.opacity(0.15), in: Capsule())
            }
        }
        .task(id: attachment.assetId) {
            guard let core = model.hub.core else { return }
            image = await VaultThumbnails.shared.image(core: core, assetId: attachment.assetId, global: false, maxPixel: 200)
        }
    }
}

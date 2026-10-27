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
private let paperHead = Color(red: 0.118, green: 0.137, blue: 0.188)
private let paperMuted = Color(red: 0.357, green: 0.392, blue: 0.459)
private let paperCardLine = Color(red: 0.839, green: 0.859, blue: 0.894)
private let paperCardHead = Color(red: 0.910, green: 0.929, blue: 0.957)
private let paperTile = Color(red: 0.165, green: 0.416, blue: 0.541)
private let paperDayBox = Color(red: 0.118, green: 0.165, blue: 0.267)
private let paperDayInk = Color(red: 0.725, green: 0.757, blue: 0.820)
private let paperCallout = Color(red: 0.992, green: 0.941, blue: 0.882)
private let paperCalloutLine = Color(red: 0.945, green: 0.773, blue: 0.569)
private let paperCrewInk = Color(red: 0.659, green: 0.333, blue: 0.055)

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

    /// A value inside one part of the document, such as `\.header` and then `\.company`.
    func bind<Part>(_ part: WritableKeyPath<CallSheetDocument, Part>, _ kp: WritableKeyPath<Part, String>) -> Binding<String> {
        bind({ $0[keyPath: part][keyPath: kp] }, { $0[keyPath: part][keyPath: kp] = $1 })
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

/// A section with the PDF's heading: its name on the left, an optional note on the right, a rule under it.
struct CSPaperSection<Content: View>: View {
    let title: String
    var note: String?
    var onRemove: (() -> Void)?
    var disabled = false
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            VStack(spacing: 3) {
                HStack(spacing: 8) {
                    Text(title.uppercased()).font(.system(size: 12.5, weight: .bold)).tracking(1.4)
                        .accessibilityAddTraits(.isHeader)
                    Spacer(minLength: 4)
                    if let note {
                        Text(note).font(.system(size: 9.5, weight: .semibold)).foregroundStyle(paperMuted)
                            .accessibilityIdentifier("callsheet-note-\(csSlug(title))")
                    }
                    CSRemoveSection(title: title, onRemove: onRemove, disabled: disabled)
                }
                Rectangle().fill(paperHead).frame(height: 2)
            }
            .foregroundStyle(paperHead)
            .padding(.top, 10)
            content()
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
        .accessibilityIdentifier("callsheet-section-\(csSlug(title))")
    }
}

/// The small cross that takes an added section off the sheet.
struct CSRemoveSection: View {
    let title: String
    var onRemove: (() -> Void)?
    var disabled = false

    var body: some View {
        if let onRemove, !disabled {
            Button(action: onRemove) { Image(systemName: "xmark").font(.system(size: 10, weight: .bold)) }
                .buttonStyle(.plain)
                .help("Remove \(title)")
                .accessibilityLabel("Remove \(title)")
                .accessibilityIdentifier("callsheet-remove-section-\(csSlug(title))")
        }
    }
}

/// A bordered card on the paper, as the PDF draws its field groups.
struct CSCard<Content: View>: View {
    @ViewBuilder var content: () -> Content
    var body: some View {
        VStack(alignment: .leading, spacing: 0) { content() }
            .padding(.horizontal, 12).padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(RoundedRectangle(cornerRadius: 7).strokeBorder(paperCardLine))
    }
}

/// A small tracked label in capitals, the PDF's eyebrow.
struct CSEyebrow: View {
    let text: String
    var color: Color = paperMuted
    var body: some View {
        Text(text.uppercased()).font(.system(size: 9, weight: .bold)).tracking(1.6).foregroundStyle(color)
    }
}

/// Lays its children in equal columns that fit the width, each at least `minWidth` wide, and wraps
/// the rest onto new rows. The call sheet uses it so the paper wraps on a narrow window instead of
/// scrolling sideways (Windows 5bc07a01: `repeat(auto-fit, minmax(260px, 1fr))`).
struct CSWrap: Layout {
    var minWidth: CGFloat
    var spacing: CGFloat = 10
    var rowSpacing: CGFloat = 8

    /// How many columns of at least `minWidth` fit in `width` (at least one, at most `count`).
    static func columns(width: CGFloat, count: Int, minWidth: CGFloat, spacing: CGFloat) -> Int {
        guard count > 0 else { return 1 }
        guard width.isFinite, width > 0 else { return count }
        let fit = Int(((width + spacing) / (minWidth + spacing)).rounded(.down))
        return max(1, min(count, fit))
    }

    private func width(_ proposal: ProposedViewSize, _ count: Int) -> CGFloat {
        if let w = proposal.width, w.isFinite { return w }
        return CGFloat(count) * minWidth + CGFloat(max(0, count - 1)) * spacing
    }

    private func rows(_ width: CGFloat, _ subviews: Subviews) -> (cols: Int, colWidth: CGFloat, heights: [CGFloat]) {
        let cols = Self.columns(width: width, count: subviews.count, minWidth: minWidth, spacing: spacing)
        let colWidth = max(0, (width - CGFloat(cols - 1) * spacing) / CGFloat(cols))
        var heights: [CGFloat] = []
        for (i, v) in subviews.enumerated() {
            let h = v.sizeThatFits(ProposedViewSize(width: colWidth, height: nil)).height
            if i % cols == 0 { heights.append(h) } else { heights[heights.count - 1] = max(heights[heights.count - 1], h) }
        }
        return (cols, colWidth, heights)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let w = width(proposal, subviews.count)
        let r = rows(w, subviews)
        let h = r.heights.reduce(0, +) + CGFloat(max(0, r.heights.count - 1)) * rowSpacing
        return CGSize(width: w, height: h)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let r = rows(bounds.width, subviews)
        var y = bounds.minY
        for (i, v) in subviews.enumerated() {
            let row = i / r.cols, col = i % r.cols
            if col == 0, row > 0 { y += r.heights[row - 1] + rowSpacing }
            let x = bounds.minX + CGFloat(col) * (r.colWidth + spacing)
            v.place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: ProposedViewSize(width: r.colWidth, height: r.heights[row]))
        }
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

// MARK: Layout

/// One block of the paper. The paper draws the blocks in the order `CSLayout.blocks` gives, which
/// is the exported PDF heading by heading (core exports/callsheet.rs; Windows CallSheetEditor.tsx):
/// masthead, the time callout, Production, Locations, Scene schedule, then the "People on the day"
/// group (Talent / Cast, Extras, Crew, Department notes) and the "Day-of logistics" group
/// (Practical / Safety information, Weather, Advance schedule, Walkie channels and contacts,
/// Special notes, Attachments & reference images).
enum CSBlock: Hashable {
    case masthead, times, production, locations, scenes
    case peopleGroup, talent, extras, crew, departments
    case logisticsGroup, practical, weather, advance, communication, specialNotes, attachments

    /// The heading the block shows; the masthead and the time callout carry none, as in the PDF.
    var heading: String? {
        switch self {
        case .masthead, .times: return nil
        case .production: return "Production"
        case .locations: return "Locations"
        case .scenes: return "Scene schedule"
        case .peopleGroup: return "Cast & Crew"
        case .talent: return "Talent / Cast"
        case .extras: return "Extras"
        case .crew: return "Crew"
        case .departments: return "Department notes"
        case .logisticsGroup: return "Production notes"
        case .practical: return "Practical / Safety information"
        case .weather: return "Weather"
        case .advance: return "Advance schedule"
        case .communication: return "Walkie channels and contacts"
        case .specialNotes: return "Special notes"
        case .attachments: return "Attachments & reference images"
        }
    }
}

/// The paper's order, field labels and notes, kept apart from the drawing so tests can read them.
enum CSLayout {
    static func blocks(_ doc: CallSheetDocument) -> [CSBlock] {
        let o = doc.optional
        var out: [CSBlock] = [.masthead, .times, .production, .locations, .scenes, .peopleGroup, .talent]
        if o.extras != nil { out.append(.extras) }
        if o.crew != nil { out.append(.crew) }
        if o.departments != nil { out.append(.departments) }
        out += [.logisticsGroup, .practical]
        if o.weather != nil { out.append(.weather) }
        if o.advance != nil { out.append(.advance) }
        if o.communication != nil { out.append(.communication) }
        if o.specialNotes != nil { out.append(.specialNotes) }
        if o.attachments != nil || o.referenceImages != nil { out.append(.attachments) }
        return out
    }

    static func plural(_ n: Int, _ one: String, _ many: String) -> String { "\(n) \(n == 1 ? one : many)" }

    static func sceneCount(_ doc: CallSheetDocument) -> Int { doc.scenes.filter { $0.kind.isEmpty }.count }

    /// The masthead's counts line: "3 scenes · 2 cast members · 1 location".
    static func counts(_ doc: CallSheetDocument) -> String {
        [plural(sceneCount(doc), "scene", "scenes"), plural(doc.cast.count, "cast member", "cast members"),
         plural(doc.locations.count, "location", "locations")].joined(separator: " \u{00B7} ")
    }

    /// As the PDF: "Shoot day" over the day's number when the label ends in one, else the label itself.
    static func shootDay(_ dayLabel: String) -> (eyebrow: String, number: String?) {
        let label = dayLabel.trimmingCharacters(in: .whitespaces)
        let digits = String(label.reversed().prefix { $0.isASCII && $0.isNumber }.reversed())
        if digits.isEmpty { return (label.isEmpty ? "Shoot day" : label, nil) }
        return ("Shoot day", digits.count < 2 ? "0" + digits : digits)
    }

    /// The note on the right of a section heading, where the PDF prints one.
    static func note(_ block: CSBlock, _ doc: CallSheetDocument, readOnly: Bool) -> String? {
        switch block {
        case .locations: return plural(doc.locations.count, "location", "locations")
        case .scenes: return plural(sceneCount(doc), "scheduled scene", "scheduled scenes")
        case .talent: return plural(doc.cast.count, "cast member", "cast members")
        case .crew: return plural(doc.optional.crew?.count ?? 0, "crew member listed", "crew members listed")
        case .practical: return readOnly ? "Quick reference" : "Review before issuing"
        default: return nil
        }
    }

    static var productionColumns: [[(String, WritableKeyPath<CallSheetHeader, String>)]] {
        [[("Production company", \.company), ("Production address", \.address), ("Producer", \.producer)],
         [("Director", \.director), ("1st AD", \.firstAd), ("Production contact", \.contact)]]
    }

    /// The time callout after the crew call: (shown label, field name for assistive tech, value).
    static var times: [(String, String, WritableKeyPath<CallSheetTimes, String>)] {
        [("General call", "General call", \.generalCall), ("Shooting call", "Shooting call", \.shootingCall),
         ("Breakfast", "Breakfast", \.breakfast), ("Lunch", "Lunch", \.lunch), ("Est. wrap", "Estimated wrap", \.estimatedWrap)]
    }

    static var locationFields: [(String, Bool, WritableKeyPath<CallSheetDocLocation, String>)] {
        [("Address", true, \.address), ("Meeting point", false, \.meetingPoint), ("Access", false, \.access),
         ("Parking", false, \.parking), ("Nearest hospital", true, \.hospital), ("Emergency contact", true, \.emergencyContact),
         ("Notes", false, \.notes)]
    }

    static var practicalFields: [(String, WritableKeyPath<CallSheetPractical, String>)] {
        [("Parking", \.parking), ("Meeting point", \.meetingPoint), ("Transport", \.travelNotes), ("Meals", \.mealBreak),
         ("Toilets", \.toilets), ("Access", \.access), ("Safety", \.safety)]
    }

    /// The practical section's labels in order: the emergency contact and hospital cards, the
    /// practical notes, then the day notes (custom fields follow, named by the user).
    static func practicalLabels(_ doc: CallSheetDocument) -> [String] {
        ["Production emergency contact"] + doc.locations.map { "\($0.name) \u{00B7} Nearest hospital" }
            + practicalFields.map(\.0) + ["Special instructions", "Production notes", "Day notes"]
    }

    static var advanceFields: [(String, Bool, WritableKeyPath<CallSheetAdvance, String>)] {
        [("Day", true, \.day), ("Location", false, \.location), ("Cast", false, \.cast), ("Props", false, \.props),
         ("Wardrobe", false, \.wardrobe), ("Preparation", false, \.preparation)]
    }

    static var communicationFields: [(String, WritableKeyPath<CallSheetCommunication, String>)] {
        [("Walkie channels", \.walkieChannels), ("Emergency contact", \.emergencyContact),
         ("Production contact", \.productionContact), ("Note", \.note)]
    }

    static let sceneColumns = ["Start", "Sc", "Heading / description", "I/E", "D/N", "Pages", "Cast", "Location"]
    static let talentColumns = ["ID", "Actor", "Character", "Status", "Pickup", "Makeup", "Call", "Contact and notes"]
}

// MARK: The paper

struct CallSheetPaper: View {
    @Bindable var model: CallSheetEditorModel
    @FocusState private var crewFocused: Bool

    var body: some View {
        if let doc = model.doc, let d = model.detail {
            let ro = !model.editable
            VStack(alignment: .leading, spacing: 8) {
                ForEach(CSLayout.blocks(doc), id: \.self) { block in
                    self.block(block, doc, d, ro)
                }
            }
            .padding(.horizontal, 26).padding(.vertical, 20)
            .frame(maxWidth: 860, alignment: .leading)
            .background(Color.white)
            .foregroundStyle(paperInk)
            .environment(\.colorScheme, .light)
            .font(.system(size: 12))
            .shadow(color: .black.opacity(0.12), radius: 4, y: 1)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Call sheet document")
            .accessibilityIdentifier("callsheet-document")
            .onChange(of: model.focusCrewToken) { _, _ in crewFocused = true }
        }
    }

    @ViewBuilder private func block(_ b: CSBlock, _ doc: CallSheetDocument, _ d: CallSheetDetail, _ ro: Bool) -> some View {
        let o = doc.optional
        switch b {
        case .masthead: masthead(doc, d, ro)
        case .times: times(doc, ro)
        case .production: production(ro)
        case .locations: locations(doc, ro)
        case .scenes: scenes(doc, d, ro)
        case .peopleGroup: groupTitle(eyebrow: "People on the day", b)
        case .talent: talent(doc, ro)
        case .extras: extrasSection(o.extras ?? [], ro)
        case .crew: crewSection(doc, o.crew ?? [], ro)
        case .departments: departmentsSection(o.departments ?? [], ro)
        case .logisticsGroup: groupTitle(eyebrow: "Day-of logistics", b)
        case .practical: practical(doc, ro)
        case .weather: weather(ro)
        case .advance: advanceSection(ro)
        case .communication: communicationSection(ro)
        case .specialNotes:
            CSPaperSection(title: "Special notes", onRemove: { model.edit { $0.optional.specialNotes = nil } }, disabled: ro) {
                CSArea(label: "Special notes", text: model.bind({ $0.optional.specialNotes ?? "" }, { $0.optional.specialNotes = $1 }), disabled: ro)
            }
        case .attachments: attachmentsAndImages(o.attachments, o.referenceImages, ro)
        }
    }

    /// The title of a group of sections, as in the PDF: a small tracked line above the group's name.
    private func groupTitle(eyebrow: String, _ b: CSBlock) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(eyebrow.uppercased()).font(.system(size: 9)).tracking(1.1).foregroundStyle(paperMuted)
            Text(b.heading ?? "").font(.system(size: 15, weight: .bold)).foregroundStyle(paperHead)
        }
        .padding(.top, 14)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
        .accessibilityIdentifier("callsheet-group-\(csSlug(b.heading ?? ""))")
    }

    // MARK: Masthead

    private func masthead(_ doc: CallSheetDocument, _ d: CallSheetDetail, _ ro: Bool) -> some View {
        let main = VStack(alignment: .leading, spacing: 4) {
            CSEyebrow(text: "Call sheet \u{00B7} Revision \(Int(d.revision))")
            CSEdit(label: "Production title", text: model.bind(\.title), need: true, disabled: ro, wide: true, minWidth: 200)
                .font(.system(size: 26, weight: .heavy))
            Text(CSLayout.counts(doc)).font(.system(size: 11)).foregroundStyle(paperMuted)
                .accessibilityIdentifier("callsheet-counts")
        }
        let day = CSLayout.shootDay(doc.dayLabel)
        let dayBox = VStack(alignment: .leading, spacing: 2) {
            CSEyebrow(text: day.eyebrow, color: paperDayInk)
            if let n = day.number { Text(n).font(.system(size: 26, weight: .heavy)) }
            if let long = d.dateLong {
                Text(long).fontWeight(.bold)
            } else {
                Text("\(csNeedsInput): set the date in the schedule").fontWeight(.bold).foregroundStyle(paperNeedFill)
            }
            if !doc.dayOf.isEmpty { Text(doc.dayOf).font(.system(size: 10.5)).foregroundStyle(paperDayInk).padding(.top, 4) }
        }
        .foregroundStyle(Color.white)
        .padding(.horizontal, 16).padding(.vertical, 12)
        .frame(minWidth: 150, alignment: .leading)
        .background(paperDayBox, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("callsheet-day-line")
        return ViewThatFits(in: .horizontal) {
            HStack(alignment: .top, spacing: 18) {
                main.frame(minWidth: 240, maxWidth: .infinity, alignment: .leading)
                dayBox
            }
            VStack(alignment: .leading, spacing: 12) {
                main.frame(maxWidth: .infinity, alignment: .leading)
                dayBox
            }
        }
        .padding(.bottom, 6)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Masthead")
        .accessibilityIdentifier("callsheet-section-masthead")
    }

    // MARK: Time callout

    private func times(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSWrap(minWidth: 90, spacing: 0, rowSpacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                CSEyebrow(text: "Crew call", color: paperCrewInk)
                CSEdit(
                    label: "Crew call",
                    text: Binding(
                        get: { model.doc?.crewCall ?? "" },
                        set: { v in
                            model.crewInvalid = false
                            model.edit { $0.crewCall = v }
                        }),
                    need: true, disabled: ro, wide: true, invalid: model.crewInvalid)
                    .font(.system(size: 22, weight: .heavy))
                    .focused($crewFocused)
            }
            .padding(.trailing, 10)
            ForEach(CSLayout.times, id: \.1) { shown, name, kp in
                VStack(alignment: .leading, spacing: 2) {
                    CSEyebrow(text: shown)
                    CSEdit(label: name, text: model.bind(\.times, kp), disabled: ro, wide: true, minWidth: 50)
                        .font(.system(size: 14, weight: .bold))
                }
                .padding(.horizontal, 10)
                .overlay(alignment: .leading) { Rectangle().fill(paperCalloutLine).frame(width: 1) }
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 12)
        .background(paperCallout, in: RoundedRectangle(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(paperCalloutLine))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Day and times")
        .accessibilityIdentifier("callsheet-section-day-and-times")
    }

    // MARK: Production

    private func production(_ ro: Bool) -> some View {
        CSPaperSection(title: "Production") {
            CSWrap(minWidth: 260, rowSpacing: 8) {
                ForEach(Array(CSLayout.productionColumns.enumerated()), id: \.offset) { _, fields in
                    CSCard {
                        ForEach(fields, id: \.0) { label, kp in
                            CSLabeled(label: label, text: model.bind(\.header, kp), disabled: ro)
                        }
                    }
                }
            }
        }
    }

    // MARK: Locations

    private func locations(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Locations", note: CSLayout.note(.locations, doc, readOnly: ro)) {
            VStack(alignment: .leading, spacing: 8) {
                if doc.locations.isEmpty {
                    Text("\(csNeedsInput): no location for these scenes").foregroundStyle(paperNeedInk)
                        .accessibilityIdentifier("callsheet-no-locations")
                }
                ForEach(Array(doc.locations.enumerated()), id: \.offset) { i, l in
                    CSCard {
                        HStack(spacing: 10) {
                            Text(String(format: "%02d", i + 1)).foregroundStyle(Color.white)
                                .padding(.horizontal, 8).padding(.vertical, 4)
                                .background(paperTile, in: RoundedRectangle(cornerRadius: 5))
                            Text(l.name.uppercased()).fontWeight(.bold).foregroundStyle(paperHead)
                            Spacer(minLength: 0)
                        }
                        .padding(.horizontal, 12).padding(.vertical, 10)
                        .background(paperCardHead)
                        .padding(.horizontal, -12).padding(.top, -6).padding(.bottom, 6)
                        ForEach(CSLayout.locationFields, id: \.0) { label, need, kp in
                            locField(label, i, need, ro, kp)
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 7))
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("callsheet-location-\(i)")
                }
            }
        }
    }

    private func locField(_ label: String, _ i: Int, _ need: Bool, _ ro: Bool, _ kp: WritableKeyPath<CallSheetDocLocation, String>) -> some View {
        CSLabeled(
            label: label,
            text: model.bind({ $0.locations[safe: i]?[keyPath: kp] ?? "" }, { d, v in if d.locations.indices.contains(i) { d.locations[i][keyPath: kp] = v } }),
            need: need, disabled: ro)
    }

    // MARK: Scene schedule

    private func scenes(_ doc: CallSheetDocument, _ d: CallSheetDetail, _ ro: Bool) -> some View {
        CSPaperSection(title: "Scene schedule", note: CSLayout.note(.scenes, doc, readOnly: ro)) {
            if doc.scenes.isEmpty {
                Text("No scenes on this day.").foregroundStyle(.gray).accessibilityIdentifier("callsheet-no-scenes")
            } else {
                Grid(alignment: .topLeading, horizontalSpacing: 8, verticalSpacing: 3) {
                    GridRow {
                        ForEach(CSLayout.sceneColumns, id: \.self) {
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

    // MARK: Talent / Cast

    private func talent(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Talent / Cast", note: CSLayout.note(.talent, doc, readOnly: ro)) {
            if doc.cast.isEmpty {
                Text("No cast in these scenes.").foregroundStyle(.gray).accessibilityIdentifier("callsheet-no-cast")
            } else {
                Grid(alignment: .topLeading, horizontalSpacing: 8, verticalSpacing: 3) {
                    GridRow {
                        ForEach(CSLayout.talentColumns, id: \.self) {
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
                            VStack(alignment: .leading, spacing: 2) {
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
            need: need, placeholder: placeholder, disabled: ro, minWidth: 48)
    }

    // MARK: Practical / Safety information

    /// As the PDF: the production emergency contact and each location's nearest hospital come first
    /// as cards, then one card with the practical notes, the day notes and the custom fields.
    private func practical(_ doc: CallSheetDocument, _ ro: Bool) -> some View {
        CSPaperSection(title: "Practical / Safety information", note: CSLayout.note(.practical, doc, readOnly: ro)) {
            VStack(alignment: .leading, spacing: 8) {
                CSWrap(minWidth: 260, rowSpacing: 8) {
                    CSCard {
                        CSEyebrow(text: "Production emergency contact")
                        CSEdit(label: "Production emergency contact", text: model.bind(\.practical.emergencyContact), need: true, disabled: ro, wide: true)
                    }
                    ForEach(Array(doc.locations.enumerated()), id: \.offset) { i, l in
                        CSCard {
                            CSEyebrow(text: "\(l.name) \u{00B7} Nearest hospital")
                            CSEdit(
                                label: "\(l.name) \u{00B7} Nearest hospital",
                                text: model.bind({ $0.locations[safe: i]?.hospital ?? "" }, { d, v in if d.locations.indices.contains(i) { d.locations[i].hospital = v } }),
                                need: true, disabled: ro, wide: true)
                        }
                    }
                }
                CSCard {
                    ForEach(CSLayout.practicalFields, id: \.0) { label, kp in
                        CSLabeled(label: label, text: model.bind(\.practical, kp), disabled: ro)
                    }
                    area("Special instructions", model.bind(\.practical.specialInstructions), ro)
                    area("Production notes", model.bind(\.practical.productionNotes), ro)
                    area("Day notes", model.bind(\.dayNotes), ro)
                    ForEach(Array(doc.extraFields.enumerated()), id: \.offset) { i, f in
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
                        .padding(.vertical, 1.5)
                    }
                }
            }
        }
    }

    private func area(_ label: String, _ text: Binding<String>, _ ro: Bool) -> some View {
        HStack(alignment: .top, spacing: 6) {
            Text(label).font(.system(size: 11.5, weight: .semibold)).frame(minWidth: 128, alignment: .leading)
            CSArea(label: label, text: text, disabled: ro)
        }
        .padding(.vertical, 1.5)
    }

    // MARK: Added sections

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
                ForEach(CSLayout.advanceFields, id: \.0) { label, need, kp in
                    CSLabeled(
                        label: label,
                        text: model.bind({ $0.optional.advance?[keyPath: kp] ?? "" }, { d, v in if d.optional.advance != nil { d.optional.advance![keyPath: kp] = v } }),
                        need: need, disabled: ro)
                }
            }
        }
    }

    private func crewSection(_ doc: CallSheetDocument, _ rows: [CallSheetCrewMember], _ ro: Bool) -> some View {
        CSPaperSection(title: "Crew", note: CSLayout.note(.crew, doc, readOnly: ro), onRemove: { model.edit { $0.optional.crew = nil } }, disabled: ro) {
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
                ForEach(CSLayout.communicationFields, id: \.0) { label, kp in
                    CSLabeled(
                        label: label,
                        text: model.bind({ $0.optional.communication?[keyPath: kp] ?? "" }, { d, v in if d.optional.communication != nil { d.optional.communication![keyPath: kp] = v } }),
                        disabled: ro)
                }
            }
        }
    }

    /// The PDF prints the attachment names and the pictures under one heading; each block keeps its
    /// own heading and remove button.
    private func attachmentsAndImages(_ files: [CallSheetAttachment]?, _ images: [CallSheetAttachment]?, _ ro: Bool) -> some View {
        CSPaperSection(title: "Attachments & reference images") {
            VStack(alignment: .leading, spacing: 8) {
                if let files { attachmentsSection(files, ro) }
                if let images { referenceSection(images, ro) }
            }
        }
    }

    private func subHead(_ title: String, _ ro: Bool, _ remove: @escaping () -> Void) -> some View {
        HStack {
            Text(title).fontWeight(.bold)
            Spacer(minLength: 4)
            CSRemoveSection(title: title, onRemove: remove, disabled: ro)
        }
    }

    private func attachmentsSection(_ items: [CallSheetAttachment], _ ro: Bool) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            subHead("Attachments", ro) { model.edit { $0.optional.attachments = nil } }
            FlowLayout(spacing: 8) {
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
        VStack(alignment: .leading, spacing: 4) {
            subHead("Reference images", ro) { model.edit { $0.optional.referenceImages = nil } }
            FlowLayout(spacing: 8) {
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

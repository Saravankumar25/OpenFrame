// Small views shared by the Breakdown tabs: chips, banners, the sheet frame, the notice line, the
// category mark (colour with icon, text and tooltip), fields that save when you leave them, scene
// chips, pictures from managed assets, the availability editor and the "what still uses this"
// note. Colours come from the theme; the category colours come from the core.

import AppKit
import SwiftUI
import OpenFrameCore

// MARK: Chips and banners

enum BDTone {
    case plain, accent, green, yellow, red, blue, purple

    var tokens: (bg: String, fg: String) {
        switch self {
        case .plain: ("surface2", "ink2")
        case .accent: ("accent-soft", "accent-ink")
        case .green: ("green-soft", "green-t")
        case .yellow: ("yellow-soft", "yellow-t")
        case .red: ("red-soft", "red-t")
        case .blue: ("blue-soft", "blue-t")
        case .purple: ("purple-soft", "purple-t")
        }
    }
}

struct BDChip: View {
    let text: String
    var tone: BDTone = .plain
    var symbol: String?
    @Environment(\.ofTheme) private var theme

    var body: some View {
        HStack(spacing: 3) {
            if let symbol { Image(systemName: symbol).imageScale(.small).accessibilityHidden(true) }
            Text(text).lineLimit(1)
        }
        .font(.caption.weight(.medium))
        .foregroundStyle(theme[tone.tokens.fg])
        .padding(.horizontal, 7)
        .padding(.vertical, 2)
        .background(theme[tone.tokens.bg], in: Capsule())
        .fixedSize()
    }
}

struct BDBanner<Actions: View>: View {
    enum Tone { case info, warn, err, ok }
    let tone: Tone
    @ViewBuilder var content: () -> Text
    @ViewBuilder var actions: () -> Actions
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let (bg, fg, symbol): (String, String, String) = switch tone {
        case .info: ("blue-soft", "blue-t", "info.circle")
        case .warn: ("yellow-soft", "yellow-t", "exclamationmark.triangle")
        case .err: ("red-soft", "red-t", "xmark.octagon")
        case .ok: ("green-soft", "green-t", "checkmark.circle")
        }
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: symbol).accessibilityHidden(true)
            content().fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 8)
            actions()
        }
        .font(.callout)
        .foregroundStyle(theme[fg])
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme[bg], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .accessibilityElement(children: .contain)
    }
}

extension BDBanner where Actions == EmptyView {
    init(_ tone: Tone, _ text: String) {
        self.init(tone: tone, content: { Text(text) }, actions: { EmptyView() })
    }
}

// MARK: Sheet frame

/// The frame of every sheet: a title, an optional line under it, the content and a row of buttons.
struct BDSheetFrame<Content: View, Footer: View, Leading: View>: View {
    let title: String
    var sub: String?
    var width: CGFloat = 520
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @ViewBuilder var leading: () -> Leading
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            VStack(spacing: PageSpacing.tight) {
                Text(title).font(.title3.weight(.semibold)).multilineTextAlignment(.center).accessibilityAddTraits(.isHeader)
                if let sub { Text(sub).font(.callout).foregroundStyle(.secondary).multilineTextAlignment(.center).fixedSize(horizontal: false, vertical: true) }
            }
            .frame(maxWidth: .infinity)
            content()
            HStack(spacing: 8) {
                leading()
                Spacer(minLength: 0)
                footer()
            }
        }
        .padding(PageSpacing.large)
        .frame(width: width)
        .background(theme["panel"])
    }
}

extension BDSheetFrame where Leading == EmptyView {
    init(title: String, sub: String? = nil, width: CGFloat = 520,
         @ViewBuilder content: @escaping () -> Content, @ViewBuilder footer: @escaping () -> Footer) {
        self.init(title: title, sub: sub, width: width, content: content, footer: footer, leading: { EmptyView() })
    }
}

/// The confirmation sheet for deleting or archiving, with the consequence in plain words.
struct BDConfirmSheet<Detail: View>: View {
    let title: String
    let confirmLabel: String
    var danger = false
    let onConfirm: () -> Void
    let onCancel: () -> Void
    @ViewBuilder var detail: () -> Detail

    var body: some View {
        BDSheetFrame(title: title, width: 460) {
            detail()
        } footer: {
            Button("Cancel", action: onCancel).keyboardShortcut(.cancelAction)
            Button(confirmLabel, role: danger ? .destructive : nil, action: onConfirm)
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("confirm-\(confirmLabel)")
        }
    }
}

// MARK: Notice line

struct BDNoticeBar: View {
    @Bindable var hub: BreakdownHub
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Group {
            if let n = hub.notice {
                HStack(spacing: 12) {
                    Text(n.text).lineLimit(2).accessibilityIdentifier("breakdown-notice")
                    if n.undoable {
                        Button("Undo") { Task { await hub.undoLast() } }
                            .buttonStyle(.link)
                            .accessibilityIdentifier("breakdown-notice-undo")
                    }
                    Button { hub.dismissNotice() } label: { Label("Dismiss", systemImage: "xmark") }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Dismiss")
                }
                .font(.callout)
                .padding(.horizontal, 14)
                .padding(.vertical, 9)
                .background(theme["panel"], in: Capsule())
                .overlay(Capsule().strokeBorder(theme["line2"]))
                .shadow(color: .black.opacity(0.12), radius: 6, y: 2)
                .padding(.bottom, 14)
                .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
                .accessibilityElement(children: .contain)
                .accessibilityAddTraits(.updatesFrequently)
            }
        }
        .animation(reduceMotion ? nil : .spring(response: 0.3, dampingFraction: 1), value: hub.notice)
    }
}

// MARK: Category mark

/// A colour dot with an icon and the category's name. The tooltip and the accessible name carry
/// the core's description, so colour is never the only signal.
struct CategoryMark: View {
    let look: CategoryLook
    var showsLabel = true
    var labelOverride: String?
    var size: CGFloat = 12
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let tint = look.color ?? theme["muted"]
        HStack(spacing: 5) {
            Circle().fill(tint).frame(width: 8, height: 8).accessibilityHidden(true)
            Image(systemName: look.symbol).font(.system(size: size)).foregroundStyle(tint).accessibilityHidden(true)
            if showsLabel { Text(labelOverride ?? look.label).lineLimit(1) }
        }
        .help(look.tooltip)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(labelOverride ?? look.label)
        .accessibilityHint(look.tooltip)
        .accessibilityIdentifier("category-mark-\(look.category.rawValue)")
    }
}

// MARK: Fields

/// A labelled field of a drawer.
struct BDField<Content: View>: View {
    let label: String
    var hint: String?
    @ViewBuilder var content: () -> Content
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.caption.weight(.semibold)).foregroundStyle(theme["muted"])
            content()
            if let hint { Text(hint).font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true) }
        }
    }
}

/// A text field that keeps a local copy while you type and saves when you leave it, press Return
/// (one line) or Tab. Escape puts the saved text back. The core holds the truth.
struct BufferedField: View {
    let id: String
    let label: String
    let value: String?
    var multiline = false
    var required = false
    var placeholder = ""
    var disabled = false
    var hint: String?
    let onCommit: (String) -> Void

    @State private var draft = ""
    @State private var shown: String?
    @FocusState private var focused: Bool

    var body: some View {
        BDField(label: label, hint: hint) {
            TextField(placeholder, text: $draft, axis: multiline ? .vertical : .horizontal)
                .lineLimit(multiline ? 3...8 : 1...1)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .disabled(disabled)
                .onSubmit { if !multiline { commit() } }
                .onExitCommand {
                    draft = value ?? ""
                    focused = false
                }
                .accessibilityLabel(label)
                .accessibilityIdentifier(id)
        }
        .onAppear { draft = value ?? ""; shown = value }
        .onChange(of: value) { _, new in
            if !focused { draft = new ?? ""; shown = new }
        }
        .onChange(of: focused) { _, now in if !now { commit() } }
    }

    private func commit() {
        let v = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        if v == (value ?? "").trimmingCharacters(in: .whitespacesAndNewlines) { return }
        if required && v.isEmpty {
            draft = value ?? ""
            return
        }
        onCommit(v)
    }
}

/// A contact field. Roles that may not see contacts get a plain statement instead.
struct ContactField: View {
    let id: String
    let label: String
    let value: String?
    let hidden: Bool
    let canEdit: Bool
    let onCommit: (String) -> Void

    var body: some View {
        if hidden {
            BDField(label: label) {
                Text("Contact details are hidden for your role.").font(.callout).foregroundStyle(.secondary)
            }
        } else {
            BufferedField(id: id, label: label, value: value, disabled: !canEdit,
                          hint: "Private: visible to Owners and Editors only. Never shared with the assistant or put in exports.",
                          onCommit: onCommit)
        }
    }
}

// MARK: Scene chips

/// Scene chips; a click opens that scene's breakdown (FSD 28.4).
struct SceneChips: View {
    let scenes: [ProductionSceneRef]
    var empty = "Not used in any scene yet."
    @Environment(BreakdownHub.self) private var hub

    var body: some View {
        if scenes.isEmpty {
            Text(empty).font(.callout).foregroundStyle(.secondary)
        } else {
            BDFlowLayout(spacing: 6) {
                ForEach(scenes, id: \.sceneId) { s in
                    Button { hub.openScene(s.sceneId) } label: {
                        BDChip(text: s.chipLabel)
                    }
                    .buttonStyle(.plain)
                    .help(s.inSource ? s.heading : "\(s.heading) (scene removed from current source)")
                    .accessibilityLabel(s.chipLabel)
                    .accessibilityHint("Opens the breakdown of this scene")
                }
            }
        }
    }
}

/// Lays children out in rows, wrapping to the next row when a row is full.
struct BDFlowLayout: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 400
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0, maxX: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > 0, x + size.width > width { x = 0; y += rowHeight + spacing; rowHeight = 0 }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
            maxX = max(maxX, x - spacing)
        }
        return CGSize(width: proposal.width ?? maxX, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowHeight: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > bounds.minX, x + size.width > bounds.maxX { x = bounds.minX; y += rowHeight + spacing; rowHeight = 0 }
            s.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}

// MARK: Pictures

/// A picture from a managed asset (read and reduced off the main thread, then cached).
struct AssetImageView: View {
    let asset: AssetInfo?
    var maxPixel = 480
    var fill = true
    var stand: String = "photo"
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var image: NSImage?
    @State private var failed = false

    var body: some View {
        ZStack {
            theme["surface2"]
            if let image {
                Image(nsImage: image).resizable().aspectRatio(contentMode: fill ? .fill : .fit)
            } else {
                Image(systemName: failed || (asset != nil && asset?.available == false) ? "photo.badge.exclamationmark" : stand)
                    .font(.title3)
                    .foregroundStyle(theme["muted"])
            }
        }
        .clipped()
        .accessibilityHidden(true)
        .task(id: asset?.id) {
            image = nil
            failed = false
            guard let asset, asset.available, let core = app.core else { return }
            if let loaded = await VaultThumbnails.shared.image(core: core, assetId: asset.id, global: false, maxPixel: maxPixel) {
                image = loaded
            } else {
                failed = true
            }
        }
    }
}

enum ImagePicker {
    @MainActor
    static func choose(multiple: Bool) -> [String] {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.png, .jpeg, .gif, .webP, .bmp]
        panel.allowsMultipleSelection = multiple
        panel.canChooseDirectories = false
        panel.prompt = "Choose"
        guard panel.runModal() == .OK else { return [] }
        return panel.urls.map(\.path)
    }
}

// MARK: Availability

/// Dated availability (inclusive ranges). Saving replaces the owner's list as one undo step.
struct AvailabilityEditor: View {
    let owner: AvailabilityOwner
    let ownerId: String
    let windows: [AvailabilityWindowDto]
    let canEdit: Bool
    let name: String
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme
    @State private var kind: AvailabilityKind = .unavailable
    @State private var start = Date()
    @State private var end = Date()
    @State private var endTouched = false
    @State private var note = ""
    @State private var busy = false

    var body: some View {
        BDField(label: "Availability", hint: "Dates are advisory: the schedule warns about them and never moves a scene by itself.") {
            VStack(alignment: .leading, spacing: 6) {
                if windows.isEmpty { Text("No dates entered.").font(.caption).foregroundStyle(theme["muted"]) }
                ForEach(windows, id: \.id) { w in
                    HStack(spacing: 6) {
                        BDChip(text: w.kind.title, tone: w.kind == .unavailable ? .red : .green)
                        Text(AvailabilityDates.range(w) + (w.note.map { ": \($0)" } ?? "")).font(.callout)
                        Spacer(minLength: 0)
                        if canEdit {
                            Button {
                                Task { busy = true; await hub.removeAvailability(owner: owner, ownerId: ownerId, existing: windows, windowId: w.id, name: name); busy = false }
                            } label: { Label("Remove", systemImage: "xmark") }
                                .buttonStyle(.borderless)
                                .disabled(busy)
                                .accessibilityLabel("Remove \(AvailabilityDates.range(w))")
                        }
                    }
                    .accessibilityElement(children: .combine)
                }
                if canEdit {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 8) {
                            Picker("Availability", selection: $kind) {
                                ForEach(AvailabilityKind.allCases, id: \.self) { Text($0.title).tag($0) }
                            }
                            .labelsHidden()
                            .fixedSize()
                            .accessibilityIdentifier("av-\(owner.rawValue)-kind")
                            DatePicker("First day", selection: $start, displayedComponents: .date)
                                .labelsHidden()
                                .datePickerStyle(.field)
                                .accessibilityLabel("First day")
                                .accessibilityIdentifier("av-\(owner.rawValue)-start")
                                .onChange(of: start) { _, s in if !endTouched || end < s { end = s } }
                            Text("to").foregroundStyle(theme["muted"])
                            DatePicker("Last day", selection: Binding(get: { end }, set: { end = $0; endTouched = true }),
                                       in: start..., displayedComponents: .date)
                                .labelsHidden()
                                .datePickerStyle(.field)
                                .accessibilityLabel("Last day")
                                .accessibilityIdentifier("av-\(owner.rawValue)-end")
                        }
                        HStack(spacing: 8) {
                            TextField("Note (optional)", text: $note)
                                .textFieldStyle(.roundedBorder)
                                .accessibilityLabel("Availability note")
                                .accessibilityIdentifier("av-\(owner.rawValue)-note")
                            Button("Add Dates") {
                                Task {
                                    busy = true
                                    let ok = await hub.addAvailability(owner: owner, ownerId: ownerId, existing: windows, name: name,
                                                                      start: AvailabilityDates.format(start), end: AvailabilityDates.format(max(start, end)),
                                                                      kind: kind, note: note)
                                    if ok { note = "" }
                                    busy = false
                                }
                            }
                            .disabled(busy)
                            .accessibilityIdentifier("av-add-dates")
                        }
                    }
                    .padding(.top, 4)
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Availability of \(name)")
    }
}

// MARK: What still uses a record

/// What still refers to a record, and what deleting it does (the stable identity rule).
struct ReferencesNote: View {
    let kind: ProductionRecordKind
    let id: String
    @Environment(BreakdownHub.self) private var hub
    @State private var refs: ProductionReferences?
    @State private var loading = true

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if loading {
                ProgressView().controlSize(.small)
            } else if let r = refs {
                Text(r.consequence).font(.callout).fixedSize(horizontal: false, vertical: true)
                if !r.scenes.isEmpty {
                    Text("Used in").font(.caption).foregroundStyle(.secondary)
                    SceneChipsStatic(scenes: r.scenes)
                }
                ForEach(r.links, id: \.self) { Text("\u{2022} \($0)").font(.callout) }
            }
        }
        .task(id: id) {
            loading = true
            refs = await hub.references(kind: kind, id: id)
            loading = false
        }
    }
}

/// Scene chips that do not navigate (inside a sheet).
struct SceneChipsStatic: View {
    let scenes: [ProductionSceneRef]
    var body: some View {
        BDFlowLayout(spacing: 6) {
            ForEach(scenes, id: \.sceneId) { BDChip(text: $0.chipLabel) }
        }
    }
}

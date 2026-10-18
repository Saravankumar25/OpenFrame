// Small views shared by the Story screens: inline text fields, the sheet frame, the notice line,
// the drag and drop plumbing and the inspector's text fields.

import AppKit
import SwiftUI
import UniformTypeIdentifiers
import OpenFrameCore

// MARK: Theme

extension ResolvedTheme {
    /// The colour of a card label name such as "orange".
    func label(_ name: String) -> Color { self[StoryLogic.colorToken(name)] }
}

// MARK: Inline text

/// A one-line or multi-line field that opens focused. Return saves, Escape cancels, and leaving
/// the field saves (as on Windows). It reports once.
struct InlineField: View {
    let initial: String
    let label: String
    var placeholder = ""
    var multiline = false
    let onCommit: (String) -> Void
    let onCancel: () -> Void

    @State private var text = ""
    @State private var done = false
    @FocusState private var focused: Bool

    var body: some View {
        TextField(placeholder, text: $text, axis: multiline ? .vertical : .horizontal)
            .lineLimit(multiline ? 1...5 : 1...1)
            .textFieldStyle(.plain)
            .focused($focused)
            .onSubmit { commit() }
            .onExitCommand {
                done = true
                onCancel()
            }
            .onChange(of: focused) { _, now in if !now { commit() } }
            .onAppear {
                text = initial
                focused = true
            }
            .accessibilityLabel(label)
            .accessibilityIdentifier("inline-\(label)")
    }

    private func commit() {
        guard !done else { return }
        done = true
        onCommit(text)
    }
}

// MARK: Sheet frame

/// The frame of every Story sheet: a title, an optional line under it, the content and a row of
/// buttons. Escape cancels; the default button answers Return.
struct StorySheetFrame<Content: View, Footer: View>: View {
    let title: String
    var sub: String?
    var width: CGFloat = 480
    @ViewBuilder var content: () -> Content
    @ViewBuilder var footer: () -> Footer
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.title3.weight(.semibold)).accessibilityAddTraits(.isHeader)
                if let sub { Text(sub).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
            }
            content()
            HStack(spacing: 8) {
                Spacer(minLength: 0)
                footer()
            }
        }
        .padding(22)
        .frame(width: width)
        .background(theme["panel"])
    }
}

/// A notice banner: info, warning, error or ok.
struct StoryBanner: View {
    enum Tone { case info, warn, err, ok }
    let tone: Tone
    let text: String
    @Environment(\.ofTheme) private var theme

    var body: some View {
        let (bg, fg, symbol): (String, String, String) = switch tone {
        case .info: ("blue-soft", "blue-t", "info.circle")
        case .warn: ("yellow-soft", "yellow-t", "exclamationmark.triangle")
        case .err: ("red-soft", "red-t", "xmark.octagon")
        case .ok: ("green-soft", "green-t", "checkmark.circle")
        }
        Label {
            Text(text).fixedSize(horizontal: false, vertical: true)
        } icon: {
            Image(systemName: symbol)
        }
        .font(.callout)
        .foregroundStyle(theme[fg])
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme[bg], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .accessibilityElement(children: .combine)
    }
}

// MARK: Notice line

struct StoryNoticeBar: View {
    @Bindable var model: StoryModel
    @Environment(\.ofTheme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Group {
            if let n = model.notice {
                HStack(spacing: 12) {
                    Text(n.text).lineLimit(2).accessibilityIdentifier("story-notice")
                    if n.undoable {
                        Button("Undo") { Task { await model.undoLast() } }
                            .buttonStyle(.link)
                            .accessibilityIdentifier("story-notice-undo")
                    }
                    Button { model.dismissNotice() } label: { Image(systemName: "xmark") }
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
        .animation(reduceMotion ? nil : .spring(response: 0.3, dampingFraction: 1), value: model.notice)
    }
}

// MARK: Saved status

struct StorySavedStatus: View {
    let status: StorySaveStatus
    @Environment(\.ofTheme) private var theme

    var body: some View {
        Text(status.state.text)
            .font(.caption)
            .foregroundStyle(status.state == .error ? theme["red-t"] : theme["muted"])
            .accessibilityIdentifier("story-save-status")
            .accessibilityLabel(status.state.text.isEmpty ? "" : "Status: \(status.state.text)")
    }
}

// MARK: Inspector text fields

/// A labelled field of the inspector that saves through a `FieldSaver`.
struct SavedField: View {
    let label: String
    let saver: FieldSaver
    var rows = 3
    var placeholder = ""
    var hint: String?
    var multiline = true
    var limit: Int?
    @Environment(\.ofTheme) private var theme
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.callout.weight(.medium))
            Group {
                if multiline {
                    TextEditor(text: Binding(get: { saver.text }, set: { change($0) }))
                        .scrollContentBackground(.hidden)
                        .font(.body)
                        .frame(minHeight: CGFloat(rows) * 19 + 8)
                        .padding(4)
                        .overlay(alignment: .topLeading) {
                            if saver.text.isEmpty && !placeholder.isEmpty {
                                Text(placeholder).foregroundStyle(.tertiary).padding(.horizontal, 9).padding(.vertical, 12).allowsHitTesting(false)
                            }
                        }
                } else {
                    TextField(placeholder, text: Binding(get: { saver.text }, set: { change($0) }))
                        .textFieldStyle(.plain)
                        .padding(8)
                }
            }
            .focused($focused)
            .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(focused ? theme["accent"] : theme["line"]))
            .accessibilityLabel(label)
            .accessibilityIdentifier("story-field-\(label)")
            if let hint { Text(hint).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
        }
        .onChange(of: focused) { _, now in
            saver.focused = now
            if !now { Task { await saver.flush() } }
        }
        .onDisappear { Task { await saver.flush() } }
    }

    private func change(_ next: String) {
        if let limit, next.count > limit { return }
        saver.edit(next)
    }
}

// MARK: Attachments

struct StoryAttachmentsField: View {
    @Bindable var story: StoryModel
    let items: [StoryAttachmentDto]
    let ownerType: String
    let ownerId: String
    @State private var importing = false

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Attachments").font(.callout.weight(.medium))
            Text("Reference material only; attachments never become production assets automatically. Up to \(CardDrawerModel.attachmentsMax) per card.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            FlowLayout(spacing: 6) {
                ForEach(items, id: \.id) { a in
                    HStack(spacing: 4) {
                        Button {
                            if let p = a.asset.path { NSWorkspace.shared.open(URL(fileURLWithPath: p)) }
                        } label: {
                            Label(a.asset.originalName, systemImage: "doc")
                        }
                        .buttonStyle(.borderless)
                        .disabled(!a.asset.available)
                        .help(a.asset.available ? a.asset.originalName : "The file is missing")
                        Button { Task { await story.removeAttachment(a.id) } } label: { Image(systemName: "xmark.circle") }
                            .buttonStyle(.borderless)
                            .accessibilityLabel("Remove \(a.asset.originalName)")
                    }
                    .padding(.horizontal, 8).padding(.vertical, 3)
                    .background(.quaternary, in: Capsule())
                }
                Button("Attach\u{2026}") { importing = true }
                    .controlSize(.small)
                    .disabled(items.count >= CardDrawerModel.attachmentsMax)
            }
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.item]) { result in
            if case let .success(url) = result {
                let scoped = url.startAccessingSecurityScopedResource()
                Task {
                    await story.addAttachment(ownerType: ownerType, ownerId: ownerId, path: url.path)
                    if scoped { url.stopAccessingSecurityScopedResource() }
                }
            }
        }
    }
}

/// Wraps its children onto new lines.
struct FlowLayout: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowH: CGFloat = 0, maxW: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x > 0, x + s.width > width { x = 0; y += rowH + spacing; rowH = 0 }
            x += s.width + spacing
            rowH = max(rowH, s.height)
            maxW = max(maxW, x - spacing)
        }
        return CGSize(width: proposal.width ?? maxW, height: y + rowH)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowH: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x > bounds.minX, x + s.width > bounds.maxX { x = bounds.minX; y += rowH + spacing; rowH = 0 }
            v.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(s))
            x += s.width + spacing
            rowH = max(rowH, s.height)
        }
    }
}

// MARK: Drag and drop

enum StoryDragType {
    /// The kind of every Story drag. The dragged items themselves live in the model.
    static let utType = UTType(exportedAs: "studio.openframe.story-item", conformingTo: .data)

    static func provider() -> NSItemProvider {
        NSItemProvider(item: Data("story-item".utf8) as NSData, typeIdentifier: utType.identifier)
    }
}

/// Reports where the pointer is over a zone and commits on release. Which zone it is can depend
/// on what is dragged (an act column accepts acts as columns and cards as contents).
struct StoryDropDelegate: DropDelegate {
    let model: StoryModel
    let zone: () -> StoryDropZone
    let size: CGSize
    var vertical = true

    func validateDrop(info: DropInfo) -> Bool {
        info.hasItemsConforming(to: [StoryDragType.utType]) && model.accepts(zone())
    }

    func dropEntered(info: DropInfo) { update(info) }

    func dropUpdated(info: DropInfo) -> DropProposal? {
        update(info)
        return DropProposal(operation: model.accepts(zone()) ? .move : .forbidden)
    }

    func dropExited(info: DropInfo) { model.leave(zone()) }

    func performDrop(info: DropInfo) -> Bool {
        let ok = model.accepts(zone())
        Task { await model.commitDrop() }
        return ok
    }

    private func update(_ info: DropInfo) {
        let first = vertical ? info.location.y < size.height / 2 : info.location.x < size.width / 2
        model.hover(zone(), firstHalf: first)
    }
}

extension View {
    /// Makes the view a drop zone of the Story Board.
    func storyDrop(_ model: StoryModel, size: CGSize, vertical: Bool = true, zone: @escaping () -> StoryDropZone) -> some View {
        onDrop(of: [StoryDragType.utType], delegate: StoryDropDelegate(model: model, zone: zone, size: size, vertical: vertical))
    }
}

/// Measures a view's size into a binding.
struct SizeReader: ViewModifier {
    @Binding var size: CGSize
    func body(content: Content) -> some View {
        content.background(GeometryReader { g in
            Color.clear
                .onAppear { size = g.size }
                .onChange(of: g.size) { _, new in size = new }
        })
    }
}

extension View {
    func reading(size: Binding<CGSize>) -> some View { modifier(SizeReader(size: size)) }
}

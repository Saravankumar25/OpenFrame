// The review viewer: scene list, the read-only page, and the comment column.

import AppKit
import SwiftUI
import OpenFrameCore

/// The page of one scene as styled, selectable, read-only text. The selection is how a reviewer
/// quotes a passage. Custom elements keep the look their author chose.
struct PKScenePaper: NSViewRepresentable {
    let scene: PackageReviewScene
    @Binding var selection: String

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSTextView.scrollableTextView()
        guard let text = scroll.documentView as? NSTextView else { return scroll }
        text.isEditable = false
        text.isSelectable = true
        text.drawsBackground = true
        text.backgroundColor = .textBackgroundColor
        text.textContainerInset = NSSize(width: 36, height: 28)
        text.delegate = context.coordinator
        text.setAccessibilityLabel("Scene text")
        text.setAccessibilityIdentifier("review-scene-text")
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let text = scroll.documentView as? NSTextView else { return }
        let built = Self.attributed(scene)
        if text.string != built.string { text.textStorage?.setAttributedString(built) }
    }

    static func attributed(_ scene: PackageReviewScene) -> NSAttributedString {
        let out = NSMutableAttributedString()
        let mono = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        func add(_ text: String, indent: CGFloat, width: CGFloat?, bold: Bool = false, italic: Bool = false, upper: Bool = false, center: Bool = false) {
            let p = NSMutableParagraphStyle()
            p.firstLineHeadIndent = indent
            p.headIndent = indent
            if let width { p.tailIndent = -max(0, 470 - indent - width) }
            p.alignment = center ? .center : .left
            p.paragraphSpacing = 10
            var font = bold ? NSFont.monospacedSystemFont(ofSize: 13, weight: .bold) : mono
            if italic { font = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask) }
            out.append(NSAttributedString(string: (upper ? text.uppercased() : text) + "\n", attributes: [
                .font: font, .paragraphStyle: p, .foregroundColor: NSColor.textColor,
            ]))
        }
        add("\(scene.label)  \(scene.heading)", indent: 0, width: nil, bold: true, upper: true)
        for e in scene.elements where e.elementType != "scene_heading" {
            let t = e.text.isEmpty ? " " : e.text
            switch e.elementType {
            case "character": add(t, indent: 150, width: nil, upper: true)
            case "dialogue": add(t, indent: 90, width: 270)
            case "parenthetical": add(t, indent: 125, width: 200, italic: true)
            case "transition": add(t, indent: 300, width: nil, upper: true)
            case "centered": add(t, indent: 0, width: nil, italic: e.italic, upper: e.uppercase, center: true)
            default: add(t, indent: 0, width: nil, italic: e.italic, upper: e.uppercase)
            }
        }
        return out
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: PKScenePaper
        init(_ parent: PKScenePaper) { self.parent = parent }

        @MainActor func textViewDidChangeSelection(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            let r = tv.selectedRange()
            let s = r.length > 0 ? (tv.string as NSString).substring(with: r) : ""
            if s != parent.selection { parent.selection = s }
        }
    }
}

struct ReviewViewerSheet: View {
    @State var model: ReviewViewerModel
    var picker: FilePicking = PanelFilePicker()
    let close: () -> Void
    @State private var selection = ""
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            topBanner
            if let ws = model.workspace {
                if ws.scenes.isEmpty {
                    VStack(spacing: 6) {
                        Text(ws.typeLabel).font(.headline)
                        Text(ws.summary.map { "\(Int($0.count)) \($0.label.lowercased())" }.joined(separator: " \u{00B7} ") + " from \u{201C}\(ws.projectTitle)\u{201D}.")
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .accessibilityIdentifier("review-summary")
                } else {
                    HStack(spacing: 0) {
                        sceneList(ws)
                        Divider()
                        paper
                        Divider()
                        commentsColumn(ws)
                    }
                }
            } else if let e = model.errorText {
                IXBanner(.err, e).padding(16).accessibilityIdentifier("review-error")
                Spacer()
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(width: 1040, height: 680)
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Review package")
        .accessibilityIdentifier("review-viewer")
        .task { await model.load() }
        .onDisappear { model.finish() }
    }

    private var topBanner: some View {
        VStack(spacing: 6) {
            IXBanner(tone: .info, text: Text(model.banner).bold() + Text(model.workspace.map { "  \($0.draftName.isEmpty ? $0.typeLabel : $0.draftName) \u{00B7} exported by \($0.exportedBy) \u{00B7} \(PKText.day(ms: $0.exportedAt))" } ?? "")) {
                if model.canComment {
                    Button("Export Response Package") {
                        if let url = picker.saveFile(title: "Export Response Package", suggestedName: model.suggestedResponseName, directory: nil, message: nil) {
                            Task { await model.exportResponse(to: url) }
                        }
                    }
                    .controlSize(.small).buttonStyle(.borderedProminent)
                    .disabled(model.myCount == 0)
                    .help(model.myCount == 0 ? "Add a comment first" : "")
                    .accessibilityIdentifier("review-export-response")
                }
                Button { close() } label: { Image(systemName: "xmark") }
                    .buttonStyle(.borderless).keyboardShortcut(.cancelAction)
                    .accessibilityLabel("Close review").accessibilityIdentifier("review-close")
            }
            if let p = model.exportedPath {
                PKDoneBanner(title: "Response package exported. Send it back to the author.", path: p, app: model.app)
            }
        }
        .padding(10)
    }

    private func sceneList(_ ws: PackageReviewWorkspace) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 2) {
                HStack { Text(ws.draftName).font(.callout.weight(.semibold)); IXChip(text: "Review package", tone: .blue) }
                    .padding(.bottom, 6)
                ForEach(ws.scenes, id: \.id) { s in
                    let on = model.scene?.id == s.id
                    Button { model.sceneId = s.id } label: {
                        HStack {
                            Text(s.label).font(.caption).foregroundStyle(.secondary).frame(minWidth: 24, alignment: .leading)
                            Text(s.heading.isEmpty ? "(untitled scene)" : s.heading).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                            if model.count(for: s.id) > 0 { IXChip(text: "\(model.count(for: s.id))") }
                        }
                        .padding(.horizontal, 8).padding(.vertical, 5)
                        .background(on ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(on ? .isSelected : [])
                    .accessibilityIdentifier("review-scene-\(s.id)")
                }
            }
            .padding(8)
        }
        .frame(width: 250)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Scenes")
        .accessibilityIdentifier("review-scenes")
    }

    @ViewBuilder private var paper: some View {
        if let scene = model.scene {
            PKScenePaper(scene: scene, selection: $selection)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .id(scene.id)
        } else {
            Spacer()
        }
    }

    private func commentsColumn(_ ws: PackageReviewWorkspace) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text("Comments on Scene \(model.scene?.label ?? "")").font(.callout.weight(.semibold))
                if model.comments.isEmpty { Text("No comments on this scene yet.").font(.callout).foregroundStyle(.secondary) }
                ForEach(model.comments, id: \.id) { c in commentCard(c, ws) }
                if ws.canComment, model.scene != nil { composer }
                if !ws.canComment { Text("Read-only. Nothing here changes your project.").font(.caption).foregroundStyle(.secondary) }
                if let e = model.errorText { IXBanner(.err, e).accessibilityIdentifier("review-error") }
            }
            .padding(12)
        }
        .frame(width: 320)
        .background(theme["panel"])
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Comments")
        .accessibilityIdentifier("review-comments")
    }

    private func commentCard(_ c: PackageReviewComment, _ ws: PackageReviewWorkspace) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(c.authorName).fontWeight(.semibold)
                Text(PKText.day(ms: c.createdAt)).foregroundStyle(.secondary)
                Spacer()
                if c.mine { IXChip(text: "Yours", tone: .green) }
                if c.mine && ws.canComment {
                    Button { Task { await model.deleteComment(c.id) } } label: { Image(systemName: "trash") }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Delete this comment").accessibilityIdentifier("review-delete-\(c.id)")
                }
            }
            .font(.caption)
            if let q = c.quotedText { Text("\u{201C}\(q)\u{201D}").font(.caption).italic().foregroundStyle(.secondary) }
            Text(c.body).font(.callout).textSelection(.enabled)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("review-comment-\(c.id)")
    }

    private var composer: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Add a comment").font(.callout.weight(.semibold))
                Spacer()
                Button("Quote selection") { model.quoteSelection(selection) }.controlSize(.small)
                    .accessibilityIdentifier("review-quote-selection")
            }
            if !model.quote.isEmpty {
                HStack {
                    Text("\u{201C}\(model.quote)\u{201D}").font(.caption).italic().foregroundStyle(.secondary).lineLimit(3)
                    Spacer()
                    Button { model.removeQuote() } label: { Image(systemName: "xmark") }.buttonStyle(.borderless)
                        .accessibilityLabel("Remove quote").accessibilityIdentifier("review-remove-quote")
                }
            }
            TextEditor(text: $model.body)
                .font(.callout)
                .frame(height: 90)
                .scrollContentBackground(.hidden)
                .padding(4)
                .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 6))
                .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(theme["line"]))
                .accessibilityLabel("Comment")
                .accessibilityIdentifier("review-comment-field")
            HStack {
                Text("Command+Return to add").font(.caption).foregroundStyle(.secondary)
                Spacer()
                Button("Comment") { Task { await model.addComment() } }
                    .keyboardShortcut(.return, modifiers: .command)
                    .buttonStyle(.borderedProminent).controlSize(.small)
                    .disabled(!model.canAdd)
                    .accessibilityIdentifier("review-add-comment")
            }
        }
        .padding(10)
        .background(theme["surface2"], in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(theme["line"]))
    }
}

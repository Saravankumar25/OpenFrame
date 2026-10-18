// The Keyboard Shortcuts screen: Command, Default, Current, Conflict (Windows: Shortcuts.tsx).
// It is a Settings tab and, from Help, a sheet over the main window.

import AppKit
import OpenFrameCore
import SwiftUI

extension Notification.Name {
    /// Help > Keyboard Shortcuts: the main window shows the shortcuts sheet.
    static let openFrameShowShortcuts = Notification.Name("studio.openframe.showShortcuts")
}

struct ShortcutsSettingsView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var model: ShortcutsSettingsModel

    init(model: ShortcutsSettingsModel) {
        _model = State(initialValue: model)
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            if !model.conflictList.isEmpty {
                Label("Some shortcuts are used by two commands. Change one of them below; until then the first command in the list wins.",
                      systemImage: "exclamationmark.triangle")
                    .font(.callout)
                    .foregroundStyle(theme["yellow-t"])
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme["yellow-soft"])
                    .accessibilityIdentifier("shortcuts-conflict-banner")
            }
            list
        }
        .task { await ShortcutStore.shared.reload(core: app.core) }
        .confirmationDialog(
            model.pending?.title ?? "", isPresented: Binding(get: { model.pending != nil }, set: { if !$0 { model.pending = nil } }),
            titleVisibility: .visible
        ) {
            Button(model.pending?.confirm ?? "Continue") { Task { await model.confirmPending() } }
                .accessibilityIdentifier("shortcuts-confirm-move")
            Button("Cancel", role: .cancel) { model.pending = nil }
                .accessibilityIdentifier("shortcuts-cancel-move")
        } message: {
            Text(model.pending?.body ?? "")
        }
        .confirmationDialog(
            "Restore all keyboard shortcuts?", isPresented: $model.askingResetAll, titleVisibility: .visible
        ) {
            Button("Restore all defaults", role: .destructive) { Task { await model.resetAll() } }
                .accessibilityIdentifier("shortcuts-confirm-reset")
            Button("Cancel", role: .cancel) { model.askingResetAll = false }
                .accessibilityIdentifier("shortcuts-cancel-reset")
        } message: {
            Text("Every shortcut you changed goes back to its default. This can't be undone, but you can change them again at any time.")
        }
    }

    private var header: some View {
        HStack(spacing: 12) {
            TextField("Filter commands or keys", text: $model.filter)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 320)
                .accessibilityLabel("Filter commands")
                .accessibilityIdentifier("shortcuts-filter")
            Text("\(model.shown.count) commands")
                .font(.caption).foregroundStyle(.secondary)
                .accessibilityIdentifier("shortcuts-count")
            Spacer()
            Button {
                model.askingResetAll = true
            } label: {
                Label("Restore all defaults", systemImage: "arrow.counterclockwise")
            }
            .disabled(!model.hasChanges || model.busy)
            .accessibilityIdentifier("shortcuts-restore-all")
        }
        .padding(12)
    }

    private var list: some View {
        List {
            Text("Change, remove or add keys. Every command also has a button or menu on screen. Your shortcuts apply in every project on this computer.")
                .font(.callout).foregroundStyle(.secondary)
                .listRowSeparator(.hidden)
            ForEach(model.groups, id: \.self) { group in
                Section(group) {
                    ForEach(model.commands(in: group)) { command in
                        row(command)
                    }
                }
            }
            if model.shown.isEmpty {
                Text("No command matches \u{201C}\(model.filter)\u{201D}.")
                    .foregroundStyle(.secondary)
                    .accessibilityIdentifier("shortcuts-none")
            }
        }
        .listStyle(.inset)
        .accessibilityIdentifier("shortcuts-list")
    }

    private func row(_ c: ShortcutCommand) -> some View {
        let current = model.current(c)
        let clash = model.conflictNames(c)
        return VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .top, spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(c.label).fontWeight(.medium)
                    Text("On screen: \(c.route)").font(.caption).foregroundStyle(.secondary)
                    HStack(spacing: 4) {
                        Text("Current").font(.caption).foregroundStyle(.secondary)
                        if current.isEmpty && model.recording != c.id { Text("None").font(.callout).foregroundStyle(.secondary) }
                        ForEach(Array(current.enumerated()), id: \.offset) { index, chord in
                            keyChip(c, chord, index: index)
                        }
                        if model.recording == c.id {
                            ShortcutRecorder(
                                label: "Press the new keys for \(c.label). Escape cancels.",
                                onPress: { press in Task { await model.record(press, for: c) } },
                                onCancel: { model.cancelRecording() })
                                .frame(width: 190, height: 24)
                                .accessibilityIdentifier("shortcut-recorder-\(c.id)")
                        }
                        if model.isChanged(c) { Text("(changed)").font(.caption).foregroundStyle(.secondary) }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                VStack(alignment: .leading, spacing: 1) {
                    Text("Default").font(.caption2).foregroundStyle(.secondary)
                    Text(c.defaults.isEmpty ? "None" : c.defaults.map(ShortcutCatalogue.display).joined(separator: ", "))
                        .font(.callout).foregroundStyle(.secondary)
                }
                .frame(width: 90, alignment: .leading)
                VStack(alignment: .leading, spacing: 1) {
                    Text("Conflict").font(.caption2).foregroundStyle(.secondary)
                    Text(clash.isEmpty ? "None" : "Also used by \(clash.joined(separator: ", "))")
                        .font(.callout)
                        .foregroundStyle(clash.isEmpty ? Color.secondary : theme["red-t"])
                }
                .frame(width: 110, alignment: .leading)
                VStack(alignment: .trailing, spacing: 4) {
                    Button {
                        model.startRecording(c)
                    } label: {
                        Label("Add", systemImage: "plus")
                    }
                    .disabled(model.busy || current.count >= ShortcutCatalogue.maxKeysPerCommand || model.recording == c.id)
                    .accessibilityLabel("Add a shortcut for \(c.label)")
                    .accessibilityIdentifier("shortcut-add-\(c.id)")
                    Button("Restore default") { Task { await model.restore(c) } }
                        .disabled(model.busy || !model.isChanged(c))
                        .accessibilityLabel("Restore the default for \(c.label)")
                        .accessibilityIdentifier("shortcut-restore-\(c.id)")
                }
                .controlSize(.small)
                .frame(width: 112, alignment: .trailing)
            }
            if let error = model.error, error.id == c.id {
                Text(error.text)
                    .font(.caption).foregroundStyle(theme["red-t"])
                    .accessibilityIdentifier("shortcut-error-\(c.id)")
            }
        }
        .padding(.vertical, 3)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("shortcut-row-\(c.id)")
    }

    private func keyChip(_ c: ShortcutCommand, _ chord: String, index: Int) -> some View {
        HStack(spacing: 2) {
            Text(ShortcutCatalogue.display(chord))
                .font(.system(.callout, design: .rounded).weight(.medium))
            Button {
                Task { await model.remove(chord, from: c) }
            } label: {
                Image(systemName: "xmark").font(.system(size: 8, weight: .bold))
            }
            .buttonStyle(.plain)
            .disabled(model.busy)
            .accessibilityLabel("Remove \(ShortcutCatalogue.display(chord)) from \(c.label)")
            .accessibilityIdentifier("shortcut-remove-\(c.id)-\(index)")
        }
        .padding(.horizontal, 7).padding(.vertical, 3)
        .background(theme["panel"], in: Capsule())
        .overlay(Capsule().stroke(theme["line"], lineWidth: 1))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("shortcut-key-\(c.id)-\(index)")
    }
}

/// Focused key catcher: the next key combination becomes the new shortcut. It takes Command
/// combinations before the menus see them and Escape before the sheet does.
struct ShortcutRecorder: NSViewRepresentable {
    let label: String
    let onPress: (ShortcutCatalogue.Press) -> Void
    let onCancel: () -> Void

    func makeNSView(context: Context) -> RecorderView {
        let view = RecorderView()
        view.onPress = onPress
        view.onCancel = onCancel
        view.setAccessibilityLabel(label)
        DispatchQueue.main.async { view.window?.makeFirstResponder(view) }
        return view
    }

    func updateNSView(_ view: RecorderView, context: Context) {
        view.onPress = onPress
        view.onCancel = onCancel
    }

    final class RecorderView: NSView {
        var onPress: (ShortcutCatalogue.Press) -> Void = { _ in }
        var onCancel: () -> Void = {}

        override var acceptsFirstResponder: Bool { true }
        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            DispatchQueue.main.async { [weak self] in
                guard let self, let window = self.window else { return }
                window.makeFirstResponder(self)
            }
        }

        override func resignFirstResponder() -> Bool {
            DispatchQueue.main.async { [weak self] in self?.onCancel() }
            return true
        }

        override func performKeyEquivalent(with event: NSEvent) -> Bool {
            guard window?.firstResponder === self else { return false }
            handle(event)
            return true
        }

        override func keyDown(with event: NSEvent) { handle(event) }

        private func handle(_ event: NSEvent) {
            let plain = event.modifierFlags.intersection([.command, .option, .shift, .control]).isEmpty
            if plain && event.keyCode == 53 { onCancel(); return }
            if plain && event.keyCode == 48 { onCancel(); return }
            onPress(ShortcutCatalogue.press(event))
        }

        override func draw(_ dirtyRect: NSRect) {
            let path = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 6, yRadius: 6)
            NSColor.controlAccentColor.withAlphaComponent(0.12).setFill()
            path.fill()
            NSColor.controlAccentColor.setStroke()
            path.lineWidth = 1
            path.stroke()
            let text = "Press the new keys\u{2026} (Esc cancels)" as NSString
            let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: 11), .foregroundColor: NSColor.labelColor]
            let size = text.size(withAttributes: attrs)
            text.draw(at: NSPoint(x: 8, y: (bounds.height - size.height) / 2), withAttributes: attrs)
        }

        override func isAccessibilityElement() -> Bool { true }
        override func accessibilityRole() -> NSAccessibility.Role? { .button }
    }
}

/// Help > Keyboard Shortcuts: the same table in a sheet. While a key is being recorded Escape
/// belongs to the recorder, so the sheet stays open until recording ends.
struct ShortcutsSheet: View {
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @State private var model: ShortcutsSettingsModel

    init(app: AppModel) {
        _model = State(initialValue: ShortcutsSettingsModel(app: app))
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Keyboard Shortcuts").font(.title2.weight(.semibold))
                Spacer()
                Button("Done") { dismiss() }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("shortcuts-done")
            }
            .padding([.horizontal, .top], 16)
            ShortcutsSettingsView(model: model)
        }
        .frame(minWidth: 740, idealWidth: 780, minHeight: 540, idealHeight: 640)
        .interactiveDismissDisabled(model.recording != nil)
        .accessibilityIdentifier("shortcuts-sheet")
    }
}

/// The Settings tab: makes its model once the core is there.
struct ShortcutsSettingsTab: View {
    @Environment(AppModel.self) private var app
    @State private var model: ShortcutsSettingsModel?

    var body: some View {
        Group {
            if let model { ShortcutsSettingsView(model: model) }
        }
        .onAppear { if model == nil { model = ShortcutsSettingsModel(app: app) } }
    }
}

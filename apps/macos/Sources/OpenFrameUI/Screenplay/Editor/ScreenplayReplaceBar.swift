// Replace All for the script (Command+Option+F). Find and Find Next stay with the system find
// bar (Command+F, Command+G); this bar adds what the core's replace does: match case, whole
// word, and the scope of the draft or the current scene. The core makes the change as one step
// that project undo takes back, so the editor is saved first and shows the core's text after.

import SwiftUI

struct ScreenplayReplaceBar: View {
    @Bindable var model: ScreenplayWorkspaceModel
    @Environment(\.ofTheme) private var theme
    @FocusState private var focusFind: Bool

    var body: some View {
        HStack(spacing: 10) {
            TextField("Find", text: $model.findText)
                .textFieldStyle(.roundedBorder)
                .frame(minWidth: 140, maxWidth: 220)
                .focused($focusFind)
                .accessibilityLabel("Find")
                .accessibilityIdentifier("replace-find")
            TextField("Replace with", text: $model.replaceText)
                .textFieldStyle(.roundedBorder)
                .frame(minWidth: 140, maxWidth: 220)
                .accessibilityLabel("Replace with")
                .accessibilityIdentifier("replace-with")
            Toggle("Match Case", isOn: $model.matchCase)
                .toggleStyle(.checkbox)
                .accessibilityIdentifier("replace-match-case")
            Toggle("Whole Word", isOn: $model.wholeWord)
                .toggleStyle(.checkbox)
                .accessibilityIdentifier("replace-whole-word")
            Picker("Scope", selection: $model.scopeIsScene) {
                Text("This Scene").tag(true)
                Text("Whole Script").tag(false)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 190)
            .accessibilityLabel("Scope")
            .accessibilityIdentifier("replace-scope")
            Button("Replace All") { Task { await model.replaceAll() } }
                .disabled(model.findText.isEmpty || !model.canEdit)
                .keyboardShortcut(.return, modifiers: [])
                .accessibilityIdentifier("replace-all")
            Spacer(minLength: 0)
            Button {
                model.showReplace = false
                model.textView.map { $0.window?.makeFirstResponder($0) }
            } label: {
                Image(systemName: "xmark.circle.fill")
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .keyboardShortcut(.cancelAction)
            .help("Close")
            .accessibilityLabel("Close replace")
            .accessibilityIdentifier("replace-close")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(theme["panel"])
        .overlay(alignment: .bottom) { Divider() }
        .onAppear { focusFind = true }
        .onChange(of: model.replaceFocusRequest) { focusFind = true }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Replace in script")
        .accessibilityIdentifier("replace-bar")
    }
}
